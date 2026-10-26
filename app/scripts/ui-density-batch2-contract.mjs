import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const densityViewports = [
  { width: 1280, height: 752, dpr: 2 },
  { width: 1366, height: 768, dpr: 1 },
  { width: 1920, height: 1080, dpr: 1 },
];
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function selectDensitySurfaceInPage(surface) {
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const activate = async (selector) => {
    const button = document.querySelector(selector);
    if (!(button instanceof HTMLButtonElement) || button.disabled || button.offsetWidth <= 0 || button.offsetHeight <= 0) {
      throw new Error("Missing visible navigation control: " + selector);
    }
    button.click(); await settle();
    return { selector, label: button.textContent.trim() };
  };
  const path = [await activate('[data-workspace-option="' + (surface === "sources" ? "control" : "setup") + '"]')];
  if (surface === "sources") path.push(await activate('[data-control-mode-option="live"]'));
  // setupAreas in uiModes.ts has stable Lighting, Video, I/O, Security order;
  // these buttons currently have no data-id. Use their real onClick handlers
  // after fixture initialization, which may override saved workspace state.
  else path.push(await activate('.setupAreaTabs > button:nth-child(' + (surface === "video" ? 2 : 4) + ')'));
  return path;
}

// The production browser fixture supplies data only. This probe does not invoke
// output, placement, media verification, or authority mutations. Existing
// projection controls and production Timeline click/drag contracts stay separate.
export async function exerciseBatch2DensityInPage(phase) {
  const checks = {}, evidence = {};
  const check = (name, condition, detail) => {
    checks[name] = Boolean(condition);
    if (detail !== undefined) evidence[name] = detail;
  };
  const q = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const rect = (node) => {
    const b = node.getBoundingClientRect();
    return { x: b.x, y: b.y, width: b.width, height: b.height, right: b.right, bottom: b.bottom };
  };
  const identity = (node) => node.id || node.getAttribute("data-video-output-detail")
    || node.getAttribute("data-video-setup-tool") || node.className || node.tagName;
  const rendered = (node) => {
    if (!(node instanceof HTMLElement) || !node.isConnected || node.offsetWidth <= 0 || node.offsetHeight <= 0) return false;
    for (let current = node; current instanceof HTMLElement; current = current.parentElement) {
      const style = getComputedStyle(current);
      if (current.hidden || style.display === "none" || /hidden|collapse/.test(style.visibility)) return false;
      if (current instanceof HTMLDetailsElement && current !== node && !current.open
        && !q(":scope > summary", current)?.contains(node)) return false;
    }
    return true;
  };
  const isVideo = phase.startsWith("video-");
  const isSources = phase.startsWith("sources-");
  const scope = q(isVideo ? ".videoSetupPanel" : isSources ? "[data-timeline-source-shelf]" : "[data-agent-authority-panel]");
  if (!rendered(scope)) throw new Error("Missing density surface: " + phase);
  const owner = () => isVideo ? q("[data-video-setup-scroll]", scope)
    : isSources ? q("#timeline-source-context-panel-sources", scope) : scope;
  const ancestors = [];
  for (let node = scope.parentElement; node; node = node.parentElement) ancestors.push(node);
  const nodes = () => [...new Set([scope, ...all("*", scope), ...ancestors])].filter(rendered);
  const scrolls = (node, axis) => /(auto|scroll)/.test(getComputedStyle(node)[axis === "x" ? "overflowX" : "overflowY"])
    && (axis === "x" ? node.scrollWidth > node.clientWidth + 1 : node.scrollHeight > node.clientHeight + 1);
  const contentBox = (node) => {
    const b = rect(node), s = getComputedStyle(node);
    return { x: b.x + node.clientLeft + parseFloat(s.paddingLeft),
      y: b.y + node.clientTop + parseFloat(s.paddingTop),
      right: b.x + node.clientLeft + node.clientWidth - parseFloat(s.paddingRight),
      bottom: b.y + node.clientTop + node.clientHeight - parseFloat(s.paddingBottom) };
  };
  const inspect = (node) => {
    if (!rendered(node)) return { rendered: false, target: node ? identity(node) : "missing" };
    const box = rect(node), clip = { x: 0, y: 0, right: innerWidth, bottom: innerHeight };
    for (let parent = node.parentElement; parent; parent = parent.parentElement) {
      const b = rect(parent), s = getComputedStyle(parent);
      if (s.overflowX !== "visible") {
        clip.x = Math.max(clip.x, b.x + parent.clientLeft);
        clip.right = Math.min(clip.right, b.x + parent.clientLeft + parent.clientWidth);
      }
      if (s.overflowY !== "visible") {
        clip.y = Math.max(clip.y, b.y + parent.clientTop);
        clip.bottom = Math.min(clip.bottom, b.y + parent.clientTop + parent.clientHeight);
      }
    }
    const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
    return { rendered: true, target: identity(node), box, clip,
      contained: box.x >= clip.x - 1 && box.right <= clip.right + 1 && box.y >= clip.y - 1 && box.bottom <= clip.bottom + 1,
      hit: hit === node || node.contains(hit), fontSize: parseFloat(getComputedStyle(node).fontSize),
      disabled: Boolean(node.disabled) };
  };
  const reachable = (state) => state.rendered && state.contained && state.hit && state.onlyOwnerVerticalScroll !== false;
  const reveal = async (node) => {
    const scrollOwner = owner();
    const before = nodes().map((element) => ({ element, left: element.scrollLeft, top: element.scrollTop }));
    // Only the contracted owner may move. Never rescue a clipped control with
    // scrollIntoView, a child scrollport, or horizontal scrolling.
    if (rendered(node) && scrollOwner?.contains(node)) {
      for (let attempt = 0; attempt < 3; attempt += 1) {
        const b = rect(node), p = rect(scrollOwner);
        const top = p.y + scrollOwner.clientTop + 2;
        const bottom = p.y + scrollOwner.clientTop + scrollOwner.clientHeight - 2;
        if (b.y < top) scrollOwner.scrollTop -= top - b.y;
        else if (b.bottom > bottom) scrollOwner.scrollTop += b.bottom - bottom;
        await settle();
        if (reachable(inspect(node))) break;
      }
    }
    const state = inspect(node);
    state.onlyOwnerVerticalScroll = before.every(({ element, left, top }) => element.scrollLeft === left
      && (element === scrollOwner || element.scrollTop === top));
    state.scrollMovement = before.filter(({ element, left, top }) => element.scrollLeft !== left || element.scrollTop !== top)
      .map(({ element, left, top }) => ({ target: identity(element), beforeLeft: left, beforeTop: top,
        afterLeft: element.scrollLeft, afterTop: element.scrollTop, owner: element === scrollOwner }));
    return state;
  };
  const click = async (selector, name) => {
    const node = q(selector, scope), state = await reveal(node);
    check(name + "Reachable", reachable(state) && !node?.disabled, state);
    if (!(node instanceof HTMLElement) || node.disabled) throw new Error("Missing enabled density control: " + selector);
    node.click(); await settle();
    return node;
  };
  const disclosure = async (selector, open, name) => {
    const node = q(selector, scope);
    if (!(node instanceof HTMLDetailsElement)) throw new Error("Missing disclosure: " + selector);
    if (node.open !== open) await click(selector + " > summary", name);
    check(name + "State", node.open === open);
    return node;
  };
  const restoreTop = async () => { if (owner()) owner().scrollTop = 0; await settle(); };
  const reachControls = async (root, name) => {
    const controls = all("button,input:not([type=hidden]),select,summary", root).filter(rendered);
    const chosen = [...new Set([controls[0], controls.at(-1)])].filter(Boolean);
    const states = [];
    for (const control of chosen) states.push(await reveal(control));
    check(name + "FirstAndLastReachableThroughOwner", states.length > 0 && states.every(reachable), states);
    return states;
  };
  const fullWidth = (node, name) => {
    const b = rect(node), available = contentBox(node.parentElement);
    check(name, Math.abs(b.x - available.x) <= 2 && Math.abs(b.right - available.right) <= 2, { box: b, available });
  };
  const headingEvidence = async () => {
    const headings = all(isSources ? ".timelineExternalSourceShelfHeader h3" : "h2,h3,details > summary", scope).filter(rendered);
    const result = [];
    for (const heading of headings) {
      const state = await reveal(heading);
      const range = document.createRange(); range.selectNodeContents(heading);
      const text = range.getBoundingClientRect(), box = rect(heading);
      const lineTops = [...new Set([...range.getClientRects()].filter((b) => b.width > 0 && b.height > 0).map((b) => Math.round(b.top)))];
      // A line box may be shorter than font ink. Check ink against actual
      // clipping ancestors, not against the line-height rectangle itself.
      const clip = { ...state.clip };
      const s = getComputedStyle(heading);
      if (s.overflowX !== "visible") { clip.x = Math.max(clip.x, box.x + heading.clientLeft); clip.right = Math.min(clip.right, box.x + heading.clientLeft + heading.clientWidth); }
      if (s.overflowY !== "visible") { clip.y = Math.max(clip.y, box.y + heading.clientTop); clip.bottom = Math.min(clip.bottom, box.y + heading.clientTop + heading.clientHeight); }
      result.push({ text: heading.textContent.trim(), lines: lineTops.length, fontSize: state.fontSize,
        reachable: reachable(state), fits: text.left >= Math.max(box.x, clip.x) - 1 && text.right <= Math.min(box.right, clip.right) + 1
          && text.top >= clip.y - 1 && text.bottom <= clip.bottom + 1, box, clip,
        textBounds: { x: text.x, y: text.y, width: text.width, height: text.height, right: text.right, bottom: text.bottom },
        lineHeight: s.lineHeight, overflowX: s.overflowX, overflowY: s.overflowY });
    }
    check("headingsSingleLineAndUnclipped", result.length > 0 && result.every((item) => item.lines === 1 && item.reachable && item.fits), result);
  };
  const measure = async () => {
    const vertical = nodes().filter((node) => scrolls(node, "y"));
    const horizontal = nodes().filter((node) => scrolls(node, "x"));
    const nested = vertical.filter((node) => vertical.some((parent) => parent !== node && parent.contains(node)));
    const describe = (node) => ({ target: identity(node), width: node.clientWidth, scrollWidth: node.scrollWidth,
      height: node.clientHeight, scrollHeight: node.scrollHeight, left: node.scrollLeft, top: node.scrollTop });
    const roots = [document.documentElement, document.body, q(".app"), q(".layout")].filter(Boolean);
    check("viewportOverflowZero", scrollX === 0 && scrollY === 0 && roots.every((node) => node.scrollWidth <= node.clientWidth + 1
      && node.scrollHeight <= node.clientHeight + 1 && node.scrollLeft === 0 && node.scrollTop === 0), roots.map(describe));
    check("noHorizontalScroll", horizontal.length === 0 && scope.scrollWidth <= scope.clientWidth + 1
      && (!owner() || owner().scrollWidth <= owner().clientWidth + 1), horizontal.map(describe));
    check("noNestedScroll", nested.length === 0, { vertical: vertical.map(describe), nested: nested.map(describe) });
    check("onlyContractedScrollOwner", Boolean(owner()) && vertical.every((node) => node === owner()), vertical.map(describe));
    check("scopeContained", inspect(scope).contained, inspect(scope));
    if (owner()) check("scrollOwnerContained", inspect(owner()).contained, inspect(owner()));
    await headingEvidence();
    evidence.controlDimensions = all("button,summary,input:not([type=checkbox]):not([type=range]),select", scope).filter(rendered).map((node) => ({
      target: identity(node), text: (node.textContent ?? "").trim().slice(0, 80), width: node.offsetWidth, height: node.offsetHeight,
      fontSize: parseFloat(getComputedStyle(node).fontSize) }));
    await restoreTop();
  };

  if (isVideo) {
    const config = '[data-video-output-detail="configuration"]';
    const correction = '[data-video-output-detail="correction"]';
    if (phase === "video-default") {
      check("detailsInitiallyClosed", [q(config, scope), q(correction, scope)].every((node) => node instanceof HTMLDetailsElement && !node.open));
      const tools = all("[data-video-setup-tool]", scope);
      check("creationToolsInitiallyClosed", tools.length === 2 && tools.every((node) => node instanceof HTMLDetailsElement && !node.open));
      const action = q('.videoOutputActionDock [data-action="set-display-window-open"]', scope);
      check("primaryOutputActionInitiallyReachable", reachable(inspect(action)), inspect(action));
      check("primaryOutputActionSizePreserved", action?.offsetHeight >= 40, inspect(action));
      check("selectedOutputRetained", all(".videoOutputDeck.active", scope).length === 1);
    } else if (phase === "video-configuration" || phase === "video-configuration-restored") {
      await disclosure(config, true, "configuration");
      check("configurationExcludesCorrection", !q(correction, scope).open);
      fullWidth(q(config, scope), "configurationUsesFullWidth");
      const input = q("[data-video-output-config] input", scope);
      check("configurationLabelReachable", reachable(await reveal(input)));
      if (phase === "video-configuration") {
        input.value = "Density QA output draft";
        input.dispatchEvent(new Event("input", { bubbles: true })); await settle();
        window.__syndocalBatch2VideoDraft = input.value;
      } else check("configurationDraftSurvivesDisclosureSwitch", input.value === window.__syndocalBatch2VideoDraft);
      await reachControls(q(config, scope), "configuration");
    } else if (phase === "video-correction") {
      await disclosure(correction, true, "correction");
      check("correctionExcludesConfiguration", !q(config, scope).open);
      fullWidth(q(correction, scope), "correctionUsesFullWidth");
      const mapping = q(".videoOutputMapping", scope);
      check("mappingPresent", rendered(mapping));
      await reachControls(mapping, "mapping");
      const projector = q(".projectorMapSurface", scope);
      const available = contentBox(owner()), p = projector ? rect(projector) : null;
      check("projectorWithinOwnerWidth", p && p.width > 0 && p.x >= available.x - 1 && p.right <= available.right + 1, p);
      await reachControls(q(".videoOutputActionDock", scope), "outputActionDock");
      const preview = q(".videoOutputPreviewCard", scope);
      if (preview) {
        const previewState = await reveal(preview);
        check("previewReachableThroughOwner", reachable(previewState), previewState);
      }
    } else if (phase === "video-output-tools" || phase === "video-composition-tools") {
      await disclosure(config, false, "closeConfiguration");
      await disclosure(correction, false, "closeCorrection");
      const kind = phase === "video-output-tools" ? "outputs" : "compositions";
      for (const other of ["outputs", "compositions"]) await disclosure('[data-video-setup-tool="' + other + '"]', other === kind, other + "Tool");
      const tool = q('[data-video-setup-tool="' + kind + '"]', scope);
      fullWidth(tool, "creationToolUsesFullWidth");
      await reachControls(tool, "creationTool");
    } else throw new Error("Unknown Video density phase: " + phase);
    await measure();
  } else if (isSources) {
    const shelfBox = rect(scope), context = scope.closest('[data-workspace-pane="lower-right"]');
    const available = context ? contentBox(context) : null;
    check("fillsAssignedContextHeight", available && Math.abs(shelfBox.y - available.y) <= 2
      && Math.abs(shelfBox.bottom - available.bottom) <= 2, { shelf: shelfBox, available });
    fullWidth(scope, "shelfUsesFullWidth");
    if (phase === "sources-scenes") {
      await click('[data-timeline-source-shelf-filter="lighting"]', "lightingFilter");
      check("lightingFilterSelected", q('[data-timeline-source-shelf-filter="lighting"]', scope).getAttribute("aria-pressed") === "true");
      const banks = all("[data-timeline-source-shelf-bank]", scope).filter(rendered);
      const cards = all('[data-timeline-external-source="scene"]', scope).filter(rendered);
      check("sceneCardsAndBanksPresent", banks.length > 0 && cards.length > 0, { banks: banks.length, cards: cards.length });
      check("sceneCardsRetainDragAffordance", cards.every((node) => node.draggable && !node.disabled && node.dataset.timelineExternalSourceKind === "Lighting"));
      const boxes = banks.map(rect);
      check("banksUseOneColumn", boxes.length > 0 && boxes.every((b, index) => index === 0 || b.y >= boxes[index - 1].bottom - 1), boxes);
      check("bankSceneListsHaveNaturalHeight", banks.every((bank) => {
        const list = q(".timelineExternalSourceShelfBankScenes", bank);
        return list && list.scrollHeight <= list.clientHeight + 1 && !scrolls(list, "y");
      }));
      const last = await reveal(cards.at(-1));
      check("lastSceneReachableThroughBody", reachable(last), last);
      for (const bank of banks) fullWidth(bank, "bankUsesFullWidth" + bank.dataset.timelineSourceShelfBankId);
    } else if (phase === "sources-media") {
      await click('[data-timeline-source-shelf-filter="video"]', "videoFilter");
      check("videoFilterSelected", q('[data-timeline-source-shelf-filter="video"]', scope).getAttribute("aria-pressed") === "true");
      const cards = all("[data-timeline-source-media-id]", scope).filter(rendered);
      const boxes = cards.map(rect);
      check("mediaCardsUseOneColumn", boxes.length > 0 && boxes.every((b, index) => index === 0 || b.y >= boxes[index - 1].bottom - 1), boxes);
      check("mediaActionsBelowIdentity", cards.every((card) => rect(q(".timelineExternalSourceCardActions", card)).y
        >= rect(q(".timelineExternalSourceCardTitle", card)).bottom - 1));
      const placement = all('[data-timeline-external-source="media_asset"]', scope).filter(rendered);
      check("videoAndAudioActionsRetained", placement.some((node) => node.dataset.timelineExternalSourceKind === "Video")
        && placement.some((node) => node.dataset.timelineExternalSourceKind === "Audio"));
      check("mediaVerificationGuardRetained", placement.every((node) => node.disabled === !node.draggable));
      await reachControls(cards.at(-1), "lastMediaCard");
      const details = q('[data-timeline-source-click-placement*="Video"]', scope);
      check("mediaLaneChoiceInitiallyDisclosed", details instanceof HTMLDetailsElement && !details.open
        && all("select", details).length === 2);
      await disclosure('[data-timeline-source-click-placement*="Video"]', true, "mediaLaneChoices");
      await reachControls(details, "mediaLaneChoices");
      await disclosure('[data-timeline-source-click-placement*="Video"]', false, "closeMediaLaneChoices");
    } else if (phase === "sources-context") {
      await click('[data-timeline-source-shelf-mode="inspector"]', "inspectorTab");
      check("inspectorSemantics", q('[data-timeline-source-shelf-mode="inspector"]', scope).getAttribute("aria-selected") === "true"
        && rendered(q("#timeline-source-context-panel-inspector", scope)) && !q("#timeline-source-context-panel-sources", scope));
      await click('[data-timeline-source-shelf-mode="sources"]', "sourcesTab");
      const tab = q('[data-timeline-source-shelf-mode="sources"]', scope);
      tab.focus({ preventScroll: true });
      tab.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })); await settle();
      check("contextEndKeyboardSelection", document.activeElement?.getAttribute("data-timeline-source-shelf-mode") === "video-preview"
        && q('[data-timeline-source-shelf-mode="video-preview"]', scope).getAttribute("aria-selected") === "true");
      document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true })); await settle();
      check("contextHomeKeyboardSelection", document.activeElement === tab && tab.getAttribute("aria-selected") === "true");
      check("sourceFiltersRetained", all("[data-timeline-source-shelf-filter]", scope).length === 4);
      check("verifyAllControlRetained", rendered(q("[data-timeline-source-media-verify-all]", scope)));
    } else throw new Error("Unknown Sources density phase: " + phase);
    const controls = all("[data-timeline-source-shelf-mode],[data-timeline-source-shelf-filter],[data-timeline-external-source]", scope).filter(rendered);
    check("sourceHitTargetsPreserved", controls.length > 0 && controls.every((node) => node.offsetWidth >= 24 && node.offsetHeight >= 24), controls.map(inspect));
    await measure();
  } else if (phase === "security-top" || phase === "security-advanced") {
    fullWidth(scope, "securityUsesFullWorkspaceWidth");
    const fields = all(".agentAuthorityForm input,.agentAuthorityForm select", scope);
    check("securityFormsRetained", fields.length >= 14 && all(".agentAuthoritySection", scope).length === 5);
    // The localization observer translates aria-label. The sole direct header
    // action is structurally stable in both locales and stays read-only here.
    const refresh = q(":scope > .panelHeader > button", scope);
    const top = await reveal(refresh);
    check("securityRefreshReachable", reachable(top), top);
    const targets = [q(".agentAuthorityGrantForm button", scope), ...all(".agentAuthorityConsentForm button", scope), q("#agent-audit-heading", scope), q(".agentAuthoritySection:last-child", scope)?.lastElementChild];
    const reached = [];
    for (const target of targets) reached.push(await reveal(target));
    check("securityAdvancedAndAuditReachableThroughPanel", reached.length === 5 && reached.every(reachable), reached);
    await measure();
    if (phase === "security-advanced") await reveal(targets.at(-1));
  } else throw new Error("Unknown Batch2 density phase: " + phase);
  return { checks, evidence };
}

export async function runUiDensityBatch2Checks({ CdpClient, cdpPort, appUrl, screenshotDir, waitForApp }) {
  if (!screenshotDir) throw new Error("--ui-density-batch2-only requires SYNDOCAL_VIEWPORT_SCREENSHOT_DIR");
  mkdirSync(screenshotDir, { recursive: true });
  const version = await (await fetch("http://127.0.0.1:" + cdpPort + "/json/version")).json();
  const browser = new CdpClient(version.webSocketDebuggerUrl);
  await browser.connect();
  const results = [];
  const waitFor = async (page, expression, label) => {
    for (let attempt = 0; attempt < 150; attempt += 1) {
      if (page.runtimeExceptions.length) throw new Error(page.runtimeExceptions.join("\n"));
      if (await page.evaluate(expression)) return;
      await pause(50);
    }
    throw new Error("Timed out: " + label);
  };
  try {
    for (const locale of ["en", "ja"]) for (const viewport of densityViewports) {
      const label = "ui-density-batch2-" + locale + "-" + viewport.width + "x" + viewport.height + "-dpr" + viewport.dpr;
      const result = { label, locale, viewport, fixture: "timeline-layered", freshBrowserContext: true,
        evidenceBoundary: "Production browser fixture; no native output, media verification, Timeline placement, or authority mutation proof.",
        checks: {}, measurements: {}, screenshots: [] };
      let contextId = null, page = null;
      const capture = async (phase) => {
        const filename = label + "-" + phase + ".png";
        const screenshot = await page.send("Page.captureScreenshot", { format: "png", fromSurface: true, captureBeyondViewport: false });
        writeFileSync(join(screenshotDir, filename), screenshot.data, "base64");
        result.screenshots.push(filename);
      };
      const probe = async (phase) => {
        const measured = await page.evaluate("(" + exerciseBatch2DensityInPage.toString() + ")(" + JSON.stringify(phase) + ")");
        result.measurements[phase] = measured.evidence;
        for (const [name, passed] of Object.entries(measured.checks)) result.checks[phase + "." + name] = passed;
        await capture(phase);
      };
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
        await page.connect(); await page.send("Page.enable"); await page.send("Runtime.enable");
        await page.send("Emulation.setDeviceMetricsOverride", { width: viewport.width, height: viewport.height, deviceScaleFactor: viewport.dpr, mobile: false });
        await page.send("Page.addScriptToEvaluateOnNewDocument", { source:
          "localStorage.setItem('syndocal.uiLocale.v1'," + JSON.stringify(locale) + ");" +
          "localStorage.setItem('syndocal.uiScale.v1','100');" +
          "(()=>{const surface=new URL(location.href).searchParams.get('syndocalDensitySurface');" +
          "localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:surface==='sources'?'control':'setup',setup_sub_tab:surface==='security'?'authority':'video',control_mode:surface==='sources'?'live':'edit',control_category:'position'}));})();",
        });
        for (const [surface, selector, phases] of [
          ["video", ".videoSetupPanel [data-video-setup-scroll]", ["video-default", "video-configuration", "video-correction", "video-configuration-restored", "video-output-tools", "video-composition-tools"]],
          ["sources", "[data-timeline-source-shelf]", ["sources-scenes", "sources-media", "sources-context"]],
          ["security", "[data-agent-authority-panel]", ["security-top", "security-advanced"]],
        ]) {
          const url = new URL(appUrl); url.searchParams.set("syndocalDensitySurface", surface);
          await page.send("Page.navigate", { url: url.toString() }); await waitForApp(page);
          result.measurements[surface + "Navigation"] = await page.evaluate("(" + selectDensitySurfaceInPage.toString() + ")(" + JSON.stringify(surface) + ")");
          await waitFor(page, "Boolean(document.querySelector(" + JSON.stringify(selector) + "))", surface);
          await pause(150);
          for (const phase of phases) await probe(phase);
        }
        const observed = await page.evaluate("({locale:document.documentElement.lang,width:innerWidth,height:innerHeight,dpr:devicePixelRatio})");
        result.observed = observed;
        result.checks.localeAndViewportExact = observed.locale === locale && observed.width === viewport.width && observed.height === viewport.height && observed.dpr === viewport.dpr;
        result.checks.scenarioCompleted = true;
      } catch (error) {
        result.error = error instanceof Error ? error.stack : String(error);
        result.checks.scenarioCompleted = false;
        if (page) try { await capture("failure"); } catch {}
      } finally {
        result.runtimeExceptions = page?.runtimeExceptions ?? [];
        result.checks.noRuntimeExceptions = result.runtimeExceptions.length === 0;
        result.passed = Object.keys(result.checks).length > 1 && Object.values(result.checks).every(Boolean);
        result.failedChecks = Object.entries(result.checks).filter(([, passed]) => !passed).map(([name]) => name);
        writeFileSync(join(screenshotDir, label + ".json"), JSON.stringify(result, null, 2) + "\n");
        results.push(result);
        console.log((result.passed ? "pass " : "fail ") + label + " failed=" + JSON.stringify(result.failedChecks));
        page?.close();
        if (contextId) await browser.send("Target.disposeBrowserContext", { browserContextId: contextId });
      }
    }
  } finally {
    browser.close();
    writeFileSync(join(screenshotDir, "ui-density-batch2-results.json"), JSON.stringify(results, null, 2) + "\n");
  }
  const failures = results.filter((result) => !result.passed);
  if (results.length !== 6 || failures.length) throw new Error("Batch2 density failed: " + JSON.stringify(failures.map(({ label, failedChecks, error }) => ({ label, failedChecks, error }))));
  console.log("Batch2 density passed: 6 fresh contexts, EN/JA, owner DPR2 and two desktop sizes.");
}
