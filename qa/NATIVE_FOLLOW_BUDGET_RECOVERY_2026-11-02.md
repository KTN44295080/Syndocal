# Native Follow budget recovery checkpoint

Runtime base: `80831ffd6d3b9398f2e19b6f2ec97395d535679c` on shared `main`.
The separately committed Syphon work does not overlap these video files.

## Behavior and compatibility

Follow previously converted a broker allocation refusal into a terminal native
fault. At 2560x1600, retained CPU history could prevent every subsequent Follow
tick from fitting the 512 MiB ledger, so failed ticks never replaced that history.

The canonical CPU bridge now retains the typed refusal within the invocation
that produced its returned error. Follow restores the exact owner before deciding
whether the refusal is a healthy `BudgetDeferred`. The first consecutive refusal
preserves history/LastValid; the second releases replaceable CPU queues through
the same recovery operation as ordinary output. A complete GPU-to-CPU Handover
is preserved and published through its existing owner path rather than dropped
by an unmatched `history.take()`.

Cancellation, revocation, native faults, unfinished work, and retained reservations
retain their existing rejection/retirement behavior. An outgoing Follow scope may
already have completed before an incoming scope refuses admission. The failed
output does not rerun that scope, produce Fresh, present, or acknowledge Follow.
The existing `BudgetDeferred` API comment now states that partial-side boundary.

There is no project/schema/IPC change, general history-capacity reduction,
internal retry, new thread, or added successful-frame copy. The old CPU bridge
entry point delegates to one implementation. No UI sizing or control changes.

## Evidence

Evidence root: `%LOCALAPPDATA%/SyndocalDev/follow-budget-20261102/`.
Independent review of the stable runtime and regression-test diff found no
blocking issue. All six focused tests passed, including partial-side clocks,
distinct source identities, Stop/revocation, exact error association, real
Handover ownership, recovery to Fresh, and zero managed bytes after Stop.

The 21-configuration C4R12 matrix has 20 passing configurations with zero
first-party warnings. The full app run recorded 2345 passes, one known audio
clock timing failure (3002 vs 3000 ms), and 25 ignored tests. The unchanged audio
assertion passed on an isolated rerun; the original failed run is retained.
Initial missing DLL staging in the fresh archive was corrected using the pinned
schema-4 FFmpeg/NDI identities; affected final matrix runs use those payloads.

Matched C4 diagnostic (`native_gpu_group_fx_budget_trace_across_follow_crossfade_at_c4_extent`):
60 steady resident ticks, 60 Follow ticks, 60 resident ticks. Each cell is
`[resident, CPU, refused]`; the steady phase is `[60,0,0]` in every case.

| Workload | Follow, base `80831ffd` | Follow, candidate | After Follow, base | After Follow, candidate | Peak MiB (candidate) |
| --- | --- | --- | --- | --- | --- |
| Three ungrouped layers | `[0,0,60]` | `[0,40,20]` | `[58,0,2]` | `[56,2,2]` | 468.8 |
| Transformed groups | `[0,0,60]` | `[0,36,24]` | `[52,4,4]` | `[55,3,2]` | 500.1 |
| Transformed groups + group FX | `[0,0,60]` | `[0,30,30]` | `[21,25,14]` | `[19,28,13]` | 500.1 |

Base refusals were terminal runtime errors (180 of 200 failed ticks); every
candidate refusal is a healthy `BudgetDeferred` (91 of 91).

All three steady phases remain resident for 60/60 ticks. This existing C4
diagnostic uses the same snapshot on both sides; dedicated regressions add
different source identities. These are headless recovery observations, not
physical output or smooth-Follow acceptance.

- Matched alternating paced before/after probe (C4R12 paced headless, 5400
  ticks, 3 real decoder inputs, 2560x1600, 3 ISF stages): tick work mean/p95/p99
  ms before 6.53/9.74/10.77 and 5.87/8.66/10.16, after 6.57/10.03/11.07 and
  5.67/7.85/9.11; 0 ticks over 16.67 ms and 0 late look-ahead in all four runs.
  No regression.
- Integrated Windows `pnpm --dir app tauri build --no-bundle` (pinned MSVC
  14.44 and FFmpeg/NDI runtime staging, QA identifier
  `jp.seraf.ktn.syndocal.qa.follow-budget-20261102`, fresh profile, no output
  armed): 7m22s, 0 warnings. `target/release/syndocal.exe` launched as exactly
  one responsive `Syndocal` window, maximized, then closed normally.

## Remaining boundary and next action

This change removes permanent budget refusal. Follow is still CPU-composed and
can skip one third to one half of ticks in the C4 pressure case; the tight group
FX case also returns slowly to the resident route. Smooth 60fps Follow requires
a separately bounded retention/admission or resident-composition change.
The integrated C5 campaign, 30-minute soak and operator hardware acceptance
remain required; neither this matrix nor the paced probe closes those gates.
