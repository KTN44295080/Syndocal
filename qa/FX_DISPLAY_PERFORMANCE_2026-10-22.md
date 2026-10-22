# Display FX performance diagnosis

Base: shared `main`, `2907a2eac7fc487251153f6aa9d258c64564f5c4`.
The initial checkpoint below was measurement only, **not a release pass**.
It changed no product implementation, output policy, schema or version.
The later bounded workspace change is recorded separately at the end.

The [previous runtime checkpoint](CHATGPT_RUNTIME_HANDOFF_2026-10-21.md)
recorded approximately 12.63 fps for an older native 3-layer/3-FX case, while
an isolated 1080p, one-chain ISF experiment took approximately 7 ms. The latter
is not a whole-output timing. The native fixture has three stages on **each**
of three layers (nine stages total), and actual output is 2560 x 1600.

## Current native measurements

Evidence root: `target/qa/fx-perf-2026-10-22/native/`.
Campaign: `31018a1c-21b3-4503-bed4-1130cc00db24`.

The unchanged source was built with the isolated campaign identifier and
`pnpm --dir app tauri build --no-bundle --config <campaign>/tauri.qa.conf.json`.
`build-01.log` records the exact MSVC 14.44.35207 pin and first-on-PATH linker;
`build-01.json` confirms unchanged HEAD/clean source around the build.
Copied QA executable SHA256:
`d41de3535a3c9465c0f9a13d3022411e349bc9d1895693759b5572385c48d5f2`.

Both fixtures use the same three enabled layers of the same silent 1920 x 1080,
30 fps H.264 file, and differ only in their layer ISF effects and output label.
Fixtures, media hashes, native process/CDP ancestry and main-window maximization
are recorded by the existing bounded performance harness. There were no
overlapping Cargo/rustc/link processes during measurement. Actual output geometry
was observed without a forced resize. Local native Display activation used its
actual confirmation dialogs; DMX routes remained disabled and Lighting permission
false. No audio input, camera, network video or external hardware was activated.

| Current source, actual 2560 x 1600 Display | 0 FX | 3 FX per layer |
| --- | ---: | ---: |
| Measurement interval | 19.486411 s | 59.491783 s |
| Native application submissions / interval | 1169 | 730 |
| Submission counter fps | 59.9905 | 12.2706 |
| Dropped-frame counter delta | 0 | 2840 |
| Prepare p50 / p95 | 0.6025 / 2.5894 ms | 78.3543 / 90.0274 ms |
| Native validation samples | 600 | 600 |
| Native frame-budget result | passed | **failed** |
| GPU composition layer slots | 3 | 0 |
| Decode wall mean per call over measured samples | 0.4795 ms | 20.9152 ms |
| Standby command wall time | 591.8931 ms | 583.4747 ms |

Reports: `<campaign>/perf-h264-1080-3l-01/evidence.json` and
`<campaign>/perf-h264-1080-3l-3fx-01/evidence.json`; derived `summary.json`.
Prepare excludes final GPU composition/present, so the two prepare columns do
not represent identical composition boundaries. Native counters establish local
application submission, not physical scanout or received FPS. The probe's
`passed: true` means observation and retirement completed; it does **not** turn
the FX frame-budget failure into a pass. The 0-FX 600-frame window had one
deadline miss, within its existing acceptance contract.

Both Stop operations returned Ready/Standby, denied both output domains, and
left no active routes/live windows. Both exact owned processes were closed via
their protected discard/stop dialog. Their fixture files were unchanged.
These single Stop observations are not latency ceilings against arbitrary
drivers or GPU/SDK stalls.

## Matched component measurements

A temporary `cfg(test)`-only harness ran two explicitly ignored release tests
with the same pinned compiler environment. Each command selected and passed
exactly one test; no wall-time assertion was added. After measurement, the
temporary module and its registration were removed from the product tree.
Its archived source is `native/temporary-cost-tests.rs`, SHA256
`789eb96414cb14c97ffe293070b2332844a363066e7b6e586566294f4b133ba4`.
The archived `.cmd` files require reinstating that temporary test registration
to reproduce; alone they no longer select tests in the clean product tree.

| Component, three layers, 12 warm samples | 1920 x 1080 p50 | 2560 x 1600 p50 |
| --- | ---: | ---: |
| Existing opaque Normal CPU blend, 8 row bands | 1.3922 ms | 2.8086 ms |
| Same CPU blend, 1 row band | 4.4785 ms | 9.6830 ms |
| Actual fixture's three-stage ISF per layer, all three layers, including input copies | 23.395 ms | 45.125 ms |

Logs: `native/cpu-cost-before.log`, `native/isf-matched-cost.log`.
The matched ISF p95 totals were 23.588 / 47.183 ms respectively. At actual output
size, individual apply medians were 12.925 / 12.531 / 12.834 ms; stack-runtime
medians were 12.348 / 11.797 / 12.264 ms. These include GPU upload, submission,
synchronous readback and host work; they are not isolated GPU kernel times and
exclude native decode/presentation. Do not sum percentiles or subtract these
separate experiments to invent an exact native time breakdown.

## Finding and next bounded work

The source and current counters agree: executing layer FX excludes native GPU
composition (`gpu_composition_eligible` / `try_prepare_gpu_composition` in
`crates/video/src/lib.rs`). The canonical preparation still batches stages
within each layer, but `isf_pass_runtime.rs` allocates upload/ping/pong/readback
resources and synchronously returns CPU pixels for each chain. The current
three-layer ISF boundary is substantially larger than the CPU blend alone.

The proposed opaque-row bulk-copy change was **not implemented**: its measured
upper scope was only about 2.8 ms of a roughly 78 ms prepare path. A simple GPU
eligibility relaxation was also not made: the existing path owns effect faults,
LastValid/blackout behavior and CPU/GPU frame boundaries, which need explicit
parity and failure proof before changing the contract.

Next: profile the matched chain's allocation/upload/wait/readback/copy phases,
then evaluate bounded GPU workspace reuse or a shared-device composition seam
with retained resource ownership, cancellation, budget, pixel, failure and
LastValid parity. Do not infer wasted work solely from zero lookahead hits:
`libav_decoder.rs` retains an earlier prefetched frame as the continuation's
terminal frame when the next request advances past it, along with the advanced
decoder session.

Web ChatGPT via Chat On Steroids performed one bounded read-only investigation
and stopped without edits, builds, app/device actions or extra workers. Its two
source candidates were independently checked by the supervisor and the
measurements above. Evidence: `native/web-investigation.txt` / `.png`,
conversation <https://chatgpt.com/c/6ad90ce4-df8c-83ec-9dad-973b3e7e3480>.

Rust warnings in the measured configurations: 0 (baseline 0, delta 0). The
existing Vite chunk notice remains 1 (baseline 1, delta 0). The original standard
executable was preserved and restored byte-for-byte, SHA256
`d493076c1fe137e523543cafaf3a7742d5ce52cf518aca0b264231c553adc371`.
Original profiles and previous evidence remain preserved. Hardware, other-OS,
network video, arbitrary-workload 60 fps and full release acceptance remain open.

## Bounded workspace reuse checkpoint

Base: shared `main`, `1d012e7bbad6ce2ec4bc8f9eba7af8fed4d5965b`.
Only ISF scratch allocation ownership changed; the CPU-to-GPU-to-CPU boundary,
effect execution, stage failure policy, feedback, LastValid and output authority
contracts remain in place. No product/schema version changed.

`isf_stack_workspace.rs` owns one input/ping/pong texture set, readback buffer
and sampler per synchronous ISF runtime. Each stack takes exclusive ownership;
only readable output, unmap, balanced GPU scopes and successful cleanup allow
return to the runtime. Failure drops the set. A host-size mismatch drops the old
set before constructing its replacement. Scratch is independent of feedback
instances, layers and shader fingerprints. The existing 512 MiB shared budget
still reserves `12 * width * height + padded_row * height` exactly once, on both
fresh and reused stacks; stage/transient accounting and LRU are unchanged.

Temporary test-only phase clocks identified repeated scratch construction as
a material cost. At 2560 x 1600, representative per-layer warm medians were
2.57 ms input allocation/preparation, 2.01 ms remaining workspace/encoder setup,
2.22 ms readback allocation/copy encoding, 0.60 ms upload, 0.81 ms GPU wait/map
and 2.46 ms host unpack. These are host phase timings from a component probe,
not GPU kernel timings or an additive native-frame decomposition. After reuse,
the matching setup phases were near zero; upload, GPU wait and host copy remain.

| Matched fixture, three layers, three stages each, input copies included | Before p50 / p95 | Reuse p50 / p95 |
| --- | ---: | ---: |
| 1920 x 1080 | 22.618 / 24.276 ms | 12.050 / 12.595 ms |
| 2560 x 1600 | 44.093 / 46.299 ms | 21.774 / 22.247 ms |

Component evidence: `native/isf-phase-before-02.log` / `.json` and
`native/isf-phase-reuse-01.log` / `.json`, each selecting one explicit ignored
release probe with four warmup and twelve measured iterations. The first
`isf-phase-before.log` attempt is invalid: mixed batch-file line endings broke
environment initialization and the fixture variable was absent; it failed its
one selected test and is not evidence. The corrected `-02` log verifies the
exact MSVC pin and first-on-PATH linker before Cargo. Temporary instrumentation
and its module registration have been removed from the product tree; archived
probe source and the original instrumented runtime remain under `native/`.

Focused release regressions: `isf-regression-01.log`, **72 passed / 1 ignored**,
including the three new scratch lifetime tests and existing pixel/clock/image/
audio/feedback/size-budget coverage. The ignored test is an existing explicit
timing experiment. The same exact compiled test artifact also passed the
`last_valid` filter (**10**) and `video_render_cancellation_tests` (**4**), with
one overlapping test; these are not 86 unique tests. Each direct test invocation
uses the pinned MSVC environment; `unit-artifact.json` binds the tested executable
and unchanged source hashes. None of these test successes is native Display FPS
acceptance.

Web ChatGPT through Chat On Steroids performed one bounded read-only design
investigation, then one independent review of the stable three-file source/test
diff and component evidence. It reported no definite P1/P2 issue and stopped.
The supervisor inspected the diff and actual selected test results. Evidence:
`native/web-workspace-review.txt` / `.png` and
`native/web-workspace-stable-review.txt` / `.png`, in the same conversation above.
Real OOM, device poll failure and near-limit GPU LRU stress are not claimed.

Native campaign: `03c6c891-16fe-436b-8208-fc295fda0ba8`.
The QA-identifier build used the exact pinned MSVC procedure and
`pnpm --dir app tauri build --no-bundle --config <campaign>/tauri.qa.conf.json`.
The build's three source hashes stayed unchanged. Both runs launched the built
checkout executable at `target/release/syndocal.exe` directly, required exactly
one responsive native main window, and verified maximization before control.
SHA256: `427d1d284f799328b15a6c5c3b2d3b18cfff53e70afecf61a8359d6c06256086`.
Fixture/media/descriptor/process ancestry and output safety checks are retained
from the existing harness; only its exact allowed executable path was tightened
from the earlier QA copy to this checkout's release executable.

| Reuse build, actual 2560 x 1600 Display | 0 FX | 3 FX per layer |
| --- | ---: | ---: |
| Measurement interval | 19.580469 s | 59.301346 s |
| Application submissions / interval | 1169 | 871 |
| Submission counter fps | 59.7023 | 14.6877 |
| Dropped-frame counter delta | 6 | 2688 |
| Prepare p50 / p95 | 0.7515 / 6.5692 ms | 75.1589 / 84.1431 ms |
| Native validation samples | 600 | 600 |
| Native frame-budget result | passed | **failed** |
| GPU composition layer slots | 3 | 0 |
| Stop command wall time | 553.4765 ms | 579.4313 ms |

Reports: `<campaign>/perf-h264-1080-3l-01/evidence.json` and
`<campaign>/perf-h264-1080-3l-3fx-01/evidence.json`, plus the corresponding
launch/build records and `nofx-01.log` / `fx-01.log`. The earlier source baseline
was 59.9905 / 12.2706 fps. These single bounded native observations show a smaller
end-to-end improvement than the isolated scratch probe; they do not establish
a repeatable speedup across other loads. The native FX budget still fails.
The no-FX validation window had one deadline miss and its measurement interval
had six dropped submissions; neither a perfect cadence nor physical scanout is
claimed. There were no decoder failures or native output errors in either run.

Both Stop operations returned Ready/Standby with both output domains denied,
no live windows/routes or health rows. Exact owned PIDs 26028 and 43240 were
closed via the protected Discard/Stop/Close dialog; fixture hashes stayed
unchanged. No unknown hardware, audio input, DMX, network video or macOS output
was opened. The tested build is preserved in `<campaign>/bin-workspace/` with
the seven pinned FFmpeg DLLs. The previous standard executable was then restored
byte-for-byte to its SHA256 recorded above; this restored executable does not
contain the workspace change.

Compiler warnings: Rust 0 (baseline 0, delta 0), existing Vite chunk notice 1
(baseline 1, delta 0). Direct invocations of the already-built unit executable
have no separate compiler-warning measurement. `final-audit.json` binds the
archive/restoration, source/fixture identities, stopped PIDs and build warning
counts. User-requested row-decoration work was delegated separately after the
FX measurement; its CSS is not embedded in this native test artifact.

Next bounded investigation: observe native ISF input dimensions, scratch
reuse/recreation counts and per-layer phase totals. The isolated warm savings
alone do not explain the remaining roughly 75 ms native prepare path. Preserve
the canonical rendering/fault paths and do not claim 60 fps completion.

## Native phase observation and takeover checkpoint

Base: `main` at `f6c5fad38f4b1b0d06bf9467076a1828064bca3d`, which includes
the committed scratch reuse implementation. Campaign:
`native/88baa1b5-514c-47d4-8626-21b0729a59e5/` under the evidence root above.
The inherited temporary trace was reviewed, measured, archived, and removed.
It did not change rendering eligibility, LastValid, cancellation, or fault policy.

The exact checkout release executable was built with the pinned MSVC 14.44
procedure and an isolated QA identifier. SHA256:
`a1891fbb02cdd5714d6fec10d94b02e9195b9b2878df0b23e07ba5b15810e725`.
`build-takeover-01.json` binds the unchanged three trace source files and binary.
The same executable, fixture, and media were used in fresh processes for trace
ON and trace OFF. The fixture contains three H.264 layers with three ISF stages
on each layer; every observed stack and the actual Display were 2560 x 1600.
The trace OFF child had the opt-in environment variable removed and produced
an empty stderr file. Both native runs required one responsive exact executable
instance and observed maximization before acquiring output control.

| Same executable, actual 2560 x 1600 Display | Trace ON | Trace OFF |
| --- | ---: | ---: |
| Measurement interval | 59.603023 s | 59.647925 s |
| Application submissions / interval | 1092 | 1181 |
| Submission counter fps | 18.3212 | 19.7995 |
| Dropped-frame counter delta | 2484 | 2399 |
| Native prepare p50 / p95 | 56.7685 / 62.5595 ms | 54.9651 / 60.5187 ms |
| Native validation samples / deadline misses | 600 / 600 | 600 / 600 |
| Native frame-budget result | **failed** | **failed** |
| Stop command wall time | 412.6158 ms | 413.4085 ms |

Evidence: `perf-h264-1080-3l-3fx-trace02/evidence.json` and
`perf-h264-1080-3l-3fx-off02/evidence.json`, with their launch records. Their
`passed` field means the guarded observation and Stop completed; it does not
override `frame_budget_pass: false`. Both runs had zero decoder/output failures.
Stop returned Ready/Standby with both output domains denied, no open output
windows, active transport routes, or health rows. PIDs 32520 and 38456 were then
closed through the protected Discard/Stop/Close dialog. Earlier `trace01` and
`off01` measurement attempts failed the maximization preflight before Arm and
remain preserved. Japanese-path helpers read JSON explicitly as UTF-8.

These are single bounded observations, not a controlled speedup claim against
the earlier 14.6877 fps campaign. The trace reached its hard cap before the end of the ON
run, so the ON/OFF whole-run difference is not a per-frame tracing cost estimate.
Neither the submission counter nor these checks establish physical scanout.

`trace-analysis-01.json` retains all 512 records and validates their correlation,
phase bounds, dimensions, stage counts, and monotonic scratch counters. Parent 1
is excluded as warmup; parent 897 is excluded because the cap cut off its third
stack record. The adopted sample is 55 complete parents / 165 stacks. All 170 recorded
stacks succeeded at 2560 x 1600. Runtime 1, at its last observed call 2690, had
**created 1 / reused 2689 / resized 0**; this is not a lifetime count after the
last sampled call. Scratch selection was below the trace's 1 microsecond
resolution on every adopted stack.

| Same-parent phase sum across three stacks | p50 | p95 |
| --- | ---: | ---: |
| Entire synchronous stacks | 18.310 ms | 19.401 ms |
| Map / poll / wait / receive | 9.101 ms | 10.325 ms |
| CPU unpack / unmap | 6.268 ms | 6.764 ms |

The separately scoped compose span was 7.041 / 7.844 ms p50 / p95. Percentiles
must not be added to reconstruct a whole frame. The wait bucket includes queue
completion and host wait; it is not GPU kernel time.

Synchronous stderr writes contaminate enclosing spans. Each span clocks its end
before writing its line, so layer FX includes stack log writes, the prepare loop
includes layer log writes, and outer spans include inner log writes. In
particular, the 23.627 ms FX residual and 11.693 ms prepare-loop residual are
**not** established product overhead. Trace Display p50 82.456 ms and native
prepare p50 56.7685 ms also have different boundaries and sample populations;
their difference cannot be used to subtract an exact observer cost.

Independent review found no P1/P2 issue in the temporary trace and verified
the evidence interpretation and removal patch. The pinned release ISF test run
passed **72 tests / 1 ignored** (`isf-regression-takeover-01.log`). Build warnings:
Rust 0 (baseline 0, delta 0); existing Vite chunk notice 1 (baseline 1, delta 0).
The measured executable, seven FFmpeg DLLs, three trace sources, and exact patch
are preserved in `trace-archive/`. Its manifest verifies both tracked Rust files
returned to HEAD and the temporary trace module was removed. Header layout
acceptance and the final standard executable are recorded separately in
[the header checkpoint](TOPBAR_LAYOUT_2026-10-22.md).

The diagnosis rules out repeated scratch recreation in this sampled workload.
The next bounded performance investigation should buffer records until after
the Display end timestamp, then separately measure the input
`Arc::unwrap_or_clone`, shader/control/stage-key preparation, queue retirement,
and output LastValid clone. Current evidence does not justify changing GPU
eligibility or LastValid policy. The 60 fps native FX budget remains unresolved.

## Buffered host attribution, 2026-10-22

This bounded observation continues from main/HEAD/origin/main
`491fb3bec909045c639a7891bfa72a1c99b5a3e5`. All three refs were rechecked before
closeout. The initial working tree was clean; the temporary Rust changes were
owned by this campaign, and their hashes remained equal to the successful build
manifest until removal. No product optimization or commit/push was performed.

Campaign: `target/qa/fx-perf-2026-10-22/native/6281a662-d969-45c2-9adc-3de1018d05c2/`.
The existing native harness, QA-only identifier/profile, and known silent H264
were reused. The profile identifier ends in this campaign UUID. Local Display
was 2560 x 1600, with three playing layers and three FX stages per layer; input
media was 1920 x 1080, 30 fps, 12 seconds, `has_audio: false`. Lighting remained
denied, with no DMX fixtures or external transport routes. MSVC 14.44.35207 was
both the explicit linker pin and first `where.exe link.exe` result. The successful
build was reused after the message-stream retry; neither build nor baseline
backup was repeated.

| Identity | SHA256 |
| --- | --- |
| Original and restored `target/release/syndocal.exe` | `cd4c3a1f8d3048a9407d086de48d44be37c2711295bfd91f48038cd2cc12516c` |
| Instrumented executable | `ec2db0667b308b79f96677b07ac9bf1419ef8612c214463760ea9990ab88177d` |
| Campaign `perf-h264-1080-3l-3fx.sdc` | `1ad7bed8a2e59b6e86ae4a23800bb70e5f39554f9991dc9befb697ae9e11dd75` |
| Known `h264_1080p30.mp4` | `53861258351df591b53450228d532c60fd914caa8de1d88b2029e480488e0c2b` |
| Complete on02 stderr trace | `4041cb6934fbcc1fe11f71f284b5813c3b10ad187a4151e00db5cb85541ae92e` |

The opt-in `SYNDOCAL_QA_ISF_NATIVE_TRACE=1` implementation records primitive
values in a thread-local bounded buffer: every 16th parent, at most 96 parents,
and at most 96 records per parent. The Display preparation span fixes its end
timestamp before formatting or stderr output. Children share that parent and
record start/end timestamps; the Display record is last, followed by an envelope
with record count, overflow flag and flush duration. Preallocation occurs before
the first sampled root timestamp. Clock/buffer overhead remains inside measured
spans. Rendering order, GPU eligibility, fault/LastValid behavior and cancel/Stop
contracts were left intact. The added scopes measure existing operations only.

`on01` failed before capture: Arm returned `forbidden` following a Desktop
identity/state change and a rejected stale-window input. Its exact rejection is
preserved; this observation does not establish a lease-expiry cause. The measure
exit was **1**, health samples **0**, and native stderr **0 bytes**. Its finalizer
returned Ready/Standby in **408.481 ms**, with no output windows/routes/health.
The process was closed, then a fresh `on02` process used the same successful
binary and a new attempt. No rejected invocation was redispatched.

`on02` completed the requested 60-second observation and Stop, exit **0**.
Evidence is `perf-h264-1080-3l-3fx-on02/evidence.json`, with both attempts' launch
records, measure logs and exit JSON retained. There are **104 health polls**;
the first-to-last counter interval is **59.2202822 s**, **979 deliveries**,
**16.531498 fps**, and **2573 dropped frames** during that interval. Final health
has 600 render samples and 599 presentation samples. Native validation has
**600/600 deadline misses**, `frame_budget_pass: false`, zero decoder failures
and zero lifetime output errors. The evidence `passed` flag covers guarded
capture and successful retirement only; it is not a frame-budget acceptance.

`buffered-analysis-01.json` validates **3672 span records / 68 complete parent
envelopes / 204 stacks**, with no buffer overflow or partial parent. All 68
parents have exactly 54 records: one each of Display/artistic/prepare/queue-sync/
LastValid/LastValid-clone, three each of layer/stack/chain/input-clone/stage-keys/
queue-push/queue-publish, and nine each of shader/control/image preparation.
The first parent is the only warmup exclusion. The adopted population is
**67 complete parents / 3618 span records / 201 stacks**, all at 2560 x 1600
with three stages and `status: ok`. Bounds, phase sums, child containment and
root/envelope end agreement pass. The 96-parent cap was not reached. At the last
sample, parent 1073, runtime 1 had **3219 calls: created 1 / reused 3218 /
resized 0**. These counters end at that sample, not at process lifetime end.

Each table row is computed per adopted parent before taking percentiles.
Multi-layer and multi-stage rows sum only their own records within that parent;
rows overlap where a span contains another span and must not be added together.

| Same-parent measured operation | p50 ms | p95 ms |
| --- | ---: | ---: |
| Entire traced Display preparation | 38.193 | 58.265 |
| Prepare prefetch | 2.141 | 21.060 |
| Prepare layer loop | 25.231 | 27.793 |
| Layer provider/decode access, three layers | 0.017 | 0.023 |
| Input `Arc::unwrap_or_clone`, three layers | 5.421 | 6.002 |
| Shader preparation, nine stages | 0.007 | 0.009 |
| Control preparation, nine stages | 0.000 | 0.002 |
| Image binding preparation, nine stages | 0.000 | 0.000 |
| Stage-key preparation, three chains | 0.000 | 0.002 |
| Entire three FX chains | 16.163 | 19.032 |
| Entire three synchronous stacks | 16.117 | 18.993 |
| Stack upload, three stacks | 1.942 | 2.295 |
| Stack encode, three stacks | 0.560 | 0.684 |
| Stack submit, three stacks | 0.339 | 0.450 |
| Stack map/poll/wait/receive, three stacks | 6.427 | 9.446 |
| Stack CPU unpack/unmap, three stacks | 6.600 | 7.245 |
| Queue synchronization/retention | 0.011 | 0.013 |
| Queue push including retirement, three layers | 1.713 | 1.916 |
| Queue publish, three layers | 0.000 | 0.001 |
| Compose | 7.005 | 7.583 |
| Output LastValid lookup/clone/replace | 2.387 | 2.729 |
| Output LastValid clone alone | 1.779 | 2.031 |
| Chain runtime residual after contained stacks | 0.009 | 0.011 |
| Layer residual after decode/input/chain/queue scopes | 1.762 | 2.129 |
| Prepare-loop residual after contained layers | 0.005 | 0.006 |
| Post-Display-end trace flush, excluded from root | 221.160 | 241.118 |

Residuals are differences of matched durations for each individual parent,
followed by percentiles; no independent percentiles were subtracted. The small
layer provider/access time does not include the separately measured prefetch
work and is not total decode time. Zero entries are microsecond-resolution
observations, not proof of zero cost. The wait span includes host waiting and
queue completion, not isolated GPU kernel time. Input ownership/clone,
queue retirement and output LastValid have measurable host costs; shader,
control and stage-key preparation do not dominate this sampled workload.
The remaining layer residual is still unattributed. This evidence supports a
later ownership/copy-lifetime investigation, not an unmeasured behavior change.

Synchronous log flush now occurs after the traced root end, but still stalls
the sampled render iterations. Its p50 **221.160 ms** and p95 **241.118 ms**
make the observed **16.531498 fps unsuitable for standard-build acceptance or
a speedup/regression comparison** with the historical approximately 19.8 fps.
There was no new trace-OFF run in this unit. Native health preparation p50/p95
was **39.3355 / 257.0376 ms**, with a different boundary/population from the 67
traced parents. Neither that difference nor earlier trace percentiles are used
to subtract an observer cost. The traced root is the Display preparation seam,
not final GPU composition, present completion, or physical scanout.

| Check | Exit / result |
| --- | --- |
| Campaign/baseline preparation (session 11597) | 0 |
| Existing harness fixture preparation and source diff check (39209) | 0 |
| Analysis helper syntax check | 0 |
| Pinned native build (27454), `build-buffered-host-01.json/log` | 0 |
| on01 launch / maximize (19470 / 30605) | 0 / 0 |
| on01 measure (33359), `measure-on01-exit.json` | 1, Arm rejected before capture |
| on02 launch / maximize (10437 / 84275) | 0 / 0 |
| on02 measure (99138), `measure-on02-exit.json` | 0 |
| Buffered analysis (54252) | 0 |
| Exact app / measurement-node / compiler absence check (82133) | 0, none found |
| Archive/removal/exe restoration (40600) | 0 |
| Hashes, refs, sample counts, warnings, helper archive, diff check (89879) | 0 |
| Final diff check / Rust diff / status / stat / exe hashes / process absence (56982) | All six exits 0; only this QA file modified |

Build warnings: **Rust 0 (baseline 0, delta 0); existing Vite chunk notice 1
(baseline 1, delta 0)**. The warning/linker scan also exited 0 (5385). This unit
did not rerun the earlier 72-test/1-ignored suite, fault injection, or a new
standard-build runtime acceptance. No independent worker/swarm was used.

Stop on02 returned **Ready/Standby in 413.568 ms**, lighting/video both denied,
no open live/test-pattern windows, no active routes and no health rows. Both
PIDs **38672 / 36248** were closed through Discard, Stop and Close. No matching
app, measurement node or compiler remained. `closeout.json` and
`final-checks-01.json` preserve restoration checks, all source/executable/helper
hashes, warnings and sample counts. `buffered-archive/` contains the measured
exe, seven FFmpeg DLLs, all three temporary Rust sources, exact tracked patch
and copies of the harness/helpers. Raw capture/build/exit evidence remains in
the same campaign; the original `baseline/` was preserved unchanged.

The two Rust files match their saved original bytes, `isf_native_trace.rs` has
been removed, and the standard executable matches the original SHA256 above.
The final tracked difference is this QA addition only. Git's final diff checks
also emitted the working-copy LF-to-CRLF conversion notice; this is separate
from the unchanged build-warning counts above. No other
machine, unknown DMX/audio device, external output or macOS was exercised.
Physical scanout, controlled observer overhead, remaining layer residual and
the 60 fps native FX budget remain unverified or unresolved.

Supervisor closeout review inspected the archived temporary diff and trace
module, the same-parent analysis implementation and results, the Stop evidence,
and this QA addition. No blocking finding was identified for this diagnostic
checkpoint. The tracked runtime files are unchanged; the restored executable
hash was independently checked. This review does not accept the native frame
budget or substitute for review of a future runtime optimization.
