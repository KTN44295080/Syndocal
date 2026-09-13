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
