import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const files = new Map(
  await Promise.all(
    [
      "../src/components/TouchCuePanel.tsx",
      "../src/components/CueManagementPanel.tsx",
      "../src/components/DmxPatchMapPanel.tsx",
      "../src/components/SetupFixtureListPanel.tsx",
      "../src/components/SetupMappingWorkspace.tsx",
      "../src/components/TouchFixturePickerPanel.tsx",
      "../src/components/TouchVideoPanel.tsx",
      "../src/components/VideoLayerListPanel.tsx",
      "../src/components/VideoClipGridPanel.tsx",
    ].map(async (path) => [path, await readFile(new URL(path, import.meta.url), "utf8")]),
  ),
);

const expectedGuidance = [
  ["../src/components/TouchCuePanel.tsx", "No cues. Create one in Control &gt; Live with Store Cue."],
  ["../src/components/CueManagementPanel.tsx", "No cues. Choose a scope, then Store Cue."],
  ["../src/components/DmxPatchMapPanel.tsx", "Use Setup &gt; Patch to assign one."],
  ["../src/components/SetupFixtureListPanel.tsx", "Load a profile in Library, then use Patch Fixture."],
  ["../src/components/SetupMappingWorkspace.tsx", "Select a fixture in the patch grid or list to edit its setup."],
  ["../src/components/TouchFixturePickerPanel.tsx", "Patch one in Setup &gt; Patch."],
  ["../src/components/TouchVideoPanel.tsx", "Add one in Setup &gt; Output."],
  ["../src/components/VideoLayerListPanel.tsx", "Add a file, still, or input above."],
  ["../src/components/VideoClipGridPanel.tsx", "VJ Program is created Off and Blackout. No output window opens automatically."],
];

for (const [path, guidance] of expectedGuidance) {
  assert.ok(files.get(path)?.includes(guidance), `${path} is missing empty-state guidance: ${guidance}`);
}

console.log("empty state guidance ok");
