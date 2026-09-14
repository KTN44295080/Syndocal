# ShowClock Venue Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `SHOWCLOCK-VENUE-001` (section 8, Open)
- Q1 row: `COV-SHOWCLOCK-001`
- Branch: `codex/showclock-review-20260912`
- Current base: `e5c61392bee24a03f5fb860704a797a8e62ab4c8`
- Existing software authority: `qa/SHOWCLOCK_IMPLEMENTATION_CHECKPOINT_2026-09-13.md`

This checkpoint carries the venue marker forward to the current tree. The
ShowClock implementation checkpoint remains the source of the exact protocol,
LAN, IPC/UI, two-process loopback, native-build, and process-smoke results. No
new venue or two-machine acceptance claim is added here.

## Current-source boundary

The current source keeps authenticated ShowClock admission, generation/fence
ownership, Manual Hold, explicit Re-arm, output retirement, and no automatic
failover in the existing implementation. The H2/H5 UI additions do not convert
loopback or process evidence into venue acceptance.

## Unresolved acceptance

`SHOWCLOCK-VENUE-001` stays Open. The required wired two-machine rehearsal must
exercise real-switch partition, primary crash/restart, stale-peer rejoin,
device loss, replay restoration, and zero simultaneous physical output. A
witness/interlock or automatic failover claim is not permitted without its own
accepted decision and evidence. The local environment did not provide the
operator venue topology for this gate.

Next action is the named two-machine venue run with switch/failure timeline,
both process identities, output-owner transitions, raw ShowClock logs, and
physical-output observation bound to the exact artifacts.

## Takeover rerun — 2026-09-14

The current source at `7daf00fe` was revalidated with the pinned Windows
procedure: `vcvars64.bat -vcvars_ver=14.44`, MSVC `14.44.35207`, and the
absolute `Hostx64\\x64\\link.exe` first in `where.exe link.exe`.

The following current-source and loopback checks passed with exit code 0:

```text
cargo test -p protocol --release --locked -j 1 show_clock -- --nocapture --test-threads=1
  21 passed, 0 failed, 0 ignored
cargo test -p io --release --locked -j 1 show_clock_lan -- --nocapture --test-threads=1
  3 passed, 0 failed, 0 ignored
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 show_clock_ipc::tests -- --nocapture --test-threads=1
  7 passed, 0 failed, 0 ignored
cargo test -p io --release --locked -j 1 --test show_clock_two_process -- --nocapture --test-threads=1
  2 passed, 0 failed, 0 ignored
pnpm.cmd --dir app run check:frontend-invokes
  480 commands
pnpm.cmd --dir app run check:frontend-command-routing
  133 renderer, 31 server-authoritative, 28 raw, 479 facade dispatches
node app/scripts/check-tauri-admission-inventory.mjs
  539 commands, 18 negative fixtures rejected
pnpm.cmd --dir app run check:output-control-runtime
  output-control and Standby Sync contracts PASS
```

This rerun confirms the authenticated ShowClock admission, generation/fence
policy, Manual Hold/Re-arm, exact-peer loopback, Tauri IPC lifecycle, and
two-process software boundary remain intact. It does not add evidence for a
real switch, a second machine, power-loss/crash/restart replay restoration,
stale-peer rejoin, device loss, physical output observation, or zero
simultaneous physical output. `SHOWCLOCK-VENUE-001` remains Open.
