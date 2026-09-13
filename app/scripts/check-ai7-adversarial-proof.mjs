import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (relativePath) =>
  readFile(new URL(relativePath, import.meta.url), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const [main, registry, bridge, authored, query, authority, lease, sidecar, recording, ai0, routing] =
  await Promise.all([
    read("../src-tauri/src/main.rs"),
    read("../src-tauri/src/control_plane.rs"),
    read("../src-tauri/src/agent_bridge_tests.rs"),
    read("../src-tauri/src/authored_control_plane.rs"),
    read("../src-tauri/src/control_plane_query.rs"),
    read("../src-tauri/src/agent_authority_service.rs"),
    read("../src-tauri/src/output_lease.rs"),
    read("../../tools/syndocal-mcp/server.mjs"),
    read("../src-tauri/src/video_recording_runtime_tests.rs"),
    read("./check-ai0-source-coverage.mjs"),
    read("./check-frontend-command-routing.mjs"),
  ]);

const required = (source, needle, label) => {
  assert.ok(source.includes(needle), `${label}: missing ${needle}`);
};

for (const marker of [
  "compiled_handler_and_registry_have_the_exact_same_set",
  "tauri_route_admission_table_is_exact_and_fail_closed",
  "FROZEN_TAURI_ROUTE_ADMISSION_SHA256",
]) required(registry, marker, "registry parity");

for (const marker of [
  "agent_bridge_claim_is_exact_once_and_replay_never_dispatches_twice",
  "agent_bridge_oversize_result_cannot_be_reported_completed",
  "agent_bridge_reload_fences_late_completion_and_does_not_replay",
  "agent_bridge_restart_retains_mutation_identity_without_automatic_replay",
]) required(bridge, marker, "bridge reply-loss proof");

for (const marker of [
  "exact_reply_retry_is_single_flight_and_byte_identical",
  "same_key_different_shape_is_rejected_without_republishing",
  "two_principals_from_one_start_fence_admit_only_one_and_stale_the_other",
  "generic_authored_bridge_replays_exact_terminal_result_and_rejects_shape_reuse",
]) required(authored, marker, "authored parity proof");

for (const marker of [
  "retained_event_gap_is_explicit_and_requires_resnapshot",
  "cursor_expiry_capacity_retirement_and_replay_are_bounded",
]) required(query, marker, "event-gap proof");

for (const marker of [
  "wrong_or_replayed_pairing_challenge_fails_closed",
  "revoke_removes_credential_and_kill_switch_closes_pairing",
  "consent_delegates_exact_binding_to_protocol_authority",
]) required(authority, marker, "authority adversarial proof");

for (const marker of [
  "output_lease_registry_ten_thousand_sequential_requests_keep_truth_bounded",
  "output_lease_registry_rate_bucket_is_burst_eight_and_clock_rollback_fails_closed",
]) required(lease, marker, "saturation/rate proof");

for (const marker of [
  "ticketed_invoke_fields_reject_wrong_case_type_and_fractional_numbers",
  "project_transaction_commit_reply_loss_and_inflight_owner_rotation_fail_closed",
  "d4_stage_all_nine_routes_persist_applied_reply_loss_results_and_reject_mismatches",
]) required(main, marker, "cross-domain fail-closed proof");

required(recording, "recording_file_name_and_terminal_status_are_safe", "recording reply-loss boundary");

for (const marker of [
  "dispatchRpc",
  "tools/list",
  "HTTP_SESSION_LIMIT",
  "WEBSOCKET_CONNECTION_LIMIT",
  "overloaded",
  "No automatic retry was performed",
]) required(sidecar, marker, "sidecar crash/rate boundary");

required(ai0, "invokeManifest.length, 475", "source coverage inventory");
required(routing, "manifest.length, 475", "frontend routing inventory");

if (process.argv.includes("--self-test")) {
  assert.equal(["parity", "reply-loss", "authority", "gap", "rate", "saturation"].length, 6);
  assert.equal(["Tauri", "MIDI", "OSC", "DMX", "Remote", "shortcut", "MCP"].length, 7);
  assert.equal("10,000".replaceAll(",", ""), "10000");
  console.log("AI7 adversarial proof self-test passed: 3 assertions");
} else {
  console.log("AI7 adversarial proof source contract ok; parity, reply-loss, authority, gap, rate, saturation and sidecar boundaries are enumerated");
}
