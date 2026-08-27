import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const appRoot = new URL("..", import.meta.url);
const read = (path) => readFile(new URL(path, appRoot), "utf8");
const [types, panel] = await Promise.all([
  read("src/types.ts"),
  read("src/components/TimelineBankPanel.tsx"),
]);
const updateFollowBlock = panel.match(
  /const updateFollow = \(patch: Partial<TimelineFollowSummary>\) => \{[\s\S]*?\n  \};/,
)?.[0];
const holdControl = panel.match(
  /<label class="toggleRow">(?:(?!<\/label>)[\s\S])*?data-timeline-follow-hold(?:(?!<\/label>)[\s\S])*?<\/label>/,
)?.[0];
const transitionControl = panel.match(
  /<label>\s*Video transition[\s\S]*?<\/label>/,
)?.[0];
const fadeControl = panel.match(
  /<label>\s*Fade \(ms\)[\s\S]*?<\/label>/,
)?.[0];

assert.ok(updateFollowBlock, "Follow patch handler is present");
assert.ok(holdControl, "Follow hold checkbox control is present");
assert.ok(transitionControl, "Follow transition control is present");
assert.ok(fadeControl, "Follow fade control is present");

assert.match(
  types,
  /export interface TimelineFollowSummary \{[\s\S]*?hold_first_destination_measure\?: boolean;/,
  "TimelineFollowSummary keeps the hold flag additive and optional",
);
assert.match(
  panel,
  /fault_policy: "hold",\s*hold_first_destination_measure: false,/,
  "new Follow defaults the hold flag to false",
);
assert.match(
  panel,
  /const followDestinationHoldEnabled = \(\) =>\s*!followIsCut\(\) && \(currentFollow\(\)\?\.hold_first_destination_measure \?\? false\);/,
  "the hold checkbox preserves true for non-Cut and is false/disabled for Cut",
);
assert.match(
  holdControl,
  /checked=\{followDestinationHoldEnabled\(\)\}[\s\S]*?disabled=\{followIsCut\(\)\}[\s\S]*?onChange=\{\(event\) => updateFollow\(\{ hold_first_destination_measure: event\.currentTarget\.checked \}\)\}/,
  "non-Cut edits write the checkbox value through the existing Follow patch path",
);
assert.doesNotMatch(
  holdControl,
  /props\.onSetFollow\(/,
  "the hold checkbox does not bypass the Follow patch path",
);
assert.match(
  updateFollowBlock,
  /const nextFollow: TimelineFollowSummary = \{\s*\.\.\.base,\s*\.\.\.patch,/,
  "editing an existing Follow starts from its current fields and preserves the hold flag",
);
assert.match(
  updateFollowBlock,
  /if \(nextFollow\.video_kind === "Cut"\) \{\s*nextFollow\.hold_first_destination_measure = false;\s*\}\s*void props\.onSetFollow\(nextFollow\);/,
  "Cut Follow payloads send a normalized false hold flag",
);
assert.ok(
  transitionControl.includes("const videoKind = event.currentTarget.value as VideoClipTakeKind;")
  && transitionControl.includes("updateFollow({")
  && transitionControl.includes("video_kind: videoKind,")
  && transitionControl.includes('value_milliunits: videoKind === "Cut" ? 0 :'),
  "Cut selection uses the normalized Follow patch path and zero-duration transition",
);
assert.match(
  fadeControl,
  /disabled=\{followIsCut\(\) \|\| followDestinationHoldEnabled\(\)\}/,
  "holding the destination measure disables fade editing while Cut remains disabled",
);
assert.match(
  panel,
  /data-timeline-follow-hold-hint[\s\S]*?Transition uses one source measure, then the destination first measure loops until F13\./,
  "the hold hint documents the one-source-measure then F13 release behavior",
);

console.log("Timeline Follow hold UI: true retention, non-Cut checkbox patching, Cut=false dispatch, and hold/fade guidance are present (static model checks; no DOM harness).");
