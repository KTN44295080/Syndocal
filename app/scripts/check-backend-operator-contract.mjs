import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const srcRoot = new URL("../src/", import.meta.url);
const collectSourceFiles = async (directory) => {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const url = new URL(`${entry.name}${entry.isDirectory() ? "/" : ""}`, directory);
    if (entry.isDirectory()) return collectSourceFiles(url);
    return /\.(?:ts|tsx)$/.test(entry.name) ? [url] : [];
  }));
  return nested.flat();
};
const frontendSources = await Promise.all((await collectSourceFiles(srcRoot)).map(async (url) => ({
  url,
  source: await readFile(url, "utf8"),
})));
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
  frontendSources.flatMap(({ source }) => (
    [...source.matchAll(/\b(?:invoke|tauriInvoke)(?:<[^;()]*?>)?\(\s*"([a-z0-9_]+)"/g)]
      .map((match) => match[1])
  )),
);
const rawComponentMutationCalls = frontendSources.flatMap(({ url, source }) => {
  if (url.pathname.endsWith("/App.tsx")) return [];
  return [...source.matchAll(/\btauriInvoke(?:<[^;()]*?>)?\(\s*"([a-z0-9_]+)"/g)]
    .map((match) => match[1])
    .filter((command) => mutationCommands.includes(command))
    .map((command) => `${url.pathname.split("/src/")[1]}:${command}`);
});

const unregisteredMutations = mutationCommands.filter((command) => !registeredCommands.has(command));
const unregisteredLiteralCalls = [...literalFrontendCommands].filter((command) => !registeredCommands.has(command));

assert.deepEqual(unregisteredMutations, [], "every declared project mutation must be a registered backend command");
assert.deepEqual(unregisteredLiteralCalls, [], "every literal frontend invoke must resolve to a registered backend command");
assert.deepEqual(
  rawComponentMutationCalls,
  [],
  "feature modules must route every project mutation through the App transaction/operator facade",
);
for (const [start, end, label] of [
  ["fn import_gdtf(", "fn load_gdtf_wheel_media(", "GDTF import preview"],
  ["fn list_gdtf_fixture_cache(", "fn cache_gdtf_from_share(", "GDTF cache listing"],
  ["fn cache_gdtf_from_share(", "fn get_fixture_profile_health(", "GDTF Share cache"],
  ["fn load_verified_fixture_profile(", "fn repair_fixture_profile(", "verified profile preview"],
]) {
  const commandSource = section(backend, start, end);
  assert.equal(
    commandSource.includes("cache_fixture_profile_for_state"),
    false,
    `${label} must remain machine/session-only and never mutate project profile authority`,
  );
}
const patchCommands = section(backend, "fn patch_fixture(", "fn remove_fixture(");
assert.match(
  patchCommands,
  /project_transaction_id:[\s\S]*?expected_epoch:[\s\S]*?owner_id:[\s\S]*?project_transaction_for_owner_epoch/,
  "PATCH must validate the exact backend transaction ticket, project epoch, and renderer owner",
);
assert.match(
  patchCommands,
  /patch_fixtures_published\(candidates\)/,
  "PATCH must publish the complete fixture batch through one definitive engine acknowledgement",
);
assert.doesNotMatch(
  patchCommands,
  /EngineCommand::PatchFixture/,
  "PATCH must not fall back to per-fixture queue sends",
);
assert.match(
  app,
  /projectTransactionId:\s*transaction\.transaction_id[\s\S]*?expectedEpoch:\s*transaction\.project_epoch/,
  "the App mutation facade must pass the backend-authoritative transaction ticket",
);
const repairCommand = section(backend, "fn repair_fixture_profile(", "fn create_custom_fixture_profile(");
assert.match(
  repairCommand,
  /profile:\s*FixtureProfileSummary[\s\S]*?project_transaction_id:[\s\S]*?expected_epoch:[\s\S]*?owner_id:[\s\S]*?project_transaction_for_owner_epoch/,
  "profile Repair must carry the armed inline profile and validate the exact backend transaction owner ticket",
);
assert.match(
  repairCommand,
  /repair_fixture_profile_published/,
  "profile Repair must use a definitive engine publication acknowledgement",
);
assert.doesNotMatch(
  repairCommand,
  /EngineCommand::ReplaceFixtureProfile|std::thread::sleep/,
  "profile Repair must not use queue-and-poll publication",
);
assert.match(
  app,
  /repair_fixture_profile",\s*\{[\s\S]*?profilePath:\s*profile\.source_path[\s\S]*?profile,/,
  "profile Repair must pass the exact armed profile instead of a memory URI alone",
);
const stageFileRead = section(backend, "fn load_stage_map_preset_file(", "fn import_stage_map_preset(");
assert.doesNotMatch(
  stageFileRead,
  /State<'_, AppState>|EngineCommand|\.engine/,
  "stage-map file selection must remain a pure read outside the project transaction",
);
const stageImport = section(backend, "fn import_stage_map_preset(", "fn get_visualizer_scene(");
assert.match(
  stageImport,
  /project_transaction_id:[\s\S]*?expected_epoch:[\s\S]*?owner_id:[\s\S]*?project_transaction_for_owner_epoch/,
  "stage-map import must validate the exact backend transaction owner ticket",
);
assert.match(
  stageImport,
  /upsert_stage_map_preset_published/,
  "stage-map import must use a definitive engine publication acknowledgement",
);
const stageController = frontendSources.find(({ url }) => url.pathname.endsWith("/createStageMapController.ts"))?.source ?? "";
assert.match(
  stageController,
  /expectedProjectEpoch\s*=\s*options\.projectEpoch\(\)[\s\S]*?load_stage_map_preset_file[\s\S]*?if \(preset === null\)[\s\S]*?import_stage_map_preset[\s\S]*?__expectedProjectEpoch:\s*expectedProjectEpoch/,
  "stage-map import must open and validate the file before beginning the transactional apply",
);
assert.match(
  app,
  /requestedExpectedEpoch[\s\S]*?requestedExpectedEpoch !== currentEpoch[\s\S]*?nothing was applied/,
  "dialog-based project mutations must reject an identity change before Begin",
);
const beginTransactionMatches = [
  ...app.matchAll(/tauriInvoke<ProjectTransactionTicket>\("begin_project_transaction",\s*\{([\s\S]*?)\}\)/g),
];
assert.equal(beginTransactionMatches.length, 3, "all three Begin call sites must remain covered by the owner contract");
for (const match of beginTransactionMatches) {
  assert.match(match[1], /ownerId:\s*projectTransactionOwnerId/, "every Begin must bind the renderer owner");
}
const finishTransactionMatches = [
  ...app.matchAll(
    /tauriInvoke<ProjectHistoryMutationResult>\("(?:commit|cancel)_project_transaction",\s*\{([\s\S]*?)\}\)/g,
  ),
];
assert.equal(
  finishTransactionMatches.length,
  6,
  "all three Commit and all three Cancel call sites must remain covered by the owner contract",
);
for (const match of finishTransactionMatches) {
  assert.match(match[1], /ownerId:\s*projectTransactionOwnerId/, "every Commit/Cancel must prove the renderer owner");
}
assert.match(
  app,
  /register_project_transaction_owner[\s\S]*?ownerId:\s*projectTransactionOwnerId/,
  "every renderer must register its concrete window generation before starting project work",
);
assert.doesNotMatch(app, /register_project_transaction_owner[\s\S]{0,160}!paneWindow/, "pane owners must be registered too");
assert.match(
  backend,
  /WindowEvent::Destroyed[\s\S]*?retire_project_transaction_owner_for_window/,
  "destroying a pane must retire its owner and truthfully finalize any interrupted edit",
);
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
