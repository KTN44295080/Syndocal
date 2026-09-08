# Native admission repair ? 2026-09-08

Base: `5fc63f1b29fc78f5c9033b5146e575788eeaf084`.
Branch: `chatgpt/macos-artifact-validation`. No main/release promotion.

## Reproduced native failure

The existing candidate EXE SHA-256
`A47AD5D8E589C03D04C86E2E7ECA2508BF3571DECB25D0E7851D5D880D898EE5`
was launched by ChatGPT through the authorized PC connection. The exact process
owned a responsive maximized window and a verified loopback-only CDP listener.
A real WebView called the installed official Tauri API, without IPC mocks,
project mutation or physical-output commands. The first `get_snapshot` failed:
`Tauri app command 'get_snapshot' is not in the reviewed admission inventory`.
No thumbnail case ran. The owned process exited and no debug listener remained.
This invalidates treating window-launch success as application-IPC acceptance.

## Cause and repair

The source inventory grew from 515 to 516 commands, with exactly one addition:
`cancel_native_thumbnail_request_v1`. The former inventory hash still matched
the parent, but the frozen count/hash had not been updated. Its cached error
causes the entire native admission table to return no class for any command.
The existing Rust exact-table test also failed before this repair.

The repair explicitly classifies only the existing exact-ticket cancel route as
RecoveryMaintenance and updates the reviewed count/hash. Ticket ownership and
unknown/retired-command rejection remain unchanged. No new executable command,
MCP operation, physical-output route or fallback is added.

## Source coverage and validation scope

The release command now runs `check-tauri-admission-inventory.mjs` before its
existing checks. The strict handler parser and frozen count/hash checks reject
15 malformed/drifting fixtures. This checks source identity only; the Rust gate
continues to prove route classes, risk/capability restrictions and retired/unknown
rejection. It adds no application runtime work or new dependency.

Broader control-plane tests exposed old fixed-count expectations in addition to
the failed native table. The inspected current sets contain 280 engine variants,
457 frontend invokes, 516 Tauri routes, 116 remote and 206 MIDI/OSC/DMX descriptors.
The legacy registry therefore has 1575 descriptors; the canonical source inventory
adds the existing 33 keyboard sources for 1608. The canonical executable operation
count remains 47. Tests keep their full set/risk/schema checks and exact totals.
The local class inventory is 133 renderer-ticketed and 28 recovery-maintenance
commands; these are metadata counts, not permission to expose them externally.

## Delegation and evidence limits

One Luna/Codex native-test attempt used workspace-write without disabling safety
settings. Its command cwd was C:\ and access to this repository was denied, so
it stopped before reviewing or launching the supplied scripts. Its agent exit 0
is not a test pass. ChatGPT subsequently ran the distinct authorized native probe
through the normal PC connector. A later inline review request was service-denied;
no second Luna review was executed. The repair has self-review only so far.
The original broad terminal inventory request and a later unrelated history query
were also denied and were not rerouted or counted as executed evidence.

## Native probe contract

The probe uses the installed official Tauri Channel/invoke implementation inside
the actual WebView, not a fake IPC callback. The launcher's fixed EXE hash, own
process identity, loopback-listener ancestry and maximized/responsive window are
checked before evaluation. A separate WebView data directory avoids reusing the
operator's browser data; this is not a claim of isolated backend machine storage.

After confirming a high fixture ID is absent from the current snapshot, the probe
requests thumbnails only for that absent ID. It checks delivery of one native
schema/lane/request-ID ticket, cancellation acknowledgement, retired-ticket replay
rejection and rejection of the old missing-Channel argument shape. It does not
add or load media, mutate a project, detach panes or issue physical output commands.
A false cancellation acknowledgement can mean the missing-ID request already
ended; it is not counted as stopping a running decoder. Successful rendering,
App-facade teardown ordering and hard decoder stop deadlines remain unproven.

Baseline evidence is kept in `target/qa/thumbnail-webview-channel-20260908-01/`.
A repaired artifact must use a separate result directory and its newly observed
EXE hash; the initial failed results must never be overwritten as successful runs.
The older frontend-routing checker failure (450 expected vs 457 actual) is a
separate retained issue; this patch does not weaken or claim to fix that checker.
Mac artifact-gate work, signing, physical-device acceptance and main integration
are outside this repair.

## Executed automated checks

All Cargo runs used the maintained exact Build Tools MSVC 14.44.35207 linker
pin and PATH-first verification. No compiler flags, SDK pins or fail-closed
assertions were disabled.

- Baseline existing exact-admission test: 0 passed / 1 failed (516 vs 515).
- Final `cargo test -p syndocal --release --locked control_plane::tests:: -- --test-threads=1`:
  30 passed / 0 failed / 0 ignored. The filter also includes authored-control-plane
  tests. Intermediate 26/4 and 29/1 failures are retained, not overwritten.
- Final `cargo test -p syndocal --release --locked thumbnail -- --test-threads=1`:
  23 passed / 0 failed / 0 ignored.
- Existing Node native-thumbnail-request client tests were rerun: 17 passed,
  no failures/skips. These are mock-IPC client tests, not native transport proof.
- New offline inventory checker: 516-name/hash match and 15 negative fixtures.
- `check:release`: exit 0 with the new inventory gate in its command chain.
- `check:tauri-build-wrapper`: 243 assertions / 27 hostile fixtures passed.
- Completion/Q1-Q4 validators passed; their acceptance rows were not closed.

Source-family inventories and class mappings are retained in `source-counts.json`,
`route-classes.json` and `inventory-diff.json` alongside the test logs.
The source edits are confined to control-plane inventory metadata/test expectations,
the offline checker, its release-chain entry, and this evidence note.

## Repaired native artifact and observed boundary

The maintained no-bundle build exited 0; optimized Rust build 3m27s.
Fresh EXE: `target/release/syndocal.exe`, 64,696,832 bytes, SHA-256
`4694427CB3E163000BC6E093384521C77E81076340E76E5700A0941F220675F9`.
This is the base plus this repair's source changes, not a frozen release tag.

Runs in `thumbnail-webview-channel-20260908-02` and `-03` each verified one
responsive/maximized exact-process Syndocal window, and the official WebView
`get_snapshot` call resolved instead of failing native admission. However the
probe then failed its own snapshot-shape assertion before either thumbnail lane.
The first probe assumed an unwrapped snapshot. Source inspection confirmed an
EngineSnapshotRuntimeWireResponse containing `snapshot` and `timeline_runtime`.
After adjusting the envelope in a new evidence directory, the strict array check
still failed. Protocol VideoSnapshot.media_assets is optional on the wire when
empty (`serde(default, skip_serializing_if = "Vec::is_empty")`); the probe had
incorrectly required it to be present. A follow-up edit/run request respecting
that declared omission rule was service-denied and was not reissued or delegated.

Neither repaired probe is a complete IPC/cancellation acceptance pass. Both kept
empty case lists and exit 1. Both owned applications exited and left zero debug
listeners. Final observed run: PID 54700 at `2026-09-08T02:23:38.1616542Z`.
No physical-output or project-mutation command was issued. The failure artifacts
remain unchanged. This checkpoint fixes the native admission outage and proves
that specific denial is gone; full thumbnail transport, operator teardown and
successful rendering remain open acceptance items.

Final targeted Rust test logs and the no-bundle build log contain zero Rust
warning diagnostics. A full pre-change compiler-warning baseline was not measured;
no overall supported-matrix warning delta is claimed. Independent review and the
complete real-WebView thumbnail cancellation gate remain required before release
acceptance. Only this repair's four files are included in the checkpoint.

## Concurrent branch update and publication ownership

The first push to the working branch was rejected as non-fast-forward. Its
remote tip had advanced to `362781120aba6f57fbc70b291285ff9a3fed5db7`, whose
one-commit delta from this base changes `crates/io/src/midi.rs` and
`qa/M4_IO_VALIDATION.md`. Those files are not part of this repair and have not
been reviewed or tested together with it. No force-push, implicit merge, reset
or rebase was used. The tested repair is published separately on
`chatgpt/native-admission-repair-20260908`; integration with the new MIDI commit
and promotion to main remain separate checks. The original dirty Mac files
are outside this checkpoint and were not staged.
