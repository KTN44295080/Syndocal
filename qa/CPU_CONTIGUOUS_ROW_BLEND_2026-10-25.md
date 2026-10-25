# Contiguous CPU composition rows, 2026-10-25

Base: `f0d9d66d5e2df59978a72871c2ebf2ff01327db7`, shared `main`.
This unit follows the adopted borrowed-input and packed ISF readback changes.
It changes CPU composition only: the existing axis-aligned sampler lookup
admits a row slice only when every covered source pixel is consecutive, with
no holes, repeats or skips. Other lookups retain the indexed loop. The row
helper preserves the existing alpha, opacity, blend table and pixel blend
branches; a completely opaque Normal/opacity-one row can use a slice copy.
Crop coverage and floating-point sampling decisions still come from the
existing sampler, including its rounding. Color/FX adjustments keep their
existing eligibility rules. No GPU-composition admission, ISF stage, fault,
LastValid, cancellation, schema or output-authority behavior is changed.

## Focused release checks

Evidence: `target/qa/isf-next-20261024/`, with per-run source hashes and exit
records. All three commands used `cargo test -p video --features libav --release`
under pinned MSVC **14.44.35207**, with the absolute linker verified first on
PATH. All ran on October 25 UTC, at the base above plus this owned diff.

| Filter | Passed | Failed | Ignored | Exit |
| --- | ---: | ---: | ---: | ---: |
| `axis_aligned_composite_tests` | 12 | 0 | 3 | 0 |
| `axis_aligned_contiguous_row_cost --include-ignored --nocapture` | 1 | 0 | 0 | 0 |
| `c1_` | 27 | 0 | 0 | 0 |

The pixel comparisons cover partial alpha at row start/middle/end, crop and
translation, NaN opacity, blend modes, table-sized layers and 1/3/8 row bands,
alongside the existing generic/axis-aligned/table parity checks. The ignored
tests in the first command are two explicit timing experiments and the existing
exhaustive all-2^32-float-pattern check. Only the new row timing experiment was
explicitly requested; the other two were not run. C1 includes output LastValid
identity/remapping, blackout, stage-local failure and composition ordering.
Rust warnings: current **0**, recorded baseline **0**, delta **0**.

The initial ignored apply helper stopped before tracked writes because its
prefix guard expected one extra blank line. Exact original-test bytes were
verified as the draft prefix, the guard was corrected, and application passed.
No product assertion or acceptance test was weakened.

## Matched CPU cost experiment

`cpu-cost-01.log` compares the exact pre-change indexed axis-aligned loop with
the new actual dispatcher, in the same release test process: 2560 x 1600,
three layer blends, default transform, identical destination/source pixels,
same table construction and Rayon band policy. Each case uses 12 ABBA rounds;
the first two rounds warm up, leaving **20 samples per path**. Destination reset
and exact output equality checks are outside timing. Values measure the whole
three-layer CPU operation, not single-row memcpy or native FPS.
The reported p50 is the 11th sorted sample (upper middle), and p95 is the 19th.

| Source / blend / opacity | Before p50 / p95 ms | After p50 / p95 ms |
| --- | ---: | ---: |
| Opaque / Normal / 0.7 | 6.932 / 7.964 | 4.885 / 5.247 |
| Opaque / Normal / 1.0 | 3.058 / 3.573 | 1.806 / 2.008 |
| Opaque / Add / 0.5 | 7.688 / 8.472 | 5.019 / 5.569 |
| Mixed alpha / Normal / 0.7 | 23.642 / 25.774 | 22.682 / 23.049 |
| Last pixel of every row partial / Normal / 1.0 | 3.094 / 3.802 | 2.192 / 2.544 |

The first case matches the current FX fixture's Normal/0.7 layer policy and
shows about **29.5% lower median CPU composition cost**. The last case exercises
the unfavorable complete alpha scan followed by per-pixel fallback; it did not
regress in this bounded experiment. These synthetic results are not a general
speed guarantee or an isolated end-to-end native FPS improvement.

## Native acceptance and review

`pnpm --dir app tauri build --no-bundle` passed under the pinned wrapper using
the isolated absolute target `target/readback-copy-20261024` and a new QA
identifier `jp.seraf.ktn.syndocal.qa.output-authoring.44af4539-a14d-4a2a-a5f7-2dcea85a997b`.
The build record pins all six relevant source hashes, including the existing
readback/Cow files, and verifies them unchanged after building. The executable
SHA256 is `b6728b7e9fc90212f8cef1c6ebfe80819c83077d5de3aa3f022e7a2fc622b88c`.
Rust warnings: **0**; existing Vite large-chunk warnings: **1** (baseline 0/1,
delta 0/0). This was an internal no-bundle check, not a distributed artifact.

Campaign `target/qa/isf-next-20261024/44af4539-a14d-4a2a-a5f7-2dcea85a997b/`
contains `build-01`, `launch-01`, `preflight-01`, the raw performance evidence and
normal-close records. Exactly one responsive `Syndocal` window was verified
for PID **27856** / HWND **46534300**, maximized before output actions (CSS
1280 x 752, DPR 2). The known silent H264 1920 x 1080 fixture ran three layers
with three stages each, local Display at actual **2560 x 1600**, trace/log OFF,
five-second warmup and a 60-second capture request. The fixture SHA256 remains
`1ad7bed8a2e59b6e86ae4a23800bb70e5f39554f9991dc9befb697ae9e11dd75`;
media is the same 30 fps, 12-second silent clip recorded in the prior FX QA.

| Native observation, October 25 UTC | Result |
| --- | --- |
| Capture start / finish | 01:01:51.908 / 01:02:53.423 |
| All saved polls | 103 |
| First-to-last deliveries / elapsed | 1934 / 59.6710511 s |
| Counter FPS | **32.4110262** |
| Final rolling render wall p50 / p95 | 27.4437 / 44.8616 ms; excludes GPU composition/present |
| Final rolling delivered frame p50 / p95 | 29.4989 / 47.0044 ms |
| Final bounded native deadline misses | **600/600; budget pass=false** |
| Decode/session/hardware/output errors | 0 |
| Stop command wall time | **696.1734 ms** |

The saved harness report records `passed=true` for deliveries, error and bounded
Stop checks; no separate capture-process exit receipt is retained in the campaign.
Its `passed` flag does **not** mean the frame budget passed. The source includes Cow and
readback changes as well as row composition, and there is no matched native
before/after pair for this unit. Do not derive an isolated row-blend FPS gain
from the earlier 28.7 fps composed capture or archived 20/29 fps observations.
The 60 fps goal remains unmet; this capture also does not establish physical
scanout, long-run stability, other workloads, devices, platforms or releases.

Stop returned Ready/Standby with lighting/video disallowed, Display closed,
empty active routes and empty health. The app was then closed through its
normal UI, discarding only the dedicated QA project's in-memory state. All
seven recorded body/child PIDs and port 9273 were verified gone, with no compiler
running and stdout/stderr empty. The protected normal executable plus seven
DLLs remain byte-identical (8/8) to the prior restoration manifest. Normal exe
SHA256 remains `232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`;
it is not claimed to contain this new source checkpoint.

Chat On Steroids independently reviewed the stable applied source, matching
the draft SHA256 values (lib `9bdc9710...`, row helper `73a77a9a...`, tests
`e3eba851...`), with no blocking correctness finding. The full review, including
an independent alpha/helper check, is retained in
`target/qa/steroids-row-review-20261025/REVIEW-9bdc9710.md`.
Its separate log/QA check confirms actual selected test names, counts, exit
codes, pinned linker and before/after source hashes, without rerunning tests.
CPU percentile evidence is the experiment's aggregate output and verified
selection code; individual timing samples were not retained for independent
percentile recomputation. The full native review is retained in the same folder's
`NATIVE-01.md`: all 103 polls, source/build identities, Stop, saved close records
and current protected runtime hashes agree. Its required correction to distinguish
the saved harness result from a separately retained process-exit receipt is
applied above. No source or runtime blocker was found and no rerun was needed.

This bounded change reduces measured CPU composition cost. Remaining work is
the ISF transfer/wait/readback and decoder scheduling share of full-frame time;
the GPU-composition exclusion for executing FX remains intact to preserve
existing failure and LastValid contracts. No unknown DMX/audio hardware,
external transport, physical output test or macOS receiver was exercised.
