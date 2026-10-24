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

## Deferred FX input ownership, 2026-10-22 (performance acceptance unresolved)

This section records the **unadopted 2026-10-22 checkpoint**. Its contrary
measurement remains evidence; the separate 2026-10-24 source-adoption decision
and its limits are recorded at the end of this document.

Base: `cfe8ca56a18873ebd0d967b74d73b337e6f31763`; main/HEAD/origin/main
matched and the tree was clean at entry. This unit changes `crates/video/src/lib.rs`
and adds `crates/video/src/isf_borrowed_input_tests.rs`, plus this QA entry.
No other dirty files or overlapping edits were observed. One independent,
read-only reviewer inspected the frozen product/test diff and actual test logs.
There were no blocking product findings. The single P3 documentation finding
was fixed by moving the previous diagnostic closeout above this section.

The shared-frame FX path now starts with `Cow::Borrowed` and passes it through
Clip then Layer. A successful ISF operation supplies its own owned output,
avoiding the previous eager `Arc::unwrap_or_clone` before FX. If the chains
return the borrowed input (empty/bypassed/disabled or preparation failure),
ownership conversion occurs only at queue insertion. That fallback still uses
`Arc::unwrap_or_clone`, so an exclusively owned input can move without copying.
The existing owned-frame entry point is a thin `Cow::Owned` adapter into the
same canonical implementation; it returns `into_owned()` at its boundary.
There is no duplicate hot-path implementation.

ISF runtime internals are unchanged, including their all-runtime-failure return
and stage-local bypass. All existing cancellation checkpoints, stage preparation,
fault mapping, FRAMEINDEX/feedback handling and Clip/Layer order remain in place.
The public VideoFrame/schema/API, GPU eligibility, queue insertion/retirement,
LastValid policy and input-retention limits were not changed. The source Arc is
held only for the current loop iteration; no new cache or persistent source
reference is introduced. Borrowed source pixels/metadata and reference counts
are checked after both success and failure.

Evidence directory:
`target/qa/fx-perf-2026-10-22/native/borrowed-input-20261022-033854/`.
`tests-complete-03.json` and `review-checks-03.json` record the frozen source
hashes, complete executed test names, log hashes, warning comparison and exe hash.
All accepted runs use `cargo test -p video --features libav --release FILTER --
--test-threads=1`, after `vcvars64.bat -vcvars_ver=14.44`. Every accepted log
verifies the explicit MSVC **14.44.35207** linker pin and matching PATH-first
`where.exe link.exe` result.

| Final filter / log | Passed | Failed | Ignored | Filtered out | Exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| `isf`, `isf-03.log` | 76 | 0 | 1 | 280 | 0 |
| `c1_`, `c1_-03.log` | 27 | 0 | 0 | 330 | 0 |
| `video_render_cancellation_tests`, corresponding `-03.log` | 4 | 0 | 0 | 353 | 0 |

These are **107 successful test invocations / 105 distinct passing tests**;
two C1 ISF tests appear in both filters. The sole ignored test is
`isf_targetless_batch_timing_experiment`, whose existing reason is "explicit
matched GPU timing experiment; no wall-time assertion". It was not used as
performance evidence. All four new tests actually ran and passed, including:

- Empty, bypassed, disabled and all-prepare-failed returns: full pixel/metadata
  equality, stable fault scope/stage identity, borrowed allocation retained and
  owned-input allocation preserved through the compatibility adapter.
- All runtime stages failing on invalid DATE: unchanged input pixels, identical
  faults through owned/borrowed paths, and exactly one FRAMEINDEX consumed before
  the next successful shader result.
- A shared-provider integration with noncommuting Clip add-red then Layer invert:
  expected pixel **235/235/225/255**, later-stage execution after a prepare fault,
  stable Clip fault identity, no LastValid publication from faulty output and
  unchanged decoder frame/reference count. A missing Layer projection explicitly
  keeps the Clip result **20/20/30/255** and the existing Layer resolution fault.
- Cancellation before processing and after GPU work: Cancelled is propagated,
  shared pixels remain intact and no queue/LastValid output is published.

The selected existing tests also cover feedback/instance/resize boundaries,
scratch readback failure, resource eligibility, image bindings, invalid chains,
prepare-failure clock progression, output fault freshness and cancellation
without CPU retry or LastValid/black fallback. No new normal-path-only or
wall-time acceptance assertion was substituted for these contracts.

Failure evidence remains intact. Initial runner preflight (session 68413)
exited 1 before Cargo because Git could not resolve the intended working
directory; the runner now derives the repository from its script path.
The first batch had LF-only endings and was misparsed by cmd, bypassing the
intended environment setup. That invalid run (54173) was explicitly terminated
through its exact owned runner PID 39932, exited 1, and executed zero tests;
its `test.cmd` and `isf-01.log` are retained and are not accepted evidence.
Process absence was verified before starting a new attempt. The new batch
copies the existing working harness with CRLF endings (`harness-correction.json`).

The first correctly pinned suite, `isf-02.log` (94224, exit 101), ran **75 passed /
1 failed / 1 ignored**. The new integration fixture omitted the required matching
legacy projection for its canonical Layer chain. The existing fail-closed
resolver therefore rejected Layer, leaving the correct Clip-only pixel.
The fixture now supplies the matching invert projection; the original expected
235/235/225 pixel and fault-identity assertions are retained. The missing-projection
case is also asserted explicitly. The supervisor independently identified the
same fixture issue. Product resolution policy was not relaxed. The corrected
full selected suite completed in session **61891, exit 0**; aggregation/source
hash/exe/diff checks completed in **36934, exit 0**.

Rust warnings: **baseline 0 / current 0 / delta 0** for the release+libav test
configuration. The baseline is the existing
`88baa1b5-514c-47d4-8626-21b0729a59e5/isf-regression-takeover-01.log`;
all three final logs were scanned. Frontend/Vite was not part of those Rust tests.
The ordinary `target/release/syndocal.exe` was frozen and saved with SHA256:
`cd4c3a1f8d3048a9407d086de48d44be37c2711295bfd91f48038cd2cc12516c`.
No measured speed improvement or 60 fps claim follows from these Rust checks.

The user subsequently authorized the native gate, independent review and owned
commit/push in this lane. Native campaign:
`target/qa/fx-perf-2026-10-22/native/aa0ed288-4228-4c0e-9879-805cb30f8a5e/`.
It preserves `original-runtime/` (exe plus seven DLLs), `candidate-source/`,
`baseline-source/`, per-attempt build/capture logs and process/session records.
Both variants use one QA-only identifier, the same known silent H264 fixture,
three layers x three FX, 2560 x 1600 local Display, five seconds warmup and a
60-second capture request, with trace OFF and `RUST_LOG=off`. Compiler overlap
is rejected before, during and after capture. The original profile is not used.

Baseline build `01` (session 74730) and capture `baseline01` (26091) exited 0.
Actual native geometry was 2560 x 1600. Its measurement interval delivered
1185 frames in 59.6077604 seconds (19.8799618 counter fps); Standby retirement
took 431.759 ms. Outputs/routes/health were empty afterward, and exact PID 43944
and the measurement node were absent after the observed native close action
(process verification 69918, exit 0). Application stdout/stderr were both empty.

Candidate build `01` (71571, native exit 0) is **excluded**, never launched or
measured: Windows `copyFile` preserved the restored source's 03:41:45 UTC mtime,
so Cargo reused the baseline `video` rlib despite the correct source hash.
`invalid-candidate-build-01.json`, that executable and full logs preserve this
failure of the harness. After its real process exited, repair 26209 (exit 0)
advanced only the source mtime, verified unchanged bytes, and created attempt
`02` with an explicit `Compiling video` acceptance check. A session-reader
`INVALID_ARGUMENT` for 71571 was handled by following PID 5436 and its log;
the running build was not restarted. Candidate `02` is tracked by session
49398 / runner PID 38984 and `build-candidate-02-session.json`.

Candidate build `02` (49398) and capture `candidate02` (41578) exited 0.
Both accepted builds actually recompiled `video`, used the pinned MSVC
14.44.35207 linker first on PATH, and ran with identical QA configuration.
The full capture populations and rolling diagnostic windows are distinct:

| Measurement | Baseline `baseline01` | Candidate `candidate02` |
| --- | ---: | ---: |
| Complete polling samples | 104 | 102 |
| Counter interval | 59.6077604 s | 59.299355 s |
| Delivered frames | 1185 | 1007 |
| Counter fps | 19.8799618 | 16.9816350 |
| Dropped-frame counter delta | 2390 | 2548 |
| Final present / render rolling samples | 599 / 600 | 599 / 600 |
| Native validation frames / deadline misses | 600 / 600 | 600 / 600 |
| Native frame-budget pass | false | false |
| Standby retirement | 431.759 ms | 441.045 ms |
| Rust warnings / existing Vite large-chunk warning | 0 / 1 | 0 / 1 |

The candidate was slower in this single sequential matched pair; this is
**not an accepted performance improvement**. Decoder work also differed
(lifetime end totals, including warmup: decode wall time 14,693,155 us / 1317
calls versus 23,618,645 us / 1141 calls; lookahead hits 1995 versus 1245).
These are not first-to-last capture deltas and do not isolate the cause to the
ownership change. No cross-population percentile arithmetic is used.
Repeated controlled measurements, memory-pressure behavior and the 60 fps
budget remain unresolved. Historical 19.8 fps and traced 16.53 fps are not
an unconditional comparison baseline.

The fixture SHA256 is
`1ad7bed8a2e59b6e86ae4a23800bb70e5f39554f9991dc9befb697ae9e11dd75`;
the known silent H264 media SHA256 is
`53861258351df591b53450228d532c60fd914caa8de1d88b2029e480488e0c2b`.
Accepted baseline/candidate exe SHA256 values are respectively
`58693b78a310faba662a7c6973090a047c8f7af76714d7f41fc940f9ec8f0761` and
`148e27b77afb2515d668a32e0d9cd7b0b6f10479dc9db54038128b01d1430cf3`.
Both captures used actual 2560 x 1600 Display; decode failures, libav session
errors and hardware errors were zero. Stop returned Ready/Standby with video
and lighting disallowed, no live/test window, no active route and empty health.
Exact candidate PID 45652 and its measurement node were verified absent
(74517, exit 0). Both applications' stdout/stderr were empty with trace OFF.

Restoration (74913, exit 0) verified all eight original runtime files, including
the ordinary exe SHA256 `cd4c3a1f8d3048a9407d086de48d44be37c2711295bfd91f48038cd2cc12516c`.
Candidate source hashes remain the reviewed/tested hashes above. Aggregation
(26306, exit 0) wrote `comparison-summary.json` and archived the final harness;
the excluded first candidate build and all failure evidence remain intact.
Compiler-warning delta is zero (Rust 0 to 0; Vite large-chunk 1 to 1).

The user then prioritized the one-row header / DMX Patch UI correction.
The ownership candidate and its tests are preserved as unfinished work and
are not included in a completed performance claim or the UI-only commit.
Unknown DMX/audio hardware, external output, macOS, physical scanout and
long-run stability were not exercised. Resume performance investigation from
the saved source/runtime/fixture hashes and full same-campaign evidence.

The completed UI-priority checkpoint and its isolated native gate are recorded in
[UI_SINGLE_ROW_2026-10-22.md](UI_SINGLE_ROW_2026-10-22.md). Its normal executable
contains the UI correction with baseline video code; the candidate Rust source
was restored as unfinished work and is excluded from that UI commit.

## Adopted borrowed-input copy reduction, 2026-10-24

This is a limited **source-adoption checkpoint** on
`6d5aeb9e7215d0ba96502988e1eb05d10af772ce`. It integrates the already reviewed
Cow implementation described above without further product changes. Successful
ISF processing borrows the shared input and supplies a new owned frame; ownership
conversion is deferred until needed. The owned-input API delegates to the same
canonical processing path. Public API/schema, decoder-owned pixels, Clip then
Layer ordering, stage-local faults/bypass, failure returns, FRAMEINDEX/feedback,
LastValid, queue/GPU eligibility, input-retention and cancellation policies are
unchanged. The frozen source SHA256 values are:

- `crates/video/src/lib.rs`:
  `6a672620ecc25a06a72fbfe9f64a69091268c4409fa5c0cfd851138701a12430`
- `crates/video/src/isf_borrowed_input_tests.rs`:
  `029c77acd5a8a5997734ba5983b1f6f0801fb8ae180967730ada776c33d89ca3`

The earlier single pair, **A 19.8799618 / B 16.9816350 fps on 2026-10-22**,
is retained above and was not accepted as an improvement. Neither that pair,
the separate October 22 `b01`, the traced 16.53 fps run nor the composed
readback capture below is included in the following four-run population.

### Fresh archived-executable comparison

Campaigns under `target/qa/fx-perf-2026-10-22/native/` are
`ownership-pair-20261024-root02` (B then A) and
`ownership-pair-20261024-root03` (A then B). Their saved manifests identify
**old binary base `cfe8ca56...`**, with the same UI, old readback implementation,
FFmpeg DLLs and QA configuration; their checkout guard is current `6d5aeb9e...`.
The archived source comparison differs only in the ownership candidate.
Both accepted exe hashes remain A `58693b78...` and B `148e27b7...` (full values
above). All archived runtime bytes were rehashed. The UI manifests match across
457 build entries and the seven DLL hashes match; the QA configuration was the
same saved invocation, not a new independent extraction from the PE resources.

Both orders used the same known silent 1920 x 1080 H264, three layers x three
stages, local Display at actual **2560 x 1600**, log/trace OFF, five seconds
warmup and a 60-second capture request. Fixture/media hashes are those above;
the common harness SHA256 is
`13f111d649ed0eb4ef133081de7c776af0e3f2c7962292a1bc8428c5c75af05b`.
The maximized native window records have CSS viewport 1280 x 752, DPR 2.
The analysis uses all **411/411 saved polls**, with **zero exclusions**.

| Chronological run | Polls | First-to-last s | Delivered frames | Counter fps | Decode wall delta us | Decode calls | Mean ms/call |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| root02 B, PID 33892 | 102 | 59.1885421 | 1669 | 28.1980251715 | 7187281 | 1669 | 4.3063397244 |
| root02 A, PID 44124 | 103 | 59.2233549 | 1214 | 20.4986698584 | 13230388 | 1214 | 10.8981779242 |
| root03 A, PID 32444 | 103 | 59.5619650 | 1213 | 20.3653455691 | 13487846 | 1213 | 11.1194113768 |
| root03 B, PID 41248 | 103 | 59.5972254 | 1748 | 29.3302244906 | 6393446 | 1748 | 3.6575778032 |

| Same-interval decoder delta | root02 B | root02 A | root03 A | root03 B |
| --- | ---: | ---: | ---: | ---: |
| Requests | 5007 | 3642 | 3639 | 5244 |
| Lookahead decode | 4827 | 3633 | 3630 | 4995 |
| Lookahead hit | 4296 | 1920 | 1875 | 4599 |
| Lookahead discard | 0 | 0 | 0 | 0 |
| Session reset | 15 | 15 | 15 | 15 |
| Frame reuse | 180 | 3 | 3 | 249 |
| Sequential continue | 4812 | 3624 | 3621 | 4980 |
| Hardware frames | 5343 | 5337 | 5376 | 5376 |
| Decode/session/hardware errors | 0 | 0 | 0 | 0 |

FPS is delivered-frame delta divided by monotonic elapsed time between each
run's first and last saved poll. Decoder counters use those exact endpoint
polls; mean decode time is delta wall time / delta calls, not a subtraction of
percentiles. All polls have health/window records, monotonically increasing
time and nondecreasing cumulative counters, stable incarnation/epochs and
2560 x 1600 output. Health, window and ownership queries within a poll are
sequential rather than an atomic, frame-aligned snapshot.

For trends, each run is partitioned at saved polls nearest 10/20/30/40/50 seconds
from its first poll. Adjacent bins share endpoints, cover every adjacent interval
once and sum to the full frame and decoder deltas; no interpolation is used.
Approximate 10-second FPS ranges are root02 B **26.54-28.65**, root02 A
**19.42-21.37**, root03 A **20.03-20.85**, root03 B **28.96-29.60**.
The complete bins and all normalized polls are retained in
`target/qa/ownership-pair-analysis-20261024/adoption/four-run-analysis.json`.

Both fresh orders show higher candidate FPS and less synchronous decode waiting
in this archived workload. Initial playback positions nevertheless differ:
root02 B/A **8007/9024 ms**, root03 A/B **10299/6931 ms**; these snapshots precede
the first counter poll by about 322-360 ms. Lookahead hits and reuse change with
FPS and decode waiting. The observations therefore do not isolate ownership-only
time savings, establish a general improvement percentage or guarantee incremental
FPS on current main with the newer readback implementation. All four final
rolling validation windows have **600/600 deadline misses, budget pass=false**;
these are bounded diagnostic windows, not the 411-poll capture population.
**60 fps remains unmet.**

All four capture exits are 0. Stop command times in chronological order are
**616.5856 / 579.7110 / 624.9473 / 583.6411 ms**, each returning Ready/Standby,
video/lighting disallowed, no live/test output and empty routes/health. Saved
normal-close records verify each exact application plus six child processes
gone (seven total), no QA listener and no compiler; stdout/stderr are empty.
These are saved termination observations, not newly launched process checks.
Root02 and root03 restored all eight protected normal runtime files at
**2026-10-24 04:46:22.854 UTC** and **04:51:56.704 UTC**, respectively.
Their protected archives and the present `target/release` bytes were rehashed
8/8 against the saved manifests; the normal exe is still
`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.

The separate failed `ownership-pair-20261024-01` remains preserved: Arm was
forbidden, B/PID 18976 produced **0 samples**, session 30823 exited 1, Stop
took 411.6034 ms, all seven owned processes closed and all eight normal runtime
files were restored. No A capture was made. This failed attempt is not a fifth
performance observation and no product guard was relaxed.

### Existing contract and composed-native evidence

No Rust tests, GPU tests, build or native run were repeated for this adoption.
The raw logs, complete test names, exit records and source hashes were checked:

| Actual date (UTC) / evidence | Passed | Failed | Ignored | Exit | Source basis |
| --- | ---: | ---: | ---: | ---: | --- |
| 2026-10-22, `c1_-03.log` | 27 | 0 | 0 | 0 | `cfe8ca56...` plus the identical Cow hashes |
| 2026-10-22, `video_render_cancellation_tests-03.log` | 4 | 0 | 0 | 0 | Same historical candidate |
| 2026-10-24, readback `workspace-01.log`, including ignored GPU cases | 4 | 0 | 0 | 0 | Current UI + committed readback bytes + identical Cow hashes |
| 2026-10-24, readback `isf-01.log` | 76 | 0 | 2 | 0 | Same composed source bytes |

The historical October 22 ISF 76/0/1 run and the corrected fixture failure are
also retained above. The October 24 records were produced while HEAD still
read `693182041ea1d52cd0afa48bc5899bf7f9407b16`, before the readback commit;
their before/after hashes match both readback files committed at `6d5aeb9e...`
and the two unchanged Cow files. They verify that source composition, not a
test invocation made after the commit. The older C1/cancel runs are explicitly
historical evidence and are not described as fresh 6d runs.

The 76-test ISF run executed all four borrowed-input contract tests, including
failure pixels/fault identity, noncommuting Clip/Layer order, missing-projection
fail-closed behavior, shared-input immutability and cancellation. Its two ignored
tests are `isf_targetless_batch_timing_experiment` (explicit matched GPU timing;
no wall-time assertion) and `mapped_readback_copy_timing_experiment` (explicit
matched mapped-copy timing; no wall-time assertion). The latter actually ran
within the separate four-test GPU run, which includes failed readback, rejected
inputs, scratch reuse/pixels/odd pitch/resize and the timing experiment. These
filter totals are not asserted to be distinct tests across filters. All accepted
logs verify MSVC **14.44.35207** pinned and first on PATH; Rust warnings are **0**,
with **0 delta** against the recorded baseline.

The existing composed native gate is documented separately in
[ISF_READBACK_COPY_2026-10-24.md](ISF_READBACK_COPY_2026-10-24.md), campaign
`target/qa/isf-readback-copy-2026-10-24/fb2a460b-1788-4bcb-a637-bc6fd1cec9ac/`.
Its isolated current-UI + readback + identical-Cow build exited 0, with exe
SHA256 `76c60a6102820bef8c989d5f80c80bb0719c6bc2a6173a263b988ab7d164d6bb`.
The same silent H264 3-layer/3-stage workload at 2560 x 1600 produced **103 polls,
1715 frames / 59.6843671 s = 28.7344925 fps**, zero decode/output errors and
**600/600 misses**. PID 46544 Stop was **707.6515 ms**, followed by Standby,
empty routes/health and saved disappearance of that app, six children and port
9273. This establishes the existing composed native/Stop gate; it is not a
matched before/after comparison or part of the four archived-exe observations.
The then-live separate PID 19836 was outside that close record and was retired
in the subsequent restoration unit. Build warnings were Rust **0**, existing
Vite large-chunk **1**, with recorded warning delta **0**. There is no new
compiler-warning measurement in this adoption-only unit.

The original source review (`independent-review.json`, reviewer
`/root/review_borrowed_fx`) has no blockers and its source hashes still match;
the subsequent readback/composed review is recorded in the readback QA.
The four raw captures were independently recomputed by
`/root/verify_archived_pair` (session **40738, exit 0**), agreeing with the
integration verifier (**17354, exit 0**) on all polls, endpoint deltas, trends,
saved Stop/close records and runtime hashes. Verification details, input hashes,
the original Cow diff and final integration records are retained under
`target/qa/ownership-pair-analysis-20261024/adoption/`.

The adoption preflight first exited 1 (session 69900) because its helper sorted
porcelain status lines before normalizing leading whitespace. It reported the
same expected three paths in a different order and stopped before any Git
mutation. Correcting only that helper comparison passed in **17545, exit 0**:
base/index/ownership guards, protected source/runtime hashes and `git diff
--check` all passed. This was not a product/test failure or a repeated build.

Only the two frozen Cow files and this QA file belong to the adoption commit.
Normal runtime bytes and the separate ignored row-composition draft are
preserved. The accepted change removes an unnecessary normal-success input
copy; overall performance, memory-pressure behavior, long-run stability,
physical scanout, other workloads/platforms and the 60 fps goal remain open.
Unknown DMX/audio hardware and external outputs were not exercised.
