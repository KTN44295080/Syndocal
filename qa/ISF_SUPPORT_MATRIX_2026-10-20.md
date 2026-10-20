# ISF support matrix — T23 / T27 / T32 — 2026-10-20

## Scope and conclusion

This audit covers the ISF filter path in `crates/video`, its native import/validation seams, and the FX rejection explanations. The comparison target is ISF JSON specification v2 and the official ISF documentation listed below. Syndocal remains a **partial ISF host**.

T23 adds ordered multi-pass rendering, named targets, persistent feedback, RGBA32Float targets, bounded per-frame dimension expressions, and shader-relative PNG/JPEG imports. Point2D, color and event controls already existed; their actual rendered output is now covered by small fixtures. Image helpers now use each sampled texture's dimensions.

The historical T23 work was isolated to branch `task23-isf` in `wt-task23`, based on `036a268c01e7a6dcf8b41830e99e4593f2922817`.

T27 adds bounded mono `audio` / `audioFFT` textures using the live input selected in Setup. Its original work was isolated to `task27-audio-mapping` in `wt-task27`, based on `e1d72e2306620d9c64454abf426ded50c1261f4d`. The audio contract supersedes T23's unsupported-audio finding. See [T27 acceptance](T27_AUDIO_CONTROL_ISF_ACCEPTANCE_2026-10-20.md) and [T27 main integration](T27B_MAIN_INTEGRATION_2026-10-20.md) for mapping safety and the remaining native acceptance boundary.

T32 adds DATE, zero-image generators and named image bindings on `task32-isf-date-images` in `wt-task32`, originally based on `7fe67b00`. T32b integrates main `cbae80d3` into accepted T32 `27dbf145` on that task branch. The current contracts below include both image and audio inputs. Original T23, T27 and T32 validation counts are historical; combined T32b evidence is recorded separately at the end. No application, native harness, release build, push or merge into main is part of the T32b worktree validation.

### Official comparison sources

- **S1 — JSON attributes and inputs:** [ISF JSON Reference](https://docs.isf.video/ref_json).
- **S2 — Image helpers:** [ISF Functions Reference](https://docs.isf.video/ref_functions).
- **S3 — Standard variables:** [ISF Variables Reference](https://docs.isf.video/ref_variables).
- **S4 — Vertex stages:** [ISF Primer: Vertex Shaders](https://docs.isf.video/primer_chapter_5.html).
- **S5 — Passes and persistence:** [ISF Primer: Multi-Pass and Persistent Buffers](https://docs.isf.video/primer_chapter_7).
- **S6 — Specification and version context:** [mrRay/ISF_Spec](https://github.com/mrRay/ISF_Spec) and [Adapting GLSL to ISF](https://docs.isf.video/primer_chapter_9).

Sources were inspected on 2026-10-19/20 UTC. The status column describes the resulting implementation, including its explicit limits. “Supported” means the stated operation works within those limits, rather than a claim that arbitrary ISF packs are compatible.

## Passes, targets and resources

| Feature | Current status | Implemented behavior and remaining boundary | Evidence |
| --- | --- | --- | --- |
| Ordered `PASSES` | Supported, bounded | One shader runs once per declared pass, in order. Missing/empty PASSES becomes one ordinary pass. The declared image input remains the stage's input across all passes; named targets expose earlier pass results. Maximum 16 passes. | S1/S5; `isf_runtime.rs:644`; `isf_pass_runtime.rs:163`; fixture `multipass.fs`. |
| Named `TARGET` | Partially supported | Each distinct name has separate read/write textures. A pass reads its previous front and writes the opposite texture; later passes see the new front. Names must be safe and unique across inputs, targets and imports. Reusing the same TARGET name in multiple pass declarations is rejected. | `isf_runtime.rs:506`; `isf_pass_runtime.rs:163`; multipass and invalid-metadata tests. |
| Nonpersistent targets | Supported | Initial target contents are transparent black on each frame. A later pass sees earlier writes in that frame. | S5; `isf_completeness_tests.rs:85`. |
| `PERSISTENT` targets | Supported, bounded | Positive numeric flags or true retain target contents between successful frames. PERSISTENT requires TARGET. History belongs to an effect instance, not a shader pipeline. Reallocation clears history when evaluated target dimensions or shader/resource content change. A fixed-size target can retain history across a host-size change. | `isf_runtime.rs:46`, `:644`; `isf_pass_runtime.rs:464`; persistence and fixed-size resize tests. |
| `FLOAT` targets | Supported on capable adapters | Uses **RGBA32Float**, including negative values, values above 1 and full 32-bit precision. Requires wgpu `FLOAT32_FILTERABLE` because image sampling uses the filtering sampler. Other adapters get a specific error. Ordinary RGBA8 shaders do not require this optional feature. | S1; `isf_runtime.rs:156`; `isf_pass_runtime.rs:647`; FLOAT fixture and GPU test. |
| `WIDTH` / `HEIGHT` constants | Supported | Numbers and numeric strings resolve to positive integer pixel dimensions. Fractional results are truncated. Invalid, nonfinite, zero and out-of-range results fail explicitly. Device texture limits apply before allocation. | S1; `isf_resources.rs:25`, `:78`; dimension and oversized-pass tests. |
| Dimension expressions | Partially supported | Evaluated once per frame from host WIDTH/HEIGHT and current scalar controls. Supports `+ - * / % ^`, unary signs, parentheses, exponent notation, and `min/max/floor/ceil/abs/round/trunc/pow/sqrt/clamp`. Accepts `$WIDTH`, `$HEIGHT`, `$control` and bare variable names. This is a bounded arithmetic subset, not the complete DDMathParser language. Vector controls cannot define dimensions. | S1; `isf_resources.rs:20`, `:151`; arithmetic, live-control and invalid-expression tests. |
| Final pass size/format | Supported as host adaptation | A final named/FLOAT/differently sized pass is sampled back into the requested host size and RGBA8 format. Intermediate values retain their target precision. Host frame identifiers and timing metadata are preserved. | `isf_pass_runtime.rs:163`; final-FLOAT test at `isf_completeness_tests.rs:125`. |
| `IMPORTED` images | Partially supported | Native file import and stage validation pass the source path. Relative PNG/JPEG assets are decoded before rendering, cached as prepared RGBA8 resources, then uploaded for the instance. Missing context/files, invalid declarations, unsupported formats and decode failures give specific errors. | S1; `isf_resources.rs:465`; `app/src-tauri/src/main.rs:27621`, `:43904`; imported-image fixtures/tests. |
| IMPORTED path forms | Partially supported | Accepts child paths using either separator. Absolute paths, URLs, parent traversal, alternate streams and canonical paths escaping the shader folder are rejected. The ISF reference permits parent-relative paths; this host intentionally requires resources to remain in the shader folder. | S1; `isf_resources.rs:465`; path rejection tests. |
| Resource identity and reload | Partially supported | Prepared-cache identity includes source text and source path, so identical shaders in different pack directories load their own images. GPU history fingerprints include decoded image contents. Existing cached assets are not watched for file changes; a source/path change or a fresh renderer/reload is needed to guarantee re-decoding. Imported bytes are not embedded into project files. | `lib.rs:6311`; `isf_runtime.rs:506`; distinct-pack cache test at `isf_completeness_tests.rs:207`. |

All short Rust paths in this document are relative to `crates/video/src/` unless an `app/` prefix is present. T23 line references are historical baseline locations; the T27/T32/T32b sections name current source files and regression entry points.

### Resource bounds

- Maximum 16 passes, 16 total sampled textures (declared image slots, audio inputs, named targets and metadata imports), four image declarations and 16 scalar/vector controls. A bound still occupies its declared image slot. Additional layer dependencies are limited to 32 links (33 layers).
- Dimension expressions: at most 1,024 ASCII bytes, 256 AST nodes and 32 nested parse levels.
- Imported images: at most 15, at most 16 MiB per encoded file, at most 8,192 pixels per axis, and at most 64 MiB of combined decoded RGBA per prepared shader. The total-texture limit can reduce the available import count.
- Metadata imports and bound stills share the decoded-byte and import-count limits. The prepared-shader cache and pipeline cache each retain at most 64 entries. Pipeline identity includes WGSL hash, target format and texture count. Persistent/import/bound-still/audio GPU instance resources are retained in a 64-entry LRU pool.
- Stage-owned texture accounting is capped at 256 MiB per stage: both target textures, imports, bound stills, audio textures, additional dynamic images, transparent defaults and every separately allocated unnamed intermediate. The 512 MiB global limit additionally covers the shared host upload/ping/pong textures and padded readback across the resource pool and current encoder. Anonymous allocations accumulate until submission completes; a cache hit recalculates their cost for the current host size. Driver overhead and small pipeline/uniform allocations are outside this texture/readback estimate.
- LRU eviction, a failed stage that loses its resource state, changed target dimensions, changed shader/resource fingerprint, a new renderer, or a new project namespace can reset feedback. A failed submitted stack cannot publish new history. This is bounded live-renderer state, not project-persisted feedback.

The main checks and allocation ownership are in `isf_pass_runtime.rs:128`, `:163`, `:430`, and `:464`. Resource decode bounds are in `isf_resources.rs:9` and `:465`.

## Input types

ISF defines nine input kinds (S1). T32 accepts zero through four dynamic image declarations. By default, an image named exactly `inputImage` receives the host frame even when it is not first in INPUTS; if absent, the first declared image does. The parameter editor exposes the other image declarations; unbound additional images are transparent. Explicit project/API bindings can override the primary input.

| Input kind | Status | Behavior and limits | Evidence |
| --- | --- | --- | --- |
| `image` | Supported, bounded | Zero through four dynamic images. Named additional inputs bind to a stable layer output ID, a shader-relative PNG/JPEG still, or transparent none. Layer references use dependency order and reject direct/indirect cycles and missing IDs. Existing one-image shaders keep their host input. Native transition conventions are not automatically assigned. | S1; `isf_runtime.rs`; `isf_layer_inputs.rs`; `isf_layer_binding_tests.rs`. |
| `float` | Supported | Existing defaults/ranges and per-frame control resolution remain. Bounded values become float uniforms. | `isf_runtime.rs:759`, `:832`; threshold GPU regression; all 14 built-ins compile. |
| `bool` | Supported | Existing control value becomes a GLSL boolean using the established threshold. | `isf_runtime.rs:832`; parameter-model and built-in checks. |
| `long` | Partially supported | Existing integer controls and enumerated values/labels work; storage is a float control slot before GLSL int conversion, so arbitrary full-width integer precision is not guaranteed. | `isf_runtime.rs:759`, `:832`; FX parameter and built-in checks. |
| `point2D` | Supported | Two components, default and overridden coordinates. This was already implemented; T23 adds a rendered fixture. | `isf_completeness_tests.rs:251`; `point2d.fs`. |
| `color` | Supported | Four components including alpha, with existing defaults/ranges. This was already implemented; T23 pins all four rendered channels. | `isf_completeness_tests.rs:270`; `color.fs`. |
| `event` | Partially supported | Existing host pulse value is shared by all passes and stops affecting pixels once the host resets it. The renderer does not consume/reset the event itself after exactly one frame; existing host pulse timing is preserved. Preview audition keeps its existing event restrictions. | `isf_completeness_tests.rs:283`; `event.fs`; FX audition/UX checks. |
| `audio` | Partially supported, T27 | One channel row from the selected live-input mix. Default width 512; integer MAX from 1 to 512. Signed waveform samples map to grayscale around 0.5, with alpha 1. Smaller widths use linear resampling. MAX=1 uses RMS of the published 512-sample window, mapped to 0.5–1; this is an explicit host policy, not a prescribed RMS formula in S1. RGBA8 precision. | S1; `isf_runtime.rs`; `isf_pass_runtime.rs`; `isf_audio_tests.rs`; `audio.fs`. |
| `audioFFT` | Partially supported, T27 | One channel row of normalized FFT magnitudes on a linear frequency axis. Default width 512; integer MAX 1–512. Smaller widths preserve the peak in each bin group. Uses the analyzer's existing FFT, with no second transform. RGBA8 precision; no stereo rows, custom FFT size or arbitrary texture widths. | Same fixture and tests; `crates/audio/src/live_features.rs`. |

### T27 publication, ownership and cost

The analyzer publishes two fixed 512-element float arrays: waveform and FFT. The sample payload is 4 KiB plus generation, sequence and sample-rate metadata, shared by Arc with engine/render consumers. It is excluded from project JSON and UI snapshot serialization. The recording worker carries the same 4 KiB in its bounded binary request, with generation, sequence, sample rate and captured DATE in the request header. The combined SRF1 envelope stores audio before injected image bytes. Older headers without DATE use the deterministic epoch fallback, and headers without audio select silence. Program, live Preview, display, Spout, NDI and recording contexts capture the publication; ordinary thumbnails explicitly select silence.

Each renderer retains only a changed generation/sequence. Each ISF instance allocates one width-by-one RGBA8 texture per declared audio input and uploads only a new publication or a transition to silence. Waveform silence is byte 128 (the nearest 8-bit representation of 0.5); FFT silence is zero. Stop, stale input, generation retirement, project replacement and invalid samples clear the publication. Reusing a static decoded image still evaluates its ISF stages, so audio changes do not depend on a media PTS change.

At width 512 each input costs 2 KiB of texture storage. Audio inputs count toward the existing 16 sampled-texture limit and the 256 MiB per-stage / 512 MiB pool accounting. No capture thread, extra FFT or unbounded waveform queue is introduced. The runtime uses the existing GPU ISF stage and readback boundary, including when a CPU compositor consumes its result. There is **no pure CPU ISF evaluator**: an unavailable adapter retains the existing explicit effect failure behavior. C1 compositor eligibility, mapping, pacing and marked last-valid fallback remain in place.

The original five-test T27 suite includes three actual GPU pixel tests without an adapter-skip branch and two metadata tests, covering: initial silence; new and duplicate identities; generation replacement; stop/invalid-frame clearing; width 2 endpoint/FFT-peak resampling; width 1 RMS; invalid MAX values; and texture-count bounds. These are bounded-host acceptance tests, not full ISF conformance or real-device acceptance.

## T32 image binding and generator behavior

Image bindings are authored as `image_bindings: [{name, source}]` on each ISF effect/stage. The source is `{kind: "none"}`, `{kind: "layer", layer_id}`, or `{kind: "imported", path}`. Missing fields deserialize to an empty list and empty lists are omitted, preserving older project data. Layer IDs, not bank indexes or display labels, own references. Existing effect replacement/preset/save paths carry the bindings; no extra Tauri route is introduced.

A sampled layer is its current Clip/transition/Layer-FX result before group/composition transformations and compositing. A dependency can be outside the visible composition. It is prepared before consumers, using shared frame ownership. Every render clears its binding map; a removed layer, an unavailable frame or a dependency stage fault produces an explicit error instead of exposing an old binding. Pipeline identity remains shader/layout based; resource identity additionally includes authored bindings and imported contents, so retargeting a binding invalidates feedback without resetting it whenever live image pixels change.

A zero-image generator runs as an ordinary layer effect that ignores the incoming frame. Its RENDERSIZE comes from the actual host frame supplied at that effect scope (or a declared pass size); output/Preview requests use their requested host dimensions. This work does not add a new native ISF source kind: use an existing generator/source layer as its host. Imported stills retain the T23 shader-folder containment and decode limits; projects store relative paths, not embedded image bytes. Assets are not watched continuously for content changes.

Layer bindings are resolved on snapshot-backed output, Preview and layer-thumbnail paths. Scheduling follows the actual composition/output scope; groups must be enabled and visible, and LayerBus dependencies are included only for active runtime buses with visible members. A shared layer does not pull in unrelated compositions. Thumbnails resolve layer/clip dependencies only. The older keyed-input mix API has no authored layer-output context and gives a specific unavailable-context error. Timeline Follow layer bindings are rejected at authored/engine-ready admission because its standalone combine API has no unambiguous layer context. None and imported stills remain usable in those APIs. Custom vertex stages stay explicitly rejected: the fixed fullscreen vertex path has no custom varying or vertex-init contract, and adding one would exceed this bounded fragment-host change. The integrated runtime also supplies T27 audio/audioFFT textures; audio and image inputs share the sampled-texture and memory limits above.

## Sampling helpers and standard variables

| Feature | Status | Resulting contract | Evidence |
| --- | --- | --- | --- |
| `IMG_PIXEL(image, coord)` | Supported, bounded | Pixel coordinates are normalized by that image's actual texture size, including differently sized targets/imports. Shared linear clamp-to-edge sampling; no optional bias overload. | S2; `isf_runtime.rs:832`; multipass/import tests. |
| `IMG_NORM_PIXEL(image, coord)` | Supported, bounded | Samples normalized coordinates using the same sampler. No optional bias overload or mipmap chain. | S2; imported known-pixel test. |
| `IMG_THIS_PIXEL(image)` | Supported within legacy orientation | Uses fragment pixel position divided by the sampled image's size. It is no longer accidentally equivalent to normalized sampling for images whose size differs from the render pass. | `isf_completeness_tests.rs:323`. |
| `IMG_THIS_NORM_PIXEL(image)` | Supported within legacy orientation | Uses the current normalized fragment coordinate. The resized-import test distinguishes its interpolated output from IMG_THIS_PIXEL. | `isf_runtime.rs:832`; `isf_completeness_tests.rs:323`. |
| `IMG_SIZE(image)` | Supported | Returns the selected image/target/import texture's actual dimensions. | S2; `multipass.fs`, `imported.fs`. |
| `RENDERSIZE` | Supported | Current pass dimensions, including expression-sized targets. | S3; `isf_pass_runtime.rs:163`; multipass fixture. |
| `PASSINDEX` | Supported | Zero-based pass ordinal. Every frame starts its pass sequence at zero. | S3; `isf_runtime.rs:921`; multipass fixture. |
| `TIME` | Partially supported | Host-supplied nonnegative seconds, unchanged across a chain's passes. Production uses the frame's media PTS, rather than introducing a separate shader-start clock. | S3; `lib.rs:6360`; `isf_runtime.rs:921`. |
| `TIMEDELTA` | Partially supported | Host-supplied nonnegative delta, unchanged across passes. Production uses frame duration; it does not force the first shader frame to zero or measure wall-clock time since the preceding render. | S3; same uniform/caller evidence. |
| `FRAMEINDEX` | Partially supported | Preserves the existing runtime-wide chain counter. All passes/stages of a stage-local chain see the same value; a nonempty stage-local chain advances once even if all stages fail. It is not an independent counter starting at zero for each newly imported document/instance. | S3; `isf_runtime.rs:197`, `:264`; frame-index and C1 topology regressions. |
| `DATE` | Supported with an explicit host clock convention | A vec4 contains UTC year, month 1..12, day and fractional seconds within the day. Live time is captured once for a render and shared across its stages/passes. `set_isf_date_override` freezes/injects values for deterministic rendering. The recording parent captures DATE into each request; the worker replays that value. | S3; `isf_runtime.rs`; `isf_layer_inputs.rs`; `app/src-tauri/src/video_recording_renderer_process.rs`. |
| `isf_FragNormCoord` / `gl_FragCoord` | Partially supported | Existing fullscreen vertex and wgpu pixel-coordinate convention remain. The host keeps its established top-left image convention; ISF describes bottom-left normalized origin. No orientation conversion was introduced into the parity-sensitive path. | S3; fullscreen vertex in `isf_runtime.rs`, wrapper at `:832`. |

## Vertex stages, GLSL and host behavior

| Area | Status | Boundary | Evidence |
| --- | --- | --- | --- |
| Default vertex stage | Supported | Existing generated fullscreen triangle supplies normalized coordinates to fragment filters. | S4; `isf_runtime.rs` vertex shader and `isf_pass_runtime.rs:647`. |
| Custom vertex shader | Unsupported, explicit file rejection | Native/path-aware preparation detects a same-basename .vs or .vert companion and rejects it. Source-only preparation cannot detect absent path context. Custom varyings and `isf_vertShaderInit()` are not provided. | S4; `isf_runtime.rs:506`; companion-file test at `isf_completeness_tests.rs:231`. |
| GLSL version adaptation | Partially supported | Fragment code is wrapped as GLSL 450, parsed/validated by naga 25 and emitted as WGSL for wgpu 25. gl_FragColor and texture2D aliases are supplied; the texture2D alias is placed after generated texture declarations. This is not general GLSL 1.20/ES/core compatibility. | S6; `isf_runtime.rs:832`. |
| Explicit version/extensions and advanced constructs | Unsupported by current policy/subset | Existing checks reject #version, #extension, #include, common loop spellings and low-level declarations. This text-based check can also reject comments; it is not a complete GLSL token validator. Parser/GPU validation remains the following gate. | `isf_runtime.rs:697`; existing negative tests. |
| JSON `ISFVSN` | Partially supported | v2-shaped metadata is consumed, but ISFVSN is not negotiated or validated. Missing/other version tags do not select a different language/runtime. | S1/S6; metadata parsing at `isf_runtime.rs:506`. |
| CPU/GPU parity and output pacing | Preserved | The existing C1 artistic eligibility, compositor parity, output mapping, last-valid evidence and pacing logic are not redesigned. Passes stay on the GPU until the existing readback boundary. No native frame-rate claim is made. | 26 C1/parity regression tests; changes centered at `lib.rs:6311` and `:6360`. |
| Preview audition and thumbnails | Preserved, histories isolated | Strong scope/stage/input identity plus output/preview/project namespace prevents feedback from crossing instances. Legacy Preview/output and thumbnail entry points also select explicit namespaces. Existing audition commit/reset restrictions remain. Complex frontend thumbnails keep their existing fallback behavior. | `lib.rs:4565`, `:4685`, `:6404`; `layer_thumbnail_render.rs:29`; two renderer integration regressions and FX audition/UX checks. |
| Rejection explanations | Updated, EN/JA | Covers image bindings, DATE, audio MAX and the shared texture limit alongside pass/format/import errors. Removes obsolete unsupported-image/audio guidance and preserves raw diagnostic text. | `app/src/fxRejectionExplanation.ts`; `app/src/uiLocalization.ts`; 38 bilingual catalog cases and the T32b browser gate. |

## Rendered fixtures and regression evidence

Fixture directory: `crates/video/src/isf_fixtures/`. Image assets are generated in worktree-local test directories and removed by each test.

| Fixture / regression | Known result or invariant |
| --- | --- |
| `audio.fs` (T27) | Silent waveform/FFT gives [128,0,255,255]; waveform -0.5 / FFT 0.75 gives [64,191,255,255]. The original five tests check lifecycle, publication identity, MAX bounds and resampling; T32b adds combined image/audio coverage below. |
| `multipass.fs` | An 8×4 host with a half-size target yields RGBA [128,128,255,255], checking current pass size, actual IMG_SIZE, target sampling and PASSINDEX. |
| `persistent.fs` | Red advances 1,2,3 across frames. A different preview/stage/input/project namespace starts at 1. Changed target dimensions start at 1. Nonpersistent form stays at 1. |
| `float.fs` | Stores 2049, -2 and 2 in a FLOAT target; final RGBA [255,128,128,255] distinguishes full 32-bit storage from half precision and proves unclamped intermediates. |
| Final FLOAT adaptation | A fixed 1×1 FLOAT target produces a 4×2 RGBA8 host image with [64,128,191,255]. |
| `imported.fs` | Generated two-pixel red/green PNG sampled in a four-pixel host proves import resolution and per-image sizes. Separate directories with identical source keep distinct resources. JPEG decode also has bounded known-color checks. |
| `point2d.fs` | Default output [64,191,0,255]; control override [128,64,0,255]. |
| `color.fs` | Output [64,128,191,128] pins RGBA including alpha. |
| `event.fs` | Every pass sees the same active event; after host reset the next output returns to black. |
| Resized import helpers | IMG_THIS_PIXEL returns red,green,green,green. IMG_THIS_NORM_PIXEL returns red,[191,64,0,255],[64,191,0,255],green. |
| Anonymous buffer budget | Five 4096×4096 unnamed RGBA8 allocations are rejected before allocation; a subsequent small shader still renders correctly. |
| Fixed pass dimensions / host resize | Host 16→8→16 with fixed 16×16 targets preserves red history 1→2→3 and recalculates transient bytes 0→1024→0. |
| Public renderer isolation | Composition/output scopes, Preview/output namespaces, legacy output/Preview/thumbnails and epoch changes are exercised through public render methods. |

## Historical T23 validation and review

The following counts and ports are the recorded T23 checkpoint. Original T27 evidence is in the linked acceptance note; T32 and T32b evidence have separate sections below.

The final Rust acceptance runs used the debug profile through `pcargo-here.cmd`, with the worktree as CWD and **CARGO_TARGET_DIR removed**, following the corrected handoff. Earlier shared-target output is not acceptance evidence.

- `cargo check`: workspace check passes with zero warnings.
- `cargo test -p video isf -- --test-threads=1 --nocapture`: **34 passed, 0 failed, 0 ignored**. The FLOAT fixture reports `FLOAT32_FILTERABLE=true`; its pixel path ran.
- `cargo test -p video c1_ -- --test-threads=1`: **26 passed, 0 failed, 0 ignored**, including GPU/CPU compositor parity and artistic/failure/frame-index regressions.
- All eight `check-fx-*` scripts pass: catalog, live-session, parameters, audition, color-palettes, thumbnail-browser, ux-browser and scroll-browser. Catalog checks 14 built-ins and 24 bilingual rejection cases; scrolling checks 34 viewport/form combinations.
- Browser fixtures run sequentially on Vite **5380** and CDP **9380**, with isolated profiles and evidence under this worktree. Thumbnail dependency scanning is scoped to avoid an unrelated full-app scan racing server shutdown.
- `pnpm run check:localization`: **4101/4101 (100%)**, zero unprotected user-data labels.
- `pnpm exec tsc --noEmit`: passes.
- `git diff --check`: passes. Modified Rust files are UTF-8 without BOM and LF; formatting is restricted to new files and owned hunks.

Local validation logs are under `target/t23-validation/`; browser evidence is under `target/qa/t23-fx-ux/` and `target/qa/t23-fx-scroll/`. These ignored artifacts are not the commit's source deliverable.

An independent read-only agent reviewed supplied scheduler and renderer identity code. It identified anonymous-pass allocation accounting and cache-hit transient-cost refresh; both were fixed and regression-tested. The final bounded review has no remaining blocking finding in those supplied areas. Root reviewed the resource loader, expression parser, GPU descriptors, namespace entry points and native path propagation. The independent reviewer did not run commands or review every resource-loader detail. Claude's independent native review remains required before merging.

No `syndocal.exe`, native/real-app harness, release build, push or merge was run. Live output pacing, physical display/Spout delivery, native file-dialog interaction, real-world pack benchmarking and complete ISF conformance are outside this validation.

## Historical T32 rendered fixtures and validation

T32 adds these deterministic checks on top of the historical T23 baseline:

| Fixture / regression | Result or invariant |
| --- | --- |
| `date.fs` | Injected `[2026,10,20,43200.25]` produces RGBA `[26,10,20,64]` in both passes and across changed media TIME. Changing the injected date produces `[30,12,1,128]`; nonfinite and invalid calendar values fail before GPU execution. |
| `generator.fs` | An 8x4 host produces `[128,64,191,255]` regardless of host pixel contents, preserving the host dimensions and frame metadata. |
| `two_images.fs` | A blue primary plus a red overlay produces `[128,0,128,255]`. Unbound overlay produces `[0,0,128,128]`; a 1x1 overlay reports its own IMG_SIZE. `inputImage` wins even when declared after the overlay. |
| Current layer output and faults | Layer references sample post-Clip/Layer-FX pixels, survive list reorder, and refresh after binding changes. Missing sources, cycles, depth overflow, missing current frames and failed dependency stages produce explicit diagnostics. Stage-local faults preserve the current diagnostic frame with Error evidence; LastValid remains reserved for renders that cannot produce a frame. |
| Scope and parity | Composition/Output binding pixels agree between Preview and Output. Thumbnails keep layer-only scope. Shared layers do not import unrelated composition dependencies; idle/invisible LayerBus configuration does not add dependencies. Existing C1 CPU/GPU and last-valid behavior remains covered. |
| Persistence and admission | Old JSON keeps its original shape. Layer/Imported/None bindings round-trip, follow moved stages and survive native sanitization. Ambiguous source payloads are rejected. Deleting a referenced source is atomic and rejected; deleting its consumer and owned chains succeeds. Dormant authored chains and presets participate in validation. |
| Resource lifetime | Dynamic frame changes retain feedback, authored binding changes invalidate it, explicit primary None is transparent, and RGBA/BGRA secondary inputs share conversion behavior. Missing, unsafe and invalid imported stills are rejected using T23 bounds. An invalid dynamic input is followed by a successful render to check failure cleanup. |

Completed debug checks through `pcargo-here.cmd`, with CARGO_TARGET_DIR unset and temporary test files inside this worktree:

- `cargo test -p protocol isf -- --test-threads=1`: **11 passed, 0 failed, 0 ignored**.
- `cargo test -p engine isf -- --test-threads=1`: **20 passed, 0 failed, 0 ignored**.
- `cargo test -p video isf -- --test-threads=1`: **59 passed, 0 failed, 0 ignored**; the known-pixel GPU tests require an adapter and ran.
- `cargo test -p video c1_ -- --test-threads=1`: **26 passed, 0 failed, 0 ignored**, including RGBA/BGRA/BC1/BC3/BC7 compositor parity.
- TypeScript `tsc --noEmit`, `check-fx-catalog.mjs` and `check-fx-parameters.mjs`: passed. Catalog coverage includes 14 built-ins and **37 bilingual rejection fixtures**.
- Complete existing plus T32 `check-fx-ux-browser.mjs`: passed on Vite **5480** / CDP **9480**. Covers None/layer/still selection, stable IDs, relative-path validation, one explicit Apply write, rejected/no-op write restoration, generator hints, EN/JA and two themes. Controls are at least 44px tall with at least 14px text.
- `check-localization.mjs`: **4162/4162 (100%)**, zero unprotected user-data labels.

Rust logs are under `target/t32-validation/`. T32 browser screenshots were inspected under `target/qa/t32-isf-images/`; the final post-hint-cleanup browser run wrote refreshed evidence under `target/qa/fx-ux-12/`. These ignored artifacts remain worktree evidence, not committed source files.

Independent read-only reviews covered low-level GPU resource ownership, binding resolution and render scope, native sanitization/recording IPC, atomic engine deletion, and frontend documentation. Review findings led to required-current-frame validation, upload cleanup, a root-order-independent depth bound, explicit scope scheduling, authoritative UI write confirmation and strict None deserialization. No blocking finding remained in the reviewed implementation after those corrections. Native application/physical-output acceptance remains with the main agent.

### T32 final workspace and native unit gates

- `cargo check --workspace`: passed, **zero warnings**.
- `cargo test -p syndocal isf_image_binding_native_tests -- --test-threads=1`: **2 passed, 0 failed, 0 ignored**. CPU-only shader preparation checks authored-name rejection, native sanitization, stack projection and JSON round-trip.
- `cargo test -p syndocal render_request_keeps_external_frame_bytes_binary -- --test-threads=1`: **1 passed, 0 failed, 0 ignored**. The actual SRF1 envelope retains captured DATE and binary image bytes; a header without DATE receives the deterministic epoch fallback, and an unknown field is rejected.
- All seven final Rust commands above completed with **zero warnings**. Changed Rust files use UTF-8 without BOM and LF. New files and owned hunks were formatted; final staged `git diff --check` passed.

The native tests ran only the filtered Rust test runner. No Syndocal application session, native/real-app harness, native-gate, build-nobundle, release build, push or merge was run. Physical output/Spout, live pacing and native file-dialog acceptance remain outside this worktree's validation.

## T32b integration and validation

T32b integrates the handoff's main commit `cbae80d3` (T27 and T34) into accepted T32 `27dbf145` on `task32-isf-date-images`. The seven conflicts retain DATE, zero-image generators and named image bindings together with live waveform/FFT textures, silence clearing and the recording audio payload.

- Image slots, audio inputs, named targets and metadata imports share the 16 sampled-texture limit. Both branches' allocations remain within the same 256 MiB stage and 512 MiB aggregate accounting, including bound stills and transient image uploads.
- The recording SRF1 request carries captured DATE in its header, optional waveform/FFT bytes before injected image bytes, and both historical defaults. Waveform/FFT sample arrays remain outside project and request JSON; the request header carries their publication metadata.
- Follow requests now explicitly carry the current optional audio publication. Both native callers forward the same publication used for their two side renders; None clears audio before the Follow chain. DATE and image-frame isolation remain intact.
- The image-input editor and Audio assignment behavior coexist. Audio MAX and the shared texture limit have accurate EN/JA explanations; browser fixtures no longer describe audio as unsupported. The duplicated parent no-controls hint is removed. T34's tested immediate removal/live-action behavior is preserved.

### Combined regressions

| Regression | Verified invariant |
| --- | --- |
| `isf_audio_combines_named_images_date_and_multipass_without_stale_uploads` | One shader uses an authored Layer 2 image binding, waveform, FFT, fixed DATE and a named two-pass target. Live pixels are `[128,64,191,128]`; changing only the image under the same audio identity yields `[48,64,191,128]`; stopping audio yields `[48,128,0,128]`. Missing required images fail, then a genuinely unbound image renders transparently. The same runtime also renders an audio-only generator. |
| `isf_audio_counts_against_shared_texture_budget_and_accepts_generators` | Four images plus twelve audio inputs are accepted; four images plus thirteen audio inputs are rejected. Zero-image audio generators are accepted. |
| `isf_follow_audio_request_is_explicit_and_silence_does_not_inherit_previous_render` | Fixed DATE stays visible while a Follow shader changes from live `[64,191,128,255]` to explicit silence `[128,0,128,255]`, back to the same publication, and then to silence for invalid audio. |
| `render_request_keeps_external_frame_bytes_binary` | DATE, audio and RGBA round-trip in one real SRF1 envelope. Pre-DATE, pre-audio and both-missing headers retain their intended defaults and binary frames. Outer/nested unknown fields, truncated or nonfinite audio, an audio-header/body mismatch and an oversized request length are rejected. |

### Required debug gates

All Cargo commands used `pcargo-here.cmd` from this worktree, with `CARGO_TARGET_DIR` removed, the pinned MSVC 14.44.35207 linker, worktree-local TEMP/TMP and existing FFmpeg DLLs. `check --workspace --all-targets` passed with zero warnings. Every test command below also completed with zero warnings.

| Test command passed to the wrapper | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| `test -p video isf -- --test-threads=1 --nocapture` | 66 | 0 | 0 |
| `test -p video c1_ -- --test-threads=1` | 27 | 0 | 0 |
| `test -p protocol -- --test-threads=1` | 307 | 0 | 0 |
| `test -p engine isf -- --test-threads=1` | 21 | 0 | 0 |
| `test -p engine audio_control -- --test-threads=1` | 7 | 0 | 0 |
| `test -p syndocal --bin syndocal -- --test-threads=4` | 2209 | 0 | 23 |
| `test -p io --lib -- --test-threads=1` | 255 | 0 | 4 |

The protocol total comprises 295 unit tests, 8 integration tests and 4 doctests. All 66 selected video ISF tests ran without ignored cases, including the new mixed GPU and Follow regressions; the FLOAT test reported `FLOAT32_FILTERABLE=true`.

### Frontend and browser gates

| Gate | Result |
| --- | --- |
| `check-ai0-source-coverage.mjs` | Passed: Tauri 588, Engine 286, Remote 116, MIDI/OSC/DMX 240, Frontend 508, Keyboard 33, UI literal invokes 348 and 6 authority gates. |
| `check-fx-catalog.mjs` | Passed: 14 built-ins and 38 bilingual rejection fixtures, including the combined texture budget and current audio MAX guidance. |
| `check-fx-parameters.mjs` | Passed. |
| `check-audio-control-mapping.mjs` | Passed: 19 checks. |
| `check-destructive-actions.mjs` | Passed; T34 confirmation removals remain effective. |
| `check-localization.mjs` | Passed: 4148/4148 static Japanese labels, zero unprotected user-data labels. |
| `pnpm exec tsc --noEmit` | Passed. |
| `check-fx-ux-browser.mjs` | Passed on Vite 5480 / CDP 9480: existing FX flows plus image bindings, generators, current audio rejection, EN/JA and two themes. |

Validation logs and the command-result manifests are under `target/t32b-validation/`. Seventeen browser screenshots are under `target/qa/t32b-fx-ux/`; the English image-input editor and Japanese audio-MAX rejection screenshots were also inspected visually. These ignored artifacts remain local worktree evidence.

Independent read-only reviews covered GPU binding order and resource ownership, audio identity/silence, DATE and native context propagation, recording compatibility and bounds, image UI and T34 behavior. No blocking finding remains in those reviewed areas. The all-targets check also exposed missing cross-branch fields in old test fixtures; their explicit silent audio contexts and image-binding projections were updated before the passing gates above. New or changed test functions were formatted independently; no module/import reorder was applied to unrelated code.

Ignored counts are the top-level runner statuses. Some native subprocess helpers are registered as ignored for direct discovery and are exercised by parent tests. Optional device/environment cases were not enabled. No product application session, native/real-app harness, release build, push or merge into main was run. Physical output, live device behavior and native file-dialog acceptance remain outside this validation. Main independently advanced to `7d15227d` (T33) after this merge began; this evidence is for the handoff-selected `cbae80d3` integration.
