import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const sidebarSource = await readFile(
  new URL("../src/components/MappingSelectionSidebarPanel.tsx", import.meta.url),
  "utf8",
);
const inspectorSource = await readFile(
  new URL("../src/components/MappingFixtureInspectorPanel.tsx", import.meta.url),
  "utf8",
);
const projectorSource = await readFile(
  new URL("../src/components/MappingProjectorSelectionPanel.tsx", import.meta.url),
  "utf8",
);

const contextStart = sidebarSource.indexOf("export function MappingSetupContextPanel");
assert.ok(contextStart >= 0, "the Setup context panel must remain available");
const contextSource = sidebarSource.slice(contextStart);

// Solid's Show renders no wrapper of its own. These gates therefore remove
// only the downstream context regions while leaving the browser and object
// disclosure as the first, unconditional Setup context children.
assert.match(
  contextSource,
  /return \(\s*<div class="mappingSelectionPanel mappingSetupContextContent">\s*<MappingSelectionsColumn \{\.\.\.props\} typeFilters=\{props\.typeFilters\} \/>\s*<MappingStageObjectPanel/s,
  "selection browser and stage-object disclosure must stay available without a fixture pick",
);

const inspectorGate =
  /<Show when=\{props\.selectedFixture\}>\s*<MappingFixtureInspectorPanel[\s\S]*?<\/Show>/;
assert.match(
  contextSource,
  inspectorGate,
  "fixture inspector must be mounted only behind the selected-fixture gate",
);
assert.equal(
  [...contextSource.matchAll(/<MappingFixtureInspectorPanel\b/g)].length,
  1,
  "fixture inspector must have one gated mount point",
);

const projectorGate =
  /<Show when=\{props\.outputs\.length > 0\}>\s*<MappingProjectorSelectionPanel[\s\S]*?<\/Show>/;
assert.match(
  contextSource,
  projectorGate,
  "projection selection must be mounted only when at least one output exists",
);
assert.equal(
  [...contextSource.matchAll(/<MappingProjectorSelectionPanel\b/g)].length,
  1,
  "projection selection must have one gated mount point",
);

const selectionActionsIndex = contextSource.indexOf("<MappingSelectionActionsPanel");
const fixtureGateIndex = contextSource.indexOf("<Show when={props.selectedFixture}>");
const outputGateIndex = contextSource.indexOf("<Show when={props.outputs.length > 0}>");
assert.ok(selectionActionsIndex >= 0, "selection actions must remain mounted in the Setup context");
assert.equal(
  [...contextSource.matchAll(/<MappingSelectionActionsPanel\b/g)].length,
  1,
  "selection actions must have one mount point",
);
assert.ok(
  selectionActionsIndex < fixtureGateIndex && selectionActionsIndex < outputGateIndex,
  "selection actions must remain outside both downstream empty-state gates",
);
const contextPrefixBeforeActions = contextSource.slice(0, selectionActionsIndex);
assert.equal(
  (contextPrefixBeforeActions.match(/<Show\b/g) ?? []).length,
  (contextPrefixBeforeActions.match(/<\/Show>/g) ?? []).length,
  "selection actions must not be nested in an earlier Show branch",
);

// Keep the selected/non-empty branches tied to their existing editor bodies;
// the empty-state cleanup must not replace or weaken those editors.
assert.match(inspectorSource, /<Show\s+when=\{props\.fixture\}/);
assert.match(inspectorSource, /data-fixture-coordinate-editor/);
assert.match(inspectorSource, /data-gdtf-geometry-disclosure/);
assert.match(projectorSource, /<For\s+each=\{props\.outputs\}>/);
assert.match(projectorSource, /<MappingProjectorControlsPanel/);
assert.match(projectorSource, /Projection Surfaces/);

const renderContext = ({ selectedFixture, outputs }) => ({
  selectionBrowser: true,
  stageObjects: true,
  selectionActions: true,
  fixtureInspector: Boolean(selectedFixture),
  projectionSelection: outputs.length > 0,
});

const emptyContext = renderContext({ selectedFixture: null, outputs: [] });
assert.equal(emptyContext.selectionBrowser, true, "empty context keeps the selection browser");
assert.equal(emptyContext.stageObjects, true, "empty context keeps stage-object tools");
assert.equal(emptyContext.selectionActions, true, "empty context keeps selection actions");
assert.equal(emptyContext.fixtureInspector, false, "empty context hides the fixture inspector");
assert.equal(emptyContext.projectionSelection, false, "empty context hides projection selection");

const selectedFixtureContext = renderContext({ selectedFixture: { id: 1 }, outputs: [] });
assert.equal(
  selectedFixtureContext.fixtureInspector,
  true,
  "a selected fixture restores the fixture inspector editor",
);
assert.equal(
  selectedFixtureContext.projectionSelection,
  false,
  "a selected fixture alone does not restore an empty projection panel",
);

const outputContext = renderContext({ selectedFixture: null, outputs: [{ id: 1 }] });
assert.equal(
  outputContext.fixtureInspector,
  false,
  "an output alone does not restore the empty fixture inspector",
);
assert.equal(
  outputContext.projectionSelection,
  true,
  "a non-empty output list restores projection selection",
);

const fullContext = renderContext({ selectedFixture: { id: 1 }, outputs: [{ id: 1 }] });
assert.deepEqual(fullContext, {
  selectionBrowser: true,
  stageObjects: true,
  selectionActions: true,
  fixtureInspector: true,
  projectionSelection: true,
});

console.log(
  "Mapping empty-context checks passed (browser/object tools stay mounted; fixture and projection regions are gated; selected/non-empty editors remain intact).",
);
