# Luna resume handoff — recording stdin cancellation

This is the authoritative pause/resume note created for the user's 2026-09-07
request to continue this work with Luna rather than Astra. The initial pause
boundary and resume plan below have been superseded by the continuation result
recorded later in this document. At handoff creation no new task was created;
the tree was local and uncommitted.

## User intent and authorization

Continue the bounded Windows recording stdin cancellation tranche, preserve
existing recordings and retained worker ownership, resolve applicable failures,
obtain independent review and Windows native evidence, then commit/push owned
files under AGENTS.md. Do not resume the entire product roadmap.
The user explicitly approved adding this PC's exact Visual Studio Build Tools
2022/MSVC 14.44.35207 environment to the supported build rules. This approval
does not need to be requested again. Do not use gpt-5.6-terra for delegation.
The user requests Luna for continuation; do not silently resume on Astra.

## Git and ownership at pause

Checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`.
Branch: `codex/syndocal-v1.2`.
Implementation/test base: `6a65ee2b98906c3b939ec1a7fa2c73fd39b12f2b`.
Current HEAD and verified origin tracking ref after the user-requested pull:
`da446afa9adb86b43b68eabf20f2503acfe6ca54`.
`git fetch origin` followed by `git pull --ff-only origin codex/syndocal-v1.2`
fast-forwarded these two commits without conflicts:
`2ee235d fix: simplify DMX setup and publish installer`, then
`da446af test: include overlap viewport contract`.
All 10 owned dirty files retained identical SHA256 across the pull. No stash,
stage, commit or push was performed by this tranche. The handoff was subsequently
updated to record the pull. The pause-time tests below are superseded by the
continuation evidence recorded after the pull.
One worktree (this checkout), no stashes. Recheck before editing.

Owned dirty files (preserve all):

- `app/scripts/run-tauri.mjs`
- `app/scripts/check-tauri-build-wrapper.mjs`
- `qa/WINDOWS_NATIVE_BUILD.md`
- `app/src-tauri/src/video_recording_encoder.rs`
- `app/src-tauri/src/video_recording_encoder_tests.rs`
- `app/src-tauri/src/video_recording_encoder_stdin.rs` (new)
- `app/src-tauri/src/video_recording_encoder_inherited_stdin_tests.rs` (new)
- `app/src-tauri/src/recording_publication_tests.rs` (diagnostic assertion only)
- `qa/RECORDING_STDIN_CANCEL_2026-09-06.md` (new, candidate explanation)
- this handoff (new).

The pulled changes touch main.rs, App.tsx, DMX UI/controllers, styles and viewport
checks, not the currently owned file paths. Conflict-free pull and hash preservation
do not establish runtime integration acceptance. Revalidate the final source tree.
No subordinate agent remains active; all three lanes ended. Native test/build
processes were absent at pause.

## Implementation and review

Read `qa/RECORDING_STDIN_CANCEL_2026-09-06.md` for the bounded design.
RecordingStdin owns the original ChildStdin, uses no additional frame buffer or
thread, and coordinates handle destruction with CancelIoEx. Existing supervisor
continues after direct-child reap. Explicit Stop gets 250ms post-reap grace;
abort or process failure cancels immediately. Cancellation fails both write and
encoder result, preserving the existing publication success requirement.
OS cancellation/termination failure still retains ownership; active rendering
and total shutdown deadlines are not solved.

Independent reviewer found no blocking issue in the stable recording and
Build Tools changes before tests. Wrapper validation passed 243 assertions /
27 hostile mutation fixtures; actual vcvars capture verified exact linker pin
and PATH-first ordering. CRLF fixture normalization was necessary for the
existing checker anchors on this Windows checkout. Community support and
restricted hosted Enterprise support remain. ASIO/soak harnesses still have
their narrower Community-only preflight.

## Execution evidence and failures

Keep the ignored directory `target/qa/recording-stdin-cancel-20260906/` intact:

- `run-cargo.mjs`: wrapper-verified Cargo runner, sets explicit FFmpeg/FFprobe
  paths and DLL search directory. Read it before reuse; its selected LGPL
  executable lacks libx264 and needs a suitable external encoder selection.
- `recording-tests.log`: compiler completed in 3m19s; the pause-time result was
  53 passed, 1 failed, 6 ignored, 1727 filtered. Both new inherited-stdin
  Stop/Drop tests and complete-frame graceful EOF test passed. Inherited-stderr
  regression passed. The failure was resolved in the continuation result below.
  Ignored subprocess helper tests are invoked by their owning tests, not
  standalone acceptance. First-party compiler warnings observed: 0.
- Failure at pause: `recording_artifact::publication::tests::recording_publication_recovers_after_process_exit_at_each_boundary`
  at `recording_publication_tests.rs:308`, expected error to contain
  `restored or preserved`. The test-only diagnostic edit now includes
  `{point:?}: {error}` without weakening the assertion; the targeted rerun
  passed, as recorded below.
- `ffmpeg-smoke.log`: 1 selected test, FAILED (0 passed, 1786 filtered).
  `recording_command_writes_a_real_video_and_audio_mp4` failed because the
  selected LGPL FFmpeg returned `Unknown encoder 'libx264'`. This does not prove
  successful graceful MP4 finalization. Select an existing suitable executable
  or provision an appropriate external test encoder; do not replace the app's
  LGPL libav libraries or weaken the codec/decoding test to manufacture a pass.
- `native-launch.ps1`: prepared exact-checkout launch/maximize/responsiveness
  verification helper. It was executed successfully in the continuation and
  produced `native-launch.json`.

The historical accepted compiler-warning baseline was 0 on Community; this
PC's changed Build Tools test configuration observed 0. A pre-change build on
this configuration was not measured, so do not invent its baseline/delta.
No current native release build, launch acceptance or hardware output occurred
at the pause boundary; the continuation result below updates this state.

## 2026-09-07 continuation result

The safe resume actions were executed without changing the user's installed
Syndocal process or activating physical output:

- The publication boundary test passed `1/1` after the diagnostic assertion was
  added. The earlier publication failure was not reproduced.
- An exact Gyan FFmpeg 8.1.2 full shared pair was downloaded into the ignored QA
  evidence area, copied to ASCII staging at `C:\SyndocalQA`, and verified
  against all seven pinned runtime DLL hashes. The ignored real MP4 test passed
  with `libx264` and AAC. The original LGPL `Unknown encoder 'libx264'` log is
  retained and was not overwritten.
- The recording filter passed `54 passed / 6 ignored / 0 failed`, including both
  inherited-stdin Stop/Drop tests, complete-frame graceful EOF, publication
  boundaries, and inherited-stderr cancellation. Subprocess helpers remain
  ignored and are invoked only by their owning tests.
- `pnpm.cmd --dir app run check:tauri-build-wrapper` passed `243 assertions /
  27 hostile mutation fixtures`. The native no-bundle build passed in 3m32s
  under Build Tools MSVC `14.44.35207`; the exact artifact is
  `target/release/syndocal.exe`, version `1.2.0-alpha.69`, SHA-256
  `7B99B6B7B518DD6D18EDC6C3FC6270FA63696B4BB3D5BD30E4536B5488A3BF28`.
- `native-launch.json` verified exactly one responsive, maximized `Syndocal`
  window for that artifact and recorded `physicalOutputOperations=0`. Only the
  exact checkout process was stopped after verification; the installed path was
  not terminated.
- Current author/publisher metadata is now `Seraf()` and `Seraf() / KTN`.
  This does not claim Authenticode signing; the alpha artifact remains
  unsigned without a real certificate and signing key.

The full `check:release` reached `release metadata ok: Syndocal 1.2.0-alpha.69 /
.sdc / signed updater overlay / Seraf() / KTN`, then stopped at the existing
ASIO v3 source-anchor assertion `loader mutation source missing: CloseFn`.
That unrelated gate remains open and is not a failure of the recording tranche.
The broader recording boundaries remain open: active renderer cancellation,
total shutdown deadlines, OS cancellation/termination failure, and broader
recording acceptance are not claimed complete.

## Artifact and process protection

At the pause boundary, `target/release/syndocal.exe` was an OLD artifact
(2026-08-19 23:46:19 local),
50,990,080 bytes, SHA256
`08B225536509DF6B1F20A2778CA332B619CB5A18912072B4E04E1A9DE38BFB8A`.
Never label that pause-time artifact as the current candidate. The fresh
continuation artifact and its SHA-256 are recorded in the continuation result
above; recheck the exact path before using it in a later run.

At pause PID 44016 was running
`C:\Users\janua\AppData\Local\Syndocal\syndocal.exe`.
It is an installed application, not this checkout artifact: do not terminate it
as part of the checkout build gate. Recheck identity; PID alone is not authority.
No physical device/output operation is authorized by this handoff. Preserve
other applications, shows, credentials, QA evidence and upstream-owned work.

## Pre-continuation safe resume actions (completed)

These were the safe actions listed when this handoff was created. They were
executed in the continuation above; the list is retained as an audit trail.

1. Read AGENTS.md, this handoff, candidate note and qa/WINDOWS_NATIVE_BUILD.md;
   inspect Git/worktree/stash/process state. The two upstream commits are already
   pulled; do not redo that integration. Preserve owned dirty work and recheck
   for any further upstream changes; do not reset/stash another owner's changes.
2. Run the publication test with its improved diagnostic (from checkout root):
   `node target/qa/recording-stdin-cancel-20260906/run-cargo.mjs test -p syndocal --locked recording_publication_recovers_after_process_exit_at_each_boundary -- --nocapture`.
   Save new output separately; diagnose the actual phase/error before deciding
   whether a bounded test or runtime fix is appropriate.
3. Verify an external FFmpeg/FFprobe pair with libx264 and AAC, update only the
   ignored runner's explicit executable selection as needed, then rerun
   `recording_command_writes_a_real_video_and_audio_mp4 -- --ignored --nocapture`
   through the same verified Cargo environment. Retain the first failure log.
4. After fixes, rerun affected recording regressions, obtain independent review
   of any new substantive change, and update the candidate evidence. Do not
   count ignored tests as run or mask the publication failure.
5. Run `pnpm.cmd --dir app tauri build --no-bundle` under the maintained wrapper.
   It must verify the exact checkout executable before stopping only that path.
   Verify required runtime DLL paths, launch the freshly built exact artifact,
   and confirm exactly one responsive maximized Syndocal window. Review the
   ignored launch helper before use. Avoid physical output activation.
6. Once applicable gates pass, update one final checkpoint plus the priority row
   in `qa/REMAINING_WORK_2026-09-05.md`; inspect staged owned diff, commit/push
   and verify upstream equality. Do not claim all Recording acceptance complete.

Suggested continuation prompt:

> qa/LUNA_RESUME_HANDOFF_2026-09-07.mdを読んで、Lunaで録画stdin停止対応を
> 引き継いでください。未コミット差分と証拠を保全し、保存復旧テストの原因調査、
> libx264対応FFmpegでの実保存検証、native検証、独立レビュー、commit/pushまで
> 進めてください。Build Tools正式対応は承認済み。実機出力は操作しないでください。
