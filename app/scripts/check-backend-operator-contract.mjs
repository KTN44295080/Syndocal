import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";

const normalizeNewlines = (source) => source.replace(/\r\n/g, "\n");
const app = normalizeNewlines(await readFile(new URL("../src/App.tsx", import.meta.url), "utf8"));
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
  source: normalizeNewlines(await readFile(url, "utf8")),
})));
const backend = normalizeNewlines(await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8"));
const controlPlane = normalizeNewlines(await readFile(new URL("../src-tauri/src/control_plane.rs", import.meta.url), "utf8"));
const midi = normalizeNewlines(await readFile(new URL("../../crates/io/src/midi.rs", import.meta.url), "utf8"));
const osc = normalizeNewlines(await readFile(new URL("../../crates/io/src/osc.rs", import.meta.url), "utf8"));
const remote = normalizeNewlines(await readFile(new URL("../../crates/io/src/remote_ws.rs", import.meta.url), "utf8"));

const section = (source, start, end) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing section start: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing section end: ${end}`);
  return source.slice(startIndex, endIndex);
};

const mutationSection = section(app, "const projectMutationCommands = new Set([", "const projectMutationLabel");
const mutationCommands = [...mutationSection.matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]);
const handlerSection = section(backend, "tauri::generate_handler![", ".build(tauri::generate_context!())");
const registeredCommandList = [...handlerSection.matchAll(/^\s*([a-z][a-z0-9_]+),?\s*$/gm)]
  .map((match) => match[1]);
const registeredCommands = new Set(registeredCommandList);
assert.equal(
  registeredCommands.size,
  registeredCommandList.length,
  "production generate_handler inventory contains a duplicate command",
);
const frozenAdmissionCount = Number(
  controlPlane.match(/const FROZEN_TAURI_ROUTE_ADMISSION_COUNT: usize = (\d+);/)?.[1],
);
const frozenAdmissionSha256 = controlPlane.match(
  /const FROZEN_TAURI_ROUTE_ADMISSION_SHA256: &str =\s*"([a-f0-9]{64})";/,
)?.[1];
const currentAdmissionSha256 = createHash("sha256")
  .update([...registeredCommandList].sort().join("\n"), "utf8")
  .digest("hex");
assert.equal(
  frozenAdmissionCount,
  registeredCommands.size,
  "D3 frozen Tauri admission count differs from the production generate_handler inventory",
);
assert.equal(
  frozenAdmissionSha256,
  currentAdmissionSha256,
  "D3 frozen Tauri admission fingerprint differs from the production generate_handler inventory",
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
const patchCommands = section(backend, "fn patch_fixtures(", "fn remove_fixture(");
assert.match(
  patchCommands,
  /project_transaction_id:[\s\S]*?expected_epoch:[\s\S]*?owner_id:[\s\S]*?project_transaction_for_owner_epoch/,
  "PATCH must validate the exact backend transaction ticket, project epoch, and renderer owner",
);
assert.match(
  patchCommands,
  /patch_fixtures_(?:allocated_)?published/,
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
  /const invokeFixtureProfileRepair = async \([\s\S]*?repair_fixture_profile",\s*\{[\s\S]*?fixtureId,[\s\S]*?profilePath:\s*armedProfile\.source_path,[\s\S]*?modeName,[\s\S]*?profile:\s*armedProfile,/,
  "profile Repair must pass the exact armed profile through the shared typed facade instead of a memory URI alone",
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
const beginRecoveryHelper = section(
  app,
  "const beginProjectTransactionWithRecovery = async (",
  "const queryProjectTransactionTerminal = async (",
);
assert.match(
  beginRecoveryHelper,
  /tauriInvoke<ProjectTransactionTicket>\("begin_project_transaction",\s*beginArgs\)/,
  "the shared Begin recovery helper must invoke the canonical backend command with its exact arguments",
);
const beginTransactionMatches = [...app.matchAll(/\bbeginProjectTransactionWithRecovery\(/g)];
assert.equal(beginTransactionMatches.length, 1, "the sole central mutation workflow must use the shared recovery helper");
const transactionIdentityMatches = [
  ...app.matchAll(/const transactionIdentity = \{([\s\S]*?)\n\s*\};/g),
];
assert.equal(transactionIdentityMatches.length, 1, "the sole central mutation workflow must define an exact identity");
for (const match of transactionIdentityMatches) {
  assert.match(match[1], /ownerId:\s*projectTransactionOwnerId/, "every transaction identity must bind the renderer owner");
}
const cancelRecoveryHelper = section(
  app,
  "const cancelProjectTransactionWithRecovery = async (",
  "const commitProjectTransactionWithRecovery = async (",
);
assert.match(cancelRecoveryHelper, /ownerId:\s*identity\.ownerId/);
assert.match(cancelRecoveryHelper, /"cancel_project_transaction",\s*cancelArgs/);
assert.equal(
  [...app.matchAll(/\bcancelProjectTransactionWithRecovery\(/g)].length,
  1,
  "the central Cancel workflow must use the owner-bound recovery helper",
);
assert.equal(
  [...app.matchAll(/\bcommitProjectTransactionWithRecovery\(/g)].length,
  0,
  "no specialized workflow may bypass the central inline Commit recovery path",
);
const genericCommit = section(app, "transaction = await beginProjectTransactionWithRecovery", "const listen = <T,>");
assert.match(
  genericCommit,
  /"commit_project_transaction"[\s\S]*?ownerId:\s*projectTransactionOwnerId/,
  "the generic Commit workflow must bind the renderer owner",
);
assert.match(
  genericCommit,
  /catch \(commitError\)[\s\S]*?queryProjectTransactionTerminal\(transactionIdentity\)[\s\S]*?terminal\.status !== "committed"[\s\S]*?mutation = terminal\.mutation/s,
  "the generic Commit workflow must recover only the exact committed terminal mutation after reply loss",
);
assert.match(
  app,
  /register_project_transaction_owner[\s\S]*?ownerId:\s*projectTransactionOwnerId/,
  "every renderer must register its concrete window generation before starting project work",
);
assert.doesNotMatch(app, /register_project_transaction_owner[\s\S]{0,160}!paneWindow/, "pane owners must be registered too");
const genericInvoke = section(app, "const invoke = async <T,>", "const listen = <T,>");
assert.match(
  genericInvoke,
  /await awaitProjectTransactionOwnerRegistrationBarrier\(\);/,
  "the generic native facade must fail closed until its project owner is registered",
);
assert.ok(
  genericInvoke.indexOf("await awaitProjectTransactionOwnerRegistrationBarrier();")
    < genericInvoke.indexOf("const rendererTicketedMutation"),
  "owner registration must precede every generic transaction/mapping dispatch decision",
);
assert.doesNotMatch(
  genericInvoke,
  /register_project_transaction_owner/,
  "the registration primitive must never recurse through the generic invoke facade",
);
const ownerRegistration = section(
  app,
  "const projectTransactionOwnerRegistrationStatusKey",
  "const refreshFixtureGroups",
);
assert.match(
  ownerRegistration,
  /const projectTransactionOwnerRegistrationStatusKey = "project-owner-registration"/,
  "owner registration failures need a stable, dedicated status key",
);
assert.match(
  ownerRegistration,
  /if \(projectTransactionOwnerRegistration\) return projectTransactionOwnerRegistration/,
  "per-WebView owner registration must be single-flight and idempotent",
);
assert.match(
  ownerRegistration,
  /tauriInvoke<[^>]+>\("register_project_transaction_owner",\s*\{\s*ownerId:\s*projectTransactionOwnerId/s,
  "owner registration must use the raw Tauri primitive to avoid barrier recursion",
);
assert.match(
  ownerRegistration,
  /if \(projectTransactionOwnerRegistration === attempt\) \{\s*projectTransactionOwnerRegistration = null;/s,
  "a failed owner registration must clear only its own in-flight promise",
);
assert.match(
  ownerRegistration,
  /projectTransactionOwnerRegistrationHasFailure && !projectTransactionOwnerRegistrationRetryArmed[\s\S]*?return Promise\.reject\(projectTransactionOwnerRegistrationFailure\);/s,
  "background callers must receive the cached owner-registration failure without issuing retry IPC",
);
assert.match(
  ownerRegistration,
  /projectTransactionOwnerRegistration\) \{[\s\S]*?projectTransactionOwnerRegistration = null;[\s\S]*?retryArmedForFailedAttempt = projectTransactionOwnerRegistrationRetryRequestedDuringFlight;[\s\S]*?projectTransactionOwnerRegistrationHasFailure = true;[\s\S]*?projectTransactionOwnerRegistrationRetryArmed = retryArmedForFailedAttempt;/s,
  "a failed registration must cache failure while preserving one explicit retry requested during the pending attempt",
);
assert.match(
  ownerRegistration,
  /if \(projectTransactionOwnerRegistration\) \{[\s\S]*?projectTransactionOwnerRegistrationRetryRequestedDuringFlight = true;[\s\S]*?else if \(projectTransactionOwnerRegistrationHasFailure\) \{[\s\S]*?projectTransactionOwnerRegistrationRetryArmed = true;/s,
  "an explicit gesture/event must retain a retry request received before a pending registration rejects",
);
assert.match(
  ownerRegistration,
  /event\.isTrusted\) armProjectTransactionOwnerRegistrationRetry\(\);[\s\S]*?window\.addEventListener\("pointerdown"[\s\S]*?window\.addEventListener\("keydown"/s,
  "only trusted local input may arm a failed owner registration retry",
);
const registrationSuccess = section(
  ownerRegistration,
  "}).then((recovered) => {",
  "}).catch((error) => {",
);
assert.ok(
  registrationSuccess.indexOf("if (projectTransactionOwnerRegistrationDisposed)")
    < registrationSuccess.indexOf("clearProjectTransactionOwnerRegistrationFailure();"),
  "a late registration success must reject before status/history side effects after disposal",
);
assert.ok(
  registrationSuccess.indexOf("throw new DOMException")
    < registrationSuccess.indexOf("window.dispatchEvent("),
  "a disposed registration must not publish recovered history before aborting",
);
assert.match(
  ownerRegistration,
  /void ensureProjectTransactionOwnerRegistration\(\)\.catch\(\(\) => undefined\);/,
  "each main/pane App must proactively start its own owner registration",
);
assert.match(
  ownerRegistration,
  /projectTransactionOwnerRegistrationBarrier\?\.identity === projectTransactionOwnerRegistrationIdentity[\s\S]*?projectTransactionOwnerRegistrationBarrier = null/s,
  "cleanup must not detach a newer App registration barrier after reload/remount",
);
assert.match(
  ownerRegistration,
  /current\.key === projectTransactionOwnerRegistrationStatusKey[\s\S]*?key !== projectTransactionOwnerRegistrationStatusKey/s,
  "unkeyed background failures must not erase the actionable owner-registration failure",
);
assert.match(
  ownerRegistration,
  /result\.kind === "clear"[\s\S]*?current\.key === projectTransactionOwnerRegistrationStatusKey[\s\S]*?appStatusFromMessage\("Ready"\)/s,
  "post-commit refresh must not unconditionally clear a live owner-registration failure",
);
const registeredOwnerCommand = section(
  app,
  "const invokeRegisteredOwnerCommand = async <T,>",
  "onCleanup(() => {\n    projectTransactionOwnerRegistrationDisposed",
);
assert.ok(
  registeredOwnerCommand.indexOf("await awaitProjectTransactionOwnerRegistrationBarrier();")
    < registeredOwnerCommand.indexOf("return tauriInvoke<T>(command, args);"),
  "direct owner-bound commands must await registration before raw dispatch",
);
for (const command of [
  "lock_project_operator_session",
  "unlock_project_operator_session",
  "set_operator_selection_context",
]) {
  assert.doesNotMatch(
    app,
    new RegExp(`tauriInvoke(?:<[^>]+>)?\\(\\s*["']${command}["']`),
    `${command} must not bypass the registered-owner command wrapper`,
  );
  assert.match(
    app,
    new RegExp(`invokeRegisteredOwnerCommand(?:<[^>]+>)?\\(\\s*["']${command}["']`),
    `${command} must route through the registered-owner command wrapper`,
  );
}
const selectionSync = section(app, "let syncedOperatorSelectionKey", "const showDimmerPanel");
assert.match(
  selectionSync,
  /projectTransactionOwnerRegistrationRevision\(\);[\s\S]*?if \(key === syncedOperatorSelectionKey\) return;/s,
  "a recovered owner registration must revisit the current unsynced selection",
);
assert.match(
  selectionSync,
  /if \(requestedOperatorSelectionKey === key\) \{\s*syncedOperatorSelectionKey = key;/s,
  "a stale selection reply must not mark an older selection context as current",
);
assert.match(
  selectionSync,
  /if \(operatorSelectionSyncInFlight[\s\S]*?return;[\s\S]*?operatorSelectionSyncInFlight = true/s,
  "selection synchronization must serialize in-flight owner-bound calls",
);
assert.match(
  selectionSync,
  /failedOperatorSelectionKey = key;[\s\S]*?failedOperatorSelectionKey !== null[\s\S]*?requestedOperatorSelectionKey !== failedOperatorSelectionKey[\s\S]*?void syncOperatorSelectionContext\(\);/s,
  "a newer selection arriving during a failed in-flight dispatch must re-drive only the latest desired context",
);
const programAudioSync = section(app, "const syncProgramAudioHandoffConfig = async", "const assignVideoDeck");
assert.match(
  programAudioSync,
  /projectTransactionOwnerRegistrationRevision\(\);/,
  "pending program-audio synchronization must retry after another call restores owner registration",
);
assert.match(
  programAudioSync,
  /current\.key === projectTransactionOwnerRegistrationStatusKey[\s\S]*?current[\s\S]*?: appStatusFromMessage\(String\(error\), "program-audio-handoff"\)/s,
  "program-audio failures must not overwrite an active owner-registration status",
);
assert.match(
  programAudioSync,
  /failedProgramAudioHandoffSignature = signature;[\s\S]*?failedProgramAudioHandoffSignature !== null[\s\S]*?desiredProgramAudioHandoffSignature !== failedProgramAudioHandoffSignature[\s\S]*?void syncProgramAudioHandoffConfig\(\);/s,
  "a newer program-audio configuration arriving during a failed in-flight dispatch must re-drive only the latest desired config",
);
const startupOpenOwnership = section(app, "// Startup/CLI-open ownership belongs", "// Drag/drop is WebView-local");
assert.match(
  startupOpenOwnership,
  /if \(!isTauriRuntime\(\) \|\| paneWindow\) \{\s*return;/s,
  "only the main App may own startup and app-wide queued project opens",
);
const listenerIndex = startupOpenOwnership.indexOf('listen<string[]>("syndocal://open-project"');
const barrierIndex = startupOpenOwnership.indexOf("await awaitProjectTransactionOwnerRegistrationBarrier();");
const startupIndex = startupOpenOwnership.indexOf("await loadStartupProject();");
const readyIndex = startupOpenOwnership.indexOf("queuedOpenProjectDrainReady = true;");
const drainIndex = startupOpenOwnership.indexOf("await loadQueuedOpenProjects();");
assert.ok(
  listenerIndex >= 0 && listenerIndex < barrierIndex && barrierIndex < startupIndex && startupIndex < readyIndex && readyIndex < drainIndex,
  "main queued-open listener must install first, then register, load startup, mark ready, and serially drain",
);
assert.match(
  startupOpenOwnership,
  /syndocal:\/\/open-project"[\s\S]*?armProjectTransactionOwnerRegistrationRetry\(\);[\s\S]*?requestQueuedOpenProjects\(\);[\s\S]*?void runMainProjectOpenBootstrap\(\);/s,
  "a new single-instance open must arm one retry, request a serialized drain, and resume the main bootstrap",
);
assert.doesNotMatch(
  startupOpenOwnership,
  /loadProjectPath\(|void\s+(?:loadStartupProject|loadQueuedOpenProjects)\s*\(/,
  "the app-wide event must not directly load payload paths or restore parallel startup/drain calls",
);
const queuedOpenDrain = section(app, "let queuedOpenProjectDrainReady = false;", "const loadPhase1SampleProject");
assert.match(
  queuedOpenDrain,
  /queuedOpenProjectDrainRequested = true;\s*if \(queuedOpenProjectDrainReady\) void loadQueuedOpenProjects\(\);/s,
  "pre-ready open events must only set a drain request and cannot destructively take paths",
);
assert.match(
  queuedOpenDrain,
  /if \(queuedOpenProjectDrainInFlight\) \{\s*queuedOpenProjectDrainRequested = true;\s*return;/s,
  "queued opens must remain single-flight while a drain is active",
);
assert.match(
  queuedOpenDrain,
  /queuedOpenProjectDrainInFlight = false;[\s\S]*?if \(queuedOpenProjectDrainReady && queuedOpenProjectDrainRequested\) \{\s*void loadQueuedOpenProjects\(\);/s,
  "an event received during a failed queue take must schedule one follow-up serialized drain",
);
assert.equal(
  [...queuedOpenDrain.matchAll(/invoke<string\[\]>\("take_open_project_paths"\)/g)].length,
  1,
  "only the serialized queue drain may destructively take queued project paths",
);
assert.match(
  startupOpenOwnership,
  /projectTransactionOwnerRegistrationRevision\(\);[\s\S]*?mainProjectOpenBootstrapReady && !queuedOpenProjectDrainReady[\s\S]*?void runMainProjectOpenBootstrap\(\);/s,
  "a successful explicit registration retry must resume the incomplete main bootstrap exactly through its single-flight helper",
);
assert.match(
  startupOpenOwnership,
  /if \(mainProjectOpenBootstrapInFlight\) return mainProjectOpenBootstrapInFlight;[\s\S]*?if \(!startupProjectLoaded\)[\s\S]*?await loadStartupProject\(\);[\s\S]*?startupProjectLoaded = true;[\s\S]*?queuedOpenProjectDrainReady = true;[\s\S]*?await loadQueuedOpenProjects\(\);/s,
  "main bootstrap must stay single-flight and complete registration, startup, then queue drain in order",
);
assert.match(
  startupOpenOwnership,
  /mainProjectOpenBootstrapInFlight = null;[\s\S]*?projectTransactionOwnerRegistrationRetryArmed[\s\S]*?void runMainProjectOpenBootstrap\(\);/s,
  "a retained explicit retry arm must resume bootstrap only after its failed single-flight slot releases",
);
assert.match(
  startupOpenOwnership,
  /try \{[\s\S]*?await listen<string\[\]>\("syndocal:\/\/open-project"[\s\S]*?\} catch \(error\) \{[\s\S]*?Project open listener unavailable:[\s\S]*?\}\s*if \(disposed\) return;\s*mainProjectOpenBootstrapReady = true;[\s\S]*?await runMainProjectOpenBootstrap\(\);/s,
  "listener setup failure must remain isolated from main registration/startup/queue bootstrap",
);
const runOwnerRegistrationRearmFixture = async () => {
  // Mirrors the source's retry state machine while the source assertions above
  // bind this fixture to its concrete names and ordering. A poll storm after a
  // rejection is fail-closed; one trusted/event arm permits one same-owner retry.
  const ownerId = "stable-owner-uuid";
  let rawRegisterCalls = 0;
  const rawRegisterOwners = [];
  const pending = [];
  let registration = null;
  let failure = null;
  let hasFailure = false;
  let retryArmed = false;
  let retryRequestedDuringFlight = false;
  const rawRegister = () => new Promise((resolve, reject) => {
    rawRegisterCalls += 1;
    rawRegisterOwners.push(ownerId);
    pending.push({ resolve, reject });
  });
  const ensure = () => {
    if (registration) return registration;
    if (hasFailure && !retryArmed) return Promise.reject(failure);
    if (hasFailure) {
      hasFailure = false;
      failure = null;
      retryArmed = false;
    }
    let attempt;
    attempt = rawRegister().then(() => {
      hasFailure = false;
      failure = null;
      retryArmed = false;
      retryRequestedDuringFlight = false;
    }).catch((error) => {
      if (registration === attempt) registration = null;
      const retryArmedForFailedAttempt = retryRequestedDuringFlight;
      retryRequestedDuringFlight = false;
      hasFailure = true;
      failure = error;
      retryArmed = retryArmedForFailedAttempt;
      throw error;
    });
    registration = attempt;
    return attempt;
  };
  const armExplicitRetry = () => {
    if (registration) retryRequestedDuringFlight = true;
    else if (hasFailure) retryArmed = true;
  };
  const first = ensure();
  pending.shift().reject(new Error("first registration rejected"));
  await assert.rejects(first, /first registration rejected/);
  await Promise.all(Array.from({ length: 16 }, () => ensure().catch(() => undefined)));
  assert.equal(rawRegisterCalls, 1, "poll callers must not retry a cached owner-registration failure");
  armExplicitRetry();
  const retryA = ensure();
  const retryB = ensure();
  assert.strictEqual(retryA, retryB, "the explicit retry must remain single-flight");
  pending.shift().resolve();
  await retryA;
  assert.equal(rawRegisterCalls, 2, "one explicit retry arm must issue exactly one more registration IPC");
  assert.deepEqual(rawRegisterOwners, [ownerId, ownerId], "owner UUID must remain stable across a retry");
};
await runOwnerRegistrationRearmFixture();
const runPendingOpenRetryArmFixture = () => {
  const ownerId = "stable-owner-uuid";
  let rawRegisterCalls = 1;
  const rawRegisterOwners = [ownerId];
  let registrationInFlight = true;
  let retryRequestedDuringFlight = false;
  let retryArmed = false;
  const armOpenEvent = () => {
    if (registrationInFlight) retryRequestedDuringFlight = true;
  };
  armOpenEvent();
  // The first IPC now rejects; retain the open event's arm rather than
  // requiring a second external event to drain the backend-queued path.
  registrationInFlight = false;
  retryArmed = retryRequestedDuringFlight;
  retryRequestedDuringFlight = false;
  assert.equal(retryArmed, true, "an open event during pending registration must retain one retry arm after rejection");
  if (retryArmed) {
    retryArmed = false;
    rawRegisterCalls += 1;
    rawRegisterOwners.push(ownerId);
  }
  assert.equal(rawRegisterCalls, 2, "the retained pending-open arm must issue one same-owner retry");
  assert.deepEqual(rawRegisterOwners, [ownerId, ownerId], "pending-open retry must not rotate the owner UUID");
};
runPendingOpenRetryArmFixture();
const runDisposedLateRegistrationFixture = async () => {
  let resolveRegistration;
  let statusWrites = 0;
  let historyDispatches = 0;
  let waitingCommandContinuations = 0;
  let disposed = false;
  const rawRegistration = new Promise((resolve) => { resolveRegistration = resolve; });
  const registration = rawRegistration.then((recovered) => {
    if (disposed) throw new DOMException("registration abandoned", "AbortError");
    statusWrites += 1;
    if (recovered) historyDispatches += 1;
    waitingCommandContinuations += 1;
  });
  disposed = true;
  resolveRegistration({ recovered: true });
  await assert.rejects(registration, (error) => error?.name === "AbortError");
  assert.equal(statusWrites, 0, "a disposed late registration resolve must not write status");
  assert.equal(historyDispatches, 0, "a disposed late registration resolve must not dispatch recovered history");
  assert.equal(waitingCommandContinuations, 0, "a disposed late registration resolve must not continue waiting commands");
};
await runDisposedLateRegistrationFixture();
const runRecoveredDesiredSyncFixture = () => {
  let registrationReady = false;
  let registrationRevision = 0;
  let selectionSynced = false;
  let audioApplied = false;
  let selectionDispatches = 0;
  let audioDispatches = 0;
  const replayDesiredAutomaticSync = (observedRevision) => {
    if (observedRevision !== registrationRevision || !registrationReady) return;
    if (!selectionSynced) {
      selectionDispatches += 1;
      selectionSynced = true;
    }
    if (!audioApplied) {
      audioDispatches += 1;
      audioApplied = true;
    }
  };
  // Initial owner registration fails: desired state remains pending and no
  // poll-driven retry/automatic native dispatch is permitted.
  replayDesiredAutomaticSync(registrationRevision);
  assert.equal(selectionDispatches, 0);
  assert.equal(audioDispatches, 0);
  // A different, explicitly armed owner-bound/generic call succeeds. Both
  // effects observe the revision and resend their still-pending desired state.
  registrationReady = true;
  registrationRevision += 1;
  replayDesiredAutomaticSync(registrationRevision);
  assert.equal(selectionDispatches, 1, "registration recovery must replay pending selection exactly once");
  assert.equal(audioDispatches, 1, "registration recovery must replay pending program audio exactly once");
};
runRecoveredDesiredSyncFixture();
const runLatestDesiredAfterFailureFixture = async (label) => {
  let desired = "initial";
  let applied = null;
  let inFlight = false;
  const pending = [];
  const dispatches = [];
  const dispatch = (value) => new Promise((resolve, reject) => {
    dispatches.push(value);
    pending.push({ resolve, reject });
  });
  const sync = async () => {
    if (inFlight) return;
    inFlight = true;
    let failed = false;
    let failedDesired = null;
    try {
      while (desired !== applied) {
        const attempted = desired;
        try {
          await dispatch(attempted);
        } catch (error) {
          failedDesired = attempted;
          throw error;
        }
        applied = attempted;
      }
    } catch {
      failed = true;
    } finally {
      inFlight = false;
      if ((!failed || (failedDesired !== null && desired !== failedDesired)) && desired !== applied) {
        void sync();
      }
    }
  };
  const first = sync();
  desired = "latest";
  pending.shift().reject(new Error(`${label} initial dispatch rejected`));
  await first;
  await new Promise((resolve) => queueMicrotask(resolve));
  assert.deepEqual(dispatches, ["initial", "latest"], `${label} must re-drive the newer desired value after the older in-flight dispatch fails`);
  pending.shift().resolve();
  await new Promise((resolve) => queueMicrotask(resolve));
  assert.equal(applied, "latest", `${label} must apply only the latest desired value after recovery`);
};
await runLatestDesiredAfterFailureFixture("selection");
await runLatestDesiredAfterFailureFixture("program audio");
const runQueueTakeFailureEventFixture = async () => {
  let ready = true;
  let inFlight = false;
  let requested = false;
  let takeCalls = 0;
  const pending = [];
  const take = () => new Promise((resolve, reject) => {
    takeCalls += 1;
    pending.push({ resolve, reject });
  });
  const drain = async () => {
    if (inFlight) {
      requested = true;
      return;
    }
    inFlight = true;
    try {
      requested = false;
      await take();
    } catch {
      // Source records the truthful error; this fixture checks scheduling.
    } finally {
      inFlight = false;
      if (ready && requested) void drain();
    }
  };
  const first = drain();
  requested = true; // event received while the first destructive take is pending
  pending.shift().reject(new Error("take rejected"));
  await first;
  await new Promise((resolve) => queueMicrotask(resolve));
  assert.equal(takeCalls, 2, "an event during a failed queue take must schedule one replacement drain");
  pending.shift().resolve();
};
await runQueueTakeFailureEventFixture();
const runListenerFailureBootstrapFixture = async () => {
  let registerCalls = 0;
  let startupCalls = 0;
  let takeCalls = 0;
  const listen = async () => {
    throw new Error("listener unavailable");
  };
  const register = async () => { registerCalls += 1; };
  const loadStartup = async () => { startupCalls += 1; };
  const takeQueued = async () => { takeCalls += 1; };
  try {
    await listen();
  } catch {
    // Event listener availability is non-authoritative for startup state.
  }
  await register();
  await loadStartup();
  await takeQueued();
  assert.equal(registerCalls, 1, "listener failure must not skip initial owner registration");
  assert.equal(startupCalls, 1, "listener failure must not skip the initial startup project");
  assert.equal(takeCalls, 1, "listener failure must not skip the pre-existing open queue drain");
};
await runListenerFailureBootstrapFixture();
const destroyedWindowRetirement = section(
  backend,
  "fn handle_destroyed_window_authority_retirement(",
  "fn transition_project_transaction_window_owner<T>(",
);
assert.match(
  destroyedWindowRetirement,
  /retire_project_transaction_owner_for_window(?:_incarnation)?/,
  "the shared destroyed-window boundary must retire its exact project owner",
);
assert.match(
  backend,
  /(?:tauri::)?WindowEvent::Destroyed[\s\S]*?handle_destroyed_window_authority_retirement/,
  "destroying a pane must use the shared authority retirement boundary",
);
assert.ok(
  app.includes("if (!isTauriRuntime())") && app.includes("throw new Error(tauriBackendUnavailableMessage)"),
  "the frontend invoke facade must fail closed without the native backend",
);
assert.ok(
  app.includes('invokeRegisteredOwnerCommand<OperatorSelectionContext>("set_operator_selection_context"')
    && app.includes("selectedControlTargetFixtures().map((fixture) => fixture.id)")
    && app.includes("visibleControls().map((control) => control.attribute)"),
  "the selected fixtures and visible feature order must be synchronized through the registered owner boundary",
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
