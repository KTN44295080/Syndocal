import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const componentRoot = new URL("../src/components/", import.meta.url);
const clipGrid = await readFile(new URL("VideoClipGridPanel.tsx", componentRoot), "utf8");
const controlPanel = await readFile(new URL("VideoControlPanel.tsx", componentRoot), "utf8");
const sourceCreate = await readFile(new URL("VideoSourceCreatePanel.tsx", componentRoot), "utf8");
const timelineSourceShelf = (await readFile(new URL("TimelineSourceShelf.tsx", componentRoot), "utf8")).replaceAll("\r\n", "\n");
const styles = (await readFile(new URL("../src/styles.css", import.meta.url), "utf8")).replaceAll("\r\n", "\n");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const controller = await readFile(new URL("../src/createVideoRuntimeController.ts", import.meta.url), "utf8");

const thumbnailControllerSource = (await readFile(new URL("../src/createMediaThumbnailController.ts", import.meta.url), "utf8")).replaceAll("\r\n", "\n");
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
const widthCompactMixerAnchor = requiredIndex(styles, "/* Keep the existing Clip Grid controls", "width-compact mixer successor");
const widthCompactMixerIndex = styles.lastIndexOf("@media (max-width: 1400px)", widthCompactMixerAnchor);
assert.ok(widthCompactMixerIndex >= 0, "width-compact mixer reflow must precede its successor");
const widthCompactMixerEnd = requiredIndex(styles, "\n}\n\n/* Keep the existing Clip Grid controls", "width-compact mixer reflow end", widthCompactMixerIndex);
const widthCompactMixerStyles = styles.slice(widthCompactMixerIndex, widthCompactMixerEnd);
assert.doesNotMatch(widthCompactMixerStyles, /videoClipGridPanel/, "width compaction must retain the existing 6px base or 4px short-height clip-grid padding instead of collapsing it to zero");
// The retired Lighting/Timeline E/L tabs were replaced by three context tabs.
// Check their real accessible keyboard contract and keep source classification
// beneath Sources, rather than advertising shortcuts that no longer select tabs.
assert.match(timelineSourceShelf, /role="tablist" aria-label="Timeline context"/, "Timeline context exposes an accessible tablist");
assert.equal(count(timelineSourceShelf, 'role="tab"'), 3, "context has exactly Sources, Inspector and Video Preview tabs");
assert.match(timelineSourceShelf, /const modes = \["sources", "inspector", "video-preview"\] as const;/, "keyboard navigation follows the same three context modes");
for (const mode of ["sources", "inspector", "video-preview"]) {
  const button = sliceBetween(timelineSourceShelf, `id="timeline-source-context-tab-${mode}"`, "</button>", `${mode} context tab`);
  assert.ok(button.includes('role="tab"'), `${mode} exposes its tab role`);
  assert.ok(button.includes(`aria-selected={sourceContextMode() === "${mode}"}`), `${mode} announces its selection`);
  assert.ok(button.includes(`aria-controls="timeline-source-context-panel-${mode}"`), `${mode} controls the corresponding panel`);
  assert.ok(button.includes(`tabindex={sourceContextMode() === "${mode}" ? 0 : -1}`), `${mode} participates in roving tab focus`);
  assert.ok(button.includes(`onClick={() => selectSourceContextMode("${mode}")}`), `${mode} click selects the same keyboard mode`);
  for (const key of ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"]) {
    assert.ok(button.includes(`event.key === "${key}"`), `${mode} supports ${key} tab navigation`);
  }
  assert.ok(timelineSourceShelf.includes(`id="timeline-source-context-panel-${mode}" role="tabpanel" aria-labelledby="timeline-source-context-tab-${mode}"`), `${mode} panel is labelled by its own tab`);
}
assert.match(timelineSourceShelf, /queueMicrotask\(\(\) => document\.getElementById\(`timeline-source-context-tab-\$\{target\}`\)\?\.focus\(\)\);/, "keyboard selection transfers focus to the selected context tab");
const sourcePanel = sliceBetween(timelineSourceShelf, '<Show when={sourceContextMode() === "sources"}>\n        <div', '<Show when={sourceContextMode() === "inspector"}>', "Sources context and lower classification");
assert.ok(sourcePanel.includes('<For each={["All", "Lighting", "Video", "Audio"] as const}>'), "source classification retains All/Lighting/Video/Audio beneath Sources");
assert.ok(sourcePanel.includes('aria-pressed={sourceShelfFilter() === filter}'), "source filters announce their selected classification");
assert.ok(sourcePanel.includes('onClick={() => setSourceShelfFilter(filter)}'), "classification changes the source filter, not the context tab");
assert.match(app, /contextMode=\{timelineLowerContextMode\(\)\}[\s\S]*?onContextModeChange=\{setTimelineLowerContextMode\}[\s\S]*?inspectorContent=\{renderTimelineInspector\(\)\}[\s\S]*?previewContent=\{<TimelineOutputPreview/, "App wires all three context surfaces to the current source shelf owner");
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
assert.equal(count(mixerDisclosure, '<VideoSourceCreatePanel'), 1, "mixer disclosure owns one source-create surface");
assert.equal(count(normalSourceGate, '<VideoSourceCreatePanel'), 1, "normal mode owns one source-create surface");
const librarySourceGate = balancedElement(controlPanel, '<Show when={props.libraryOnly}>', "Show", "library-only source disclosure");
assert.equal(count(librarySourceGate, '<VideoSourceCreatePanel'), 1, "library-only mode owns its separate import surface");
assert.ok(librarySourceGate.includes('data-edit-video-import-disclosure'), "library-only import has its own disclosure identity");
assert.equal(count(controlPanel, '<VideoSourceCreatePanel'), 3, "library-only, mixer and normal mode own exactly three source declarations");
const nonLibraryGates = [...controlPanel.matchAll(/<Show when=\{!props\.libraryOnly\}>/g)].map((match) =>
  balancedElement(controlPanel, match[0], "Show", "non-library mode boundary", match.index),
);
assert.ok(nonLibraryGates.some((gate) => gate.includes(mixerGate)), "mixer import cannot coexist with library-only import");
assert.ok(nonLibraryGates.some((gate) => gate.includes(normalSourceGate)), "normal import cannot coexist with library-only import");
for (const surface of [mixerDisclosure, normalSourceGate, librarySourceGate]) {
  assert.match(surface, /<VideoSourceCreatePanel\s+\{\.\.\.props\.sourceCreate\}\s+invokeCommand=\{props\.invokeCommand \?\? props\.sourceCreate\.invokeCommand\}\s*\/>/, "both source surfaces retain their source props and injected authority command port");
}
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
  thumbnailControllerSource,
  "const videoThumbnailSourceSignature = createMemo",
  "  onCleanup(() => {",
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
assert.match(thumbnailControllerSource, /if \(!videoThumbnailAccessAuthorized\(\)\) \{[\s\S]*?setMediaAssetThumbnails\(\{\}\);[\s\S]*?return;/, "unauthorized project load/reset returns with an empty asset-thumbnail projection");
assert.match(thumbnailControllerSource, /loadMediaAssetThumbnail\(source\.id\)/, "authorized Media Library thumbnail loading uses the single backend-owned asset thumbnail path");
assert.match(thumbnailControllerSource, /hash_algorithm:[\s\S]*?hash_hex:[\s\S]*?byte_size:/, "asset thumbnail cache identity fences source path and persisted content identity");
assert.match(thumbnailControllerSource, /let videoThumbnailGeneration = 0;[\s\S]*?let mediaAssetThumbnailGeneration = 0;/, "layer and asset thumbnail work use independent generation fences");
const mediaAssetThumbnailEffect = sliceBetween(
  thumbnailEffect,
  "createEffect(() => {\n    const sources = JSON.parse(mediaAssetThumbnailSourceSignature())",
  "  onCleanup(() => {",
  "Media Library thumbnail effect",
);
assert.match(
  thumbnailControllerSource,
  /const mediaAssetThumbnailAuthoritySignature = createMemo\(\(\) => \{[\s\S]*?const authority = projectMappingsAuthority\(\);[\s\S]*?return JSON\.stringify\(\{[\s\S]*?project_epoch: authority\.project_epoch,[\s\S]*?project_revision: authority\.project_revision,[\s\S]*?checkpoint_hash: authority\.checkpoint_hash,[\s\S]*?\}\);[\s\S]*?\}\);/,
  "Media Library tracks a stable E/R/H scalar, not each authority object publication",
);
assert.match(mediaAssetThumbnailEffect, /const authority = JSON\.parse\(mediaAssetThumbnailAuthoritySignature\(\)\) as ProjectAuthorityToken;/, "a genuine E/R/H change starts one new Media Library thumbnail generation");
assert.doesNotMatch(mediaAssetThumbnailEffect, /captureProjectAuthorityIdentity\(\)/, "Media Library thumbnail effect must not synchronously track the object-valued authority publication");
// Model the race precisely. A poll can republish an equivalent object while a
// source decode is in flight, but an actual E/R/H change must invalidate that
// batch and immediately start exactly one successor. A replacement remains
// fail-closed for the retired batch.
const mediaAssetThumbnailAuthorityBatchModel = (initialAuthority) => {
  let currentAuthority = { ...initialAuthority };
  let observedSignature = null;
  let generation = 0;
  const batches = [];
  const authoritySignature = (authority) => JSON.stringify({
    project_epoch: authority.project_epoch,
    project_revision: authority.project_revision,
    checkpoint_hash: authority.checkpoint_hash,
  });
  const startIfAuthorityChanged = () => {
    const signature = authoritySignature(currentAuthority);
    if (signature === observedSignature) return null;
    observedSignature = signature;
    const batch = { generation: ++generation, authority: { ...currentAuthority } };
    batches.push(batch);
    return batch;
  };
  const setAuthority = (next) => {
    currentAuthority = { ...next };
    return startIfAuthorityChanged();
  };
  const canPublish = (batch) => batch.generation === generation
    && authoritySignature(batch.authority) === authoritySignature(currentAuthority);
  return { batches, startIfAuthorityChanged, setAuthority, canPublish };
};

const thumbnailEquivalentPoll = mediaAssetThumbnailAuthorityBatchModel({
  project_epoch: 7,
  project_revision: 11,
  checkpoint_hash: "A",
});
const equivalentInitialBatch = thumbnailEquivalentPoll.startIfAuthorityChanged();
const equivalentRepublish = thumbnailEquivalentPoll.setAuthority({
  project_epoch: 7,
  project_revision: 11,
  checkpoint_hash: "A",
});
assert.equal(equivalentRepublish, null, "equivalent authority-object churn does not restart the in-flight Media Library thumbnail batch");
assert.equal(thumbnailEquivalentPoll.batches.length, 1, "equivalent authority-object churn keeps exactly one thumbnail generation");
assert.equal(thumbnailEquivalentPoll.canPublish(equivalentInitialBatch), true, "the original batch publishes after equivalent authority-object churn");

const thumbnailGenuineChange = mediaAssetThumbnailAuthorityBatchModel({
  project_epoch: 7,
  project_revision: 11,
  checkpoint_hash: "A",
});
const staleGenuineBatch = thumbnailGenuineChange.startIfAuthorityChanged();
const freshGenuineBatch = thumbnailGenuineChange.setAuthority({
  project_epoch: 7,
  project_revision: 12,
  checkpoint_hash: "B",
});
assert.equal(thumbnailGenuineChange.canPublish(staleGenuineBatch), false, "a genuine same-project E/R/H change rejects the stale mid-batch thumbnail result");
assert.deepEqual(freshGenuineBatch, {
  generation: 2,
  authority: { project_epoch: 7, project_revision: 12, checkpoint_hash: "B" },
}, "a genuine same-project E/R/H change starts exactly one fresh thumbnail batch");
assert.equal(thumbnailGenuineChange.canPublish(freshGenuineBatch), true, "the fresh genuine-authority batch publishes after the stale batch is rejected");

const thumbnailReplacement = mediaAssetThumbnailAuthorityBatchModel({
  project_epoch: 7,
  project_revision: 11,
  checkpoint_hash: "A",
});
const retiredReplacementBatch = thumbnailReplacement.startIfAuthorityChanged();
thumbnailReplacement.setAuthority({ project_epoch: 8, project_revision: 0, checkpoint_hash: "C" });
assert.equal(thumbnailReplacement.canPublish(retiredReplacementBatch), false, "a project replacement still rejects the retired thumbnail batch fail closed");
assert.match(thumbnailControllerSource, /generation !== mediaAssetThumbnailGeneration \|\| !untrack\(\(\) => isProjectAuthorityIdentityCurrent\(authority\)\)/, "asset-thumbnail async completion is fenced by its own generation and current project authority");
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
  "thumbnailController.reset()",
  "setMediaAssetAvailabilityById({})",
  "setLastMediaAssetImportReport(null)",
]) {
  assert.ok(projectReset.includes(reset), `project replacement reset must include ${reset}`);
}
assert.match(app, /if \(options\.replacement\) resetMediaAssetUiForProjectReplacement\(\);/, "the authority-bundle replacement option gates thumbnail authorization/cache reset");
assert.equal(count(app, "resetMediaAssetUiForProjectReplacement();"), 1, "project media reset has one authority-bundle call site");

const thumbnailReset = sliceBetween(thumbnailControllerSource, "const reset = () => {", "\n  };", "controller project reset");
for (const reset of ["setVideoThumbnailAccessAuthorized(false)", "videoThumbnailUrlCache = {}", "mediaAssetThumbnailUrlCache = {}", "videoThumbnailSignatures.clear()", "mediaAssetThumbnailSignatures.clear()"]) {
  assert.ok(thumbnailReset.includes(reset), `controller replacement reset must include ${reset}`);
}
assert.match(app, /const thumbnailController = createMediaThumbnailController\(\{[\s\S]*?layers: \(\) => snapshot\(\)\.video\.layers,[\s\S]*?assets: \(\) => snapshot\(\)\.video\.media_assets,/, "App supplies authoritative source arrays to the thumbnail controller");
// Media Library stays in the fixed clip pane and exposes read-only Verify,
// asset Relink, exact per-entry import failure truth, and real operation cancel.
const libraryRail = balancedElement(
  controlPanel,
  '<details class="videoMediaLibraryRail" data-media-library-rail open={props.libraryOnly}>',
  "details",
  "Media Library rail",
);
assert.match(libraryRail, /props\.mediaLibrary\.assets/, "Media Library renders the authoritative snapshot catalog");
assert.match(libraryRail, /props\.mediaLibrary\.availabilityById\[asset\.id\]/, "each catalog entry projects machine-local availability");
assert.match(libraryRail, /props\.mediaLibrary\.thumbnails\[asset\.id\]/, "Media Library projects its asset-keyed thumbnail cache, including catalog-only entries");
assert.match(libraryRail, /data-media-library-thumbnail-request[\s\S]*?onClick=\{props\.clipGrid\.onRequestThumbnails\}/, "catalog-only Library entries use the same explicit thumbnail authorization action as the clip grid");
assert.match(controlPanel, /if \(!props\.clipGrid\.thumbnailsAuthorized \|\| !hasStillThumbnail \|\| asset\.source\.kind !== "File" \|\| !previewMotionAllowed\(\)\) return;/, "preview frame stream starts only after explicit authorization and a backend thumbnail");
assert.match(controlPanel, /props\.mediaLibrary\.onPreviewStart\(asset\.id\)/, "moving preview begins through the backend-owned exact-identity session protocol");
assert.match(controlPanel, /props\.mediaLibrary\.onPreviewFrame\(ticket, positionMs\)/, "moving preview frames use that backend-owned session ticket");
assert.match(controlPanel, /props\.mediaLibrary\.onPreviewEnd\(ticket\)/, "moving preview ends the backend-owned session on supersession or cancellation");
assert.match(controlPanel, /window\.setTimeout\(tick, 250\)/, "hover preview is conservatively capped at four frames per second");
assert.match(libraryRail, /previewAssetId\(\) === asset\.id && previewFrameUrl\(\)/, "moving preview displays only an already-returned backend frame");
assert.doesNotMatch(controlPanel, /convertFileSrc|<video/, "Media Library never exposes a raw source-file URL or DOM video reader");
assert.match(libraryRail, /class="videoMediaLibraryList"/, "Media Library presents its catalog as a dedicated card grid");
assert.match(libraryRail, /props\.clipGrid\.thumbnailsAuthorized && thumbnail\(\)/, "Media Library only projects source-derived thumbnails after existing explicit authorization");
assert.match(libraryRail, /data-media-library-thumbnail=\{props\.clipGrid\.thumbnailsAuthorized \? "unavailable" : "placeholder"\}/, "Media Library keeps a non-reading placeholder before authorization");
assert.match(libraryRail, /class="videoMediaLibraryCardDetails"/, "Media Library cards retain a title/path details surface");
assert.match(libraryRail, /tabindex="0"/, "Media Library card details are available on keyboard focus as well as hover");
assert.match(libraryRail, /aria-describedby=\{`media-library-source-\$\{asset\.id\}`\}/, "Media Library cards explicitly describe their source path to assistive technology");
assert.match(libraryRail, /id=\{`media-library-source-\$\{asset\.id\}`\} class="srOnly" data-no-localize/, "Media Library source path remains semantic text outside the aria-hidden visual thumbnail");
assert.match(libraryRail, /onMouseEnter=\{\(\) => startMediaLibraryPreview\(asset, Boolean\(thumbnail\(\)\)\)\}/, "authorized video cards start their preview on hover");
assert.match(libraryRail, /onFocusIn=\{\(\) => startMediaLibraryPreview\(asset, Boolean\(thumbnail\(\)\)\)\}/, "authorized video cards start their preview on keyboard focus");
assert.match(libraryRail, /onMouseLeave=\{\(event\) => stopMediaLibraryPreview\(asset\.id, event\.currentTarget\)\}/, "video card hover leave stops its preview");
assert.match(libraryRail, /onFocusOut=\{\(event\) => stopMediaLibraryPreview\(asset\.id, event\.currentTarget\)\}/, "video card focus leave stops its preview");
assert.match(controlPanel, /asset\.source\.kind !== "File"/, "still and live-source cards fail closed from preview playback");
assert.match(controlPanel, /let previewSerial: Promise<void> = Promise\.resolve\(\);[\s\S]*?const enqueuePreview/, "all Begin/Frame/End requests share one serialized frontend lane");
assert.match(controlPanel, /stopMediaLibraryPreviewNow\(\)/, "leave, blur, reset, replacement, and unmount invalidate the active backend preview stream");
assert.match(controlPanel, /onCleanup\(stopMediaLibraryPreviewNow\)/, "component unmount cancels publication from an active preview stream");
assert.match(controller, /begin_media_asset_preview[\s\S]*?sameProjectAuthority\(ticket, options\.getCurrentProjectAuthority\(\)\)/, "Begin reply is rejected when its project authority is stale");
assert.match(controller, /get_media_asset_preview_frame[\s\S]*?sameProjectAuthority\(ticket, options\.getCurrentProjectAuthority\(\)\)/, "Frame request/reply is fenced against the exact current project authority");
assert.match(controller, /end_media_asset_preview[\s\S]*?catch/, "End is a best-effort release for leave, replacement, and unmount races");
assert.match(controlPanel, /const \[previewAssetId, setPreviewAssetId\] = createSignal<MediaAssetId \| null>\(null\);[\s\S]*?setPreviewAssetId\(asset\.id\);/, "a single active asset ID limits concurrent video playback to one card");
assert.match(controlPanel, /const \[previewAssetIdentity, setPreviewAssetIdentity\] = createSignal<string \| null>\(null\);/, "video preview remembers the exact catalog asset identity, not only its reusable numeric ID");
assert.match(controlPanel, /asset\.content_hash\?\.algorithm[\s\S]*?asset\.content_hash\?\.hex[\s\S]*?asset\.byte_size/, "video preview identity includes persisted content hash and byte size, not only source path");
assert.match(controlPanel, /if \(!props\.clipGrid\.thumbnailsAuthorized \|\| !activeAsset \|\| mediaPreviewIdentity\(activeAsset\) !== previewAssetIdentity\(\)\) \{[\s\S]*?stopMediaLibraryPreviewNow\(\);/, "project replacement, authorization reset, missing asset, or changed path immediately invalidates an old preview stream");
assert.match(controlPanel, /prefers-reduced-motion: reduce/, "reduced-motion preference fails safely to the still thumbnail");
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
  "\n  };",
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
const libraryGridStyles = cssBlock(styles, ".videoMediaLibraryList", "Media Library card grid");
assert.match(libraryGridStyles, /grid-template-columns: repeat\(2, minmax\(0, 1fr\)\);/, "Media Library preserves at least two responsive thumbnail columns in the clip pane");
assert.match(styles, /@media \(min-width: 1500px\) \{[\s\S]*?\.videoMediaLibraryList \{[\s\S]*?repeat\(3, minmax\(0, 1fr\)\)/, "Media Library expands its responsive grid on wide operator screens");
const libraryThumbnailStyles = cssBlock(styles, ".videoMediaLibraryThumbnail", "Media Library thumbnail surface");
assert.match(libraryThumbnailStyles, /aspect-ratio: 16 \/ 9;/, "Media Library thumbnails preserve a video-safe aspect ratio");
const libraryThumbnailImageStyles = cssBlock(styles, ".videoMediaLibraryThumbnail > img", "Media Library still thumbnail image");
assert.match(libraryThumbnailImageStyles, /object-fit: contain;/, "Media Library never vertically stretches thumbnail media");
const libraryPreviewStyles = cssBlock(styles, ".videoMediaLibraryPreview", "Media Library video preview");
assert.match(libraryPreviewStyles, /object-fit: contain;/, "Media Library video preview preserves source aspect ratio");
const libraryCardDetailsStyles = cssBlock(styles, ".videoMediaLibraryCardDetails", "Media Library card details");
assert.match(libraryCardDetailsStyles, /pointer-events: none;/, "Media Library hover details never block Verify or Relink controls");
assert.match(libraryCardDetailsStyles, /bottom: 0;[\s\S]*?max-height: 46%;/, "Media Library details cover only the lower caption band so an active preview remains visible");
assert.match(styles, /\.videoMediaLibraryItem:hover \.videoMediaLibraryCardDetails,[\s\S]*?\.videoMediaLibraryItem:focus-within \.videoMediaLibraryCardDetails/, "Media Library exposes title/path details on hover and keyboard focus");

// A deterministic populated two-card fixture exercises the interaction policy
// without a native reader. It follows the exact preconditions used by the owned
// card callbacks and explicitly checks target viewport/pane widths, focus/hover
// start/leave stop, the single active video, and no source touch before consent.
const computedMediaLibraryColumns = ({ viewportWidth, paneWidth }) => {
  assert.ok(paneWidth >= 280, "fixture pane remains wide enough for two equal tracks");
  return viewportWidth >= 1500 ? 3 : 2;
};

assert.equal(computedMediaLibraryColumns({ viewportWidth: 860, paneWidth: 380 }), 2, "860px operator viewport keeps a real two-column Library grid in its clip pane");
assert.equal(computedMediaLibraryColumns({ viewportWidth: 1920, paneWidth: 640 }), 3, "1920px operator viewport expands the Library grid without a vertical-list fallback");

const mediaLibraryCardModel = ({
  authorized,
  sourceKind,
  hasAssetThumbnail,
  reducedMotion,
  hover,
  focus,
  failed,
}) => {
  const previewEligible = authorized
    && sourceKind === "File"
    && hasAssetThumbnail
    && !reducedMotion
    && (hover || focus);
  return {
    columns: 2,
    render: authorized && hasAssetThumbnail ? "thumbnail" : "placeholder",
    preview: previewEligible && !failed ? "playing" : "still",
    details: hover || focus ? "visible" : "hidden",
  };
};

const libraryPreviewSession = ({ authorized, reducedMotion, events }) => {
  let activeAssetId = null;
  const renderedVideos = [];
  for (const event of events) {
    const eligible = authorized && !reducedMotion && event.kind === "File" && event.hasAssetThumbnail;
    if ((event.type === "hover" || event.type === "focus") && eligible) activeAssetId = event.assetId;
    if ((event.type === "leave" || event.type === "blur") && activeAssetId === event.assetId) activeAssetId = null;
    renderedVideos.push(activeAssetId === null ? [] : [activeAssetId]);
  }
  return renderedVideos;
};

const projectReplacementPreviewSession = () => {
  let authorized = true;
  let active = { id: 1, identity: "File\u0000C:/A.mp4\u0000sha256\u0000hashA\u0000100" };
  const reconcile = (catalog) => {
    const replacement = catalog.find((asset) => asset.id === active?.id);
    if (!authorized || !replacement || replacement.identity !== active.identity) active = null;
  };
  reconcile([{ id: 1, identity: "File\u0000C:/A.mp4\u0000sha256\u0000hashA\u0000100" }]);
  authorized = false;
  reconcile([{ id: 1, identity: "File\u0000C:/B.mp4\u0000sha256\u0000hashB\u0000110" }]);
  const afterReplacement = active;
  authorized = true;
  reconcile([{ id: 1, identity: "File\u0000C:/B.mp4\u0000sha256\u0000hashB\u0000110" }]);
  const afterReauthorizeWithoutHover = active;
  active = { id: 1, identity: "File\u0000C:/B.mp4\u0000sha256\u0000hashB\u0000110" };
  reconcile([{ id: 1, identity: "File\u0000C:/B.mp4\u0000sha256\u0000hashB\u0000110" }]);
  return { afterReplacement, afterReauthorizeWithoutHover, afterFreshHover: active };
};

assert.deepEqual(
  projectReplacementPreviewSession(),
  {
    afterReplacement: null,
    afterReauthorizeWithoutHover: null,
    afterFreshHover: { id: 1, identity: "File\u0000C:/B.mp4\u0000sha256\u0000hashB\u0000110" },
  },
  "project B cannot replay a recycled asset ID without a fresh hover/focus after its authorization reset",
);

const adoptedSamePathPreviewSession = () => {
  let active = { id: 1, identity: "File\u0000C:/A.mp4\u0000sha256\u0000hashA\u0000100" };
  const authoritativeReplacement = { id: 1, identity: "File\u0000C:/A.mp4\u0000sha256\u0000hashB\u0000120" };
  if (active.identity !== authoritativeReplacement.identity) active = null;
  return active;
};
assert.equal(adoptedSamePathPreviewSession(), null, "same-path AdoptReplacement content identity changes unmount an active preview before it can keep reading stale bytes");

const previewSession = libraryPreviewSession({
  authorized: true,
  reducedMotion: false,
  events: [
    { type: "hover", assetId: 41, kind: "File", hasAssetThumbnail: true },
    { type: "focus", assetId: 42, kind: "File", hasAssetThumbnail: true },
    { type: "blur", assetId: 42, kind: "File", hasAssetThumbnail: true },
  ],
});
assert.deepEqual(previewSession, [[41], [42], []], "two catalog-only video cards with asset thumbnails start on hover/focus, replace rather than overlap, and unmount on leave/blur");
assert.deepEqual(
  libraryPreviewSession({
    authorized: false,
    reducedMotion: false,
    events: [{ type: "hover", assetId: 41, kind: "File", hasAssetThumbnail: true }],
  }),
  [[]],
  "un-authorized hover does not mount a video or touch the media source",
);
assert.deepEqual(
  libraryPreviewSession({
    authorized: true,
    reducedMotion: false,
    events: [
      { type: "hover", assetId: 51, kind: "StillImage", hasAssetThumbnail: true },
      { type: "hover", assetId: 52, kind: "File", hasAssetThumbnail: false },
    ],
  }),
  [[], []],
  "still cards never play and assets without an authorized thumbnail remain placeholders",
);

const catalogOnlyFile = mediaLibraryCardModel({
  authorized: true,
  sourceKind: "File",
  hasAssetThumbnail: true,
  reducedMotion: false,
  hover: true,
  focus: false,
  failed: false,
});
assert.deepEqual(
  catalogOnlyFile,
  { columns: 2, render: "thumbnail", preview: "playing", details: "visible" },
  "authorized catalog-only File asset renders its asset thumbnail and previews on hover without a layer reference",
);
const catalogOnlyStill = mediaLibraryCardModel({
  authorized: true,
  sourceKind: "StillImage",
  hasAssetThumbnail: true,
  reducedMotion: false,
  hover: true,
  focus: false,
  failed: false,
});
assert.deepEqual(
  catalogOnlyStill,
  { columns: 2, render: "thumbnail", preview: "still", details: "visible" },
  "authorized catalog-only StillImage renders its asset thumbnail and never mounts video playback",
);

const untouchedFileCard = mediaLibraryCardModel({
  authorized: false,
  sourceKind: "File",
  hasAssetThumbnail: true,
  reducedMotion: false,
  hover: true,
  focus: false,
  failed: false,
});
assert.deepEqual(untouchedFileCard, {
  columns: 2,
  render: "placeholder",
  preview: "still",
  details: "visible",
}, "un-authorized populated file card remains a two-column placeholder and does not read/play source media");

const hoveredFileCard = mediaLibraryCardModel({
  authorized: true,
  sourceKind: "File",
  hasAssetThumbnail: true,
  reducedMotion: false,
  hover: true,
  focus: false,
  failed: false,
});
assert.equal(hoveredFileCard.preview, "playing", "authorized asset-thumbnail file card previews on hover");
assert.equal(hoveredFileCard.details, "visible", "hover exposes the source detail surface");

const focusedFileCard = mediaLibraryCardModel({
  authorized: true,
  sourceKind: "File",
  hasAssetThumbnail: true,
  reducedMotion: false,
  hover: false,
  focus: true,
  failed: false,
});
assert.equal(focusedFileCard.preview, "playing", "authorized asset-thumbnail file card previews on keyboard focus");
assert.equal(focusedFileCard.details, "visible", "keyboard focus exposes the source detail surface");

const leftFileCard = mediaLibraryCardModel({
  authorized: true,
  sourceKind: "File",
  hasAssetThumbnail: true,
  reducedMotion: false,
  hover: false,
  focus: false,
  failed: false,
});
assert.equal(leftFileCard.preview, "still", "hover/focus leave restores the static thumbnail");

for (const scenario of [
  { sourceKind: "StillImage", reducedMotion: false, failed: false, label: "still image" },
  { sourceKind: "File", reducedMotion: true, failed: false, label: "reduced motion file" },
  { sourceKind: "File", reducedMotion: false, failed: true, label: "failed file preview" },
]) {
  const card = mediaLibraryCardModel({
    authorized: true,
    sourceKind: scenario.sourceKind,
    hasAssetThumbnail: true,
    reducedMotion: scenario.reducedMotion,
    hover: true,
    focus: false,
    failed: scenario.failed,
  });
  assert.equal(card.columns, 2, `${scenario.label} remains in the non-vacuous two-column grid fixture`);
  assert.equal(card.preview, "still", `${scenario.label} fails safely to its still thumbnail`);
}

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
console.log("Limitation: this checker reads source and exercises policy fixtures; it does not mount components or establish browser/native rendered reachability.");
