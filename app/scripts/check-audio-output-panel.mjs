import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const read = (relativePath) =>
  readFile(resolve(scriptDirectory, relativePath), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const [panel, css] = await Promise.all([
  read("../src/components/AudioOutputPanel.tsx"),
  read("../src/components/AudioOutputPanel.css"),
]);

const assertions = [
  [
    /export type AudioOutputBackend = "normal-wasapi" \| "show-asio";/u,
    panel,
    "backend type must distinguish Normal WASAPI and Show ASIO",
  ],
  [
    /export type AudioOutputState = "Ready" \| "Locked" \| "Active" \| "Fault";/u,
    panel,
    "state type must expose the four exact operator states",
  ],
  [
    /export interface AudioOutputView[\s\S]*?backend: AudioOutputBackend;[\s\S]*?driverId: string;[\s\S]*?sampleRate: number \| null;[\s\S]*?bufferFrames: number \| null;[\s\S]*?programLeft: number \| null;[\s\S]*?programRight: number \| null;[\s\S]*?cue: number \| null;[\s\S]*?spare: number \| null;[\s\S]*?state: AudioOutputState;[\s\S]*?reason: string;/u,
    panel,
    "view type must carry backend, exact stream settings, channels, state, and reason",
  ],
  [
    /export interface AudioOutputOptions[\s\S]*?drivers: readonly AudioOutputDriverOption\[\];[\s\S]*?sampleRates: readonly AudioOutputRateOption\[\];[\s\S]*?bufferFrames: readonly AudioOutputBufferOption\[\];[\s\S]*?channels: readonly AudioOutputChannelOption\[\];[\s\S]*?hasSpare: boolean;/u,
    panel,
    "options type must expose driver/rate/buffer/channel capability lists",
  ],
  [
    /export interface AudioOutputPanelProps[\s\S]*?view: AudioOutputView;[\s\S]*?options: AudioOutputOptions;[\s\S]*?canRefresh: boolean;[\s\S]*?canRevalidate: boolean;[\s\S]*?canStart: boolean;[\s\S]*?canStop: boolean;[\s\S]*?canReturnToNormal: boolean;[\s\S]*?canTest: boolean;/u,
    panel,
    "props type must expose parent-owned action gates",
  ],
  [/Normal WASAPI/u, panel, "Normal WASAPI label must be visible"],
  [/Show ASIO/u, panel, "Show ASIO label must be visible"],
  [/aria-label="Audio output backend"/u, panel, "backend selector needs an accessible name"],
  [/aria-label="Audio output driver"/u, panel, "driver selector needs an accessible name"],
  [/aria-label="Audio output sample rate"/u, panel, "sample-rate selector needs an accessible name"],
  [/aria-label="Audio output buffer"/u, panel, "buffer selector needs an accessible name"],
  [/label="PROGRAM L"/u, panel, "PROGRAM L channel selector must be present"],
  [/label="PROGRAM R"/u, panel, "PROGRAM R channel selector must be present"],
  [/label="CUE"/u, panel, "CUE channel selector must be present"],
  [/label="Spare"/u, panel, "optional Spare channel selector must be present"],
  [/data-audio-output-action="refresh"[\s\S]*?>\s*Refresh\s*</u, panel, "Refresh action must be present"],
  [/data-audio-output-action="revalidate"[\s\S]*?>\s*Revalidate\s*</u, panel, "Revalidate action must be present"],
  [/data-audio-output-action="start"[\s\S]*?>\s*Start\s*</u, panel, "Start action must be present"],
  [/data-audio-output-action="stop"[\s\S]*?>\s*Stop\s*</u, panel, "Stop action must be present"],
  [/data-audio-output-action="return-to-normal"[\s\S]*?>\s*Return to normal\s*</u, panel, "explicit Return to normal action must be present"],
  [/Test PROGRAM L/u, panel, "PROGRAM L test must be present"],
  [/Test PROGRAM R/u, panel, "PROGRAM R test must be present"],
  [/Test PROGRAM Stereo/u, panel, "PROGRAM stereo test must be present"],
  [/Test CUE/u, panel, "CUE test must be present"],
  [/Test Spare/u, panel, "Spare test must be present"],
  [/props\.testMode !== null && props\.testMode !== (?:test|kind)/u, panel, "test actions must remain mutually exclusive"],
  [/const testDisabled = [\s\S]*?props\.canTest[\s\S]*?props\.livePlaybackActive/u, panel, "tests must be disabled during live playback"],
  [/const soloDisabled = [\s\S]*?props\.canSolo[\s\S]*?props\.livePlaybackActive/u, panel, "solo must be disabled during live playback"],
  [/onTest\(null\)/u, panel, "the selected test must have an explicit stop path"],
  [/PROGRAM-only/u, panel, "PROGRAM-only preflight must be present"],
  [/CUE-only/u, panel, "CUE-only preflight must be present"],
  [/<details[\s\S]*?Output configuration[\s\S]*?<\/details>/u, panel, "configuration must be a compact disclosure group"],
  [/<details[\s\S]*?Preflight and tests[\s\S]*?<\/details>/u, panel, "preflight/tests must be a compact disclosure group"],
  [/import "\.\/AudioOutputPanel\.css";/u, panel, "panel must import only its local CSS surface"],
  [/Output \$\{option\.index \+ 1\}/u, panel, "channel labels must cross the zero-based boundary exactly once"],
];

for (const [pattern, source, message] of assertions) {
  assert.match(source, pattern, message);
}

assert.doesNotMatch(`${panel}\n${css}`, /\b(?:position\s*:\s*(?:fixed|sticky)|portal|<dialog)\b/iu, "panel must remain an in-flow deck item");
assert.doesNotMatch(`${panel}\n${css}`, /MOTU/iu, "panel must not hard-code a device name");
assert.doesNotMatch(`${panel}\n${css}`, /autoplay|auto[- ]start|fallback/iu, "panel must not offer automatic start or fallback behavior");
assert.doesNotMatch(`${panel}\n${css}`, /project\s+save|save\s+project/iu, "panel must not expose project-saving text");

const cssRuleSelectors = [...css.matchAll(/(?:^|})\s*([^@{}]+)\{/gm)]
  .map((match) => match[1].trim())
  .filter((selector) => selector.length > 0);
for (const selectorGroup of cssRuleSelectors) {
  for (const selector of selectorGroup.split(",")) {
    assert.match(selector.trim(), /^\.audioOutput(?:[A-Z-]|$)/u, `CSS selector must stay local: ${selector.trim()}`);
  }
}

console.log(`audio output panel checks passed (${assertions.length + 5} assertions)`);
