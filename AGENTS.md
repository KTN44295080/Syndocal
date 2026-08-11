# KDMX development completion gate

- Changes that affect the native UI or runtime are not complete after a frontend-only build.
- Immediately before every native release build, find any running process whose resolved executable path is exactly this checkout's `target/release/syndocal.exe`, verify that exact path, and force-terminate only that process. Do this proactively so the linker can replace the executable; do not wait for an access-denied build failure. Never terminate Daslight or an unrelated `syndocal.exe` from another checkout.
- Before handing off such changes, run `pnpm --dir app tauri build --no-bundle` successfully.
- Launch `target/release/syndocal.exe` after the build and verify that exactly one responsive `Syndocal` window is available.
- Before using UI automation or manual QA actions on Syndocal, verify the intended Syndocal window and maximize it. Perform Syndocal UI operations only while that verified target window is maximized, except when a test explicitly covers restore/minimize behavior.
- Do not claim native verification when only browser harnesses, TypeScript, Vite, or Rust unit tests were run.

## UI sizing obligation

- Do not treat making UI elements smaller as a default improvement or routine density fix.
- Preserve existing typography, controls, icons, spacing, and hit-target sizes unless the user explicitly requests a smaller size for a named element.
- Solve space pressure with reflow, disclosure, pagination, or internal scrolling before considering any size reduction.
- Any explicitly requested size reduction must remain local to the named element; do not use it as permission to shrink adjacent or shared UI.

## Delegation and concurrency

- When delegating implementation, prefer Codex CLI `gpt-5.6-luna` with `model_reasoning_effort="max"` when it is available. For difficult or heavy implementation work, `gpt-5.6-terra` and then `gpt-5.6-sol` are also permitted.
- Separate implementation from adversarial review whenever concurrency permits. The reviewer must independently inspect failure modes, regressions, and proof strength instead of merely confirming the implementer's summary.
- Do not idle while delegated work is running. Advance independent read-only investigation, test planning, documentation checks, release evidence, or non-overlapping implementation in parallel.
- Assign explicit file ownership before concurrent edits. Never let agents edit the same files concurrently; keep review read-only until the implementation owner reports a stable checkpoint.
- The supervising agent remains responsible for integration, required native verification, and the final completion claim even when implementation or review is delegated.
