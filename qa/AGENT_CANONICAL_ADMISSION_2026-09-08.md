# Canonical adapter admission — 2026-09-08

Base: `00e878500f9c0613871109aa612c435d6393be06`.
Worktree branch: `chatgpt/macos-artifact-validation`.
This checkpoint owns the canonical adapter, its focused checker, this record
and the remaining-work scope clarification. The separate uncommitted macOS
validator remains outside acceptance.

## Reproduced defect and correction

`CANONICAL_TAURI_COMMANDS` is a plain object. Reading it with an untrusted
operation ID admitted inherited names such as `constructor`, `__proto__` and
`toString`. Actual production-module tests with a mock invoke observed a
mutation-start notification and a non-string invoke argument instead of rejection.
No native command or device operation was used to reproduce this condition.

The adapter now requires an own property before reading the command, announcing
a mutation or invoking Tauri. The same 47 reviewed operations remain permitted.
There is no new fallback, operation, schema, transport or UI behavior.
The check is constant-time and adds no polling, worker, frame copy or allocation
proportional to scene size. No application performance improvement is claimed.

Rust admission and the MCP server already validate their own 47-operation
whitelists. This is a direct renderer-adapter admission defect, not evidence of
external arbitrary-command execution or of a bypass through those earlier gates.

## Automated and independent evidence

On Windows with Node 22.22.1 and pnpm 10.9.0:

- Regression added before the fix: failed with `Missing expected rejection`.
- After the fix, `check:agent-bridge`: PASS, 11 groups. The new group checks
  all 12 Object.prototype own names and all 47 allowed operation IDs.
- `pnpm --dir app exec tsc --noEmit`: exit 0.
- `pnpm --dir app run check:release`: exit 0 after the adapter change.
- `check:tauri-build-wrapper`: 243 assertions / 27 hostile fixtures passed.
- Independent read-only Luna review: no blockers. The reviewer also confirmed
  the earlier Rust and MCP-server whitelists; it did not claim external exploitability.

Logs are retained under `target/qa/macos-artifact-local/`:
`agent-prototype-before.log`, `agent-prototype-after.log`, `release-03.log`,
`wrapper-02.log`, and `agent-prototype-review.txt`.

The first native build failed because bindgen could not locate libclang. The
failure log and verbose dependency diagnostic are retained, not overwritten.
The existing Build Tools LLVM directory was selected through process-local
`LIBCLANG_PATH`; no global installation, library pin or build gate was changed.

## Native evidence and completion boundary

`pnpm --dir app tauri build --no-bundle` passed using the exact Build Tools
MSVC 14.44.35207 linker and the maintained wrapper. The successful build log
contains zero Rust warning diagnostics. A pre-change native warning baseline
for this isolated worktree was not measured; no baseline/delta is invented.

Artifact: this worktree's `target/release/syndocal.exe`, 64,420,864 bytes,
version `1.2.0-alpha.69`, SHA-256:
`85730E4D0EDF2E4D48E62DD4C3E986079402E8014F31F1F98EC8D2DDACCBFF8F`.
It was built from the stated base plus this checkpoint's product-source diff.
The subsequent commit is not presented as a pre-build frozen release tag.

At `2026-09-07T17:40:24.4751013Z`, PID 4252 had exactly one visible `Syndocal`
window for that executable path, responsive and maximized. The helper issued
no physical-output command. It stopped only its own exact-path application;
the post-check found zero remaining processes for that executable.
`native-build-02.log`, `native-launch-01.json` and `native-launch-01.log`
retain this evidence. This is window acceptance, not hardware or whole-show QA.

The admission fix has its focused checks, independent review and native gate.
Full AI parity, Mac acceptance, physical devices, distribution/signing and
product-wide completion remain outside this bounded checkpoint.
