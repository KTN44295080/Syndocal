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
const destinationStartControl = panel.match(
  /<label>\s*Destination start[\s\S]*?<\/label>/,
)?.[0];
const fadeControl = panel.match(
  /<label>\s*Fade \(ms\)[\s\S]*?<\/label>/,
)?.[0];

assert.ok(updateFollowBlock, "Follow patch handler is present");
assert.ok(holdControl, "Follow hold checkbox control is present");
assert.ok(transitionControl, "Follow transition control is present");
assert.ok(destinationStartControl, "Follow destination-start control is present");
assert.ok(fadeControl, "Follow fade control is present");

assert.match(
  types,
  /export interface TimelineFollowSummary \{[\s\S]*?hold_first_destination_measure\?: boolean;/,
  "TimelineFollowSummary keeps the hold flag additive and optional",
);
assert.match(
  types,
  /export type TimelineFollowDestinationStartMode = "play" \| "wait_for_pedal";[\s\S]*?destination_start_mode\?: TimelineFollowDestinationStartMode;/,
  "TimelineFollowSummary exposes the additive persisted destination-start mode",
);
assert.match(
  panel,
  /fault_policy: "hold",\s*hold_first_destination_measure: false,\s*destination_start_mode: "play",/,
  "new Follow defaults to immediate play without a destination hold",
);
assert.match(
  panel,
  /const followDestinationHoldEnabled = \(\) =>\s*!followIsCut\(\) && \(currentFollow\(\)\?\.hold_first_destination_measure \?\? false\);/,
  "the hold checkbox preserves true for non-Cut and is false/disabled for Cut",
);
assert.match(
  holdControl,
  /checked=\{followDestinationHoldEnabled\(\)\}[\s\S]*?disabled=\{followIsCut\(\) \|\| followDestinationWaitsForPedal\(\)\}[\s\S]*?onChange=\{\(event\) => updateFollow\(\{ hold_first_destination_measure: event\.currentTarget\.checked \}\)\}/,
  "non-Cut immediate-play edits write the checkbox value through the existing Follow patch path",
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
  /if \(nextFollow\.video_kind === "Cut"\) \{\s*nextFollow\.hold_first_destination_measure = false;\s*nextFollow\.destination_start_mode = "play";\s*\} else if \(nextFollow\.destination_start_mode === "wait_for_pedal"\) \{\s*nextFollow\.hold_first_destination_measure = false;\s*\}\s*void props\.onSetFollow\(nextFollow\);/,
  "Cut and wait-for-pedal Follow payloads normalize incompatible destination holds",
);
assert.match(
  destinationStartControl,
  /data-timeline-follow-destination-start[\s\S]*?value=\{currentFollow\(\)\?\.destination_start_mode \?\? "play"\}[\s\S]*?<option value="play">Play immediately<\/option>[\s\S]*?<option value="wait_for_pedal">Wait for Pedal 1<\/option>/,
  "the operator explicitly chooses immediate play or a Pedal 1 start wait",
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
  /disabled=\{followIsCut\(\) \|\| followDurationLocked\(\)\}/,
  "one-measure hold/wait modes disable fade editing while Cut remains disabled",
);
assert.match(
  panel,
  /data-timeline-follow-hold-hint[\s\S]*?Transition uses one source measure, then the destination first measure loops until F13\./,
  "the hold hint documents the one-source-measure then F13 release behavior",
);
assert.match(
  panel,
  /Transition uses one source measure, then the destination waits at its start until Pedal 1\./,
  "the wait hint documents paused destination installation and Pedal 1 start",
);

console.log("Timeline Follow UI: immediate/hold and one-measure wait-for-Pedal-1 modes are explicit, normalized, and persisted (static model checks; no DOM harness).");
