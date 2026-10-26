# Native frame attribution - task44 - 2026-10-25

**Campaign status (env06): all four native captures and safe exits COMPLETE; observer acceptance NOT MET.** Fresh OFF→ON→ON→OFF yielded 26.7422→27.4792→26.6494→23.3623fps, 103 polls each, all eight foreground readings matched. Pooled ON/OFF difference +8.0214% and OFF spread 13.4905% exceed the fixed 5% threshold; the difference is inside OFF spread. The nine-group exclusive table exists for 3,206 complete Presented parents but is diagnostic only, not an accepted ADR S0 product time budget. All four Stop/normal-close/7PID/9273 gates passed; normal8, restored isolated8, archive8 and backup8 hashes match. Source9/control-plane/foreign15 unchanged, recorded HEAD/local origin7e618cc is the metadata plus DMX CSS checkpoint; the same saved 7b-based exe was used without rebuilding. **60fps not met.** Earlier attempts remain historical and are excluded. Campaign report: RESULT-06 and env06 below. No optimization, stage/commit/push occurred in that campaign.

## Task48b inactive-path correction - 2026-10-26

Checkpoint base: shared `main` at `f5cd38404dbf6a5f225cb8ceda71688646b98bdd`.
All ten owned files matched RESULT-06 SHA256 values before editing. Task48's
independent review blocked the commit because feature ON with the output env
absent still entered TLS/RefCell at per-frame hooks.

The startup configuration now sets a process-lifetime atomic enable flag once,
before any render/decode worker dispatch. With the feature compiled in but
`SYNDOCAL_QA_FRAME_TIMING_OUT` absent, every hot TLS access seam first performs
one relaxed flag load and returns before ACTIVE, clocks, decoder diagnostics,
TLS/RefCell or buffers. This includes span/metadata hooks, parent admission and
completion, and the main per-frame demand counter checks through
`recording_active`. Feature OFF still compiles out the module and call sites.
The flag is not ACTIVE: Stop only closes generation admission, so already
admitted parents and their late spans still finish in enabled processes.

The disabled regression runs in an isolated test process, asserts zero TLS
initializations after exercising all hooks, then holds an exclusive RefCell
borrow while exercising them again to prove that existing TLS is not touched.
Enabled capture tests retain their lifecycle, nesting, overflow and post-ACK
export assertions; their test-only enable helper never participates in product
startup. Independent review approved the stable nine-source hashes with no
remaining blockers (`target/qa/task48b-attribution-inactive-gate/independent-review.json`).

The active trace format, finite capacity, canonical encoder/FRAMEINDEX,
cancellation/LastValid policy, product cleanup order, single handler and IPC
inventory are unchanged. No native campaign rerun is required by task48b and
none was performed; the saved native campaign remains evidence for its recorded
binary only. Observer acceptance and 60fps remain unresolved. No optimization,
hardware output, product version change, or cache cleanup is part of this fix.

Fresh task48b checks (release, serialized tests):

| Configuration / filter | Passed / ignored | Actual exit | Rust warnings |
| --- | ---: | ---: | ---: |
| OFF video `isf_` | 94 / 5 | 0 | 0 |
| OFF video `c1_` | 27 / 0 | 0 | 0 |
| OFF video `cancel` | 19 / 0 | 0 | 0 |
| OFF video `parallel_decode_tests` | 14 / 1 | 0 | 0 |
| OFF video `gpu_surface::tests` | 6 / 0 | 0 | 0 |
| OFF app `control_plane` | 123 / 0 | 0 | 0 |
| ON video `native_frame_timing` | 8 / 0, plus 1 isolated child execution | 0 | 0 |
| ON app `control_plane` | 123 / 0 | 0 | 0 |

Raw commands/logs and actual exits are in
`target/qa/task48b-attribution-inactive-gate/checks.json`. All eight attempts
verified vcvars MSVC 14.44.35207, the exact absolute Build Tools linker pin and
PATH-first resolution before Cargo. Historical task48 warning baseline / fresh
task48b current / delta are **0 / 0 / 0** for these OFF/ON configurations; no
separate baseline build was run. The initial PowerShell 5 harness invocation
exited 1 before Cargo because its BOM-less UTF-8 script was decoded as ANSI;
explicit UTF-8 ScriptBlock loading resolved it. vcvars' `vswhere.exe` diagnostic
does not prevent initialization/pin verification and is not a Rust warning.
`git diff --check` and staged whitespace checks pass; Git's
line-ending advisories are separate from compiler warnings. The known foreign
15 files, control_plane and normal/isolated executables retain all 18 protected
hashes. The checkpoint stages only the ten owned attribution files on `main`.

## Current design and resolved inventory blocker

The supervisor declined control-plane expansion. All four QA Tauri commands and the second handler were removed. Raw main.rs now contains one canonical `generate_handler!` whose complete body matches HEAD. `control_plane.rs` is byte-identical to HEAD (SHA256 `16670a4666c568c58cbd4edf54690bb9d995ae570ed33f75b5f5affc167ec4f1`); command inventory/classification was not edited.

`qa-native-frame-timing` remains opt-in in both crates, without dependency/default changes. An instrumented process records only when its startup environment contains `SYNDOCAL_QA_FRAME_TIMING_OUT=<absolute new JSON path>`. No env means inactive; feature OFF compiles out hooks. The first native Display iteration starts once; existing worker Stop closes admission while an open parent can finish. Normal close's existing ExitRequested boundary must succeed, then fresh Ready/Standby/all-deny, empty retirement labels, retired trace owner and ACK are required before the one cold export attempt. Serialization/file I/O happens after the measured Standby Stop, without new IPC or thread. Existing-file overwrite is refused.

One monotonic/UTC calibration (midpoint and uncertainty) supplies absolute timestamps for each parent. Warmup/measurement selection is external and post hoc. The primitive store is **3072 parents x 64 children, 8 ISF attempts/parent; 3,735,624 bytes** (headless and env06 native measured; excludes cold export allocations/RSS). Overflow/partial flags invalidate attribution. Env06 ON captures retained 1,819/1,777 parents without overflow. 3072/65 = 47.26 parents/s before extra startup time; this is not a guarantee that 65 seconds at 60fps fits.

Code locations in the env06 source before task48b: main.rs:13703 (Stop), 84309 (post-boundary export), 85426 (first iteration), 136305 (startup env), 137512 (ExitRequested); native_frame_timing.rs:199..313 (automatic lifecycle), 333..362 (clock calibration), 442..450 (idempotent Stop). Existing thin render/decode/ISF/presenter hooks preserve production paths and owners.

## Fresh env02 headless checks

Evidence: `target/qa/native-frame-attribution-20261025/env02/`. All **21 attempts** have actual exit 0, verified MSVC14.44.35207 vcvars/absolute linker/PATH-first, stable nine source hashes plus unchanged control_plane, and Rust warnings 0. `headless-summary-02.json` contains complete raw test-name lists, counts and receipt/log hashes; original receipts/logs and source snapshots are preserved. **452 passed / 0 failed / 12 ignored executions**, with overlaps across filters/features; not unique test counts.

| Filter/group | Feature ON passed / ignored | Feature OFF passed / ignored | Actual exit |
| --- | ---: | ---: | ---: |
| Trace lifecycle + actual 3x3 probe | 8 / 0 | Compiled out | 0 |
| control_plane::tests substring | 51 / 0 | 51 / 0 | 0 / 0 |
| isf_ | 95 / 5 | 94 / 5 | 0 / 0 |
| c1_ | 27 / 0 | 27 / 0 | 0 / 0 |
| cancel | 20 / 0 | 19 / 0 | 0 / 0 |
| parallel_decode_tests | 14 / 1 | 14 / 1 | 0 / 0 |
| gpu_surface::tests | 6 / 0 | 6 / 0 | 0 / 0 |
| Five existing Stop/retirement/Standby tests | 5 / 0 (five attempts) | Not selected | all 0 |
| output_pacer / live / native_display_ | 2 / 0; 2 / 0; 11 / 0 | Not selected | all 0 |

Each inventory invocation includes **16 core control_plane tests** plus matching authored/project control-plane tests. No test assertions were changed. The six ignored benchmark kinds are targetless batch, independent batch, backend, phase, mapped readback, and task20 decode-demand (the latter requires explicit media/result env); each occurs ON/OFF. They are not pass claims. Expected invalid-media FFmpeg stderr belongs to negative fixtures, not compiler warnings.

`headless-probe-02.json`: one actual 3-layer x 3-stage GPU batch, 65x3 padded-pitch input, FRAMEINDEX 0/1/2, 17 complete children, flags/overflow 0, exact exclusive coverage and mapped timestamp consistency; calibration uncertainty 1us. This is correctness evidence, not native performance.

Independent reviews: `source-review-02.md`, `headless-evidence-review-02.md`, `harness-review-02.md`. Headless reviewer independently matched all raw tests/counts/hashes and found no remaining source/headless blocker. Native acceptance is separate.

## Previous env02 native attempt, explicit stop boundary

Same instrumented exe intended for OFF->ON->ON->OFF, fixed 5s warmup + 60s and **5% observer threshold**. One isolated Tauri build completed, actual session **50928 / exit0**, MSVC pin/PATH-first verified. Exe SHA256 `63047e2aa815685419779ac5747daef9c6ec15eed7c61e2959f10dc453b71557`. Rust warning baseline/current/delta **0/0/0** for configurations run; native Vite advisory **1/1/0**, same 530.35kB App chunk. Baseline warnings refer to archived prior checks/build, not a fresh product baseline build.

OFF01 PID **24832**, instance `cdd0884511e51c7776d3939b965bdf24`, one responsive maximized QA window, viewport **1280x752 / DPR2**. Preflight passed Ready/Standby/empty routes/health. Arm and local Display returned receipts; harness advanced past first successful delivery. The native Display foreground condition did not become accepted within 30s. Capture session **42432 / exit1**, `samples=[]`, no warmup or measurement interval. The operator's later activation targeted an already-closed Display and returned WINDOW_NOT_FOUND; no redispatch or guard relaxation followed.

| Run | Result | Measurement polls | Exported parent population | FPS / observer |
| --- | --- | ---: | --- | --- |
| env02/off01 | Foreground deadline before warmup | 0 | Inactive; no trace file | N/A |
| env02/on01 | Not started | N/A | Not captured | N/A |
| env02/on02 | Not started | N/A | Not captured | N/A |
| env02/off02 | Not started | N/A | Inactive run not started | N/A |

PROPOSAL-01 section 4 requires safe shutdown/evidence preservation and stopping when matched conditions fail. The remaining runs were not dispatched. `analyze-02.mjs` was syntax checked but not run on missing evidence. ON/OFF impact, OFF spread, native path counts/overflow, all phase p50/p95, same-interval decoder/FPS and 60fps budget acceptance are **not evaluable**. Requested Display2560x1600 is the fixture target; no measurement poll establishes actual geometry in this failed run.

Every phase remains N/A in native: retention/join/recv/demand; chain preparation/queue; ISF upload/encode/submit/map-wait/mapped-copy/unmap/cleanup; CPU composition; LastValid lookup/clone/replace; presenter acquire/reconfigure/upload/encode/submit/validation/present; per-parent residual and outside-parent gap. No old trace/headless/candidate FPS or percentile was substituted.

## Safe exit and preservation

Existing Stop completed in **545.3134ms**, Ready/Standby, video/lighting false, live/test windows closed, empty health/routes/live_sources. First after-close check (session56038, exit1) found the normal close confirmation still open; `after-close-attempt-01.json` preserves it. The observed exact QA dialog's **Discard, Stop and Close** was selected; no forced termination. Final close check **session13071 / exit0** confirms all recorded PIDs **24832/56548/56192/52864/29788/38344/39432** and port9273 gone. stdout/stderr both 0 bytes; OFF trace absent.

`runtime-restoration-02.json` (**session79130 / exit0**) verifies:
- Protected normal runtime **8/8 unchanged**, exe `232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.
- Instrumented runtime **8/8 archived** under env02/instrumented-runtime, exe `63047e2aa815685419779ac5747daef9c6ec15eed7c61e2959f10dc453b71557`.
- Isolated runtime **8/8 restored** from the protected pre-task backup, exe `d2fd36c5622513ebef19f0e295dab888d793197c6b9060e1fb8f5a7cc037307f`.
- Nine sources/control-plane unchanged, compiler and owned native process empty, no relaunch after restore.

`native-attempt-summary-02.json` and independent `native-failure-review-02.md` distinguish the foreground failure from confirmed Stop/teardown. Preflight-to-capture UTC differs by **2h17.018s**; the cause (real elapsed delay versus wall-clock change) is unverified. Stop/capture internal monotonic/wall intervals agree. No synthetic two-hour correction or cross-run aggregation was applied.

## History and remaining work

Parent campaign `RESULT-01.md` preserves the prior multiple-handler failure (0samples), E0282 compile attempt and original runtime restoration. Its control-plane expansion proposal was rejected and is superseded by the no-IPC design. The old QA text is archived as `env02/qa-note-before-env02.md`; this is a new attempt, not reuse of failed OFF01 data.

Full current report: `target/qa/native-frame-attribution-20261025/RESULT-02.md`; final state/source hashes/check exits: `env02/FINAL-STATE-02.json`. Remaining gates: fresh complete four-run foreground-controlled comparison; actual ON automatic first-iteration capture and post-ACK normal-close export; native UTC/poll alignment/coverage/capacity; observer5%/OFF spread and same-parent phase attribution. Feature-OFF application was compiled/tested, not separately native launched. Device loss/real OOM/deep recovery/long runs/other platforms/physical outputs remain outside this evidence. No DMX/audio/external output was started. No optimization or 60fps claim; stop here.

## Final workspace boundary

At 2026-10-25T13:57:33.936Z, final Git inspection found **15 newly dirty files outside task44 ownership** (frontend/checker work, two io files, and RELEASE_HYGIENE QA). No outside file was edited/reset/staged by this task. The original ownership whitelist assertion stopped with **session24468 / exit1**, preserved in `final-checks-attempt-01.json`; it was not relaxed into an acceptance pass. Read-only `workspace-state-on-stop-02.json` (**session17325 / exit0**) records all paths and confirms owned nine source hashes/control-plane unchanged, fixture/media/config hashes matching, normal/archive/restored runtime hashes all matching, no compiler/native/measurement process and no port9273 listener. Git diff --check and staged diff checks exited0; index empty, HEAD/local origin remain7b. Remote was not queried. The shared working tree is not clean or exclusively owned. No additional build/capture was started.

## Env03: authorized restart, harness changes and startup failure

The supervisor identified those 15 files as known Codex task45/45b changes and authorized preserving them while continuing. `env03/handoff-03.json` freezes their hashes, the nine task44 sources, unchanged control_plane, normal/isolated/archive eight-file sets, and the empty native/compiler lane. Shared-tree non-exclusivity is recorded; it was not treated as a new blocker. No product source was edited and no new build/test was run in env03.

The new campaign plans retain **the same saved feature-on exe `63047e2a...`, OFF->ON->ON->OFF, 5s+60s, and 5% observer threshold**. All input hashes match env02. Full foreground reads now include expected PID/title and actual HWND/title/PID, read clocks, match, and phase in both evidence and an immediate JSONL. The exact predicate, 30s deadline and before/after same-HWND requirement are unchanged. Added writes/read records are outside the timed capture. Helpers passed Node2/PowerShell3 syntax checks (session61598, exit0); analyzer syntax passed (session98867, exit0). Independent static review is `env03/independent-review-03.md`.

At **2026-10-25T14:17:20.792Z**, launch session81476/exit0 recorded PID15200, the exact instrumented exe, responding main title Syndocal and HWND1839872. Preflight session23299/exit1 then found no port9273 listener. Subsequent live OS checks found PID15200 still alive but no WebView children and no Syndocal main window. The owned top-level window inventory contained only the single-instance helper, Tao event target and IME windows. stdout/stderr were both 0 bytes; the narrow Application-error query returned no matching events. The cause of the main-window disappearance is **unverified**; a launch receipt alone did not establish a durable native surface. Raw retained terminal results are in `env03/off01/startup-failure-03.json`.

This failure occurred **before the foreground predicate was read**. It is neither evidence of background Display nor a foreground predicate mismatch. No Arm/Display mutation, warmup, measurement, Stop timing or trace export was dispatched. Do not merge env02's foreground timeout into this failure or call either attempt an observer run.

| Env03 run | Launch / preflight | Polls / selected parents | FPS / observer |
| --- | --- | ---: | --- |
| off01 | exit0 / exit1; main surface and debugger absent | 0 / N/A, inactive | N/A |
| on01 | Not launched | 0 / not captured | N/A |
| on02 | Not launched | 0 / not captured | N/A |
| off02 | Not launched | 0 / not captured | N/A |

## Env03 S0 attribution boundary (ADR-01 section 10.2)

`env03/analyze-03.mjs` is ready but **was not run on missing evidence**. It assigns each child-exclusive interval once and asserts per-parent conservation before R7 percentiles. RetentionJoin/Recv are assigned by their nearest Retention/Demand ancestor. Inclusive whole-parent/Prepare diagnostics and outside-parent gap are separate. No historical or headless number fills the missing native cells below.

| Nonoverlapping group | Native exclusive p50 / p95 ms | Included detail |
| --- | --- | --- |
| Retention | N/A / N/A | Retention subtree, including its join/recv |
| Demand | N/A / N/A | Demand subtree, including its join/recv |
| ISF upload/encode | N/A / N/A | Upload, encode, finish/submit host wall |
| ISF completion/copy | N/A / N/A | Map/poll/receive, mapped allocation/copy, unmap, cleanup |
| Queue | N/A / N/A | Queue publication/retirement |
| CPU composition | N/A / N/A | Whole CPU composition exclusive wall |
| Output LastValid | N/A / N/A | Lookup, clone, replace |
| Presenter | N/A / N/A | Acquire/reconfigure/upload/encode/submit/validation/present |
| Other | N/A / N/A | Chain preparation, Prepare/Isf-exclusive bookkeeping, parent residual |

Ordinary/Batch2/Batch3/Recovery/Decline distribution and native ON retained bytes are **not measured**, not zero-valued ON observations. OFF has no trace file/environment. The existing headless primitive-store measurement **3,735,624 bytes** and cap **3072 parents** remain design/reference evidence only; actual 5s+60s capacity, native coverage and overflow remain unverified. ON/OFF difference, OFF spread and the 5% acceptance decision are N/A. **ADR S0 must not consume this attempt as an accepted current time budget.**

## Env03 cleanup, restoration and stopping

`cleanup-startup-03.ps1` revalidated the recorded PID/path/start time/exe hash, inventoried descendants, requested CloseMainWindow, and waited five seconds. The process remained; only that exact owned QA PID was terminated. Session90292/exit0 and `off01/startup-cleanup-03.json` explicitly record **forcedExactQaProcessTermination=true, normalCloseGatePassed=false**. Recorded PID15200 and port9273 are gone; no WebView descendants were present in the pre-cleanup inventory. This is failed-startup cleanup, **not** proof of normal close/Stop/retirement ACK. Standby/empty health/routes could not be read because preflight never connected.

`env03/runtime-restoration-03.json` (session57985/exit0) verifies isolated runtime8 restored to exe **`d2fd36c5622513ebef19f0e295dab888d793197c6b9060e1fb8f5a7cc037307f`**, normal8 unchanged at **`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`**, archive8 unchanged at instrumented exe **`63047e2aa815685419779ac5747daef9c6ec15eed7c61e2959f10dc453b71557`**, and original backups8 intact. Source9/control_plane/known foreign15 hashes match; no owned native/compiler/measurement process or listener existed before restoration. No relaunch followed.

The 21 env02 headless attempts, 452 pass/12 ignored executions and native build/warnings above are reused frozen-source evidence, **not env03 reruns**. Env03 invoked no compiler, so it has no new Rust warning measurement. Its syntax/diff checks emitted no compiler warnings; Git line-ending advisories are separately recorded. Fresh four-run native comparison, automatic ON capture/export, controlled phase attribution, Stop/normal-close gate, and 60fps remain incomplete. Existing failure/trace artifacts and all unrelated files are preserved. Full report: `target/qa/native-frame-attribution-20261025/RESULT-03.md`. Stop here; no retry, optimization, stage/commit/push.

## Startup investigation 04: no reproduction, conditional restart withheld

The supervisor authorized a bounded startup investigation and one restart per run only if a non-instrumentation cause could be established before any output dispatch. The outcome is **root cause unverified**, not a finding that instrumentation is exonerated. No new OFF→ON→ON→OFF campaign or retry was started. Full evidence and limitations: `target/qa/native-frame-attribution-20261025/env03/startup-window-loss-investigation.md`.

`startup-inspection-04.json` (41033 exit0) found no attributable WebView crash event/dump in the performed checks. The only Application/WER event was unrelated Ableton RADAR_PRE_LEAK_64; no WebView log/provider was registered, Crashpad reports/attachments were empty, and no .dmp/.wer was found. A metrics-file timestamp near14:17:20 indicates profile activity only. Direct access to QA AppData crash-reports and reviewer Cargo registry source was denied by approved-read-root restrictions and was not bypassed. No observed residual app/WebView/listener established a single-instance conflict. Static review found unchanged setup/single-instance code and no new startup window teardown in the env-absent instrumentation path.

| Startup-only diagnostic | Launch / preflight actual exit | Main / children / listener | Close |
| --- | --- | --- | --- |
| Saved non-instrumented d2fd36c…, PID39816 |15122:0 /1908:0 | Main persisted after launcher;7PIDs,9273; Ready/Standby, empty output | Existing close dialog;67336 command1:0; all7/port gone |
| Instrumented63047e2a…, env absent, PID59700 |4387:0 /20438:0 | Original quick-ready predicate; main persisted after launcher;7PIDs,9273; Ready/Standby, empty output | Existing window-close command/dialog;32242:0,36988:0,96485:0; all7/port gone |

Both exact instances were maximized1280×752/DPR2. No output geometry or Stop latency was measured. Instrumented cleanup encountered Desktop focus rejection before any input (requestedHWND15863458, actual66990); existing native close/UI was used after exact PID/instance/Standby checks. This is neither startup-loss reproduction nor Display foreground success. The baseline preflight/close spans an unexplained host/session date gap, which is not treated as timing evidence.

Before instrumented installation,2038 exit1 correctly stopped on base drift without copying. The new ignored04b helper verifies exact7f HEAD, direct parent7b and exactly9release-ledger/checker/documentation paths, while retaining all source/runtime/process/destination guards. Build provenance remains7b. Independent hash review79925 exit0 preceded installation67009 exit0. Known foreign15 changes remain protected.

Restoration87487 exit0: isolated8/8=d2fd36c…, protected normal8/8=232eb5c…, candidate archive8/8=63047e2a…, original backup8/8 unchanged; source9/control-plane/foreign15 match. No relaunch after restoration. Prior failedenv03 forced cleanup remains a failed normal-close gate; the two new startup-only normal closes do not replace active-output Stop/retirement acceptance.

**The nine S0 groups in the table above remain N/A/N/A.** Ordinary/Batch2/Batch3/Recovery/Decline counts and native ON retained bytes are unmeasured. No active-ON0record result exists. Headless3735624bytes/3072parents are reference only; native capacity, overflow, UTC/poll coverage, observer5%, OFF spread and attribution remain unverified. The previous21attempts/452pass/12ignored and build warning counts were not rerun. No compiler was invoked in investigation04, so new Rust/Vite warning measurements are N/A. Final state, evidence receipts and full report: `env03/FINAL-STATE-04.json`, `target/qa/native-frame-attribution-20261025/RESULT-04.md`. Stop here.

## Env05: accepted startup risk, failed after-capture foreground gate

The supervisor explicitly accepted the unresolved startup-loss risk and requested a new four-run sequence. New plans retain the same saved instrumented63047e2a… exe, fixture/media/config/profile,5s warmup+60s capture, observer5%, strict Display foreground predicate and Stop/retirement assertions. Startup attempts0–2 preserve per-attempt PID/exe/logs; any output-dispatch marker or accepted preflight forbids another startup. No product source, IPC or permissions changed. Limited helper-delta review63237 exit0 found no blocker; Node2/PowerShell4 syntax checks26695 exit0. Analyzer syntax17692 exit0; the complete four-run analyzer was not executed on incomplete evidence.

OFF01 started app43820 once. The nested launcher exited, but its startup wrapper remained waiting; after exact PID/parent/command checks, only wrapper47092 was terminated (wrapper session37745 exit−1). Original app43820 was retained and passed direct preflight44428 exit0. Its accepted launch and startup-history record attempt0/restart0. This is not a repeated app startup or a recurrence of the main/WebView/port disappearance. The wrapper evidence is preserved; future work should avoid that nested wrapper rather than replay it.

| Run | Actual result | Polls / app restarts | CounterFPS / observer |
| --- | --- | ---: | --- |
| off01 | capture55606 exit1; after-capture foreground mismatch |103 /0 |14.8110523704 diagnostic only /N/A |
| on01 | Not started |N/A /0 |N/A |
| on02 | Not started |N/A /0 |N/A |
| off02 | Not started |N/A /0 |N/A |

Before warmup, foreground matched Display HWND2428844/PID43820. After the full timed loop it was **Claude HWND1180380/PID52132**. These two raw observations and their clocks are retained in evidence and foreground-observations.jsonl. The transition time, mechanism and operator are unverified; neither user action nor a causal explanation of low FPS is inferred. A stale UI reference on an earlier Arm-dialog input and the subsequent observed Display confirmation are separately recorded in operator-events-05.json. There were no root UI actions after ready.

### Env05 diagnostic population, not accepted performance evidence

All103polls/102adjacent pairs were CPU-checked: monotonic clocks/counters, actual2560×1600, video allowed/lighting false, stable Ready epoch, native/decode/session/hardware errors0. Same first/last sample interval,2026-10-26 11:26:27.532–11:27:27.016 JST (raw02:26:27.532–02:27:27.016Z), is881deliveries/59.4826065s =14.8110523704fps; drop delta2689. This numeric integrity check does not override the foreground failure.

Same-interval decoder deltas:25,542,606µs/881calls (28.992742ms/call), requests/libav requests/successes2643 each, lookahead decode/hit/discard2637/396/0, reset15, open/eviction0/0, errors0. One call here groups3layer demand; background overlap and retention are not exclusive phase attribution. Lifetime deadline delta881/881miss; final bounded600/600miss,frame_budget_pass=false. Final prepare p50/p9567.9357/89.8277ms (600) and delivery interval70.8591/92.8332ms (599) are separate rolling populations, not full-interval or presenter-function measurements. No historical comparison or speed regression/improvement conclusion is made.

CPU audit27782 exit0: off01/failure-numeric-review-05.json/md. Root independently recalculated samples/deliveries/elapsed/FPS at88137 exit0. All raw samples are retained.

| ADR10.2 exclusive group | Native p50 /p95 ms |
| --- | --- |
| Retention |N/A /N/A |
| Demand |N/A /N/A |
| ISF upload/encode |N/A /N/A |
| ISF completion/copy |N/A /N/A |
| Queue |N/A /N/A |
| CPU composition |N/A /N/A |
| Output LastValid |N/A /N/A |
| Presenter |N/A /N/A |
| Other |N/A /N/A |

Ordinary/Batch2/Batch3/Recovery/Decline counts, native ON retained bytes, observer5% and OFF spread remain **N/A**: there is no ON population or second OFF run. OFF trace absent is not an active-ON0record result. Headless3735624bytes/3072parents remains reference evidence only. S0 has no accepted numeric budget; **60fps not met**.

### Env05 safe stop and final state

Stop535.8497ms returned Ready/Standby, video/lighting false, closed live/test windows, emptyhealth/routes/sources. Before-close52418 exit0, existing normal-close dialog45376 exit0, after-close39211 exit0 confirm all7recorded PIDs43820/58504/54492/42828/52892/58604/59812 and9273 gone. No app force termination. stdout/stderr0bytes; OFF trace absent. This safe Stop/close does not convert capturePassed=false into a performance pass.

Restoration75440 exit0: isolated8=d2fd36c…, normal8=232eb5c…, originalbackup8=d2fd36c…, candidatearchive8=63047e2a… all exact hashes. source9/control-plane/foreign15 unchanged; no app/compiler/measurement/listener before restoration, no relaunch after it. Full identities/status: env05/FINAL-STATE-05.json; actual command outcomes: env05/operation-sessions-05.json and receipts; current report: target/qa/native-frame-attribution-20261025/RESULT-05.md.

No fresh Rust test/build or compiler-warning measurement was made. The existing21attempts/452pass/12ignored and prior warning baseline/current/delta remain historical. No stage/commit/push, optimization, physicalDMX/audio/external output or continuation into the remaining runs. Stop here.

## Env06: complete four-run capture, observer not accepted

The supervisor reported that env05's Claude foreground observation coincided with an owner message and authorized a fresh uninterrupted campaign. This is recorded context, not a quantified causal explanation of 14.8fps. Env05 evidence remains intact and is excluded. Env06 retains the exact foreground predicate, 5s warmup/60s capture, max two startup/preflight-only restarts before any output dispatch, and immediate campaign stop after Arm-or-later failure. All four apps started once, attempt0/restart0; no failure/restart rule was exercised. Direct launcher and separate preflight avoided the prior nested-wrapper wait.

Same saved feature-on exe `63047e2aa815685419779ac5747daef9c6ec15eed7c61e2959f10dc453b71557`, eight DLL/runtime identities, silent H2641080p30, three layers × three stages, QA profile/config/fixture, log/legacy trace OFF and actual Display2560×1600 were verified. Only `SYNDOCAL_QA_FRAME_TIMING_OUT` differed ON/OFF. Source9 and foreign15 hashes remained frozen. HEAD/local origin is `7e618ccaca9d55e14dc9d32ec033a47044d98eff`; parent7f is the earlier metadata checkpoint, grandparent7b is build provenance. New HEAD differs from7f only in `app/src/styles.css`; the exact chain/path-set is guarded and that CSS is not rebuilt into the saved exe. No source edit/build was performed.

| Run | PID | Capture session / exit | Polls | Deliveries / elapsed seconds | Counter FPS | Stop ms |
| --- | ---: | --- | ---: | --- | ---: | ---: |
| OFF01 |11976 |41679 /0 |103 |1586 /59.3069945 |26.74220829 |516.5590 |
| ON01 |56896 |75106 /0 |103 |1626 /59.1720603 |27.47918514 |491.3955 |
| ON02 |62204 |5772 /0 |103 |1583 /59.4009903 |26.64938736 |515.9021 |
| OFF02 |54688 |87907 /0 |103 |1383 /59.1978736 |23.36232564 |525.7937 |

All412polls have monotonic time/cumulative counters, correct geometry, lighting=false and native/decode/session/hardware errors0. Eight foreground readings match the exact run PID/title and unchanged HWND. Main viewport1280×752/DPR2 and maximize were checked. FPS is calculated from the same first/last saved sample delta, not final rolling health. Each final bounded validation remains600/600miss with frame_budget_pass=false. Native capture integrity is distinct from the 60fps performance gate.

Pooled OFF=2969/118.5048681s=25.05382308fps; pooled ON=3209/118.5730506s=27.06348520fps. ON/OFF=(ON/OFF−1)×100=**+8.02137905%**; OFF spread=(maxOFF−minOFF)/pooledOFF×100=**13.49048663%**. Both exceed5%, and the ON/OFF difference is within OFF spread. **attributionAccepted=false; observer effect not identifiable.** No speed improvement or accepted S0 time budget is claimed. No extra measurement or optimization was started.

### Env06 ADR §10.2 exclusive diagnostic population

The native monotonic/UTC calibration uncertainty is1µs. Complete Presented parents wholly inside each run's first/last poll UTC, with1000µs+1µs margin at each edge, yield ON01 1625 + ON02 1581 = **3206parents**. Warmup/boundary/outcome exclusions total390. Each direct-child-exclusive interval is assigned once, nearest Retention/Demand ancestry owns its subtree, and parent residual belongs to Other. All3206parents conserve the nine-group sum exactly (max error0µs) before R7 quantiles. Percentiles across rows must not be added.

| Nonoverlapping group | Exclusive p50 ms | Exclusive p95 ms |
| --- | ---: | ---: |
| Retention |0.0140 |0.0170 |
| Demand |2.1150 |21.7740 |
| ISF upload/encode/submit |2.6250 |3.13025 |
| ISF completion/copy |17.5905 |20.1695 |
| Queue |1.8030 |2.19825 |
| CPU composition |5.2940 |5.98275 |
| Output LastValid |2.4865 |3.1115 |
| Presenter |1.2560 |1.5160 |
| Other |1.2090 |1.5660 |

Whole-parent34.9075/54.15125ms, inclusive Prepare32.6170/51.7520ms and outside-parent gap0.001/0.002ms are separate series. Subphase map/poll wait10.2695/12.89375ms and mapped copy7.2790/8.15725ms are host wall; neither is GPU shader time or a guaranteed removable cost. The p95 whole-parent tail's161parents have Demand mean22.71539ms and completion/copy mean17.87545ms, a same-tail observation only.

| Path/capacity | ON01 | ON02 |
| --- | ---: | ---: |
| All trace parents |1819 |1777 |
| Presented / Cancelled |1818 /1 |1776 /1 |
| Selected complete Presented parents |1625 |1581 |
| Selected Batch3 jobs |4875 |4743 |
| All Batch3 jobs |5457 |5331 |
| Excluded parents |194 |196 |
| Retained primitive bytes |3735624 |3735624 |

All3596trace parents are complete, flags0, unprepared0, overflow=false. Selected9618jobs and all10788jobs are Batch3; Ordinary/Batch2/Recovery/Decline each0observations. The two Cancelled parents are at Stop outside the selected interval. FRAMEINDEX continuity checks covered4872/4740job pairs. OFF has env absent and no trace, not active-zero. Native capacity sufficed for these runs; it does not establish capacity for65s at60fps or longer runs. Serialized JSON sizes9,204,859/8,994,109bytes are cold output, not retained primitive storage.

Same first/last decoder deltas (one call can group3layers): OFF01 wall7,545,077µs/1586calls; ON01 6,678,853/1625; ON02 7,614,651/1583; OFF02 12,801,960/1383. Means4.75730/4.11006/4.81027/9.25666ms/call. Reset15each; all error deltas0. These cumulative counter differences are not exclusive phases. Health/window/decoder reads are non-atomic; ON01 deliveries1626 vs demand1625 and ON02 deliveries1583 vs selectedparents1581 remain unadjusted. OFF02's higher demand cost is observed, its cause is unresolved.

### Env06 exit, preservation and evidence

Each Stop returned Ready/Standby, video/lighting=false, closed windows, emptyhealth/routes/live_sources. Each normal close used the existing guarded close command and observed confirmation; after-close checked all7recorded PIDs and9273 gone, stdout/stderr0bytes. Both ON traces were absent at Stop/before-close and present only after normal close, with complete/retired records. No forced app termination. Root restored isolated8runtime at session33287/exit0 to d2fd36c…; normal8=232eb5c…, archive8=63047e2a…, backup8=d2fd36c… all exact hashes. No relaunch followed.

Analyzer91052/exit0 (`analyze-06.json`, SHA256 `d778fc8b009d07fbaaf379030c7058ad3de385e7f9ab5dcc0a9d7bfa771e657e`) checked all samples/spans and reported0issues. Root independent FPS/conservation/hash audit36615/exit0 confirmed it. `analyzer-result-06.md` contains complete numeric boundaries; `operation-sessions-06.json`, all originalrun evidence and receipts, and `FINAL-STATE-06.json` preserve actual exits, PID/instance/foreground/hash checks. `RESULT-06.md` is the full report.

No new Cargo/Rust/native build/test was run. Frozen env02 evidence remains21attempts,452passed/0failed/12ignored executions with overlapping filters; Rust ON/OFF baseline/current/delta0/0/0 and Vite1/1/0 are historical, not new env06 warning measurements. Env06 Node5/PowerShell3 syntax12474/exit0 and analyzer syntax22283/exit0 passed. Git diff --check exited0 with the same5line-ending advisories, not compiler warnings. source9/control-plane/foreign15 and all25dirty paths remain preserved; index empty, no stage/commit/push. Remaining: accepted observer-controlled S0 budget, cause of OFF variation, prior intermittent startup loss, non-Batch3 native/recovery/device-loss/OOM/long-run boundaries and60fps. Stop here.
