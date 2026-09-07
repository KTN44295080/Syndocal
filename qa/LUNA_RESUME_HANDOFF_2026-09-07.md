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
- The synthetic 30-minute FFmpeg A/V test passed `1/1` under the exact pinned
  Build Tools linker. Its report records `1800s`, `54000` frames, `0.0ms`
  start/end drift, video luma `255/0/255`, and audio peaks `4276/0/4123` for
  first/middle/last samples. The MP4 is retained at
  `target/qa/recording-stdin-cancel-20260906/long-av-sync-20260907/` with
  SHA-256 `24B34D009F80C6468FDCBED0927E161DB8D82305E032124527B5F7AC3A89F286`.
  This is synthetic command/codec evidence, not full in-app or hardware
  recording acceptance.
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

The full `check:release` now passes, including release metadata, output-control,
safety-blackout, ASIO packaging/v3, timeline audio/loop, snapshot watermark,
project bootstrap, video output/window observation and camera-input checks.
The previous `loader mutation source missing: CloseFn` failure was in the
checker self-test: its normalized mutation source was compared with the raw
multi-line fixture. The checker now normalizes both mutation sides and the
22-assertion ASIO v3 self-test passes.
The broader recording boundaries remain open: active renderer cancellation,
total shutdown deadlines, OS cancellation/termination failure, and broader
recording acceptance are not claimed complete.

## 2026-09-07 renderer-cancellation continuation

The recording renderer boundary was continued without touching the installed
Syndocal process or activating physical output. `VideoRenderCancellation` is a
small public contract in its own video module, and cancellable FFmpeg process
polling/termination/reader joining is in its own process module. The existing
renderer and decoder APIs remain thin compatibility seams into one canonical
implementation; ordinary rendering retains the synchronous decode path and
does not spawn cancellation readers. The recording loop is the only current
caller of the cancellable output-preview seam.

The callback is observed before and between decoder/input, layer, composition,
transition and effect stages. A cancelled direct FFmpeg child is killed,
waited, and its readers are joined before returning. In-process HAP/libav or a
single CPU/GPU/effect operation remains synchronous and is checked only at its
boundaries. Descendant process-tree cleanup and an OS-level hard total-stop
deadline remain explicitly unresolved.

Current verification:

- New video cancellation tests: `2/2` passed.
- Cancellable process termination/reap test: `1/1` passed.
- Recording filter: `54 passed / 6 ignored / 0 failed`.
- `cargo check -p syndocal --locked`: passed with no first-party warnings.
- `pnpm.cmd --dir app run check:release`: passed.
- `pnpm.cmd --dir app tauri build --no-bundle`: passed with Build Tools MSVC
  `14.44.35207`. Fresh `target/release/syndocal.exe` is version
  `1.2.0-alpha.69`, SHA-256
  `C753D6F8CE250D645D5176904F1FA139146B0FC7D680417F7087D0888F1B3886`.
  Exact-checkout launch observed one responsive, maximized window and
  `physicalOutputOperations=0`; only that exact checkout process was stopped.
- Full video run: `171 passed / 3 failed / 3 ignored`; the three failures are
  existing output-routing/GPU comparison failures outside this diff and are
  not counted as cancellation acceptance.

The applicable detailed record is
[RECORDING_RENDERER_WAIT_2026-09-06.md](RECORDING_RENDERER_WAIT_2026-09-06.md).
The remaining-work priority row must retain the distinction between this
cooperative boundary and the unresolved hard shutdown deadline.

## 2026-09-07 Setup I/O presentation cleanup

The fixed three-screen layout was retained. The Setup I/O connection picker
now keeps all six connections on one desktop rail, shows only the connection
name plus live state/action in each card, and keeps the descriptive summary in
the accessible label/tooltip. The selected workbench no longer renders a
second generic active-connection header. Web Remote is direct when selected;
its related safety, endpoint, standby, and DJ Link sections remain grouped as
disclosures in the same workbench. Runtime and IPC ownership remain in
`App.tsx` and the existing panels; this is a presentation-only consolidation.

Focused evidence: app build passed; Setup I/O viewport contract passed at all
five required sizes; output-control/audio-control checks passed; DJ Link
frontend checks passed; browser fixture inspection at 1920x1032 and 1280x720
reported no page errors and confirmed six cards in one row. The browser check
used the bundled Playwright runtime with installed Chrome because the Browser
plugin was unavailable. Native, hardware, and physical-output acceptance were
not rerun for this CSS/presentation-only change.
The broader localization check remains red at its existing 3657/3692 static
coverage gate; the new compact card labels are registered in the Japanese
dictionary, and the reported untranslated entries are outside this picker.

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

## 2026-09-07 pre-final checkpoint (superseded)

この節は最終補正前の履歴であり、現在の判定は下の `final continuation` と
`final verification correction` を正とする。

上記の旧記録に残る「process tree / total deadline / localization未完了」は、
現行ソースでは次の実装・検証で更新されている。履歴としての旧記述は削除せず、
この節を現在の引き継ぎ状態とする。

- 当時の録画フィルタは正確なBuild Tools MSVC 14.44.35207 linkerで
  `56 passed / 6 ignored / 0 failed`。停止中rendererの協調キャンセル、stdin/
  stderrのWindows I/Oキャンセル、FFmpeg直接子の終了確認、Job Objectによる
  子孫回収、10秒のworker total-stop deadlineとnamed reaper移管を含む。
- 新設の `recording_encoder_failure_reaps_the_descendant_process_tree` は
  親終了後もpipeを保持する子孫を生成し、failure後に子孫が回収されることを
  実プロセスで確認した。任意の同期HAP/libav/GPU/effect命令を安全に強制中断
  することはできないため、deadlineは呼び出し元の復帰・所有権移管を保証し、
  in-process instructionの物理hard-killを主張しない。
- H.264/AAC実MP4は `1/1`、合成30分A/Vは `1/1`。後者は54,000 frames、
  start/end drift `0.0ms`、first/middle/last luma `255/0/255`、audio peak
  `4276/0/4123`。実show・カメラ・物理デバイスの受入証明とは区別する。
- この時点の `cargo test -p video --locked` は `171 passed / 3 failed / 3 ignored`。
  失敗は `output_preview_renderer_requests_only_routed_layers_and_skips_blackout_decode`,
  `gpu_compositor::tests::gpu_compositor_matches_cpu_transform_crop_and_source_size`,
  `gpu_compositor::tests::gpu_output_mapping_matches_cpu_aspect_modes` の既存
  output-routing/GPU比較経路で、今回のUI/MCP/録画差分外として残す。
- localizationは `3693/3693 (100.0%)`、unprotected bare user-data labels `0`。
  Setup I/Oは固定三画面レイアウトのまま、接続カードを一列の要約表示へ整理した。
- backend/MCPは専用モジュールへ分離し、canonical registryのbounded capability
  discoveryとread-only recording statusを追加した時点で、MCPは8 typed tools、実EXEで
  initialize/tools/list、capability/statusを `pending→completed` として確認した。
  `47 canonical operations / 1606 source inventory` を返し、FailClosedは
  discovery-only。動的Tauri invoke、script、DOM操作、独自retryは追加していない。
- native no-bundle buildは成功。`target/release/syndocal.exe`、version
  当時の `1.2.0-alpha.69`、SHA-256 `AC6C2F786E5C0FFACCD379392C3BE850F11A6977711F67564EB28812A4179CE7`。
  exact checkout版で一つのresponsive/maximized `Syndocal` window、
  `physicalOutputOperations=0`を確認し、そのPIDだけ停止した。

残る非主張は、既存のvideo 3比較失敗、実show/物理デバイス受入、任意の同期
in-process命令の強制中断、およびcanonical registryのFailClosed領域を含む
製品全体AI parityである。これは安全境界であり、今回のbounded implementation
を「全製品操作がMCPで完了」と誤表示しない。

## 2026-09-07 final continuation — current source and acceptance

上記の pre-final checkpoint を、今回の最終検証結果で更新する。旧記録の
数値は履歴として保持し、この節を現在の候補に対する判定とする。

- Full video は `174 passed / 0 failed / 3 ignored`。旧来の3失敗
  (`output_preview_renderer_requests_only_routed_layers_and_skips_blackout_decode`,
  `gpu_compositor_matches_cpu_transform_crop_and_source_size`,
  `gpu_output_mapping_matches_cpu_aspect_modes`) は、GPU source sampling、
  output mapping、preview retentionの境界を修正して全て通過した。残る3 ignored
  は既存のGPU・出力経路比較のignored条件で、今回の変更による失敗ではない。
- 録画はrendererを `--syndocal-recording-renderer-worker` の長寿命子プロセスへ
  分離した。親はencoderと最新の外部入力、子は同期decoder/HAP/libav/GPU/effect
  renderを所有する。Stop/deadline watchdog、Job Objectによる子孫回収、10秒の
  total-stop deadline、250msのreap確認を分離責務として実装した。録画フィルタは
  直列実行で `57 passed / 0 failed / 6 ignored`。renderer worker回帰2/2、
  descendant process-tree回帰1/1、実H.264/AAC MP4 1/1、合成30分A/V 1/1
  (54,000 frames、start/end drift 0.0ms)も通過した。
- MCPは9 typed tools、47 reviewed canonical operationsの静的typed adapter、
  bounded capability discovery、read-only recording statusを提供する。実EXEで
  descriptor/PID/executable identity、MCP initialize、tools/list、capabilityと
  recording statusの `pending -> completed` を確認した。未レビュー・FailClosed
  操作、動的Tauri invoke、script/DOM操作、独自retryは実行できない。
- 現行release artifactは `target/release/syndocal.exe`、version
  `1.2.0-alpha.69`、SHA-256
  `9001B78E509B72927345BE00E6F91B292AF59CA6B9C1289CFC4DAB5A645AA4D8`。
  exact checkout版を起動し、PID 7124、title `Syndocal`、responsive/maximized、
  responsive/maximizedを確認後、そのPIDだけ停止した。physical outputは起動
  受入中に有効化していない。publisher/companyは `Seraf() / KTN`。
- `cargo test -p io --locked -- --test-threads=1` は `180 passed / 0 failed /
  2 ignored`。現ホストの安全な列挙では ASUSカメラ3台、virtual MIDIのみを検出し、
  serial DMX/Enttec/COMポートと物理MIDIは検出できなかった。ASUS cameraの
  DirectShow 1280x720 NV12/30fpsは、production workerのstart/stop 2回を1/1で
  確認した。実DMX、物理MIDI操作、venue/show総合受入は機材・会場がないため未実施。
- Authenticodeは `app/scripts/sign-windows-artifact.mjs` と `sign:windows` を追加し、
  SDK signtool、SHA-256、timestamp、`/verify /pa /all /tw` をfail-closedで実行する。
  現ホストのCurrentUser証明書ストアに証明書・秘密鍵がなく、artifactの状態は
  `NotSigned`。鍵を捏造せず、信頼済み署名完了とは主張しない。
- localizationは `3693/3693 (100.0%)`、unprotected bare user-data labels `0`。
  fixed three-screen layoutとSetup I/Oの要約表示は維持されている。

## 2026-09-07 final verification correction

最終continuation後の直列再実行で、録画関連は `cargo test -p syndocal
--locked recording -- --test-threads=1` が `57 passed / 0 failed / 6 ignored`、
`video_recording` filter単体も `31 passed / 0 failed / 5 ignored` になった。
継承stdinを保持した子孫を含む停止テストは10回連続で通過し、先行実行の一時的な
1件失敗は再現しなかった。`cargo test -p video --locked` は
`174 passed / 0 failed / 3 ignored`、`cargo test -p io --locked --
--test-threads=1` は `180 passed / 0 failed / 2 ignored` である。

現行UIのfocused受入はSetup Video 5サイズ、VJ Clip Bank 5サイズ、Live Audio
英日10ケース、復元/FailClosed、MCP 15群、frontend invoke 456件、camera UI、
`pnpm run check:release` が全て通過した。ASUS 5M webcamはcanonical
DirectShow profile（NV12 1280x720/30fps）でproduction workerの2回start/stopを
`1/1`で確認した。

ネイティブ候補の現ハッシュは上記 `9001B78E...` であり、会社名は
`Seraf() / KTN`。SDK `signtool.exe` の `/verify /pa /all /tw` 経路は存在するが、
このホストに証明書/秘密鍵がないため状態は `NotSigned`。現ホストにserial
DMX/Enttec/COMまたは物理MIDIはなく、実DMX、物理MIDI操作、会場での総合show受入は
未実施である。loopbackや自己署名でこれらを完了扱いにしない。

この候補でソース実装・ソフトウェア/native検証は完了した。外部設備が必要な
実機/実show受入と、外部秘密情報が必要なAuthenticode署名だけは、現ホストで
実行不能な境界として残る。
