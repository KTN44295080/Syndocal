# ShowClock runtime and LAN revalidation — 2026-09-12

## Scope

This checkpoint continues the accepted ShowClock decisions with an additive,
authenticated action-payload extension to the v1 action schema:

- `crates/protocol/src/show_clock_runtime.rs` owns the deterministic estimator,
  bounded correction, stale/hold policy, action horizon/late policy, fixed
  schedule capacity, generation rebind, and output ownership gate.
- `crates/io/src/show_clock_lan.rs` owns a manually paired, exact-peer UDP
  unicast adapter. It has no discovery path and never grants output ownership.
- Authentication remains the protocol boundary: callers must first admit an
  authenticated sample/action, then pass the accepted body through runtime
  policy. Project, lease, audio, recording, clock, and fencing generations are
  checked together before output dispatch.
- `ShowClockActionPayload` binds Release, Take, ClipLaunch/Transition, and
  TimelineJump to their required typed fields. Payload presence, kind, IDs,
  and values are validated before canonical authentication bytes are accepted.

The frozen software policy is:

| Item | Value |
| --- | --- |
| First sample | Establishes the local-to-show offset |
| Later correction | At most 1,000 µs per accepted sample |
| Stale threshold | The lower of configured 750,000 µs and the sample expiry |
| Lock threshold | 3 accepted samples |
| Action horizon | ±10,000,000 µs from current show time |
| Action queue | 256 actions, no eviction |
| LAN | Explicit peer address, UDP port constant 44,666, no discovery |
| Late action | Execute immediately, Drop, or Hold as signed in the action |
| Recovery | Stale/Fault → explicit Manual Hold → operator-confirmed advanced fence |

## Implementation

The estimator publishes monotonic show time and never steps it backwards. A
valid sample in Hold updates no output state and cannot leave Hold by itself.
The action scheduler refuses generation mismatches and out-of-horizon actions,
does not consume the queue when output authorization fails, and clears old
actions on an explicitly armed fence rebind. The output gate requires exact
owner identity and exact project/lease/audio/recording/clock/fencing context,
plus Locked estimator state. The native integration holds the local lighting
ownership permit while the gate is armed; video actions reacquire a local video
permit for each dispatch.

The LAN adapter serializes a strict versioned envelope containing an already
authenticated sample or action. It ignores traffic from non-paired endpoints
without extending the absolute receive deadline and fails closed on malformed
or oversized packets from the paired endpoint.

## Evidence

All direct Cargo commands were run after `vcvars64.bat -vcvars_ver=14.44` with
the exact Build Tools linker pinned first:

`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`

| Command | Observed result |
| --- | --- |
| `cargo test -p protocol --locked show_clock -- --nocapture --test-threads=1` | PASS — 18 focused tests |
| `cargo test -p protocol --locked -- --test-threads=1` | PASS — 236 unit, 7 integration, 4 doctests; 0 failed/ignored |
| `cargo test -p io --locked show_clock_lan -- --nocapture --test-threads=1` | PASS — 3 focused loopback tests |
| `cargo test -p io --locked -- --test-threads=1` | PASS — 184 unit tests, 3 ignored, 0 failed; 2 two-process integration tests passed; 0 doctests |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --locked show_clock_ipc::tests -- --nocapture --test-threads=1` | PASS — 6 lifecycle/action/fence/output-dispatch tests |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --locked control_plane::tests -- --nocapture --test-threads=1` | PASS — 30 command-admission tests |
| `pnpm.cmd --dir app exec tsc --noEmit; pnpm.cmd --dir app run build` | PASS — TypeScript and Vite production build; 354 modules transformed |
| `pnpm.cmd --dir app run check:frontend-invokes; pnpm.cmd --dir app run check:frontend-command-routing; node app/scripts/check-tauri-admission-inventory.mjs; pnpm.cmd --dir app run check:output-control-runtime` | PASS — 464 frontend commands; routing 133/31/28/471; 523 native commands with 18 negative fixtures rejected; output-control contracts pass |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — exact MSVC 14.44.35207 linker; final release executable built in 3m38s without first-party warnings |
| Exact `target/release/syndocal.exe` process smoke | PASS — exactly 1 exact-path process, `Syndocal` title, nonzero window handle, `Responding=True`, maximize requested, exact-path cleanup complete |
| `git diff --check` | PASS |

The final current-source executable SHA-256 is
`916E8221F81984A8740DFF569CB988BBEA54A2ADBB633194C6E381988595E132`.
It is an unsigned, unpublished process-smoke binary, not release acceptance.

The new protocol tests cover required and kind-bound action payloads, payload
authentication bytes, and operator confirmation for the initial Manual Fence
arm. The new LAN tests cover signed-sample round trip over two explicitly paired
loopback sockets, wrong-source traffic that cannot extend timeout, and
malformed paired traffic rejected before any caller can process it. The
protocol tests cover authentication-before-estimation, backward receive-time
atomicity, bounded slew, monotonic output, expiry-derived STALE, Hold/Re-arm,
late action behavior, generation invalidation, output gate protection, and a
10,000-sample deterministic fault/soak loop.
The estimator regression also proves that STALE remains latched until explicit
Manual Hold and an advanced armed fence; the native dispatcher regression
proves that STALE/FAULT revokes an armed local output gate and permit before
queue polling.

## Tauri IPC/UI and process loopback

The native app now owns a process-lifetime ShowClock worker through the typed
commands `get_show_clock_status`, `start_show_clock`, `stop_show_clock`,
`schedule_show_clock_action`, `hold_show_clock`, and `rearm_show_clock`.
`arm_show_clock_output` is a separate explicit operator-confirmed command.
Primary emits signed samples every 250 ms and signs actions; Standby admits the
exact paired UDP sender before updating the estimator or fixed-capacity action
queue. Worker stop joins the thread before the socket is replaced, and a
process-local session registry rejects reuse of a session incarnation. The
Setup/IO ShowClock LAN panel exposes pairing settings plus action scheduling,
Manual Hold, and an operator-confirmed Re-arm form; only non-secret settings
are remembered. The session and key are deliberately not persisted, so a
process restart requires a fresh paired session and key.

An accepted action is retained in the generation-bound scheduler while the
ShowClock output gate is disarmed. Primary output arm requires the explicit
confirmation and a local lighting ownership permit. Standby output arm
requires `LOCKED` estimator state plus the already-established Manual Hold /
Re-arm fence. Manual Hold and Re-arm disarm the gate and drop the permit.
Lighting actions are dispatched through the existing local `EngineHandle`; Take
and ClipLaunch/Transition actions additionally recheck local video ownership
per operation. `output_armed` therefore records software ownership/gate state,
not physical output observation.

The parent/child integration test sends three authenticated samples across two
separate worker processes and verifies Standby admission plus LOCKED estimator
state. The Tauri worker test additionally verifies clean stop, STALE after
Primary loss, same-process session reuse rejection, and fresh-session restart
boundary. Frontend command inventory/routing, TypeScript, Vite production
build, native admission inventory, and output-control contract checks also pass.

The current IPC/UI surface exposes action scheduling, Manual Hold, operator
Re-arm, explicit output-arm confirmation, and all-domain generation context
selection. The UI and native dispatcher now cover the local software ownership
boundary; native UI interaction and physical-output acceptance remain separate
gates.

## Boundaries that remain open

This is current-source software and loopback evidence only. It does not prove
the native UI interaction matrix, physical
MIDI/OSC/DMX output, external Art-Net nodes, a real wired two-machine switch,
partition/rejoin, crash/restart with restored replay state, automatic failover,
witness/physical interlock, venue behavior, signing, or product completion.
The process worker intentionally requires a fresh session/key after restart;
there is no replay-state restoration implementation in this checkpoint.
