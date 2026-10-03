# Live native MCP process loss, restart and explicit re-Arm

Branch `codex/showclock-review-20260912`, base `30c20162`.
The preceding registered-owner proof deliberately preserved live output through
authority retirement/transfer. This checkpoint adds actual forced termination
of the isolated native process while the loopback sender is active, then fresh
pairing, authority and explicit re-Arm after native restart.

## Scope and behavior

`check-native-lifecycle.mjs --controller-restart` owns a separate opt-in lane.
The common `native-controller-output.mjs` still owns the independently authored
fixture/private project, ephemeral `127.0.0.1` UDP receiver, authenticated stdio
client, full 512-channel oracle and final project-replacement disarm. The new
`native-controller-restart.mjs` owns the process-loss assertions. The runner owns
exact executable/process checks, termination/relaunch, pairing, credential ACL,
revocation and process cleanup. No product runtime, IPC, schema or adapter
authorization changes are made.

The initial fixture has Dimmer 65535, Pan/Tilt 32768 and all other channels zero.
Explicit Arm emits `[255,0,0,0,128,0,128,0]` followed by zero slots. Explicit master
500 milliunits emits `[128,0,0,0,128,0,128,0]`. The probe checks:

1. Closing the owned stdio client preserves the live half image and gate state;
   adapter disconnect is distinct from losing the registered native owner.
2. The runner verifies the exact QA process has terminated. After queued packets
   drain for 250ms, the receiver observes no new packet for 500ms.
3. Native restart leaves both runtime gates closed and sends no packet. Reusing
   owner text, fresh pairing and exact grants do not reclaim the old lease or
   restore the persisted desired role. Process/session incarnation and instance
   identity change; old/new native process IDs are recorded.
4. Old terminal lookup and exact persisted UUID replay return `unknown` without
   a result; changing the shape under that UUID returns `request_conflict`.
   A fresh UUID carrying the old process fence returns `stale_fence`.
5. Explicit reload of the original private source restores full master, keeps
   gates closed and has no lease. Old lease Arm and master requests return
   `forbidden`, preserving state and receiver silence.
6. Fresh exact lease acquisition changes authority only; output remains silent.
7. Separate explicit R4 Arm resumes the independent full image. At least ten new
   complete 512-channel frames must arrive; a cached image cannot satisfy the
   probe. Exact terminal replay leaves state/image unchanged.
8. Native replacement with a private empty disabled project closes both gates
   and stops the sender. The original source hash remains unchanged. The common
   helper and runner close/revoke/remove their owned resources.

## Exact replay expectation

The [first probe expectation](artifacts/controller-restart-expectation-before-2026-10-03.json)
and [log](artifacts/controller-restart-expectation-before-2026-10-03.txt) retain
seven successful groups before an incorrect harness expectation: exact UUID
replay after restart was expected to conflict, but returned `unknown`.

Source inspection of `agent_bridge_ledger.rs` confirms the contract:
`Ledger::new` retains bounded mutation shape identity but clears every previous
process response; `status_owned` does not disclose an unbound old receipt.
`begin_owned` returns the unknown retained identity without enqueueing a dispatch
when the principal/incarnation/command shape matches, and conflicts on a different
shape. Pairing incarnation is process-local; fresh credentials and launch proof
are still required. The corrected probe requires exactly `unknown` with no result,
adds the changed-shape conflict check, then proves old process fence and old lease
rejection. It does not accept mutation uncertainty as success, automatically
resubmit an unknown mutation, relax an authority check or alter production code.
This proves replay suppression after a completed pre-crash operation, not durable
completion or an in-flight reply-loss/crash publication result.

## Validation

[Current restart proof](artifacts/native-controller-restart-2026-10-03.json) and
[log](artifacts/native-controller-restart-2026-10-03.txt) pass all 19 groups:
twelve shared/live-restart groups and seven base lifecycle groups. The exact QA
PID 59292 was terminated while 211 packets had been received. The new QA PID
169160 had a new instance/process/session identity, effective Standby, persisted
desired Lighting and `StartupDenied`; both output gates stayed closed. No new
packet arrived through restart, pairing/grants, source reload, stale mutations
or fresh acquisition. Separate explicit Arm resumed full DMX.

The receiver validates the wire header of all 267 datagrams. Four stable phases
compare all 512 slots in 105 newly received frames:

| Stable phase | New frames | Dimmer slot |
| --- | ---: | ---: |
| Initial Arm | 22 | 255 |
| Before process termination | 22 | 128 |
| Stdio disconnect with native owner alive | 23 | 128 |
| Explicit Arm after restart and exact replay | 38 | 255 |

[Retirement regression](artifacts/controller-restart-output-regression-2026-10-03.json)
and [log](artifacts/controller-restart-output-regression-2026-10-03.txt) pass the
existing 16 groups after common disarm code extraction and flag wiring. Both
runs observe no native panic, clean up their exact QA process, sidecar, receiver,
credential and private files, and preserve normal app identity. Evidence pins
current helper/runner/sidecar SHA-256 values.
[CLI isolation](artifacts/controller-restart-cli-2026-10-03.txt) also proves eight
conflicting opt-in combinations reject before executable selection or launch.
Node syntax and owned diff checks pass. No generated test result is counted as
a Rust or physical-device test.

The current cached optimized QA executable has SHA-256
`f14e39191d0836247dd62154126c9d691f92381dc7ae8523c9c780846021fd81`, built with
the exact MSVC 14.44.35207 procedure in the preceding checkpoint. Product source
is unchanged, so no new Cargo/Tauri build or compiler-warning measurement is
performed. Probe-log warnings and native panic locations are measured separately.
Both native probe logs contain zero warnings and both native panic arrays are
empty; these are not a new compiler-warning measurement.
The normal executable/profile is not restarted or debugged by this lane.

Computer Use and subagents are not used. Normal-profile identity, protected dirty
files and unrelated processes must remain unchanged. Forced native termination
with software loopback does not establish physical fixtures, video/serial DMX,
worker-specific failure, TTL while live, durable in-flight mutation crash/restart,
venue behavior, clean installation or complete AI3/AI8 acceptance.

The Q4 mirror/master record this slice as
`EV-AI8-NATIVE-CONTROLLER-RESTART-2026-10-03`, with 19 selected native groups and
the separate 16-group regression. Shared base groups are not added to distinct
coverage. Evidence count advances 171 to 172. Overall AI control and safety risk
status remain open; actual live TTL, worker-specific failure, in-flight durability
and hardware/domain acceptance are still outside this proof.
