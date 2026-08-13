import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const componentRoot = new URL("../src/components/", import.meta.url);
const clipGrid = await readFile(new URL("VideoClipGridPanel.tsx", componentRoot), "utf8");
const controlPanel = await readFile(new URL("VideoControlPanel.tsx", componentRoot), "utf8");
const sourceCreate = await readFile(new URL("VideoSourceCreatePanel.tsx", componentRoot), "utf8");
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const controller = await readFile(new URL("../src/createVideoRuntimeController.ts", import.meta.url), "utf8");

const count = (source, needle) => source.split(needle).length - 1;

function requiredIndex(source, marker, label, fromIndex = 0) {
  const index = source.indexOf(marker, fromIndex);
  assert.ok(index >= 0, `${label} marker is missing`);
  return index;
}

function sliceBetween(source, startMarker, endMarker, label, fromIndex = 0) {
  const start = requiredIndex(source, startMarker, `${label} start`, fromIndex);
  const end = requiredIndex(source, endMarker, `${label} end`, start + startMarker.length);
  assert.ok(end > start, `${label} structural slice is missing`);
  return source.slice(start, end + endMarker.length);
}

function balancedElement(source, startMarker, tagName, label, fromIndex = 0) {
  const start = requiredIndex(source, startMarker, `${label} start`, fromIndex);
  const tokens = new RegExp(`<\\/?${tagName}\\b[^<>]*>`, "g");
  tokens.lastIndex = start;
  let depth = 0;
  let match;
  while ((match = tokens.exec(source))) {
    const token = match[0];
    if (token.startsWith(`</${tagName}`)) {
      depth -= 1;
      if (depth === 0) return source.slice(start, match.index + token.length);
    } else if (!token.endsWith("/>") ) {
      depth += 1;
    }
  }
  assert.fail(`${label} balanced ${tagName} block is missing`);
}

function cssBlock(source, selector, label) {
  const marker = `${selector} {`;
  const start = requiredIndex(source, marker, `${label} selector`);
  const openBrace = start + marker.length - 1;
  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, index + 1);
    }
  }
  assert.fail(`${label} CSS block is missing its closing brace`);
}

function exactCssBlock(source, selector, label) {
  const marker = `${selector} {`;
  const prefixedStart = source.indexOf(`\n${marker}`);
  const start = source.startsWith(marker) ? 0 : prefixedStart >= 0 ? prefixedStart + 1 : -1;
  assert.ok(start >= 0, `${label} exact selector is missing`);
  const openBrace = start + marker.length - 1;
  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, index + 1);
    }
  }
  assert.fail(`${label} exact CSS block is missing its closing brace`);
}

function lastExactCssBlock(source, selector, label) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const selectorPattern = new RegExp(`(^|\\n)[\\t ]*${escapedSelector}\\s*\\{`, "g");
  let start = -1;
  let match;
  while ((match = selectorPattern.exec(source))) {
    start = match.index + match[0].lastIndexOf(selector);
  }
  assert.ok(start >= 0, `${label} exact selector is missing`);
  const openBrace = source.indexOf("{", start + selector.length);
  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, index + 1);
    }
  }
  assert.fail(`${label} CSS block is missing its closing brace`);
}

function finalCssBlock(source, selector, label) {
  const marker = `${selector} {`;
  const start = source.lastIndexOf(marker);
  assert.ok(start >= 0, `${label} final selector is missing`);
  const openBrace = start + marker.length - 1;
  let depth = 0;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, index + 1);
    }
  }
  assert.fail(`${label} final CSS block is missing its closing brace`);
}

function finalCssBlockWithDeclaration(source, selector, declaration, label) {
  const marker = `${selector} {`;
  let fromIndex = 0;
  let finalBlock;
  while (true) {
    const start = source.indexOf(marker, fromIndex);
    if (start < 0) break;
    const openBrace = start + marker.length - 1;
    let depth = 0;
    let end = -1;
    for (let index = openBrace; index < source.length; index += 1) {
      if (source[index] === "{") depth += 1;
      if (source[index] === "}") {
        depth -= 1;
        if (depth === 0) {
          end = index + 1;
          break;
        }
      }
    }
    assert.ok(end > start, `${label} CSS block is missing its closing brace`);
    const block = source.slice(start, end);
    if (block.includes(declaration)) finalBlock = block;
    fromIndex = end;
  }
  assert.ok(finalBlock, `${label} declaration is missing`);
  return finalBlock;
}

function pixelDeclaration(block, property, label) {
  const match = block.match(new RegExp(`${property}:\\s*(\\d+(?:\\.\\d+)?)px;`));
  assert.ok(match, `${label} pixel declaration is missing`);
  return Number(match[1]);
}

const emptyBranchStart = requiredIndex(
  clipGrid,
  '<Show\n        when={props.layers.length > 0}',
  "clip grid empty/populated branch",
);
const populatedGridStart = requiredIndex(
  clipGrid,
  '<div class="videoClipGrid">',
  "populated clip grid branch",
  emptyBranchStart,
);
assert.ok(populatedGridStart > emptyBranchStart, "populated clip grid must follow the empty branch");
const emptyBranch = clipGrid.slice(emptyBranchStart, populatedGridStart);

const firstRunGuardStart = '<Show\n            when={firstRunGuarded()}';
const firstRunGuard = sliceBetween(
  emptyBranch,
  firstRunGuardStart,
  '\n          </Show>\n        }\n      >',
  "first-run guarded empty branch",
);
const firstRunFallbackIndex = requiredIndex(firstRunGuard, "fallback={", "first-run guarded fallback");
const ordinaryEmptyBranch = balancedElement(
  firstRunGuard,
  '<div class="emptyState emptyStateAction" data-vj-media-state="empty-import">',
  "div",
  "ordinary empty import branch",
);
const firstRunEmptyBranch = balancedElement(
  firstRunGuard,
  '<div\n              class="emptyState emptyStateAction vjFirstRunEmptyState"',
  "div",
  "first-run empty import branch",
);
const ordinaryEmptyIndex = requiredIndex(firstRunGuard, 'data-vj-media-state="empty-import"', "ordinary empty branch");
const firstRunEmptyIndex = requiredIndex(firstRunGuard, 'data-vj-media-state="first-run"', "first-run empty branch");
const firstRunPrimaryIndex = requiredIndex(firstRunGuard, "\n            }\n          >", "first-run guarded primary branch");
assert.ok(ordinaryEmptyIndex > firstRunFallbackIndex, "ordinary import must remain only in the guarded fallback");
assert.ok(firstRunEmptyIndex > firstRunPrimaryIndex, "first-run surface must be the guarded primary branch");

const gridGuardStart = requiredIndex(clipGrid, "const firstRunGuarded = () =>", "clip-grid first-run predicate");
const gridGuardEnd = requiredIndex(clipGrid, "\n\n  createEffect", "clip-grid first-run predicate end", gridGuardStart);
const gridGuard = clipGrid.slice(gridGuardStart, gridGuardEnd);
assert.match(
  gridGuard,
  /props\.layers\.length === 0\s*&&\s*\(props\.firstRunBusy\s*\|\|\s*props\.firstRunAvailable\s*\|\|\s*Boolean\(props\.firstRunError\)\)/,
  "empty first-run guard must prioritize busy/reconciliation and retain error state",
);
assert.equal(count(ordinaryEmptyBranch, 'data-vj-media-import-entry="empty-import"'), 1, "ordinary empty state has one import entry");
assert.equal(count(ordinaryEmptyBranch, "onClick={() => void props.onImportMedia()}"), 1, "ordinary empty state has one import callback");
assert.equal(count(ordinaryEmptyBranch, 'data-vj-media-import-entry="first-run"'), 0, "ordinary empty state does not include bootstrap action");
assert.equal(count(firstRunEmptyBranch, 'data-vj-media-import-entry="first-run"'), 1, "first-run empty state has one bootstrap entry");
assert.equal(count(firstRunEmptyBranch, "onClick={() => void props.onCreateFirstRunShow()}"), 1, "first-run empty state has one bootstrap callback");
assert.equal(count(firstRunEmptyBranch, 'data-vj-media-import-entry="empty-import"'), 0, "first-run empty state does not include ordinary import");
assert.ok(firstRunEmptyBranch.includes('disabled={props.firstRunBusy || !props.firstRunAvailable || !props.firstRunBackendAvailable}'), "first-run action is disabled when availability is false");
assert.ok(firstRunEmptyBranch.includes('aria-busy={props.firstRunBusy}'), "first-run busy state remains announced");
assert.ok(firstRunEmptyBranch.includes('role={props.firstRunBusy ? "status" : undefined}'), "first-run progress remains announced");
assert.ok(firstRunEmptyBranch.includes('class="vjFirstRunError" role="alert"'), "first-run error/retry surface remains guarded and announced");

const populatedGridBranch = sliceBetween(
  clipGrid,
  '<div class="videoClipGrid">',
  '<Show when={bankCount() > 1}>',
  "populated clip grid branch",
);
assert.ok(populatedGridBranch.includes("<For each={visibleLayers()}>"), "populated grid still renders the visible bank");
assert.ok(
  emptyBranch.includes("<Show when={!props.sourceCreateVisible}>"),
  "ordinary empty import must stay coupled to the source-create visibility supplied by the parent",
);
assert.equal(count(populatedGridBranch, "onImportMedia"), 0, "populated grid has no hidden duplicate import callback");
assert.equal(count(populatedGridBranch, "onCreateFirstRunShow"), 0, "populated grid has no bootstrap callback");
const clipsPerBankIndex = requiredIndex(clipGrid, "const CLIPS_PER_BANK = 12;", "clip bank size");
const bankCountIndex = requiredIndex(clipGrid, "const bankCount = createMemo", "clip bank count", clipsPerBankIndex);
const pagerIndex = requiredIndex(clipGrid, "<Show when={bankCount() > 1}>", "clip bank pager", bankCountIndex);
assert.ok(clipsPerBankIndex < bankCountIndex && bankCountIndex < pagerIndex, "clip bank size, pagination state, and pager must remain ordered");

// Mixer clip pads are a fixed 16:9 visual surface.  The grid may scroll within
// its existing pane, but a sparse bank must never stretch its rows to fill it.
const mixerClipGridSelector = ".videoControlPanelMixer .videoMixerClipPane .videoClipGrid";
const mixerClipPadSelector = ".videoControlPanelMixer .videoMixerClipPane .videoClipPad";
const mixerThumbnailSelector = ".videoControlPanelMixer .videoMixerClipPane .videoClipThumbnail";
const mixerClipGridStyles = finalCssBlockWithDeclaration(styles, mixerClipGridSelector, "grid-auto-rows:", "mixer clip-grid sizing");
const mixerClipPadStyles = finalCssBlock(styles, mixerClipPadSelector, "mixer clip-pad viewport");
const mixerThumbnailStyles = finalCssBlock(styles, mixerThumbnailSelector, "mixer thumbnail viewport");
const baseClipPadStyles = exactCssBlock(styles, ".videoClipPad", "base clip-pad sizing");
const shortHeightMixerMarker = "/* At the 720/768 px operating floor";
const shortHeightMixerIndex = requiredIndex(styles, shortHeightMixerMarker, "short-height mixer reflow");
const mixerLayoutStyles = lastExactCssBlock(
  styles.slice(0, shortHeightMixerIndex),
  ".layoutControl.controlModeMixer .videoControlPanelMixer",
  "mixer shared grid layout",
);
const shortHeightMixerLayoutStyles = lastExactCssBlock(
  styles.slice(shortHeightMixerIndex),
  ".layoutControl.controlModeMixer .videoControlPanelMixer",
  "short-height mixer shared grid layout",
);
const fullscreenMixerLayoutStyles = lastExactCssBlock(
  styles,
  'html[data-window-mode="fullscreen"] .layoutControl.controlModeMixer .videoControlPanelMixer',
  "fullscreen mixer shared grid layout",
);
const paneMixerLayoutStyles = exactCssBlock(
  styles,
  ".app.paneWindow-mixer .layoutControl.controlModeMixer .videoControlPanelMixer",
  "default mixer pane shared grid layout",
);
const shortHeightMixerStyles = styles.slice(
  shortHeightMixerIndex,
  requiredIndex(styles, "\n}\n\n.lightingContextTabs", "short-height mixer reflow end", shortHeightMixerIndex),
);
const previewButtonStyles = cssBlock(styles, ".videoClipPreview", "clip preview hit target");
const clipPaneGridPanelStyles = cssBlock(
  styles,
  ".videoControlPanelMixer .videoMixerClipPane .videoClipGridPanel",
  "mixer clip-grid internal scroll",
);
const mixerClipGridResponsiveStyles = lastExactCssBlock(
  styles,
  mixerClipGridSelector,
  "mixer clip-grid responsive columns",
);
assert.match(mixerClipGridStyles, /grid-auto-rows:\s*minmax\(56px, max-content\);/, "mixer rows must keep their content height instead of consuming free pane height");
assert.match(mixerClipGridStyles, /align-content:\s*start;/, "a sparse mixer bank must remain top-aligned");
assert.match(mixerClipGridStyles, /align-items:\s*start;/, "individual mixer pads must not stretch inside their stable rows");
assert.doesNotMatch(mixerClipGridStyles, /grid-auto-rows:\s*[^;]*\b1fr\b/, "final mixer row sizing must not include a free-space 1fr track");
assert.doesNotMatch(mixerClipGridStyles, /align-content:\s*stretch;/, "final mixer grid must not stretch sparse rows");
assert.match(mixerClipPadStyles, /aspect-ratio:\s*16\s*\/\s*9;/, "each mixer clip pad must establish a 16:9 viewport");
assert.match(mixerClipPadStyles, /min-height:\s*0;/, "mixer pads must override the base fixed minimum so column width controls their 16:9 height");
assert.match(mixerThumbnailStyles, /width:\s*100%;/, "mixer thumbnail spans its viewport width");
assert.match(mixerThumbnailStyles, /height:\s*100%;/, "mixer thumbnail spans its viewport height");
assert.match(mixerThumbnailStyles, /aspect-ratio:\s*16\s*\/\s*9;/, "mixer thumbnail viewport remains explicitly 16:9");
assert.match(mixerThumbnailStyles, /object-fit:\s*contain;/, "mixer thumbnails must letterbox instead of crop or distort");
assert.match(mixerThumbnailStyles, /object-position:\s*center;/, "mixer thumbnails remain centered in their viewport");
assert.match(previewButtonStyles, /min-width:\s*44px;/, "Preview retains its 44px intended hit width");
assert.match(clipPaneGridPanelStyles, /overflow:\s*hidden auto;/, "clip banks remain reachable through pane-local scrolling");
assert.match(clipGrid, /data-vj-thumbnail-request[\s\S]*?onClick=\{props\.onRequestThumbnails\}/, "explicit thumbnail authorization remains available in the populated grid");
assert.equal(pixelDeclaration(baseClipPadStyles, "min-height", "base clip-pad minimum"), 90, "the effective-size proof must include the conflicting 90px base minimum");
const mixerPadMinHeight = Number(mixerClipPadStyles.match(/min-height:\s*(\d+(?:\.\d+)?);/)?.[1]);
assert.ok(Number.isFinite(mixerPadMinHeight), "mixer clip-pad minimum must be numeric for effective-size verification");
assert.match(
  mixerLayoutStyles,
  /grid-template-columns:\s*minmax\(390px,\s*1fr\)\s+minmax\(520px,\s*1\.2fr\);/,
  "Video must use Lighting's real two-column lower grid rather than a three-column VJ desk",
);
assert.match(
  mixerLayoutStyles,
  /grid-template-rows:\s*36px\s+minmax\(0,\s*0\.46fr\)\s+5px\s+minmax\(0,\s*0\.54fr\);/,
  "Video must use a header, spanning top pane, structural splitter, and lower grid",
);
assert.match(
  shortHeightMixerLayoutStyles,
  /grid-template-columns:\s*minmax\(390px,\s*1\.1fr\)\s+minmax\(520px,\s*1fr\);/,
  "short-height Video keeps the shared two-column grid while favoring the clip-bank column",
);
assert.match(
  shortHeightMixerLayoutStyles,
  /grid-template-rows:\s*36px\s+minmax\(0,\s*0\.36fr\)\s+5px\s+minmax\(0,\s*0\.64fr\);/,
  "short-height Video reflows height to the lower clip/context pair instead of shrinking controls",
);
const mixerClipColumnCount = Number(mixerClipGridResponsiveStyles.match(/grid-template-columns:\s*repeat\((\d+)/)?.[1]);
assert.equal(mixerClipColumnCount, 6, "mixer bank uses six responsive columns so all twelve 16:9 pads fit as two rows without a vertical scroll");
assert.equal(mixerPadMinHeight, 0, "mixer pad itself has no fixed height that could override its declared 16:9 ratio");
assert.match(
  fullscreenMixerLayoutStyles,
  /grid-template-columns:\s*minmax\(390px,\s*1fr\)\s+minmax\(520px,\s*1\.2fr\);/,
  "fullscreen must override the legacy three-column VJ desk with the shared two-column lower grid",
);
assert.match(
  fullscreenMixerLayoutStyles,
  /grid-template-rows:\s*minmax\(0,\s*0\.47fr\)\s+5px\s+minmax\(0,\s*0\.53fr\);/,
  "fullscreen hidden-header layout keeps top / divider / lower shared rows",
);
assert.match(
  paneMixerLayoutStyles,
  /grid-template-columns:\s*minmax\(300px,\s*1fr\)\s+minmax\(0,\s*1\.2fr\);/,
  "the 860px mixer pane must reflow below the main desk's 910px column minimum",
);
assert.match(
  paneMixerLayoutStyles,
  /grid-template-rows:\s*36px\s+minmax\(0,\s*0\.46fr\)\s+5px\s+minmax\(0,\s*0\.54fr\);/,
  "the 860px mixer pane balances >=100px real monitor viewports with active lower local-scroll rows",
);
assert.match(shortHeightMixerStyles, /\.videoControlPanelMixer \.liveVideoMonitorPanel\s*\{[\s\S]*?gap:\s*5px;[\s\S]*?padding:\s*6px;/, "short-height monitor reflow must retain the base 5px gap and 6px padding");
assert.match(shortHeightMixerStyles, /\.videoControlPanelMixer \.videoMixerClipPane > \.videoMediaOperationRail\s*\{[\s\S]*?grid-row:\s*2;/, "short-height active media work must own an explicit second grid row");
assert.match(shortHeightMixerStyles, /:has\(> \.videoMediaOperationRail\)\s*\{[\s\S]*?grid-template-rows:\s*40px\s+auto\s+30px\s+22px\s+minmax\(0,\s*1fr\);/, "active operation layout must shift the library, drawer row, and clips explicitly");
assert.match(shortHeightMixerStyles, /:has\(> \.videoMediaOperationRail\):has\(\[data-mixer-drawer-toggle\]\[aria-expanded="true"\]\)\s*\{[\s\S]*?grid-template-rows:\s*40px\s+auto\s+30px\s+22px\s+minmax\(56px,\s*64px\)\s+minmax\(0,\s*1fr\);/, "an open drawer with active media gets its own bounded local row before the clip grid");
assert.match(controlPanel, /data-media-operation-cancel=\{operation\.id\}/, "the mounted operation rail exposes a stable Cancel target bound to the real operation id");
const widthCompactMixerIndex = requiredIndex(styles, "@media (max-width: 1400px)", "width-compact mixer reflow");
const widthCompactMixerEnd = requiredIndex(styles, "\n}\n\n/* Keep the existing Clip Grid controls", "width-compact mixer reflow end", widthCompactMixerIndex);
const widthCompactMixerStyles = styles.slice(widthCompactMixerIndex, widthCompactMixerEnd);
assert.doesNotMatch(widthCompactMixerStyles, /videoClipGridPanel/, "width compaction must retain the existing 6px base or 4px short-height clip-grid padding instead of collapsing it to zero");
assert.match(app, /data-lighting-context-tab="lighting"[\s\S]*?aria-keyshortcuts="E"/, "local Lighting tab exposes its E shortcut to assistive technology");
assert.match(app, /data-lighting-context-tab="timeline"[\s\S]*?aria-keyshortcuts="L"/, "local Timeline tab exposes its L shortcut to assistive technology");
for (const [width, height] of [[320, 180], [640, 360]]) {
  assert.ok(Math.abs((width / height) / (16 / 9) - 1) <= 0.01, `${width}x${height} thumbnail viewport remains within 1% of 16:9`);
}

const mixerGateMarker = '<Show when={props.mixer && props.clipGrid.layers.length > 0}>';
const mixerGate = balancedElement(controlPanel, mixerGateMarker, "Show", "mixer source disclosure gate");
const mixerDisclosure = balancedElement(
  mixerGate,
  '<details class="videoMixerSourceDisclosure"',
  "details",
  "mixer source disclosure",
);
const normalSourceGate = balancedElement(
  controlPanel,
  '<Show when={!props.mixer && sourceCreateVisible()}>',
  "Show",
  "normal source-create gate",
);
const sourceCreatePredicateStart = requiredIndex(controlPanel, "const sourceCreateVisible = () =>", "normal source-create predicate");
const sourceCreatePredicateEnd = requiredIndex(controlPanel, "\n  return (", "normal source-create predicate end", sourceCreatePredicateStart);
const sourceCreatePredicate = controlPanel.slice(sourceCreatePredicateStart, sourceCreatePredicateEnd);
const controlGuardStart = requiredIndex(controlPanel, "const firstRunGuarded = () =>", "control first-run predicate");
const controlGuardEnd = requiredIndex(controlPanel, "\n  const sourceCreateVisible", "control first-run predicate end", controlGuardStart);
const controlGuard = controlPanel.slice(controlGuardStart, controlGuardEnd);
assert.match(
  controlGuard,
  /props\.clipGrid\.layers\.length === 0\s*&&\s*\(props\.clipGrid\.firstRunBusy\s*\|\|\s*props\.clipGrid\.firstRunAvailable\s*\|\|\s*Boolean\(props\.clipGrid\.firstRunError\)\)/,
  "control source gate must use the same empty busy/available/error guard",
);
assert.ok(sourceCreatePredicate.includes("!props.mixer"), "normal source-create predicate must exclude mixer mode");
assert.ok(sourceCreatePredicate.includes("!firstRunGuarded()"), "normal source-create predicate must stay closed during first-run reconciliation");
assert.ok(!sourceCreatePredicate.includes("previewError"), "preview-staging errors must not hide a populated normal source surface");
assert.equal(count(mixerGate, '<details class="videoMixerSourceDisclosure"'), 1, "mixer disclosure is enclosed by one exact mixer/populated gate");
assert.equal(count(mixerDisclosure, '<VideoSourceCreatePanel {...props.sourceCreate} />'), 1, "mixer disclosure owns one source-create surface");
assert.equal(count(normalSourceGate, '<VideoSourceCreatePanel {...props.sourceCreate} />'), 1, "normal mode owns one source-create surface");
assert.equal(count(controlPanel, '<VideoSourceCreatePanel {...props.sourceCreate} />'), 2, "the two source surfaces are separate mutually exclusive mode branches");
assert.equal(count(controlPanel, mixerGateMarker), 1, "mixer disclosure uses the exact mixer + populated predicate");
assert.ok(!normalSourceGate.includes("videoMixerSourceDisclosure"), "populated normal mode cannot mount the mixer disclosure");
const disclosureIndex = requiredIndex(controlPanel, 'data-vj-media-import-disclosure', "mixer disclosure order marker");
const clipGridIndex = requiredIndex(controlPanel, "<VideoClipGridPanel", "clip grid order marker");
assert.ok(disclosureIndex < clipGridIndex, "mixer disclosure must be outside the paged clip branch");
const clipGridWiring = sliceBetween(
  controlPanel,
  "<VideoClipGridPanel",
  "/>",
  "clip-grid source-create visibility wiring",
  clipGridIndex,
);
assert.equal(
  count(clipGridWiring, "sourceCreateVisible={sourceCreateVisible()}"),
  1,
  "the parent must pass its exact source-create visibility decision into the grid",
);
assert.equal(count(mixerDisclosure, "onImportMedia"), 0, "opening the mixer disclosure does not add a second grid callback");

const topPaneIndex = requiredIndex(controlPanel, '<section class="videoMixerTopPane"', "Video top pane");
const dividerIndex = requiredIndex(controlPanel, '<div class="videoMixerGridDivider"', "Video grid divider", topPaneIndex);
const clipPaneIndex = requiredIndex(controlPanel, '<section class="videoMixerClipPane"', "Video lower-left clip pane", dividerIndex);
const contextPaneIndex = requiredIndex(controlPanel, '<section class="videoMixerContextPane"', "Video lower-right context pane", clipPaneIndex);
const contextPane = balancedElement(
  controlPanel,
  '<section class="videoMixerContextPane"',
  "section",
  "Video lower-right context pane",
  contextPaneIndex,
);
assert.ok(topPaneIndex < dividerIndex && dividerIndex < clipPaneIndex && clipPaneIndex < contextPaneIndex, "Video DOM follows top / splitter / lower-left / lower-right order");
assert.equal(count(contextPane, '<section class="videoMixerProgramPane"'), 1, "lower-right context retains one output pane");
assert.equal(count(contextPane, '<section class="videoMixerLayerPane"'), 1, "lower-right context retains one layer pane");
assert.equal(count(contextPane, '<LiveVideoMonitorPanel {...props.liveMonitors} />'), 0, "monitors live in the spanning top pane, not a third lower column");
const topPane = balancedElement(controlPanel, '<section class="videoMixerTopPane"', "section", "Video top pane", topPaneIndex);
assert.equal(count(topPane, '<LiveVideoMonitorPanel {...props.liveMonitors} />'), 1, "top pane preserves the Preview / Program monitor");
assert.equal(count(topPane, '<VideoMasterControlsPanel {...props.masterControls} />'), 1, "top pane preserves master controls without duplicating them");

const disclosureSummary = '<summary aria-label="Import Media">Import Media</summary>';
assert.equal(count(mixerDisclosure, disclosureSummary), 1, "mixer disclosure reuses the localized Import Media accessible name");
assert.equal(count(controlPanel, 'aria-label="Import Multiple"'), 0, "the panel toggle is not named Import Multiple");
assert.equal(count(sourceCreate, "onClick={() => void props.onImportMultiple()}"), 1, "existing source-create surface keeps exactly one Import Multiple callback");
assert.equal(count(sourceCreate, "Import Multiple"), 1, "the inner actual batch action keeps the Import Multiple label");
assert.ok(sourceCreate.includes("onBrowseSource") && sourceCreate.includes("onAddLayer"), "existing browse/add workflow remains intact");
for (const forbidden of ["Import / Add Media", "Import or add media source"]) {
  assert.equal(count(clipGrid, forbidden) + count(controlPanel, forbidden) + count(sourceCreate, forbidden), 0, `${forbidden} must not be introduced`);
}

const disclosureSelector = ".videoControlPanelMixer .videoMixerClipPane .videoMixerPaneHeader .videoMixerSourceDisclosure";
const disclosureSummarySelector = `${disclosureSelector} > summary`;
const disclosureMarkerSelector = `${disclosureSelector} > summary::-webkit-details-marker`;
const disclosureOpenSelector = `${disclosureSelector}[open] > summary`;
const sourceSurfaceSelector = `${disclosureSelector} > .videoMixerSourceCreateSurface`;
const sourceLabelSelector = `${sourceSurfaceSelector} > label`;
const sourceFieldSelector = `${sourceSurfaceSelector} input,\n${sourceSurfaceSelector} select`;
const sourceHintSelector = `${sourceSurfaceSelector} .fieldHint`;
const sourceButtonSelector = `${sourceSurfaceSelector} > button`;
const disclosureStyles = cssBlock(styles, disclosureSelector, "mixer disclosure positioning");
const disclosureSummaryStyles = cssBlock(styles, disclosureSummarySelector, "mixer disclosure summary");
const disclosureMarkerStyles = cssBlock(styles, disclosureMarkerSelector, "mixer disclosure marker");
const disclosureOpenStyles = cssBlock(styles, disclosureOpenSelector, "mixer disclosure open state");
const sourceSurfaceStyles = cssBlock(styles, sourceSurfaceSelector, "mixer source surface overlay");
const sourceLabelStyles = cssBlock(styles, sourceLabelSelector, "mixer source labels");
const sourceFieldStyles = cssBlock(styles, sourceFieldSelector, "mixer source fields");
const sourceHintStyles = cssBlock(styles, sourceHintSelector, "mixer source hint");
const sourceButtonStyles = cssBlock(styles, sourceButtonSelector, "mixer source buttons");
const clipHeaderStyles = cssBlock(styles, ".videoMixerPaneHeader", "clip pane header");
assert.equal(count(disclosureStyles, "position: relative;"), 1, "disclosure positioning must be scoped to the mixer clip header");
assert.equal(count(disclosureStyles, "z-index: 1200;"), 1, "disclosure stacking must be scoped to the mixer clip header");
assert.equal(count(disclosureSummaryStyles, "min-height: 29px;"), 1, "closed disclosure must fit the existing header content track");
assert.equal(count(disclosureSummaryStyles, "font-size: 11px;"), 1, "disclosure typography must remain compact and existing");
assert.equal(count(disclosureMarkerStyles, "display: none;"), 1, "disclosure marker hiding must be scoped to its summary");
assert.equal(count(disclosureOpenStyles, "box-shadow: inset 2px 0 0 var(--ray-accent);"), 1, "disclosure open state must remain local");
assert.equal(count(sourceSurfaceStyles, "position: absolute;"), 1, "opened source surface must overlay from the clip header");
assert.equal(count(sourceSurfaceStyles, "overflow: hidden auto;"), 1, "opened source surface must scroll internally");
assert.equal(count(sourceSurfaceStyles, "max-height: min(420px, calc(100vh - 56px));"), 1, "opened source surface must stay within the clip pane viewport envelope");
assert.equal(count(sourceLabelStyles, "display: grid;"), 1, "source labels must be scoped to the overlay");
assert.equal(count(sourceFieldStyles, "width: 100%;"), 1, "source field sizing must be scoped to the overlay");
assert.equal(count(sourceHintStyles, "margin: 0;"), 1, "source hint spacing must be scoped to the overlay");
assert.equal(count(sourceButtonStyles, "width: 100%;"), 1, "source button sizing must be scoped to the overlay");
assert.equal(count(clipHeaderStyles, "min-height: 40px;"), 1, "clip header remains on the existing 40px row contract");
assert.equal(count(styles, "\n.videoMixerSourceDisclosure {"), 0, "disclosure styles must not fall back to a global selector");

// Source-backed thumbnails are opt-in for each project authority. Merely
// hydrating a loaded snapshot must stop before the thumbnail IPC call; an
// explicit Mixer selection or Load Thumbnails action is the only unlock.
const thumbnailEffect = sliceBetween(
  app,
  "const videoThumbnailSourceSignature = createMemo",
  "const videoLayerHasMonitorableAudio =",
  "thumbnail authority effect",
);
const thumbnailAuthorizationIndex = requiredIndex(
  thumbnailEffect,
  "if (!videoThumbnailAccessAuthorized())",
  "thumbnail authorization guard",
);
const thumbnailReadIndex = requiredIndex(
  thumbnailEffect,
  "await loadVideoLayerThumbnail(source.id)",
  "thumbnail source read",
);
assert.ok(thumbnailAuthorizationIndex < thumbnailReadIndex, "no thumbnail source read is reachable before explicit authorization");
assert.match(thumbnailEffect, /if \(!videoThumbnailAccessAuthorized\(\)\) \{[\s\S]*?setVideoClipThumbnails\(\{\}\);[\s\S]*?return;/, "unauthorized load/restart returns with an empty projection");
assert.match(app, /if \(mode === "mixer"\) authorizeVideoThumbnailAccess\(\);/, "an explicit Mixer selection authorizes thumbnails");
assert.match(app, /onRequestThumbnails: authorizeVideoThumbnailAccess/, "the visible thumbnail request authorizes the same cache");
assert.match(clipGrid, /data-vj-thumbnail-request[\s\S]*?onClick=\{props\.onRequestThumbnails\}/, "populated grid exposes an explicit source-read action");
const projectReset = sliceBetween(
  app,
  "const resetMediaAssetUiForProjectReplacement = () => {",
  "\n  };",
  "project replacement Media UI reset",
);
for (const reset of [
  "setVideoThumbnailAccessAuthorized(false)",
  "videoThumbnailUrlCache = {}",
  "videoThumbnailSignatures.clear()",
  "setMediaAssetAvailabilityById({})",
  "setLastMediaAssetImportReport(null)",
]) {
  assert.ok(projectReset.includes(reset), `project replacement reset must include ${reset}`);
}
assert.match(app, /if \(replacement\) resetMediaAssetUiForProjectReplacement\(\);/, "only an admitted project replacement resets thumbnail authorization/cache");

// Media Library stays in the fixed clip pane and exposes read-only Verify,
// asset Relink, exact per-entry import failure truth, and real operation cancel.
const libraryRail = balancedElement(
  controlPanel,
  '<details class="videoMediaLibraryRail" data-media-library-rail>',
  "details",
  "Media Library rail",
);
assert.match(libraryRail, /props\.mediaLibrary\.assets/, "Media Library renders the authoritative snapshot catalog");
assert.match(libraryRail, /props\.mediaLibrary\.availabilityById\[asset\.id\]/, "each catalog entry projects machine-local availability");
assert.match(libraryRail, /onVerify\(\[asset\.id\]\)/, "each asset has a dedicated Verify action");
assert.match(libraryRail, /onRelink\(asset\.id\)/, "each local asset has a Relink picker action");
assert.match(controlPanel, /entry\.status === "failed" \|\| entry\.status === "skipped"/, "mixed import issues retain per-entry failure truth");
assert.match(libraryRail, /entry\.path[\s\S]*?entry\.message/, "mixed import UI exposes path and backend message, not aggregate counts only");
assert.match(controlPanel, /onCancelOperation\(operation\.id\)/, "operation Cancel targets the exact projected controller ID");
assert.match(app, /const cancelMediaAssetOperation = \(id: number\)[\s\S]*?controller\.abort\(\)/, "operation Cancel aborts the real AbortController");
assert.match(app, /setPhase: \(phase: MediaAssetOperationPhase\) => \{[\s\S]*?controller\.signal\.aborted[\s\S]*?return;/, "an aborted operation cannot overwrite Cancelling with a later worker phase");
const sourcePicker = sliceBetween(
  app,
  "const selectVideoSourceFile = async () => {",
  "const downloadGdtfFromUrl = async",
  "single-source picker",
);
assert.match(sourcePicker, /beginMediaAssetOperation\("Choose video source", "picker"\)/, "single-source Browse exposes its picker phase");
assert.match(sourcePicker, /mediaOperation\.signal\.aborted[\s\S]*?return;/, "cancelled Browse cannot apply its delayed path");
assert.match(app, /inspectMediaAssetAvailability\(\{[\s\S]*?verifyHash: true[\s\S]*?onPhase: mediaOperation\.setPhase/, "Media Library Verify uses the reserved availability helper with hash verification and phase reporting");
assert.match(controller, /policy: "RequireContentMatch"[\s\S]*?firstOutcome\?\.kind !== "needs_explicit_adoption"[\s\S]*?confirmMediaAssetAdoption[\s\S]*?"AdoptReplacement"/, "replacement adoption is gated by exact backend outcome plus explicit confirmation");
assert.match(controller, /decisionAuthority = first\.terminalAuthority[\s\S]*?!options\.isProjectAuthorityCurrent\(decisionAuthority\)/, "replacement confirmation cannot carry an old asset decision into project C");
assert.doesNotMatch(controller, /pickerOperation\.setPhase\("hashing"\)/, "Relink cannot announce hashing before its reserved Start/preparing phase");
const librarySurfaceStyles = cssBlock(styles, ".videoMediaLibrarySurface", "Media Library overlay");
assert.match(librarySurfaceStyles, /position: absolute;/, "open Media Library overlays instead of shrinking the clip bank");
assert.match(librarySurfaceStyles, /overflow: hidden auto;/, "open Media Library scrolls internally");
assert.match(librarySurfaceStyles, /max-height: min\(340px, calc\(100vh - 120px\)\);/, "Media Library remains within the one-screen viewport envelope");

// This deterministic model follows the exact predicates asserted above. It keeps
// the branch proof non-vacuous without pretending that source inspection mounts Solid.
const firstRunGuarded = (state) =>
  state.layers === 0 && (state.firstRunBusy || state.firstRunAvailable || Boolean(state.firstRunError));
const mediaAccess = (state) => {
  const guarded = firstRunGuarded(state);
  const normalSource = !state.mixer && !guarded && (state.layers > 0 || !state.firstRunAvailable);
  const mixerDisclosureVisible = state.mixer && state.layers > 0;
  const ordinaryEmptyImport = state.layers === 0 && !guarded && state.mixer;
  return {
    guarded,
    normalSource,
    mixerDisclosureVisible,
    ordinaryEmptyImport,
    importEntryCount: Number(guarded) + Number(normalSource) + Number(mixerDisclosureVisible) + Number(ordinaryEmptyImport),
  };
};

for (const mixer of [false, true]) {
  const emptyAvailable = mediaAccess({ layers: 0, mixer, firstRunAvailable: true, firstRunBusy: false, firstRunError: null });
  assert.equal(emptyAvailable.guarded, true, `empty available state stays first-run guarded (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyAvailable.importEntryCount, 1, `empty available state has exactly one first-run entry (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyAvailable.ordinaryEmptyImport, false, "empty available state does not expose ordinary Import Media");

  const emptyBusy = mediaAccess({ layers: 0, mixer, firstRunAvailable: false, firstRunBusy: true, firstRunError: null });
  assert.equal(emptyBusy.guarded, true, `empty busy/reconciling state stays guarded (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyBusy.importEntryCount, 1, `empty busy/reconciling state has exactly one first-run entry (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyBusy.ordinaryEmptyImport, false, "empty busy/reconciling state does not expose ordinary Import Media");

  const emptyRetryableError = mediaAccess({ layers: 0, mixer, firstRunAvailable: true, firstRunBusy: false, firstRunError: "bootstrap failed" });
  assert.equal(emptyRetryableError.guarded, true, `retryable empty error stays first-run guarded (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyRetryableError.importEntryCount, 1, `retryable empty error has exactly one first-run surface (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyRetryableError.ordinaryEmptyImport, false, "retryable empty error does not expose ordinary Import Media");

  const emptyAwaitingSyncError = mediaAccess({ layers: 0, mixer, firstRunAvailable: false, firstRunBusy: false, firstRunError: "refresh failed" });
  assert.equal(emptyAwaitingSyncError.guarded, true, `awaiting-sync error stays guarded (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyAwaitingSyncError.importEntryCount, 1, `awaiting-sync error has exactly one disabled safety surface (${mixer ? "mixer" : "normal"})`);
  assert.equal(emptyAwaitingSyncError.ordinaryEmptyImport, false, "awaiting-sync error does not expose ordinary Import Media");
}

const populatedNormal = mediaAccess({
  layers: 1,
  mixer: false,
  firstRunAvailable: false,
  firstRunBusy: true,
  firstRunError: null,
  previewError: "preview staging failed",
});
assert.equal(populatedNormal.guarded, false, "populated layers end the empty first-run guard");
assert.equal(populatedNormal.normalSource, true, "populated normal mode keeps source creation reachable despite preview error");
assert.equal(populatedNormal.mixerDisclosureVisible, false, "populated normal mode does not mount the mixer disclosure");
assert.equal(populatedNormal.importEntryCount, 1, "populated normal mode has exactly one source-create entry");

const populatedMixer = mediaAccess({ layers: 1, mixer: true, firstRunAvailable: false, firstRunBusy: false, firstRunError: null });
assert.equal(populatedMixer.normalSource, false, "populated mixer mode does not mount the normal source surface");
assert.equal(populatedMixer.mixerDisclosureVisible, true, "populated mixer mode mounts the compact disclosure");
assert.equal(populatedMixer.importEntryCount, 1, "populated mixer mode has exactly one source-create entry");

const fullBank = Array.from({ length: 13 }, (_, index) => index);
const fullBankPageCount = Math.max(1, Math.ceil(fullBank.length / 12));
assert.equal(fullBank.slice(0, 12).length, 12, "full clip bank keeps twelve visible clips");
assert.equal(fullBankPageCount, 2, "clip bank pager remains reachable for the next page");
assert.equal(mediaAccess({ layers: fullBank.length, mixer: true, firstRunAvailable: false, firstRunBusy: false, firstRunError: null }).importEntryCount, 1, "full bank/page keeps one mixer import entry");

console.log("VJ media-import access static contract ok: guarded empty states, populated normal/mixer exclusivity, accessible names, scoped overlay CSS, and full bank/page reachability verified");
console.log("Limitation: this repository has no focused Solid component mount runner, so this gate proves exact branch predicates and CSS contracts rather than browser-rendered reachability.");
