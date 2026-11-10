# Native resident Display soak — 2026-11-08

**Result: FAIL, 29/30 minutes passed.** Minute 13 exceeded the existing native
600-work-sample maximum: 35,604 µs versus 33,333 µs. Every minute remained valid
and passed the fixed delivery-rate and cadence gates. Memory/queue bounds,
lease renewal, shared boundaries and clean Stop/exit passed. This is a retained
performance failure, not an observation-invalid run or release acceptance.

## Scope and artifact

Base: `main` at `f02a2937198d2be410266c7b3e45e1cc846c4609`. Product source is
unchanged from the earlier [C5 comparison](NATIVE_60FPS_COMPARISON_2026-11-08.md).
No product/harness code, threshold, default route, profile history or receipt
was changed. The owned changes are this note and the earlier note's clarification
of shell versus native exit codes.

Artifact paths below are relative to
`target/qa/steroids-60fps-design-20261025/c5/`; retain these evidence directories.

- Final build: `build-f02a2937198d-normal-2611082035/exe-receipt.json`.
- EXE SHA-256:
  `bb567f8099baca89a268299a527f646cc8535250623bc17caa943fa31f757b35`.
- QA identifier: `jp.seraf.ktn.syndocal.qa.c5xf02a2937n2611082035`.
- Maintained Tauri no-bundle build, pinned Build Tools MSVC 14.44.35207 and
  inventory-verified FFmpeg/NDI runtime; committed source verified unchanged.
  Previous preparation build warnings 0, final build warnings 0, delta 0.
- Same fixed group-FX fixture as the comparison: three H.264 1080p layers with
  three ISF stages each, transformed Screen group; fixture SHA-256
  `bc59772e5b3b6c8ca0422f74df7f98675642f97cd92a910b68dd3b2db9f736cd`.
- Borderless native Display 2560×1600 / 180 Hz, AC power, trace OFF, normal
  non-timing executable. Resident ON is explicitly opted in by the runner.
  Video allowed, lighting denied; no lighting fixtures, DMX disabled, no audio
  or timeline play.

## Prerequisite comparison and retained invalid attempt

The first fresh build `build-f02a2937198d-normal-2611081942` was consumed by
`runs/20261108-202147-604408-f02a2937198d`: **INCOMPLETE**. All three ON windows
passed, but OFF s04's ending health read took 28.9311 ms, above the fixed 10 ms
V8 boundary. All six cleanup results were clean. Its shell returned nonzero;
the native numeric exit was not captured separately. Its raw data and failed
campaign remain unchanged, and its profile was not reused for soak.

A full ABBAAB was collected again with the new 2035 artifact/profile, without
changing the protocol or replacing individual windows:
`runs/20261108-205025-309230-f02a2937198d`, **C5 PASS, native exit 0**.
All six sessions completed, all windows were valid, and all teardown checks
passed. ON: 59.9949 / 59.9969 / 60.0053 fps; every G1–G8 passed. OFF remained
below target (mean 7.613 fps). Four ON Presented intervals exceeded 20 ms, so
this is not a zero-late-interval claim.

Soak dry-run `dry-run/dry-run-20261108-210355-070317.json` passed with
`exe.qa_profile_clean_after_passing_c5=PASS`: all six existing ledger entries
were bound to this exact EXE/hash and retained C5 PASS with raw clean teardown.
Independent review confirmed qualification before the soak. A taskbar warning
was retained; live V1 checks subsequently proved no occlusion in every window.

## Thirty-minute result

`runs/20261108-210413-233249-f02a2937198d`, 21:04–21:35 JST:
**C5 SOAK FAIL, native exit 1** (explicitly propagated by the shell command).
The same app, health writer, MCP principal and output remained alive throughout.

| Criterion | Observed result |
| --- | --- |
| Duration / windows | 1800.015164 s; 30/30 ordered and valid |
| Passing minutes | 29 PASS; minute 13 FAIL only at G3 |
| Rate, G1 | All 30 pass the unchanged `N ≥ floor(60×T) − 1` rule |
| Cadence, G2 | All 30 pass; whole-minute p95 ≤17.5 ms |
| G4–G8 | All 30 pass; no reported error deltas, intended resident path and identity/async bounds proved |
| Shared boundaries | All 29 S1/S0 pairs identical; gaps exactly 0 ms |
| Renewal | 150 successful renewals; maximum unrenewed gap 16,256.45 ms, bound 30,000 ms |
| OS working-set growth | Maximum +108,544,000 bytes; bound +134,217,728 |
| OS private-byte growth | Maximum +109,592,576 bytes; bound +134,217,728 |
| OS peak-working-set growth | +18,780,160 bytes; bound +134,217,728 |
| Pending / retirement high-water | 1 / 0; no growth |
| HAP / libav / session / CLI cache high-water | 0 / 3 / 3 / 0; no growth |

The complete per-minute rows, boundaries, memory series, renewal proof and gate
results are retained in `SUMMARY.md`, `c5-result.json` and the session's raw
`window-1.json` through `window-30.json`. Do not average away minute 13.

### Minute 13: observed work spike, cause unknown

Window 13 lasted 59.9899 s and delivered 3598 frames, meeting required 3598.
All V conditions and G1/G2/G4–G8 passed. Presented p95 was 17.3 ms, maximum
41.0 ms (0.1 ms histogram bin upper edge), with one interval over 20 ms.
Presented cadence and native work are different measurements; these two maxima
are not proved to describe the same frame.

Raw status observations:

| JST | Native budget/state | Work mean / maximum µs | Misses / samples |
| --- | --- | --- | --- |
| 21:17:17.877 | true / passed | 6554 / 11372 | 0 / 600 |
| 21:17:22.926 | false / failed | 6637 / 35604 | 2 / 600 |
| 21:17:28.087 | false / failed | 6692 / 35604 | 2 / 600 |
| 21:17:33.128 | true / passed | 6602 / 11929 | 0 / 600 |

The maximum exceeds 33,333 µs; the mean and 2/600 miss rate remain within their
existing bounds. The final minute-boundary snapshot recovered, but the retained
intermediate failures correctly keep G3 false. CPU/GPU/decoder/OS attribution
is not established by this trace-OFF capture.

`validation_reason` retained the same older `resident output 2 is in flight`
message before, during and after these observations, including passed states.
`append_observation_sample` recomputes the bounded work window and validation
state without refreshing that string. It is not evidence that the in-flight
message caused this spike. No gate or reason field was edited to obtain a pass.

## Cleanup and continuation boundary

Stop fulfilled in 315.55 ms. Standby/Ready denied video and lighting; Display,
health, routes and sources cleared. Lease relinquishment, principal revocation,
credential removal, sidecar closure and graceful app exit all passed. No dialog,
forced termination, cleanup error or leftover owned process; CDP 9285 was free
in both saved cleanup and the post-campaign OS check.

Protected normal EXE SHA-256 remained
`1d8e757ea8e7b1ffee8a51496d1c4ef2aa0399148a373c55313f252f1212d228`.
Unrelated Vite directories were untouched. The harness and acceptance constants
remain at the reviewed hashes. No additional compiler measurement is claimed
for the subsequent documentation-only update.

OS memory bounds are a proxy, not GPU-byte boundedness: GPU-ledger bytes and
GPU-history/FrameQueue occupancy remain unexposed. Physical scanout, native
fault/clock/loop and venue/release gates remain separate.

The thirty-minute acceptance is **not complete**. Keep resident default OFF and
retain the spike and all preceding invalid attempts. Next work is a separately
identified diagnostic capture of the work spike before attempting adoption or
another acceptance run. This used profile cannot qualify for another soak under
the current retained-history rule; do not delete its ledger to manufacture reuse.

## 2026-11-09 rerun after the live-tick stall fixes (Claude)

Product: `c58d7b45` (live resident Display may hold a layer's frame instead
of waiting on a behind look-ahead) + `9d57a91d` (resident Display surface
uses Mailbox when supported, AutoVsync otherwise). Receipted normal build
`c5/build-9d57a91d828c-normal-2611091812/`. Same group-FX fixture,
2560×1600 / 180 Hz, MCP control, lighting denied, unchanged gates. The
operator's Claude and ChatGPT windows were minimized for the run (an earlier
attempt, `c5/runs/20261109-174422-474889-9d57a91d828c`, is INVALID only
by V1 occlusion from a ChatGPT window; its ON windows passed every gate).

- **C5 PASS** — `c5/runs/20261109-182338-497390-9d57a91d828c`: ON
  59.9966 / 60.0088 / 59.9955 fps, whole max 19.0 / 18.7 / 24.0 ms, 0/0/1
  intervals >20 ms, every G1–G8 PASS; OFF 7.63 fps (expected baseline FAIL).
- **Soak FAIL (resource growth only)** — `c5/runs/20261109-184818-256747-9d57a91d828c`:
  30/30 windows valid and PASS (min 59.9844 fps, G3 work budget passed in
  every minute, max interval 33.4 ms in minute 10, ≤2 intervals >20 ms per
  minute). Failed only `resource_growth_bounded`: private-byte high-water
  +142,065,664 B over minute 1 (bound 134,217,728); working set +129,044,480
  (pass). Queue/cache/session/retirement growth 0. Live holds: 0 in every
  window, so the hold path did not run.

The memory is not higher than the 11/08 run that passed: private high-water
range 2077.8–2219.8 MB (median 2095.2) versus 2082.7–2227.0 MB (median
2139.8) then. The difference is the baseline: this run's minute 1 was its
lowest minute (2077.8 MB), 11/08's minute 1 was mid-range (2117.4 MB). The
gate is unchanged and the verdict stays FAIL; headroom against a
first-minute baseline needs either a lower sawtooth peak or a repeat run.

## 2026-11-10 rerun after lowering the loop-wrap memory peak (Claude)

Product: `b7deae29` caps a libav look-ahead run at three frames (was four).
A headless 3-layer group probe (`target/qa/task-mem-1109/`, allocation meter
plus the native GPU Display bench) attributed the private-byte sawtooth to
the three 12 s clips wrapping at the same moment: each wrap briefly held
look-ahead frames for the old and the new run, about +6 output-sized frames
(~16.4 MB each) for ~200 ms. With three frames the wrap peak is +3 frames and
work time is unchanged; three still covers the next request plus two slack
frames at 60 Hz with a 30 fps clip. A clip-start predecode cap was also tried
and reverted (no measurable effect). Receipted normal build
`c5/build-b7deae29754c-normal-2611101805/`; same fixture, 2560×1600 / 180 Hz,
MCP control, lighting denied, unchanged gates, Claude/ChatGPT minimized.

- **C5 PASS** — `c5/runs/20261110-181703-050208-b7deae29754c`: ON
  60.0064 / 60.0008 / 59.9940 fps, whole max 20.1 / 20.1 / 20.7 ms, one
  interval >20 ms per ON window, every G1–G8 PASS; OFF 7.23 / 7.36 / 7.38 fps
  (expected baseline FAIL).
- **Soak PASS** — `c5/runs/20261110-183023-404704-b7deae29754c`: 1800.0 s,
  30/30 windows valid and PASS, min 59.988 fps, max rolling p95 17.61 ms,
  worst whole-window max 33.8 ms (minute 16), ≤4 intervals >20 ms per minute,
  150 lease renewals (max gap 15.5 s / 30 s bound). `resource_growth_bounded`
  True: private +102,916,096 B over minute 1 (bound 134,217,728), working set
  +102,883,328, OS peak working set +23,023,616; queue/cache/session/
  retirement growth 0.

Private high-water range 2077.6–2180.5 MB (median 2097.8) versus
2077.8–2219.8 MB (median 2095.2) on 11/09: the minute-1 baseline is the same,
the peak minute is 39 MB lower, and the headroom against the bound is now
31.3 MB instead of −7.8 MB. The remaining sawtooth (+~100 MB at a few wraps)
is the old/new decoder session overlap at a simultaneous wrap; it is bounded
and non-monotonic. 60 fps native resident Display acceptance for 1.2.0 is
met on this host with these thresholds.
