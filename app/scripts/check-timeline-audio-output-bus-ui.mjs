import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(path, import.meta.url), "utf8");

const [types, app, panel, overview, commands] = await Promise.all([
  read("../src/types.ts"),
  read("../src/App.tsx"),
  read("../src/components/TimelineCueEventsPanel.tsx"),
  read("../src/components/TimelineOverview.tsx"),
  read("../src/tauriInvokeCommands.ts"),
]);

const assertions = [
  [/export type TimelineAudioOutputBus = "PROGRAM" \| "CUE";/, types, "Timeline output bus must remain a closed PROGRAM/CUE union."],
  [/output_bus\?: TimelineAudioOutputBus;/, types, "Legacy frontend fixtures may omit output_bus while native migration resolves PROGRAM."],
  [/data-timeline-audio-output-bus[\s\S]*?<option value="PROGRAM">PROGRAM<\/option>[\s\S]*?<option value="CUE">CUE<\/option>/, panel, "Audio Clip properties must expose the exact PROGRAM/CUE selector."],
  [/value=\{clip\(\)\.output_bus \?\? "PROGRAM"\}/, panel, "Legacy clips must render as PROGRAM."],
  [/const setTimelineAudioClipOutputBus =[\s\S]*?invoke\("set_timeline_audio_clip_output_bus", \{[\s\S]*?id: clip\.id,[\s\S]*?outputBus,[\s\S]*?\}\);/, app, "Root Timeline bus edits must use the field-specific native command."],
  [/const setTimelineAudioClipOutputBus =[\s\S]*?invoke\("set_cue_child_timeline_audio_clip_output_bus", \{[\s\S]*?cueId: childCueId,[\s\S]*?clipId: clip\.id,[\s\S]*?outputBus,[\s\S]*?\}\);/, app, "Child Timeline bus edits must use the field-specific native command."],
  [/data-timeline-audio-output-bus=\{preview\(\)\.output_bus \?\? "PROGRAM"\}/, overview, "Timeline clips must expose their resolved logical bus marker."],
  [/const label = `\$\{outputBus\} · \$\{timelineAudioClipName\(clip\.path\)\}`;/, overview, "Timeline clip labels must visibly identify PROGRAM or CUE."],
  [/"set_timeline_audio_clip_output_bus"/, commands, "The frontend invoke inventory must include the field-specific bus command."],
  [/"set_cue_child_timeline_audio_clip_output_bus"/, commands, "The frontend invoke inventory must include the child field-specific bus command."],
];

for (const [pattern, source, message] of assertions) {
  if (!pattern.test(source)) throw new Error(message);
}

const explicitProgramInitializers = app.match(/output_bus: "PROGRAM"/g)?.length ?? 0;
if (explicitProgramInitializers < 2) {
  throw new Error("Both child and root Audio Clip creation paths must author PROGRAM explicitly.");
}

console.log(`timeline audio output bus UI checks passed (${assertions.length + 1} assertions)`);
