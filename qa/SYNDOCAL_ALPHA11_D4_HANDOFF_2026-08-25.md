# Syndocal alpha.11 D4 Stage transaction handoff

Status: **IN PROGRESS — integrated/focused acceptance complete; native acceptance pending**

This document records the D4 integration checkpoint. The source/focused gates are
accepted; it does not authorize an alpha.11 native or release claim until the
remaining native rows are filled from direct evidence.

## Integration state at handoff creation

- Integration worktree: `C:\Users\kouty\Documents\KDMX-d4-integration`
- Branch: `codex/d4-stage-integration`
- Current pushed HEAD: `f45a7606f7ad07771d3189f889808cbe7556aa95`
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
- [ ] Run the required native release build with the exact VS2022 x64
  `link.exe` pinned; verify the resolved linker path before Cargo/Tauri.
  - Exact build command(s): **PENDING**
  - Linker path evidence: **PENDING**
  - Build result/hash/warnings: **PENDING**
- [ ] Launch the built `target/release/syndocal.exe`, verify exactly one
  responsive Syndocal window, maximize the verified window, and perform the
  manual Stage transaction QA sequence.
  - Launch/window evidence: **PENDING**
  - Manual Stage QA sequence/result: **PENDING**
- [ ] Update the authoritative roadmap/release/QA handoff with native evidence, then commit and
  push the accepted checkpoint.
  - Focused source/version checkpoint `f45a760` is pushed at upstream `0/0`;
    final native-evidence documentation commit remains pending.

## Explicit non-claims and remaining boundaries

- D4 source/focused integration is accepted; native acceptance is not, and
  alpha.11 is not released.
- No native build, native launch, responsive-window check, maximized-window
  manual QA, or native warning-ratchet result is claimed until the pending
  evidence above is recorded.
- This handoff does not claim Windows ASIO product completion, real-device
  enumeration/negotiation, capture or output soak, or physical hardware QA.
- Five displays were enumerated read-only and `DISPLAY3` is a real active
  3840x2160/240 Hz monitor, but no Syndocal 4K UI acceptance is claimed yet.
  DJ-Link live-network acceptance and other external/hardware results remain open.
- AI3 durable terminal recovery, replacement-output retirement/re-Arm, and the
  remaining post-D4 roadmap are outside this D4 pre-acceptance boundary.

## Next safe action

Run the required exact-linker native release build, launch only this worktree's
executable, verify exactly one responsive maximized Syndocal window, and execute
the Stage transaction QA on the real 3840x2160 `DISPLAY3`. Then record executable
version/hash, warning ratchet, window/geometry evidence, manual results and
remaining non-claims before the final documentation commit/push.
