# Syndocal Codex handoff — 2026-08-19

## Current checkpoint

- Branch: `codex/syndocal-v1.0`
- Baseline HEAD at takeover: `df7e335c14fe82bb534fbd8867dcd431777e1522`
- The previous Codex reached its context/token limit while continuing AI3. Treat Timeline Transport and the canonical Timeline Follow Abort tranche as verified. AI3 as a whole remains unreviewed and incomplete. Safety Blackout, Arm/Takeover, and output lease are known WIP examples, not an exhaustive remaining-work list; runtime generations, output ownership, rate limits, and physical-resource idempotency also require audit against the authoritative roadmap.
- The working tree was clean before this handoff-document update. Do not assume that code present in the baseline commit is complete merely because it is committed.

## Immediate blocker

`cargo check -p syndocal --locked` reaches the application crate and currently fails in `app/src-tauri/src/main.rs`:

- `take_over_standby_core` accepts `&AppState`.
- It passes that value to `load_project_from_file_with_control_mappings_in_scope`, which requires `&State<'_, AppState>`.
- The reported call is near line 37838 and the callee signature is near line 38833 in the takeover baseline.

This is a source-level type-boundary regression in the current HEAD, not an environment, FFmpeg, bindgen, or MSVC failure. Repair and test this boundary before making any native completion claim.

## Verified takeover environment

The Windows development environment was installed and locally verified:

- Node.js `22.22.1`
- pnpm `10.9.0`
- Rust/Cargo `1.97.1`
- Visual Studio 2022 Build Tools `17.14` with MSVC and Windows SDK `10.0.26100`
- LLVM/Clang and libclang `19.1.5`
- shared FFmpeg `n8.1.2-44-g7c533d0f86-20260818`

User environment variables now contain the Node, Cargo, and FFmpeg binary directories plus `FFMPEG_DIR` and `LIBCLANG_PATH`. A newly started terminal/Codex process is required to inherit them.

The following commands and gates passed during takeover. The absolute tool paths were used because the already-running Codex process had not inherited the newly registered user environment yet:

- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app install --frozen-lockfile`
- `set PATH=C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64;%PATH%&& C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app build`
- `cargo fmt --all -- --check`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-release-metadata.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-backend-operator-contract.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-timeline-follow-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-timeline-transport-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-safety-blackout-runtime.mjs`

The exact native check invocation was:

```cmd
call C:\Progra~2\MICROS~2\2022\BuildTools\Common7\Tools\VsDevCmd.bat -arch=amd64&& set FFMPEG_DIR=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1&& set LIBCLANG_PATH=C:\Progra~2\MICROS~2\2022\BuildTools\VC\Tools\Llvm\x64\bin&& set PATH=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin;C:\Users\janua\.cargo\bin;%PATH%&& C:\Users\janua\.cargo\bin\cargo.exe check -p syndocal --locked
```

It compiled through MSVC/SDK, bindgen, FFmpeg and the workspace dependencies before failing at the committed Take Over source mismatch described above.

No native release-build or launch success is claimed for this machine yet because the current Rust source does not compile.

## Required continuation order

1. Repair the Take Over state-type boundary without weakening the lifecycle/output-ownership fence.
2. Re-run focused Take Over/standby and project-replacement tests, then `cargo check -p syndocal --locked` and the relevant full suites.
3. Audit the entire AI3 definition against `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`: runtime generations, output ownership, rate limits, Blackout/Arm/Takeover safety, output-activation lease behavior, and physical-resource idempotency. Confirm that the runtime/output bridge remains fail-closed without the later grant service. Scoped grants, prepared human consent, revocation, and the consent service itself belong to AI4 and must not be pulled into AI3. The known WIP list is not exhaustive; do not mark AI3 complete from static presence alone.
4. Close AI3 with the repository-native completion gate: exact-checkout process stop, `pnpm --dir app tauri build --no-bundle`, exact executable launch, exactly one responsive Syndocal window, and maximized-window QA.
5. Only then proceed to AI4.

## Persistent collaboration and checkpoint rules

- Use a dedicated implementation agent and a separate read-only/adversarial reviewer for material implementation work.
- Default implementation delegation is `gpt-5.6-luna` with maximum reasoning. Escalate difficult work to `gpt-5.6-terra` high/xhigh, then `gpt-5.6-sol` when needed. The supervising Sol agent owns integration and the final claim.
- While an agent or build is running, advance non-overlapping investigation, test planning, documentation, or review work; do not idle.
- At each meaningful verified checkpoint, update the roadmap/status/handoff documentation, commit with a descriptive message, and push the active branch. Never leave the only usable handoff in chat history.

## ASIO product requirement

Windows ASIO support is an explicit product implementation and release requirement, not an optional undocumented experiment. Existing bridge, smoke, 100-cycle, and native-UI evidence must be preserved, but they do not close the requirement by themselves. Completion must also resolve the distribution/license boundary and the open acceptance items recorded in `qa/ASIO_INPUT_ACCEPTANCE.md`, including supported-driver breadth, device loss/recovery, sustained low-latency operation, observable actual buffer/XRUN behavior, and native end-to-end QA. Unsupported or failed ASIO selection must remain fail-closed and must not silently fall back to another driver or WASAPI.
