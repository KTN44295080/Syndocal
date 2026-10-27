import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const viewports = [
  { width: 1280, height: 752, dpr: 2 },
  { width: 1366, height: 768, dpr: 1 },
  { width: 1920, height: 1080, dpr: 1 },
];
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function navigateBatch4InPage(surface) {
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const click = async (selector) => {
    const node = document.querySelector(selector);
    if (!(node instanceof HTMLElement) || node.disabled || !node.checkVisibility() || node.offsetWidth <= 0 || node.offsetHeight <= 0) {
      throw new Error("Missing visible Batch4 navigation: " + selector);
    }
    node.click();
    await settle();
  };
  if (surface === "perform" || surface === "videoLive") {
    await click('[data-workspace-option="touch"]');
    await click('[data-control-domain="video"]');
    if (surface === "videoLive") await click('[data-control-video-tools]');
  } else if (surface === "editVideo" || surface === "mediaHealth") {
    await click('[data-workspace-option="control"]');
    await click('[data-edit-domain-navigation] [data-control-mode-option="mixer"]');
    if (surface === "mediaHealth") {
      const rail = document.querySelector("[data-media-library-rail]");
      if (rail instanceof HTMLDetailsElement) rail.open = true;
      await settle();
      await click('[data-media-health-open]');
    }
  } else if (surface === "editLighting") {
    await click('[data-workspace-option="control"]');
    await click('[data-edit-domain-navigation] [data-control-mode-option="edit"]');
  } else if (surface === "timelinePlayback") {
    await click('[data-workspace-option="touch"]');
    await click('[data-control-domain="timeline"]');
  } else if (surface === "setupIo") {
    await click('[data-workspace-option="setup"]');
    await click(".setupAreaTabs button:nth-child(3)");
  } else if (surface === "setupLighting") {
    await click('[data-workspace-option="setup"]');
    await click(".setupAreaTabs button:nth-child(1)");
  } else if (surface === "projectMenu") {
    await click("[data-project-drop-surface]");
  } else if (surface === "workspaces") {
    await click("[data-project-drop-surface]");
    await click("[data-workspace-operations] .workspaceOperationsButton");
  }
  return { surface };
}

export function detectBatch4TruncationInPage(surface) {
  const rendered = (node) => {
    if (!(node instanceof HTMLElement) ||
        !node.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true }) ||
        node.offsetWidth <= 0 ||
        node.offsetHeight <= 0) return false;
    const rect = node.getBoundingClientRect();
    return rect.right > 0 && rect.bottom > 0 && rect.left < innerWidth && rect.top < innerHeight;
  };
  const ownText = (node) =>
    [...node.childNodes]
      .filter((child) => child.nodeType === Node.TEXT_NODE)
      .map((child) => child.textContent ?? "")
      .join(" ")
      .replace(/\s+/g, " ")
      .trim();
  const selector = (node) => {
    if (node.id) return "#" + CSS.escape(node.id);
    for (const attribute of [...node.attributes]) {
      if (attribute.name.startsWith("data-") && attribute.value) {
        return node.tagName.toLowerCase() + "[" + attribute.name + "=" + JSON.stringify(attribute.value) + "]";
      }
    }
    const classes = [...node.classList].slice(0, 2).map((name) => "." + CSS.escape(name)).join("");
    return node.tagName.toLowerCase() + classes;
  };
  const surfaceRoots = {
    perform: "[data-video-perform-layout]",
    videoLive: ".controlVideoToolsDialog[open]",
    editVideo: ".videoControlPanelMixer",
    editLighting: ".sceneMatrixPanel",
    timelinePlayback: ".controlPlaybackPanel",
    setupIo: ".setupIoPanel",
    setupLighting: ".layoutSetup",
    projectMenu: ".appProjectMenu",
    workspaces: ".workspaceOperationsPopover",
    mediaHealth: ".mediaHealthDialog[open]",
  };
  const root = document.querySelector(surfaceRoots[surface] ?? "body") ?? document.body;
  const nodes = [root, ...root.querySelectorAll("*")].filter(rendered);
  const normalizedText = (node) => (node.textContent ?? "").replace(/\s+/g, " ").trim();
  const completeValueVisibleElsewhere = (node, value) => {
    if (!value) return false;
    return nodes.some((candidate) => {
      if (candidate === node || candidate.contains(node) || node.contains(candidate)) return false;
      if (normalizedText(candidate) !== value) return false;
      const style = getComputedStyle(candidate);
      return candidate.scrollWidth <= candidate.clientWidth + 1 &&
        candidate.scrollHeight <= candidate.clientHeight + 1 &&
        style.textOverflow !== "ellipsis";
    });
  };
  const raw = [];
  for (const node of nodes) {
    const style = getComputedStyle(node);
    const widthDelta = node.scrollWidth - node.clientWidth;
    const widthOverflow = widthDelta > 1;
    const ellipsis = style.textOverflow === "ellipsis" && widthOverflow;
    if (!widthOverflow && !ellipsis) continue;
    const directText = ownText(node);
    const text = (directText || node.textContent || "").replace(/\s+/g, " ").trim();
    const interactive = node.matches("button,input,select,textarea,summary,a,[role=button],[role=tab],[role=menuitem],[role=option]") ||
      Boolean(node.closest("button,a,summary,label,[role=button],[role=tab],[role=menuitem],[role=option]"));
    const semantic = node.matches("label,dt,dd,output,strong,small,[role=status],[aria-label],[class*=label i],[class*=name i],[class*=title i],[class*=value i],[class*=state i]");
    const scrollOwner = ["auto", "scroll"].includes(style.overflowX);
    const userData = node.matches("[data-no-localize]") || Boolean(node.closest("[data-no-localize]"));
    const title = node.getAttribute("title") ?? node.closest("[title]")?.getAttribute("title") ?? "";
    const completeViaTitle = Boolean(title && text && title.includes(text));
    const completeVisibleElsewhere = userData && completeViaTitle && completeValueVisibleElsewhere(node, title);
    const primaryLiveName = Boolean(
      node.matches(".videoPerformClipTitle,.videoPerformLayerName,.sceneMatrixCuePrimaryRow [data-no-localize],.sceneMatrixCueListTabs [data-no-localize],.sceneMatrixBankJumpStrip [data-no-localize]") ||
      node.closest(".videoPerformClipTrigger,.sceneMatrixCuePrimaryRow button,.sceneMatrixCueListTabs button,.sceneMatrixBankJumpStrip button")
    );
    const titleExempt = userData && completeViaTitle && (!primaryLiveName || completeVisibleElsewhere);
    const rect = node.getBoundingClientRect();
    const visuallyHidden = node.classList.contains("srOnly") ||
      node.getAttribute("aria-hidden") === "true" ||
      style.clipPath !== "none" ||
      (style.clip !== "auto" && style.clip !== "rect(auto, auto, auto, auto)") ||
      (rect.width <= 2 && rect.height <= 2);
    const textOwnsWidthOverflow = Boolean(directText) && widthDelta > 3;
    const actionable = Boolean(text) &&
      !visuallyHidden &&
      !scrollOwner &&
      !titleExempt &&
      (interactive || semantic) &&
      (ellipsis || textOwnsWidthOverflow);
    raw.push({
      selector: selector(node),
      text: text.slice(0, 240),
      clientWidth: node.clientWidth,
      scrollWidth: node.scrollWidth,
      widthDelta,
      textOverflow: style.textOverflow,
      overflowX: style.overflowX,
      interactive,
      semantic,
      userData,
      completeViaTitle,
      completeVisibleElsewhere,
      primaryLiveName,
      titleExempt,
      visuallyHidden,
      actionable,
      reason: actionable ? (ellipsis ? "ellipsis" : "own-text-width-overflow") : null,
      rect: { x: Math.round(rect.x), y: Math.round(rect.y), width: Math.round(rect.width), height: Math.round(rect.height) },
    });
  }
  const actionable = raw.filter((finding) => finding.actionable);
  return {
    surface,
    rawCount: raw.length,
    actionableCount: actionable.length,
    raw,
    actionable,
    observed: { locale: document.documentElement.lang, width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
  };
}

export function detectPerformClipNameReadabilityInPage() {
  const titles = [...document.querySelectorAll("[data-video-perform-layout] .videoPerformMatrixCell.occupied .videoPerformClipTitle")];
  const clips = titles.map((node) => {
    const text = (node.textContent ?? "").replace(/\s+/g, " ").trim();
    const requiredVisibleCharacters = Math.min(12, text.length);
    const bounds = node.getBoundingClientRect();
    const textNode = [...node.childNodes].find((child) => child.nodeType === Node.TEXT_NODE);
    let visibleCharacters = 0;
    if (textNode && bounds.width > 0 && bounds.height > 0) {
      for (let index = 0; index < textNode.textContent.length; index += 1) {
        const range = document.createRange();
        range.setStart(textNode, index);
        range.setEnd(textNode, index + 1);
        const rects = [...range.getClientRects()];
        const visible = rects.some((rect) =>
          rect.width >= 0 &&
          rect.height > 0 &&
          rect.left >= bounds.left - 1 &&
          rect.right <= bounds.right + 1 &&
          rect.top >= bounds.top - 1 &&
          rect.bottom <= bounds.bottom + 1);
        range.detach();
        if (!visible) break;
        visibleCharacters = index + 1;
      }
    }
    return {
      text,
      visibleCharacters,
      requiredVisibleCharacters,
      passed: visibleCharacters >= requiredVisibleCharacters,
      clientWidth: node.clientWidth,
      clientHeight: node.clientHeight,
      scrollWidth: node.scrollWidth,
      scrollHeight: node.scrollHeight,
    };
  });
  return {
    clipCount: clips.length,
    clips,
    passed: clips.length > 0 && clips.every((clip) => clip.passed),
  };
}

export async function runUiDensityBatch4Checks({ CdpClient, cdpPort, appUrl, screenshotDir, waitForApp }) {
  if (!screenshotDir) throw new Error("--ui-density-batch4-only requires SYNDOCAL_VIEWPORT_SCREENSHOT_DIR");
  mkdirSync(screenshotDir, { recursive: true });
  const version = await (await fetch("http://127.0.0.1:" + cdpPort + "/json/version")).json();
  const browser = new CdpClient(version.webSocketDebuggerUrl);
  await browser.connect();
  const results = [];
  const surfaces = [
    ["perform", "vj-bank"],
    ["editVideo", "vj-bank"],
    ["editLighting", "scene-matrix"],
    ["timelinePlayback", "timeline-layered"],
    ["setupIo", "setup-io"],
    ["setupLighting", "patch"],
    ["projectMenu", "vj-bank"],
    ["workspaces", "vj-bank"],
    ["mediaHealth", "vj-bank"],
    ["videoLive", "vj-bank"],
  ];
  try {
    for (const locale of ["en", "ja"]) {
      for (const viewport of viewports) {
        const label = "ui-density-batch4-" + locale + "-" + viewport.width + "x" + viewport.height + "-dpr" + viewport.dpr;
        const result = { label, locale, viewport, surfaces: {}, screenshots: [], checks: {}, totalRaw: 0, totalActionable: 0 };
        let contextId = null;
        let page = null;
        try {
          contextId = (await browser.send("Target.createBrowserContext", { disposeOnDetach: true })).browserContextId;
          const targetId = (await browser.send("Target.createTarget", { url: "about:blank", browserContextId: contextId })).targetId;
          let target = null;
          for (let attempt = 0; attempt < 100 && !target; attempt += 1) {
            target = (await (await fetch("http://127.0.0.1:" + cdpPort + "/json/list")).json()).find((entry) => entry.id === targetId);
            if (!target) await pause(25);
          }
          if (!target?.webSocketDebuggerUrl) throw new Error("Fresh context target has no CDP endpoint");
          page = new CdpClient(target.webSocketDebuggerUrl);
          await page.connect();
          await page.send("Page.enable");
          await page.send("Runtime.enable");
          await page.send("Emulation.setDeviceMetricsOverride", { width: viewport.width, height: viewport.height, deviceScaleFactor: viewport.dpr, mobile: false });
          await page.send("Page.addScriptToEvaluateOnNewDocument", { source:
            "localStorage.setItem('syndocal.uiLocale.v1'," + JSON.stringify(locale) + ");" +
            "localStorage.setItem('syndocal.uiScale.v1','100');localStorage.setItem('syndocal.sceneFxEditorWide.v1','0');" });

          for (const [surface, fixture] of surfaces) {
            const url = new URL(appUrl);
            url.searchParams.set("syndocalViewportFixture", fixture);
            await page.send("Page.navigate", { url: url.toString() });
            await waitForApp(page);
            await page.evaluate("(" + navigateBatch4InPage.toString() + ")(" + JSON.stringify(surface) + ")");
            await pause(120);
            const finding = await page.evaluate("(" + detectBatch4TruncationInPage.toString() + ")(" + JSON.stringify(surface) + ")");
            result.surfaces[surface] = finding;
            result.totalRaw += finding.rawCount;
            result.totalActionable += finding.actionableCount;
            if (surface === "perform") {
              const clipNameReadability = await page.evaluate("(" + detectPerformClipNameReadabilityInPage.toString() + ")()");
              result.clipNameReadability = clipNameReadability;
              result.checks.clipNamesReadableAtGlance = clipNameReadability.passed;
            }
            const filename = label + "-" + surface + ".png";
            const shot = await page.send("Page.captureScreenshot", { format: "png", fromSurface: true, captureBeyondViewport: false });
            writeFileSync(join(screenshotDir, filename), shot.data, "base64");
            result.screenshots.push(filename);
          }
          const observed = await page.evaluate("({locale:document.documentElement.lang,width:innerWidth,height:innerHeight,dpr:devicePixelRatio})");
          result.checks.localeAndViewportExact = observed.locale === locale && observed.width === viewport.width && observed.height === viewport.height && observed.dpr === viewport.dpr;
          result.checks.noActionableTruncation = result.totalActionable === 0;
          result.checks.scenarioCompleted = true;
        } catch (error) {
          result.error = error instanceof Error ? error.stack : String(error);
          result.checks.scenarioCompleted = false;
        } finally {
          result.runtimeExceptions = page?.runtimeExceptions ?? [];
          result.checks.noRuntimeExceptions = result.runtimeExceptions.length === 0;
          result.passed = Object.keys(result.checks).length > 1 && Object.values(result.checks).every(Boolean);
          writeFileSync(join(screenshotDir, label + ".json"), JSON.stringify(result, null, 2) + "\n");
          results.push(result);
          console.log((result.passed ? "pass " : "fail ") + label + " actionable=" + result.totalActionable);
          page?.close();
          if (contextId) await browser.send("Target.disposeBrowserContext", { browserContextId: contextId });
        }
      }
    }
  } finally {
    browser.close();
    writeFileSync(join(screenshotDir, "ui-density-batch4-results.json"), JSON.stringify(results, null, 2) + "\n");
  }
  const failures = results.filter((result) => !result.passed);
  if (results.length !== 6 || failures.length) {
    throw new Error("Batch4 density failed: " + JSON.stringify(failures.map(({ label, totalActionable, error }) => ({ label, totalActionable, error }))));
  }
  console.log("Batch4 density passed: 6 fresh contexts; 10 production fixture surfaces; zero operator-actionable truncation.");
}
