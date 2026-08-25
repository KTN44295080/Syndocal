import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const observationPath = path.join(appRoot, "src", "videoOutputWindowObservation.ts");
const appPath = path.join(appRoot, "src", "App.tsx");
const invokesPath = path.join(appRoot, "src", "tauriInvokeCommands.ts");
const manifestPath = path.join(appRoot, "src", "tauri-invoke-manifest.json");
const command = "get_video_output_window_observation_v1";

const source = await readFile(observationPath, "utf8");
const appSource = await readFile(appPath, "utf8");
// Syntax/emit smoke only. Project-wide type correctness is proved separately
// by the mandatory `pnpm --dir app exec tsc --noEmit` integration gate.
const emitted = ts.transpileModule(source, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    strict: true,
  },
  fileName: observationPath,
  reportDiagnostics: true,
});
const diagnostics = emitted.diagnostics ?? [];
assert.equal(
  diagnostics.filter((diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error).length,
  0,
  diagnostics.map((diagnostic) => ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")).join("\n"),
);
const moduleUrl = `data:text/javascript;base64,${Buffer.from(emitted.outputText, "utf8").toString("base64")}`;
const observation = await import(moduleUrl);

const exactLive = (id, label, hwnd) => ({
  output_id: id,
  label,
  live_open: true,
  live_window_label: `video-output-${id}`,
  native_window_handle_decimal: hwnd,
});
const exactClosed = (id, label) => ({
  output_id: id,
  label,
  live_open: false,
  live_window_label: `video-output-${id}`,
  native_window_handle_decimal: null,
});
const exactRoot = (outputs) => ({
  schema_version: 1,
  source: "app-owned-read-only",
  outputs,
});

const positive = await observation.requestVideoOutputWindowObservationV1(async () => exactRoot([
  exactLive("41", "LED Program", "1001"),
  exactLive("42", "Projector Program", "1002"),
  exactLive("18446744073709551615", "Maximum ID", "18446744073709551615"),
  exactClosed("43", "Spare"),
]));
assert.deepEqual(positive, exactRoot([
  exactLive("41", "LED Program", "1001"),
  exactLive("42", "Projector Program", "1002"),
  exactLive("18446744073709551615", "Maximum ID", "18446744073709551615"),
  exactClosed("43", "Spare"),
]));

const reject = async (name, value, expected) => {
  await assert.rejects(
    () => observation.requestVideoOutputWindowObservationV1(async () => value),
    (error) => error instanceof Error && error.message.includes(expected),
    name,
  );
};

await reject("future schema", { ...exactRoot([]), schema_version: 2 }, "schema_version is unsupported");
await reject("proxy source", { ...exactRoot([]), source: "operator-authored" }, "not app-owned-read-only");
await reject("unknown root field", { ...exactRoot([]), proof_file: "C:\\fake.json" }, "unsupported field set");
await reject("outputs is not an array", { ...exactRoot([]), outputs: {} }, "outputs must be an array");
await reject("entry is not an object", exactRoot([null]), "output 0 must be an object");
await reject("unknown entry field", exactRoot([{
  ...exactLive("41", "LED", "11"),
  title_match: true,
}]), "output 0 has an unsupported field set");
await reject("duplicate output ID", exactRoot([exactLive("41", "LED", "11"), exactLive("41", "Projector", "12")]), "duplicates output ID 41");
await reject("test pattern label", exactRoot([{
  ...exactLive("41", "LED", "11"),
  live_window_label: "video-output-41-test-pattern",
}]), "not its exact live label");
await reject("live output lacks HWND", exactRoot([{
  ...exactLive("41", "LED", "11"),
  native_window_handle_decimal: null,
}]), "HWND must be one canonical positive decimal string");
await reject("closed output retains HWND", exactRoot([{
  ...exactClosed("41", "LED"),
  native_window_handle_decimal: "11",
}]), "must have null HWND");
await reject("empty label", exactRoot([exactLive("41", "", "11")]), "label must be one non-empty single-line string");
await reject("newline label", exactRoot([exactLive("41", "LED\nProgram", "11")]), "label must be one non-empty single-line string");
await reject("live_open is not boolean", exactRoot([{
  ...exactLive("41", "LED", "11"),
  live_open: 1,
}]), "live_open must be boolean");
await reject("numeric output ID", exactRoot([exactLive(41, "LED", "11")]), "ID must be one canonical positive decimal string");
await reject("zero output ID", exactRoot([exactLive("0", "LED", "11")]), "ID must be one canonical positive decimal string");
await reject("leading-zero output ID", exactRoot([exactLive("041", "LED", "11")]), "ID must be one canonical positive decimal string");
await reject("signed output ID", exactRoot([exactLive("+41", "LED", "11")]), "ID must be one canonical positive decimal string");
await reject("spaced output ID", exactRoot([exactLive(" 41", "LED", "11")]), "ID must be one canonical positive decimal string");
await reject("overflow output ID", exactRoot([exactLive("18446744073709551616", "LED", "11")]), "unsigned 64-bit range");
await reject("numeric HWND", exactRoot([exactLive("41", "LED", 11)]), "HWND must be one canonical positive decimal string");
await reject("fractional HWND", exactRoot([exactLive("41", "LED", "1.5")]), "HWND must be one canonical positive decimal string");
await reject("zero HWND", exactRoot([exactLive("41", "LED", "0")]), "HWND must be one canonical positive decimal string");
await reject("leading-zero HWND", exactRoot([exactLive("41", "LED", "011")]), "HWND must be one canonical positive decimal string");
await reject("signed HWND", exactRoot([exactLive("41", "LED", "+11")]), "HWND must be one canonical positive decimal string");
await reject("spaced HWND", exactRoot([exactLive("41", "LED", " 11")]), "HWND must be one canonical positive decimal string");
await reject("overflow HWND", exactRoot([exactLive("41", "LED", "18446744073709551616")]), "unsigned 64-bit range");

let invokedCommand = null;
const requested = await observation.requestVideoOutputWindowObservationV1(async (receivedCommand) => {
  invokedCommand = receivedCommand;
  return exactRoot([exactLive("41", "LED Program", "1001")]);
});
assert.equal(invokedCommand, command, "receiver invokes only the exact app-owned observation command");
assert.equal(requested.outputs[0].native_window_handle_decimal, "1001");

const invokeTuple = await readFile(invokesPath, "utf8");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const tupleCommands = [...invokeTuple.matchAll(/^\s+"([a-z0-9_]+)",$/gm)].map((match) => match[1]);
assert.deepEqual(tupleCommands, [...tupleCommands].sort(), "typed invoke list remains sorted");
assert.deepEqual(manifest, [...manifest].sort(), "invoke manifest remains sorted");
assert.ok(tupleCommands.includes(command), "typed invoke list admits the exact observation command");
assert.ok(manifest.includes(command), "invoke manifest admits the exact observation command");
assert.match(source, /operator-authored files, environment variables, or arbitrary JSON/, "receiver rejects proxy input routes by contract");
assert.match(source, /get_video_output_window_observation_v1/, "receiver names only the dedicated command");
assert.doesNotMatch(source, /export const parseVideoOutputWindowObservationV1/, "raw parser is not an application entrypoint");
assert.match(
  appSource,
  /requestVideoOutputWindowObservationV1\(\(command\) => invoke\(command\)\)/,
  "the mounted native QA consumer must route the strict receiver through the normal typed invoke facade",
);
assert.match(
  appSource,
  /if \(isTauriRuntime\(\)\)[\s\S]*?window\.__syndocalReadVideoOutputWindowObservationV1 = readVideoOutputWindowObservationV1;/,
  "the strict receiver must be available only from a mounted Tauri App",
);
assert.match(
  appSource,
  /if \(getCurrentWindow\(\)\.label !== "main"\)[\s\S]*?throw new Error\("Video output window observation is available only from the main Syndocal window\."\)/,
  "the app-owned reader must self-verify the exact main Tauri window before invoking the backend",
);
assert.match(
  appSource,
  /window\.__syndocalReadVideoOutputWindowObservationV1 === readVideoOutputWindowObservationV1[\s\S]*?delete window\.__syndocalReadVideoOutputWindowObservationV1;/,
  "cleanup must remove only the exact mounted native QA consumer identity",
);
assert.doesNotMatch(
  appSource,
  /__syndocalReadVideoOutputWindowObservationV1[\s\S]{0,300}__TAURI_INTERNALS__/,
  "the native QA consumer must not bypass the typed invoke facade through raw Tauri internals",
);
assert.doesNotMatch(
  appSource,
  /plugin:window\|get_current_window/,
  "the reader must not depend on the nonexistent legacy window-label command",
);

console.log("video-output window observation contract passed");
