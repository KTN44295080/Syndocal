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

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const [app, types, panel, chrome, invokes, manifest, localization] = await Promise.all([
  read("src/App.tsx"),
  read("src/types.ts"),
  read("src/components/RemoteControlPanel.tsx"),
  read("src/components/WorkspaceChrome.tsx"),
  read("src/tauriInvokeCommands.ts"),
  read("src/tauri-invoke-manifest.json"),
  read("src/uiLocalization.ts"),
]);

for (const source of [app, types]) {
  assert.match(source, /DjTrackTriggerMapping/);
  assert.match(source, /dj_track_triggers|djTrackTriggers/);
}
assert.match(types, /DjTrackSelector/);
assert.match(panel, /DjTrackTriggerMapping/);
assert.match(panel, /djTrackTriggers/);
assert.match(types, /interface DjLinkRuntimeStatus/);
assert.match(types, /contentId/);
assert.match(types, /timelineId/);
assert.match(types, /outcome\?: string \| null/);
assert.match(types, /interface RemoteControlConfig[\s\S]*dj_link_enabled/);
assert.match(panel, /data-io-disclosure="dj-link"/);
assert.match(panel, /Use Current Track/);
assert.match(panel, /once_per_play_session/);
assert.match(panel, /status\.available !== false/);
assert.match(panel, /disabled=\{!linkStatus\(\)\?\.trackContentId/);
for (const label of ["Available", "Enabled", "Outcome", "Peer", "Generation", "Heartbeat", "Last event"]) {
  assert.match(panel, new RegExp(`>${label}\\s`), `${label} diagnostic is not rendered`);
}
assert.doesNotMatch(panel, /MIDI|Filter CC|Stop MIDI|hotkey/i);
assert.match(app, /list_show_lan_interfaces/);
assert.match(app, /rotate_dj_link_token/);
assert.match(app, /djLinkTokenClearTimer/);
assert.match(app, /djLinkBindIp\(\)[\s\S]*remoteBindIp\(\)/);
assert.doesNotMatch(app, /localStorage[^\n]*(dj|DJ)[^\n]*token/i);
// Header density contract: master values remain available to non-header
// controls/API paths, while the two persistent slider widgets are gone.
assert.doesNotMatch(chrome, /data-topbar-master|topbarMasterCluster/);

const connected = availableRemoteControlStatus({
  running: true,
  active_connections: 1,
  rejected_connections: 0,
  clients: [],
  dj_link: {
    available: true,
    connected: true,
    peer: "192.168.1.20",
    generation: 7,
    master: true,
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
const unavailableAfterRejectedPoll = unavailableRemoteControlStatus();
assert.equal(unavailableAfterRejectedPoll.running, false);
assert.equal(unavailableAfterRejectedPoll.dj_link?.available, false);
assert.equal(unavailableAfterRejectedPoll.dj_link?.connected, false);
assert.equal(unavailableAfterRejectedPoll.dj_link?.trackContentId, undefined);
const rejectedPollProjection = projectRemoteControlStatusPoll(null);
assert.equal(rejectedPollProjection.running, false);
assert.equal(rejectedPollProjection.status.dj_link?.available, false);
assert.equal(rejectedPollProjection.status.dj_link?.trackTitle, undefined);
assert.match(app, /remoteStatusPollGeneration/);
assert.match(app, /projectRemoteControlStatusPoll\(null\)/);

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

const manifestCommands = JSON.parse(manifest);
const invokeMatch = invokes.match(/export const FRONTEND_TAURI_INVOKE_COMMANDS = \[([\s\S]*?)\];/);
assert.ok(invokeMatch, "frontend invoke tuple is present");
const tupleCommands = [...invokeMatch[1].matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]);
assert.deepEqual(tupleCommands, [...tupleCommands].sort(), "frontend invoke tuple must stay sorted");
assert.deepEqual(manifestCommands, [...manifestCommands].sort(), "invoke manifest must stay sorted");
for (const command of ["list_show_lan_interfaces", "rotate_dj_link_token"]) {
  assert.ok(tupleCommands.includes(command), `${command} missing from frontend tuple`);
  assert.ok(manifestCommands.includes(command), `${command} missing from invoke manifest`);
}

for (const key of [
  "DJ Link",
  "Enable DJ Link",
  "Show-LAN bind IP",
  "Track mappings",
  "Use Current Track",
  "Rotate token",
  "DJ Link token generated. Copy it now; it will not be shown again.",
]) {
  assert.match(localization, new RegExp(`['\"]${key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}['\"]\\s*:`), `${key} is not localized`);
}

console.log("DJ Link frontend contract checks passed");
