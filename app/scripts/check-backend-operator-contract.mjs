import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const midi = await readFile(new URL("../../crates/io/src/midi.rs", import.meta.url), "utf8");
const osc = await readFile(new URL("../../crates/io/src/osc.rs", import.meta.url), "utf8");
const remote = await readFile(new URL("../../crates/io/src/remote_ws.rs", import.meta.url), "utf8");

const section = (source, start, end) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing section start: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing section end: ${end}`);
  return source.slice(startIndex, endIndex);
};

const mutationSection = section(app, "const projectMutationCommands = new Set([", "const projectMutationLabel");
const mutationCommands = [...mutationSection.matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]);
const handlerSection = section(backend, ".invoke_handler(tauri::generate_handler![", ".build(tauri::generate_context!())");
const registeredCommands = new Set(
  [...handlerSection.matchAll(/^\s*([a-z][a-z0-9_]+),?\s*$/gm)].map((match) => match[1]),
);
const literalFrontendCommands = new Set(
  [...app.matchAll(/\b(?:invoke|tauriInvoke)(?:<[^;()]*?>)?\(\s*"([a-z0-9_]+)"/g)].map((match) => match[1]),
);

const unregisteredMutations = mutationCommands.filter((command) => !registeredCommands.has(command));
const unregisteredLiteralCalls = [...literalFrontendCommands].filter((command) => !registeredCommands.has(command));

assert.deepEqual(unregisteredMutations, [], "every declared project mutation must be a registered backend command");
assert.deepEqual(unregisteredLiteralCalls, [], "every literal frontend invoke must resolve to a registered backend command");
assert.ok(
  app.includes("if (!isTauriRuntime())") && app.includes("throw new Error(tauriBackendUnavailableMessage)"),
  "the frontend invoke facade must fail closed without the native backend",
);
assert.ok(
  app.includes('tauriInvoke<OperatorSelectionContext>("set_operator_selection_context"')
    && app.includes("selectedControlTargetFixtures().map((fixture) => fixture.id)")
    && app.includes("visibleControls().map((control) => control.attribute)"),
  "the selected fixtures and visible feature order must be synchronized to the backend",
);
for (const command of [
  "set_operator_selection_context",
  "get_operator_selection_context",
  "set_operator_feature_fader",
]) {
  assert.ok(registeredCommands.has(command), `${command} must be registered for backend callers`);
}
assert.ok(
  (backend.match(/operator_feature_fader_command\(/g) ?? []).length >= 4,
  "Tauri, MIDI, OSC, and Remote must share operator_feature_fader_command",
);
assert.ok(midi.includes("SetSelectedFeatureFader"), "MIDI must expose the shared selected-feature action");
assert.ok(
  midi.includes("build_feedback_messages_with_operator_selection")
    && backend.includes("Some(&operator_selection)"),
  "MIDI output feedback must consume the same runtime operator selection",
);
assert.ok(osc.includes("SetSelectedFeatureFader"), "OSC must expose the shared selected-feature action");
assert.ok(
  remote.includes("SetOperatorSelection(OperatorSelectionContext)")
    && remote.includes("SetOperatorFeatureFader"),
  "Remote/AI clients must expose typed selection and feature-fader actions",
);

console.log(
  `backend operator contract ok: ${registeredCommands.size} commands, ${literalFrontendCommands.size} literal frontend calls, ${mutationCommands.length} transactional mutations`,
);
