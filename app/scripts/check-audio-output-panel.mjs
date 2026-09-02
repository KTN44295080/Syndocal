import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const read = (relativePath) =>
  readFile(resolve(scriptDirectory, relativePath), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const [panel, css, app] = await Promise.all([
  read("../src/components/AudioOutputPanel.tsx"),
  read("../src/components/AudioOutputPanel.css"),
  read("../src/App.tsx"),
]);

const assertions = [
  [
    /export type AudioOutputBackend = "normal-wasapi" \| "show-asio";/u,
    panel,
  "backend type must distinguish Normal WASAPI and ASIO",
  ],
  [
    /export type AudioOutputState = "Ready" \| "Locked" \| "Active" \| "Fault";/u,
    panel,
    "state type must expose the four exact operator states",
  ],
  [
    /export type AudioOutputCueRoute = "same-asio" \| "split-device";/u,
    panel,
    "CUE route type must expose only same-ASIO and split-device",
  ],
  [
    /export interface AudioOutputView[\s\S]*?backend: AudioOutputBackend;[\s\S]*?driverId: string;[\s\S]*?sampleRate: number \| null;[\s\S]*?bufferFrames: number \| null;[\s\S]*?programLeft: number \| null;[\s\S]*?programRight: number \| null;[\s\S]*?cue: number \| null;[\s\S]*?cueRoute: AudioOutputCueRoute;[\s\S]*?cueDeviceName: string \| null;[\s\S]*?cueTopologyFingerprint: string \| null;[\s\S]*?spare: number \| null;[\s\S]*?state: AudioOutputState;[\s\S]*?reason: string;/u,
    panel,
    "view type must carry backend, exact stream settings, channels, state, and reason",
  ],
  [
    /export interface AudioOutputOptions[\s\S]*?drivers: readonly AudioOutputDriverOption\[\];[\s\S]*?sampleRates: readonly AudioOutputRateOption\[\];[\s\S]*?bufferFrames: readonly AudioOutputBufferOption\[\];[\s\S]*?channels: readonly AudioOutputChannelOption\[\];[\s\S]*?cueEndpoints: readonly AudioOutputCueEndpointOption\[\];[\s\S]*?hasSpare: boolean;/u,
    panel,
    "options type must expose driver/rate/buffer/channel capability lists",
  ],
  [
    /export interface AudioOutputPanelProps[\s\S]*?view: AudioOutputView;[\s\S]*?options: AudioOutputOptions;[\s\S]*?canRefresh: boolean;[\s\S]*?canRevalidate: boolean;[\s\S]*?canStart: boolean;[\s\S]*?canStop: boolean;[\s\S]*?canReturnToNormal: boolean;/u,
    panel,
    "props type must expose parent-owned action gates",
  ],
  [/Normal WASAPI/u, panel, "Normal WASAPI label must be visible"],
  [/props\.view\.backend === "normal-wasapi"[\s\S]*?"Windows default PROGRAM output"[\s\S]*?: "PROGRAM stereo and CUE output control"/u, panel, "header subtitle must describe the active backend instead of advertising hidden ASIO controls"],
  [/<Show when=\{props\.view\.backend === "show-asio"\}>\s*<div class="audioOutputActions"/u, panel, "ASIO lifecycle actions must stay hidden in Normal WASAPI mode"],
  [/classList=\{\{ "audioOutputFieldGrid--normal": props\.view\.backend === "normal-wasapi" \}\}/u, panel, "Normal WASAPI configuration must collapse to one useful field"],
  [/<Show when=\{props\.view\.backend === "normal-wasapi"\}>[\s\S]*?data-audio-output-normal-notice[\s\S]*?<TimelineCueAudioRoutingPanel/u, panel, "Normal WASAPI must expose authoritative Timeline WDM routing in Setup Audio"],
  [/<Show when=\{props\.view\.backend === "show-asio"\}>\s*<div class="audioOutputCueRouting"/u, panel, "ASIO routing must stay hidden in Normal WASAPI mode"],
  [/\{ value: "show-asio", label: "ASIO" \}/u, panel, "ASIO label must be visible"],
  [/aria-label="Audio output backend"/u, panel, "backend selector needs an accessible name"],
  [/aria-label="Audio output driver"/u, panel, "driver selector needs an accessible name"],
  [/aria-label="Audio output sample rate"/u, panel, "sample-rate selector needs an accessible name"],
  [/aria-label="Audio output buffer"/u, panel, "buffer selector needs an accessible name"],
  [/aria-label="CUE route"/u, panel, "CUE route selector needs an accessible name"],
  [/aria-label="CUE WDM endpoint"/u, panel, "CUE WDM endpoint selector needs an accessible name"],
  [/label="PROGRAM L"/u, panel, "PROGRAM L channel selector must be present"],
  [/label="PROGRAM R"/u, panel, "PROGRAM R channel selector must be present"],
  [/label="CUE"/u, panel, "CUE channel selector must be present"],
  [/label="Spare"/u, panel, "optional Spare channel selector must be present"],
  [/data-audio-output-row="program"[\s\S]*?PROGRAM L[\s\S]*?PROGRAM R/u, panel, "PROGRAM must be one explicit two-channel row"],
  [/data-audio-output-row="cue"[\s\S]*?same-asio[\s\S]*?split-device/u, panel, "CUE row must switch between same-ASIO and split-device controls"],
  [/data-no-localize/u, panel, "WDM endpoint names must opt out of UI localization"],
  [/data-audio-output-clock-warning[^>]*role="alert"[\s\S]*?PROGRAM and CUE use separate device clocks\. Timing can drift; no clock lock is claimed\./u, panel, "split-device clock warning must be exact and alert-visible"],
  [/data-audio-output-action="refresh"[\s\S]*?>\s*Refresh\s*</u, panel, "Refresh action must be present"],
  [/data-audio-output-action="revalidate"[\s\S]*?>\s*Revalidate\s*</u, panel, "Revalidate action must be present"],
  [/data-audio-output-action="start"[\s\S]*?>\s*Start\s*</u, panel, "Start action must be present"],
  [/data-audio-output-action="stop"[\s\S]*?>\s*Stop\s*</u, panel, "Stop action must be present"],
  [/data-audio-output-action="return-to-normal"[\s\S]*?>\s*Return to normal\s*</u, panel, "explicit Return to normal action must be present"],
  [/data-audio-output-disclosure="preflight"|data-audio-output-test|onTest|onSoloModeChange|PROGRAM-only|CUE-only|Safe audio output tests/u, panel, "debug preflight/test operations must not be exposed by the normal audio panel", true],
  [/canTest: boolean;|canSolo: boolean;|livePlaybackActive: boolean;|testMode: AudioOutputTest|soloMode: AudioOutputSoloMode/u, panel, "debug-only state must not be part of the normal audio panel props", true],
  [/const testDisabled|const soloDisabled|testLabel\(|testKinds/u, panel, "debug-only UI helpers must not remain in the normal audio panel", true],
  [/<details[\s\S]*?Output configuration[\s\S]*?<\/details>/u, panel, "configuration must be a compact disclosure group"],
  [/<details[\s\S]*?preflight[\s\S]*?<\/details>/u, panel, "preflight/test disclosure must not be present in the normal audio panel", true],
  [/<AudioOutputPanel[\s\S]*?canTest=|<AudioOutputPanel[\s\S]*?onTest=|<AudioOutputPanel[\s\S]*?showAdvancedDiagnostics=\{true\}/u, app, "production AudioOutputPanel must not wire debug operations into the normal UI", true],
  [/import "\.\/AudioOutputPanel\.css";/u, panel, "panel must import only its local CSS surface"],
  [/Output \$\{option\.index \+ 1\}/u, panel, "channel labels must cross the zero-based boundary exactly once"],
];

for (const [pattern, source, message, negative = false] of assertions) {
  if (negative) assert.doesNotMatch(source, pattern, message);
  else assert.match(source, pattern, message);
}

assert.doesNotMatch(`${panel}\n${css}`, /\b(?:position\s*:\s*(?:fixed|sticky)|portal|<dialog)\b/iu, "panel must remain an in-flow deck item");
assert.doesNotMatch(`${panel}\n${css}`, /MOTU/iu, "panel must not hard-code a device name");
assert.doesNotMatch(`${panel}\n${css}`, /autoplay|auto[- ]start|fallback/iu, "panel must not offer automatic start or fallback behavior");
assert.doesNotMatch(`${panel}\n${css}`, /project\s+save|save\s+project/iu, "panel must not expose project-saving text");
assert.match(css, /\.audioOutputFieldGrid\.audioOutputFieldGrid--normal\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 360px\);/u, "Normal WASAPI layout must not reserve empty ASIO columns or overflow narrow I/O panes");

const cssRuleSelectors = [...css.matchAll(/(?:^|})\s*([^@{}]+)\{/gm)]
  .map((match) => match[1].trim())
  .filter((selector) => selector.length > 0);
for (const selectorGroup of cssRuleSelectors) {
  for (const selector of selectorGroup.split(",")) {
    assert.match(selector.trim(), /^\.audioOutput(?:[A-Z-]|$)/u, `CSS selector must stay local: ${selector.trim()}`);
  }
}

console.log(`audio output panel checks passed (${assertions.length + 5} assertions)`);
