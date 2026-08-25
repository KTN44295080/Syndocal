# KDMX development completion gate

- Changes that affect the native UI or runtime are not complete after a frontend-only build.
- Immediately before every native release build, find any running process whose resolved executable path is exactly this checkout's `target/release/syndocal.exe`, verify that exact path, and force-terminate only that process. Do this proactively so the linker can replace the executable; do not wait for an access-denied build failure. Never terminate Daslight or an unrelated `syndocal.exe` from another checkout.
- Before every Windows Cargo/Tauri native build or test, initialize
  `vcvars64.bat -vcvars_ver=14.44`, require the resolved linker to be exactly
  `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`, and pin that same absolute path in
  `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`. Print and verify both the pinned
  variable and `where.exe link.exe` before Cargo starts; fail closed if the exact
  linker is missing or not first, and never allow Cargo to fall through to Git
  for Windows' incompatible `usr\bin\link.exe`. If a command launched the Git
  linker, that entire gate attempt is invalid and must be rerun from the start.
  The only edition-root exception is the official GitHub-hosted `windows-2022`
  image, which installs Visual Studio 2022 Enterprise instead of Community. In
  that marked CI context, require the exact corresponding Enterprise
  `...\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`; do not relax the toolset,
  architecture, absolute Cargo pin, or `where.exe`-first requirements. Never
  apply this hosted-runner exception to a local or self-hosted invocation.
  When a `cmd.exe` command mutates `PATH` after `vcvars64.bat`, use delayed
  expansion (`cmd /v:on` and `!PATH!`); `%PATH%` is expanded before `vcvars64`
  executes and can silently discard the MSVC additions. Keep this invariant
  enforced by the Tauri build wrapper and its focused checker, not only by
  operator memory.
- Before handing off such changes, run `pnpm --dir app tauri build --no-bundle` successfully.
- Launch `target/release/syndocal.exe` after the build and verify that exactly one responsive `Syndocal` window is available.
- Before using UI automation or manual QA actions on Syndocal, verify the intended Syndocal window and maximize it. Perform Syndocal UI operations only while that verified target window is maximized, except when a test explicitly covers restore/minimize behavior.
- Do not claim native verification when only browser harnesses, TypeScript, Vite, or Rust unit tests were run.

## UI sizing obligation

- Do not treat making UI elements smaller as a default improvement or routine density fix.
- Preserve existing typography, controls, icons, spacing, and hit-target sizes unless the user explicitly requests a smaller size for a named element.
- Solve space pressure with reflow, disclosure, pagination, or internal scrolling before considering any size reduction.
- Any explicitly requested size reduction must remain local to the named element; do not use it as permission to shrink adjacent or shared UI.

## Checkpoints, documentation, and handoff

- At every periodic or meaningful progress checkpoint, update the relevant roadmap/release/QA documents and leave a concise handoff containing the current branch/HEAD, verified evidence, remaining blockers, and next action. Do not leave the operational state only in chat.
- After the checkpoint validation passes, create a meaningful commit and push it. Record exact commands and any unverified external or hardware acceptance in the handoff; never mark an item complete from an unverified result. If a push fails, preserve the commit and report the push failure and recovery action.
- Treat stale generated artifacts as an ongoing cleanup obligation. At every checkpoint and before the next versioned native build, inventory workspace size and delete verified-regenerable obsolete build/cache trees after the owning checkpoint is committed and pushed. A cleanup harness is eligible for execution only when it is tracked at that checkpoint, all of its focused safety tests pass, and an independent adversarial review has accepted its exact recurring target set; otherwise perform no harness-driven deletion and record the blocked cleanup as remaining work. Any added deletion target requires a new exact-path safety review. Resolve and verify every absolute deletion target, preserve current release/QA/evidence artifacts and user-authored files, and record the paths and reclaimed bytes in the handoff. After removing a worktree, package store, `node_modules`, or cache that may share links with the active checkout, immediately verify the active checkout's dependency graph and rebuild the affected surface from its frozen lockfile before claiming the cleanup safe; record any restoration separately from reclaimed bytes.

## Bounded completion and pause contracts

- When the user defines an exact checkpoint to finish and then stop, freeze every out-of-scope implementation lane at a stable, attributable state and do not advance it while completing that checkpoint.
- Before stopping, create one explicitly authoritative resume handoff. Record the final branch, HEAD, upstream equality, owned dirty files in every companion worktree, stashes, protected untracked evidence, relevant running processes and artifact identities, exact validation commands and warning counts, all known remaining work, the first safe resume action for each item, and every native, hardware, external, or coverage claim that remains unverified.
- Commit and push the completed checkpoint and its documentation according to the checkpoint rules above, terminate delegated or automatic continuation work, and verify that no next-version implementation has started. Once the requested stop boundary is reached, stop; do not continue into the remaining roadmap merely because capacity is available.

## Windows ASIO product gate

- Windows ASIO support is an explicit product and release requirement, not an optional runtime preference. Keep the default MIT/WASAPI path and the separately licensed, non-default ASIO bridge distinct.
- ASIO completion requires explicit device enumeration and selection, sample-rate/native-format/channel/buffer negotiation, low-latency callback I/O, exclusive ownership and open/start/stop/free error handling, fail-closed disconnect/XRUN/no-callback recovery, project persistence and stale-selection locking, plus deterministic tests and real-device QA. The authoritative gate and current status are in `qa/ASIO_INPUT_ACCEPTANCE.md`; do not claim ASIO release completion while its unchecked gates remain.

## Product version discipline

- Do not leave product artifacts on one version indefinitely. Follow the active release-train policy in `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`: distributed development artifacts advance a SemVer prerelease ordinal, release candidates advance their RC ordinal after fixes, and published tags/artifacts are immutable.
- Keep workspace Cargo, frontend package, Tauri, release-check script, artifact names, updater metadata, and current user-facing version text synchronized. Run `pnpm --dir app run check:release` after a product-version change and record it in the handoff.
- Product SemVer is independent from project/template/cache/control-plane/API/ABI schema versions. Change a schema version only for its own compatibility boundary with migration and future-version rejection proof; never use a product bump to hide a wire/schema incompatibility.

## Warning ratchet

- Every checkpoint must report first-party warning counts for the configurations it ran. No checkpoint may add a first-party warning, and a warning in a modified file blocks that tranche.
- Remove existing warnings in focused commits and reach zero first-party warnings before beta/release-candidate acceptance. Do not suppress debt with crate-wide `allow(dead_code)`, rustc command-line warning-allow flags, fake reads, or an arbitrary Vite chunk-limit increase.
- A narrow third-party/platform allowlist requires a reason, owner, upstream reference where applicable, and expiry. Once a supported matrix reaches zero, make warnings errors in CI and do not regress it.

## Fail-closed and clean-break obligation

- Treat compatibility as a deliberate, evidenced product decision, never as the automatic default. Do not preserve an obsolete behavior, schema, route, state, or adapter merely because something might still depend on it.
- Fail closed on invalid, ambiguous, stale, unsupported, underdetermined, or unverifiable state. The failure must be visible, specific, and actionable; never silently accept, promote, normalize, synthesize, or substitute a more permissive result.
- Do not stack fallback paths, legacy branches, retries, or compatibility shims to conceal an unresolved invariant or ownership failure. Diagnose and repair the authoritative path. A safety fallback is allowed only when its bounded trigger, owner, telemetry, tests, and removal condition are explicit.
- When a clean break is chosen, remove the retired design completely in the same tranche: entry points, persisted state, schemas, adapters, UI, runtime branches, tests, documentation, generated inventories, and obsolete artifacts. Do not leave a dormant second path that can accidentally reactivate.
- Add a migration only when preserved user data or an external protocol explicitly requires one. Keep it one-way, bounded, observable, and separately versioned; reject invalid and future versions, and record the removal milestone instead of supporting it indefinitely.
- Completion proof must cover both sides of the boundary: the intended path succeeds, and invalid or retired paths are unreachable or fail closed. At each checkpoint, document the old path, new path, reason for the break, deliberately unsupported cases, and any temporary migration with its deletion gate.

## Delegation and concurrency

- Keep task decomposition, instruction design, integration decisions, and completion claims under the supervising `gpt-5.6-sol` agent. Delegated summaries are evidence inputs, not substitutes for the supervisor's diff and gate verification.
- Use the capability hierarchy `gpt-5.6-sol` > Zen/Ox `opencode/x-preview-f-free` > `gpt-5.6-terra` > `gpt-5.6-luna`. Make Ox the default delegated lane for almost all bounded implementation, investigation, and independent adversarial review whenever it is callable and suitable.
- Use `gpt-5.6-terra` at high/xhigh effort as a secondary difficult implementation or review lane when additional independent capacity is useful. Any implementation produced by Terra must receive an independent Ox adversarial review before integration.
- Use `gpt-5.6-luna` at max effort only for small, explicit, low-ambiguity parallel tasks, including work that the Sol supervisor has first decomposed into a narrowly specified implementation assignment. Do not assign broad design, ambiguous root-cause work, or final review to Luna merely because a lane is free.
- For every material implementation task with available agent capacity, actively delegate implementation and an independent adversarial review as separate tasks. Default both assignments to independent Ox sessions first, and keep the implementer and reviewer independent.
- If Ox-alpha is unavailable or unsuitable, any integration that requires Ox review blocks unless the supervising Sol records a narrow, reasoned exception and assigns an independent Terra review at high/xhigh effort; self-review is forbidden.
- The reviewer must independently inspect failure modes, regressions, and proof strength instead of merely confirming the implementer's summary. Assign explicit file ownership before concurrent edits and keep review read-only until the implementation owner reports a stable checkpoint.
- Do not idle while delegated work is running. Advance independent read-only investigation, test planning, documentation checks, release evidence, or non-overlapping implementation in parallel.
- Work is parallel by default, and eliminating avoidable elapsed time is an explicit operating obligation: continuously fill every safely independent available lane with a capability-appropriate assignment under the hierarchy above instead of leaving capacity unused or waiting serially. Whenever the supervising lane is blocked on a build, cleanup, native QA, external I/O, or another agent, assign a concrete non-overlapping implementation, adversarial review, evidence audit, version task, or hardware preflight. Leave capacity idle only when no safe productive task exists, and record that reason at the next checkpoint. Serialize only true dependencies, exclusive UI/native steps, destructive actions, or same-file ownership.
- Never let agents edit the same files concurrently; keep review read-only until the implementation owner reports a stable checkpoint.
- The supervising agent remains responsible for integration, required native verification, and the final completion claim even when implementation or review is delegated.
