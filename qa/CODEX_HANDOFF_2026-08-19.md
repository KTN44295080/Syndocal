# Syndocal Codex handoff — 2026-08-19

## Current checkpoint

- Branch: `codex/syndocal-v1.0`
- Baseline HEAD at takeover: `df7e335c14fe82bb534fbd8867dcd431777e1522`
- The previous Codex reached its context/token limit while continuing AI3. Treat Timeline Transport and the canonical Timeline Follow Abort tranche as verified. AI3 has now been audited and remains incomplete in all five roadmap categories. The detailed audit matrix and ordered gaps are recorded in `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`.
- The working tree was clean before this handoff-document update. Do not assume that code present in the baseline commit is complete merely because it is committed.

## Take Over repair checkpoint

The source-level Take Over blocker from the takeover baseline is repaired in the
current checkpoint:

- project-load/replacement internals now accept `&AppState`, so both Tauri
  `State<'_, AppState>` callers and the authenticated runtime use the same
  boundary without weakening lifecycle or output-ownership ordering;
- control-plane Take Over carries the consent-bound `{session_id, generation}`
  into the core, rechecks status after taking the lifecycle lock, and strictly
  verifies the exact manifest before stopping the Standby worker;
- a valid, invalid-JSON, corrupt-project, or noncanonical newer manifest cannot
  make Exact mode apply an older checkpoint; the legacy local command alone
  retains explicit `LocalLatest` fallback behavior;
- stale or invalid Exact selection returns before worker stop, project
  publication, or output transition.

The normal default-feature `cargo check -p syndocal --locked` now succeeds in
the configured MSVC/FFmpeg/libclang environment. Focused proof passes the strict
filesystem reader, four Take Over tests, external replacement join ordering, and
polling-worker replacement scope. This closes the compile/generation-binding
repair only. It does **not** close Take Over's larger project-swap physical-output
retirement/re-arm fence or AI3 as a whole.

Independent adversarial review found no remaining P0/P1 in this repair. Residual
P2 proof hardening remains: add explicit empty/`+011`/alphabetic manifest-tail
cases, a deterministic core test proving stale Exact rejection leaves the worker
running, and an execute-to-core integration seam beyond the current selector and
filesystem tests.

## AI3 audit result

The five-part AI3 definition has now been audited against committed baseline
`896fb407f896edd46fe938a07a8bd5c71cfc956d`; the detailed matrix is recorded in
`qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`. Every category remains partial:
runtime generations, output ownership, rate limits, Blackout/Arm/Take Over
safety, and physical-resource idempotency.

The most immediate P0 is the disconnected R4 output vertical. Release Blackout,
Arm, and Take Over exist as backend OutputControl WIP but are not canonical
registry operations and the GUI still invokes legacy direct commands. The next
safe slice should migrate those three local operations together through one
local-only, physically confirmed R4 controller and fail closed any remaining
energizing bypass. It must not add external principals, grants, sidecar access,
or claim the AI4 consent service complete. Output lease/orphan/restart semantics,
durable physical receipts/audit, full project-swap retirement fencing, and
10,000-call realtime-budget proof remain later AI3 work.

## Verified takeover environment

The Windows development environment was installed and locally verified:

- Node.js `22.22.1`
- pnpm `10.9.0`
- Rust/Cargo `1.97.1`
- Visual Studio 2022 Build Tools `17.14` with MSVC and Windows SDK `10.0.26100`
- LLVM/Clang and libclang `19.1.5`
- shared FFmpeg `n8.1.2-44-g7c533d0f86-20260818`

User environment variables now contain the Node, Cargo, and FFmpeg binary directories plus `FFMPEG_DIR` and `LIBCLANG_PATH`. A newly started terminal/Codex process is required to inherit them.

The following commands and gates passed during environment takeover and the Take Over repair. The absolute tool paths were used because the already-running Codex process had not inherited the newly registered user environment yet:

- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app install --frozen-lockfile`
- `set PATH=C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64;%PATH%&& C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app build`
- `cargo fmt --all -- --check`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-release-metadata.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-backend-operator-contract.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-timeline-follow-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-timeline-transport-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-safety-blackout-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app run check:output-ownership`
- `cargo test -p syndocal --locked exact_standby_reader_rejects_invalid_newer_candidates_without_local_fallback` — 1 passed
- `cargo test -p syndocal --locked takeover` — 4 passed
- `cargo test -p syndocal --locked external_project_replacement_joins_standby_worker_before_fenced_swap` — 1 passed
- `cargo test -p syndocal --locked standby_polling_replacement_never_attempts_lifecycle_stop` — 1 passed

The exact native check invocation was:

```cmd
call C:\Progra~2\MICROS~2\2022\BuildTools\Common7\Tools\VsDevCmd.bat -arch=amd64&& set FFMPEG_DIR=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1&& set LIBCLANG_PATH=C:\Progra~2\MICROS~2\2022\BuildTools\VC\Tools\Llvm\x64\bin&& set PATH=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin;C:\Users\janua\.cargo\bin;%PATH%&& C:\Users\janua\.cargo\bin\cargo.exe check -p syndocal --locked
```

The takeover-baseline invocation originally exposed the committed type mismatch.
After the repair, the same default-feature command completed successfully with
warnings only.

The repository-native completion gate for this repair checkpoint also passed:

```powershell
$target = [IO.Path]::GetFullPath((Join-Path (Get-Location) 'target\release\syndocal.exe'))
Get-CimInstance Win32_Process -Filter "Name='syndocal.exe'" |
  Where-Object { $_.ExecutablePath -and [StringComparer]::OrdinalIgnoreCase.Equals([IO.Path]::GetFullPath($_.ExecutablePath), $target) } |
  ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
```

The exact-checkout process count was `0` immediately before the build. The exact,
reproducible build invocation was:

```cmd
call C:\Progra~2\MICROS~2\2022\BuildTools\Common7\Tools\VsDevCmd.bat -arch=amd64&& set FFMPEG_DIR=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1&& set LIBCLANG_PATH=C:\Progra~2\MICROS~2\2022\BuildTools\VC\Tools\Llvm\x64\bin&& set PATH=C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64;C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin;C:\Users\janua\.cargo\bin;%PATH%&& C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app tauri build --no-bundle
```

It completed successfully and produced
`target/release/syndocal.exe`. That exact executable was launched and inspected
through Win32/process metadata at verification time: PID `20540`, executable path equal to this
checkout, title `Syndocal`, `Responding=True`, exactly one exact process, exactly
one responsive Syndocal window, and `IsZoomed=True` after maximizing. A later
exact-path reinspection still found PID `20540` responsive; this PID is historical
checkpoint evidence rather than a promise that the process will remain alive.
Before the next release build, inspect again and stop only that exact path as
required by `AGENTS.md`.

## Required continuation order

1. Preserve the completed Take Over type/generation-binding repair and its strict filesystem regressions.
2. Migrate local Release Blackout, Arm, and Take Over together through one canonical local-only OutputControl R4 vertical. Keep S0 engage on its priority lane, remove/fail-close legacy energizing GUI routes, and do not introduce AI4 external grants or principals.
3. Implement the missing AI3 output lease state machine: owner incarnation, resource set, monotonic generation, TTL expiry with unchanged physical state, orphaned state, stale-owner rejection, idempotent retry, restart non-reclamation, and a fail-closed forced-transfer ownership/state transition. AI4 supplies the human presence, authorization, grants, and consent for that transition; it does not own the underlying AI3 transfer state machine.
4. Fence Take Over/new/load/recovery through acknowledged physical retirement, project replacement, and explicit re-arm; then add durable physical receipt/audit and crash/reply-loss/saturation proof.
5. Close AI3 with the repository-native completion gate: exact-checkout process stop, `pnpm --dir app tauri build --no-bundle`, exact executable launch, exactly one responsive Syndocal window, and maximized-window QA.
6. Only then proceed to AI4.

## Persistent collaboration and checkpoint rules

- Use a dedicated implementation agent and a separate read-only/adversarial reviewer for material implementation work.
- Default implementation delegation is `gpt-5.6-luna` with maximum reasoning. Escalate difficult work to `gpt-5.6-terra` high/xhigh, then `gpt-5.6-sol` when needed. The supervising Sol agent owns integration and the final claim.
- While an agent or build is running, advance non-overlapping investigation, test planning, documentation, or review work; do not idle.
- At each meaningful verified checkpoint, update the roadmap/status/handoff documentation, commit with a descriptive message, and push the active branch. Never leave the only usable handoff in chat history.

## ASIO product requirement

Windows ASIO support is an explicit product implementation and release requirement, not an optional undocumented experiment. Existing bridge, smoke, 100-cycle, and native-UI evidence must be preserved, but they do not close the requirement by themselves. Completion must also resolve the distribution/license boundary and the open acceptance items recorded in `qa/ASIO_INPUT_ACCEPTANCE.md`, including supported-driver breadth, device loss/recovery, sustained low-latency operation, observable actual buffer/XRUN behavior, and native end-to-end QA. Unsupported or failed ASIO selection must remain fail-closed and must not silently fall back to another driver or WASAPI.
