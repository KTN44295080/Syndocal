# Native work-spike diagnostic — 2026-11-09

Base: `main` at `5e9fe8ad85aa38aa92ca585621e1222d0ea6731a`.
The [thirty-minute soak](NATIVE_60FPS_SOAK_2026-11-08.md) remains **FAIL**:
minute 13 retained a 35,604 µs native-work sample above the unchanged 33,333 µs
G3 bound. Its CPU/GPU/decoder/OS cause is unknown. Resident default stays OFF.

## Bounded diagnostic

The existing full trace fills its 3,072-parent capacity in about 51 seconds,
so it cannot capture a first spike thirteen minutes later. A separate coarse
recorder is compiled only with `qa-native-frame-timing` and activated only by
`SYNDOCAL_QA_NATIVE_SPIKE_OUT=<absolute-new-file>`. It is mutually exclusive
with `SYNDOCAL_QA_FRAME_TIMING_OUT`. Without the feature the hooks are absent;
with neither environment variable, the new recorder does not initialize TLS
or its clock.

The recorder counts every admitted iteration and retains the first 16 selected
records in fixed storage below 16 KiB. Further matches increment overflow.
It performs no allocation, file I/O, serialization or control-plane call in the
frame loop. Export uses the existing Stop/owner-retirement/ACK boundary and
`create_new`; existing files are never overwritten. Partial, unobserved,
unprepared and invalid observations remain explicit, rather than disappearing.

Each selected record has the exact pacing-excluded work duration and sample
Instant used by G3, a work-sample sequence and validation epoch. Last Presented
request sequence is separate; it must not be presented as the identity of that
work sample. Coarse extent, permit, retirement/configure, prepare, present and
metrics-lock intervals are CPU wall time. Present includes the separately
reported pacing subset. Neither the buckets nor their sum prove GPU execution
time or causation. A frame admitted before Stop can finish afterwards.

The retained harness patch `C5_SPIKE_DIAGNOSTIC_2026-11-09.patch` applies after
`C5_HARNESS_2026-11-08.patch` to the protected ignored C5 harness. Its
`--diagnostic-spikes off|on` modes use the same timing executable and resident
GPU ON workload, one MCP-controlled window, 15 s warmup and at most 1,800 s.
OFF/ON here controls the observer. Both always report
`DIAGNOSTIC-NOT-GATING`; success means consistent diagnostic evidence and clean
lifecycle, never C5/soak acceptance. Ordinary C5 and soak gates are unchanged.

## Verified checkpoint

Evidence lives under `target/qa/native-spike-diagnostic-20261109/`:

- Exact MSVC 14.44.35207 pin and PATH-first checks were printed and verified.
- `cargo test -p video --features qa-native-frame-timing native_frame_ --
  --test-threads=1`: 23 passed, zero failures/ignored; 702 unrelated tests
  filtered. Covers fixed capacity, 100,000 fast iterations before a spike,
  shared metric clock/pacing, disabled TLS, handoff, Stop/in-flight completion,
  explicit partials and post-ACK create-new export.
- `pnpm --dir app tauri build --no-bundle --config <isolated QA config>`:
  release build succeeded. Source hashes match `normal-source-before.json`.
  The feature-OFF EXE SHA-256 is
  `b7662f58cddb60130b9093dd2f859fe2a4f1620e3064a2dcb511b44d4b08dad9`.
- `cargo check -p syndocal --features qa-native-frame-timing,ndi`: completed
  successfully (`check-feature-crlf.log`). The first wrapper attempt rejected
  mixed LF/CRLF batch input before Cargo; it is retained as `check-feature.log`
  and does not count as a compile result. The corrected CRLF wrapper verified
  the exact pin/PATH before Cargo.
- `normal-native-proof.json`: exactly one responsive, maximized native
  `Syndocal` window at 2560×1600; rendered root populated. Video-only QA profile
  stayed effective Standby with lighting/video denied over three ownership
  polls; no live video outputs. Graceful exit 0, no forced termination, CDP
  9287 free. No physical output was requested.
- Synthetic harness tests cover diagnostic admission, malformed schema,
  clock/identity/counter consistency, Stop boundary and failure exit. Eleven
  invalid session-driver configurations reject before profile/process work.
  Existing C5/soak analyzer regressions: 46 passed.
- Stable Rust diff and harness changes independently reviewed. The review's
  Stop-boundary and unit-label findings were corrected and retested.

Compiler warnings in the completed video tests, feature-OFF release build and
feature-ON app check: zero. No new baseline compile was run, so a numerical
baseline/delta is not inferred. Feature-ON execution and the observer comparison
were completed below; the original soak failure remains unresolved.

Protected `target/release/syndocal.exe` remains SHA-256
`1d8e757ea8e7b1ffee8a51496d1c4ef2aa0399148a373c55313f252f1212d228`.
Unrelated Vite directories and prior failed/invalid evidence remain untouched.

## Native capture at `ba7be6d3`

The receipted artifact is `c5/build-ba7be6d3b48b-timing-2611090534/`, under
`target/qa/steroids-60fps-design-20261025/`. EXE SHA-256:
`16631dd06af60620d8889adaf59a5db725c7f56a1e82a81ae328f1fe26073e80`.
Committed source archive, all source files and nine pinned runtime files were
verified. Release build exited 0 with zero compiler warnings. The normal EXE
hash above is unchanged. Profile:
`jp.seraf.ktn.syndocal.qa.c5xba7be6d3t2611090534`.

The owner confirmed preparation before these five runs. All used the same
artifact, canonical grouped fixture, 2560×1600/180 Hz, resident GPU ON, 15 s
warmup and authenticated MCP control. Lighting stayed denied. Each run used a
new process; only the first used a fresh profile, so these are not four
independent cold-start comparisons. Every run exited 0 and independently
proved clean Stop, Standby/Ready/all-deny, empty routes/health, MCP teardown,
graceful close, no owned leftovers and free CDP port. No dialogs or forced exit.

### Observer OFF / ON / ON / OFF

Run IDs below are under `c5/runs/`, all ending `-ba7be6d3b48b`.

| Run prefix | Observer | Host FPS | Reconstructed work mean µs | Work deadline misses |
| --- | --- | --- | --- | --- |
| `20261109-072006-618176` | OFF | 59.9994 | 7142.4 | 6 |
| `20261109-072218-799116` | ON | 59.9852 | 6874.2 | 1 |
| `20261109-072429-207640` | ON | 59.9939 | 6871.5 | 1 |
| `20261109-072640-943122` | OFF | 60.0063 | 7067.3 | 4 |

All four 60 s windows passed V1–V9 and G1–G8. The campaign verdicts remain
`DIAGNOSTIC-NOT-GATING`, not C5 acceptance. The weighted work means were
7104.85 µs OFF and 6872.85 µs ON (descriptive difference −3.265%). No work-mean
increase was observed in these four runs. Integer lifetime means are only an
approximation; this is not proof of causation, zero overhead or an overhead
upper bound. ON's Presented p99 was 17.7 ms versus OFF's 17.6 ms, host CPU
averages were 12.95% versus 12.45%, and health roundtrips above 10 ms were
10/219 versus 4/220. Those differences also do not establish causation.

Both ON captures retained 2312 bytes, with 4525 admitted samples each and zero
matches above 33,333 µs. Their smaller 60fps deadline misses remain visible.
Partial, unobserved, invalid, incomplete, unprepared and overflow counts were
zero. Comparison details are retained in
`target/qa/native-spike-diagnostic-20261109/observer-comparison.json`.

### Thirty-minute diagnostic: prepare burst reproduced

Run `20261109-072858-337749-ba7be6d3b48b` measured 1800.028 s and 107981
Presented receipts (59.9885 host FPS; required 108000). It is deliberately
`INVALID` under the unchanged V6=60 s acceptance rule; every other V condition
passed. G1 and G3 were false. G2/G4–G8 passed. Native exit 0 proves diagnostic
and lifecycle completion, not 60fps acceptance.

The capture completed all 108987 attempted iterations: 61 excluded, 108923
fast, three matched. All partial/unobserved/invalid/incomplete/unprepared and
overflow counters were zero; retained bytes 2312. The three matched samples
were consecutive, same output 1 and validation epoch 2:

| G3 work sequence | JST 2026-11-09 start → sample | Work ms | Prepare ms | Present ms |
| --- | --- | --- | --- | --- |
| 73183 | 07:49:45.183801 → .454275 | 270.474 | 269.785 | 0.628 |
| 73184 | .454282 → .548708 | 94.425 | 93.646 | 0.756 |
| 73185 | .548717 → .586614 | 37.897 | 37.097 | 0.782 |

Prepare occupied 99.745% / 99.175% / 97.889% of each work sample. Pacing and
metrics-lock wait were zero µs in all three. Clock identity residual was
0/1/0 µs; bucket rounding/unassigned residual was 4/2/4 µs. This is wall time
in the coarse prepare interval, including its surrounding demand-counter and
evidence bookkeeping, not proved CPU execution or GPU time.

Raw statuses at 07:49:46.020 (lifetime count 73211, bounded count 600) and
07:49:51.148 (73518/600) both strictly contain all three work sequences and
report maximum 270474 µs / G3=false. At 07:49:58.235 (73944/600), the samples
have aged out and G3=true. The recovered final state does not erase failure.
Health at the same time recorded dropped slots 0→20, one interval above
100 ms and cumulative maximum 271.8367 ms. Individual receipt identity is
not carried by that histogram, so only time correspondence is established.

The burst overlapped a successful MCP renewal of about 1224.052 ms. Of 158
renewals, 31 were at least that long; renewal occupied 9.408% of the measured
window. This overlap does not prove a renewal cause. Decoder reset increased
303→306 across the surrounding status reads, but the same +3 recurred about
every 11–12 s with G3 passing elsewhere. Cache/session counts stayed 3, errors
and evictions stayed zero. Multi-layer cumulative decode wall time cannot be
assigned to an individual spike. Near-event host busy was about 21–23%; AC,
180 Hz and an unoccluded screen were maintained, without build processes or
another Syndocal. One-second whole-host samples cannot exclude a blocked/preempted
thread, a driver/GPU wait or short resource pressure.

Stop took 337.433 ms, graceful close 209 ms and cleanup 4958.644 ms. Capture,
source/runtime hashes, clock/sequence correspondence and cleanup were reviewed
independently. All raw data, preceding failures and unrelated Vite directories
remain preserved. No source or threshold changed during the five runs.

## Continuation boundary

The failure is narrowed to **prepare wall time**, with three genuine adopted
G3 samples and a coincident cadence disturbance. CPU/decoder/GPU/OS/renewal
attribution is still open; the older 35.604 ms spike is not proved to share
this cause. This is not a performance fix or a passing soak. Keep default OFF.

Next split the prepare interval at its engine-context/snapshot reads, previous
GPU ring completion, demand/decode acquisition, ISF submission/completion and
final settlement. Preserve exact G3 clocks and bounded Stop/ACK ownership.
Only after attribution and a justified fix can a fresh formal acceptance
campaign be considered. Physical scanout and release gates remain separate.

## Prepare phase summaries — implementation after `c0e62e4f`

The next diagnostic reuses the existing `span(Phase)` call sites on the
synchronous resident owner. Each selected record additionally reports
`phase_wall_us` and `phase_calls` in the explicit 31-entry `samples.phase_order`.
The existing 26 phases cover demand, retention, ISF submission/waits and
presentation. Five appended phases identify ContextCapture,
PresentationSnapshot, PresentationAuthority, ResidentRingResolve and
ResidentSettle. The app hooks only bracket existing calls; authority order,
Stop, rendering and compatibility policy are unchanged.

Phase times are **inclusive wall time**. Nested and repeated phases can
overlap; never sum these arrays as CPU work, GPU time or a critical path.
The collector uses a fixed 16-deep owner-local stack, generation/iteration and
non-reused serial tokens. Its guard is not Send. A stale, misordered,
overflowing or unfinished span produces explicit invalid/partial evidence;
it cannot charge another frame. An admitted frame's spans may complete after
Stop. Worker threads without the resident buffer record nothing. Disabled
mode touches no diagnostic TLS/clock. The fixed 16-record / 16 KiB bound and
original G3 sample Instant/pacing identity remain enforced.

The writer now emits only `native-work-spikes-2`; the V1 writer is removed.
The new cold reader rejects retired, future and malformed formats, exact phase
order/length/type mismatches and inconsistent duration/call accounting. It
does not interpret inclusive sums as exclusive time. Ordinary C5/soak gates,
the full trace's 3072/64/8 capacities and 8 MiB bound remain unchanged.

For V1 evidence above, preserve the original raw data and reports. To reanalyse
it, restore its matching reader from `C5_SPIKE_DIAGNOSTIC_2026-11-09.patch` into
a separate harness copy and use a **copy of the run**, never the protected
original. The current CLI refuses incompatible ON captures before report
writes; synthetic retired/future/malformed inputs proved unchanged hashes for
raw capture, `c5-result.json` and `SUMMARY.md`. The newer reader delta is
retained in `C5_SPIKE_PREPARE_2026-11-09.patch`, applied after the prior patches.

Evidence is under `target/qa/native-spike-prepare-20261109/`. The pinned
MSVC/PATH-first formal video test run exited 0: 30 passed, zero failed/ignored,
702 filtered, zero compiler warnings. This includes prior full-trace checks,
nested/repeated inclusive phase summaries, bounded depth, stale/misordered
tokens, incomplete spans, Stop/in-flight completion and disabled/no-owner
clock checks. Two earlier wrapper attempts were invalid and are not counted;
the accepted wrapper has all 14 lines encoded CRLF and verified the exact
linker before Cargo. Synthetic diagnostic/CLI protection tests passed, as did
the 46 ordinary analyzer regressions and a normal synthetic CLI reanalysis.

The feature-OFF `pnpm --dir app tauri build --no-bundle` completed successfully
in the reused isolated Cargo target with a new QA identifier. Its EXE SHA-256
is `b5fbab046b3b6ecad9e151e97fbd110fa9da71de953b4feb7a597d5eae06e5a1`.
`normal-native-proof.json` verifies one responsive maximized 2560×1600 Syndocal
window, a populated rendered root, denied lighting/video over three polls,
no live video outputs, normal exit 0, no force and free CDP 9287. The source
hash manifest remained identical before/after the build. All nine isolated
runtime identities and the protected normal EXE were verified.

`cargo check -p syndocal --features qa-native-frame-timing,ndi` also exited 0.
The frozen five-file Rust diff and cold harness received independent review.
For video feature tests, feature-ON app check and feature-OFF release build,
compiler warnings were previous-checkpoint baseline 0 / current 0 / delta 0;
no extra baseline build was run. The invalid wrapper attempts are excluded.

Actual V2 capture size, observer cost and prepare-internal attribution remain
unmeasured. Next build a receipted timing artifact from this committed source,
then compare observer OFF/ON before another long diagnostic. No passing
acceptance or root-cause fix is claimed.
