# KDMX / Syndocal repository guidance

## Scope and authority

- Follow the user's current task and explicit stop boundary. These rules do not authorize unrelated roadmap work, hardware output, publication, or cleanup.
- This file owns operating rules. For roadmap work, read the relevant section of [the completion flow](qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md) and its domain acceptance document. Small edits do not require a full roadmap, repository map, or historical handoff review.
- Dated checkpoints and saved delegation prompts describe their recorded tree; they are not current instructions or permission to resume work. Confirm live state when it matters to the task.
- Complete the authorized change, inspect the result, and fix failures caused by it through the applicable checks. Routine implementation choices, local edits, and affected-check reruns do not need repeated approval. Ask when consequential ambiguity would change requested behavior, data compatibility, or scope; not for every protocol edit or implementation-order adjustment.

## Ownership and delegation

- Inspect Git state before editing and preserve unrelated changes. Do not reset, stash, stage, or commit another owner's work. Stop an affected lane if unexpected concurrent edits overlap its ownership; resolve ownership before continuing.
- Do not use `gpt-5.6-terra` for delegated work, including implementation, investigation or review (user preference, 2026-09-05).
- The supervising agent owns decomposition, integration, and completion claims. Delegate when an independent task benefits from it, using available capabilities without a fixed model hierarchy, lane count, or idle-lane reporting.
- Assign exclusive file ownership before concurrent edits. Material runtime, schema, safety, or cross-cutting changes need independent review of the stable diff and evidence. Small local edits do not require separate implementation and review agents. Use another suitable independent reviewer when a particular model is unavailable.
- Delegated summaries are evidence inputs. Inspect the diff and relevant results; rerun checks when changes, failures, environment differences, or insufficient evidence justify it, instead of duplicating every successful run.

## Validation proportional to the change

| Change | Completion evidence |
| --- | --- |
| Documentation or instructions only | Diff, affected references and machine-consumed structure, relevant existing validator, and `git diff --check`. No product build, UI launch, version bump, hardware QA, or cache cleanup. |
| Display text only; no layout, interaction, or runtime change | Affected wording/localization and existing focused contract. Inspect rendering if wrapping or clipping is plausible. No native build solely for copy. |
| Local presentation/layout only | Focused checks and rendering at affected viewport sizes; preserve sizing and containment contracts. Broaden when shared layout is affected. Browser evidence does not establish native behavior. |
| Interaction, native UI integration, IPC, runtime, persistence, output, or native build/package configuration | Focused regressions plus the Windows native gate below when the Windows native surface is affected. Frontend-only build is not completion proof for this row. |
| Release acceptance or broad shared-layout changes | Applicable release/domain or viewport matrix. Do not run the full matrix for every local edit. |

- Confirm selected tests actually ran and inspect exit status. Fix regressions without weakening assertions to manufacture a pass; explain changed acceptance assertions when their contract changes.
- Stop broadening validation once applicable checks pass unless a new finding justifies more. Do not add tests that merely mirror a trivial edit.
- Local checks are autonomous within their actual side-effect boundary. Tests that open physical devices, energize DMX, control another app, or reach external services are not disposable local tests merely because they live here.

## Windows native gate

- Before Windows Cargo/Tauri native builds or tests, follow [the exact MSVC procedure](qa/WINDOWS_NATIVE_BUILD.md). Keep linker pinning and PATH-first checks enforced by `app/scripts/run-tauri.mjs` and its focused checker.
- For changes requiring native acceptance, run `pnpm --dir app tauri build --no-bundle`. Immediately before release builds, resolve and verify this checkout's `target/release/syndocal.exe`; force-terminate only processes with that exact executable path. Never terminate Daslight or another checkout's executable.
- After building, launch that executable and verify exactly one responsive `Syndocal` window. Verify the intended window and maximize it before Syndocal UI actions, except explicit restore/minimize tests.
- Browser, TypeScript, Vite, and Rust unit evidence is not native-window or hardware evidence. Do not claim unobserved device or physical output results.

## UI sizing

- Preserve typography, controls, icons, spacing, and hit targets unless the user requests a smaller size for a named element. Reductions stay local to that element.
- Solve space pressure through reflow, disclosure, pagination, or internal scrolling before size reduction.

## Checkpoints and stopping

- A checkpoint is a completed, agreed unit of work, not every progress message. For substantive changes, update the relevant status/QA document or one concise handoff with branch/base, changed behavior, evidence, unresolved boundaries, and next action. Do not copy the operational log into multiple documents.
- After applicable checkpoint validation passes, commit and push explicitly owned files unless the user requests otherwise. Inspect the staged diff, preserve unrelated dirty files, and verify upstream equality. If push fails, preserve the commit and report recovery. No hash-only follow-up documentation commit is required; record the base and let Git identify the checkpoint commit.
- Read-only audits and conversational answers require no repository writes or commits. For an explicit multi-lane pause, record owned dirty work, relevant worktrees/stashes, protected evidence, applicable artifact/process identities, and first safe resume actions in one authoritative handoff. Stop at the requested boundary and stop delegated continuation owned by the task.
- Cache cleanup is separate maintenance, not a checkpoint prerequisite. When in scope, follow [the cleanup safety boundary](qa/BUILD_CACHE_CLEANUP.md).

## Product and compatibility boundaries

- Windows ASIO remains a product/release requirement. Keep default MIT/WASAPI separate from the separately licensed, non-default ASIO bridge. [ASIO acceptance](qa/ASIO_INPUT_ACCEPTANCE.md) owns device, lifecycle, persistence, deterministic-test and real-device gates; unchecked required gates block ASIO completion claims, not unrelated tasks.
- Ordinary internal commits do not require a product bump. Intentionally distributed development artifacts advance the prerelease ordinal; RC fixes advance the RC ordinal; published tags/artifacts are immutable. Follow the completion flow's version policy for distributed artifacts.
- On product-version changes, synchronize Cargo, frontend, Tauri, release scripts, artifact names, updater metadata and current user-facing version; run `pnpm --dir app run check:release`. Product SemVer is independent of project/template/cache/control-plane/API/ABI schema versions.
- Treat compatibility as an evidenced decision. Fail closed on invalid, ambiguous, stale, unsupported or unverifiable product state with a specific actionable failure. Repair the authoritative path instead of hiding broken invariants behind legacy adapters, retries or permissive substitutes.
- When choosing a clean break, remove retired entry points, state/schema, adapters, runtime/UI paths, tests and documentation in the same bounded change. Preserve user data or required external protocols through one-way, bounded, observable migration only when needed, with independent schema versioning and invalid/future-version rejection proof. Temporary fallbacks need a trigger, owner, telemetry, tests and removal condition.
- For changed compatibility boundaries, prove intended success and invalid/retired-path rejection; explain old behavior, new behavior and reason. Do not add a compatibility report to unrelated changes.

## Warnings

- Do not add first-party warnings; warnings in modified code block its tranche. Report baseline/current/delta for configurations actually run. Docs-only checks without a compiler have no compiler-warning measurement; do not invent a zero.
- Remove warning debt in focused work and reach zero first-party warnings before beta/RC acceptance. Do not hide debt with crate-wide `allow(dead_code)`, warning-allow compiler flags, fake reads or arbitrary Vite chunk limits.
- Narrow third-party/platform allowlists need a reason, owner, upstream reference where applicable and expiry. Keep warnings-as-errors in CI for supported configurations that reached zero.
