# ISF support matrix — T23 — 2026-10-20

## Scope and conclusion

This audit covers the ISF filter path in `crates/video`, its native import/validation seams, and the FX rejection explanations. The comparison target is ISF JSON specification v2 and the official ISF documentation listed below. Syndocal remains a **partial ISF host**.

T23 adds ordered multi-pass rendering, named targets, persistent feedback, RGBA32Float targets, bounded per-frame dimension expressions, and shader-relative PNG/JPEG imports. Point2D, color and event controls already existed; their actual rendered output is now covered by small fixtures. Image helpers now use each sampled texture's dimensions.

The work is isolated to branch `task23-isf` in `wt-task23`, based on `036a268c01e7a6dcf8b41830e99e4593f2922817`. No application, native harness, release build, push or merge is part of this evidence.

### Official comparison sources

- **S1 — JSON attributes and inputs:** [ISF JSON Reference](https://docs.isf.video/ref_json).
- **S2 — Image helpers:** [ISF Functions Reference](https://docs.isf.video/ref_functions).
- **S3 — Standard variables:** [ISF Variables Reference](https://docs.isf.video/ref_variables).
- **S4 — Vertex stages:** [ISF Primer: Vertex Shaders](https://docs.isf.video/primer_chapter_5.html).
- **S5 — Passes and persistence:** [ISF Primer: Multi-Pass and Persistent Buffers](https://docs.isf.video/primer_chapter_7).
- **S6 — Specification and version context:** [mrRay/ISF_Spec](https://github.com/mrRay/ISF_Spec) and [Adapting GLSL to ISF](https://docs.isf.video/primer_chapter_9).

Sources were inspected on 2026-10-19/20 UTC. The status column describes the resulting implementation, including its explicit limits. “Supported” means the stated operation works within those limits, rather than a claim that arbitrary ISF packs are compatible.

## Passes, targets and resources

| Feature | Status after T23 | Implemented behavior and remaining boundary | Evidence |
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

All short Rust paths in this document are relative to `crates/video/src/` unless an `app/` prefix is present.

### Resource bounds

- Maximum 16 passes, 16 total sampled textures (the one image input plus targets/imports), and 16 scalar/vector controls.
- Dimension expressions: at most 1,024 ASCII bytes, 256 AST nodes and 32 nested parse levels.
- Imported images: at most 15, at most 16 MiB per encoded file, at most 8,192 pixels per axis, and at most 64 MiB of combined decoded RGBA per prepared shader. The total-texture limit can reduce the available import count.
- The prepared-shader cache and pipeline cache each retain at most 64 entries. Pipeline identity includes WGSL hash, target format and texture count. Persistent/import GPU instance resources are retained in a 64-entry LRU pool.
- Main texture/readback accounting is capped at 256 MiB per stage and 512 MiB across the pool and current encoder. Accounting includes both target textures, imports, every separately allocated unnamed intermediate, host upload/ping/pong and padded readback. Anonymous allocations accumulate until submission completes; a cache hit recalculates their cost for the current host size. Driver overhead and small pipeline/uniform allocations are outside this texture/readback estimate.
- LRU eviction, a failed stage that loses its resource state, changed target dimensions, changed shader/resource fingerprint, a new renderer, or a new project namespace can reset feedback. A failed submitted stack cannot publish new history. This is bounded live-renderer state, not project-persisted feedback.

The main checks and allocation ownership are in `isf_pass_runtime.rs:128`, `:163`, `:430`, and `:464`. Resource decode bounds are in `isf_resources.rs:9` and `:465`.

## Input types

ISF defines nine input kinds (S1). The filter API still requires exactly one dynamic image input.

| Input kind | Status | Behavior and limits | Evidence |
| --- | --- | --- | --- |
| `image` | Partially supported | Exactly one host frame. Multiple dynamic image inputs, native ISF transition inputs and zero-image generators are rejected. Imported images and pass targets do not count as that required dynamic input. | `isf_runtime.rs:506`; existing rejection and built-in tests. |
| `float` | Supported | Existing defaults/ranges and per-frame control resolution remain. Bounded values become float uniforms. | `isf_runtime.rs:759`, `:832`; threshold GPU regression; all 14 built-ins compile. |
| `bool` | Supported | Existing control value becomes a GLSL boolean using the established threshold. | `isf_runtime.rs:832`; parameter-model and built-in checks. |
| `long` | Partially supported | Existing integer controls and enumerated values/labels work; storage is a float control slot before GLSL int conversion, so arbitrary full-width integer precision is not guaranteed. | `isf_runtime.rs:759`, `:832`; FX parameter and built-in checks. |
| `point2D` | Supported | Two components, default and overridden coordinates. This was already implemented; T23 adds a rendered fixture. | `isf_completeness_tests.rs:251`; `point2d.fs`. |
| `color` | Supported | Four components including alpha, with existing defaults/ranges. This was already implemented; T23 pins all four rendered channels. | `isf_completeness_tests.rs:270`; `color.fs`. |
| `event` | Partially supported | Existing host pulse value is shared by all passes and stops affecting pixels once the host resets it. The renderer does not consume/reset the event itself after exactly one frame; existing host pulse timing is preserved. Preview audition keeps its existing event restrictions. | `isf_completeness_tests.rs:283`; `event.fs`; FX audition/UX checks. |
| `audio` | Unsupported | Clear prepare-time rejection with a suggestion to drive ordinary controls through Syndocal Audio FFT nodes. No placeholder waveform texture. | `isf_runtime.rs:506`; `app/src/fxRejectionExplanation.ts:53`; bilingual rejection fixture. |
| `audioFFT` | Unsupported | Same explicit rejection. Existing reactive-band values are not exposed as though they were ISF's channel/sample texture contract. | Same runtime/UI evidence; audio data-flow assessment below. |

### Why audio textures remain unsupported

The existing live publication exposes an `AudioSpectrumPoint`, up to 16 reactive bands, level/onset/beat features and generation/sequence identifiers. Its protocol structures contain no per-channel waveform or full FFT texture payload: `crates/protocol/src/lib.rs:358`, `:369`, `:420`.

Native publication converts analyzer output into those reactive features in `app/src-tauri/src/main.rs:48376` and `:48416`. The video effect context at `lib.rs:188` carries clip runtime and project epoch, not an audio texture feed. ISF's audio contract has channel rows and sample/FFT columns (S1). A faithful implementation therefore needs a new publication and renderer ownership path with explicit generation, channel and lifetime semantics. There is no clean existing texture hook in these inspected seams. T23 preserves the explicit unsupported result and the existing Audio FFT control alternative.

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
| `DATE` | Unsupported | No date/time uniform is injected; use produces a shader diagnostic. DATE is reserved against input-name collision. | S3; `isf_runtime.rs:727`, `:832`. |
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
| Rejection explanations | Updated, EN/JA | Removes obsolete “single-pass only” / “external images unsupported” guidance. Distinguishes pass sizes, flags/target configuration, FLOAT adapter support, vertex companions and image resource errors; preserves raw diagnostic text. | `app/src/fxRejectionExplanation.ts:65`; 20 new dictionary pairs in `app/src/uiLocalization.ts:28`; 24 bilingual catalog cases. |

## Rendered fixtures and regression evidence

Fixture directory: `crates/video/src/isf_fixtures/`. Image assets are generated in worktree-local test directories and removed by each test.

| Fixture / regression | Known result or invariant |
| --- | --- |
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

## Validation and review

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
