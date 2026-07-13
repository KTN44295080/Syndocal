import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const largeShowMode = process.argv.includes("--large-show");
const viewportFixture = process.env.SYNDOCAL_VIEWPORT_FIXTURE ?? (largeShowMode ? "large-show" : "timeline");
const defaultUrl =
  viewportFixture === "none"
    ? "http://127.0.0.1:5173/"
    : `http://127.0.0.1:5173/?syndocalViewportFixture=${encodeURIComponent(viewportFixture)}`;
const appUrl = process.env.SYNDOCAL_VIEWPORT_URL ?? defaultUrl;
const shouldStartVite = appUrl === defaultUrl && process.env.SYNDOCAL_VIEWPORT_NO_SERVER !== "1";
const shouldCheckTimelineAutomation = new URL(appUrl).searchParams.get("syndocalViewportFixture") === "timeline";
const vitePort = 5173;
const cdpPort = Number(process.env.SYNDOCAL_CDP_PORT ?? 9227);
const screenshotDir = process.env.SYNDOCAL_VIEWPORT_SCREENSHOT_DIR
  ? resolve(process.env.SYNDOCAL_VIEWPORT_SCREENSHOT_DIR)
  : null;
const allViewports = [
  { width: 1280, height: 720 },
  { width: 1366, height: 768 },
  { width: 2048, height: 1129 },
];
const viewports = largeShowMode || process.env.SYNDOCAL_VIEWPORT_SINGLE === "1" ? [allViewports[1]] : allViewports;
const setupTabs = [
  { area: "Lighting", tab: "Library", id: "library" },
  { area: "Lighting", tab: "Profiles", id: "profiles" },
  { area: "Lighting", tab: "Patch", id: "patch" },
  { area: "Video", tab: "Outputs", id: "video" },
  { area: "Mapping", tab: "Stage Map", id: "mapping" },
  { area: "I/O", tab: "DMX", id: "dmx" },
  { area: "I/O", tab: "MIDI", id: "midi" },
  { area: "I/O", tab: "OSC", id: "osc" },
  { area: "I/O", tab: "Remote", id: "remote" },
];
const controlTabs = [
  { id: "edit", label: "Live Edit" },
  { id: "live", label: "Timeline" },
  { id: "mixer", label: "VJ Desk" },
];
const viewportRecentProjects = [
  "C:/shows/front-room.sdc",
  "C:/shows/main-stage.sdc",
  "C:/shows/projector-map-test.sdc",
  "C:/shows/festival/live-floor.sdc",
  "C:/shows/long/path/with/a/very-long-syndocal-project-name-for-menu-containment.sdc",
  "C:/shows/backup.sdc",
];
const viewportRecoveryCheckpoint = {
  version: 1,
  app: "Syndocal",
  saved_at: "2026-07-09T12:34:00.000Z",
  source_path: "C:/shows/recovered-main-stage.sdc",
  signature: "viewport-recovery",
  project: {
    version: 1,
    app: "Syndocal",
    custom_profiles: [],
    snapshot: {
      fixtures: [],
      cues: [],
    },
  },
};

const sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms));

function executableExists(command) {
  if (command.includes("\\") || command.includes("/")) {
    return existsSync(command);
  }
  const pathDirs = (process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":");
  const extensions = process.platform === "win32" ? ["", ".exe", ".cmd", ".bat"] : [""];
  return pathDirs.some((dir) => extensions.some((extension) => existsSync(join(dir, `${command}${extension}`))));
}

function findBrowser() {
  const envCandidates = [process.env.CHROME_PATH, process.env.EDGE_PATH].filter(Boolean);
  const platformCandidates =
    process.platform === "win32"
      ? [
          join(process.env.PROGRAMFILES ?? "C:\\Program Files", "Google\\Chrome\\Application\\chrome.exe"),
          join(process.env["PROGRAMFILES(X86)"] ?? "C:\\Program Files (x86)", "Google\\Chrome\\Application\\chrome.exe"),
          join(process.env.LOCALAPPDATA ?? "", "Google\\Chrome\\Application\\chrome.exe"),
          join(process.env.PROGRAMFILES ?? "C:\\Program Files", "Microsoft\\Edge\\Application\\msedge.exe"),
          join(process.env["PROGRAMFILES(X86)"] ?? "C:\\Program Files (x86)", "Microsoft\\Edge\\Application\\msedge.exe"),
        ]
      : process.platform === "darwin"
        ? [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
          ]
        : ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "microsoft-edge"];
  return [...envCandidates, ...platformCandidates].find((candidate) => candidate && executableExists(candidate)) ?? null;
}

function startProcess(command, args, options = {}) {
  const child = spawn(command, args, {
    cwd: options.cwd,
    stdio: options.stdio ?? "ignore",
    windowsHide: true,
    shell: false,
    env: { ...process.env, ...(options.env ?? {}) },
  });
  child.on("error", (error) => {
    console.error(`Failed to start ${command}: ${error.message}`);
  });
  return child;
}

async function stopProcess(child) {
  if (!child || child.killed) {
    return;
  }
  try {
    child.kill("SIGTERM");
  } catch {
    // Ignore cleanup races.
  }
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (child.exitCode !== null || child.signalCode !== null) {
      return;
    }
    await sleep(50);
  }
  try {
    child.kill("SIGKILL");
  } catch {
    // Ignore cleanup races.
  }
}

async function waitForHttp(url, description) {
  let lastError = null;
  for (let attempt = 0; attempt < 80; attempt += 1) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return response;
      }
      lastError = new Error(`${response.status} ${response.statusText}`);
    } catch (error) {
      lastError = error;
    }
    await sleep(250);
  }
  throw new Error(`${description} did not become ready: ${lastError?.message ?? "unknown error"}`);
}

class CdpClient {
  constructor(webSocketUrl) {
    this.webSocketUrl = webSocketUrl;
    this.nextId = 1;
    this.pending = new Map();
    this.socket = null;
  }

  async connect() {
    this.socket = new WebSocket(this.webSocketUrl);
    await new Promise((resolveConnect, rejectConnect) => {
      this.socket.addEventListener("open", resolveConnect, { once: true });
      this.socket.addEventListener("error", rejectConnect, { once: true });
    });
    this.socket.addEventListener("message", (event) => {
      const data = JSON.parse(event.data);
      if (!data.id || !this.pending.has(data.id)) {
        return;
      }
      const { resolve: resolveCall, reject } = this.pending.get(data.id);
      this.pending.delete(data.id);
      if (data.error) {
        reject(new Error(JSON.stringify(data.error)));
        return;
      }
      resolveCall(data.result);
    });
  }

  send(method, params = {}) {
    if (!this.socket) {
      throw new Error("CDP socket is not connected.");
    }
    const id = this.nextId;
    this.nextId += 1;
    this.socket.send(JSON.stringify({ id, method, params }));
    return new Promise((resolveCall, reject) => {
      this.pending.set(id, { resolve: resolveCall, reject });
    });
  }

  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (result.exceptionDetails) {
      throw new Error(result.exceptionDetails.text ?? "Runtime evaluation failed.");
    }
    return result.result.value;
  }

  close() {
    this.socket?.close();
  }
}

async function createCdpClient() {
  const response = await fetch(`http://127.0.0.1:${cdpPort}/json/new?${encodeURIComponent(appUrl)}`, {
    method: "PUT",
  });
  if (!response.ok) {
    throw new Error(`Failed to create Chrome target: ${response.status} ${response.statusText}`);
  }
  const target = await response.json();
  const client = new CdpClient(target.webSocketDebuggerUrl);
  await client.connect();
  await client.send("Runtime.enable");
  await client.send("Page.enable");
  return client;
}

async function waitForApp(client) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    const ready = await client.evaluate("Boolean(document.querySelector('.app'))");
    if (ready) {
      return;
    }
    await sleep(100);
  }
  throw new Error("Syndocal app shell did not mount.");
}

async function seedViewportLocalStorage(client) {
  await client.evaluate(`(() => {
    window.localStorage.setItem('syndocal.recentProjects.v1', ${JSON.stringify(JSON.stringify(viewportRecentProjects))});
    window.localStorage.setItem('syndocal.projectRecovery.v1', ${JSON.stringify(JSON.stringify(viewportRecoveryCheckpoint))});
    window.localStorage.setItem('syndocal.workspaceLayout.v1', JSON.stringify({
      workspace_tab: 'setup',
      setup_sub_tab: 'patch',
      control_mode: 'edit',
      timeline_desk_surface: 'show',
      edit_desk_surface: 'attributes',
      control_category: 'position',
    }));
  })()`);
}

async function clickByText(client, text) {
  const clicked = await client.evaluate(`(() => {
    const wanted = ${JSON.stringify(text)}.toLowerCase();
    const nodes = [...document.querySelectorAll('button, [role="tab"], a')];
    const exact = nodes.find((node) => (node.textContent || '').trim().toLowerCase() === wanted);
    const loose = exact || nodes.find((node) => (node.textContent || '').trim().toLowerCase().includes(wanted));
    if (!loose) {
      return false;
    }
    loose.click();
    return true;
  })()`);
  if (!clicked) {
    throw new Error(`Could not find tab/button text: ${text}`);
  }
}

async function clickVisibleByText(client, selector, text) {
  const clicked = await client.evaluate(`(() => {
    const wanted = ${JSON.stringify(text)}.toLowerCase();
    const nodes = [...document.querySelectorAll(${JSON.stringify(selector)})]
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        const style = window.getComputedStyle(node);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      });
    const exact = nodes.find((node) => (node.textContent || '').trim().toLowerCase() === wanted);
    const loose = exact || nodes.find((node) => (node.textContent || '').trim().toLowerCase().includes(wanted));
    if (!loose) {
      return false;
    }
    loose.click();
    return true;
  })()`);
  if (!clicked) {
    throw new Error(`Could not find visible ${selector} text: ${text}`);
  }
}

async function pressKey(client, code, key = code, modifiers = 0) {
  await client.send("Input.dispatchKeyEvent", { type: "keyDown", code, key, modifiers });
  await client.send("Input.dispatchKeyEvent", { type: "keyUp", code, key, modifiers });
}

async function checkWorkspaceLayoutPersistence(client) {
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await clickVisibleByText(client, ".timelineDeskTabs button", "Playback");
  await sleep(100);
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  const restored = await client.evaluate(`(() => ({
    workspace: (document.querySelector('.workspaceTabs button.active')?.textContent || '').trim(),
    controlMode: (document.querySelector('.controlModeTabs button.active')?.textContent || '').trim(),
    desk: (document.querySelector('.timelineDeskTabs button.active')?.textContent || '').trim(),
  }))()`);
  if (restored.workspace !== "Control" || restored.controlMode !== "Timeline" || restored.desk !== "Playback") {
    throw new Error(`Workspace layout did not restore: ${JSON.stringify(restored)}`);
  }

  await clickVisibleByText(client, ".appMenuButton", "...");
  await clickVisibleByText(client, ".appProjectMenu button", "Reset Layout");
  await sleep(100);
  const reset = await client.evaluate(`(() => {
    const stored = JSON.parse(window.localStorage.getItem('syndocal.workspaceLayout.v1') || '{}');
    return {
      workspace: (document.querySelector('.workspaceTabs button.active')?.textContent || '').trim(),
      setupTab: (document.querySelector('.setupModeTabs button.active')?.textContent || '').trim(),
      stored,
    };
  })()`);
  if (
    reset.workspace !== "Setup" ||
    reset.setupTab !== "Patch" ||
    reset.stored.workspace_tab !== "setup" ||
    reset.stored.setup_sub_tab !== "patch" ||
    reset.stored.control_mode !== "edit"
  ) {
    throw new Error(`Workspace layout reset failed: ${JSON.stringify(reset)}`);
  }
  console.log("pass workspace-layout-persistence");
}

async function checkKeyboardNavigation(client) {
  const firstWorkspaceFocus = await client.evaluate(`(() => {
    const button = [...document.querySelectorAll('.workspaceTabs button')]
      .find((candidate) => (candidate.textContent || '').trim() === 'Setup');
    button?.focus();
    return (document.activeElement?.textContent || '').trim();
  })()`);
  await pressKey(client, "Tab", "Tab");
  const tabOrder = await client.evaluate(`(() => {
    const active = document.activeElement;
    const style = active ? getComputedStyle(active) : null;
    return {
      label: (active?.textContent || '').trim(),
      outlineStyle: style?.outlineStyle ?? '',
      outlineWidth: Number.parseFloat(style?.outlineWidth ?? '0'),
    };
  })()`);

  await pressKey(client, "F1");
  await sleep(60);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(80);
  const setupEditableGuard = await client.evaluate(`(async () => {
    const visible = (element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return !element.disabled && rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const input = [...document.querySelectorAll('.layoutSetup input:not([type="checkbox"])')].find(visible);
    const activeSetupBefore = (document.querySelector('.setupModeTabs button.active')?.textContent || '').trim();
    const activeToolBefore = (document.querySelector('.mappingViewportControls strong')?.textContent || '').trim();
    input?.focus();
    input?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, code: 'Digit1', key: '1' }));
    input?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, code: 'KeyP', key: 'p' }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const activeSetupAfter = (document.querySelector('.setupModeTabs button.active')?.textContent || '').trim();
    const activeToolAfter = (document.querySelector('.mappingViewportControls strong')?.textContent || '').trim();
    return {
      found: Boolean(input),
      setupModePreserved: activeSetupBefore.length > 0 && activeSetupAfter === activeSetupBefore,
      mappingToolPreserved: activeToolBefore.length > 0 && activeToolAfter === activeToolBefore,
    };
  })()`);

  await pressKey(client, "F2");
  await sleep(80);
  const controlEditableGuard = await client.evaluate(`(async () => {
    const visible = (element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return !element.disabled && rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const input = [...document.querySelectorAll('.layoutControl input:not([type="checkbox"]), .layoutControl select')].find(visible);
    const activeModeBefore = (document.querySelector('.controlModeTabs button.active')?.textContent || '').trim();
    input?.focus();
    input?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, code: 'KeyL', key: 'l' }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const activeModeAfter = (document.querySelector('.controlModeTabs button.active')?.textContent || '').trim();
    return {
      found: Boolean(input),
      modePreserved: activeModeBefore === 'Live Edit' && activeModeAfter === activeModeBefore,
    };
  })()`);

  const result = {
    firstWorkspaceFocus,
    tabOrder,
    setupEditableGuard,
    controlEditableGuard,
  };
  result.passed =
    firstWorkspaceFocus === "Setup" &&
    tabOrder.label === "Control" &&
    tabOrder.outlineStyle !== "none" &&
    tabOrder.outlineWidth >= 1 &&
    setupEditableGuard.found &&
    setupEditableGuard.setupModePreserved &&
    setupEditableGuard.mappingToolPreserved &&
    controlEditableGuard.found &&
    controlEditableGuard.modePreserved;
  await client.evaluate(`(() => {
    window.__syndocalKeyboardNavigationCheck = ${JSON.stringify(result)};
  })()`);
  return result.passed;
}

async function measure(client, label) {
  return await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const visibleCount = (selector) => [...document.querySelectorAll(selector)]
      .filter((element) => {
        const rect = element.getBoundingClientRect();
        const style = window.getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      }).length;
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    const layout = document.querySelector('.layout');
    const documentStyle = window.getComputedStyle(documentElement);
    const bodyStyle = window.getComputedStyle(body);
    const appStyle = app ? window.getComputedStyle(app) : null;
    const layoutStyle = layout ? window.getComputedStyle(layout) : null;
    const appRect = app ? app.getBoundingClientRect() : null;
    const layoutRect = layout ? layout.getBoundingClientRect() : null;
    const compactMappingStage = document.querySelector('.layoutSetup.setupMode-patch .mappingVisualizer.compact .visualizerStage');
    const compactMappingStageRect = compactMappingStage?.getBoundingClientRect() ?? null;
    const videoSetupSidebar = document.querySelector('.videoSetupPanel .videoSetupSidebar');
    const videoSetupSidebarRect = videoSetupSidebar?.getBoundingClientRect() ?? null;
    const videoSetupOutputDesk = document.querySelector('.videoSetupPanel .videoSetupOutputDesk');
    const videoSetupOutputDeskRect = videoSetupOutputDesk?.getBoundingClientRect() ?? null;
    const profileLoadPanel = document.querySelector('.setupMode-library .profileLoadPanel');
    const profileLoadPanelRect = profileLoadPanel?.getBoundingClientRect() ?? null;
    const loadedProfileSummaryPanel = document.querySelector('.setupMode-library .loadedProfileSummaryPanel');
    const loadedProfileSummaryPanelRect = loadedProfileSummaryPanel?.getBoundingClientRect() ?? null;
    const customProfileAttributePane = document.querySelector('.setupMode-profiles .customProfileAttributePane');
    const customProfileAttributePaneRect = customProfileAttributePane?.getBoundingClientRect() ?? null;
    const customProfilePreviewDesk = document.querySelector('.setupMode-profiles .customProfilePreviewDesk');
    const customProfilePreviewDeskRect = customProfilePreviewDesk?.getBoundingClientRect() ?? null;
    const dmxOutputConfigPanel = document.querySelector('.setupMode-dmx .dmxOutputConfigPanel');
    const dmxOutputConfigPanelRect = dmxOutputConfigPanel?.getBoundingClientRect() ?? null;
    const outputDiagnosticsDesk = document.querySelector('.setupMode-dmx .outputDiagnosticsDesk');
    const outputDiagnosticsDeskRect = outputDiagnosticsDesk?.getBoundingClientRect() ?? null;
    const lightingRuntimeDesk = document.querySelector('.setupMode-dmx .lightingRuntimeDesk');
    const lightingRuntimeDeskRect = lightingRuntimeDesk?.getBoundingClientRect() ?? null;
    const midiMappingEditorDesk = document.querySelector('.setupMode-midi .mappingEditorDesk');
    const midiMappingEditorDeskRect = midiMappingEditorDesk?.getBoundingClientRect() ?? null;
    const midiMappingListDesk = document.querySelector('.setupMode-midi .mappingListDesk');
    const midiMappingListDeskRect = midiMappingListDesk?.getBoundingClientRect() ?? null;
    const oscMappingEditorDesk = document.querySelector('.setupMode-osc .mappingEditorDesk');
    const oscMappingEditorDeskRect = oscMappingEditorDesk?.getBoundingClientRect() ?? null;
    const oscMappingListDesk = document.querySelector('.setupMode-osc .mappingListDesk');
    const oscMappingListDeskRect = oscMappingListDesk?.getBoundingClientRect() ?? null;
    const remoteServerDesk = document.querySelector('.setupMode-remote .remoteServerDesk');
    const remoteServerDeskRect = remoteServerDesk?.getBoundingClientRect() ?? null;
    const remoteEndpointDesk = document.querySelector('.setupMode-remote .remoteEndpointDesk');
    const remoteEndpointDeskRect = remoteEndpointDesk?.getBoundingClientRect() ?? null;
    window.scrollTo(9999, 9999);
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const movedX = window.scrollX;
    const movedY = window.scrollY;
    window.scrollTo(0, 0);
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    return {
      label: ${JSON.stringify(label)},
      innerWidth,
      innerHeight,
      documentClientWidth: documentElement.clientWidth,
      documentScrollWidth: documentElement.scrollWidth,
      documentClientHeight: documentElement.clientHeight,
      documentScrollHeight: documentElement.scrollHeight,
      bodyScrollWidth: body.scrollWidth,
      bodyScrollHeight: body.scrollHeight,
      appClientWidth: app ? app.clientWidth : null,
      appScrollWidth: app ? app.scrollWidth : null,
      appClientHeight: app ? app.clientHeight : null,
      appScrollHeight: app ? app.scrollHeight : null,
      appPosition: appStyle?.position ?? '',
      appOverflowX: appStyle?.overflowX ?? '',
      appOverflowY: appStyle?.overflowY ?? '',
      appRectWidth: appRect ? Math.round(appRect.width) : null,
      appRectHeight: appRect ? Math.round(appRect.height) : null,
      documentOverflowX: documentStyle.overflowX,
      documentOverflowY: documentStyle.overflowY,
      bodyOverflowX: bodyStyle.overflowX,
      bodyOverflowY: bodyStyle.overflowY,
      layoutOverflowX: layoutStyle?.overflowX ?? '',
      layoutOverflowY: layoutStyle?.overflowY ?? '',
      layoutClientWidth: layout ? layout.clientWidth : null,
      layoutClientHeight: layout ? layout.clientHeight : null,
      layoutScrollWidth: layout ? layout.scrollWidth : null,
      layoutScrollHeight: layout ? layout.scrollHeight : null,
      layoutRectWidth: layoutRect ? Math.round(layoutRect.width) : null,
      layoutRectHeight: layoutRect ? Math.round(layoutRect.height) : null,
      visibleAppStatusLineCount: visibleCount('.appStatusLine[role="status"]'),
      appStatusTone: document.querySelector('.appStatusLine')?.getAttribute('data-status-tone') ?? '',
      visibleProjectMenuCount: visibleCount('.appProjectMenu'),
      visibleProjectMenuItemCount: visibleCount('.appProjectMenu button[role="menuitem"]'),
      visibleProjectMenuShortcutCount: document.querySelectorAll('.appProjectMenu button[aria-keyshortcuts]').length,
      visibleRecentProjectMenuItemCount: visibleCount('.appProjectMenu .recentProjectMenuItem'),
      visibleRecoveryProjectMenuItemCount: visibleCount('.appProjectMenu .recoveryProjectMenuItem'),
      visibleUpdateMenuLabelCount: [...document.querySelectorAll('.appProjectMenu .appProjectMenuLabel span')]
        .filter((node) => (node.textContent || '').trim() === 'Application Updates').length,
      visibleUpdateCheckButtonCount: [...document.querySelectorAll('.appProjectMenu button[role="menuitem"]')]
        .filter((button) => (button.textContent || '').includes('Check for Updates')).length,
      visibleUserTemplateMenuLabelCount: [...document.querySelectorAll('.appProjectMenu .appProjectMenuLabel span')]
        .filter((node) => (node.textContent || '').trim() === 'User Templates').length,
      visibleUserTemplateActionCount: [...document.querySelectorAll('.appProjectMenu button[role="menuitem"]')]
        .filter((button) => /(?:New from|Save as) Template/.test(button.textContent || '')).length,
      documentLanguage: document.documentElement.lang,
      visibleJapaneseLanguageLabelCount: [...document.querySelectorAll('.appProjectMenu .appProjectMenuLabel span')]
        .filter((node) => (node.textContent || '').trim() === 'UI言語').length,
      visibleJapaneseSaveButtonCount: [...document.querySelectorAll('.appProjectMenu button[role="menuitem"] span')]
        .filter((node) => (node.textContent || '').trim() === '保存').length,
      preservedUserFixtureLabelCount: [...document.querySelectorAll('.fixtureList .fixture strong')]
        .filter((node) => ['Video', 'Save', 'Output'].includes((node.textContent || '').trim())).length,
      translatedUserFixtureCollisionCount: [...document.querySelectorAll('.fixtureList .fixture strong')]
        .filter((node) => ['映像', '保存', '出力'].includes((node.textContent || '').trim())).length,
      timelineAutomationRangeCount: document.querySelectorAll('.timelineOverview .timelineAutomationRange').length,
      timelineAutomationHandleCount: document.querySelectorAll('.timelineOverview .timelineAutomationHandle').length,
      timelineAutomationKeyframeCount: document.querySelectorAll('.timelineOverview .timelineAutomationKeyframe').length,
      timelineAutomationChipCount: document.querySelectorAll('.timelineKeyframeStrip button').length,
      timelineAutomationCurveCount: document.querySelectorAll('.timelineKeyframeCurve').length,
      timelineAutomationValueInputCount: document.querySelectorAll('.timelineKeyframeValue').length,
      timelineAutomationEnabledToggleCount: document.querySelectorAll('.timelineAutomationEnabled input[type="checkbox"]').length,
      timelineAutomationScopeCount: document.querySelectorAll('.timelineAutomationScope').length,
      timelineAutomationBatchButtonCount: document.querySelectorAll('.timelineAutomationBatchActions button').length,
      timelineAutomationGroupButtonCount: [...document.querySelectorAll('.timelineAutomationActions button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'group').length,
      timelineAutomationRangeWidths: [...document.querySelectorAll('.timelineOverview .timelineAutomationRange > rect:not(.timelineAutomationHandle)')]
        .map((rect) => Number(rect.getAttribute('width') || 0)),
      mappingUseInEffectsButtonCount: [...document.querySelectorAll('.mappingSelectionPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'use in effects').length,
      mappingWaveDraftButtonCount: [...document.querySelectorAll('.mappingSelectionPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'wave draft').length,
      visibleMappingHotkeyHelpCount: visibleCount('.mappingHotkeyHelp'),
      mappingHotkeyHelpKeyCount: document.querySelectorAll('.mappingHotkeyHelp kbd').length,
      visiblePatchActionRowCount: visibleCount('.patchActionRow'),
      visiblePatchAutoButtonCount: [...document.querySelectorAll('.fieldWithAction button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'auto').length,
      visiblePatchPrimaryButtonCount: [...document.querySelectorAll('.patchActionRow button.primary')]
        .filter((button) => (button.textContent || '').trim().toLowerCase().startsWith('patch fixture')).length,
      visiblePatchNextFreeButtonCount: [...document.querySelectorAll('.patchActionRow button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase().startsWith('next free')).length,
      visiblePatchFootprintCount: visibleCount('.footprint'),
      visibleDmxAddressGridCount: visibleCount('.dmxAddressGrid'),
      dmxAddressCellCount: document.querySelectorAll('.dmxAddressCell').length,
      dmxAddressOccupiedCellCount: document.querySelectorAll('.dmxAddressCell.occupied').length,
      dmxAddressPlannedCellCount: document.querySelectorAll('.dmxAddressCell.planned').length,
      visibleDmxFixtureBlockCount: visibleCount('.dmxPatchFixtureBlock'),
      compactMappingStageWidth: compactMappingStageRect ? Math.round(compactMappingStageRect.width) : 0,
      compactMappingStageHeight: compactMappingStageRect ? Math.round(compactMappingStageRect.height) : 0,
      videoSetupSidebarWidth: videoSetupSidebarRect ? Math.round(videoSetupSidebarRect.width) : 0,
      videoSetupOutputDeskWidth: videoSetupOutputDeskRect ? Math.round(videoSetupOutputDeskRect.width) : 0,
      visibleProfileLoadPanelCount: visibleCount('.setupMode-library .profileLoadPanel'),
      visibleLoadedProfileSummaryPanelCount: visibleCount('.setupMode-library .loadedProfileSummaryPanel'),
      profileLoadPanelWidth: profileLoadPanelRect ? Math.round(profileLoadPanelRect.width) : 0,
      loadedProfileSummaryPanelWidth: loadedProfileSummaryPanelRect ? Math.round(loadedProfileSummaryPanelRect.width) : 0,
      visibleCustomProfileWorkbenchCount: visibleCount('.setupMode-profiles .customProfileWorkbench'),
      customProfileAttributePaneWidth: customProfileAttributePaneRect ? Math.round(customProfileAttributePaneRect.width) : 0,
      customProfilePreviewDeskWidth: customProfilePreviewDeskRect ? Math.round(customProfilePreviewDeskRect.width) : 0,
      visibleCustomProfileActionCount: visibleCount('.setupMode-profiles .customProfileActions button'),
      visibleCustomProfileDmxMapCount: visibleCount('.setupMode-profiles .customProfileDmxMap'),
      visibleDmxOutputConfigPanelCount: visibleCount('.setupMode-dmx .dmxOutputConfigPanel'),
      visibleArtRdmPanelCount: visibleCount('.setupMode-dmx .artRdmPanel'),
      visibleOutputDiagnosticsDeskCount: visibleCount('.setupMode-dmx .outputDiagnosticsDesk'),
      visibleLightingRuntimeDeskCount: visibleCount('.setupMode-dmx .lightingRuntimeDesk'),
      dmxOutputConfigPanelWidth: dmxOutputConfigPanelRect ? Math.round(dmxOutputConfigPanelRect.width) : 0,
      outputDiagnosticsDeskWidth: outputDiagnosticsDeskRect ? Math.round(outputDiagnosticsDeskRect.width) : 0,
      lightingRuntimeDeskWidth: lightingRuntimeDeskRect ? Math.round(lightingRuntimeDeskRect.width) : 0,
      midiMappingEditorDeskWidth: midiMappingEditorDeskRect ? Math.round(midiMappingEditorDeskRect.width) : 0,
      midiMappingListDeskWidth: midiMappingListDeskRect ? Math.round(midiMappingListDeskRect.width) : 0,
      oscMappingEditorDeskWidth: oscMappingEditorDeskRect ? Math.round(oscMappingEditorDeskRect.width) : 0,
      oscMappingListDeskWidth: oscMappingListDeskRect ? Math.round(oscMappingListDeskRect.width) : 0,
      remoteServerDeskWidth: remoteServerDeskRect ? Math.round(remoteServerDeskRect.width) : 0,
      remoteEndpointDeskWidth: remoteEndpointDeskRect ? Math.round(remoteEndpointDeskRect.width) : 0,
      visibleStandbySyncDeskCount: visibleCount('.setupMode-remote .standbySyncDesk'),
      visibleStandbyRoleOptionCount: document.querySelectorAll('.setupMode-remote .standbySyncDesk select option').length,
      visibleStandbyActionButtonCount: visibleCount('.setupMode-remote .standbySyncDesk > .buttonRow button'),
      visibleDmxGridSummaryCount: visibleCount('.dmxGridSummary'),
      visibleFixtureSetupEditorCount: visibleCount('.fixtureSetupEditor'),
      visibleUseProfileForPatchButtonCount: [...document.querySelectorAll('.fixtureSetupEditor button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'use profile for patch').length,
      visibleDuplicateFixtureButtonCount: [...document.querySelectorAll('.fixtureSetupEditor button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'duplicate fixture').length,
      controlModeTabCount: document.querySelectorAll('.controlModeTabs button').length,
      visibleLiveControlPanelCount: visibleCount('.liveControlPanel'),
      visibleControlStagePanelCount: visibleCount('.controlStagePanel'),
      visibleControlStageCount: visibleCount('.controlStage'),
      controlStageWidth: Math.round(document.querySelector('.controlStage')?.getBoundingClientRect().width ?? 0),
      controlStageHeight: Math.round(document.querySelector('.controlStage')?.getBoundingClientRect().height ?? 0),
      controlStageViewBoxAspect: (() => {
        const values = (document.querySelector('.controlStage')?.getAttribute('viewBox') || '')
          .trim()
          .split(/\\s+/)
          .map(Number);
        return values.length === 4 && values[3] > 0 ? values[2] / values[3] : 0;
      })(),
      controlStageGridCoverage: (() => {
        const stageRect = document.querySelector('.controlStage')?.getBoundingClientRect();
        const gridRect = document.querySelector('.controlStage .stageGrid')?.getBoundingClientRect();
        if (!stageRect || !gridRect || stageRect.width <= 0 || stageRect.height <= 0) return 0;
        return Math.min(gridRect.width / stageRect.width, gridRect.height / stageRect.height);
      })(),
      controlStageFixtureMinSize: Math.min(
        ...[...document.querySelectorAll('.controlStage .stageFixture')].map((fixture) => {
          const rect = fixture.getBoundingClientRect();
          return Math.min(rect.width, rect.height);
        }),
      ),
      visibleControlStageReferenceLabelCount: visibleCount('.controlStage .controlStageObject text, .controlStage .stageVideoSurface2d text'),
      controlWorkSurfaceOverflowCount: [...document.querySelectorAll(
        '.layoutControl .faders, .layoutControl .videoControlPanel, .layoutControl .videoOutputControlList, .layoutControl .videoLayerList'
      )]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => element.scrollHeight > element.clientHeight + 1 || element.scrollWidth > element.clientWidth + 1)
        .length,
      controlWorkSurfaceUnsafeOverflowCount: [...document.querySelectorAll(
        '.layoutControl .faders, .layoutControl .videoControlPanel, .layoutControl .videoOutputControlList, .layoutControl .videoLayerList'
      )]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => {
          const style = window.getComputedStyle(element);
          const verticalOverflow = element.scrollHeight > element.clientHeight + 1 && !['auto', 'scroll'].includes(style.overflowY);
          const horizontalOverflow = element.scrollWidth > element.clientWidth + 1 && !['auto', 'scroll'].includes(style.overflowX);
          return verticalOverflow || horizontalOverflow;
        })
        .length,
      controlWorkSurfaceOverflows: [...document.querySelectorAll(
        '.layoutControl .faders, .layoutControl .videoControlPanel, .layoutControl .videoOutputControlList, .layoutControl .videoLayerList'
      )]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            (element.scrollHeight > element.clientHeight + 1 || element.scrollWidth > element.clientWidth + 1);
        })
        .map((element) => ({
          className: element.className,
          client: [element.clientWidth, element.clientHeight],
          scroll: [element.scrollWidth, element.scrollHeight],
        })),
      effectTargetValue: (() => {
        const select = [...document.querySelectorAll('.effectEditor select')]
          .find((candidate) => [...candidate.options].some((option) => option.value === 'selection'));
        return select ? select.value : '';
      })(),
      effectTypeValue: (() => {
        const select = [...document.querySelectorAll('.effectEditor select')]
          .find((candidate) => [...candidate.options].some((option) => option.value === 'PositionWave'));
        return select ? select.value : '';
      })(),
      effectCommonAttributeValue: (() => {
        const select = [...document.querySelectorAll('.effectEditor select')]
          .find((candidate) => ![...candidate.options].some((option) => ['selection', 'fixture', 'group', 'video', 'PositionWave', 'Lfo'].includes(option.value)));
        return select ? select.value : '';
      })(),
      visibleFixtureEditSurfaceCount: visibleCount('.fixtureEditSurface'),
      visibleGroupControlBannerCount: visibleCount('.groupControlBanner'),
      visibleCuePanelCount: visibleCount('.cuePanel'),
      visibleCueLivePanelCount: visibleCount('.cuePanelLive'),
      visibleCueFormCount: visibleCount('.cueForm'),
      visibleCueEditOnlyCount: visibleCount('.cueEditOnly'),
      visibleCueLiveGoCount: visibleCount('.cueLiveGo'),
      visiblePlaybackDeskSurfaceCount: visibleCount('.playbackDeskSurface'),
      visibleProgrammerPanelCount: visibleCount('.programmerPanel'),
      visibleReferencePalettePanelCount: visibleCount('.referencePalettePanel'),
      visiblePlaybackExecutorPanelCount: visibleCount('.playbackExecutorPanel'),
      visiblePlaybackMasterCount: visibleCount('.playbackMasterControl input[type="range"]'),
      visibleTimelinePanelCount: visibleCount('.timelinePanel'),
      visibleTimelineDeskTabCount: visibleCount('.timelineDeskTabs button'),
      visibleTimelineShowSurfaceCount: visibleCount('.timelineShowSurface'),
      visibleTimelineAutomationSurfaceCount: visibleCount('.timelineAutomationSurface'),
      visibleEditDeskTabCount: visibleCount('.editDeskTabs button'),
      visibleEffectEditorCount: visibleCount('.effectEditor'),
      visibleNodeGraphPanelCount: visibleCount('.nodeGraphPanel'),
      nodeGraphAudioSourceOptionCount: [...document.querySelectorAll('.nodeGraphPanel option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase() === 'audio fft').length,
      nodeGraphLiveAudioOptionCount: [...document.querySelectorAll('.nodeGraphPanel option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase() === 'live input').length,
      visiblePanTiltPadCount: visibleCount('.panTiltPad'),
      visibleColorPlaneCount: visibleCount('.colorPlane'),
      visiblePositionReadoutCount: visibleCount('.positionReadoutStrip'),
      visibleColorReadoutCount: visibleCount('.colorReadoutStrip'),
      visibleAttributeTargetSummaryCount: visibleCount('.attributeTargetSummary'),
      visibleGroupAttributeTargetSummaryCount: visibleCount('.attributeTargetSummary.group'),
      effectTargetHintCount: visibleCount('.effectTargetHint'),
      effectTargetHintText: [...document.querySelectorAll('.effectTargetHint')]
        .map((element) => (element.textContent || '').trim().replace(/\\s+/g, ' '))
        .join(' | '),
      effectTargetMapSelectionOptionCount: [...document.querySelectorAll('.effectEditor option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase().startsWith('map selection')).length,
      visibleRawMonitorCount: visibleCount('.rawMonitor'),
      visibleVideoControlPanelCount: visibleCount('.videoControlPanel'),
      visibleVideoMasterControlCount: visibleCount('.videoMasterControls'),
      visibleVideoMasterFaderCount: visibleCount('.videoMasterFader input[type="range"]'),
      visibleVideoClipGridCount: visibleCount('.videoClipGridPanel'),
      visibleVideoClipPadCount: visibleCount('.videoClipPad'),
      visibleVideoClipTakeButtonCount: [...document.querySelectorAll('.videoClipPreviewBar button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'take').length,
      visibleVideoClipAudioButtonCount: visibleCount('.videoClipAudio'),
      visibleVideoProgramAudioToggleCount: visibleCount('.videoProgramAudioToggle input[type="checkbox"]'),
      visibleVideoAbDeckCount: visibleCount('.videoAbDeck'),
      visibleVideoDeckLoadButtonCount: visibleCount('.videoClipDeckLoad button'),
      visibleVideoRecordingBarCount: visibleCount('.videoRecordingBar'),
      visibleLiveAudioInputBarCount: visibleCount('.liveAudioInputBar'),
      visibleVideoOutputControlListCount: visibleCount('.videoOutputControlList'),
      visibleVideoOutputItemCount: visibleCount('.videoOutputControlItem'),
      visibleVideoOutputSelectedItemCount: visibleCount('.videoOutputControlItem.selected'),
      visibleVideoMixerOutputDeckCount: visibleCount('.videoMixerOutputDeck'),
      visibleVideoMixerOutputFaderCount: visibleCount('.videoControlPanelMixer .videoMixerOutputDeck input[type="range"]'),
      visibleVideoMixerOutputSelectButtonCount: [...document.querySelectorAll('.videoControlPanelMixer .videoMixerOutputDeck button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'sel').length,
      visibleVideoLayerListCount: visibleCount('.videoLayerList'),
      visibleVideoLayerItemCount: visibleCount('.videoLayerItem'),
      visibleVideoMixerLayerDeckCount: visibleCount('.videoMixerLayerDeck'),
      visibleVideoMixerLayerFaderCount: visibleCount('.videoControlPanelMixer .videoMixerLayerDeck input[type="range"]'),
      visibleVideoMixerLayerButtonCount: visibleCount('.videoControlPanelMixer .videoMixerLayerDeck button'),
      visibleVideoDeckPagerCount: visibleCount('.videoControlPanelMixer .deckPager'),
      visibleVideoMixerDiagnosticsCount: visibleCount('.videoMixerDiagnostics'),
      visibleVideoMixerSetupToolsCount: visibleCount('.videoMixerSetupTools'),
      visibleVideoMixerAutomationToolsCount: visibleCount('.videoMixerAutomationTools'),
      visibleOutputPanelCount: visibleCount('.output.controlPanel'),
      visibleSetupVideoPanelCount: visibleCount('.videoSetupPanel'),
      visibleSetupVideoOutputDeckCount: visibleCount('.videoSetupPanel .videoOutputDeck'),
      visibleSetupVideoOutputActiveDeckCount: visibleCount('.videoSetupPanel .videoOutputDeck.active'),
      visibleSetupVideoOutputDetailPaneCount: visibleCount('.videoSetupPanel .videoOutputDetailPane'),
      visibleVideoOutputMappingPanelCount: visibleCount('.videoSetupPanel .videoOutputMapping'),
      visibleVideoOutputBlendControlsCount: visibleCount('.videoSetupPanel .videoOutputBlendControls'),
      visibleProjectorMapEditorCount: visibleCount('.videoSetupPanel .projectorMapEditor'),
      visibleProjectorMapHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapHandle'),
      visibleProjectorKeystoneHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapKeystoneHandle'),
      visibleProjectorScaleHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapScaleHandle'),
      visibleProjectorRotateHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapRotateHandle'),
      visibleProjectorAspectModeButtonCount: visibleCount('.videoSetupPanel .projectorMapModeRow button'),
      visibleProjectorAspectPresetButtonCount: visibleCount('.videoSetupPanel .projectorMapAspectPresetRow button'),
      visibleProjectorResetPoseButtonCount: [...document.querySelectorAll('.videoSetupPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'reset pose').length,
      visibleMappingProjectorButtonCount: visibleCount('.mappingSelectionPanel button.projector'),
      visibleMappingProjectorControlsCount: visibleCount('.mappingSelectionPanel .mappingProjectorControls'),
      visibleMappingProjectorWarpGridCount: visibleCount('.mappingSelectionPanel .mappingProjectorWarpGrid'),
      visibleMappingProjectorActionButtonCount: visibleCount('.mappingSelectionPanel .mappingProjectorActionRow button'),
      visibleMappingProjectorResetPoseButtonCount: [...document.querySelectorAll('.mappingSelectionPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'reset pose').length,
      visibleStageVideoSurfaceCount: visibleCount('.mappingStageViewport .stageVideoSurface2d'),
      visibleTouchCuePanelCount: visibleCount('.touchCuePanel'),
      visibleTouchGoDeckCount: visibleCount('.touchGoDeck'),
      visibleTouchCuePadCount: visibleCount('.touchCuePadGrid .liveCuePad'),
      visibleTouchStagePanelCount: visibleCount('.touchStagePanel'),
      visibleTouchStageCount: visibleCount('.touchStage'),
      visibleTouchFixturePanelCount: visibleCount('.touchFixturePanel'),
      visibleTouchFixtureScrollerCount: visibleCount('.touchFixtureScroller'),
      visibleTouchRemotePanelCount: visibleCount('.touchRemotePanel'),
      visibleTouchRemoteUrlItemCount: visibleCount('.touchRemotePanel .remoteUrlItem'),
      visibleTouchRemoteCopyButtonCount: visibleCount('.touchRemotePanel .remoteUrlItem button'),
      visibleTouchRemoteOpenButtonCount: visibleCount('.touchRemotePanel .remoteUrlOpenButton'),
      visibleTouchVideoPanelCount: visibleCount('.touchVideoPanel'),
      visibleTouchVideoOutputDeckCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck'),
      visibleTouchVideoSelectedOutputDeckCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck.selected'),
      visibleTouchVideoOutputFaderCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck input[type="range"]'),
      visibleTouchVideoOutputButtonCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck button'),
      visibleTouchVideoOutputSelectButtonCount: visibleCount('.touchVideoPanel .touchVideoOutputSelect'),
      visibleTouchVideoDeckCount: visibleCount('.touchVideoPanel .touchVideoDeck'),
      visibleTouchVideoLayerFaderCount: visibleCount('.touchVideoPanel .touchVideoDeck input[type="range"]'),
      visibleTouchMasterGridCount: visibleCount('.touchMasterGrid'),
      touchUndersizedTargetCount: [...document.querySelectorAll('.layoutTouch button, .layoutTouch .buttonLink, .layoutTouch input:not([type="checkbox"]), .layoutTouch select')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width < 39.5 || rect.height < 39.5;
        }).length,
      touchUndersizedTargets: [...document.querySelectorAll('.layoutTouch button, .layoutTouch .buttonLink, .layoutTouch input:not([type="checkbox"]), .layoutTouch select')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' && (rect.width < 39.5 || rect.height < 39.5);
        })
        .slice(0, 40)
        .map((element) => {
          const rect = element.getBoundingClientRect();
          return element.tagName.toLowerCase() + '.' + (element.className || '-') + ' ' + Math.round(rect.width) + 'x' + Math.round(rect.height) + ' ' + (element.textContent || element.getAttribute('aria-label') || element.getAttribute('type') || '').trim().replace(/\\s+/g, ' ').slice(0, 42);
        }),
      touchMomentaryFlashPassed: window.__syndocalTouchMomentaryCheck?.passed === true,
      touchMomentaryFlashResult: window.__syndocalTouchMomentaryCheck ?? null,
      keyboardNavigationPassed: window.__syndocalKeyboardNavigationCheck?.passed === true,
      keyboardNavigationResult: window.__syndocalKeyboardNavigationCheck ?? null,
      timelineOverviewVisible: (() => {
        const overview = document.querySelector('.timelineOverview');
        if (!overview) {
          return false;
        }
        const rect = overview.getBoundingClientRect();
        return rect.width > 20 && rect.height > 20;
      })(),
      movedX,
      movedY,
      canWindowScrollX: movedX > 0,
      canWindowScrollY: movedY > 0
    };
  })()`);
}

function hasTimelineAutomationVisuals(result) {
  if (
    result.label.startsWith("control-live-cues-") ||
    result.label.startsWith("control-live-automation-") ||
    result.label.startsWith("control-live-playback-")
  ) {
    return true;
  }
  return (
    result.timelineOverviewVisible &&
    result.timelineAutomationRangeCount >= 2 &&
    result.timelineAutomationHandleCount >= 4 &&
    result.timelineAutomationKeyframeCount >= 4 &&
    result.timelineAutomationChipCount >= 4 &&
    result.timelineAutomationCurveCount >= 4 &&
    result.timelineAutomationValueInputCount >= 4 &&
    result.timelineAutomationEnabledToggleCount >= 2 &&
    result.timelineAutomationScopeCount >= 2 &&
    result.timelineAutomationBatchButtonCount >= 4 &&
    result.timelineAutomationGroupButtonCount >= 1 &&
    result.timelineAutomationRangeWidths.length >= 2 &&
    result.timelineAutomationRangeWidths.every((width) => width > 0)
  );
}

function hasExpectedProjectMenu(result) {
  if (!result.label.startsWith("project-menu-")) {
    return true;
  }
  return (
    result.visibleProjectMenuCount === 1 &&
    result.visibleProjectMenuItemCount >= 6 &&
    result.visibleProjectMenuShortcutCount >= 4 &&
    result.visibleRecentProjectMenuItemCount >= 5 &&
    result.visibleRecoveryProjectMenuItemCount >= 1 &&
    result.visibleUpdateMenuLabelCount === 1 &&
    result.visibleUpdateCheckButtonCount === 1 &&
    result.visibleUserTemplateMenuLabelCount === 1 &&
    result.visibleUserTemplateActionCount === 2
  );
}

function hasExpectedLocalization(result) {
  if (!result.label.startsWith("localization-ja-")) {
    return true;
  }
  return (
    result.documentLanguage === "ja" &&
    result.visibleProjectMenuCount === 1 &&
    result.visibleJapaneseLanguageLabelCount === 1 &&
    result.visibleJapaneseSaveButtonCount === 1 &&
    result.preservedUserFixtureLabelCount === 3 &&
    result.translatedUserFixtureCollisionCount === 0
  );
}

function hasExpectedKeyboardNavigation(result) {
  if (!result.label.startsWith("control-edit-keyboard-")) {
    return true;
  }
  return result.keyboardNavigationPassed;
}

function isContained(result) {
  const widthTolerance = result.documentClientWidth + 1;
  const heightTolerance = result.documentClientHeight + 1;
  const appWidthTolerance = (result.appClientWidth ?? result.documentClientWidth) + 1;
  const appHeightTolerance = (result.appClientHeight ?? result.documentClientHeight) + 1;
  const layoutWidthTolerance = (result.layoutClientWidth ?? result.documentClientWidth) + 1;
  const layoutHeightTolerance = (result.layoutClientHeight ?? result.documentClientHeight) + 1;
  const isHiddenOverflow = (value) => value === "hidden" || value === "clip";
  return (
    !result.canWindowScrollX &&
    !result.canWindowScrollY &&
    isHiddenOverflow(result.documentOverflowX) &&
    isHiddenOverflow(result.documentOverflowY) &&
    isHiddenOverflow(result.bodyOverflowX) &&
    isHiddenOverflow(result.bodyOverflowY) &&
    result.appPosition === "fixed" &&
    isHiddenOverflow(result.appOverflowX) &&
    isHiddenOverflow(result.appOverflowY) &&
    isHiddenOverflow(result.layoutOverflowX) &&
    isHiddenOverflow(result.layoutOverflowY) &&
    result.documentScrollWidth <= widthTolerance &&
    result.documentScrollHeight <= heightTolerance &&
    result.bodyScrollWidth <= widthTolerance &&
    result.bodyScrollHeight <= heightTolerance &&
    (result.appScrollWidth ?? 0) <= appWidthTolerance &&
    (result.appScrollHeight ?? 0) <= appHeightTolerance &&
    (result.layoutScrollWidth ?? 0) <= layoutWidthTolerance &&
    (result.layoutScrollHeight ?? 0) <= layoutHeightTolerance &&
    result.appRectWidth === result.innerWidth &&
    result.appRectHeight === result.innerHeight
  );
}

function hasExpectedControlModeSurface(result) {
  if (!result.label.startsWith("control-")) {
    return true;
  }
  if (result.controlModeTabCount < 3) {
    return false;
  }
  if (
    !result.label.startsWith("control-mixer-") &&
    (
      result.visibleLiveControlPanelCount !== 1 ||
      result.visibleControlStagePanelCount !== 1 ||
      result.visibleControlStageCount !== 1 ||
      result.controlStageWidth < 520 ||
      result.controlStageHeight < 190 ||
      result.controlStageViewBoxAspect < 2 ||
      result.controlStageGridCoverage < 0.95 ||
      result.controlStageFixtureMinSize < 12 ||
      result.visibleControlStageReferenceLabelCount > 1
    )
  ) {
    return false;
  }
  if (result.label.startsWith("control-edit-position-")) {
    return (
      result.visiblePanTiltPadCount >= 1 &&
      result.visiblePositionReadoutCount >= 1 &&
      result.visibleAttributeTargetSummaryCount >= 1 &&
      result.visibleGroupAttributeTargetSummaryCount >= 1 &&
      result.visibleEditDeskTabCount === 3 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleCuePanelCount === 0 &&
      result.visibleTimelinePanelCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-edit-color-")) {
    return (
      result.visibleColorPlaneCount >= 1 &&
      result.visibleColorReadoutCount >= 1 &&
      result.visibleAttributeTargetSummaryCount >= 1 &&
      result.visibleGroupAttributeTargetSummaryCount >= 1 &&
      result.visibleEditDeskTabCount === 3 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleCuePanelCount === 0 &&
      result.visibleTimelinePanelCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-edit-")) {
    if (result.label.startsWith("control-edit-effects-")) {
      return (
        result.visibleEditDeskTabCount === 3 &&
        result.visibleFixtureEditSurfaceCount === 0 &&
        result.visibleEffectEditorCount === 1 &&
        result.visibleNodeGraphPanelCount === 1 &&
        result.nodeGraphAudioSourceOptionCount === 1 &&
        result.nodeGraphLiveAudioOptionCount === 1 &&
        result.effectTargetHintCount >= 1 &&
        result.visibleRawMonitorCount === 0 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    if (result.label.startsWith("control-edit-dmx-")) {
      return (
        result.visibleEditDeskTabCount === 3 &&
        result.visibleFixtureEditSurfaceCount === 0 &&
        result.visibleEffectEditorCount === 0 &&
        result.visibleRawMonitorCount === 1 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    return (
      result.visibleFixtureEditSurfaceCount > 0 &&
      result.visibleEffectEditorCount === 0 &&
      result.visibleRawMonitorCount === 0 &&
      result.visibleEditDeskTabCount === 3 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.effectTargetHintCount === 0 &&
      result.effectTargetMapSelectionOptionCount >= 1 &&
      result.visibleCuePanelCount === 0 &&
      result.visibleTimelinePanelCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-live-")) {
    if (result.label.startsWith("control-live-playback-")) {
      return (
        result.visibleTimelineDeskTabCount === 4 &&
        result.visiblePlaybackDeskSurfaceCount === 1 &&
        result.visibleProgrammerPanelCount === 1 &&
        result.visibleReferencePalettePanelCount === 1 &&
        result.visiblePlaybackExecutorPanelCount === 1 &&
        result.visiblePlaybackMasterCount === 1 &&
        result.visibleCuePanelCount === 0 &&
        result.visibleTimelinePanelCount === 0 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    if (result.label.startsWith("control-live-cues-")) {
      return (
        result.visibleTimelineDeskTabCount === 4 &&
        result.visibleCuePanelCount === 1 &&
        result.visibleCueLivePanelCount === 1 &&
        result.visibleTimelinePanelCount === 0 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    if (result.label.startsWith("control-live-automation-")) {
      return (
        result.visibleTimelineDeskTabCount === 4 &&
        result.visibleCuePanelCount === 0 &&
        result.visibleTimelinePanelCount === 1 &&
        result.visibleTimelineShowSurfaceCount === 0 &&
        result.visibleTimelineAutomationSurfaceCount === 1 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    return (
      result.visibleCuePanelCount === 0 &&
      result.visibleCueLivePanelCount === 0 &&
      result.visibleCueFormCount === 0 &&
      result.visibleCueEditOnlyCount === 0 &&
      result.visibleTimelinePanelCount > 0 &&
      result.visibleTimelineDeskTabCount === 4 &&
      result.visibleTimelineShowSurfaceCount === 1 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleFixtureEditSurfaceCount === 0 &&
      result.visibleEffectEditorCount === 0 &&
      result.visibleRawMonitorCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-mixer-")) {
    return (
      result.visibleLiveControlPanelCount === 0 &&
      result.visibleControlStagePanelCount === 0 &&
      result.visibleControlStageCount === 0 &&
      result.visibleVideoControlPanelCount > 0 &&
      result.visibleVideoMasterControlCount > 0 &&
      result.visibleVideoMasterFaderCount > 0 &&
      result.visibleVideoClipGridCount > 0 &&
      result.visibleVideoClipPadCount > 0 &&
      result.visibleVideoClipTakeButtonCount > 0 &&
      result.visibleVideoClipAudioButtonCount > 0 &&
      result.visibleVideoProgramAudioToggleCount > 0 &&
      result.visibleVideoAbDeckCount > 0 &&
      result.visibleVideoDeckLoadButtonCount >= 2 &&
      result.visibleVideoRecordingBarCount > 0 &&
      result.visibleLiveAudioInputBarCount > 0 &&
      result.visibleVideoOutputControlListCount > 0 &&
      result.visibleVideoOutputItemCount > 0 &&
      result.visibleVideoOutputSelectedItemCount > 0 &&
      result.visibleVideoMixerOutputDeckCount >= result.visibleVideoOutputItemCount &&
      result.visibleVideoMixerOutputFaderCount >= result.visibleVideoOutputItemCount &&
      result.visibleVideoMixerOutputSelectButtonCount >= result.visibleVideoOutputItemCount &&
      result.visibleVideoLayerListCount > 0 &&
      result.visibleVideoLayerItemCount > 0 &&
      result.visibleVideoMixerLayerDeckCount >= result.visibleVideoLayerItemCount &&
      result.visibleVideoMixerLayerFaderCount >= result.visibleVideoLayerItemCount &&
      result.visibleVideoMixerLayerButtonCount >= 5 &&
      result.visibleVideoDeckPagerCount >= 2 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleVideoMixerDiagnosticsCount === 0 &&
      result.visibleVideoMixerSetupToolsCount === 0 &&
      result.visibleVideoMixerAutomationToolsCount === 0 &&
      result.visibleOutputPanelCount === 0
    );
  }
  return true;
}

function hasExpectedSetupSurface(result) {
  if (result.label.startsWith("setup-library-")) {
    return (
      result.visibleProfileLoadPanelCount >= 1 &&
      result.visibleLoadedProfileSummaryPanelCount >= 1 &&
      result.profileLoadPanelWidth >= 250 &&
      result.loadedProfileSummaryPanelWidth >= 760
    );
  }
  if (result.label.startsWith("setup-profiles-")) {
    return (
      result.visibleCustomProfileWorkbenchCount >= 1 &&
      result.customProfileAttributePaneWidth >= 460 &&
      result.customProfilePreviewDeskWidth >= 420 &&
      result.visibleCustomProfileActionCount >= 3 &&
      result.visibleCustomProfileDmxMapCount >= 1
    );
  }
  if (result.label.startsWith("setup-dmx-")) {
    return (
      result.visibleDmxOutputConfigPanelCount >= 1 &&
      result.visibleArtRdmPanelCount >= 1 &&
      result.visibleOutputDiagnosticsDeskCount >= 1 &&
      result.visibleLightingRuntimeDeskCount >= 1 &&
      result.dmxOutputConfigPanelWidth >= 250 &&
      result.outputDiagnosticsDeskWidth >= 420 &&
      result.lightingRuntimeDeskWidth >= 260
    );
  }
  if (result.label.startsWith("setup-midi-")) {
    return result.midiMappingEditorDeskWidth >= 430 && result.midiMappingListDeskWidth >= 430;
  }
  if (result.label.startsWith("setup-osc-")) {
    return result.oscMappingEditorDeskWidth >= 430 && result.oscMappingListDeskWidth >= 430;
  }
  if (result.label.startsWith("setup-remote-")) {
    return (
      result.remoteServerDeskWidth >= 360 &&
      result.remoteEndpointDeskWidth >= 500 &&
      result.visibleStandbySyncDeskCount === 1 &&
      result.visibleStandbyRoleOptionCount === 2 &&
      result.visibleStandbyActionButtonCount === 3
    );
  }
  if (result.label.startsWith("setup-patch-")) {
    return (
      result.visiblePatchActionRowCount >= 1 &&
      result.visiblePatchAutoButtonCount >= 1 &&
      result.visiblePatchPrimaryButtonCount >= 1 &&
      result.visiblePatchNextFreeButtonCount >= 1 &&
      result.visiblePatchFootprintCount >= 1 &&
      result.visibleDmxAddressGridCount >= 1 &&
      result.dmxAddressCellCount === 128 &&
      result.dmxAddressOccupiedCellCount > 0 &&
      result.dmxAddressPlannedCellCount > 0 &&
      result.visibleDmxFixtureBlockCount > 0 &&
      result.compactMappingStageWidth >= 420 &&
      result.compactMappingStageHeight >= 180 &&
      result.visibleDmxGridSummaryCount >= 1 &&
      result.visibleFixtureSetupEditorCount >= 1 &&
      result.visibleUseProfileForPatchButtonCount >= 1 &&
      result.visibleDuplicateFixtureButtonCount >= 1
    );
  }
  if (result.label.startsWith("setup-mapping-")) {
    return (
      result.mappingUseInEffectsButtonCount >= 1 &&
      result.mappingWaveDraftButtonCount >= 1 &&
      result.visibleMappingProjectorButtonCount >= 1 &&
      result.visibleMappingProjectorControlsCount >= 1 &&
      result.visibleMappingProjectorWarpGridCount >= 1 &&
      result.visibleMappingProjectorActionButtonCount >= 4 &&
      result.visibleMappingProjectorResetPoseButtonCount >= 1 &&
      result.visibleStageVideoSurfaceCount >= 1
    );
  }
  if (result.label.startsWith("setup-video-")) {
    return (
      result.visibleSetupVideoPanelCount >= 1 &&
      result.videoSetupSidebarWidth >= 220 &&
      result.videoSetupOutputDeskWidth >= 780 &&
      result.visibleSetupVideoOutputDeckCount >= 1 &&
      result.visibleSetupVideoOutputActiveDeckCount >= 1 &&
      result.visibleSetupVideoOutputDetailPaneCount >= 1 &&
      result.visibleVideoOutputMappingPanelCount >= 1 &&
      result.visibleVideoOutputBlendControlsCount >= 1 &&
      result.visibleProjectorMapEditorCount >= 1 &&
      result.visibleProjectorMapHandleCount >= 4 &&
      result.visibleProjectorKeystoneHandleCount >= 2 &&
      result.visibleProjectorScaleHandleCount >= 2 &&
      result.visibleProjectorRotateHandleCount >= 1 &&
      result.visibleProjectorAspectModeButtonCount >= 3 &&
      result.visibleProjectorAspectPresetButtonCount >= 3 &&
      result.visibleProjectorResetPoseButtonCount >= 1
    );
  }
  return true;
}

function hasExpectedMappingWaveDraft(result) {
  if (!result.label.startsWith("mapping-wave-draft-")) {
    return true;
  }
  return (
    result.visibleEffectEditorCount > 0 &&
    result.effectTargetHintCount >= 1 &&
    result.effectTargetHintText.toLowerCase().includes("map selection") &&
    result.effectTargetHintText.toLowerCase().includes("wave") &&
    result.effectTargetValue === "selection" &&
    result.effectTypeValue === "PositionWave" &&
    result.effectCommonAttributeValue.length > 0
  );
}

function hasExpectedMappingHotkeyHelp(result) {
  if (!result.label.startsWith("mapping-hotkey-help-")) {
    return true;
  }
  return result.visibleMappingHotkeyHelpCount === 1 && result.mappingHotkeyHelpKeyCount >= 20;
}

function hasExpectedTouchSurface(result) {
  if (!result.label.startsWith("touch-")) {
    return true;
  }
  return (
    result.visibleTouchCuePanelCount > 0 &&
    result.visibleTouchGoDeckCount > 0 &&
    result.visibleTouchCuePadCount >= 4 &&
    result.visibleTouchStagePanelCount > 0 &&
    result.visibleTouchStageCount > 0 &&
    result.visibleTouchFixturePanelCount > 0 &&
    result.visibleTouchFixtureScrollerCount > 0 &&
    result.visibleTouchRemotePanelCount > 0 &&
    result.visibleTouchRemoteUrlItemCount > 0 &&
    result.visibleTouchRemoteCopyButtonCount > 0 &&
    result.visibleTouchRemoteOpenButtonCount > 0 &&
    result.visibleTouchVideoPanelCount > 0 &&
    result.visibleTouchVideoOutputDeckCount > 0 &&
    result.visibleTouchVideoSelectedOutputDeckCount > 0 &&
    result.visibleTouchVideoOutputFaderCount >= result.visibleTouchVideoOutputDeckCount &&
    result.visibleTouchVideoOutputButtonCount >= 4 &&
    result.visibleTouchVideoOutputSelectButtonCount >= result.visibleTouchVideoOutputDeckCount &&
    result.visibleTouchVideoDeckCount > 0 &&
    result.visibleTouchVideoLayerFaderCount >= 2 &&
    result.visibleTouchMasterGridCount > 0 &&
    result.touchUndersizedTargetCount === 0 &&
    result.touchMomentaryFlashPassed
  );
}

async function checkTouchMomentaryFlash(client) {
  return await client.evaluate(`(async () => {
    const findFlashButton = () => [...document.querySelectorAll('.touchDimmerQuickRow button.momentary')]
      .find((candidate) => {
        const rect = candidate.getBoundingClientRect();
        const style = window.getComputedStyle(candidate);
        return !candidate.disabled && rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      });
    let button = findFlashButton();
    if (!button) {
      const dimmerCategory = [...document.querySelectorAll('.touchAttributeCategoryRail button')]
        .find((candidate) => (candidate.textContent || '').trim().toLowerCase().startsWith('dimmer'));
      dimmerCategory?.click();
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      button = findFlashButton();
    }
    if (!button) {
      window.__syndocalTouchMomentaryCheck = { passed: false, found: false, activeOnPress: false, inactiveOnRelease: false };
      return false;
    }
    button.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 1, pointerType: 'touch', isPrimary: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const activeOnPress = button.classList.contains('active');
    button.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 1, pointerType: 'touch', isPrimary: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const inactiveOnRelease = !button.classList.contains('active');
    window.__syndocalTouchMomentaryCheck = {
      passed: activeOnPress && inactiveOnRelease,
      found: true,
      activeOnPress,
      inactiveOnRelease,
    };
    return window.__syndocalTouchMomentaryCheck.passed;
  })()`);
}

async function runViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await seedViewportLocalStorage(client);
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);

  if (viewport.width === 1280) {
    await checkWorkspaceLayoutPersistence(client);
  }

  const results = [];
  await pressKey(client, "F2");
  await sleep(120);
  await checkKeyboardNavigation(client);
  results.push(await measure(client, `control-edit-keyboard-${viewport.width}x${viewport.height}`));
  await pressKey(client, "F1");
  await sleep(80);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  results.push(await measure(client, `setup-mapping-keyboard-${viewport.width}x${viewport.height}`));
  await client.evaluate(`document.querySelector('button[aria-label="Keyboard shortcut help"]')?.click()`);
  await sleep(120);
  results.push(await measure(client, `mapping-hotkey-help-${viewport.width}x${viewport.height}`));
  await pressKey(client, "Escape", "Escape");
  await sleep(80);
  await pressKey(client, "F3");
  await sleep(120);
  await checkTouchMomentaryFlash(client);
  results.push(await measure(client, `touch-keyboard-${viewport.width}x${viewport.height}`));
  await pressKey(client, "F1");
  await sleep(80);
  await clickVisibleByText(client, ".appMenuButton", "...");
  await sleep(120);
  results.push(await measure(client, `project-menu-${viewport.width}x${viewport.height}`));
  await clickVisibleByText(client, ".appMenuButton", "...");
  await sleep(80);
  if (viewport.width === 1366) {
    await client.evaluate(`window.localStorage.setItem('syndocal.uiScale.v1', '110')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
    await pressKey(client, "F2");
    await sleep(120);
    results.push(await measure(client, `interface-scale-110-${viewport.width}x${viewport.height}`));
    await client.evaluate(`window.localStorage.setItem('syndocal.uiScale.v1', '100')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
    await client.evaluate(`window.localStorage.setItem('syndocal.uiLocale.v1', 'ja')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
    await pressKey(client, "F1");
    await sleep(120);
    await clickVisibleByText(client, ".appMenuButton", "...");
    await sleep(120);
    results.push(await measure(client, `localization-ja-${viewport.width}x${viewport.height}`));
    await client.evaluate(`window.localStorage.setItem('syndocal.uiLocale.v1', 'en')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
  }
  await clickByText(client, "Setup");
  for (const setupTab of setupTabs) {
    await clickByText(client, setupTab.area);
    await clickByText(client, setupTab.tab);
    await sleep(180);
    if (screenshotDir) {
      mkdirSync(screenshotDir, { recursive: true });
      const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
      writeFileSync(join(screenshotDir, `setup-${setupTab.id}-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
    }
    results.push(await measure(client, `setup-${setupTab.id}-${viewport.width}x${viewport.height}`));
  }
  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  await clickByText(client, "Pick Visible");
  await sleep(120);
  await clickByText(client, "Wave Draft");
  await sleep(180);
  results.push(await measure(client, `mapping-wave-draft-${viewport.width}x${viewport.height}`));
  await clickByText(client, "Control");
  for (const controlTab of controlTabs) {
    await clickByText(client, controlTab.label);
    await sleep(320);
    if (screenshotDir) {
      mkdirSync(screenshotDir, { recursive: true });
      const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
      writeFileSync(join(screenshotDir, `control-${controlTab.id}-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
    }
    results.push(await measure(client, `control-${controlTab.id}-${viewport.width}x${viewport.height}`));
    if (controlTab.id === "edit") {
      await clickVisibleByText(client, ".attributeCategoryRail button", "Position");
      await sleep(120);
      results.push(await measure(client, `control-edit-position-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".attributeCategoryRail button", "Color");
      await sleep(120);
      results.push(await measure(client, `control-edit-color-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".editDeskTabs button", "Effects");
      await sleep(120);
      results.push(await measure(client, `control-edit-effects-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".editDeskTabs button", "DMX");
      await sleep(120);
      results.push(await measure(client, `control-edit-dmx-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".editDeskTabs button", "Attributes");
    }
    if (controlTab.id === "live") {
      await clickVisibleByText(client, ".timelineDeskTabs button", "Cues");
      await sleep(120);
      results.push(await measure(client, `control-live-cues-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".timelineDeskTabs button", "Automation");
      await sleep(120);
      results.push(await measure(client, `control-live-automation-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".timelineDeskTabs button", "Playback");
      await sleep(120);
      results.push(await measure(client, `control-live-playback-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".timelineDeskTabs button", "Show");
    }
  }
  await clickByText(client, "Touch");
  await sleep(180);
  await checkTouchMomentaryFlash(client);
  if (screenshotDir) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(join(screenshotDir, `touch-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
  }
  results.push(await measure(client, `touch-${viewport.width}x${viewport.height}`));
  return results;
}

async function runLargeShowViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await pressKey(client, "F1");
  await sleep(120);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(250);

  const stats = await client.evaluate(`(async () => {
    const list = document.querySelector('.virtualizedMappingFixtureList');
    if (!(list instanceof HTMLElement)) return null;
    const beforeCount = list.querySelectorAll('.mappingFixtureRowButton').length;
    const total = Number(list.getAttribute('aria-rowcount'));
    const viewportHeight = list.clientHeight;
    const scrollHeight = list.scrollHeight;
    list.scrollTop = list.scrollHeight;
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const afterRows = [...list.querySelectorAll('.mappingFixtureRowButton')];
    return {
      total,
      beforeCount,
      afterCount: afterRows.length,
      viewportHeight,
      scrollHeight,
      reachedLastFixture: afterRows.some((row) => (row.textContent || '').includes('Large Fixture 2000')),
    };
  })()`);
  const containment = await measure(client, `large-show-${viewport.width}x${viewport.height}`);
  return { stats, containment };
}

async function main() {
  const browser = findBrowser();
  if (!browser) {
    throw new Error("No Chrome/Edge/Chromium executable found. Set CHROME_PATH to run viewport containment checks.");
  }

  let viteProcess = null;
  let browserProcess = null;
  let client = null;
  const profileDir = mkdtempSync(join(tmpdir(), "syndocal-cdp-"));

  try {
    if (shouldStartVite) {
      viteProcess = startProcess(
        process.execPath,
        ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", String(vitePort), "--strictPort"],
        { cwd: appRoot },
      );
    }
    await waitForHttp(appUrl, "Syndocal dev server");

    browserProcess = startProcess(browser, [
      "--headless=new",
      "--disable-gpu",
      "--hide-scrollbars",
      "--disable-crash-reporter",
      "--disable-crashpad",
      `--remote-debugging-port=${cdpPort}`,
      `--user-data-dir=${profileDir}`,
      "--window-size=1366,768",
      "about:blank",
    ]);
    await waitForHttp(`http://127.0.0.1:${cdpPort}/json/version`, "Chrome DevTools Protocol");

    client = await createCdpClient();
    if (largeShowMode) {
      const result = await runLargeShowViewport(client, viewports[0]);
      const passed = Boolean(
        result.stats &&
        result.stats.total === 2_000 &&
        result.stats.beforeCount <= 20 &&
        result.stats.afterCount <= 20 &&
        result.stats.viewportHeight > 0 &&
        result.stats.scrollHeight > result.stats.viewportHeight &&
        result.stats.reachedLastFixture &&
        isContained(result.containment)
      );
      console.log(`${passed ? "pass" : "fail"} large-show virtualization ${JSON.stringify(result.stats)}`);
      if (!passed) throw new Error(`Large-show UI virtualization failed: ${JSON.stringify(result)}`);
      return;
    }

    const results = [];
    for (const viewport of viewports) {
      results.push(...(await runViewport(client, viewport)));
    }

    const failures = results.filter((result) => !isContained(result));
    const setupSurfaceFailures = results.filter((result) => !hasExpectedSetupSurface(result));
    const projectMenuFailures = results.filter((result) => !hasExpectedProjectMenu(result));
    const localizationFailures = results.filter((result) => !hasExpectedLocalization(result));
    const keyboardNavigationFailures = results.filter((result) => !hasExpectedKeyboardNavigation(result));
    const mappingHotkeyHelpFailures = results.filter((result) => !hasExpectedMappingHotkeyHelp(result));
    const mappingWaveDraftFailures = results.filter((result) => !hasExpectedMappingWaveDraft(result));
    const controlModeFailures = results.filter((result) => !hasExpectedControlModeSurface(result));
    const touchSurfaceFailures = results.filter((result) => !hasExpectedTouchSurface(result));
    const timelineAutomationFailures = shouldCheckTimelineAutomation
      ? results.filter((result) => result.label.startsWith("control-live-") && !hasTimelineAutomationVisuals(result))
      : [];
    const statusLineFailures = results.filter(
      (result) => result.visibleAppStatusLineCount !== 1 || !["info", "success", "warning", "error"].includes(result.appStatusTone),
    );
    for (const result of results) {
      const status = isContained(result) ? "pass" : "fail";
      const timelineSuffix = result.label.startsWith("control-live-")
        ? ` timelineAutomation=${result.timelineAutomationRangeCount}/${result.timelineAutomationHandleCount}/${result.timelineAutomationKeyframeCount}/${result.timelineAutomationChipCount}/${result.timelineAutomationCurveCount}/${result.timelineAutomationValueInputCount}/${result.timelineAutomationEnabledToggleCount}/${result.timelineAutomationScopeCount}/${result.timelineAutomationBatchButtonCount}/${result.timelineAutomationGroupButtonCount}`
        : "";
      const projectMenuSuffix = result.label.startsWith("project-menu-")
        ? ` projectMenu=${result.visibleProjectMenuCount}/${result.visibleProjectMenuItemCount}/${result.visibleProjectMenuShortcutCount}/${result.visibleRecentProjectMenuItemCount}/${result.visibleRecoveryProjectMenuItemCount}/${result.visibleUpdateMenuLabelCount}/${result.visibleUpdateCheckButtonCount}/${result.visibleUserTemplateMenuLabelCount}/${result.visibleUserTemplateActionCount}`
        : "";
      const keyboardSuffix = result.label.startsWith("control-edit-keyboard-")
        ? ` keyboard=${result.keyboardNavigationPassed ? 'pass' : 'fail'}`
        : "";
      const touchSuffix = result.label.startsWith("touch-")
        ? ` touch=${result.visibleTouchCuePanelCount}/${result.visibleTouchGoDeckCount}/${result.visibleTouchCuePadCount}/${result.visibleTouchStagePanelCount}/${result.visibleTouchStageCount}/${result.visibleTouchFixturePanelCount}/${result.visibleTouchFixtureScrollerCount}/${result.visibleTouchRemotePanelCount}/${result.visibleTouchRemoteUrlItemCount}/${result.visibleTouchRemoteCopyButtonCount}/${result.visibleTouchRemoteOpenButtonCount}/${result.visibleTouchVideoPanelCount}/${result.visibleTouchVideoOutputDeckCount}/${result.visibleTouchVideoSelectedOutputDeckCount}/${result.visibleTouchVideoOutputFaderCount}/${result.visibleTouchVideoOutputButtonCount}/${result.visibleTouchVideoOutputSelectButtonCount}/${result.visibleTouchVideoDeckCount}/${result.visibleTouchVideoLayerFaderCount}/${result.visibleTouchMasterGridCount}`
        : "";
      const mappingSuffix = result.label.startsWith("setup-mapping-")
        ? ` mapping=${result.mappingUseInEffectsButtonCount}/${result.mappingWaveDraftButtonCount}/${result.visibleMappingProjectorButtonCount}/${result.visibleMappingProjectorControlsCount}/${result.visibleMappingProjectorWarpGridCount}/${result.visibleMappingProjectorActionButtonCount}/${result.visibleMappingProjectorResetPoseButtonCount}/${result.visibleStageVideoSurfaceCount}`
        : "";
      const mappingHotkeyHelpSuffix = result.label.startsWith("mapping-hotkey-help-")
        ? ` mappingHelp=${result.visibleMappingHotkeyHelpCount}/${result.mappingHotkeyHelpKeyCount}`
        : "";
      const patchSuffix = result.label.startsWith("setup-patch-")
        ? ` patch=${result.visiblePatchActionRowCount}/${result.visiblePatchAutoButtonCount}/${result.visiblePatchPrimaryButtonCount}/${result.visiblePatchNextFreeButtonCount}/${result.visiblePatchFootprintCount}/${result.visibleDmxAddressGridCount}/${result.dmxAddressCellCount}/${result.dmxAddressOccupiedCellCount}/${result.dmxAddressPlannedCellCount}/${result.visibleDmxFixtureBlockCount}/${result.compactMappingStageWidth}x${result.compactMappingStageHeight}/${result.visibleDmxGridSummaryCount}/${result.visibleFixtureSetupEditorCount}/${result.visibleUseProfileForPatchButtonCount}/${result.visibleDuplicateFixtureButtonCount}`
        : "";
      const outputSetupSuffix = result.label.startsWith("setup-video-")
        ? ` outputSetup=${result.visibleSetupVideoPanelCount}/${result.visibleSetupVideoOutputDeckCount}/${result.visibleSetupVideoOutputActiveDeckCount}/${result.visibleSetupVideoOutputDetailPaneCount}/${result.visibleVideoOutputMappingPanelCount}/${result.visibleVideoOutputBlendControlsCount}/${result.visibleProjectorMapEditorCount}/${result.visibleProjectorMapHandleCount}/${result.visibleProjectorKeystoneHandleCount}/${result.visibleProjectorScaleHandleCount}/${result.visibleProjectorRotateHandleCount}/${result.visibleProjectorAspectModeButtonCount}/${result.visibleProjectorAspectPresetButtonCount}/${result.visibleProjectorResetPoseButtonCount}`
        : "";
      const waveDraftSuffix = result.label.startsWith("mapping-wave-draft-")
        ? ` waveDraft=${result.effectTargetValue}/${result.effectTypeValue}/${result.effectCommonAttributeValue || 'none'}`
        : "";
      const editVisualSuffix = result.label.startsWith("control-edit-position-") || result.label.startsWith("control-edit-color-")
        ? ` editVisual=${result.visiblePanTiltPadCount}/${result.visiblePositionReadoutCount}/${result.visibleColorPlaneCount}/${result.visibleColorReadoutCount}/${result.visibleGroupControlBannerCount}/${result.visibleAttributeTargetSummaryCount}/${result.visibleGroupAttributeTargetSummaryCount}`
        : "";
      const mixerSuffix = result.label.startsWith("control-mixer-")
        ? ` mixer=${result.visibleVideoOutputItemCount}/${result.visibleVideoOutputSelectedItemCount}/${result.visibleVideoMixerOutputDeckCount}/${result.visibleVideoMixerOutputFaderCount}/${result.visibleVideoMixerOutputSelectButtonCount}/${result.visibleVideoLayerItemCount}/${result.visibleVideoMixerLayerDeckCount}/${result.visibleVideoMixerLayerFaderCount}/${result.visibleVideoMixerLayerButtonCount}`
        : "";
      console.log(
        `${status} ${result.label} document=${result.documentScrollWidth}x${result.documentScrollHeight} app=${result.appScrollWidth}x${result.appScrollHeight} moved=${result.movedX},${result.movedY}${keyboardSuffix}${timelineSuffix}${touchSuffix}${projectMenuSuffix}${mappingSuffix}${mappingHotkeyHelpSuffix}${patchSuffix}${outputSetupSuffix}${waveDraftSuffix}${editVisualSuffix}${mixerSuffix}`,
      );
    }
    if (
      failures.length > 0 ||
      keyboardNavigationFailures.length > 0 ||
      setupSurfaceFailures.length > 0 ||
      projectMenuFailures.length > 0 ||
      localizationFailures.length > 0 ||
      mappingHotkeyHelpFailures.length > 0 ||
      mappingWaveDraftFailures.length > 0 ||
      controlModeFailures.length > 0 ||
      touchSurfaceFailures.length > 0 ||
      timelineAutomationFailures.length > 0 ||
      statusLineFailures.length > 0
    ) {
      console.error(
        JSON.stringify(
          {
            viewport: failures,
            keyboardNavigation: keyboardNavigationFailures,
            setupSurface: setupSurfaceFailures,
            projectMenu: projectMenuFailures,
            localization: localizationFailures,
            mappingHotkeyHelp: mappingHotkeyHelpFailures,
            mappingWaveDraft: mappingWaveDraftFailures,
            controlMode: controlModeFailures,
            touchSurface: touchSurfaceFailures,
            timelineAutomation: timelineAutomationFailures,
            statusLine: statusLineFailures,
          },
          null,
          2,
        ),
      );
      throw new Error(
        `${failures.length} viewport containment check(s), ${keyboardNavigationFailures.length} keyboard navigation check(s), ${setupSurfaceFailures.length} setup surface check(s), ${projectMenuFailures.length} project menu check(s), ${localizationFailures.length} localization check(s), ${mappingHotkeyHelpFailures.length} mapping hotkey help check(s), ${mappingWaveDraftFailures.length} mapping wave draft check(s), ${controlModeFailures.length} control mode surface check(s), ${touchSurfaceFailures.length} touch surface check(s), ${timelineAutomationFailures.length} timeline automation visual check(s), ${statusLineFailures.length} status line check(s) failed.`,
      );
    }
  } finally {
    client?.close();
    await stopProcess(browserProcess);
    await stopProcess(viteProcess);
    try {
      rmSync(profileDir, { recursive: true, force: true });
    } catch (error) {
      console.warn(`Viewport check passed, but cleanup could not remove ${profileDir}: ${error.message}`);
    }
  }
}

main().catch((error) => {
  console.error(error.message);
  process.exitCode = 1;
});
