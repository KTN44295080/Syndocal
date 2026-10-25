# Bounded independent ISF chains - 2026-10-25

Status: **Independent batch adoption approved; Task43 isolates the optional unknown-pitch regression for the commit/push checkpoint.** Shared `main` base: `cef51664d85b0c3d2af1904579a6fcf9d2c33834`. Review history: Codex task40 and Steroids REVIEW-01 **ADOPT-WITH-FIXES** → Task41 fixed F1–F3 → REVIEW-02 accepted F2/F3 but retained **F1-R1** → Task42 fixed backend pitch, passed focused checks and candidate04 bounded Windows native gate → Codex task42 **APPROVE** and Steroids REVIEW-03 **ADOPT**, no blocking findings. Reviews: `target/qa/steroids-isf-adoption-review-20261025/REVIEW-{01,02,03}.md`, `target/qa/isf-batch-fixes-20261025/{review-01,task42-review-01}.md`. Task43 authorizes staging exactly the ten sources and this note, then commit/push on `main`. No backend-selection change, physical DMX/audio or external-protocol output was performed; earlier native work used the explicitly requested isolated local Display fixture.

## Behavior and review fixes

Up to three independent resource-neutral Layer chains share one canonical encoder/submission/completion path. Inputs, scratch/readback, clocks and ownership remain separate; all maps complete before request-order CPU publication. Callback, sampled-layer dependency, executing Clip/transition, resourceful stage, invalid input and memory-pressure boundaries retain sequential execution. A declined batch reuses preparation and input Arcs. Feedback is never evicted to admit a batch. Error cleanup flushes uploads, drains/unmaps and balances Validation/OOM scopes while owners remain alive; a failed cleanup drain transfers owners into poison and blocks later GPU work. Original FRAMEINDEX, fault identity, cancellation and LastValid contracts remain intact.

- **F1 / backend-accurate upload reservation (Task42 F1-R1):** `isf_stack_workspace.rs:6` establishes the selected backend pitch once at runtime/device creation (`isf_runtime.rs:224`). Directly reviewed locked wgpu-hal 25.0.2: Vulkan uses the live adapter's `optimal_buffer_copy_row_pitch_alignment` via HAL/owning instance (query enabled on Windows/Linux/Android/FreeBSD); DX12 uses D3D12 pitch 256; Metal and GLES use 4. Reservation uses checked `align_up(width * 4, max(pitch, 256)) * height`, a conservative multiple of core 25.0.2's LCM(pitch, RGBA8 block size 4). Unknown backend, unsupported Vulkan query platform, unavailable HAL, zero/non-power-of-two/out-of-u32 pitch or arithmetic failure declines before upload/scratch movement (`isf_stack_transaction.rs:325`). Browser WebGPU/noop have no established bound. The `isf_` regression requires wgpu, wgpu-core and wgpu-hal all remain exactly 25.0.2 in Cargo.lock; a version change forces re-review. No per-frame query/copy/upload buffer/lock/thread or feedback eviction was added; one canonical encoder remains.
  The existing pitch-256 / 505 MiB oracle remains. New pitch-512 / 499 MiB oracle uses independent literals: cache 523,239,424; old three-job reservation 13,074,432; known simultaneous payload 19,267,584 bytes. Old total 536,313,856 fits 512 MiB; known total 542,507,008 exceeds it. The corrected conservative reservation is 19,365,888. Both pressure cases and the separately isolated unknown-pitch case assert no upload or submit (including cleanup) on refusal, unchanged cache/history/clocks/scratch/input references, and continued real feedback. Cache pressure is simulated accounting with small real feedback, not real high-memory/OOM. The actual headless Vulkan / AMD Radeon(TM) 8060S Graphics returned raw pitch **1**, reservation alignment **256**; normal 2/3-job batches submitted and matched pixels.
  Independent stable-source review: **APPROVE**, no actionable finding, `target/qa/isf-batch-fixes-20261025/task42-review-01.md`. Earlier REVIEW-02 was ADOPT-WITH-FIXES for F1-R1; F2/F3 were accepted there and remain unchanged by Task42. Locked source excerpts and hashes: `task42-locked-pitch-derivations.txt`.
- **F2 / allocations:** `isf_stack_transaction.rs:239,280,430` uses a single pending array, borrowed keys, inline results and reused scratch-list capacity. `isf_independent_runtime.rs:7,129` provides zero-to-three inline storage for pending/request/result containers; Layer grouping and outer stage/request adapters also use it (`lib.rs:6187`, `isf_chain_execution.rs:294`). No new crate was added. Resource-free stage metadata stays local (`isf_pass_runtime.rs:381`); nonzero stage state and all actual GPU images remain ticket-owned through cleanup. Keys become owned only for exceptional poison retention (`isf_stack_transaction.rs:169`). Ordinary chains borrow their resolved source; deferred chains move it, and labels clone only when a fault is created (`isf_chain_execution.rs:32,83,213,236`). Existing per-stage preparation, uniform/channel and output-pixel allocations remain; no allocator-count or timing-effect claim is made.
- **F3 / diagnostic boundary:** `last_stack_render_us` covers the complete diagnostic unit from encode start through CPU output/fallback copying, unmapping, feedback completion and error-scope cleanup. Single transactions and successful batches record after `finish_stack_transaction` (`isf_stack_transaction.rs:251,306,376`); ordinary/recovery wrappers record the complete chain including `current.into_owned()` (`isf_runtime.rs:493`). `last_stack_stage_count` describes the same unit: all stages across a successful batch, or the final complete ordinary/recovery chain. It is not submission-to-map-callback wait time. No separate batch-wait diagnostic was added. Runtime tests assert 2/3-job totals and recovery-chain counts; control-flow inspection establishes the timer endpoints.

## Task43 isolated unknown-pitch check

REVIEW-03 §2 P3 is addressed only in `isf_stack_transaction_tests.rs:256-335`; product code is unchanged. Three small 65×3 identity jobs and a real 4×4 feedback image use no simulated pressure. `Some(512)` first admits the same jobs, increases the actual submit counter and matches pixels. Changing only pitch to `None` then declines with no upload/submit (including cleanup), unchanged cache/history, resource clock/FRAMEINDEX, batch count, every scratch readback handle and input Arc ownership. Subsequent real feedback advances once. The existing independent pitch-256/505 MiB and pitch-512/499 MiB pressure oracles remain unchanged.

| Attempt / filter | Passed | Failed | Ignored | Actual Cargo exit |
| --- | ---: | ---: | ---: | ---: |
| task43-isf-01 / isf_ | 94 | 0 | 5 | 0 |

Command: `cargo test -p video --features libav --release --lib isf_ -- --test-threads=1 --nocapture`. Evidence: `target/qa/isf-batch-fixes-20261025/task43-isf-01.{log,json,cmd}` and `task43-isf-01-source/`. Required MSVC 14.44.35207 initialization, absolute BuildTools linker pin and PATH-first verification passed; FFmpeg remains pinned 8.1.2 SDK. All 21 archived sources match before/after hashes; the selected unknown-pitch test actually ran and passed. Five ignored tests are explicit performance experiments. Compiler warnings: archived baseline **0 / current 0 / delta 0**. The pre-existing vcvars `vswhere.exe` lookup diagnostic remains; initialization and the exact toolset/linker checks succeeded. Protected normal executable hash remained unchanged. `git diff --check` and staged diff check pass.

**No native rerun is needed or performed for this test-only change.** Candidate04 remains the earlier product-code native evidence; Task43 adds no native, physical-output, performance or 60fps claim.

## Task42 focused checks

Evidence: `target/qa/isf-batch-fixes-20261025/task42-*`. All attempts used the required release/libav/lib command and single test thread, exact BuildTools MSVC 14.44.35207 initialization, absolute linker pin and PATH-first verification; pinned FFmpeg 8.1.2 SDK. `--nocapture` records the queried adapter pitch.

| Attempt / filter | Passed | Failed | Ignored | Cargo exit | Qualification |
| --- | ---: | ---: | ---: | ---: | --- |
| task42-isf-01 / isf_ | n/a | n/a | n/a | 101 | Compile before tests: new top-level field missed the opt-in test constructor; moved cached pitch into runtime resource pool |
| task42-isf-02 / isf_ | n/a | n/a | n/a | 101 | Compile before tests: one multiline test access still used old field location; corrected |
| task42-isf-03 / isf_ | 94 | 0 | 5 | 0 | Final ten sources |
| task42-c1-01 / c1_ | 27 | 0 | 0 | 0 | Final ten sources |
| task42-cancel-01 / cancel | 19 | 0 | 0 | 0 | Final ten sources |

Counts overlap; the five ignored cases are explicit performance experiments, not rerun. `task42-focused-verification.json` verifies source stability, all 21 archived sources for every attempt, final source equality for successful attempts and log SHA256. Compiler warnings: archived baseline **0 / current 0 / delta 0** for every attempt. The vcvars startup emitted a pre-existing `vswhere.exe` lookup diagnostic, then initialized successfully; exact toolset/linker checks passed. Candidate04 native evidence is recorded below.

## Task42 candidate04 native gate

Evidence: `target/qa/isf-batch-fixes-20261025/candidate04/`; verification: `verification.json`, using the existing task41 harness with new plan/attempt paths only. Build command: `pnpm --dir app tauri build --no-bundle --config <existing isolated config>`, exact MSVC pin/PATH-first contract verified, exit **0**. Candidate SHA256 `d2fd36c5622513ebef19f0e295dab888d793197c6b9060e1fb8f5a7cc037307f`. One responsive exact candidate Syndocal window, PID **57400**, maximized before actions. Normal profile was not launched.

- Native01: exit **0**, passed, **103 samples**, all **2560 x 1600**. Native lifetime errors, decode failures, libav session/hardware errors: **0** in every sample; successful session stdout/stderr empty. No source drift between final checks/review/build.
- Descriptive rate: **2071 / 59.5871242 s = 34.7558 FPS**; dropped-frame counter delta **1506**. Prepare p50/p95 **26.0365 / 43.218 ms** (600 reported samples); present p50/p95 **28.1011 / 45.5613 ms** (599). Distinct populations; no speed claim. **60fps unmet**, `frame_budget_pass=false`, final validation misses **600/600**.
- Stop **576.314 ms**: Ready/Standby, video/lighting disallowed, no live/test window, empty health/routes/sources. Normally closed through the existing close confirmation; all **7 recorded PIDs** and port **9273** gone; all **8 protected normal runtime hashes** match. Normal exe remains `232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`. No force termination.
- Two preliminary teardown observation commands exited 1 while the close dialog was still open: an accessibility click had not activated its button. A fresh screenshot and coordinate click completed normal close; final verifier exit **0**, unchanged assertions. Arm also needed a fresh screenshot after an unavailable accessibility index; neither output mutation was redispatched. Native01 itself had no failure/retry. Details: `task42-recovery-notes.json`.
- Rust compiler warnings: baseline **0 / current 0 / delta 0**. Existing Vite >500 kB chunk advisory remains, unchanged; it is not a Rust compiler warning. No warning limit was altered.

## Task41 focused checks (historical)

Evidence: `target/qa/isf-batch-fixes-20261025/`. Every Cargo attempt used `cargo test -p video --features libav --release --lib <filter> -- --test-threads=1`, initialized `vcvars64.bat -vcvars_ver=14.44`, verified `VCToolsVersion=14.44.35207`, pinned the absolute BuildTools linker and verified it first in `where.exe link.exe`. FFmpeg remained `ffmpeg-pinned-8.1.2-sdk`.

| Attempt / filter | Passed | Failed | Ignored | Actual exit | Qualification |
| --- | ---: | ---: | ---: | ---: | --- |
| isf-01 / isf_ | ? | ? | ? | 101 | Compilation failed before tests: missing StackBatch import and Cow iterator adaptation; corrected |
| isf-02 / isf_ | 90 | 0 | 5 | 0 | Passed before final diagnostic-count assertions |
| isf-03 / isf_ | 90 | 0 | 5 | 0 | Final source |
| c1-01 / c1_ | 27 | 0 | 0 | 0 | Final source |
| cancel-01 / cancel | 19 | 0 | 0 | 0 | Final source |

Counts overlap and are not added. The five ignored cases are explicit performance experiments; none was rerun in this fix task. The 90 ISF passes include the new independent padded-upload refusal regression and existing ownership, pixels, clocks, failure recovery, feedback, cancellation and LastValid checks. `focused-verification.json` verifies all attempts' source stability, log hashes and 21 archived sources per attempt; final source hashes match isf-03/c1/cancel and the native build.

Compiler warnings: archived baseline **0 / current 0 / delta 0**, including the initially failed compile and native Rust build. The baseline log hash was reverified (`target/qa/isf-phases-20261025/isf-01.log`, `d74e04e0f64b65192235a9199f33c7ae035913f2b941b21140892135e5b1f9b6`). Native Vite's existing >500 kB chunk advisory remains **1 / 1 / delta 0**; the same `App-LKXd1Prh.js` is 530.35 kB in candidate01 and candidate03. It was not suppressed or counted as a Rust compiler warning. Final `git diff --check` is clean (exit 0).

## Task41 candidate03 native gate (historical)

Fresh evidence is under `target/qa/isf-batch-fixes-20261025/candidate03/`. Copied the existing `isf-batch-native-20261025` harness without modifying the old evidence; adapted only new evidence/attempt/launch references. The copied close verifier's log references were corrected to the second launch; `after-close-verified.json` is authoritative. Failed attempts and immediate pre-settlement teardown checks remain recorded in `recovery-notes.json`.

- `build-native.ps1`: actual exit **0**, `pnpm --dir app tauri build --no-bundle --config <existing QA config>`, isolated `target/readback-copy-20261024`; pinned linker/PATH-first checks passed. Candidate SHA256: `3bcd945c9cd0780a24fcf1e78b178d28c2eb082e7fb1429aec66e1a3f588ee09`.
- Both launches verified exactly one responsive exact-path `Syndocal` main window; preflight maximized it before UI actions and confirmed Ready/Standby with video/lighting disallowed. Successful session PID **38884**, port **9273**. No force termination was used.
- `native01`: actual exit **1**, zero samples. Arm was rejected after its lease expired while native confirmation awaited; launch-01 stderr records `outcome: Err(Expired)`. Stop returned Ready/Standby with outputs disallowed. `native02`: exit **1**, zero samples, same-WebView mutation-slot guard collision before acquisition. That session was normally closed; a fresh launch/preflight avoided redispatching any unresolved mutation.
- `native03` / capture-03: actual exit **0**, passed. All **103 samples** are **2560?1600**, native lifetime errors / decoder failures / libav session and hardware errors **0**. Successful-session stdout/stderr are empty. CPU verification of all samples and provenance exits **0** (`verification.json`).
- Descriptive delivery rate: **2080 / 59.5837531 s = 34.9088 FPS**; dropped-frame counter delta **1495**. Final prepare p50/p95 **25.6411 / 31.7486 ms** (600 reported samples), present p50/p95 **27.7389 / 34.9694 ms** (599 samples). These are distinct populations. **60fps remains unmet:** `frame_budget_pass=false`, final validation misses **600/600**.
- Stop **570.848 ms**: Ready/Standby, video/lighting disallowed, no live/test window, empty health/routes/sources. Normal close completed; all **7 recorded PIDs** and port 9273 are gone. Eight protected normal runtime file hashes still match. The first immediate close check preceded asynchronous teardown; the final observed-settlement check passes without relaxing assertions.

Protected normal `target/release/syndocal.exe` stayed `232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0` throughout checks/build/teardown. No native speed improvement is claimed from candidate03.

## Prior native numeric review retained

The saved `target/qa/isf-batch-native-20261025/native-numeric-review-01.md` reviewed **407 samples / 16 JSONs** in order **A1 ? B1 ? B2 ? A2**, with no geometry/error/Stop/teardown violation. These are prior observations, not new measurements of the F1?F3 source.

| Run | Deliveries / seconds | FPS | Stop ms |
| --- | --- | ---: | ---: |
| A1 / baseline01 | 1869 / 59.3120508 | 31.5113 | 630.599 |
| B1 / candidate01 | 1836 / 59.0894421 | 31.0715 | 549.405 |
| B2 / candidate02 | 1949 / 59.7723963 | 32.6070 | 494.715 |
| A2 / baseline02 | 1817 / 59.1533183 | 30.7168 | 601.068 |

**B1 is confounded** by foreground/window operations and a Patch-start-position click and remains retained. The aggregate +2.34% and cleaner one-pair B2/A2 +6.15% in that review do not establish repeatable general speed improvement. All four runs have `frame_budget_pass=false`; **60fps was not met**. Native visual equivalence, actual scanout cadence and physical output were not established. Prior headless phase/benchmark experiments remain under `steroids-isf-batch-20261025`; they were not rerun or substituted for this native gate.

## Task43 final source identity and remaining boundaries

Source-set SHA256: `3d462ebac40a47eb6cf0f5f03eb2ed0512c73ff54e0933d38ee05dc815a550d1` (lexically sorted relative path + NUL + lowercase file SHA256 + LF, UTF-8). Task43 changes only `isf_stack_transaction_tests.rs` from the verified starting set `44cdb1a5c0cb1f299fa7e54589164dfad10f13bedef4022e7c7daf44d019ca99`; the nine other files still match Task42 source review, successful focused checks and candidate04 build. Task41/candidate03 used the prior set `505032fe96ea3848254c33646df51bace79c36e3c39a200ca09e10d0a42091d9`.

| File under crates/video/src | Final SHA256 |
| --- | --- |
| `isf_chain_execution.rs` | `a3b8b6f5712acb3ec32e68a5d3a38474f17177ffe4d9d4d5b854aa2310d57ec9` |
| `isf_independent_layer_tests.rs` | `5c0662691138ddbcba70567aaaaa349469968b93e1ef56383c96642404eab9b2` |
| `isf_independent_runtime.rs` | `d3ee8f77f723610e2fa1dfc87d6031befa0e6167006e4f4f4fd0dd738a851eb4` |
| `isf_independent_runtime_tests.rs` | `42c6e86ec426f201d0c26dc187ece93cb977d853c667d2bc75e19d355d073a08` |
| `isf_pass_runtime.rs` | `38be7e1d969ad91841e0b6f44a6df78a4663a905fd06b8c506fb86c27ced6770` |
| `isf_runtime.rs` | `2e7a5638c9aee9b9bcebff18d8e7c765749ea58b1142616dff973871b0e2c419` |
| `isf_stack_transaction.rs` | `2f98d0459f2400189bfca74d6959f33b98bca2b610ce9209009301a4ec31bb9d` |
| `isf_stack_transaction_tests.rs` | `642a8ca3d2b608c4f2017fb35761549a8a4591fdf9961513eb1cbc2d8e719a6b` |
| `isf_stack_workspace.rs` | `840e91dad54191095d767e87d9fa3e211fa5d04abd48c8bc7b33b66d984c9625` |
| `lib.rs` | `0b65dc2b1fe7c9b1d1df5be41003eb8e2cf3e7131616659ef1e30f03d5b6724f` |

Remaining unverified: a real adapter reporting pitch >256 and cross-backend/platform execution; actual device loss, cleanup-wait failure, real high-memory/OOM and panic/unwind; the poison test injects retained state and the padding test simulates accounting. No allocator-count/actual RAM or VRAM peak measurement, long soak, other GPU/backend/platform matrix, native pixel-equivalence proof, physical monitor/receiver/protocol/DMX/audio or release/package acceptance. No 60fps acceptance or repeatable speed claim. No branches, tags, version bump or release are part of this checkpoint.
