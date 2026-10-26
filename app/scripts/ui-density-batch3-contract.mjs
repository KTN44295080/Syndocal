import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const viewports = [
  { width: 1280, height: 752, dpr: 2 },
  { width: 1366, height: 768, dpr: 1 },
  { width: 1920, height: 1080, dpr: 1 },
];
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Use real UI handlers after fixture initialization: the fixtures intentionally
// choose an initial workspace and can override a saved workspace preference.
async function navigateBatch3InPage(surface) {
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const path = [];
  const click = async (selector) => {
    const node = document.querySelector(selector);
    if (!(node instanceof HTMLElement) || node.disabled || !node.checkVisibility() || node.offsetWidth <= 0 || node.offsetHeight <= 0) {
      throw new Error("Missing visible Batch3 navigation: " + selector);
    }
    path.push({ selector, label: node.textContent.trim() });
    node.click(); await settle();
  };
  if (surface === "matrix" || surface === "quickfx") {
    await click('[data-workspace-option="touch"]');
    await click('[data-control-domain="video"]');
    await click('[data-video-perform-select-layer="1"]');
    if (surface === "quickfx") await click('[data-video-perform-inspector-tab="fx"]');
  } else {
    await click('[data-workspace-option="control"]');
    if (surface === "scenefx") {
      await click('[data-edit-domain-navigation] [data-control-mode-option="edit"]');
      await click('[data-scene-matrix-edit-strip="301"]');
      await click('[data-scene-settings-surface-control="fx"]');
      const add = document.querySelector('[data-scene-fx-disclosure="add"]');
      if (!(add instanceof HTMLDetailsElement)) throw new Error("Missing Scene FX Add disclosure");
      if (!add.open) await click('[data-scene-fx-disclosure="add"] > summary');
      await click('[data-scene-fx-chooser] [data-effect-family="COLOR FX"]');
    } else if (surface === "timeline") {
      await click('[data-edit-domain-navigation] [data-control-mode-option="live"]');
      await click('[data-timeline-desk-surface="show"]');
      await click(".timelineToolsDisclosure > summary");
    } else throw new Error("Unknown Batch3 surface: " + surface);
  }
  return path;
}

// One production geometry/interaction contract for four surfaces. It does not
// queue, Take, bypass, Save, change Follow, route audio, or mutate output state.
// Dedicated component/browser gates retain those callback/IPC/focus assertions.
export async function exerciseBatch3DensityInPage(phase) {
  const checks = {}, evidence = {};
  const check = (name, pass, detail) => { checks[name] = Boolean(pass); if (detail !== undefined) evidence[name] = detail; };
  const q = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const kind = phase.split("-")[0];
  const selector = { matrix: "[data-video-perform-matrix]", quickfx: "[data-perform-quick-fx]",
    scenefx: "[data-scene-settings]", timeline: ".timelineToolsDisclosurePanel" }[kind];
  const scope = selector && q(selector);
  const rendered = (node) => {
    if (!(node instanceof HTMLElement) || !node.isConnected || !node.offsetWidth || !node.offsetHeight) return false;
    for (let current = node; current instanceof HTMLElement; current = current.parentElement) {
      const s = getComputedStyle(current);
      if (current.hidden || s.display === "none" || /hidden|collapse/.test(s.visibility)) return false;
      if (current instanceof HTMLDetailsElement && current !== node && !current.open && !q(":scope > summary", current)?.contains(node)) return false;
    }
    return true;
  };
  if (!rendered(scope)) throw new Error("Missing Batch3 density surface: " + phase);
  const owner = kind === "matrix" ? q(".videoPerformMatrixScroll", scope)
    : kind === "quickfx" ? q("[data-perform-fx-scroll]", scope)
    : kind === "scenefx" ? q(".sceneSettingsScroller", scope) : scope;
  if (!rendered(owner)) throw new Error("Missing contracted scroll owner: " + phase);
  const rect = (node) => { const b = node.getBoundingClientRect(); return { x: b.x, y: b.y, width: b.width, height: b.height, right: b.right, bottom: b.bottom }; };
  const identity = (node) => node.id || node.getAttribute("data-timeline-tools-panel") || node.getAttribute("data-perform-fx-panel")
    || node.getAttribute("data-scene-fx-disclosure") || node.className || node.tagName;
  const ancestors = [];
  for (let node = scope.parentElement; node; node = node.parentElement) ancestors.push(node);
  const nodes = () => [...new Set([scope, ...all("*", scope), ...ancestors])].filter(rendered);
  const scrolls = (node, axis) => /(auto|scroll)/.test(getComputedStyle(node)[axis === "x" ? "overflowX" : "overflowY"])
    && (axis === "x" ? node.scrollWidth > node.clientWidth + 1 : node.scrollHeight > node.clientHeight + 1);
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
      hit: hit === node || node.contains(hit), fontSize: parseFloat(getComputedStyle(node).fontSize), disabled: Boolean(node.disabled) };
  };
  const reachable = (state) => state.rendered && state.contained && state.hit && state.onlyOwnerVerticalScroll !== false;
  const reveal = async (node, scrollOwner = owner) => {
    const before = [...new Set([...nodes(), scrollOwner])].map((element) => ({ element, left: element.scrollLeft, top: element.scrollTop }));
    if (rendered(node) && scrollOwner.contains(node)) for (let attempt = 0; attempt < 3; attempt += 1) {
      const b = rect(node), p = rect(scrollOwner);
      const top = p.y + scrollOwner.clientTop + 2, bottom = p.y + scrollOwner.clientTop + scrollOwner.clientHeight - 2;
      if (b.y < top) scrollOwner.scrollTop -= top - b.y;
      else if (b.bottom > bottom) scrollOwner.scrollTop += b.bottom - bottom;
      await settle();
      if (reachable(inspect(node))) break;
    }
    const state = inspect(node);
    state.onlyOwnerVerticalScroll = before.every(({ element, left, top }) => element.scrollLeft === left && (element === scrollOwner || element.scrollTop === top));
    state.scrollMovement = before.filter(({ element, left, top }) => element.scrollLeft !== left || element.scrollTop !== top)
      .map(({ element, left, top }) => ({ target: identity(element), beforeLeft: left, beforeTop: top, afterLeft: element.scrollLeft, afterTop: element.scrollTop, owner: element === scrollOwner }));
    return state;
  };
  const click = async (selector, name, root = scope, scrollOwner = owner) => {
    const node = q(selector, root), state = await reveal(node, scrollOwner);
    check(name + "Reachable", reachable(state) && !node?.disabled, state);
    if (!(node instanceof HTMLElement) || node.disabled) throw new Error("Missing enabled Batch3 control: " + selector);
    node.click(); await settle(); return node;
  };
  const disclose = async (selector, open, name) => {
    const details = q(selector, scope);
    if (!(details instanceof HTMLDetailsElement)) throw new Error("Missing Batch3 disclosure: " + selector);
    if (details.open !== open) await click(selector + " > summary", name);
    check(name + "State", details.open === open); return details;
  };
  const reachControls = async (root, name, every = false) => {
    const controls = all("button,input:not([type=hidden]),select,summary", root).filter(rendered);
    const selected = every ? controls : [...new Set([controls[0], controls.at(-1)])].filter(Boolean);
    const states = [];
    for (const control of selected) states.push(await reveal(control));
    check(name + "ReachableThroughOwner", states.length > 0 && states.every(reachable), states);
    return states;
  };
  const contentBox = (node) => {
    const b = rect(node), s = getComputedStyle(node);
    return { x: b.x + node.clientLeft + parseFloat(s.paddingLeft), right: b.x + node.clientLeft + node.clientWidth - parseFloat(s.paddingRight) };
  };
  const fullWidth = (node, name) => {
    const b = rect(node), available = contentBox(node.parentElement);
    check(name, Math.abs(b.x - available.x) <= 2 && Math.abs(b.right - available.right) <= 2, { box: b, available });
  };
  const textState = async (node) => {
    const state = await reveal(node);
    if (!state.rendered) return state;
    const fragments = [];
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    for (let text = walker.nextNode(); text; text = walker.nextNode()) if (text.textContent.trim() && rendered(text.parentElement)) {
      // These are explicitly abbreviated user-data readouts beside the static
      // heading, not heading text. Their full value is retained in title and in
      // the opened rack/Targets content; report their truncation separately.
      if (kind === "scenefx" && text.parentElement.closest("[data-scene-fx-target-summary],[data-scene-fx-selected-label]")) continue;
      const range = document.createRange(); range.selectNodeContents(text);
      for (const r of range.getClientRects()) if (r.width > 0 && r.height > 0) fragments.push({ x: r.x, y: r.y, width: r.width, height: r.height, right: r.right, bottom: r.bottom });
    }
    const tops = [];
    for (const r of fragments) if (!tops.some((top) => Math.abs(top - r.y) <= 3)) tops.push(r.y);
    const s = getComputedStyle(node), clip = { ...state.clip }, box = state.box;
    if (s.overflowX !== "visible") { clip.x = Math.max(clip.x, box.x + node.clientLeft); clip.right = Math.min(clip.right, box.x + node.clientLeft + node.clientWidth); }
    if (s.overflowY !== "visible") { clip.y = Math.max(clip.y, box.y + node.clientTop); clip.bottom = Math.min(clip.bottom, box.y + node.clientTop + node.clientHeight); }
    return { ...state, text: node.textContent.trim(), lines: tops.length, lineTops: tops, fragments,
      fits: fragments.length > 0 && fragments.every((r) => r.x >= Math.max(box.x, clip.x) - 1 && r.right <= Math.min(box.right, clip.right) + 1 && r.y >= clip.y - 1 && r.bottom <= clip.bottom + 1),
      lineHeight: s.lineHeight, overflowX: s.overflowX, overflowY: s.overflowY };
  };
  const measure = async () => {
    const vertical = nodes().filter((node) => scrolls(node, "y")), horizontal = nodes().filter((node) => scrolls(node, "x"));
    const nested = vertical.filter((node) => vertical.some((parent) => parent !== node && parent.contains(node)));
    const describe = (node) => ({ target: identity(node), width: node.clientWidth, scrollWidth: node.scrollWidth, height: node.clientHeight, scrollHeight: node.scrollHeight, left: node.scrollLeft, top: node.scrollTop });
    const roots = [document.documentElement, document.body, q(".app"), q(".layout")].filter(Boolean);
    check("viewportOverflowZero", scrollX === 0 && scrollY === 0 && roots.every((node) => node.scrollWidth <= node.clientWidth + 1 && node.scrollHeight <= node.clientHeight + 1 && !node.scrollLeft && !node.scrollTop), roots.map(describe));
    check("noHorizontalScroll", horizontal.length === 0 && scope.scrollWidth <= scope.clientWidth + 1 && owner.scrollWidth <= owner.clientWidth + 1, horizontal.map(describe));
    check("noNestedScroll", nested.length === 0, { vertical: vertical.map(describe), nested: nested.map(describe) });
    check("onlyContractedScrollOwner", vertical.every((node) => node === owner), vertical.map(describe));
    check("scopeContained", inspect(scope).contained, inspect(scope));
    check("ownerContained", inspect(owner).contained, inspect(owner));
    const headingSelector = kind === "matrix" ? ".videoPerformMatrixCorner strong,[data-video-perform-page-range]"
      : kind === "quickfx" ? ".videoPerformQuickFxLayerName,[data-perform-fx-view]"
      : kind === "scenefx" ? ".panelHeader h2,.panelHeader h3,[data-scene-fx-disclosure] > summary"
      : "[data-timeline-tools-tab],[data-timeline-bank-create-disclosure] > summary";
    const headings = [];
    for (const heading of all(headingSelector, scope).filter(rendered)) headings.push(await textState(heading));
    check("headingsSingleLineAndUnclipped", headings.length > 0 && headings.every((heading) => heading.lines === 1 && heading.fits && reachable(heading)), headings);
    evidence.controlDimensions = all("button,summary,input:not([type=hidden]),select", scope).filter(rendered).map((node) => ({
      target: identity(node), text: (node.textContent ?? "").trim().slice(0, 90), box: rect(node), fontSize: parseFloat(getComputedStyle(node).fontSize) }));
    evidence.ellipsizedText = all("*", scope).filter(rendered).filter((node) => getComputedStyle(node).textOverflow === "ellipsis" && node.scrollWidth > node.clientWidth + 1).map((node) => ({
      target: identity(node), text: node.textContent.trim(), title: node.title, width: node.clientWidth, scrollWidth: node.scrollWidth,
      fullSelectedReadout: kind === "matrix" && Boolean(node.closest(".videoPerformMatrixCell")) && Boolean(q("[data-video-perform-selected-clip-name]")) }));
    owner.scrollTop = 0; await settle();
  };
  const minimumTargets = (selector, name, minimum = 44, minimumFont = null) => {
    const controls = all(selector, scope).filter(rendered);
    const dimensions = controls.map((node) => ({ target: identity(node), box: rect(node), fontSize: parseFloat(getComputedStyle(node).fontSize) }));
    check(name, dimensions.length > 0 && dimensions.every(({ box, fontSize }) => box.width >= minimum - 0.5 && box.height >= minimum - 0.5 && (minimumFont === null || fontSize >= minimumFont)), dimensions);
  };
  const data = window.__syndocalBatch3Density ??= {};

  if (kind === "matrix") {
    const row = () => q('[data-video-perform-row="1"]', scope);
    const metadata = () => ({ start: Number(scope.dataset.pageStart), end: Number(scope.dataset.pageEnd), size: Number(scope.dataset.pageSize), count: Number(scope.dataset.slotCount) });
    const pageState = () => {
      const meta = metadata(), pads = all("[data-video-perform-cell-index]", row());
      const strip = q(".videoPerformLayerStrip", row()), available = owner.clientWidth - rect(strip).width;
      return { ...meta, available, ownerWidth: owner.clientWidth, stripWidth: rect(strip).width,
        indices: pads.map((pad) => Number(pad.dataset.videoPerformCellIndex)), widths: pads.map((pad) => rect(pad.closest(".videoPerformMatrixCell")).width),
        slotIds: pads.map((pad) => Number(pad.closest("[data-video-clip-slot-id]").dataset.videoClipSlotId)),
        mapping: pads.map((pad) => JSON.parse(pad.dataset.controlMapTarget ?? "null")),
        roving: all('[data-video-perform-cell-index][tabindex="0"]', scope).length,
        allVisible: pads.every((pad) => reachable(inspect(pad))), scrollLeft: owner.scrollLeft,
        previousDisabled: q('[data-video-perform-page="previous"]', scope)?.disabled,
        nextDisabled: q('[data-video-perform-page="next"]', scope)?.disabled };
    };
    if (phase === "matrix-initial") {
      const state = pageState();
      check("initialPageAndCanonicalSlots", state.start === 0 && state.count === 32 && state.end === state.size, state);
      check("containerDeterminesPageSize", state.size === Math.max(1, Math.floor(state.available / 100)), state);
      check("preserved100pxClipColumns", state.widths.every((width) => width >= 100 - 0.5), state.widths);
      check("completeColumnsMeetExistingMinimum", state.indices.length >= (innerWidth >= 1920 ? 15 : innerWidth >= 1366 ? 8 : 7) && state.allVisible, state);
      check("oneVisibleRovingPad", state.roving === 1, state.roving);
      const head = q(".videoPerformMatrixHead", scope), b = rect(owner), h = rect(head);
      const complete = all("[data-video-perform-row],[data-video-perform-group]", scope).filter((node) => { const r = rect(node); return r.y >= h.bottom - 1 && r.bottom <= b.y + owner.clientTop + owner.clientHeight + 1; });
      check("existingCompleteRowsPreserved", complete.length >= (innerWidth >= 1920 ? 4 : 3) && h.height === 44, { count: complete.length, head: h });
      // Explicit workspace navigation can clear the fixture's selected slot.
      // Prove the empty selection readout, then select via the arrow-key route;
      // clicking a pad would queue it and would test a different operation.
      if (!q("[data-video-perform-selected-clip-name]")) {
        const empty = await textState(q(".videoPerformSelectedClip", scope));
        check("unselectedClipPromptAccessible", empty.text.length > 0 && empty.fits && reachable(empty), empty);
        const first = q('[data-video-perform-cell-index="0"]', row());
        first.focus({ preventScroll: true });
        first.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true })); await settle();
        check("keyboardSelectsClipWithoutPadActivation", document.activeElement?.dataset.videoPerformCellIndex === "1" && q('[data-video-perform-cell-index="1"]', row())?.getAttribute("aria-pressed") === "true");
      }
      const name = q("[data-video-perform-selected-clip-name]");
      const nameState = name ? await textState(name) : null;
      check("selectedClipFullNameReachable", nameState && nameState.fits && reachable(nameState) && nameState.text.includes("Viewport Clip 2"), nameState);
      minimumTargets('[data-video-perform-page],.videoPerformClipTrigger', "pagerAndPadTargetsPreserved");
    } else if (phase === "matrix-pages") {
      const pages = [];
      for (let step = 0; step < 32; step += 1) {
        const state = pageState(); pages.push(state);
        if (state.nextDisabled) break;
        await click('[data-video-perform-page="next"]', "nextPage" + step);
      }
      const indices = pages.flatMap((page) => page.indices);
      check("all32AbsoluteSlotsReachableByPaging", indices.length === 32 && indices.every((index, position) => index === position), pages);
      check("everyPageContainedAndSized", pages.length > 1 && pages.every((page) => page.allVisible && page.scrollLeft === 0 && page.roving === 1 && page.widths.every((width) => width >= 99.5)), pages);
      check("canonicalSlotAndMidiIdsPreserved", pages.every((page) => page.indices.every((index, offset) => page.slotIds[offset] === 5001 + index
        && page.mapping[offset]?.[0]?.action === "VideoClipSlotTrigger" && page.mapping[offset]?.[0]?.layer_id === 1 && page.mapping[offset]?.[0]?.clip_slot_id === 5001 + index)), pages);
      check("lastPageBoundaryDisabled", metadata().end === 32 && q('[data-video-perform-page="next"]', scope).disabled && !q('[data-video-perform-page="previous"]', scope).disabled);
      data.matrixPages = pages;
    } else if (phase === "matrix-offpage") {
      await click("[data-video-perform-offpage-status]", "offPageStatus");
      const list = q("[data-video-perform-offpage-list]");
      if (!rendered(list)) throw new Error("Missing bounded off-page status list");
      const entries = all("[data-video-perform-status-jump]", list);
      const states = [];
      for (const entry of entries) states.push(await reveal(entry, list));
      const ids = entries.map((entry) => ({ kind: entry.dataset.videoPerformStatusJump, layer: Number(entry.dataset.layerId), slot: Number(entry.dataset.slotId), index: Number(entry.dataset.clipIndex) }));
      check("offPageLiveAndQueuedRuntimeRetained", ids.some((x) => x.kind === "live" && x.layer === 1 && x.slot === 5001 && x.index === 0)
        && ids.some((x) => x.kind === "queued" && x.layer === 1 && x.slot === 5002 && x.index === 1), ids);
      check("statusListIsIndependentAndContained", !owner.contains(list) && inspect(list).contained && list.scrollWidth <= list.clientWidth + 1, inspect(list));
      check("everyStatusJumpReachable", states.length >= 2 && states.every(reachable), states);
      check("statusJumpTargetsPreserved", entries.every((entry) => rect(entry).width >= 43.5 && rect(entry).height >= 43.5), entries.map(rect));
    } else if (phase === "matrix-keyboard") {
      const list = q("[data-video-perform-offpage-list]");
      await click('[data-video-perform-status-jump="live"][data-layer-id="1"]', "jumpToLive", document, list);
      check("liveJumpSelectsCanonicalSlot", metadata().start === 0 && q('[data-video-perform-cell-index="0"]', row())?.getAttribute("aria-pressed") === "true");
      await click('[data-video-perform-page="next"]', "leaveLivePage");
      await click("[data-video-perform-offpage-status]", "reopenStatus");
      await click('[data-video-perform-status-jump="queued"][data-layer-id="1"]', "jumpToQueued", document, q("[data-video-perform-offpage-list]"));
      check("queuedJumpSelectsCanonicalSlot", metadata().start === 0 && q('[data-video-perform-cell-index="1"]', row())?.getAttribute("aria-pressed") === "true");
      const pad = q('[data-video-perform-cell-index="1"]', row()); pad.focus({ preventScroll: true });
      pad.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true, cancelable: true })); await settle();
      check("endFocusesAbsoluteSlot32", document.activeElement?.dataset.videoPerformCellIndex === "31" && metadata().end === 32 && reachable(inspect(document.activeElement)), inspect(document.activeElement));
      document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true, cancelable: true })); await settle();
      check("homeFocusesAbsoluteSlot1", document.activeElement?.dataset.videoPerformCellIndex === "0" && metadata().start === 0 && reachable(inspect(document.activeElement)), inspect(document.activeElement));
      check("keyboardKeepsOneVisibleRovingPad", all('[data-video-perform-cell-index][tabindex="0"]', scope).length === 1 && owner.scrollLeft === 0);
    } else throw new Error("Unknown matrix phase: " + phase);
    // Portal entry covers a corner of the matrix intentionally; inspect heading
    // and scope after measuring and closing it, then reopen it for the screenshot.
    const statusWasOpen = Boolean(q("[data-video-perform-offpage-list]"));
    if (statusWasOpen) await click("[data-video-perform-offpage-status]", "closeStatusForGeometry");
    await measure();
    if (statusWasOpen) await click("[data-video-perform-offpage-status]", "restoreStatusForScreenshot");
  } else if (kind === "quickfx") {
    const compact = scope.dataset.fxCompact === "true";
    const panel = (view) => q('[data-perform-fx-panel="' + view + '"]', scope);
    const showView = async (view) => {
      if (compact) await click('[data-perform-fx-view="' + view + '"]', view + "View");
      check(view + "PanelVisible", rendered(panel(view)));
      check("compactViewsExclusive", !compact || ["parameters", "chain"].filter((candidate) => rendered(panel(candidate))).length === 1);
      check("inactiveFxPanelActuallyHiddenByCss", !compact || ["parameters", "chain"].filter((candidate) => candidate !== view).every((candidate) => panel(candidate).hidden && getComputedStyle(panel(candidate)).display === "none" && panel(candidate).getClientRects().length === 0));
    };
    if (phase === "quickfx-parameters") {
      check("eightStagesRetained", all("[data-perform-fx-stage]", scope).length === 8);
      check("parametersFirstInCompactInspector", rendered(panel("parameters")) && (!compact || !rendered(panel("chain"))));
      check("initialInactiveFxPanelActuallyHiddenByCss", !compact || panel("chain").hidden && getComputedStyle(panel("chain")).display === "none" && panel("chain").getClientRects().length === 0);
      const picker = q("[data-perform-fx-stage-picker]", scope);
      check("stagePickerKeepsAbsoluteIndices", picker instanceof HTMLSelectElement && picker.options.length === 8 && [...picker.options].every((option, index) => Number(option.value) === index));
      const input = q('[data-perform-fx-control="level"] input', scope);
      check("selectedThresholdParameterPresent", rendered(input));
      check("firstMainParameterInitiallyReachable", reachable(inspect(input)), inspect(input));
      await reachControls(panel("parameters"), "parameters");
      data.fxParameter = input; data.fxParameterValue = input?.value;
      if (compact) fullWidth(panel("parameters"), "compactParametersUseFullWidth");
      minimumTargets("[data-perform-fx-view],[data-perform-fx-stage-picker],[data-perform-fx-selected-bypass],[data-perform-fx-control] input,[data-perform-fx-control] button", "quickFxTargetsPreserved", 44, 14);
    } else if (phase === "quickfx-chain") {
      await showView("chain");
      const stages = all("[data-perform-fx-stage]", scope);
      check("allEightStagesRetainIdentityAndOrder", stages.length === 8 && stages.every((stage, index) => Number(stage.dataset.performFxStage) === index && stage.dataset.performFxLayer === "1"));
      await reachControls(panel("chain"), "chain");
      check("parameterDomAndValueSurviveManagement", data.fxParameter?.isConnected && q('[data-perform-fx-control="level"] input', scope) === data.fxParameter && data.fxParameter.value === data.fxParameterValue);
      if (compact) fullWidth(panel("chain"), "compactChainUsesFullWidth");
      minimumTargets("[data-perform-fx-select],[data-perform-fx-bypass]", "chainTargetsPreserved", 44, 14);
    } else if (phase === "quickfx-images") {
      await showView("parameters");
      const disclosure = q("[data-fx-image-disclosure]", scope);
      check("imageInputsInitiallyDisclosed", disclosure instanceof HTMLDetailsElement && !disclosure.open);
      await disclose("[data-fx-image-disclosure]", true, "imageInputs");
      const imageControls = all("[data-fx-image-source],[data-fx-image-path],[data-fx-image-apply]", scope).filter(rendered);
      if (imageControls.length) {
        await reachControls(q("[data-fx-image-inputs]", scope), "imageInputs");
        minimumTargets("[data-fx-image-source],[data-fx-image-path],[data-fx-image-apply]", "imageInputTargetsPreserved", 44, 14);
      } else {
        const generator = q("[data-fx-generator]", scope), state = await reveal(generator);
        check("generatorExplanationReachableWithoutArtificialInputs", reachable(state) && generator.textContent.trim().length > 0 && all("[data-fx-image-source]", scope).length === 0, state);
        evidence.imageBindingBoundary = "operator-vj has a generator-only image description. Real bindings, cycles, paths, rejection and 44px/14px controls remain required in check-fx-ux-browser.mjs.";
      }
    } else if (phase === "quickfx-restored") {
      await disclose("[data-fx-image-disclosure]", false, "closeImageInputs");
      await showView("parameters");
      check("parameterDomRetainedAfterViews", data.fxParameter?.isConnected && q('[data-perform-fx-control="level"] input', scope) === data.fxParameter && data.fxParameter.value === data.fxParameterValue);
      const picker = q("[data-perform-fx-stage-picker]", scope);
      if (compact) { picker.value = "7"; picker.dispatchEvent(new Event("change", { bubbles: true })); await settle(); }
      else await click('[data-perform-fx-stage="7"] [data-perform-fx-select]', "selectLastWideStage");
      check("stagePickerSelectsLastAbsoluteStage", q("[data-perform-fx-detail]", scope)?.dataset.performFxDetail === "7");
      if (compact) { picker.value = "0"; picker.dispatchEvent(new Event("change", { bubbles: true })); await settle(); }
      else await click('[data-perform-fx-stage="0"] [data-perform-fx-select]', "selectFirstWideStage");
      check("stagePickerReturnsToFirstStage", q("[data-perform-fx-detail]", scope)?.dataset.performFxDetail === "0");
      await reachControls(panel("parameters"), "restoredParameters");
    } else throw new Error("Unknown Quick FX phase: " + phase);
    const safety = q("[data-perform-fx-safety]", scope);
    check("safetyOutsideSwitchablePanels", rendered(safety) && !panel("chain").contains(safety) && !panel("parameters").contains(safety));
    const safetyControls = all("button", safety).filter(rendered);
    if (safetyControls.length) await reachControls(safety, "sharedSafety");
    else check("sharedSafetyTextReachable", reachable(await reveal(safety)), inspect(safety));
    await measure();
  } else if (kind === "scenefx") {
    const disclosure = (name) => '[data-scene-fx-disclosure="' + name + '"]';
    const editor = q("[data-scene-settings-effect-editor]", scope);
    if (phase === "scenefx-selected") {
      check("selectedEditorIsPrimaryExpandedSurface", rendered(editor) && all("[data-scene-fx-disclosure]", scope).length === 3 && all("[data-scene-fx-disclosure]", scope).every((node) => !node.open));
      data.sceneNarrowWidth = rect(editor).width;
      const first = all("input,button,select", editor).filter(rendered)[0];
      check("selectedEditorStartsBeforeOptionalDetails", rendered(first) && rect(editor).y < rect(owner).y + Math.min(220, owner.clientHeight / 2), rect(editor));
      check("firstSceneParameterInitiallyReachable", reachable(inspect(first)), inspect(first));
      await reachControls(editor, "selectedEditor");
    } else if (phase === "scenefx-targets") {
      await disclose(disclosure("targets"), true, "targets");
      const target = q(disclosure("targets"), scope);
      const multi = q(".sceneEffectTargetModes button:nth-child(2)", target);
      if (multi) await click(".sceneEffectTargetModes button:nth-child(2)", "multipleFixtureTargets", target);
      check("targetsRetainFixtureChoices", all("[data-effect-target-fixture]", target).length === 3);
      await reachControls(target, "targets");
    } else if (phase === "scenefx-add") {
      await disclose(disclosure("add"), true, "addFx");
      check("allEightFamiliesRetained", all("[data-effect-family]", q("[data-scene-fx-chooser]", scope)).length === 8);
      await reachControls(q(disclosure("add"), scope), "fxFamilies", true);
    } else if (phase === "scenefx-rack") {
      await disclose(disclosure("rack"), true, "fxRack");
      check("ownedFxRetained", all("[data-scene-owned-effect]", scope).length > 0 || all("[data-scene-owned-effect-id]", scope).length > 0);
      await reachControls(q(disclosure("rack"), scope), "ownedFx");
    } else if (phase === "scenefx-wide") {
      for (const name of ["add", "rack", "targets"]) await disclose(disclosure(name), false, "close" + name);
      await click("[data-scene-settings-wide-toggle]", "wideEditor");
      check("widePreferenceAndWorkspaceSharePreserved", scope.dataset.sceneSettingsWide === "true" && localStorage.getItem("syndocal.sceneFxEditorWide.v1") === "1" && rect(scope).width / innerWidth > 0.55, rect(scope));
      check("wideEditorGainsUsableWidth", rect(editor).width > data.sceneNarrowWidth + 100, { narrow: data.sceneNarrowWidth, wide: rect(editor).width });
      await reachControls(editor, "wideEditor");
    } else throw new Error("Unknown Scene FX phase: " + phase);
    check("optionalFxDisclosuresExclusive", all("[data-scene-fx-disclosure][open]", scope).length <= 1);
    const readouts = all("[data-scene-fx-target-summary],[data-scene-fx-selected-label]", scope);
    check("abbreviatedFxReadoutsRetainFullValue", readouts.length === 2 && readouts.every((node) => node.title === node.textContent.trim()), readouts.map((node) => ({ text: node.textContent.trim(), title: node.title })));
    check("editorHasNaturalHeight", editor.scrollHeight <= editor.clientHeight + 2 && !scrolls(editor, "y"), { height: editor.clientHeight, scrollHeight: editor.scrollHeight });
    const save = q(".effectSubmitActions button.primary", editor);
    check("saveReachableThroughSingleOwner", reachable(await reveal(save)), inspect(save));
    // Frozen before-density.json: Save is 134.484375x40, Cancel is 90x40,
    // both 14px. Keep those existing floors; the new disclosure entries are 44px.
    minimumTargets(".effectSubmitActions button", "sceneExistingActionTargetsPreserved", 40, 14);
    minimumTargets("[data-scene-fx-disclosure] > summary", "sceneDisclosureTargetsPreserved", 44, 14);
    const actionText = [];
    for (const button of all(".effectSubmitActions button", editor).filter(rendered)) actionText.push(await textState(button));
    check("sceneActionLabelsUnclippedAtExistingSize", actionText.length >= 2 && actionText.every((state) => state.box.width >= 89.5 && state.fits && reachable(state)), actionText);
    await measure();
  } else if (kind === "timeline") {
    const panel = (name) => q('[data-timeline-tools-panel="' + name + '"]', scope);
    const tab = (name) => q('[data-timeline-tools-tab="' + name + '"]', scope);
    const select = async (name) => {
      if (tab(name)?.getAttribute("aria-selected") !== "true") await click('[data-timeline-tools-tab="' + name + '"]', name + "Tab");
      check(name + "TabSelected", tab(name)?.getAttribute("aria-selected") === "true" && rendered(panel(name)));
    };
    if (phase === "timeline-bank") {
      check("bankDefaultAndMountedPanels", tab("bank")?.getAttribute("aria-selected") === "true" && all("[data-timeline-tools-panel]", scope).length === 3);
      check("existingTimelinesRetained", all(".timelineBankList > li", panel("bank")).length === 2);
      check("creationInitiallyDisclosed", q("[data-timeline-bank-create-disclosure]", scope) instanceof HTMLDetailsElement && !q("[data-timeline-bank-create-disclosure]", scope).open);
      await reachControls(panel("bank"), "bank");
      fullWidth(panel("bank"), "bankUsesFullWidth");
    } else if (phase === "timeline-create") {
      await disclose("[data-timeline-bank-create-disclosure]", true, "createTimeline");
      const input = q('[data-timeline-bank-create-disclosure] input[type="text"]', scope) || q("[data-timeline-bank-create-disclosure] input", scope);
      check("newTimelineInputReachable", reachable(await reveal(input)), inspect(input));
      input.value = "Density QA unsaved Timeline";
      input.dispatchEvent(new Event("input", { bubbles: true })); await settle();
      data.timelineDraft = input; data.timelineDraftValue = input.value;
      await reachControls(q("[data-timeline-bank-create-disclosure]", scope), "newTimeline");
    } else if (phase === "timeline-phases") {
      await select("phases");
      check("threePhaseRowsRetained", all(".timelinePhaseEditorBody .timelinePhaseRow", scope).length === 3 || all(".timelinePhaseBand", scope).length === 3);
      await reachControls(panel("phases"), "phases");
      fullWidth(panel("phases"), "phasesUseFullWidth");
      const toolbar = q(".timelinePhaseEditorToolbar", scope);
      if (toolbar) check("phaseToolbarFits", toolbar.scrollWidth <= toolbar.clientWidth + 1, rect(toolbar));
    } else if (phase === "timeline-audio") {
      await select("audio");
      await reachControls(panel("audio"), "audioSetupEntry");
      fullWidth(panel("audio"), "audioUsesFullWidth");
      check("audioRoutingRemainsSetupEntry", Boolean(q("[data-timeline-cue-audio-open-setup]", panel("audio"))) && all("input,select", panel("audio")).length === 0);
    } else if (phase === "timeline-restored") {
      await select("bank");
      check("unsavedTimelineDraftAndDomRetained", data.timelineDraft?.isConnected && q("[data-timeline-bank-create-disclosure] input", scope) === data.timelineDraft && data.timelineDraft.value === data.timelineDraftValue);
      tab("bank").focus({ preventScroll: true });
      tab("bank").dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true, cancelable: true })); await settle();
      check("tabEndSelectsAudio", document.activeElement === tab("audio") && tab("audio").getAttribute("aria-selected") === "true");
      tab("audio").dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true, cancelable: true })); await settle();
      check("tabHomeSelectsBank", document.activeElement === tab("bank") && tab("bank").getAttribute("aria-selected") === "true");
      tab("bank").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true })); await settle();
      check("tabArrowSelectsPhases", document.activeElement === tab("phases") && tab("phases").getAttribute("aria-selected") === "true");
      await select("bank");
      await disclose("[data-timeline-bank-create-disclosure]", false, "closeCreateTimeline");
      await reachControls(panel("bank"), "restoredBank");
    } else throw new Error("Unknown Timeline phase: " + phase);
    const tabs = all("[data-timeline-tools-tab]", scope), panels = all("[data-timeline-tools-panel]", scope);
    check("tabsAreExclusiveAndAccessible", tabs.length === 3 && panels.length === 3 && tabs.filter((node) => node.getAttribute("aria-selected") === "true").length === 1
      && tabs.filter((node) => node.tabIndex === 0).length === 1 && panels.filter(rendered).length === 1
      && tabs.every((node) => node.getAttribute("role") === "tab" && q("#" + CSS.escape(node.getAttribute("aria-controls")), scope)?.getAttribute("role") === "tabpanel"));
    check("inactiveToolsPanelsActuallyHiddenByCss", panels.filter((node) => node.hidden).length === 2 && panels.filter((node) => node.hidden).every((node) => getComputedStyle(node).display === "none" && node.getClientRects().length === 0));
    // Preserve the actual pre-batch controls, not an invented uniform square.
    // before-density.json (EN and JA): scrubber 150x22, A 25.03125x44,
    // B 24.03125x44. Every other affected control/new tab retains 44px.
    const endpoints = all(".timelinePerformanceLoopBoundaries button", scope).filter(rendered);
    const scrubber = q(".timelineScrubber", scope);
    const normalTargets = all("button,input:not([type=checkbox]),select,summary", scope).filter(rendered)
      .filter((node) => node !== scrubber && !endpoints.includes(node));
    check("timelineTargetsPreserved", normalTargets.length > 0 && normalTargets.every((node) => rect(node).width >= 43.5 && rect(node).height >= 43.5 && parseFloat(getComputedStyle(node).fontSize) >= 14), normalTargets.map(inspect));
    check("legacyLoopBoundaryTargetsPreserved", endpoints.length === 2 && endpoints.every((node, index) => rect(node).width >= [25.03125, 24.03125][index] - 0.5 && rect(node).height >= 43.5 && parseFloat(getComputedStyle(node).fontSize) >= 14), endpoints.map(inspect));
    check("legacyScrubberTargetPreserved", rendered(scrubber) && rect(scrubber).width >= 149.5 && rect(scrubber).height >= 21.5 && parseFloat(getComputedStyle(scrubber).fontSize) >= 14, inspect(scrubber));
    check("legacyScrubberReachableThroughOwner", reachable(await reveal(scrubber)), inspect(scrubber));
    const critical = [q("[data-timeline-follow-operator-badge]"), q("[data-timeline-follow-operator-abort]")];
    check("followStateAndAbortRemainOutsideTools", critical.every((node) => rendered(node) && !scope.contains(node)), critical.map(inspect));
    await measure();
  }
  return { checks, evidence };
}

export async function runUiDensityBatch3Checks({ CdpClient, cdpPort, appUrl, screenshotDir, waitForApp }) {
  if (!screenshotDir) throw new Error("--ui-density-batch3-only requires SYNDOCAL_VIEWPORT_SCREENSHOT_DIR");
  mkdirSync(screenshotDir, { recursive: true });
  const version = await (await fetch("http://127.0.0.1:" + cdpPort + "/json/version")).json();
  const browser = new CdpClient(version.webSocketDebuggerUrl); await browser.connect();
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
    for (const locale of ["en", "ja"]) for (const viewport of viewports) {
      const label = "ui-density-batch3-" + locale + "-" + viewport.width + "x" + viewport.height + "-dpr" + viewport.dpr;
      const result = { label, locale, viewport, freshBrowserContext: true,
        evidenceBoundary: "Production browser fixture; UI geometry and navigation only. Native output, queue/Take, live FX writes and audio IPC are covered by separate gates, not inferred here.",
        checks: {}, measurements: {}, screenshots: [] };
      let contextId = null, page = null;
      const capture = async (phase) => {
        const filename = label + "-" + phase + ".png";
        const shot = await page.send("Page.captureScreenshot", { format: "png", fromSurface: true, captureBeyondViewport: false });
        writeFileSync(join(screenshotDir, filename), shot.data, "base64"); result.screenshots.push(filename);
      };
      const probe = async (phase) => {
        const measured = await page.evaluate("(" + exerciseBatch3DensityInPage.toString() + ")(" + JSON.stringify(phase) + ")");
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
        page = new CdpClient(target.webSocketDebuggerUrl); await page.connect();
        await page.send("Page.enable"); await page.send("Runtime.enable");
        await page.send("Emulation.setDeviceMetricsOverride", { width: viewport.width, height: viewport.height, deviceScaleFactor: viewport.dpr, mobile: false });
        await page.send("Page.addScriptToEvaluateOnNewDocument", { source:
          "localStorage.setItem('syndocal.uiLocale.v1'," + JSON.stringify(locale) + ");" +
          "localStorage.setItem('syndocal.uiScale.v1','100');localStorage.setItem('syndocal.sceneFxEditorWide.v1','0');" });
        for (const [surface, fixture, ready, phases] of [
          ["matrix", "vj-bank", "[data-video-perform-matrix][data-slot-count]", ["matrix-initial", "matrix-pages", "matrix-offpage", "matrix-keyboard"]],
          ["quickfx", "operator-vj", "[data-perform-quick-fx] [data-perform-fx-scroll]", ["quickfx-parameters", "quickfx-chain", "quickfx-images", "quickfx-restored"]],
          ["scenefx", "scene-matrix", '[data-scene-settings-effect-editor="Color"] .colorEffectEditor', ["scenefx-selected", "scenefx-targets", "scenefx-add", "scenefx-rack", "scenefx-wide"]],
          ["timeline", "timeline-layered", "[data-timeline-tools-tab]", ["timeline-bank", "timeline-create", "timeline-phases", "timeline-audio", "timeline-restored"]],
        ]) {
          const url = new URL(appUrl); url.searchParams.set("syndocalViewportFixture", fixture);
          await page.send("Page.navigate", { url: url.toString() }); await waitForApp(page);
          result.measurements[surface + "Navigation"] = await page.evaluate("(" + navigateBatch3InPage.toString() + ")(" + JSON.stringify(surface) + ")");
          await waitFor(page, "Boolean(document.querySelector(" + JSON.stringify(ready) + "))", surface);
          await pause(150);
          for (const phase of phases) await probe(phase);
        }
        const observed = await page.evaluate("({locale:document.documentElement.lang,width:innerWidth,height:innerHeight,dpr:devicePixelRatio})");
        result.observed = observed;
        result.checks.localeAndViewportExact = observed.locale === locale && observed.width === viewport.width && observed.height === viewport.height && observed.dpr === viewport.dpr;
        result.checks.scenarioCompleted = true;
      } catch (error) {
        result.error = error instanceof Error ? error.stack : String(error); result.checks.scenarioCompleted = false;
        if (page) try { await capture("failure"); } catch {}
      } finally {
        result.runtimeExceptions = page?.runtimeExceptions ?? [];
        result.checks.noRuntimeExceptions = result.runtimeExceptions.length === 0;
        result.passed = Object.keys(result.checks).length > 1 && Object.values(result.checks).every(Boolean);
        result.failedChecks = Object.entries(result.checks).filter(([, pass]) => !pass).map(([name]) => name);
        writeFileSync(join(screenshotDir, label + ".json"), JSON.stringify(result, null, 2) + "\n");
        results.push(result); console.log((result.passed ? "pass " : "fail ") + label + " failed=" + JSON.stringify(result.failedChecks));
        page?.close(); if (contextId) await browser.send("Target.disposeBrowserContext", { browserContextId: contextId });
      }
    }
  } finally {
    browser.close();
    writeFileSync(join(screenshotDir, "ui-density-batch3-results.json"), JSON.stringify(results, null, 2) + "\n");
  }
  const failures = results.filter((result) => !result.passed);
  if (results.length !== 6 || failures.length) throw new Error("Batch3 density failed: " + JSON.stringify(failures.map(({ label, failedChecks, error }) => ({ label, failedChecks, error }))));
  console.log("Batch3 density passed: 6 fresh contexts, EN/JA, owner DPR2 and two desktop sizes; 4 production fixtures.");
}
