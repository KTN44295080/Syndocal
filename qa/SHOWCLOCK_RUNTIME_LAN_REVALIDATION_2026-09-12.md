# ShowClock runtime and LAN revalidation — 2026-09-12

## Scope

This checkpoint continues the accepted ShowClock decisions without changing
the v1 wire schema:

- `crates/protocol/src/show_clock_runtime.rs` owns the deterministic estimator,
  bounded correction, stale/hold policy, action horizon/late policy, fixed
  schedule capacity, generation rebind, and output ownership gate.
- `crates/io/src/show_clock_lan.rs` owns a manually paired, exact-peer UDP
  unicast adapter. It has no discovery path and never grants output ownership.
- Authentication remains the protocol boundary: callers must first admit an
  authenticated sample/action, then pass the accepted body through runtime
  policy. Project, lease, audio, recording, clock, and fencing generations are
  checked together before output dispatch.

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
plus Locked estimator state.

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
| `cargo test -p protocol --locked show_clock -- --nocapture --test-threads=1` | PASS — 16 focused tests |
| `cargo test -p protocol --locked -- --test-threads=1` | PASS — 234 unit, 7 integration, 4 doctests; 0 failed/ignored |
| `cargo test -p io --locked show_clock_lan -- --nocapture --test-threads=1` | PASS — 3 focused loopback tests |
| `cargo test -p io --locked -- --test-threads=1` | PASS — 184 tests, 3 ignored, 0 failed; 0 doctests |
| `git diff --check` | PASS |

The new LAN tests cover signed-sample round trip over two explicitly paired
loopback sockets, wrong-source traffic that cannot extend timeout, and
malformed paired traffic rejected before any caller can process it. The
protocol tests cover authentication-before-estimation, backward receive-time
atomicity, bounded slew, monotonic output, expiry-derived STALE, Hold/Re-arm,
late action behavior, generation invalidation, output gate protection, and a
10,000-sample deterministic fault/soak loop.

## Boundaries that remain open

This is current-source software and loopback evidence only. It does not prove
the Tauri UI/IPC is wired to ShowClock, native-window interaction, physical
MIDI/OSC/DMX output, external Art-Net nodes, a real wired two-machine switch,
partition/rejoin, crash/restart with restored replay state, automatic failover,
witness/physical interlock, venue behavior, signing, or product completion.
The LAN adapter intentionally requires the setup layer to create a fresh
session/key or restore authoritative replay state after restart; merely
reconstructing a validator with the same session resets replay history and is
not accepted as a restart proof.
