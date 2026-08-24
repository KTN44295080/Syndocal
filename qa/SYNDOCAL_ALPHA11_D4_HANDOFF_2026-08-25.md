# Syndocal alpha.11 D4 Stage transaction handoff

Status: **ACCEPTED — D4 software/native integration accepted on evidence-backed
native acceptance; whole-product, ASIO, and hardware claims remain open**

This document records the D4 integration checkpoint. The source/focused gates,
the exact-linker native release build, launch/window/display proof, and the
manual Stage transaction QA on the real 4K `DISPLAY3` are accepted from direct
evidence below. This does not claim whole-product completion, a beta/RC/tag or
release, Windows ASIO completion, or any physical-hardware result.

## Integration state at handoff creation

- Integration worktree: `C:\Users\kouty\Documents\KDMX-d4-integration`
- Branch: `codex/d4-stage-integration`
- Committed checkpoint HEAD at native acceptance:
  `63cf795d17846602419d63a007db9f3a95cfce7b`, recorded before this handoff
  update and before the concurrent uncommitted FFmpeg script/doc changes
  (`app/scripts/check-native-window-acceptance.ps1`,
  `app/scripts/check-native-workspace-operator.ps1`,
  `qa/NATIVE_WINDOW_ACCEPTANCE.md`) which this documentation-only update
  preserves untouched.
- Upstream: `origin/codex/d4-stage-integration`; divergence `0/0`
- Base alpha.10 commit: `c9ca896a0da78b69471b2d352ac549081736f987`
- Engine commits already integrated on this branch:
  - `2ea64ff3cc0fd36e0ce5bf6f99196f876bf1b483` — acknowledge atomic stage
    project mutations
  - `887505f7fa63a57aba798eb39d7bf4319a3d25e9` — update command inventory
    tripwire
  - `7804620f3faf000ff3eb990eb9ede55ed8934dcc` — reserve stage IDs from
    applied presets
- D4 source commits:
  - backend `d6ac4c8b696ffc7090a0b5ae7d1de79cc8e43cc9`
  - frontend `eb74686c9d72871adf5310788c13c576aaa335ea`
  - checker `854b518175f193b4b75d1f9b1f17e364f144de66`
- Source branch/HEAD: `codex/d4-stage-transaction` /
  `854b518175f193b4b75d1f9b1f17e364f144de66`
- Integration results: backend `0aa0b14fbf24701dfd93711372a9ff16fa78c09f`,
  frontend `6bcd879e39aa2c5270da7840f3c6ba265468d05c`, checker
  `d44f2b113c1a0c833ba622d6521a06c9a20ab34b`

## D4 acceptance scope

The implementation and its evidence must cover all of the following before D4
is marked complete:

- nine Stage renderer-ticketed routes use the receipt-backed transaction path;
- `E`/`R`/`H` stage project mutations validate the checkpoint hash both before
  and after reconciliation;
- retained receipts remain authoritative across the expected post-operation
  checks;
- legacy normal Stage replies remain compatible (`null`/number/string), while
  transactional replies validate their receipt contract;
- Stage Undo/Redo use the same acknowledged transaction semantics and are
  covered by deterministic tests;
- rejected or mismatched checkpoints fail closed without publishing a false
  success.

## Required completion checklist

- [x] D4 source implementation reaches a stable checkpoint.
  - Source HEAD: `854b518175f193b4b75d1f9b1f17e364f144de66`
  - Ox source review and Terra integrated adversarial review: code P0/P1 zero.
- [x] Cherry-pick the D4 source commits onto this integration branch.
  - Commands: `git cherry-pick d6ac4c8b696ffc7090a0b5ae7d1de79cc8e43cc9`,
    `git cherry-pick eb74686c9d72871adf5310788c13c576aaa335ea`, and
    `git cherry-pick 854b518175f193b4b75d1f9b1f17e364f144de66`.
  - Resulting commits: `0aa0b14`, `6bcd879`, `d44f2b1`.
- [x] Inspect and resolve cherry-pick conflicts without reopening the
  alpha.10 pane lifecycle or terminal-recovery contracts.
  - Backend receipt inventory retained alpha.10
    `FULL_LOCK_CORRELATED_TERMINAL_RECOVERY_RUNTIME_ROUTES` while adopting the
    D4 receipt-backed route list. The checker retained alpha.10 `createHash`,
    frozen count/SHA enforcement, and line-ending-independent `readText` while
    adding D4 assertions.
- [x] Re-verify and preserve the alpha.10 frozen command/runtime contracts
  before calling the integrated D4 checkpoint accepted:
  - registered command inventory remains `480`;
  - runtime command inventory remains `143`;
  - the frozen inventory fingerprint remains unchanged — exact fingerprint
    and checker evidence:
    `bea9db6c8cc249bc3f2bc55aaab6b20680bc6e91af882719da8a9f29731ecd44`;
  - `FULL_LOCK_CORRELATED_TERMINAL_RECOVERY_RUNTIME_ROUTES` continues to
    protect the `cancel_pane_window_close` route;
  - the App pane terminal-recovery/operator-lock exception remains intact;
  - the backend checker retains `createHash` and the frozen-inventory
    assertion rather than weakening or replacing them.
  - `pnpm --dir app run check:backend-operator-contract`: exit 0,
    480 registered / 310 literal frontend / 133 transactional; first-party
    warning lines 0.
- [x] Run the focused D4 frontend and backend gates in the prescribed order.
  - `cargo test -p engine stage_transaction_d4_ -- --nocapture`: 6/6;
    preset-ID reservation: 1/1.
  - `cargo test -p syndocal d4_stage_ -- --nocapture`: 7/7;
    `pane_window_`: 4/4; CRLF-sensitive static boundaries: 4/4.
  - `pnpm --dir app run build`: 272 modules; D2/backend/routing/invoke/project
    transaction/Tauri-wrapper checks all exit 0 at 480/133/419/26 as applicable.
  - All Windows Cargo commands used VS2022 14.44 Developer Shell and
    `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`;
    first-party warning lines 0.
- [x] Obtain independent Ox adversarial review of the stable integrated
  diff, including failure paths and proof strength.
  - Ox source review: P0/P1 zero; Terra integrated review found and rechecked
    the CRLF/source-order proof fixes, then returned code P0/P1 zero.
- [x] Advance synchronized product artifacts from `1.2.0-alpha.10` to
  `1.2.0-alpha.11` only after D4 acceptance.
  - Version/check-hardening commit: `f45a7606f7ad07771d3189f889808cbe7556aa95`.
  - `pnpm --dir app run check:release`: exit 0; release self-test: 102 groups.
- [x] Run the required native release build with the exact VS2022 x64
  `link.exe` pinned; verify the resolved linker path before Cargo/Tauri.
  - Gate command: `pnpm --dir app run check:warnings --
    --configuration windows-native-release` passed and internally ran
    `pnpm --dir app tauri build --no-bundle` successfully.
  - Linker path evidence: resolved and pinned
    `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
    before Cargo/Tauri; file version 14.44.35225.0, SHA-256
    `1523A87532C2EB737DD7B7BCFC652CE5687A4F16048A45D1A5A0E8F6451AD49E`; no
    fall-through to Git for Windows' incompatible `usr\bin\link.exe`.
  - Environment: complete shared FFmpeg SDK root exposed as `FFMPEG_DIR` =
    `C:\Users\kouty\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-8.1.2-full_build-shared`.
  - Warning totals: current/baseline total 0/0; first-party 0; third-party 0.
  - Artifact: `C:\Users\kouty\Documents\KDMX-d4-integration\target\release\syndocal.exe`,
    57,888,768 bytes, SHA-256
    `1B010C40242A5C7DD7A2797EAC1ECA2D31BCACE4455BA57C7F935611075B582B`,
    ProductVersion/FileVersion `1.2.0-alpha.11`.
- [x] Launch the built `target/release/syndocal.exe`, verify exactly one
  responsive Syndocal window, maximize the verified window, and perform the
  manual Stage transaction QA sequence.
  - Launch/window evidence: launched the exact executable as PID `117672`;
    after integrated-state cleanup exactly one responsive top-level
    application window titled `Syndocal` remained; the intended window was
    verified maximized before UI operations.
  - Display geometry: real 4K `DISPLAY3` at 3840x2160 @ 240 Hz, Windows scale
    150%, display bounds `(-3840,-429)`, work area 3840x2088; app client
    physical 3840x2088 with CSS viewport 2560x1392.
  - Pane reintegration proof: with Timeline and Stage detached, the main
    Sources surface expanded through the available workspace with no blank
    lower reserved pane. Closing the detached Timeline reintegrated a single
    fully drawn Timeline while the detached Stage remained separate with
    Sources below it. Closing the detached Stage reintegrated Stage
    lower-left and Sources lower-right beneath Timeline, after which
    `list_windows` contained only one Syndocal window, no duplicate Timeline
    existed, and the integrated Stage was visibly drawn. This directly
    resolves the reported blank-space/duplicate-Timeline/
    missing-integrated-Stage concerns.
  - Manual Stage QA sequence/result on real native 4K:
    created a stage object labeled `D4 QA alpha11 20260825-0540`; it appeared
    on both the Stage map and panel. The Project menu showed
    `Undo Add Stage Object 1 step`; Undo removed it completely and enabled
    Redo; Redo restored the exact label and map object with status
    `Redid Add Stage Object`. Saved unique stage map preset
    `D4 QA alpha11 preset 20260825-0550` holding 1 stage object. Changed the
    X maximum 10.0 -> 10.1 through the native spin control; status
    `Updated 2D stage map` and the map re-rendered. Applying the preset
    restored the X maximum exactly to 10.0 and retained the object; status
    `Applied stage map preset ... (1 object)`. The QA preset was then removed;
    status `Removed stage map preset...`.
  - Boundary: the test-fixture transform A/B step was not performed because
    the launched Untitled project had zero patched fixtures. This remains an
    unverified fixture/hardware-specific boundary, not a D4 stage-object or
    preset blocker, and no hardware acceptance is claimed.
- [x] Update the authoritative roadmap/release/QA handoff with native evidence.
  - This handoff, `RELEASE_STATUS.md`, and section 37 of
    `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` carry this evidence.
- [x] Commit and push the accepted native-evidence documentation checkpoint.
  - Commit `9e8d0af42cc816e0b1bf878a6ef8354040227861`
    (`docs: accept alpha11 D4 native checkpoint`) is pushed to
    `origin/codex/d4-stage-integration` with upstream divergence `0/0` at that
    checkpoint.
- [x] Harden native FFmpeg SDK discovery and remove verified obsolete partial
  SDK copies.
  - Both native acceptance scripts now validate required headers, import
    libraries, version-matched runtime DLLs where `.def` metadata exists, and
    deterministic WinGet/fallback ordering. Explicit invalid `FFMPEG_DIR`
    fails closed. Ox and Terra reported no P0/P1 after follow-up review.
  - `C:\temp\ffmpeg-n8.1-lgpl-shared` and
    `C:\temp\syndocal-ffmpeg-sdk-20260813` were moved to the Recycle Bin:
    4 files / 806,696 bytes each, 1,613,392 bytes total, recoverable. The
    complete WinGet SDK remained intact at 217 files / 296,016,041 bytes.

## Explicit non-claims and remaining boundaries

- D4 software/native integration is accepted at alpha.11. Alpha.11 is not
  released; no beta/RC/tag, whole-product, or whole-roadmap completion is
  claimed.
- Windows ASIO product completion, real-device enumeration/negotiation, capture
  or output soak, and ASIO licensing/artifact separation remain open.
- The fixture transform A/B exercise was not performed because the launched
  Untitled project had zero patched fixtures; fixture/hardware-specific Stage
  behavior is an explicitly unverified boundary, not a D4 stage-object or
  preset blocker. No hardware acceptance of any kind is claimed.
- DJ-Link live-network acceptance, DMX/MIDI/OSC physical I/O, display-output
  playback, recording, soak, distribution/legal, and macOS/Linux rows remain
  open under the active completion flow.
- AI3 durable terminal recovery, replacement-output retirement/re-Arm, and the
  remaining post-D4 roadmap are outside this D4 acceptance boundary.

## 2026-08-25 updated DJ-Link peer audit (recorded at this handoff)

After the D4 checkpoint above, the separately developed DJ-Link peer at
`C:\Users\kouty\Desktop\rb-output` was re-audited read-only on 2026-08-25.
The checkout is clean `main`/`origin/main` at tag `v1.1.1`, commit `cdd90e1`.
The diff from the previously audited `616c897` contains only
package/package-lock/installer/README/Hook DLL source display-version changes, with
no server/dj-agent/syndocalClient.js/config/wire changes. The current flat
`/dj-link` generic-json contract and the optional `syndocal-envelope-v1`
contract remain statically compatible. Peer gates: `npm test` 69/69 and
envelope-focused 9/9 pass; `git diff --check` passes; the only observed
warning is the Node MockTimers ExperimentalWarning. `dist/server.exe` exists
(SHA-256
`C966CE8AFAC4A54A9A8C818D75A829A2063EA92D3F434C8C31F1C0A8404BE36B`) but the
peer build does not embed Git SHA/fingerprint, so it must NOT be claimed to
belong to `cdd90e1`. The identity-bound peer artifact and the physical
wired-LAN matrix remain open. This audit requires no Syndocal implementation
change; the D4 acceptance state and every explicit non-claim above are
preserved unchanged.

## Next safe action

Commit and push the reviewed FFmpeg acceptance hardening and this cleanup
record, then integrate the accepted alpha.11 branch into main. Continue with
the ASIO alpha12 authoring route repair/verification while keeping every
physical-device boundary (real fixtures, DMX, audio devices, DJ-Link network,
soak) explicitly open until its own acceptance runs.
