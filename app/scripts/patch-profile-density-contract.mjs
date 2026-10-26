import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// This gate is deliberately separate from --patch-only. The existing Patch
// selection, transaction, drag and address-grid contracts remain unchanged.
const densityViewports = [
  { width: 1280, height: 752, dpr: 2 },
  { width: 1366, height: 768, dpr: 1 },
  { width: 1920, height: 1080, dpr: 1 },
];
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function installDensityFixtureInPage() {
  const baseInvoke = window.__TAURI_INTERNALS__.invoke;
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const state = {
    kind: "explicit density fixture augmenting the existing Patch Share mock",
    importCalls: 0, saveCalls: [], loadCalls: 0, createCalls: [], augmentation: null,
  };
  const matrix = (y) => [1, 0, 0, 0, 0, 1, 0, y, 0, 0, 1, 0, 0, 0, 0, 1];
  const enrich = (original) => {
    const profile = clone(original);
    const originalFunctionCount = profile.dmx_modes.flatMap((mode) => mode.controls)
      .reduce((sum, control) => sum + (control.functions?.length ?? 0), 0);
    const originalGeometryCount = profile.geometries.length;
    if (originalGeometryCount === 0) {
      profile.geometries = ["Body", "Head", "Beam"].map((name, index) => ({
        name, kind: index === 2 ? "Beam" : "Geometry",
        parent: index === 0 ? null : ["Body", "Head"][index - 1],
        matrix: matrix(index * 0.25), model_primitive: index === 2 ? "Cylinder" : "Cube",
        model_dimensions: { x: 0.2, y: 0.3, z: 0.2 },
      }));
    }
    if (originalFunctionCount === 0) {
      // The stock mocked cached profile has eight channels and no functions.
      // Twenty-six distinct ranges exercise three 12-entry pages.
      let ordinal = 0;
      profile.dmx_modes[0].controls = profile.dmx_modes[0].controls.map((control, index) => ({
        ...control, geometry: ["Body", "Head", "Beam"][index % 3],
        functions: Array.from({ length: index < 2 ? 4 : 3 }, (_, range) => {
          const count = index < 2 ? 4 : 3;
          ordinal += 1;
          return {
            name: "Density range " + String(ordinal).padStart(2, "0"),
            attribute: control.attribute,
            dmx_from: Math.floor(range * 256 / count),
            dmx_to: Math.floor((range + 1) * 256 / count) - 1,
            physical_from: range, physical_to: range + 1,
          };
        }),
      }));
    }
    if (profile.dmx_modes.length === 1) {
      profile.dmx_modes.push({
        name: "Density compact",
        controls: clone(profile.dmx_modes[0].controls.slice(0, 2)).map((control) => ({
          ...control, functions: control.functions.slice(0, 1),
        })),
      });
    }
    state.augmentation = {
      originalFunctionCount, originalGeometryCount,
      syntheticFunctionsAdded: originalFunctionCount === 0,
      syntheticGeometryAdded: originalGeometryCount === 0,
      geometryNames: profile.geometries.map((geometry) => geometry.name),
      modeNames: profile.dmx_modes.map((mode) => mode.name),
      modeCellCounts: profile.dmx_modes.map((mode) => mode.controls.reduce((sum, control) => sum + control.offsets.length, 0)),
      modeFunctionCounts: profile.dmx_modes.map((mode) => mode.controls.reduce((sum, control) => sum + control.functions.length, 0)),
    };
    return profile;
  };
  const loadedCustom = {
    source_path: "memory://density-loaded-profile",
    manufacturer: "Density QA", name: "Density loaded profile", short_name: "Density loaded",
    fixture_type_id: null, warnings: [], geometries: [],
    dmx_modes: [{ name: "Loaded mode", controls: [
      { attribute: "Dimmer", channel_name: "Dimmer", offsets: [1], resolution: "EightBit", default_value: 0, functions: [] },
      { attribute: "Pan", channel_name: "Pan", offsets: [2, 3], resolution: "SixteenBit", default_value: 0, functions: [] },
    ] }],
  };
  window.__TAURI_INTERNALS__.invoke = async (command, args = {}) => {
    if (command === "import_gdtf") {
      state.importCalls += 1;
      return enrich(await baseInvoke(command, args));
    }
    if (command === "save_custom_fixture_profile") {
      state.saveCalls.push(clone(args.request));
      return "memory://density-saved-profile";
    }
    if (command === "load_custom_fixture_profile") {
      state.loadCalls += 1;
      return clone(loadedCustom);
    }
    if (command === "preview_custom_fixture_profile") state.createCalls.push(clone(args.request));
    return await baseInvoke(command, args);
  };
  window.__syndocalPatchProfileDensityFixture = state;
  window.__syndocalPatchProfileDensityProof = {};
}

// Runs only against the named foreground surface. Ancestor clips are included;
// other panes, hidden tab panels and closed details bodies are not candidates.
async function exerciseDensityInPage(phase) {
  const checks = {};
  const evidence = {};
  const check = (name, condition, detail) => {
    checks[name] = Boolean(condition);
    if (detail !== undefined) evidence[name] = detail;
  };
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const wait = async (predicate) => {
    for (let attempt = 0; attempt < 100; attempt += 1) {
      if (predicate()) return true;
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
    return false;
  };
  const q = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const rendered = (element) => {
    if (!(element instanceof HTMLElement) || !element.isConnected) return false;
    const box = element.getBoundingClientRect();
    if (box.width <= 0 || box.height <= 0) return false;
    for (let node = element; node instanceof HTMLElement; node = node.parentElement) {
      const style = getComputedStyle(node);
      if (node.hidden || style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse") return false;
      if (node instanceof HTMLDetailsElement && node !== element && !node.open
        && !q(":scope > summary", node)?.contains(element)) return false;
    }
    return true;
  };
  const rect = (element) => {
    const box = element.getBoundingClientRect();
    return { x: box.x, y: box.y, width: box.width, height: box.height, right: box.right, bottom: box.bottom };
  };
  const identity = (element) => element.id || element.getAttribute("data-loaded-profile-panel")
    || element.getAttribute("data-custom-profile-action") || element.className || element.tagName;
  const overflow = (element, axis) => {
    const style = getComputedStyle(element);
    return /(auto|scroll)/.test(axis === "x" ? style.overflowX : style.overflowY)
      && (axis === "x" ? element.scrollWidth > element.clientWidth + 1 : element.scrollHeight > element.clientHeight + 1);
  };
  const inspect = (element) => {
    if (!rendered(element)) return { rendered: false, target: element ? identity(element) : "missing" };
    const box = rect(element);
    const clip = { x: 0, y: 0, right: innerWidth, bottom: innerHeight };
    const clips = [];
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
      const style = getComputedStyle(parent);
      const p = rect(parent);
      const left = p.x + parent.clientLeft, top = p.y + parent.clientTop;
      const right = left + parent.clientWidth, bottom = top + parent.clientHeight;
      if (style.overflowX !== "visible") { clip.x = Math.max(clip.x, left); clip.right = Math.min(clip.right, right); }
      if (style.overflowY !== "visible") { clip.y = Math.max(clip.y, top); clip.bottom = Math.min(clip.bottom, bottom); }
      if ((style.overflowX !== "visible" && (box.x < left - 1 || box.right > right + 1))
        || (style.overflowY !== "visible" && (box.y < top - 1 || box.bottom > bottom + 1))) clips.push(identity(parent));
    }
    const contained = box.x >= clip.x - 1 && box.right <= clip.right + 1
      && box.y >= clip.y - 1 && box.bottom <= clip.bottom + 1;
    const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
    return { rendered: true, target: identity(element), box, clips, contained,
      hit: hit === element || element.contains(hit), height: element.offsetHeight, width: element.offsetWidth,
      fontSize: parseFloat(getComputedStyle(element).fontSize) };
  };
  const reachable = (item) => item.rendered && item.contained && item.hit && item.onlyOwnerVerticalScroll !== false;
  const reveal = async (element) => {
    if (!(element instanceof HTMLElement)) return inspect(element);
    const loadedScope = element.closest(".loadedProfileSummaryPanel");
    const loadedOwner = loadedScope?.closest(".fixtureSetupContextPane");
    const monitored = loadedScope ? [document.documentElement, document.body, ...all("*", loadedScope)] : [];
    if (loadedScope) for (let parent = loadedScope; parent && parent !== document.body; parent = parent.parentElement) monitored.push(parent);
    const before = [...new Set(monitored)].map((node) => ({ node, left: node.scrollLeft, top: node.scrollTop }));
    const details = [];
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
      if (parent instanceof HTMLDetailsElement && !parent.open && !q(":scope > summary", parent)?.contains(element)) details.push(parent);
    }
    for (const parent of details.reverse()) { q(":scope > summary", parent)?.click(); await settle(); }
    for (let attempt = 0; attempt < 3; attempt += 1) {
      for (let parent = element.parentElement; parent && parent !== document.body; parent = parent.parentElement) {
        // Loaded content has natural height. Its enclosing context pane is
        // the only permitted scroll owner; never rescue a clipped child by
        // scrolling another ancestor or moving horizontally.
        if (loadedScope && parent !== loadedOwner) continue;
        if (parent.matches(".app, .layout")) continue;
        const box = rect(element), p = rect(parent), style = getComputedStyle(parent);
        if (/(auto|scroll)/.test(style.overflowY)) {
          if (box.y < p.y + parent.clientTop + 2) parent.scrollTop -= p.y + parent.clientTop + 2 - box.y;
          else if (box.bottom > p.y + parent.clientTop + parent.clientHeight - 2) parent.scrollTop += box.bottom - p.y - parent.clientTop - parent.clientHeight + 2;
        }
        if (!loadedScope && /(auto|scroll)/.test(style.overflowX)) {
          if (box.x < p.x + parent.clientLeft + 2) parent.scrollLeft -= p.x + parent.clientLeft + 2 - box.x;
          else if (box.right > p.x + parent.clientLeft + parent.clientWidth - 2) parent.scrollLeft += box.right - p.x - parent.clientLeft - parent.clientWidth + 2;
        }
      }
      await settle();
      if (reachable(inspect(element))) break;
    }
    const state = inspect(element);
    if (loadedScope) {
      state.onlyOwnerVerticalScroll = Boolean(loadedOwner) && before.every(({ node, left, top }) =>
        node.scrollLeft === left && (node === loadedOwner || node.scrollTop === top));
      state.scrollMovement = before.filter(({ node, left, top }) => node.scrollLeft !== left || node.scrollTop !== top)
        .map(({ node, left, top }) => ({ target: identity(node), beforeLeft: left, beforeTop: top,
          afterLeft: node.scrollLeft, afterTop: node.scrollTop, owner: node === loadedOwner }));
    }
    return state;
  };
  const click = async (selector, name = selector) => {
    const element = q(selector);
    const state = await reveal(element);
    check(name + "Reachable", reachable(state) && !element?.disabled, state);
    if (!element || element.disabled) throw new Error("Missing or disabled control: " + selector);
    element.click();
    await settle();
    return element;
  };
  const input = async (selector, value, name = selector) => {
    const element = q(selector);
    check(name + "Reachable", reachable(await reveal(element)));
    if (!(element instanceof HTMLInputElement || element instanceof HTMLSelectElement)) throw new Error("Missing input: " + selector);
    element.value = value;
    element.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  };
  const viewport = () => {
    const roots = [document.documentElement, document.body, q(".app"), q(".layout")].filter(Boolean);
    return { width: innerWidth, height: innerHeight, dpr: devicePixelRatio, lang: document.documentElement.lang,
      roots: roots.map((element) => ({ target: identity(element), width: element.clientWidth,
        height: element.clientHeight, scrollWidth: element.scrollWidth, scrollHeight: element.scrollHeight,
        left: element.scrollLeft, top: element.scrollTop })),
      windowLeft: scrollX, windowTop: scrollY };
  };
  const checkViewport = () => {
    const state = viewport();
    check("viewportOverflowZero", state.windowLeft === 0 && state.windowTop === 0
      && state.roots.every((item) => item.scrollWidth <= item.width + 1 && item.scrollHeight <= item.height + 1 && item.left === 0 && item.top === 0), state);
  };
  const measure = (scope) => {
    if (!rendered(scope)) throw new Error("Foreground scope is missing for " + phase);
    const nodes = [scope, ...all("*", scope)].filter(rendered);
    const ancestors = [];
    for (let parent = scope.parentElement; parent && parent !== document.body; parent = parent.parentElement) ancestors.push(parent);
    const vertical = [...nodes, ...ancestors].filter((element) => overflow(element, "y"));
    const nested = vertical.filter((element) => vertical.some((other) => other !== element && other.contains(element)));
    const loadedOwner = scope.matches(".loadedProfileSummaryPanel") ? scope.closest(".fixtureSetupContextPane") : null;
    const horizontal = [...nodes, ...(loadedOwner ? [loadedOwner] : [])].filter((element) => overflow(element, "x"));
    const headings = all("h1,h2,h3,.customProfilePreviewHeader > strong,.loadedProfileHeader > strong", scope)
      .filter(rendered).map((element) => {
        const range = document.createRange(); range.selectNodeContents(element);
        const box = rect(element), textBox = range.getBoundingClientRect();
        const lineTops = [...new Set([...range.getClientRects()].filter((item) => item.width > 0 && item.height > 0).map((item) => Math.round(item.top)))];
        // Font ink may extend above a shorter line box (14px text with
        // line-height:1.2). Only an actual clipping box can hide that ink.
        // Keep the own-width/single-line contract and inspect every real
        // overflow clip, including the heading itself when it clips.
        const clip = { x: 0, y: 0, right: innerWidth, bottom: innerHeight };
        const clippingAncestors = [];
        for (let node = element; node; node = node.parentElement) {
          const style = getComputedStyle(node);
          if (style.display === "inline") continue;
          const clipsX = style.overflowX !== "visible", clipsY = style.overflowY !== "visible";
          if (!clipsX && !clipsY) continue;
          const boundary = rect(node);
          const left = boundary.x + node.clientLeft, top = boundary.y + node.clientTop;
          const right = left + node.clientWidth, bottom = top + node.clientHeight;
          if (clipsX) { clip.x = Math.max(clip.x, left); clip.right = Math.min(clip.right, right); }
          if (clipsY) { clip.y = Math.max(clip.y, top); clip.bottom = Math.min(clip.bottom, bottom); }
          clippingAncestors.push({ target: identity(node), clipsX, clipsY, left, top, right, bottom });
        }
        const ownWidthFits = textBox.left >= box.x - 1 && textBox.right <= box.right + 1;
        const inkInsideClips = textBox.left >= clip.x - 1 && textBox.right <= clip.right + 1
          && textBox.top >= clip.y - 1 && textBox.bottom <= clip.bottom + 1;
        return { text: element.textContent.trim(), lines: lineTops.length, fits: ownWidthFits && inkInsideClips,
          ownWidthFits, inkInsideClips, box, textBox: { x: textBox.x, y: textBox.y, width: textBox.width,
            height: textBox.height, right: textBox.right, bottom: textBox.bottom }, clip, clippingAncestors };
      });
    const describe = (element) => ({ target: identity(element), width: element.clientWidth, scrollWidth: element.scrollWidth,
      height: element.clientHeight, scrollHeight: element.scrollHeight });
    const state = { scope: inspect(scope), horizontal: horizontal.map(describe), vertical: vertical.map(describe),
      nested: nested.map(describe), headings, scopeWidth: scope.clientWidth, scopeScrollWidth: scope.scrollWidth };
    if (scope.matches(".loadedProfileSummaryPanel")) {
      const owner = loadedOwner ? inspect(loadedOwner) : null;
      check("scrollOwnerContained", Boolean(owner?.contained), owner);
      if (loadedOwner) {
        const box = rect(loadedOwner), style = getComputedStyle(loadedOwner);
        const left = box.x + loadedOwner.clientLeft + parseFloat(style.paddingLeft);
        const right = box.x + loadedOwner.clientLeft + loadedOwner.clientWidth - parseFloat(style.paddingRight);
        check("contentWithinOwnerWidth", state.scope.box.x >= left - 1 && state.scope.box.right <= right + 1,
          { content: state.scope.box, ownerContentLeft: left, ownerContentRight: right });
        check("ownerHorizontalScrollZero", loadedOwner.scrollWidth <= loadedOwner.clientWidth + 1 && loadedOwner.scrollLeft === 0,
          describe(loadedOwner));
        check("loadedContextIsOnlyScrollOwner", /(auto|scroll)/.test(style.overflowY)
          && vertical.every((element) => element === loadedOwner), state.vertical);
      }
    } else {
      check("scopeContained", state.scope.contained, state.scope);
    }
    check("noHorizontalScroll", horizontal.length === 0 && scope.scrollWidth <= scope.clientWidth + 1, state.horizontal);
    check("noNestedScroll", nested.length === 0, { vertical: state.vertical, nested: state.nested });
    check("headingsSingleLineAndUnclipped", headings.length > 0 && headings.every((heading) => heading.lines === 1 && heading.fits), headings);
    if (scope.matches(".loadedProfileSummaryPanel")) {
      const panel = all("[data-loaded-profile-panel]", scope).find(rendered);
      const parent = panel?.parentElement;
      check("activePanelUsesFullWidth", panel && parent && panel.getBoundingClientRect().width >= parent.clientWidth - 2,
        panel ? { panelWidth: panel.getBoundingClientRect().width, availableWidth: parent?.clientWidth } : null);
      const pager = all("[data-loaded-profile-page]", scope).filter(rendered);
      check("pagerControlSizesPreserved", pager.every((button) => button.offsetHeight >= 44 && parseFloat(getComputedStyle(button).fontSize) >= 14),
        pager.map(inspect));
    }
    const textNodes = all(".profileDmxCell small,.profileDmxCell strong,.profileDmxCell span,.customProfileAttributeRow label", scope).filter(rendered);
    check("textSizePreserved", textNodes.every((element) => parseFloat(getComputedStyle(element).fontSize) >= 13),
      textNodes.map((element) => ({ target: identity(element), fontSize: parseFloat(getComputedStyle(element).fontSize) })));
    checkViewport();
    return { state, vertical };
  };
  const loaded = () => q(".loadedProfileSummaryPanel");
  const custom = () => q("[data-custom-profile-workspace]");
  const fixture = window.__syndocalPatchProfileDensityFixture;
  const proof = window.__syndocalPatchProfileDensityProof;
  const activeTab = () => q('[data-loaded-profile-tab][aria-selected="true"]')?.getAttribute("data-loaded-profile-tab");
  const tabState = (wanted) => {
    const tabs = all("[data-loaded-profile-tab]", loaded());
    const panels = all("[data-loaded-profile-panel]", loaded()).filter(rendered);
    check("oneActivePanel", panels.length === 1 && panels[0].getAttribute("data-loaded-profile-panel") === wanted,
      panels.map((element) => element.getAttribute("data-loaded-profile-panel")));
    check("tabSemantics", tabs.length === 3 && tabs.every((tab) => tab.getAttribute("role") === "tab")
      && tabs.filter((tab) => tab.getAttribute("aria-selected") === "true").length === 1
      && activeTab() === wanted && panels.every((panel) => panel.getAttribute("role") === "tabpanel"));
    check("rovingTabStop", tabs.filter((tab) => tab.tabIndex === 0).length === 1
      && tabs.find((tab) => tab.tabIndex === 0)?.getAttribute("data-loaded-profile-tab") === wanted);
    check("tabPanelRelationships", tabs.every((tab) => {
      const panel = document.getElementById(tab.getAttribute("aria-controls"));
      return panel && panel.getAttribute("aria-labelledby") === tab.id
        && panel.getAttribute("data-loaded-profile-panel") === tab.getAttribute("data-loaded-profile-tab");
    }));
    check("tabTargetsPreserved", tabs.every((tab) => tab.offsetHeight >= 44 && parseFloat(getComputedStyle(tab).fontSize) >= 14),
      tabs.map(inspect));
    const selected = tabs.find((tab) => tab.getAttribute("aria-selected") === "true");
    const style = selected ? getComputedStyle(selected) : null;
    const parseSrgb = (value) => {
      const rgb = value?.match(/^rgba?\((.*)\)$/i);
      const color = rgb || value?.match(/^color\(srgb\s+(.+)\)$/i);
      if (!color) return null;
      const parts = color[1].replace(/[,/]/g, " ").trim().split(/\s+/);
      if (parts.length < 3 || parts.length > 4) return null;
      const channels = parts.slice(0, 3).map((part) =>
        part.endsWith("%") ? parseFloat(part) / 100 : Number(part) / (rgb ? 255 : 1));
      const alpha = parts.length === 3 ? 1
        : parts[3].endsWith("%") ? parseFloat(parts[3]) / 100 : Number(parts[3]);
      if (![...channels, alpha].every((value) => Number.isFinite(value) && value >= 0 && value <= 1)) return null;
      return { channels, alpha };
    };
    const foreground = parseSrgb(style?.color);
    const background = parseSrgb(style?.backgroundColor);
    // Do not silently ignore alpha: this opaque tab has no compositing step.
    // A transparent future style must supply a real compositing proof.
    const opaque = foreground?.alpha === 1 && background?.alpha === 1;
    const luminance = ({ channels }) => channels.map((value) =>
      value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
      .reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);
    const foregroundLuminance = opaque ? luminance(foreground) : null;
    const backgroundLuminance = opaque ? luminance(background) : null;
    const contrastRatio = opaque
      ? (Math.max(foregroundLuminance, backgroundLuminance) + 0.05) / (Math.min(foregroundLuminance, backgroundLuminance) + 0.05)
      : null;
    const contrast = { tab: wanted, foregroundCss: style?.color ?? null, backgroundCss: style?.backgroundColor ?? null,
      foreground, background, opaque, foregroundLuminance, backgroundLuminance, contrastRatio, minimum: 4.5 };
    check("activeTabColorsOpaque", opaque, contrast);
    check("activeTabContrastAtLeast4_5", opaque && contrastRatio >= 4.5, contrast);
  };
  const restoreTop = async (scope) => {
    if (scope.matches(".loadedProfileSummaryPanel")) {
      const owner = scope.closest(".fixtureSetupContextPane");
      if (!owner) throw new Error("Loaded Profile context scroll owner is missing");
      owner.scrollTop = 0;
      await settle();
      check("ownerRestoredToTop", owner.scrollTop === 0);
      return;
    }
    for (let element = scope; element && !element.matches(".app,.layout,body"); element = element.parentElement) element.scrollTop = 0;
    for (const element of all("*", scope)) if (overflow(element, "y")) element.scrollTop = 0;
    await settle();
  };
  const fields = () => ["manufacturer", "name", "mode", "attributes"].map((name) => q('[data-custom-profile-field="' + name + '"]')?.value);
  const drafts = () => all(".customProfileAttributeRow", custom()).map((row) => ({
    attribute: q("[data-custom-profile-attribute]", row)?.value,
    bits: q("[data-custom-profile-bits]", row)?.value,
    start: q("[data-custom-profile-start]", row)?.value,
    selected: row.classList.contains("selected"),
  }));
  const action = (name) => '[data-custom-profile-action="' + name + '"]';
  const customLayout = async () => {
    const scope = custom(), layout = scope.closest(".layout"), box = rect(scope), parent = rect(layout), style = getComputedStyle(layout);
    const width = layout.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    const height = layout.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
    check("customUsesFullWorkspace", box.width >= width - 2 && box.height >= height - 2, { box, layout: parent, availableWidth: width, availableHeight: height });
    check("mapAndBandHidden", all(".setupPatchAddressDesk,[data-workspace-pane='lower']").filter(rendered).length === 0);
    const back = await reveal(q("[data-custom-workbench-back]"));
    // Back retains its measured 13px baseline; its target grew from 40 to 44.
    // Other existing Custom controls retain their 14px and 28/43px floors.
    check("backUnclippedAndReachable", reachable(back) && back.height >= 44 && back.fontSize >= 13, back);
    const controls = all("input,select,button", scope).filter(rendered).map((element) => {
      const isBack = element.matches("[data-custom-workbench-back]");
      return { ...inspect(element), minimumHeight: isBack ? 44 : element.matches(".customProfileDmxCell") ? 43 : 28,
        minimumFontSize: isBack ? 13 : 14 };
    });
    check("customControlSizesPreserved", controls.every((item) => item.height >= item.minimumHeight && item.fontSize >= item.minimumFontSize), controls);
    check("customPreviewCellWidthPreserved", all(".customProfileDmxCell", scope).filter(rendered).every((element) => element.offsetWidth >= 48));
    await restoreTop(scope);
    return measure(scope);
  };

  if (phase.startsWith("fixture-selected-")) {
    check("originalSelectedFixtureContract", all(".fixtureSetupEditor").filter(rendered).length === 1
      && all("[data-patch-profile-summary]").filter(rendered).length === 0);
    checkViewport();
  } else if (phase === "loaded-dmx") {
    check("armedProfileWithoutFixture", all("[data-patch-profile-summary]").filter(rendered).length === 1
      && all(".fixtureSetupEditor").filter(rendered).length === 0);
    check("explicitFixturePopulated", fixture.augmentation?.modeFunctionCounts[0] === 26
      && fixture.augmentation?.geometryNames.length === 3, fixture.augmentation);
    tabState("dmx");
    const sequence = [];
    for (const [key, wanted] of [["ArrowRight", "functions"], ["ArrowLeft", "dmx"], ["End", "geometry"], ["Home", "dmx"]]) {
      const tab = q('[data-loaded-profile-tab="' + activeTab() + '"]');
      await reveal(tab); tab.focus({ preventScroll: true });
      tab.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
      await settle();
      sequence.push({ key, wanted, selected: activeTab(), focused: document.activeElement?.getAttribute("data-loaded-profile-tab") });
    }
    check("tabKeyboardNavigation", sequence.every((item) => item.selected === item.wanted && item.focused === item.wanted), sequence);
    const cells = all(".profileDmxCell", loaded()).filter(rendered);
    check("dmxCellsPreserved", cells.length === fixture.augmentation.modeCellCounts[0]
      && cells.every((cell) => cell.offsetWidth >= 62 && cell.offsetHeight >= 45), cells.map(inspect));
    const lastCell = await reveal(cells.at(-1));
    check("lastDmxCellReachableThroughOwner", reachable(lastCell), lastCell);
    check("modeControlPreserved", q(".loadedProfileModeField select").offsetHeight >= 30);
    await restoreTop(loaded());
    measure(loaded());
  } else if (phase === "loaded-functions") {
    await click('[data-loaded-profile-tab="functions"]', "functionsTab");
    tabState("functions");
    const pages = [];
    for (let page = 0; page < 3; page += 1) {
      const rows = all(".profileFunctionRow", loaded()).filter(rendered);
      pages.push(rows.map((row) => row.textContent.replace(/\s+/g, " ").trim()));
      check("page" + page + "HasExpectedCount", rows.length === [12, 12, 2][page]);
      const lastRow = await reveal(rows.at(-1));
      check("page" + page + "LastRowReachableThroughOwner", reachable(lastRow), lastRow);
      const controls = [];
      for (const control of all("[data-loaded-profile-page]", loaded()).filter(rendered)) controls.push(await reveal(control));
      check("page" + page + "ControlsReachableThroughOwner", controls.length === 2 && controls.every(reachable), controls);
      if (page < 2) await click('[data-loaded-profile-page="next"]', "nextPage" + page);
    }
    check("allFunctionRangesReachable", pages.flat().length === 26 && new Set(pages.flat()).size === 26, pages);
    check("lastPageDisablesNext", q('[data-loaded-profile-page="next"]')?.disabled === true);
    await click('[data-loaded-profile-page="previous"]', "previousPage");
    check("previousPageRestoresRows", JSON.stringify(all(".profileFunctionRow", loaded()).filter(rendered).map((row) => row.textContent.replace(/\s+/g, " ").trim())) === JSON.stringify(pages[1]));
    await input(".loadedProfileModeField select", fixture.augmentation.modeNames[1], "compactMode");
    check("modeChangeUpdatesFunctions", all(".profileFunctionRow", loaded()).filter(rendered).length === fixture.augmentation.modeFunctionCounts[1]);
    await click('[data-loaded-profile-tab="dmx"]', "compactDmxTab");
    check("modeChangeUpdatesDmx", all(".profileDmxCell", loaded()).filter(rendered).length === fixture.augmentation.modeCellCounts[1]);
    await input(".loadedProfileModeField select", fixture.augmentation.modeNames[0], "restoreMode");
    check("modeRestoreUpdatesDmx", all(".profileDmxCell", loaded()).filter(rendered).length === fixture.augmentation.modeCellCounts[0]);
    await click('[data-loaded-profile-tab="functions"]', "functionsTabAfterMode");
    check("modeChangeResetsPagination", q('[data-loaded-profile-page="previous"]')?.disabled === true
      && all(".profileFunctionRow", loaded()).filter(rendered).length === 12);
    await restoreTop(loaded());
    measure(loaded());
  } else if (phase === "loaded-geometry") {
    await click('[data-loaded-profile-tab="geometry"]', "geometryTab");
    tabState("geometry");
    const rows = all(".profileGeometryRow", loaded()).filter(rendered);
    check("geometryUsesClosedDetails", rows.length === 3 && rows.every((row) => row instanceof HTMLDetailsElement && !row.open));
    check("geometrySummariesRetainIdentity", rows.every((row, index) => {
      const summary = q(":scope > summary", row);
      return summary && summary.textContent.includes(fixture.augmentation.geometryNames[index]) && summary.offsetHeight >= 44;
    }));
    const head = rows[1];
    if (!(head instanceof HTMLDetailsElement)) throw new Error("Geometry disclosure missing");
    const summary = q(":scope > summary", head);
    check("geometryDisclosureReachable", reachable(await reveal(summary)));
    summary.click(); await settle();
    check("geometryDetailsAvailable", head.open && head.textContent.includes("Body") && head.textContent.includes("0.25"), head.textContent);
    const lastGeometry = rows.at(-1);
    const lastSummary = q(":scope > summary", lastGeometry);
    const summaryState = await reveal(lastSummary);
    check("lastGeometrySummaryReachableThroughOwner", reachable(summaryState), summaryState);
    lastSummary.click(); await settle();
    const detailTail = q(".profileGeometryDetails", lastGeometry)?.lastElementChild;
    const tailState = await reveal(detailTail);
    check("expandedGeometryTailReachableThroughOwner", lastGeometry.open && reachable(tailState), tailState);
    await restoreTop(loaded());
    measure(loaded());
  } else if (phase === "custom-initial") {
    check("customActionsPresent", ["load", "save", "create"].every((name) => rendered(q(action(name)))));
    await customLayout();
  } else if (phase === "custom-edit") {
    await click(action("clear"), "clear");
    check("emptyDraftGuards", drafts().length === 0 && q(action("save")).disabled && q(action("create")).disabled);
    await click('[data-custom-profile-template="1"]', "panTiltTemplate");
    check("templateEditsDraft", drafts().length === 2 && drafts().every((row) => row.bits === "SixteenBit"), drafts());
    await input('[data-custom-profile-field="manufacturer"]', "Density QA", "maker");
    await input('[data-custom-profile-field="name"]', "Density draft", "profileName");
    await input('[data-custom-profile-field="mode"]', "Density mode", "modeName");
    await input('[data-custom-profile-attribute="0"]', "Pan Audit", "attribute");
    await input('[data-custom-profile-bits="0"]', "EightBit", "bits");
    await input('[data-custom-profile-start="0"]', "5", "start");
    check("attributeInputsUpdateDraft", drafts()[0].attribute === "Pan Audit" && drafts()[0].bits === "EightBit" && drafts()[0].start === "5", drafts());
    await click(".customProfileAttributeRows > .customProfileAttributeRow:nth-child(2)", "attributeRowSelection");
    check("rowSelectsDraft", drafts()[1].selected && !drafts()[0].selected);
    await click('[data-custom-profile-channel="5"]', "previewSelection");
    check("previewSelectsDraft", drafts()[0].selected);
    await click(action("down"), "moveDown");
    check("moveDownRetainsSelection", drafts()[1].attribute === "Pan Audit" && drafts()[1].selected);
    await click(action("up"), "moveUp");
    check("moveUpRetainsSelection", drafts()[0].attribute === "Pan Audit" && drafts()[0].selected);
    await click(action("add"), "add");
    check("addDraft", drafts().length === 3);
    await click('[data-custom-profile-remove="2"]', "remove");
    check("removeDraft", drafts().length === 2);
    await input('[data-custom-profile-field="attributes"]', "Dimmer@1:8, Pan@1:16", "conflictingRawAttributes");
    check("invalidDraftGuards", q(action("save")).disabled && q(action("create")).disabled && rendered(q(".customProfilePreview.bad")));
    const longDraft = Array.from({ length: 20 }, (_, index) => "Generic: Density " + (index + 1) + "@" + (index + 1) + ":8").join(", ");
    await input('[data-custom-profile-field="attributes"]', longDraft, "longRawAttributes");
    check("rawAttributesUpdateDraftAndPreview", drafts().length === 20 && !q(action("save")).disabled && !q(action("create")).disabled);
    await click('[data-custom-profile-channel="20"]', "lastPreviewSelection");
    check("lastPreviewSelectsDraft", drafts()[19].selected);
    const last = q('[data-custom-profile-attribute="19"]');
    check("lastDraftReachableThroughOwner", reachable(await reveal(last)));
    proof.draftBeforeBack = { fields: fields(), drafts: drafts() };
    const layout = await customLayout();
    check("workspaceOwnsLongDraftScroll", layout.vertical.length === 1 && layout.vertical[0] === custom(),
      layout.state.vertical);
  } else if (phase === "custom-reopen") {
    const back = await reveal(q("[data-custom-workbench-back]"));
    check("backReachable", reachable(back), back);
    await click("[data-custom-workbench-back]", "back");
    check("backRestoresPatch", !rendered(custom()) && all(".setupPatchAddressDesk").filter(rendered).length === 1
      && all("[data-workspace-pane='lower']").filter(rendered).length === 1);
    await click("[data-patch-open-workbench]", "reopen");
    await wait(() => rendered(custom()));
    check("draftSurvivesBackAndReopen", JSON.stringify({ fields: fields(), drafts: drafts() }) === JSON.stringify(proof.draftBeforeBack),
      { before: proof.draftBeforeBack, after: { fields: fields(), drafts: drafts() } });
    await customLayout();
  } else if (phase === "custom-callbacks") {
    await click(action("save"), "save");
    await wait(() => fixture.saveCalls.length > 0);
    check("saveCallbackRetained", fixture.saveCalls.length === 1 && fixture.saveCalls[0].name === "Density draft"
      && fixture.saveCalls[0].attributes.length === 20, fixture.saveCalls);
    await click(action("create"), "create");
    await wait(() => fixture.createCalls.length > 0);
    check("createPreviewCallbackRetained", fixture.createCalls.length === 1 && fixture.createCalls[0].attributes.length === 20, fixture.createCalls);
    await click(action("load"), "load");
    await wait(() => q('[data-custom-profile-field="name"]')?.value === "Density loaded profile");
    check("loadCallbackRetained", fixture.loadCalls === 1 && fields()[0] === "Density QA"
      && fields()[1] === "Density loaded profile" && drafts().length === 2
      && drafts()[1].bits === "SixteenBit", { fields: fields(), drafts: drafts() });
    await customLayout();
  } else {
    throw new Error("Unknown density phase: " + phase);
  }
  return { checks, evidence };
}

export async function runPatchProfileDensityChecks({ CdpClient, cdpPort, appUrl, screenshotDir, waitForApp, installShareMock }) {
  if (!screenshotDir) throw new Error("--patch-profile-density-only requires SYNDOCAL_VIEWPORT_SCREENSHOT_DIR");
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
    for (const locale of ["en", "ja"]) {
      for (const viewport of densityViewports) {
        const label = "patch-profile-density-" + locale + "-" + viewport.width + "x" + viewport.height + "-dpr" + viewport.dpr;
        const result = { label, locale, viewport, evidenceBoundary: "Production browser with explicit in-memory IPC fixtures; no native or hardware proof.",
          freshBrowserContext: true, checks: {}, measurements: {}, screenshots: [] };
        let contextId = null, page = null;
        const capture = async (phase) => {
          const filename = label + "-" + phase + ".png";
          const screenshot = await page.send("Page.captureScreenshot", { format: "png", fromSurface: true, captureBeyondViewport: false });
          writeFileSync(join(screenshotDir, filename), screenshot.data, "base64");
          result.screenshots.push(filename);
        };
        const probe = async (phase, screenshot = true) => {
          const measured = await page.evaluate("(" + exerciseDensityInPage.toString() + ")(" + JSON.stringify(phase) + ")");
          result.measurements[phase] = measured.evidence;
          for (const [name, passed] of Object.entries(measured.checks)) result.checks[phase + "." + name] = passed;
          if (screenshot) await capture(phase);
        };
        try {
          contextId = (await browser.send("Target.createBrowserContext", { disposeOnDetach: true })).browserContextId;
          result.browserContextId = contextId;
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
            "localStorage.setItem('syndocal.uiScale.v1','100');" +
            "localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'setup',setup_sub_tab:'patch',control_mode:'edit',control_category:'position'}));",
          });
          await page.send("Page.navigate", { url: appUrl });
          await waitForApp(page);
          await page.evaluate("(" + installShareMock.toString() + ")()");
          await page.evaluate("(" + installDensityFixtureInPage.toString() + ")()");
          // Remount after installing the mock so admission/cache refresh uses it.
          await page.evaluate("document.querySelector('[data-workspace-option=\"control\"]').click()");
          await waitFor(page, "Boolean(document.querySelector('.layoutControl'))", "Patch unmounted");
          await page.evaluate("document.querySelector('[data-workspace-option=\"setup\"]').click()");
          await waitFor(page, "document.querySelector('[data-patch-refresh-profiles]')?.disabled === false", "Patch cache admission");
          await probe("fixture-selected-initial", false);
          await page.evaluate("document.querySelector('[data-patch-refresh-profiles]').click()");
          await waitFor(page, "Boolean(document.querySelector('[data-patch-profile-tree=\"cache\"] [data-profile-tree-item=\"manufacturer\"]'))", "cached profile");
          await page.evaluate("(() => {const input=document.querySelector('[data-patch-profile-search]'); input.value='ColorSource'; input.dispatchEvent(new Event('input',{bubbles:true}));})()");
          await waitFor(page, "Boolean(document.querySelector('[data-patch-profile-row][data-profile-source=\"cache\"]'))", "filtered cached mode");
          await page.evaluate("document.querySelector('[data-patch-profile-row][data-profile-source=\"cache\"]').click()");
          await waitFor(page, "window.__syndocalPatchProfileDensityFixture.importCalls === 1 && Boolean(window.__syndocalPatchProfileDensityFixture.augmentation)", "synthetic profile imported");
          await page.evaluate("window.__syndocalSetControlFixtureSelection([],null,'')");
          await waitFor(page, "Boolean(document.querySelector('[data-loaded-profile-tab=\"dmx\"]'))", "armed loaded-profile tabs");
          for (const phase of ["loaded-dmx", "loaded-functions", "loaded-geometry"]) await probe(phase);
          await page.evaluate("document.querySelector('.dmxAddressCell.occupied').click()");
          await probe("fixture-selected-restored", false);
          await page.evaluate("document.querySelector('[data-patch-open-workbench]').click()");
          await waitFor(page, "Boolean(document.querySelector('[data-custom-profile-workspace]'))", "custom workspace");
          for (const phase of ["custom-initial", "custom-edit", "custom-reopen", "custom-callbacks"]) await probe(phase);
          result.fixture = await page.evaluate("JSON.parse(JSON.stringify(window.__syndocalPatchProfileDensityFixture))");
          const observed = await page.evaluate("({locale:document.documentElement.lang,width:innerWidth,height:innerHeight,dpr:devicePixelRatio})");
          result.observed = observed;
          result.checks.localeAndViewportExact = observed.locale === locale && observed.width === viewport.width
            && observed.height === viewport.height && observed.dpr === viewport.dpr;
          result.checks.scenarioCompleted = true;
        } catch (error) {
          result.error = error instanceof Error ? error.stack : String(error);
          result.checks.scenarioCompleted = false;
          if (page) {
            try { result.fixture = await page.evaluate("JSON.parse(JSON.stringify(window.__syndocalPatchProfileDensityFixture ?? null))"); } catch {}
            try { await capture("failure"); } catch {}
          }
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
    }
  } finally {
    browser.close();
    writeFileSync(join(screenshotDir, "patch-profile-density-results.json"), JSON.stringify(results, null, 2) + "\n");
  }
  const failures = results.filter((result) => !result.passed);
  if (results.length !== 6 || failures.length) {
    throw new Error("Patch profile density failed: " + JSON.stringify(failures.map(({ label, failedChecks, error }) => ({ label, failedChecks, error }))));
  }
  console.log("Patch profile density passed: 6 fresh contexts, EN/JA, owner DPR2 and two desktop sizes.");
}
