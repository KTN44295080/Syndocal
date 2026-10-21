# Runtime / native authoring checkpoint — 2026-10-21

## Status, authority and repository

This is the authoritative checkpoint replacing the original Web ChatGPT handoff. The user transferred implementation, validation and commit/push authority to Web ChatGPT using Chat On Steroids Core. The preceding read-only restriction was superseded. Codex and its subagents did not perform this continuation; independent reviewers used separate Web ChatGPT workers through Core.

The scoped runtime fixes, Recovery parser fix, focused regressions, isolated native functional checks, local Spout receiver, and final standard Windows build/startup gate passed as described below. **This is not full product/release acceptance:** the preexisting same-process post-Open authoring lease defect remains P2; the FX/4K workloads below did not meet 60 fps; physical DMX/audio/scanout, NDI runtime and macOS/Syphon SDK acceptance remain unverified.

- Original checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`, branch `main`.
- Base: `7e97b26ddfdf08c8516a5692b8e5b31abb76dc7a`. Git identifies the checkpoint commit; no hash-only follow-up commit is required.
- All 33 modified/new source, helper, checker and regression files plus this document belong to this task. Only these 34 files are intended for the checkpoint commit.
- Existing worktrees/branches were preserved: wt-merge (detached 7d15227d), wt-task32/task32-isf-date-images (e40d6bbd), wt-task33/task33-output-authoring (ad5fe649), wt-task35/task35-browser-checks (6d9e8efb).
- No reset, stash, extra worktree, product version bump, distribution, unrelated cleanup or unidentified device operation was performed.
- Evidence and scripts remain ignored under `target/qa/runtime-perf-2026-10-21/`; historical `target/qa/task20d/`, including original-storage.json, was preserved.

Below, `N` means `target/qa/runtime-perf-2026-10-21/native`, `P` means `N/ff4d7319-b2a5-4885-81a3-483822acd1c7`, and `O` means `N/8fdbc83d-b43c-491c-8309-8a091e61f082`. All paths are relative to the checkout unless stated otherwise.

## Implemented behavior

### Cancellation and bounded output retirement

- Libav uses the output owner's shared Stop plus a private decoder-retirement token. Boxed FFmpeg interrupt userdata remains alive until input close, including native open/probe error paths. Explicit packet reads avoid an iterator retrying AVERROR_EXIT indefinitely.
- Seek, decode, transfer, conversion and lookahead boundaries preserve Cancelled. Cancelled does not become CLI fallback, LastValid, black-frame success or GPU-to-CPU retry. Decoder Drop signals only its private token and joins retained sessions/receivers; the panic branch drops the native session before sender disconnect.
- Preferred decoder cancellation retains shared parallel batch decoding. The existing eight-worker limit was not increased.
- Canonical output-only Stop collection includes active, pending and quarantined outputs without consuming inputs, owners or leases. Reservation establishes all-deny and Stop before old permits drain. Real transport/native retirement precedes finish_retirement.
- External NDI/Spout/Syphon workers share the bounded retirement wait. Only finished owners are joined/released; unfinished owners and their leases stay retained. NDI's native SDK retirement acknowledgement remains part of lease settlement.
- Show Spout's two owners receive Stop before a shared one-second wait. Both joins, including SDK and Engine acknowledgement work, must finish before releasing the pair. Otherwise the complete pair, leases and failure fence remain pending. Late harvesting matches kind/pair/Stop Arc identity, preserves the original failure, and rejects overlapping fixed-name ownership through quarantine.
- Show state locks are not held across the new join/retirement path. Existing Show startup rollback and PendingDrop synchronous joins are outside this active/failed-output bounded-stop guarantee.
- Project/role transitions harvest completed owners before reservation. Native project-authority polling now revalidates an admission generation and project stamp, preventing an A -> temporary B -> A reservation race.
- The two inherited native compilation errors were fixed: keepalive authority changes use the canonical Stop collector, and the Spout stopped-send path returns the correct PhysicalOutputSendError type.

### Stop, presentation and Timeline Follow acknowledgement

- Native Display checks Stop around initialization, show, presentation and settlement. External send/fault acknowledgement and retry boundaries carry the exact worker Stop identity.
- Expected cancellation does not manufacture Follow Fault acknowledgements. A real earlier failure keeps its failure lease and original error, even when Stop arrives later. Unpublished pending acknowledgements remain retained.
- Engine exposes a stop-aware Follow settlement entry point; the previous API forwards through the same implementation. Direct runtime command enqueue removes the allocator-lock wait on this path.
- Stop is checked at entry, application and before the first state mutation. A stopped waiter checks in at most ten-millisecond slices within the existing three-second deadline. A definitive ready reply takes precedence; already applied KeepApplied receipts are preserved, not rolled back.
- Follow identity is captured once for ordinary settlement and separately only when an actual resize requires it.

### ISF batching without persistent-history/budget changes

- Only consecutive targetless, host-size, RGBA, resource-neutral stages are batched. TARGET/persistent, FLOAT, explicit dimensions, imported/bound/audio/dynamic/extra-image resource cases retain their existing transaction boundaries.
- Batching does not require extra persistent reservations or evict unrelated history. A retry covers only stateless stages at the same FRAMEINDEX/DATE. Cow avoids an unconditional initial frame copy.
- Pixel parity, persistent/history exclusions, texture budget behavior and cancellation paths are covered by the VIDEO regressions.

### Recovery schema admission

Real native QA exposed a separate existing mismatch: native ProjectFile is version 2, while the browser's Recovery storage guard accepted only project version 1. The application created a valid checkpoint that it then refused on restart.

`app/src/projectRecoveryStorage.ts` now accepts exactly numeric project versions 1 and 2, matching native legacy/current decoding. Checkpoint version 1 and storage envelope version 3 remain independent and unchanged. Future/string/null/fractional versions remain rejected, and native decoding/validation still occurs before replacing the project.

`app/scripts/check-project-recovery-e3.mjs` derives the fixture version from the native constant, exercises legacy/current round trips, rejects unsupported versions without deleting their stored bytes, and runs the existing authority/Recovery lifecycle sequence with the current native project version. The added check failed before the fix and passed afterwards. Actual preserved native checkpoint bytes were subsequently recovered; no replacement checkpoint was injected.

## Validation and artifact identity

All Windows Cargo/Tauri work followed `qa/WINDOWS_NATIVE_BUILD.md`: MSVC 14.44.35207, exact linker pinning and PATH-first checks, pinned FFmpeg 8.1.2 SDK. Native configurations that compiled in this campaign had zero Rust compiler warnings. Vite retained one preexisting >500 kB chunk warning: baseline 1, current 1, delta 0; current App chunk 530.33 kB. No warning threshold or blanket allow was added.

| Check | Observed result | Evidence |
| --- | --- | --- |
| Final VIDEO suite | 336 passed / 0 failed / 14 ignored; doctests 0; 33.64 s | `shutdown/video-final-chatgpt-01.log` |
| Native ownership/retirement/Stop regressions, NDI feature enabled | 69 passed / 0 failed / 0 ignored; 2219 filtered; 8.01 s | `N/stop-regressions-chatgpt-03.log` |
| Engine Follow ACK/retirement regressions | 21 passed / 0 failed / 0 ignored; 1186 filtered; 0.11 s | `N/engine-ack-stop-chatgpt-01.log` |
| Recovery E3 regression | Reproduced failure before fix; authority checks and production driver passed after fix | `N/recovery-schema-before-chatgpt-01.log`, `N/recovery-schema-after-chatgpt-01.log` |
| Output ownership/routing contracts | Passed | `N/ownership-contract-chatgpt-05.log`, `N/routing-contract-chatgpt-04.log` |
| Final isolated QA release build | TypeScript/Vite + native release succeeded; 33 source hashes stable | `N/qa-build-chatgpt-03.log`, before/after manifests |
| Final standard native gate | `pnpm --dir app tauri build --no-bundle` exit 0; 33 source hashes stable; exact executable launched, one responsive Syndocal main, maximized | `N/standard-build-chatgpt-01.log`, before/after manifests, `standard-01-launch.json`, `standard-01-ui-observation.json`, `standard-01-complete.json` |

The 14 ignored VIDEO tests were not represented as passing. The separate explicit timing experiment and Spout receiver opt-in test are reported below. The native regression filter is not the full native suite. Windows compilation of Syphon policy/fake tests is not macOS SDK validation. Enabling NDI for native tests did not exercise discovery or an NDI network sender.

Artifact SHA256 anchors:

| Artifact | SHA256 |
| --- | --- |
| Earlier performance QA binary, `O/bin/syndocal.exe` | `7edc1138c14b0db127e4a7056a1c8c9b868459dfa0cd717b8b73e7c7d7320916` |
| Final native authoring binary before Recovery frontend fix, `P/bin/syndocal.exe` | `3a70af6da21408ec873ea9f0cad3e40600645a825bc5f14edccae05cd8429c1d` |
| Final QA binary, `P/bin-recoveryfix/syndocal.exe` | `7bcef320e506f2b7f3cdfe375d89e6361e80d1cf0af7d6de00955f8adec62379` |
| Final standard executable, `target/release/syndocal.exe` | `7bd3f53977760fe7bae42bd46e569d6c7b169a17c1bbbbe8220a68f0ed339208` |

The final QA build manifest `P/build-recoveryfix.json` verifies the preserved parent binary, unchanged QA identifier/profile/configuration, all 30 Rust source identities unchanged from the authoring build, and seven pinned runtime DLLs. The Recovery fix changes the browser guard and checker only. Standard build manifests independently cover all 33 current source/checker files. Final standard launch PID 47112/HWND 27004158 was observed in Desktop frame-55; the native title bar exposed Restore, confirming its existing maximized state. Only that exact owned PID/path was then stopped.

## Native output authoring and actual Recovery

The isolated profile is `jp.seraf.ktn.syndocal.qa.output-authoring.ff4d7319-b2a5-4885-81a3-483822acd1c7`. The fixture contains a real empty composition, no media/audio/DMX/input routes. Only explicitly created local Video outputs were armed. Main window was maximized before functional actions; native Display was a separate small nonfullscreen window.

- `P/attempt-01`: actual create/configure/remove, exact replay, stale-fence and case-insensitive duplicate rejection, Video-only Arm, target Spout -> Display conversion, and App Undo -> Redo -> Undo passed. The unrelated sentinel Spout output continued with SDK counter progression 56 -> 4056 -> 4078 -> 4084 rather than restarting. Native Display was observed and border-dragged; its physical bounds were 1628 x 1153 on a 2560 x 1600 monitor, nonfullscreen.
- The first harness stopped before any SaveAs dispatch because it compared equivalent ordinary and extended Windows path spellings. The continuation normalized only those exact absolute path forms and resumed from the verified unchanged boundary. Independent review accepted it. Original failed evidence was preserved.
- `P/attempt-02`: canonical Japanese-path SaveAs, native CleanSave acknowledgement, exact saved bytes and native reopen passed. Saved `日本語-output-authoring.sdc` SHA256 is `5d336eb13a473589c505f11d07e09fdb3063fb7645197d22ec32faadddefa7ae`. Reopen advanced project epoch 1 -> 2 without rearming. The later dirty authoring step failed on the old-project lease defect below. The whole attempt is not relabeled passed.
- Later continuation checks preserved their failures without bypassing authority. `attempt-05` used a fresh owned process, canonical Open of the verified exact saved bytes, a newly acquired Video lease, and the actual App autosave to create a dirty Recovery checkpoint. The old binary then rejected that checkpoint because of the project-version mismatch.
- `P/attempt-06`, final QA binary: the original checkpoint/storage digest was verified unchanged before the actual App Recovery menu action. Native RecoveryAcknowledged progressed from source serial 7 through publication 8 to acknowledgement 9. Outputs stayed unarmed. Real SaveAs of `復元確認-output-authoring.sdc` produced SHA256 `5d18c4c5020e2ccf4752dcb298aa505bb9148c378ff3695f86d6d5dfdfa5b2da`, then CleanSave serial 10 and removal of the browser checkpoint key.
- A second owned restart (PID 40360 -> 44124, distinct native instances) retained the durable CleanSave serial/journal digest, showed no Recovery offer, kept both saved-file hashes exact, and did not rearm outputs. Both `attempt-06/recovery-evidence.json` and `recovery-verify-evidence.json` passed.

The fixed-build harness verifies build migration and prior evidence identity; it does not overwrite the failed checkpoint, fabricate a localStorage payload or claim that the original multi-step attempt passed. Detailed evidence/harnesses and failed attempts remain preserved under N/P.

## Performance and local Spout receiving

The [2026-10-22 matched Display measurements](FX_DISPLAY_PERFORMANCE_2026-10-22.md)
subsequently reproduced 59.99 fps without FX and 12.27 fps with three FX per layer
on unchanged current source, including 600-frame native pass/fail evidence and
matched ISF/CPU component timings. This diagnoses the remaining FX limit; it is
not a performance fix. The historical results below remain unchanged.

Final-source QA results use executable `7bcef320…`; no compiler ran during measurement. Main viewport was 2560 x 1504 at DPR 2. Native Display used 2560 x 1600 nonfullscreen output. Times below describe the sampled boundary, not a hard real-time guarantee.

| Workload / binary | Native observed submission rate | Dropped | Stop command wall time | Result/boundary |
| --- | ---: | ---: | ---: | --- |
| H264 4K30, 16 layers, no FX / final QA | 0.413445 fps (8 / 19.349607 s) | 1288 | 988.515 ms | Overloaded; 600-sample gate pending, not a 60 fps pass. Clean Ready Standby, no remaining output windows/routes/health errors. |
| Local Spout H264 1080p30, 1 layer, no FX / final QA | 59.993213 fps (3558 / 59.306709 s) | 0 | 378.6675 ms | 600-sample render p50 3.384 ms / p95 3.9893 ms; clean Ready Standby. |
| H264 1080p30, 3 layers, no FX / earlier QA | 59.9770 fps | 0 | 399.1269 ms | 600-sample native validation passed on earlier binary. VIDEO/ISF sources match final; later native lifecycle/ACK changes were not in that binary. |
| H264 1080p30, 3 layers, 3 FX / earlier QA | 12.6268 fps | 896 | 434.9251 ms | Only 263 validation samples; not a 60 fps or completed 600-sample pass. Same older-binary qualification applies. |

Evidence: `P/perf-h264-4k-16l-final-01`, `P/perf-local-spout-final-01`, `O/perf-h264-1080-3l-02`, `O/perf-h264-1080-3l-3fx-01`. Stop command wall time includes coordination, retirement and native acknowledgement; the single observed sub-second 4K result is not an isolated worker-join bound or a guaranteed latency ceiling.

The final source DEBUG ISF component experiment explicitly ran its ignored test: one passed. At 1080p, 24 alternating samples after four warmups with pixel parity each time:

- One stage: p50 8.213 -> 6.758 ms; p95 9.657 -> 8.858 ms.
- Three stages: p50 21.582 -> 7.065 ms; p95 22.484 -> 7.647 ms.

Evidence: `fx/timing-final-chatgpt-01.log`. These are isolated component timings, not native FPS. They do not erase the measured multi-stage FX end-to-end shortfall.

The independent local Spout receiver opt-in test passed: one passed, zero failed/ignored, 25.0018128 seconds at 1920 x 1080. It observed 753 content changes (30.117763/s, p95 interval 51.0074 ms), consistent with the 30 fps source. SDK frame counting was disabled: sender cadence unavailable and deliveredFps null. The 10,956 receivedFrames count is polling/readback count, not received FPS.

Evidence: `N/spout-receiver-final-chatgpt-01.log`. Sender was exactly `QA Task17 Runtime ff4d7319-b2a5-4885-81a3-483822acd1c7`; the precompiled test executable SHA256 was `78c08a026cf032c91240d52165d2fd0a9f28cec28cf8a096c71d437122cafb74`. No Cargo compilation overlapped the run. This establishes local software send/receive and changing pixels, not 60 fps receiving or physical display output.

## Independent reviews

Read-only Web ChatGPT reviews inspected stable source identities and their relevant evidence:

- `N/video-final-independent-review-current.md`: VIDEO cancellation, libav RAII, batching/history/budget; no concrete blockers.
- `N/native-stop-final-review-current.md`: native Stop/poll correctness reviewed; its redundant Follow-copy P2 was subsequently corrected and closed in the Engine follow-up below. The original finding remains preserved.
- `N/show-stop-independent-review-current.md`: active/failed Show pair ownership and bounded retirement scope accepted.
- `N/engine-ack-independent-review-current.md`: initial allocator-lock and extra Follow-copy findings were corrected; final stable diff accepted.
- `N/authoring-harness-continuation-review.md`, `N/authoring-recovery-fresh-review.md`: exact path normalization/resume and fresh-process Recovery preparation accepted without relabeling prior failures.
- `N/recovery-schema-fix-review-reconnected.md`: new review after a disconnected reviewer family; current two-file schema diff, before-fail/after-pass, actual preserved Recovery, durable restart and all 33 source hashes checked; no concrete blockers. Report SHA256 `3805c37a05b57b3a6c8ad37c43720f28ed161662c714b769d5b89fcea523a5f5`.
- `N/lease-project-binding-review.md`: independently confirmed the remaining P2 below; it is not an approval of that product path.

Reviewer connectivity briefly failed. A fresh Core/Web reviewer completed the remaining review; this is not a remaining tool blocker. No Codex agent or API substitute was used.

## Remaining confirmed defect and unverified boundaries

**P2 — same-process authoring after project Open can keep a previous-project Video lease.**

Follow-up: the unmanaged previous-project Video/Both path is now fixed and
validated in [Video authoring authority after same-process Open](VIDEO_AUTHORING_PROJECT_LEASE_2026-10-21.md).
That note records the dedicated exact-scope authority exchange, regressions,
independent review and same-process native Open/edit proof. The description below
is retained as evidence of the original checkpoint defect. Managed turnover and
current-project expiry retain their explicit lifecycle boundaries; the other
physical/performance limits below remain unchanged.

- Trigger: save/reopen to a new project epoch, then Add/Configure/Remove through the production App controller. The old owner-visible lease may be recovered or reused.
- Source: `app/src/App.tsx:23582-23589`; `app/src/outputControlController.ts:1716-1738`; Recover preserves project binding in `app/src-tauri/src/output_lease.rs:339-356,2378-2395` and `control_plane_runtime.rs:2221-2226`.
- Effect: native `main.rs:58974-58988` correctly rejects with “Video output authoring requires a Video-capable lease for this project.” The edit is not applied and outputs are not rearmed. After TTL expiry, the public query can omit the still-indexed claim (`output_lease.rs:1370-1398`), so a fresh Acquire can also conflict.
- Minimal safe direction: retain native project binding; provide a project-qualified authority observation and an explicit exact-owner/generation Relinquish -> new Acquire flow, with verified physical retirement and project fence preserved at every step. Generic Recover must not silently retarget a lease. Automatic seamless turnover needs its own native atomic receipt/admission contract and managed-lease validation; it is not supplied by this checkpoint.
- Bounded QA remedy used: close the owned stopped process, restart fresh, canonically Open the exact saved file, acquire fresh Video authority without Arm. This validated Recovery separately; it does not fix or pass same-process post-Open authoring.

Other boundaries:

- Stop is cooperative. A GPU/SDK/driver call already entered cannot be forcibly interrupted by these checks. A Stop load and present are not atomic; an already in-flight present cannot be excluded. Timeout retains owners/leases and fails closed through pending/quarantine.
- A stopped Follow waiter does not roll back an already applied Engine acknowledgement. Real prior failures and pending acknowledgement ownership remain significant.
- The existing Show startup rollback/PendingDrop synchronous joins were not converted to the bounded active/failed-output retirement protocol.
- Hardware DMX/audio/physical scanout, actual macOS/Syphon SDK, and NDI runtime/network output were not exercised. Unknown devices were not opened and the user was not asked again for names/connections.
- The recorded FX/4K envelope and older-binary qualifications above are acceptance limits. Arbitrary workloads are not claimed to meet 60 fps.
- The final standard native build/startup gate is complete; full product/release acceptance remains incomplete for the stated P2 and unavailable/unmet gates.

## Safe next action and checkpoint boundary

Inspect the staged 34 owned files and `git diff --cached --check`, commit/push this validated scoped checkpoint on main, and verify local HEAD / origin/main / remote main equality. The task's QA and standard app PIDs have been stopped after their recorded checks. Preserve every failed attempt, original binary, profile plan, source manifest, raw result and independent report.

For further authoring work, address the P2 through the explicit project-qualified authority contract and dedicated owner/generation/project/physical-retirement/replay regressions before claiming same-process reopen acceptance. Do not remove the native rejection or use generic invalid_request as permission to recycle a lease.
