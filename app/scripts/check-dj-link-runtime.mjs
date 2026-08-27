import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import {
  acknowledgeProjectAuthorityPersist,
  beginProjectAuthorityRequest,
  createProjectAuthoritySyncState,
  noteLocalProjectAuthorityEdit,
  projectAuthorityHasDirtyMappings,
  projectAuthorityShouldRetryPersist,
} from "../src/projectAuthority.ts";
import {
  availableRemoteControlStatus,
  clearedDjLinkSecret,
  projectRemoteControlStatusPoll,
  unavailableRemoteControlStatus,
} from "../src/djLinkUiState.ts";
import { retainDjTimelineOptions } from "../src/djTimelineOptions.ts";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const [app, types, panel, chrome, invokes, manifest, localization, main, persistenceRuntime, styles] = await Promise.all([
  read("src/App.tsx"),
  read("src/types.ts"),
  read("src/components/RemoteControlPanel.tsx"),
  read("src/components/WorkspaceChrome.tsx"),
  read("src/tauriInvokeCommands.ts"),
  read("src/tauri-invoke-manifest.json"),
  read("src/uiLocalization.ts"),
  read("src-tauri/src/main.rs"),
  read("src-tauri/src/dj_link_persistence_runtime.rs"),
  read("src/styles.css"),
]);

for (const source of [app, types]) {
  assert.match(source, /DjTrackTriggerMapping/);
  assert.match(source, /dj_track_triggers|djTrackTriggers/);
}
assert.match(types, /DjTrackSelector/);
assert.match(panel, /DjTrackTriggerMapping/);
assert.match(panel, /djTrackTriggers/);
assert.match(types, /interface DjLinkRuntimeStatus/);
assert.match(types, /interface DjLinkMachineStatus/);
assert.match(types, /credentialCleanupPending: boolean/);
assert.match(types, /interface DjLinkWiredCandidate/);
assert.match(types, /web_remote_enabled: boolean/);
assert.match(types, /dj_link_enabled: boolean/);
assert.match(types, /contentId/);
assert.match(types, /timelineId/);
assert.match(types, /outcome\?: string \| null/);
assert.match(types, /ownerDeck\?: string \| null/);
assert.doesNotMatch(types, /\bmaster\s*:/);
assert.doesNotMatch(types, /masterDeck/);
assert.match(types, /interface RemoteControlConfig[\s\S]*dj_link_enabled/);
assert.match(panel, /data-io-disclosure="dj-link"/);
assert.match(
  panel,
  /<div class="ioDisclosureStack">\s*<details class="ioDisclosure" data-io-disclosure="web-remote" data-io-default-surface="remote">/,
  "Web Remote must be a peer connection disclosure, not a fixed top section",
);
assert.doesNotMatch(
  panel,
  /<section class="remoteServerDesk ioConnectionDesk" data-io-default-surface="remote">/,
  "Web Remote must not retain a standalone default-surface section",
);
assert.match(
  styles,
  /\.layoutSetup\.setupMode-io \.remoteControl \{\s*grid-template-rows: minmax\(0, 1fr\);\s*overflow: hidden;/,
  "Remote connection disclosures must retain the contained one-row layout",
);
assert.match(
  styles,
  /\.layoutSetup\.setupMode-io \.remoteControl > \.ioDisclosureStack \{\s*height: 100%;\s*min-height: 0;\s*overflow: hidden auto;/,
  "Remote disclosure stack must own the scrollport",
);
assert.doesNotMatch(styles, /grid-template-rows: 80px minmax\(0, 1fr\);/);
assert.match(panel, /data-io-control="dj-link-wired-binding"/);
assert.match(panel, /djLinkWiredCandidateCount/);
assert.match(panel, /data-io-status="dj-link-wired-discovery"/);
assert.match(panel, /data-dj-link-wired-candidate-count/);
const refreshButton = panel.match(
  /<button\s+type="button"\s+data-io-control="dj-link-refresh-wired-candidates"[\s\S]*?<\/button>/,
)?.[0];
assert.ok(refreshButton, "wired discovery refresh button is present");
assert.match(
  refreshButton,
  /disabled=\{props\.djLinkTokenOperationBusy \|\| props\.djLinkWiredRefreshBusy \|\| !machineStatusKnown\(\)\}/,
  "wired discovery refresh must stay available while the read-only listener is running",
);
assert.match(refreshButton, /aria-busy=\{props\.djLinkWiredRefreshBusy\}/);
assert.match(refreshButton, /props\.djLinkWiredRefreshBusy \? "Refreshing…" : "Refresh wired bindings"/);
assert.match(panel, /data-io-status="dj-link-wired-refresh"[\s\S]*Refreshing…/);
assert.match(panel, /data-io-status="dj-link-wired-refresh-error"[\s\S]*role="status"[\s\S]*aria-live="polite"/);
assert.match(panel, /data-dj-link-wired-candidate-count[\s\S]*role="status"[\s\S]*aria-live="polite"/);
assert.doesNotMatch(
  refreshButton,
  /listenerRunning/,
  "wired discovery refresh must not be blocked by the listener state",
);
assert.match(panel, /data-io-control="dj-link-arm"/);
assert.match(panel, /data-io-control="dj-link-disarm"/);
assert.match(panel, /djLinkMachineBlockReasonText/);
assert.match(panel, /credentialCleanupPending/);
assert.doesNotMatch(panel, /DJ Link blocked:\s*\{reason\(\)\}/);
assert.match(panel, /Use Current Track/);
assert.match(panel, /once_per_play_session/);
assert.match(panel, /status\.available !== false/);
assert.match(panel, /Owner deck \/ playing/);
assert.doesNotMatch(panel, /Legacy Master diagnostic/);
assert.match(panel, /disabled=\{!linkStatus\(\)\?\.trackContentId/);
for (const label of ["Available", "Enabled", "Outcome", "Peer", "Generation", "Heartbeat", "Last event"]) {
  assert.match(panel, new RegExp(`>${label}\\s`), `${label} diagnostic is not rendered`);
}
assert.doesNotMatch(panel, /MIDI|Filter CC|Stop MIDI|hotkey/i);
assert.match(app, /list_dj_link_wired_candidates/);
assert.match(app, /get_dj_link_machine_status/);
assert.match(app, /arm_dj_link_machine/);
assert.match(app, /disarm_dj_link_machine/);
assert.match(app, /rotate_dj_link_token/);
assert.match(app, /djLinkTokenClearTimer/);
assert.match(app, /djLinkBindIp\(\)[\s\S]*remoteBindIp\(\)/);
assert.match(app, /const genericRemoteRunning/);
assert.match(app, /const djListenerRunning/);
assert.match(app, /remoteStatusHydrated/);
assert.match(app, /setRemoteStatusHydrated\(true\)/);
assert.match(app, /setRemoteStatusHydrated\(false\)/);
assert.match(app, /void refreshRemoteControlStatus\(\);\s*\}, 1_000\)/);
assert.match(app, /setDjLinkMachineStatus\(\{[\s\S]*blockReason: "machine_status_unavailable"/);
assert.match(app, /djLinkCandidateRequestGeneration \+= 1;\s*setDjLinkWiredCandidates\(\[\]\);\s*(?:setDjLinkWiredCandidateCount\(null\);\s*)?setDjLinkSelectedBinding\(""\)/);
assert.match(app, /const refreshDjLinkMachineStatusAndCandidates = async \(\) => \{[\s\S]*?if \(!status \|\| status\.blockReason === "machine_status_unavailable"\) return \[\];[\s\S]*?return refreshDjLinkWiredCandidates\(\);/);
assert.match(
  app,
  /const djLinkMachineStatusRetryTimer = typeof window !== "undefined"\s*\? window\.setInterval\(\(\) => \{\s*if \(!isTauriRuntime\(\)\) return;\s*if \(djLinkMachineStatus\(\)\.blockReason === "machine_status_unavailable"\) \{\s*void refreshDjLinkMachineStatusAndCandidates\(\);\s*\}\s*\}, 1_000\)\s*:\s*null/,
  "machine-status retry must be browser-safe, Tauri-only, bounded, and authority-gated",
);
assert.match(app, /window\.clearInterval\(djLinkMachineStatusRetryTimer\)/);
assert.doesNotMatch(app, /refreshDjLinkMachineStatus\(\)\.then\(\(\) => void refreshDjLinkWiredCandidates\(\)\)/);
assert.match(app, /if \(remoteRunning\(\)\) \{\s*await stopRemoteControl\(\);\s*await refreshRemoteControlStatus\(\);/);
assert.match(app, /if \(!remoteStatusHydrated\(\)\) \{\s*setMessage\("Checking the shared Remote listener before starting Web Remote\."\)/);
assert.match(app, /remotePort\(\) !== port/);
assert.match(panel, /listenerStatusHydrated/);
assert.match(panel, /hasDjLinkDisarmWork/);
assert.match(panel, /disabled=\{props\.listenerRunning \|\| props\.djLinkMachineStatus\.autoStartArmed\}/);
assert.match(app, /Stop the shared Remote listener before rotating the DJ Link token/);
assert.match(app, /if \(isTauriRuntime\(\)\) void refreshRemoteControlStatus\(\)/);
assert.equal(
  [...app.matchAll(/createEffect\(\(\) => \{\s*\/\/ A DJ-only auto-start[\s\S]*?if \(isTauriRuntime\(\)\) void refreshRemoteControlStatus\(\);\s*\}\);/g)].length,
  1,
  "DJ-only startup must have exactly one mount-effect listener-status poll",
);
assert.doesNotMatch(app, /candidates\[0\]\s*\?\s*djLinkBindingKey/);
assert.match(app, /let djLinkCandidateRequestGeneration = 0/);
assert.match(app, /const \[djLinkWiredRefreshBusy, setDjLinkWiredRefreshBusy\] = createSignal\(false\)/);
assert.match(app, /const \[djLinkWiredRefreshError, setDjLinkWiredRefreshError\] = createSignal<string \| null>\(null\)/);
assert.match(app, /let djLinkWiredRefreshGeneration = 0/);
assert.match(
  app,
  /const refreshDjLinkWiredCandidates = async \(\) => \{\s*if \(djLinkWiredRefreshBusy\(\)\) return \[\];\s*const requestGeneration = \+\+djLinkCandidateRequestGeneration;\s*const refreshGeneration = \+\+djLinkWiredRefreshGeneration;\s*setDjLinkWiredRefreshBusy\(true\);\s*setDjLinkWiredRefreshError\(null\);\s*try \{\s*setDjLinkWiredCandidates\(\[\]\);\s*setDjLinkSelectedBinding\(""\);\s*setDjLinkWiredCandidateCount\(null\);/,
  "wired refresh must clear stale candidates and selection before awaiting the read-only query",
);
assert.match(
  app,
  /finally \{\s*if \(refreshGeneration === djLinkWiredRefreshGeneration\) setDjLinkWiredRefreshBusy\(false\);\s*\}/,
  "a stale wired refresh must not clear a newer request's busy state",
);
assert.match(app, /setDjLinkWiredRefreshError\(null\)/);
assert.match(app, /if \(requestGeneration !== djLinkCandidateRequestGeneration\) return \[\];[\s\S]*?setDjLinkWiredRefreshError\(failureMessage\)/);
assert.match(app, /djLinkWiredRefreshBusy=\{djLinkWiredRefreshBusy\(\)\}/);
assert.match(app, /djLinkWiredRefreshError=\{djLinkWiredRefreshError\(\)\}/);
assert.match(app, /setDjLinkWiredCandidateCount\(candidates\.length\)/);
assert.match(app, /setDjLinkWiredCandidateCount\(null\)/);
assert.match(app, /setDjLinkSelectedBinding\(""\)/);
assert.doesNotMatch(app, /localStorage[^\n]*(dj|DJ)[^\n]*token/i);
const machineStatusFailureHandler = app.match(/const refreshDjLinkMachineStatus = async \(\) => \{[\s\S]*?\n  \};\n  const refreshDjLinkWiredCandidates/);
assert.ok(machineStatusFailureHandler, "machine status refresh handler is present");
assert.doesNotMatch(
  machineStatusFailureHandler[0],
  /setDjLinkToken\(/,
  "a failed machine status poll must not erase a fresh show-once token",
);
assert.match(persistenceRuntime, /Credential Manager/);
assert.match(persistenceRuntime, /web_remote_enabled: false/);
assert.match(persistenceRuntime, /validate_network_binding/);
assert.match(main, /project_mapping_not_loaded/);
assert.match(main, /mod dj_link_persistence_runtime/);
assert.doesNotMatch(main, /list_show_lan_interfaces/);
// Header density contract: master values remain available to non-header
// controls/API paths, while the two persistent slider widgets are gone.
assert.doesNotMatch(chrome, /data-topbar-master|topbarMasterCluster/);

const connected = availableRemoteControlStatus({
  running: true,
  web_remote_enabled: false,
  dj_link_enabled: true,
  active_connections: 1,
  rejected_connections: 0,
  clients: [],
  dj_link: {
    available: true,
    connected: true,
    peer: "192.168.1.20",
    generation: 7,
    ownerDeck: "rekordbox-deck-2",
    trackDeckId: "rekordbox-deck-2",
    trackActive: true,
    trackContentId: "content-1",
    trackTitle: "Track",
    trackArtist: "Artist",
    loopDivision: 2,
    released: false,
    lastEventId: "event-7",
    ageMs: 10,
  },
});
assert.equal(connected.dj_link?.connected, true);
assert.equal(connected.dj_link?.ownerDeck, "rekordbox-deck-2");
assert.equal(connected.dj_link?.trackDeckId, "rekordbox-deck-2");
const unavailableAfterRejectedPoll = unavailableRemoteControlStatus();
assert.equal(unavailableAfterRejectedPoll.running, false);
assert.equal(unavailableAfterRejectedPoll.dj_link?.available, false);
assert.equal(unavailableAfterRejectedPoll.dj_link?.connected, false);
assert.equal(unavailableAfterRejectedPoll.dj_link?.trackContentId, undefined);
const rejectedPollProjection = projectRemoteControlStatusPoll(null);
assert.equal(rejectedPollProjection.listenerRunning, false);
assert.equal(rejectedPollProjection.genericRunning, false);
assert.equal(rejectedPollProjection.djListenerRunning, false);
assert.equal(rejectedPollProjection.status.dj_link?.available, false);
assert.equal(rejectedPollProjection.status.dj_link?.trackTitle, undefined);
assert.match(app, /remoteStatusPollGeneration/);
assert.match(app, /projectRemoteControlStatusPoll\(null\)/);
assert.match(app, /timelineOptions=\{djTimelineOptions\(\)\}/);
assert.doesNotMatch(app, /timelineOptions=\{\(snapshot\(\)\.timeline_bank/);

assert.deepEqual(clearedDjLinkSecret(true), { token: null, copied: true });

// The shared CAS lane is array-agnostic. Prove a four-array DJ payload is sent,
// a rejected current generation adopts the authoritative DJ array, and a newer
// local DJ edit remains dirty for exact retry rather than being overwritten.
const localDj = [{ id: "local", timelineId: 1 }];
const sentPayload = { midi: [], osc: [], dmx: [], dj: localDj };
assert.deepEqual(sentPayload.dj, localDj);
let casState = noteLocalProjectAuthorityEdit(createProjectAuthoritySyncState());
const rejected = beginProjectAuthorityRequest(casState);
casState = rejected.state;
const sentGeneration = casState.localGeneration;
const recovered = beginProjectAuthorityRequest(casState);
casState = acknowledgeProjectAuthorityPersist(
  recovered.state,
  recovered.request,
  sentGeneration,
);
const authoritativePayload = { midi: [], osc: [], dmx: [], dj: [{ id: "remote", timelineId: 2 }] };
assert.deepEqual(authoritativePayload.dj, [{ id: "remote", timelineId: 2 }]);
assert.equal(projectAuthorityHasDirtyMappings(casState), false);

let retryState = noteLocalProjectAuthorityEdit(createProjectAuthoritySyncState());
const inFlight = beginProjectAuthorityRequest(retryState);
retryState = noteLocalProjectAuthorityEdit(inFlight.state);
retryState = acknowledgeProjectAuthorityPersist(
  retryState,
  inFlight.request,
  inFlight.state.localGeneration,
);
assert.equal(projectAuthorityHasDirtyMappings(retryState), true);
assert.equal(projectAuthorityShouldRetryPersist(retryState, inFlight.request.identityGeneration), true);

// Snapshot polling replaces the engine snapshot object. Semantically identical
// Timeline options must retain their array and option identities so the native
// select does not fall back to its placeholder between operator input events.
const retainedOptions = retainDjTimelineOptions([], [
  { id: 1, label: " Timeline 1 " },
  { id: 2, label: "" },
  { id: 0, label: "Rejected" },
]);
assert.deepEqual(retainedOptions, [
  { id: 1, label: "Timeline 1" },
  { id: 2, label: "Timeline 2" },
]);
assert.strictEqual(
  retainDjTimelineOptions(retainedOptions, [
    { id: 1, label: "Timeline 1" },
    { id: 2, label: "Timeline 2" },
  ]),
  retainedOptions,
  "equivalent status snapshots must not recreate Timeline option nodes",
);
assert.notStrictEqual(
  retainDjTimelineOptions(retainedOptions, [
    { id: 1, label: "Timeline 1" },
    { id: 3, label: "Timeline 3" },
  ]),
  retainedOptions,
  "an authored Timeline change must publish new options",
);

const manifestCommands = JSON.parse(manifest);
const invokeMatch = invokes.match(/export const FRONTEND_TAURI_INVOKE_COMMANDS = \[([\s\S]*?)\];/);
assert.ok(invokeMatch, "frontend invoke tuple is present");
const tupleCommands = [...invokeMatch[1].matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]);
assert.deepEqual(tupleCommands, [...tupleCommands].sort(), "frontend invoke tuple must stay sorted");
assert.deepEqual(manifestCommands, [...manifestCommands].sort(), "invoke manifest must stay sorted");
for (const command of [
  "arm_dj_link_machine",
  "disarm_dj_link_machine",
  "get_dj_link_machine_status",
  "list_dj_link_wired_candidates",
  "rotate_dj_link_token",
]) {
  assert.ok(tupleCommands.includes(command), `${command} missing from frontend tuple`);
  assert.ok(manifestCommands.includes(command), `${command} missing from invoke manifest`);
}
assert.ok(!tupleCommands.includes("list_show_lan_interfaces"), "retired address-only DJ picker must be absent from frontend tuple");
assert.ok(!manifestCommands.includes("list_show_lan_interfaces"), "retired address-only DJ picker must be absent from manifest");

for (const key of [
  "DJ Link",
  "Enable DJ Link",
  "Arm DJ Link",
  "Disarm DJ Link",
  "Wired binding",
  "Refresh wired bindings",
  "Refreshing…",
  "Eligible wired DJ Link bindings:",
  "No eligible wired DJ Link bindings found.",
  "Show-LAN bind IP",
  "Owner deck / playing",
  "Track mappings",
  "Use Current Track",
  "Rotate token",
  "DJ Link token rotated. Copy the replacement now; it will not be shown again.",
  "DJ Link credential cleanup is pending and will be retried at the next launch.",
  "Checking listener…",
  "Checking the shared Remote listener before starting Web Remote.",
]) {
  assert.match(localization, new RegExp(`['\"]${key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}['\"]\\s*:`), `${key} is not localized`);
}

console.log("DJ Link frontend contract checks passed");
