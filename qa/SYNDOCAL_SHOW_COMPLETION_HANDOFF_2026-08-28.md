# Syndocal 2026-08-30 show completion handoff

Status date: 2026-08-30 JST

This is the concise authoritative resume note for the final show-critical tranche. It supersedes chat-only status, but it does not supersede the detailed acceptance documents named below.

## Source authority

- Current alpha.39 native checkpoint is on branch `codex/syndocal-v1.2`; the
  build source/docs `HEAD` and upstream were both
  `ec93e9160da853ad181de70aee4db7b4a75fafbb`. This tranche repairs the Timeline
  authoring-output selector's
  exact UI truth: exactly one selectable occurrence is admitted, while
  duplicate, missing, or ambiguous identities fail closed; status-only polling
  may reapply the exact desired option without list/configuration/routing
  mutation. Browser Phase A/B passed after the strict external-video
  status-poll fixture was added; TypeScript, runtime, video-poll,
  `git diff --check`, and `pnpm --dir app run check:release` pass. Independent
  Terra select review is GO with P0/P1 `0`, and the checker review is GO with
  P0/P1/P2 `0`. First-party warnings are `0` only for the evidenced non-native
  checks. The exact MSVC `14.44.35207` Community linker was pinned and first in
  `where.exe`; `pnpm --dir app tauri build --no-bundle` exited `0` in `2m57s`
  with first-party warnings `0`. The exact executable
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
  `61,108,736` bytes, reports Product/FileVersion `1.2.0-alpha.39`, and has
  SHA-256 `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
  Exactly one checkout-owned PID `87640` is responsive with maximized
  `Syndocal` window id `2033716740`; Daslight was preserved. The reviewed
  native-evidence record was committed and pushed at
  `94b362bd2d733e447feabf0a0a6158699da6a2bf`, and `HEAD`/upstream equality was
  verified immediately after that push.

  Native alpha9 UI reverification passed without clicking the output selector or
  Refresh. The exact sequence was Play -> Pause -> status-only wait -> Play ->
  Pause -> status-only wait; throughout it, explicit-device and resolved output
  stayed at `Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`, with
  advancing output frames and no visible Backend, Local IPC, or CUE fault. The project
  remained unsaved and final Timeline state was paused. Read-only settings were
  `route=explicit_device`, `device_name=Music (Elgato Virtual Audio)`,
  fingerprint `A2D9603C75ED1A6ECBC37F0FE851AAD56C632DFF31314544F2F606C220C9479C`,
  `click_gain=1`, and `guide_gain=0.85`. The opened alpha9 project remained
  unchanged with SHA-256 `93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094`
  and last write `2026-08-29T05:53:14.5317317Z`. The native build/window
  and selector/UI regression gates are complete only; audible/device selection,
  operator audible confirmation, dedicated Show-ASIO, Unity/GPU Art-Net/Spout,
  DJ, camera, and other hardware acceptance remain open. After the current
  native show-UI evidence below is committed and pushed and `HEAD`/upstream
  equality is reverified, the next safe action is operator audible
  confirmation.

  A subsequent bounded native show-UI pass used the same exact responsive,
  maximized alpha.39 PID `87640` / window id `2033716740` and left the opened
  project paused and unsaved. Right-clicking its Timeline Audio Clip opened the
  compact `操作` disclosure; expanding it exposed the four nested groups
  `選択`, `クリップボード`, `タイミング`, and `レーン`. An outside left click
  dismissed the menu both before and after hierarchy expansion. In
  Setup I/O, the DJ Link disclosure was scrolled to its lower content while the
  upper Web Remote card remained visible and directly selectable; switching to
  Web Remote succeeded without restoring the disclosure scroll position first.
  This is native visual evidence for context-menu dismissal/hierarchy and
  Web Remote/DJ Link reachability only, not DJ/network/remote-server acceptance.

- Historical alpha.38 native authority remains immutable. The exact MSVC
  `14.44` native gate ran `pnpm --dir app tauri build --no-bundle` in `3m12s`
  with first-party warnings `0`. The resulting exact executable
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` is
  `61,114,368` bytes, reports Product/FileVersion `1.2.0-alpha.38`, and has
  SHA-256 `9E5CA0DB826D9998F7CDC76214E5CDC17597A6D4FC1CC27FEB5EF72D6E28FA37`.
  Its pushed source/build HEAD was
  `e4ec22384675aace5ed3912ddffdcfecca190919`. Immediately before the build,
  exact-path verification stopped only old checkout-owned PID `50864`;
  Daslight PID `42752` was path-verified and preserved. Exact PID `55624` is
  responsive with one `Syndocal` window, and Computer Use plus Alt+Space
  verified that it is maximized (`Maximize` disabled, `Restore` enabled).
  This proves alpha.38 native build/window only; its binary must not be
  relabeled as alpha.39. The alpha.38 authoring run completed
  Click/Guide-enabled Play -> Pause -> Play -> Pause without a CUE fault, but
  after Pause the authoring-output `<select>` displayed `3 - PX160 WAV...`
  while settings JSON and resolved output remained `Music (Elgato Virtual
  Audio)`. Preserve that drift as historical fail-closed evidence; alpha.39
  native/UI reverification was pending at that checkpoint and is now complete
  as recorded above. Audible confirmation remains open.

- The first alpha.38 native resume action was exercised on PID `55624` with the
  saved reference
  `target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha9-reference-audio.sdc`
  opened from the recent-projects menu. Settings JSON remained
  `route=explicit_device` and `device_name=Music (Elgato Virtual Audio)`. With
  Click, one-measure mode, and Guide enabled, Play -> Pause -> Play -> Pause
  tracked the Timeline bottom playing/paused state; the authoring monitor stayed
  `実行中`, `rev1`, with frames `9452160` (before start) -> `180000` (re-Play)
  -> `157440`/`1645920` (after Pause while output-frame publication continued).
  Resolved output remained `制作モニター出力: Music (Elgato Virtual Audio)` and
  no Backend fault, Local IPC error, or Cue fault appeared. This is native
  UI/runtime regression evidence only; audible confirmation is still open.

- Historical alpha.37 source train is `1.2.0-alpha.37` on branch
  `codex/syndocal-v1.2`. Checkpoint
  `1636aeb440c718c628a953b58e4f0c59d4874e35` and native-build source HEAD
  `5626a9636003462a23daf0f3de67af3cc5060e29` are pushed; its pre-tranche base was upstream-equal HEAD
  `901e4e09f829705ff488511c53e50f51a511e088`. The checkpoint integrates
  Timeline CUE anchor linearization and the
  strict fixed-name Show Spout first-black/reaping/retirement-ACK state
  machines. Exact MSVC 14.44 combined focused gates pass Timeline CUE `49/49`,
  media-audio `46/46`, Show Spout `17/17`, production R4 interleaving `1/1`,
  and generic strict-name retry `2/2`, with first-party warnings `0`.
  Complete exact-linker integration passes `1211/0/7 ignored` without default
  features and `1276/0/12 ignored` with Spout; TypeScript and the Vite
  production build also pass, and exact ASIO Timeline CUE passes `59/59`.
  `check:release` passes after synchronizing product metadata to alpha.37.
  Independent Terra xHigh production review is GO for both diffs; audio is
  P0/P1/P2 `0`, while Spout retains two explicit P2 proof-only rows for direct
  pending-publish reservation and non-synthetic post-join cleanup error.
  Combined Terra xHigh review initially returned NO-GO for a release-only
  `dead_code` warning in the deterministic source-token field. That complete
  seam is now `cfg(test)` with no fake production read; release check is
  warning-free and final rereview is GO with P0/P1/P2 `0`. A first post-fix
  full Spout run saw one transient `Busy` in the unrelated
  production-remote-stop test; its exact filter then passed three consecutive
  runs and a fresh full Spout run passed `1276/0/12`.
  The alpha.37 native build/window gate is now complete as recorded below.
  Audible `Music (Elgato Virtual Audio)`, Unity/GPU, camera hardware,
  installer/updater, dedicated Show-ASIO, DJ hardware, and final
  show-completion acceptance remain open.

- From clean pushed HEAD `5626a9636003462a23daf0f3de67af3cc5060e29`,
  the exact MSVC `14.44.35207` Community linker was pinned and first in
  `where.exe`; Git's incompatible linker remained second. `pnpm --dir app
  tauri build --no-bundle` completed in `3m04s` with first-party warnings `0`.
  `target/release/syndocal.exe` is `61,116,416` bytes, reports
  Product/FileVersion `1.2.0-alpha.37`, and has SHA-256
  `7AC54394E41751126911E6DC338536B93E484A20934B4CA9A001EB8B9F3E209E`.
  The exact-path pre-build gate stopped only historical checkout-owned
  alpha.36 PID `109972`; Daslight PID `42752` was path-verified and preserved.
  An initial Computer Use accessibility-window launch attempt left no process;
  direct launch of the same exact executable then succeeded. Exactly one
  checkout-owned alpha.37 PID `50864` is responsive with title `Syndocal`.
  Computer Use verified Restore enabled and Maximize disabled in its system
  menu, proving the exact window is maximized. The visible runtime status
  `trust_network_absent` keeps DJ Link auto-start and DJ readiness explicitly
  open. This closes native build/window only, not physical acceptance.

- Alpha.36 source checkpoint `ffdb289da1a3980883807a83b8074f0247ab3ea9`
  (`Localize same-PC show controls for alpha.36`) is pushed to
  `origin/codex/syndocal-v1.2`; immediately after push, local `HEAD` and
  upstream were exactly equal and the only dirty path was this handoff update.
  Product metadata is synchronized at `1.2.0-alpha.36`. Focused localization
  passed `3632/3632` (`100.0%`) with unprotected user-data labels `0`; the
  exact `Syndocal Background` and `Syndocal Foreground` identifiers have direct
  `data-no-localize` source assertions. Release metadata, output-control R4,
  video routing R4, the exact 439-command frontend invoke inventory,
  TypeScript/Vite production build, Timeline context-menu, five I/O
  remote-scroll viewports, format, and diff gates passed. Independent Terra
  xHigh rereview is GO with P0/P1/P2 `0`.

- From clean pushed HEAD `81a7a9cccd93d40c3f40a697bc3aad6d3616aded`,
  the exact MSVC `14.44.35207` Community linker was pinned and first in
  `where.exe`; Git's incompatible linker remained second. `pnpm --dir app
  tauri build --no-bundle` completed in `3m30s` with first-party warnings `0`.
  `target/release/syndocal.exe` is `61,049,856` bytes, reports
  Product/FileVersion `1.2.0-alpha.36`, and has SHA-256
  `150C30FE218C6C32A50169EFE7638AD0C5D1CDD8DCFDF5BCFCCEB6434DD4E79E`.
  The path-verified historical alpha.35 PID `72448` alone was stopped before
  the build. At that historical checkpoint, exactly one checkout-owned
  alpha.36 PID `109972` was responsive with title `Syndocal`, and Win32
  `IsZoomed` verified its exact window was maximized. This proves the
  historical native/window gate only; no Unity/GPU, audible
  device, DJ, or camera hardware acceptance is inferred.

- Historical alpha.35 source checkpoint `c7218c112b296652fa77f88c67e861c18d6aea1b`
  (`Harden same-PC show output isolation`) is pushed to
  `origin/codex/syndocal-v1.2`; immediately after push, local `HEAD`, upstream,
  and that hash were equal and the worktree was clean. The exact MSVC 14.44
  alpha.35 native build then ran from clean pushed HEAD
  `31a577cfaad54c0ea981a0fc98a4254b0f5cb8db`, after path-verifying and stopping
  only the historical checkout-owned alpha.34 process PID `85492`. It completed
  in `3m36s`, first-party warnings `0`; `target/release/syndocal.exe` is
  `61,045,760` bytes, Product/FileVersion `1.2.0-alpha.35`, SHA-256
  `1144DBFA1AAD3E46E88F6F3A026218B8B7AB3C3E9A234270B75EDB41213A0E7A`.
  Exactly one checkout-owned PID `72448` is responsive. Computer Use did not
  expose the window as targetable and explicit-path recovery reported an
  unready accessibility window-opened handler; maximized visual proof remains
  open.

- Historical alpha.38 product metadata is `1.2.0-alpha.38`; source/build HEAD is
  `e4ec22384675aace5ed3912ddffdcfecca190919`. The normal native artifact and
  responsive/maximized window identity are recorded above. Alpha.37 and earlier
  processes are historical execution evidence; the exact alpha.38 PID `55624`
  is historical native authority only. The current authority is the alpha.39
  native checkpoint at build source/docs `HEAD` and upstream
  `ec93e9160da853ad181de70aee4db7b4a75fafbb`. The reviewed native-evidence
  record is pushed at `94b362bd2d733e447feabf0a0a6158699da6a2bf`; the exact
  alpha.39 artifact and responsive maximized window are recorded at the top of
  this handoff.
  The show boundary is same-PC
  only: exact Art-Net `127.0.0.1:6454` plus local Spout; remote Art-Net and NDI
  are outside this acceptance scope. Alpha.35 rejects pre-existing U0 input
  before strict activation, rejects active U0 HTP/LTP input without insertion,
  and skips stale U0 merge at the final render fence. U1 and ordinary
  non-strict merge remain supported. Exact MSVC 14.44 focused proof passed
  `12/12`; the full engine gate passed `924/0/2`; first-party warnings were `0`.
  Independent Terra xHigh rereview returned GO with P0/P1 `0`. Two additional
  public-ACK/non-strict-U0 tests remain P2 proof debt. The strict Spout pair now
  requires each worker's first physical frame to be cached 1920×1080 opaque
  black, exact post-send SDK names, both first-black ACKs, and a final R4
  authority revalidation before active/live handoff. Focused Spout passed
  `13/0/0`; full Syndocal passed no-default `1205/0/7` and default libav/Spout
  `1265/0/12`; TypeScript, production build, release, format, and diff gates
  passed with first-party warnings `0`. Independent Terra xHigh rereview is GO
  with P0/P1 `0`. The alpha.35 native build passed as recorded above; physical
  Unity observation remains pending.

- Alpha.38 post-build storage evidence records `110,353,719,296` free bytes on
  `C:`. A fresh read-only traversal reports `target` as `94,214,143,142`
  logical bytes across `86,760` files, `app/node_modules` as
  `545,338,492` bytes across `3,704` files, `tools/asio-bridge/target` as
  `1,774,985,879` bytes across `5,560` files, and `app/dist` as `5,203,962`
  bytes across `306` files. Git reports one `568 KiB` temporary-object garbage
  item. No deletion target is currently accepted for this checkpoint
  (`Candidates=[]`); no deletion ran and reclaimed logical bytes remain `0`.

- The complete same-PC Art-Net/camera/fixed-Spout source checkpoint is clean and
  pushed at `138f6c3e7bd536c10a589bc644bb9bc6df269f7a` on
  `codex/syndocal-v1.2`. The same-PC show-output clean break replaces the
  retired show-only USB/Enttec activation with a payloadless, locally
  confirmed Art-Net route fixed at `127.0.0.1:6454`, wire U0, 512 bytes, and
  no serial field. Generic serial DMX remains available outside this exact
  show path. The strict sender masks DMX channel 500 to zero immediately
  before packet encoding.
- Exact MSVC `14.44.35207` focused evidence after the final rollback/UI repair:
  camera catalog `21/21`, engine Art-Net `7/7`, Syndocal Art-Net `3/3`, and
  protocol control-plane `1/1`; failures and ignored tests are zero and
  first-party warnings are zero. The wire proof observes ch1/ch5 at payload
  bytes 0/4 and ch500 zero at payload byte 499. Independent Terra xHigh review
  is GO after closing the stale Setup I/O fixture and publication-failure
  rollback proof. Ox was unavailable for this narrow review exception.
- The historical alpha.34 tranche established the fixed two-Sender Spout
  state/validation baseline. Historical alpha.35 superseded it with the strict
  first-physical-black and post-send exact-name barrier recorded above;
  focused tests, independent review, and the native build passed. Physical
  Unity/GPU observation remains open and is not inferred from source evidence.
- The non-overwriting authoring tool received independent Terra xHigh GO after
  post-write cleanup was changed to retain an unverifiable target for explicit
  quarantine/manual removal instead of risking pathname-based deletion. It
  created `DSF2026-show-alpha10-same-pc-output.sdc` as a separate `1,098,035`
  byte file with SHA-256
  `DB1C18DCEFC79F5DC8C68589BCCAA492AF2509E932542D4A5036927B5E0814BA`.
  Alpha9 and both managed MP3 sidecars retained their pinned hashes.
- C: reached zero free bytes during the focused build. A dry run proved that
  Cargo profile `dev` alone owned `205,503` generated files / `400.0 GiB`.
  `cargo clean --profile dev` removed only those regenerable development
  artifacts, retained `target/release`, QA evidence, source, and authored SDC
  files, and restored `188,021,350,400` bytes free. The subsequent exact-linker
  focused builds regenerated their required graph successfully.
- The authoritative same-PC output gate is
  [qa/DSF2026_SAME_PC_OUTPUT_ACCEPTANCE_2026-08-29.md](DSF2026_SAME_PC_OUTPUT_ACCEPTANCE_2026-08-29.md).
  Remote Art-Net/NDI is deliberately out of the current acceptance scope.

- KDMX checkout: `C:\Users\kouty\Documents\KDMX`
- Branch: `codex/syndocal-v1.2`
- Last pushed alpha.30 source checkpoint: `9c9a96da6955f1ee0098468f30e16221ccaf779e` (`feat: add hybrid ASIO program and WDM cue routing`). It was pushed to `origin/codex/syndocal-v1.2` with exact upstream equality. No alpha.30 native artifact is accepted yet.
- Historical alpha.31 source authority: `602b96a8fcb0de3fd3a3e281324550fe1d7b5630` on `origin/codex/syndocal-v1.2`. It contains the Timeline authoring monitor plus the reviewed normal-build cfg correction.
- Pushed pre-alpha.32 Timeline-audio component checkpoint, included by the current alpha.32 integration: `fedf6c48fbab59b3f0c643da402fcc026ee74fbe` on `origin/codex/syndocal-v1.2` (`Synchronize Timeline audio varispeed clocks`). Its own product metadata was still alpha.31; the synchronized alpha.32 product identity begins with the subsequent version checkpoint. Child Timeline PROGRAM/CUE now uses one canonical 250..=4000 millirate for position, Rodio speed, drift, and inverse Sink seek; nested fractional rates are rounded once, ambiguous/invalid rates refuse audio attachment, and a negative `source_offset_ms` remains silent until its exact root-output boundary. Lighting keeps its established position path.
- The checkpoint's independent Terra xHigh rereview is `GO` with no P0/P1/P2. Supervisor evidence is exact MSVC 14.44 with the Community linker first and absolutely pinned, engine Timeline audio `31 passed / 0 failed / 0 ignored`, frontend Timeline-audio contract PASS, `cargo fmt --all -- --check` PASS, `git diff --check` PASS, and first-party warnings 0. The implementer also recorded ASIO media playback `80/80` and protocol nonserialization `1/1`, both warnings 0.
- The Timeline-audio component checkpoint itself had no new native or hardware acceptance. Its preserved pre-checkpoint process PID `73380` was later path-verified and terminated immediately before the alpha.32 native build; it was never relabelled as current evidence.
- The Windows candidate-gate source checkpoint is pushed at `53d70baacbc2c9ed3f719eaf2e67aab1766cf487`. Metadata completes all hash and updater-signature checks before any EXE inspector; candidate and manual three-root inspectors receive only verified-byte `wx` copies; NSIS/MSI/outer/nested archive tools likewise receive only pre/post-hashed copies. Independent review is implementation `GO`, while actual RC acceptance remains `NO-GO` until real signed bundles and Windows runtime identity evidence exist. Supervisor self-tests passed `125/43/4/140` plus `check:release`; Node's non-handle-atomic pathname and unprovable owner/DACL boundary is explicitly limited to fresh single-writer staging.
- The synchronized alpha.32 product identity is pushed at `5d40874c629f26d6e011252622b2886d39d8d40b`. From that clean pushed HEAD, the exact MSVC 14.44 Community linker was pinned and first in `where.exe`; `pnpm --dir app tauri build --no-bundle` passed in `2m 52s` with first-party warnings 0. `target/release/syndocal.exe` is `60,713,984` bytes, Product/FileVersion `1.2.0-alpha.32`, SHA-256 `B04CE351A456C82715383A4430401F3DA4D824EC8813B32894ABE1BF98FBD90D`. With the verified local FFmpeg runtime `bin` inherited, exactly one checkout-owned process launched as PID `89524`, is responsive with title `Syndocal`, and Computer Use verified the exact window is maximized. Audible and physical acceptance remain open.
- The alpha.34 integration includes the alpha.33 camera-capture source tranche, which replaces the old free-form DirectShow route fixed at `1280x720` / `30 fps` with an explicit current-generation device/profile catalog, opaque endpoint identity, and an exact one-frame probe before Add. Its envelope is maximum `4096x2160`; profiles above `1920x1080` admit no more than `30 fps`, profiles above `1280x720` no more than `60 fps`, and capture rates up to `120 fps` only when advertised. Output presentation remains at most `60 Hz`; screen capture stays `1280x720` / `30 fps`. The exact supervisor source gates passed capture `64/0/2` and control-plane `64/0`, with MSVC 14.44 and first-party warnings 0. Independent review found P0 `0`; source fixes close bounded listing memory, child cleanup, and automatic fault-row visibility, while sustained-4K performance remains open. The native alpha.34 build passed; visual UI and hardware probes remain pending. The acceptance authority is [qa/CAMERA_INPUT_ACCEPTANCE.md](CAMERA_INPUT_ACCEPTANCE.md).
- Alpha.32 cleanup inventory was refreshed read-only. The four known build/cache roots still exist; a new exact-byte traversal of `target` exceeded the bounded 50-second window and was stopped without changing files. The latest completed exact `target` inventory therefore remains `428,244,808,551` logical bytes. The reviewed recurring-cleanup harness is not approved for Apply, so no deletion ran and reclaimed bytes remain 0.
- The documentation checkpoint containing this record is accepted only after
  its commit is pushed, `HEAD` equals `origin/codex/syndocal-v1.2`, and the
  primary worktree is clean; the close procedure rechecks all three conditions.
  Three detached alpha18/19 release-gate worktrees are clean. The
  separate `C:\Users\kouty\Documents\KDMX-asio-persistence` worktree retains
  its pre-existing owned changes in `app/scripts/check-backend-operator-contract.mjs`,
  `check-frontend-command-routing.mjs`, `check-live-audio-input.mjs`,
  `app/src-tauri/src/control_plane.rs`, `app/src-tauri/src/main.rs`,
  `app/src/App.tsx`, `app/src/tauri-invoke-manifest.json`,
  `app/src/tauriInvokeCommands.ts`, `app/src/types.ts`,
  `qa/ASIO_INPUT_ACCEPTANCE.md`, `qa/harnesses/README.md`, and
  `qa/harnesses/check-asio-build.ps1`; none was touched by this checkpoint.
- Preserved stashes are `stash@{0}` (`e9209d6` alpha.9 validation WIP) and
  `stash@{1}` (orphaned open-DMX pacing WIP). Neither was applied or modified.
- Historical alpha.29 source checkpoint: `54a4ffcce0e2029d9f0aecc713ae4436228a4d3c`, committed as `fix: stabilize ASIO output lifecycle` and pushed to `origin/codex/syndocal-v1.2` on 2026-08-29 JST.
- DJ Agent checkout: `C:\Users\kouty\Desktop\rb-output`
- Branch: `beta-v1.1.2`
- DJ Agent committed HEAD and upstream: `a13d7bff59db5e7c00e19655f87c69db7cb52005` on `beta-v1.1.2`; its worktree was clean at the recorded checkpoint.
- The DJ Agent operator-return path received independent source-review `GO` with no P0/P1/P2. The external full regression passed `506 tests / 504 passed / 0 failed / 2 skipped` with first-party warnings 0. DJ-PC pull/restart, strict preflight, active runtime version, real ACK, and physical pedal acceptance remain external gates.
- KDMX product metadata is now `1.2.0-alpha.37`. Alpha.32 source integration
  and its native build, launch, and maximized-window gate remain historical.
  The alpha.34 same-PC output/camera native build, alpha.35 U0/Spout-hardening
  native build, and alpha.36 source/native/window authority are also
  historical checkpoints after the current source bump. The alpha.37 native
  authority is exact checkout PID `50864`, built from clean pushed HEAD
  `5626a9636003462a23daf0f3de67af3cc5060e29` and verified responsive and
  maximized. Real-device audition, physical output routing,
  camera hardware, DJ acceptance, show completion, real installer/updater
  inspection, and the dedicated show-ASIO artifact remain open. The camera
  acceptance authority is [qa/CAMERA_INPUT_ACCEPTANCE.md](CAMERA_INPUT_ACCEPTANCE.md).

### 2026-08-29 historical alpha.34 same-PC output and camera integration

The product identity is `1.2.0-alpha.34`; source and native build identity are
fixed, while visual and hardware acceptance remain open. The old camera route accepted a free-form
DirectShow endpoint at fixed `1280x720` / `30 fps`. The new source path uses an
explicit current-generation DirectShow device/profile catalog, persists only an
opaque endpoint identity, and requires an exact one-frame probe before Add. The
envelope is maximum `4096x2160`; profiles above `1920x1080` admit no more than
`30 fps`, profiles above `1280x720` no more than `60 fps`, and capture rates up
to `120 fps` only when advertised. Output presentation remains capped at
`60 Hz`; screen capture stays `1280x720` / `30 fps`. The acceptance authority is
[qa/CAMERA_INPUT_ACCEPTANCE.md](CAMERA_INPUT_ACCEPTANCE.md).

On 2026-08-29, the connected `Insta360 Link` advertised `3840x2160` at
`30 fps`; `1920x1440`, `1920x1080`, and `1280x720` at `60.0002 fps`; and no
`120 fps` profile. The final exact supervisor source gates passed
capture-filtered `71 / 0 / 2`, process-lifecycle `3 / 0 / 0`, control-plane
`64 / 0 / 0`, full no-default `1203 / 0 / 7`, and full default
`1239 / 0 / 12` under MSVC `14.44.35207`, with first-party warnings `0`.
The first parallel no-default run exposed two unrelated coordination-test
timeouts (`1201 / 2 / 7`); both passed individually and the full serialized
suite passed without a source change for those tests. Independent Terra xHigh
rereview returned GO with P0/P1/P2 `0`; source fixes close bounded listing
memory, post-spawn child cleanup, deferred reaper/quarantine ownership, and
automatic stale Active-row replacement. App exit before a deferred reaper
finishes remains an explicit unverified non-Job-Object OS boundary. Sustained-4K
performance remains open.

Direct FFmpeg preflight completed 150 RGBA frames at 4K30 and 300 at 1080p60,
both exit `0`. The exact Syndocal profile probe still proves one frame only.
Sustained 4K remains unverified:
the current RGBA `Vec` clone at `60 Hz` may approach `1.98 GiB/s` of copy
traffic. Native alpha.34 UI and hardware probes remain pending; no visual or
hardware completion is claimed here.

At pushed checkpoint `fd0d40698ae849a0f327fa7769b06b2af182c82f`, the
camera/Art-Net independent source rereview was GO with P0/P1/P2 `0`. That
checkpoint's full app regression passed `1203/0/7` without default features
and `1239/0/12` with default libav/Spout features. Those counts predate the
current fixed two-Sender Spout integration and are not its full-regression
evidence.

The historical committed alpha.34 Spout integration has focused exact-linker
evidence of Syndocal show-Spout `20/0/0`, engine strict-pair `5/0/0`, and
protocol v4 command `11/0/0`, all with first-party warnings `0`. It holds both
senders on opaque black until the durable output lease commit, binds every
send to the exact output-ownership generation/epoch and project callback
epoch, blocks generic mutation of the fixed pair, and preserves unresolved
engine-retirement identity until a later R4 reconciles it. That barrier is
process-session scoped: process exit destroys the SDK senders, engine instance,
and pending ACK queue and is the explicit recovery boundary for an infinite
driver call. Independent Terra xHigh final rereview returned GO with no
unresolved P0/P1. P2 proof debt remains for a true two-worker fake-SDK timing
test, and the new show modules only partially reduce the existing oversized
app/engine orchestration files. Final exact-linker regression passed Syndocal
no-default `1205/0/7`, Syndocal default `1263/0/12`, and engine `920/0/2`;
the remaining workspace crates exited successfully and first-party warnings
were `0`. The initial matrix found only a stale expected command count and an
older unreferenced Cue 3 test fixture. Explicit command-variant assertions and
removal of the dead fixture closed both; focused reruns and the full matrix then
passed. A fresh exact-linker native build from clean pushed HEAD `138f6c3`
completed in `3m45s` with first-party warnings `0`. The resulting
`target/release/syndocal.exe` is `61,039,104` bytes, Product/FileVersion
`1.2.0-alpha.34`, SHA-256
`1FCB899E2B118B94F92B5D87ECD7A5EA3FFE32D33448319FE697D39841FA642F`.
Exactly one checkout-owned process, PID `85492`, launched and remained
responsive. Computer Use did not expose that native window as targetable after
one explicit-path recovery attempt, so maximized-window proof and Unity/GPU
observation remain pending; no visual native acceptance is inferred from the
responsive process alone.

Frontend production build, typed command/output-control checks,
`check:release`, localization/IPC inventory, format, diff, and full Rust gates
pass for the frozen source checkpoint. No native or physical output claim is
promoted from these deterministic results.

### 2026-08-29 historical alpha.30 hybrid PROGRAM/CUE source checkpoint

- The dirty alpha.30 source/UI tree now carries two explicit delivery modes:
  `CueDelivery::SameAsio` keeps PROGRAM and CUE on one selected ASIO stream and
  shared clock; `CueDelivery::ExplicitWdm` keeps PROGRAM on ASIO while the
  explicitly named WDM endpoint, including its enumerated topology fingerprint,
  owns CUE.
- Timeline CUE clips, generated Click/Guide, and the explicit CUE test path
  are wired through the same selected logical CUE route. Missing, ambiguous,
  stale, changed, or failed endpoint/session state is visible and fail-closed;
  no ASIO/PROGRAM/default-device fallback or CUE leakage to PROGRAM is allowed
  by the source contract. Session/generation fences cover activation,
  publication, timeline preparation, and retirement.
- Exact MSVC 14.44 Community-linker gates finished with first-party warnings 0.
  The focused ASIO media-audio gate passed `64/64`; the final full
  `cargo test -p syndocal --features asio -- --nocapture --test-threads=1`
  passed `1435 / 0 failed / 12 ignored`. `cargo fmt --all -- --check` and
  `git diff --check` passed (line-ending notices only).
- TypeScript, Vite, frontend command routing, audio-control `74` static plus
  `57` runtime assertions, audio-panel `53`, localization `3615/3615`, and
  `check:release` passed. Release checking included packaging `169`, ASIO v3
  `22`, and Timeline output-bus `11` assertions. Independent Terra xHigh source
  review returned GO with no P0/P1; physical WDM audition and independent-clock
  observation remain external.
- No alpha.30 native build/launch/window, real-device audition, physical output
  routing, or show completion has been verified. The oversized audio-runtime
  extraction from `app/src-tauri/src/main.rs` is deferred until after show
  acceptance because changing ownership/lifecycle boundaries before the show
  is a pre-show risk; no module-split completion is claimed.
- The next show-critical source item is a separate Timeline-authoring monitor:
  while the output router is Normal, media-library Timeline clips plus generated
  Guide/Click must share one explicitly selected WDM endpoint such as
  `Music (Elgato Virtual Audio)`. Entering show ASIO must retire that authoring route;
  missing, ambiguous, or changed endpoints must remain silent and fail closed.

### 2026-08-29 historical alpha.31 Timeline-authoring monitor checkpoint

- The Normal route now has an explicit Timeline-authoring monitor. `FollowProgram`
  preserves the existing Normal behavior; `ExplicitDevice` sends every Timeline
  media clip (logical PROGRAM or CUE) and generated Guide/Click material to one
  operator-selected WDM endpoint. The endpoint is arbitrary and machine-local,
  with exact name/topology revalidation before publication and preparation.
- Missing, ambiguous, stale, changed, or failed endpoint/session state remains
  visible and silent. There is no default-device, PROGRAM, or Show-ASIO fallback.
  Entering Show ASIO retires all Normal-authoring Timeline sinks, even when no
  normal PROGRAM stream is open; returning to authoring requires an explicit
  Normal route and output selection.
- Exact MSVC 14.44 Community-linker gates completed with first-party warnings
  0. The ASIO-enabled full suite passed `1456 discovered / 1444 passed / 0
  failed / 12 ignored`; focused media-audio passed `72/72`. TypeScript, Vite,
  release metadata/checking, audio-output control/panel, Timeline-audio,
  command-routing, and localization checkers passed.
- The normal alpha.31 native no-bundle build passed from clean pushed HEAD
  `602b96a` using exact MSVC 14.44. `target/release/syndocal.exe` is
  `60,756,480` bytes with SHA-256
  `6F9BF17A2802A2FC5F8935E8EFFEB0C57CA4A3A21B245C68BFA62315152C7F4A`.
  Exactly one launched process was responsive. The exact native window was
  maximized through its verified process handle. Product-path enumeration found
  exactly one selectable `Music (Elgato Virtual Audio)` endpoint with topology
  fingerprint
  `A2D9603C75ED1A6ECBC37F0FE851AAD56C632DFF31314544F2F606C220C9479C`.
  The exact selection was persisted at
  `%LOCALAPPDATA%\jp.seraf.ktn.syndocal\timeline-cue-audio-settings.json` while
  Syndocal was stopped, followed by a successful relaunch; audible output is
  not yet verified. The responsive maximized process at that checkpoint was PID `100320`.
  Physical output, venue
  routing, DJ Link, serial DMX, reconnect, and show completion remain external
  gates. Ox was unavailable for this tranche; under the documented narrow
  exception, an independent Terra xHigh review returned GO with no P0/P1.
- The oversized audio-runtime extraction from `app/src-tauri/src/main.rs` is
  intentionally deferred until after show acceptance; no module-split
  completion is claimed. The next safe action is the real endpoint audition.
- A subsequent source-only I/O clarity delta keeps the backend selector visible
  in Normal WASAPI, hides Show-ASIO-only lifecycle/channel/preflight controls,
  identifies PROGRAM as the Windows default output, and directs arbitrary WDM
  Timeline media/Guide/Click routing to Timeline tools -> Timeline authoring
  monitor -> Explicit Device. Its focused panel checker passed `58` assertions,
  localization passed `3616/3616` with zero unprotected labels, TypeScript
  passed, and independent Terra xHigh review returned GO with no P0/P1/P2.
  Native visual acceptance of this delta remains open.
- The first normal native attempt exposed a non-ASIO-only compile defect: the
  ASIO-gated `route_gate` declaration had one unconditional `drop`. The new
  path applies the identical cfg to that drop, preserving ASIO lock order while
  removing the undefined name from normal builds. The one-line fix is commit
  `602b96a`; an independent Terra xHigh rereview returned GO with no P0/P1.

### 2026-08-29 historical alpha.29 ASIO lifecycle safety checkpoint

- `AsioReady` can now return explicitly to Normal or be invalidated by profile
  reselection. The exact Ready ticket is cancelled first, leaving `Locked`
  with no operation/session; a validated Start is then rejected until an
  explicit revalidation. Reselect/Revalidate/Start/Stop/Normal selection are
  serialized by one lifecycle lock. Active or otherwise ambiguous states
  remain rejected without mutation.
- The show-ASIO first-party warning inventory is zero without warning
  suppression. Test-only router/result/proof helpers are compiled only for
  tests; production generation and live-audio fences remain present.
- Independent adversarial review returned GO with P0/P1 zero. Its only code
  observation was a stale lock comment, corrected before this commit.
- Exact MSVC 14.44 preflight and pinned Community linker were used. The
  show-ASIO release check completed with first-party warnings 0. The complete
  show-ASIO application test run discovered 1409 tests and finished 1397
  passed / 0 failed / 12 ignored. The frontend audio-output contract passed
  59 static assertions and 28 runtime assertions; TypeScript/Vite transformed
  302 modules. `check:release` passed including packaging 169, ABI v3 22, and
  Timeline output-bus 11 assertions. `cargo fmt --all -- --check` and
  `git diff --check` passed (Git line-ending notices only).
- `pnpm --dir app tauri build --no-bundle` completed successfully with
  first-party warnings 0 and produced the alpha.29 release executable.
  Native launch acceptance is still open: the exact process was responsive
  and setup tracing reached the end of the Tauri setup callback, but only the
  internal 16x16 single-instance window was exposed and no user-facing
  `Syndocal` window appeared. The temporary trace instrumentation was removed.
  This checkpoint therefore makes no native-window GO claim.
- The user has explicitly expanded the output requirement beyond the original
  one-device contract: PROGRAM and CUE must be independently assignable to
  physical output devices. Immediate acceptance target is TOPPING E2x2 ASIO
  outputs 1/2 for PROGRAM and a separately selected headphone/WDM endpoint for
  all CUE sources. Same-ASIO shared-clock mode remains supported; hybrid mode
  must state that it uses two clock domains, must never fall back or leak CUE
  to PROGRAM, and must fail closed on missing or ambiguous endpoints. This
  hybrid path is not implemented or hardware-accepted at the alpha.29
  checkpoint.

### 2026-08-29 historical alpha.28 integrated source checkpoint

- The previously open application integration is now present on the working
  tree. PROGRAM/CUE render, output runtime, Timeline output/transport, output
  router, normal-output boundary, bridge-v3, and preflight logic are split into
  dedicated modules. This historical checkpoint did not extract the remaining
  `TimelineCueAudioRuntime`/`MediaAudioPlayback` ownership from the already
  oversized `main.rs`; that larger application-runtime split is still deferred
  because changing its lifecycle boundary before the show is a pre-show risk.
  The only final `main.rs` repair at this checkpoint was a deterministic
  test-startup fence; production behavior was not loosened.
- The final independent ASIO drain review is `GO` with no P0/P1. Stop fences
  the callback context before join, Fault uses one state-lock interval, stale
  queued Test/Solo blocks are silenced through the two-callback drain, and
  arbitrary invalid-width/partial/non-finite blocks fail to full-width silence.
  Supervisor evidence is `165 passed / 0 failed` for exact `asio_` focused
  tests with first-party warnings 0.
- The full parallel Syndocal show-ASIO suite passed from a fresh rerun after
  repairing two test-only races: `1366` discovered, `1359 passed`, `0 failed`,
  `7 ignored`, first-party warnings 0. The repairs use a test-only router slot
  rather than competing for the production process owner, and wait for the
  engine startup snapshot before atomic no-mutation comparisons. Focused proof
  after repair was `25/25` router, `48/48` Timeline CUE, `3/3` ASIO Timeline
  Play, and `1/1` for each DJ rejection boundary.
- The final accepted default workspace gate ran with the CI-equivalent
  `--test-threads=1` setting and passed `2708` tests with `0` failures and `17`
  explicit hardware/GPU/long-duration ignores. First-party warnings were 0.
  A deliberately broader parallel `--all-features` diagnostic is not counted
  as acceptance: it first lacked the NDI runtime DLL path and later exposed
  unrelated parallel native/socket-process instability. The exact `io` suite
  passed serially (`166 passed / 0 failed / 2 ignored`), and the accepted full
  workspace rerun is the serial result above.
- Release metadata and frontend contracts passed: ASIO packaging `169`, ABI v3
  `22`, Timeline PROGRAM/CUE UI `11`, project-open bootstrap, video output
  routing/window observation, and TypeScript/Vite production build with `302`
  transformed modules. `cargo fmt --all -- --check`, three Show-ASIO script
  syntax checks, and `git diff --check` pass; first-party warnings remain 0.
- The local-only artifact authority is now split into artifact source commit
  `S`, evidence HEAD `E`, and source branch `B`. The checker rejects dirty or
  unpushed evidence, a wrong branch, non-ancestor `S`, merges, D/R/T changes,
  non-allowlisted evidence paths, and source mutation followed by revert. Its
  schema is a clean-break v3 with manifest `commit=S` and `sourceBranch=B`.
  The final source identity contains `70` exact paths, including
  `app/src/uiLocalization.ts`; every Git authority query rejects repository,
  index, object, and config overrides and disables replacement objects.
  Supervisor self-tests pass: checker `155`, runtime staging `16`, build
  orchestrator `43`, and three-display harness `90/90`, with native/Cargo/
  process actions explicitly not run by those self-tests. Independent final
  rereview is `GO` with no P0/P1. PATH-resolved Git executable pinning remains
  non-blocking P2 hardening.
- Exact normal native acceptance passed after removing two release-only
  dead-code warnings with correct cfg boundaries: `pnpm --dir app tauri build
  --no-bundle`, first-party warnings 0. The exact executable is 60,599,296
  bytes, SHA-256
  `3080A3664B4657D6B6D6EFE76448DBE13912CA118262E78C0AE02B8DC6E85C6B`.
  Exactly one process at this checkout path exposed exactly one responsive
  `Syndocal` window and was verified maximized. It was then terminated by exact
  resolved path before the final tests. This does not claim normal installer/
  updater inspection or the separate show-ASIO artifact.
- No MOTU M4 stream, M32/DL16 route, or USB-DMX physical output is claimed yet.
  This machine currently exposes no MOTU M4 device and no serial port, so those
  physical rows must be run on the equipped show system. The integrated source
  checkpoint is committed and pushed at
  `32a092267f55d32185b2cf9cc123f92067614ec1`.

## Accepted source boundaries

### Pedals and DJ Link source boundaries (not physical acceptance)

- Pedal 1 / F13 owns the current loop toggle in Timeline-control mode.
- Pedal 2 / F14 owns loop-half in both modes: Rekordbox MIDI loop-half in DJ-control mode and `DJ_TIMELINE_LOOP_HALF` for an active Syndocal Timeline loop in Timeline-control mode.
- Pedal 3 / F15 owns Timeline `+4 bars` only in Timeline-control mode.
- F13 DJ release starts HPF and emits the correlated `DJ_RELEASE` on the same edge. The local Rekordbox action then completes HPF, ChannelFader fade, stop, and reset independently of Syndocal delivery.
- Stage 2 commands require exact Timeline/play-session/release authority and revalidate current playing Timeline identity in the engine worker before mutation.
- Re-enabling a completed loop after position B re-enters at A; disabling does not jump.

Focused source evidence preserved in the committed alpha.28 source checkpoint:

- protocol DJ Link: 14 passed, 0 failed, first-party warnings 0.
- engine DJ Link: 30 passed, 0 failed, first-party warnings 0.
- Syndocal DJ Link: 119 passed, 0 failed, 1 ignored live-network test, first-party warnings 0.
- Independent Stage 2 adversarial review: GO.
- DJ Agent v1.1.10 independent adversarial review: GO. Supervisor focused rerun: 88 passed, 0 failed; `git diff --check` passed before commit.

### Imported fixture stage layout

- `960 sound waves strongpoint` is authoritative as four physical cells and four logical RGB segments. Daslight's twelve displayed cells are the known three-row duplication bug, not twelve physical emitters.
- The Strongpoint collapse requires exact normalized profile identity, twelve raw DVC cells, four logical segments, and the exact three-row duplication pattern. A fixture label cannot trigger the exception; unrelated twelve-cell profiles remain twelve cells.
- Mega Bar remains eight physical cells and eight logical RGBA segments, with separate global dimmer and strobe controls.
- Physical-only layouts remain physical-only; the frontend does not infer color roles from control names.
- Unknown persisted layout fields and duplicate PATCH beam indices fail closed.

Focused source evidence preserved in the committed alpha.28 source checkpoint:

- protocol stage-layout validation: 13 passed, 0 failed, first-party warnings 0.
- DVC stage-layout unit tests: 5 passed, 0 failed, first-party warnings 0.
- duplicate PATCH beam rejection: 1 passed.
- synthetic exact topology import: 1 passed.
- operator-owned `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` acceptance: 1 passed with Strongpoint 4/4, Mega Bar 8/8, and wristband layout absent.
- TypeScript build, Strongpoint browser contract, mapping viewport conformance, stage-label, mapping geometry, and fixture live-color checks: passed.
- Independent physical-layout adversarial review: GO.

### Show authoring and safe publication

- The DSF2026 authoring tool uses a Windows fail-closed safe-write path with exclusive create, reparse rejection, identity revalidation, flush, exact-length verification, and cleanup on failure.
- Independent safe-write review: GO; focused tests 3 passed.
- Fresh native import/save evidence uses the exact operator-owned source `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`. The report is exact at `fixtures=46`, `profiles=12`, `fixture_groups=15`, `cues=2`, `Converted=93`, `Approximate=4`, `Skipped=0`, and `Unsupported=1` for one Daslight hardware binding. The saved base has exact stage layouts: six Mega Bar layouts with 8 cells and 8 logical segments each, plus three Strongpoint layouts with 4 cells and 4 logical segments each.
- Historical alpha.27 Native Save As output artifact: `target/qa/dsf2026-native-alpha27/DSF2026-imported-alpha27.sdc`, `1,079,564` bytes, SHA-256 `B21165A70A41A4036153359E579E1433C2739EC1C3EDCC0C46B94F513238DFB1`.
- Alpha.15 is retired as a current authoring base. Its artifacts remain historical evidence only and must not be supplied to the pinned authoring CLI.
- The former alpha3-alpha8 candidates are superseded. The reviewed final candidate is the alpha9 reference-audio artifact documented below; do not deploy an earlier candidate.
- The current supervisor rerun passed the authored-show test, DJ Link frontend/runtime contract, localization `3556/3556` with 0 unprotected labels, stage-label contract, fixture-limit degree contract, Timeline context-menu browser contract, I/O disclosure scroll contract, Strongpoint segment browser contract, Stage Settings viewport contract at five sizes, TypeScript/Vite production build, release metadata, Tauri wrapper self-test, and the `frontend-typescript-vite-windows` warning ratchet. First-party warnings were zero.
- Historical alpha.27 native release evidence: the exact MSVC 14.44 linker gate produced `target/release/syndocal.exe` at `60,314,624` bytes with SHA-256 `CEBB44C713043CCE885D87E3651464F3756A5CEE2D1700728D412AC7B18C48EC`.
- Historical alpha.27 process evidence: exactly one checkout-owned process was responsive after that build, PID `87732`, HWND `124064278`, and the verified Syndocal window was maximized. This is not alpha.28 native acceptance.
- This handoff claims no DJ HELLO/ACK exchange, physical pedal acceptance, or physical DMX-output acceptance; those remain unverified.
- `target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha3.sdc` passed the current structural Timeline preflight, including the one-measure transition and indefinite pedal-release holds.
- That alpha3 artifact is not the final show file because its base predates the newly imported fixture `stage_layout`. Do not deploy it as the final stage-layout authority.

### 2026-08-28 reference-audio authoring delta

- The two operator-supplied MP3 files are now accepted only as rehearsal/reference material through the ordinary Media Library -> Timeline Audio Clip path. They are not click/guide assets and are not armed for show playback.
- The authoring helper preserves both originals, copies byte-identical files beside the generated candidate under ASCII names, records SHA-256/byte size/duration, and refuses overwrite or divergent existing bytes.
- Each authored song Timeline receives one ordinary Audio layer with one `media_asset_id` clip. The layer is muted by default and requires an explicit operator unmute before rehearsal playback.
- The operator-saved current production candidate `target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha9-reference-audio.sdc` supersedes the earlier alpha9 byte identity. It is `1,095,864` bytes with SHA-256 `93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094` and passes the current structural preflight. Measured media durations remain `214032 ms` for 人生オーバー and `273432 ms` for 惑う星; source and managed sidecar SHA-256 values match.
- Rehearsal playback now uses a distinct sibling `DSF2026-show-alpha9-rehearsal-reference-audio.sdc`, `1,092,410` bytes with SHA-256 `AC9133AFD2C9AAD022674B45222447115F624175B2C5E20DE1E803ABAA9778CE`. Its `956`-byte approval manifest has SHA-256 `ADA244FE095B48BC67DE6181F1D9EFE55131EF720A4D2B5EEB91714A7DA85AED`. The reviewed fail-closed creator pins the production source and both MP3 identities, requires that exact manifest, creates exclusively, and permits only the three synchronized `Reference Audio` layer changes from `muted=true` to `muted=false`. Independent post-publication audit confirmed no other semantic JSON differences, both source and copy passed all ten structural checks, and the source candidate and sidecars remained byte-identical. The rehearsal copy has not been loaded into the running native app or audibly accepted.
- Windows publication holds verified non-reparse parent/leaf handles across source and sidecar revalidation through candidate flush. Parent/leaf substitution, same-hash reparse substitution, partial write, and flush failure fail closed without candidate or temporary-directory residue.
- Focused authoring regression passed with first-party warnings 0, including explicit no-reference assertions for zero Timeline Audio Clips and zero added Media Library assets. The final independent Terra xHigh review is `GO` with no P0/P1.
- Alpha9 loaded successfully into the exact native process through the single-instance project-forwarding path and reported 46 fixtures plus the expected 12 embedded profiles and mappings. Timeline Audio Clip playback remains unverified.
- A cold command-line launch with alpha9 exposed an open startup race: `Project authority changed before mutation (expected epoch 0 revision 0)`. Starting Syndocal first and forwarding the project path then loaded successfully. This race is not accepted or hidden; fix it in the next narrow tranche before cold-start deployment is claimed.

### 2026-08-28 machine-local USB-DMX delta

- The project retains only the logical serial DMX route. COM/PnP identity is stored in machine-local state and is selected from a current-device dropdown, then persisted only by an explicit Confirm action.
- Missing, stale, ambiguous, renumbered, and A->B->A physical identity changes fail closed. The exact opened Windows handle identity is revalidated rather than trusting a selected port label.
- The S0 blackout epoch is linearized ahead of activation/live ticks. The same physical write gate is held from selected-frame authority through BREAK, MAB, `write_all`, and `flush`; S0-first therefore permits zero live frames, while worker-first permits at most the already-started live frame followed only by zero frames.
- The deterministic worker proof uses the real `EnttecOpenDmxSender` worker and real Open-DMX write sequence with a fake `SerialPort`, including the public `safety_blackout_engage_published` interleave and non-deadlock assertion.
- Focused Rust evidence: `cargo check -p engine`, serial DMX 14/14, engine DMX 29/29, show route 4/4, machine binding 3/3, serial A->B->A 1/1, first-party warnings 0. Frontend output-control, production build, and exact command routing/invoke gates passed.
- Final independent adversarial review is `GO` with no P0/P1/P2. Physical USB-DMX/fixture output remains open.
- Final source gates after the active/bank normalization repair: exact MSVC 14.44 workspace Rust `2683 passed / 0 failed / 17 ignored` across `2700` executed tests, followed by the final exact-gated Syndocal rerun `1199 passed / 0 failed / 12 ignored` across `1211` tests; frontend production build; release/ASIO packaging `169` assertions; Tauri wrapper `231` assertions plus `27` hostile fixtures; exact frontend route/invoke inventories; Stage label, Strongpoint, fixture-limit degree, and remote disclosure contracts. First-party warnings remained 0; `cargo fmt --all -- --check`, `git diff --check`, and `pnpm --dir app run check:release` passed.

### ASIO PROGRAM/CUE output gate added 2026-08-28

- `qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md` is now the authoritative output gate. ASIO output is a show-critical requirement, not optional polish.
- Adopted architecture C: retain the exact input/Reactive Capture ABI/schema v2 surface, and add an exact v3 output/full-duplex surface to the same canonical bridge DLL. ASIO playback uses one v3 session; a parallel v2 session on the same driver is forbidden and must fail busy.
- PROGRAM stereo and CUE mono are project-level logical buses. Physical PROGRAM L/R, CUE, optional Spare, driver identity, sample rate, format, and buffer remain machine-local. Missing bus data migrates one way to PROGRAM; CUE never falls back to PROGRAM.
- Two logical Rodio mixers feed a non-realtime renderer and bounded preallocated interleaved SPSC. The ASIO callback only copies a complete block or outputs a complete silent block and latches terminal Fault. PROGRAM/CUE share one device and clock only in `SameAsio`; `ExplicitWdm` intentionally uses independent ASIO and WDM clock domains.
- DSF2026 acceptance mapping is MOTU M4 Output 1/2 = PROGRAM L/R and Output 3 = CUE at exact 48 kHz, with Output 4 optional Spare. This is a selectable profile, not MOTU-specific code.
- Device/rate/mapping conflict, disconnect, XRUN, reset/resync, buffer/rate change, callback gap, or underflow must stop output without WASAPI/default-device/rate fallback. Explicit revalidation and Start are required.
- Independent Terra xHigh review added three implementation-blocking P0 boundaries: quiesce and join the legacy `FollowProgram` CUE/normal Rodio output before v3 Start and stay silent on Start failure/Fault; freeze exact v3 callback/queue/lifetime semantics before code; and prove arbitrary non-contiguous/reordered physical mappings with all unselected channels zero. It also requires exact queue/race injection tests and updates every show-ASIO v2-only export checker to the exact v2-nine plus v3-nine set.
- The acceptance contract was committed and pushed at `e583141cc60decff7c062db21a39f69241f894c8`. At that historical contract checkpoint, the `1.2.0-alpha.28` implementation was still uncommitted; the authoritative 2026-08-29 integrated checkpoint above supersedes that state.
- Integrated source and independent review are complete. A historical pre-alpha.30 normal no-bundle build completed, but its user-facing window gate did not; alpha.30 has no native build/window evidence. Dedicated show-ASIO artifact/loader proof, normal installer/updater inspection, authoritative non-default Timeline speed synchronization, physical output proof, and M32/DL16 routing proof remain open.

### 2026-08-28 alpha.28 ASIO implementation checkpoint (superseded source snapshot)

This subsection preserves the earlier partial snapshot. The 2026-08-29
integrated source checkpoint above is authoritative for current source status.

- Source authority remains branch `codex/syndocal-v1.2` at committed/pushed parent
  `8153ebb37a9517aad91c0da6ad06de9a80db2a1a`. The `1.2.0-alpha.28` source below
  is a shared uncommitted working checkpoint; production code integration,
  native acceptance, and physical acceptance remain pending, so no deployment
  or physical acceptance follows from it.
- Portable Timeline Audio Clips now carry only `PROGRAM | CUE`. Missing legacy
  values become PROGRAM, explicit invalid/future values reject, and root,
  child, import, save/reload, split, lane move, and sink source identity retain
  the logical bus. A stale full authoring/bank publication cannot overwrite a
  newer dedicated bus mutation. Bank preservation is keyed by
  `(TimelineId, clipId)`, so two Timelines that reuse a clip ID cannot
  contaminate one another. Independent Terra xHigh rereview is GO for this
  engine boundary; exact MSVC 14.44 focused evidence is 15/15 Timeline audio,
  1/1 strict protocol bus rejection, 1/1 root and child invoke-wire checks,
  1/1 sink retirement, and 11/11 static UI assertions, with first-party
  warnings 0.
- The v3 bridge is an actual ASIO SDK output/full-duplex implementation, not a
  CPAL callback placeholder. It owns explicit driver load/init, exact tuple
  negotiation, one `ASIOCreateBuffers` set, output/full-duplex callbacks,
  `ASIOStart`, terminal faulting, and drained Stop/Close. Full-duplex input and
  output must share the exact sample rate and fixed buffer before driver open;
  both mismatch directions reject without a handle, session, result, or false
  `actualInput`. Independent Terra xHigh rereview is GO with P0/P1 0. Final
  bridge evidence is default 31/31, ASIO 34/34 plus one physical ignored test,
  clippy `-D warnings` in both graphs, and exact v2 nine plus v3 nine exports.
  The current bridge DLL is 926,208 bytes with SHA-256
  `B21713C5C3F4A3ECA1C3A12769E05076136A43D40D7FBF225DDE525D6792A294`.
- The isolated `AudioOutputRouter` core received independent Terra xHigh GO.
  It provides process-wide ownership, sealed/ticket-bound source creation and
  recovery, late-prepare retirement, retryable exact resource teardown,
  monotonic callback drain, explicit Ready cancellation and return to Normal,
  and fail-closed identity exhaustion. Its standalone proof is 12/12 with
  first-party warnings 0. It is not yet a claim that the production app has
  routed every Rodio constructor through the owner.
- Prior focused ASIO preflight evidence is 17/17, 11/11, and 15/15, with
  first-party warnings 0. This is preflight/source evidence only; it does not
  close production code integration, the alpha.28 native build, or hardware
  acceptance.
- The show artifact source identity now includes the application owner/cue
  runtime, both v2/v3 loaders, router and tests, render/runtime modules, bridge
  build script, shared lease, ABI/native/SDK FFI and C++ callback sources,
  headers, tests, manifests, and trusted helpers. Critical runtime mutation is
  rejected before DLL inspection. The checker self-test currently passes 53
  assertions; the exact identity count is 55 after adding the split app render
  and runtime modules, the normal-output lease, the compact Audio control/UI
  integration and its focused checkers, plus the complete fifteen-file bridge
  identity.
- Current SDK-independent alpha.28 gates: TypeScript/Vite production build PASS
  (`302` modules), and `check:release` PASS including synchronized release
  metadata, ASIO packaging `169` assertions, v3 contract `22`, Timeline
  PROGRAM/CUE bus `11`, and existing video route/window gates. These checks do
  not establish complete production integration or native/hardware acceptance.
- At this superseded alpha.28 snapshot, the following work was still open:
  production app code integration, including
  v3 lifecycle wiring, two
  Rodio mixers drained by one non-RT worker into a preallocated bounded SPSC,
  app-owned mutable transport generation, complete normal/FollowProgram/
  ExplicitDevice quiesce, machine-local IPC and compact Audio UI, independent
  app/UI review, exact normal and show-ASIO native builds, real loader smoke,
  and all MOTU M4/M32/DL16 physical rows.
- Cleanup inventory was refreshed read-only at this checkpoint: `target`
  `94,214,143,142` logical bytes across `86,760` files, `app/node_modules`
  `545,338,492` bytes, `tools/asio-bridge/target` `1,774,985,879` bytes, and
  `app/dist` `5,204,539` bytes. Git also reported one approximately `568 KiB`
  garbage object. The accepted recurring deletion conditions are not currently
  satisfied, so no cleanup Apply or ad-hoc deletion ran and reclaimed bytes
  remain `0`.

## Required remaining acceptance

1. The alpha.39 native checkpoint is recorded from source/docs `HEAD` and
   upstream `ec93e9160da853ad181de70aee4db7b4a75fafbb`: exact MSVC `14.44.35207`
   pinned/where-first, build exit `0` in `2m57s`, warnings `0`, artifact
   `61,108,736` bytes with SHA-256
   `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
2. The exact launch checkpoint is complete: one responsive/maximized
   `Syndocal` window (PID `87640`, id `2033716740`) was verified and Daslight
   was preserved. The reviewed native-evidence record was committed/pushed at
   `94b362bd2d733e447feabf0a0a6158699da6a2bf`, followed by verified
   `HEAD`/upstream equality.
3. Alpha9 native UI reverification is complete without clicking the output
   selector or Refresh. The exact sequence was Play -> Pause -> status-only wait
   -> Play -> Pause -> status-only wait; throughout it, explicit-device and
   resolved output stayed at `Music (Elgato Virtual Audio)`, lifecycle `実行中`,
   `rev1`, with advancing output frames and no visible Backend, Local IPC, or CUE fault.
   The project remained unsaved and final Timeline state was paused; the
   historical alpha.38 drift to `3 - PX160 WAV...` was not observed.
4. Ask the operator to audibly confirm Timeline media and Guide/Click through
   the selected `Music (Elgato Virtual Audio)` endpoint in Normal mode, then
   verify ASIO Start retires the authoring route and stale/missing/ambiguous
   devices remain silent.
5. Build and inspect the normal NSIS/MSI/updater artifacts so the complete default-distribution ASIO-free gate is measured, not inferred only from source packaging tests.
6. Build and verify the exact local-only show-ASIO artifact from the clean
   pushed source checkpoint: exact 18 exports, v3 S/E/B manifest and source
   hashes, real loader Start/Stop/Fault smoke, and one responsive maximized
   Syndocal window.
7. Verify the now-implemented canonical non-default Timeline speed synchronization on the selected real PROGRAM/CUE devices; source and deterministic gates are closed, but physical playback remains open.
8. The DJ-Link PC may remain stopped during local audio work. Before final acceptance, pull the committed DJ Agent checkpoint without exposing the token and confirm strict preflight, active runtime version, real ACK, reconnect snapshot recovery, and physical Pedal 1/2/3 behavior.
9. For this show, verify the strict same-PC Art-Net route at `127.0.0.1:6454`, wire U0, 512 bytes, including the ch1/ch5 red-frame proof and ch500 zero. Generic serial DMX remains a product capability but is not part of this show route. Close Daslight/Easy View manually before Unity because the receiver port cannot be shared.
10. For the immediate split-device target, verify TOPPING E2x2 ASIO PROGRAM on Outputs 1/2 and the explicitly selected WDM headphone endpoint for CUE. If the venue instead supplies the preferred multichannel route, perform MOTU M4 at exact 48 kHz and M32/DL16 physical acceptance as recorded in the detailed gate.
11. Update this handoff with physical evidence and exact artifact identities. Hardware, real ACK, serial DMX, ASIO device, M32 routing, reconnect, installer/updater inspection, and dedicated show-ASIO acceptance remain explicitly unverified until observed.

## First safe resume actions

- Do not regenerate the final show from alpha3 or deploy superseded alpha4-alpha8 reference candidates; alpha9 is the reviewed reference-audio candidate.
- The exact alpha.38 PID `55624` and its maximized window are historical
  evidence only; preserve the artifact identity and the `PX160` display drift
  record, but never treat that process as alpha.39. Preserve Daslight PID
  `42752` unless the operator explicitly authorizes closing it. The DJ-Link PC
  may remain stopped until the final integration gate.
- Alpha.35-alpha.37 source/full-regression, independent review, and native build
  gates are historical. Alpha.38 native/build/window evidence is also
  historical at source/build HEAD `e4ec22384675aace5ed3912ddffdcfecca190919`.
  Alpha.39 automated source gates and independent reviews are complete; the
  native build/window checkpoint used source/docs `HEAD` and upstream
  `ec93e9160da853ad181de70aee4db7b4a75fafbb`, with exact MSVC `14.44.35207`
  pinned/where-first, build exit `0` in `2m57s`, first-party warnings `0`,
  artifact SHA-256 `7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`,
  and one responsive maximized PID `87640` / window id `2033716740`.
  Native alpha9 UI selector/status-only reverification also passed without
  clicking the output selector or Refresh; explicit/resolved output stayed
  `Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`, with advancing
  frames and no visible Backend, Local IPC, or CUE fault. The reviewed
  native-evidence record is pushed at `94b362bd2d733e447feabf0a0a6158699da6a2bf`.
  Keep Unity/GPU
  Art-Net, Spout, camera, DJ, hardware, audible, and dedicated Show-ASIO
  acceptance explicitly pending until observed and recorded against their
  acceptance documents.
- Historical alpha.38 first safe resume action was completed: the saved alpha9
  reference was opened, Click/Guide-enabled Play -> Pause -> Play -> Pause
  tracked Timeline state without a CUE fault, and the authoring monitor stayed
  `実行中`, `rev1`. This is historical alpha.38 native UI/runtime evidence
  only, not alpha.39 native evidence; audible confirmation remains open.
  After Pause, the authoring-output `<select>` drifted to `3 - PX160 WAV...`
  while settings JSON and resolved output remained Music. Keep this only as the
  historical alpha.38 fail-closed record; do not treat the alpha.38 process or
  this mismatch as current alpha.39 acceptance. Alpha.39 selector/UI
  reverification is recorded above; the next safe action is operator audible
  confirmation. Do not reuse or relabel historical alpha.31/
  alpha.32/alpha.35/alpha.36/alpha.37/alpha.38 artifacts.
- Preserve the operator-owned DVC, all token material outside the checkout, and existing QA artifacts.
