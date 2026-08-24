# Syndocal exact Windows linker checkpoint — 2026-08-25

## Authority and scope

- Branch: `codex/syndocal-v1.2`
- Pre-checkpoint base HEAD: `4009e6313f3f0c68cc58e5fc268d408df8636fad`
- Upstream before the checkpoint: `origin/codex/syndocal-v1.2`, ahead/behind `0/0`
- Product metadata remains `1.2.0-alpha.10`. This is a build-safety and QA-entrypoint checkpoint, not a distributed product artifact or release acceptance.

Every known Windows Cargo/Tauri entrypoint now fails closed unless it uses the
exact local VS2022 Community x64 toolset `14.44.35207` and absolute
`Hostx64\x64\link.exe`. The accepted pin and the complete
`where.exe link.exe` order are printed before Cargo. Git for Windows'
`usr\bin\link.exe`, stale toolsets, missing or non-file linkers, empty resolution,
and mismatched pins are rejected before Cargo starts.

The only edition-root exception is the official GitHub-hosted `windows-2022`
runner, whose published image inventory uses Visual Studio 2022 Enterprise. The
exception still requires toolset `14.44.35207`, `Hostx64\x64`, an absolute Cargo
pin, exact `where.exe`-first proof, `RUNNER_ENVIRONMENT=github-hosted`, the Windows
runner context, and the dedicated workflow marker. A local or self-hosted runner
cannot select it. Source: [GitHub Windows Server 2022 runner image inventory](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md?plain=1).

Covered entrypoints:

- guarded Tauri dev/build wrapper and both native QA launchers;
- cross-platform Windows workflow direct Cargo/rustc commands;
- warning-ratchet Cargo metadata and configured Cargo command;
- isolated ASIO build harness;
- M5 soak harness.

The soak dependency-injection seam is accepted only with `-PreflightOnly` and is
rejected before any build or soak-harness run, so a hermetic test override cannot
bypass production Cargo gating.

## Verification evidence

The following commands passed on 2026-08-25:

- `node app/scripts/check-tauri-build-wrapper.mjs` — `156 assertions`.
- `node app/scripts/test-warning-ratchet.mjs` — `warning ratchet self-tests ok`.
- `pwsh -NoProfile -File qa/test-run-soak-linker-gate.ps1` — `74 assertions`; Cargo, vcvars, `where.exe`, and the soak executable were intercepted in the hermetic cases.
- `pwsh -NoProfile -File qa/harnesses/check-asio-build.ps1 -SelfTest` — `36 assertions`; Cargo was never invoked.
- Live `qa/run-soak.ps1 -PreflightOnly` — exact Community linker first, Git linker second, exit `0`, no Cargo or soak launch.
- Live ASIO `-PreflightOnly` with `CPAL_ASIO_DIR=...\asio-sdk-2.3.4\ASIOSDK` and `LIBCLANG_PATH=C:\Program Files\LLVM\bin` — exact Community linker first, Git linker second, exit `0`, no Cargo.
- PowerShell parser checks for the two native QA launchers, ASIO harness, soak harness, and soak self-test — `0` errors in all five files.
- Python/PyYAML parse of `.github/workflows/cross-platform.yml` — `YAML_OK`, job `test`.
- `git diff --check` — no whitespace errors; CRLF conversion notices only.

The real Cargo release warning gate was also run from the exact pinned
Community environment:

```text
pnpm --dir app run check:warnings -- --configuration windows-default-release
VCToolsInstallDir = ...\MSVC\14.44.35207
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = ...\bin\Hostx64\x64\link.exe
where.exe link.exe # Community exact linker first; Git linker second
artifact coverage = 11/11
baseline warnings total/first-party/third-party = 83/83/0
current warnings total/first-party/third-party = 0/0/0
identity removals = 67
exit = 0
```

Two earlier diagnostic attempts stopped before Cargo: one lacked the explicit
FFmpeg/libclang inputs and one detected Git's linker first after pnpm rebuilt
`PATH`. Neither is counted as a Cargo result. The warning ratchet now captures a
fresh exact vcvars environment inside its own process immediately before Cargo,
which closed that path-reconstruction gap.

An independent Terra xhigh adversarial re-review reported P0 `0`, P1 `0`, and
P2 `0`. It reran the wrapper checker, both hermetic harness tests, all five
PowerShell parser checks, and `git diff --check` without Cargo, Tauri, process
control, file edits, or Git writes.

## Remaining boundaries and next action

- This checkpoint did not run a fresh no-bundle native release build or native UI QA; it changes build/tooling entrypoints, not the accepted alpha.10 product surface.
- GitHub-hosted workflow execution remains external CI evidence and is not inferred from local YAML/static checks.
- Full ASIO Cargo/native/device acceptance, the integrated long soak, D4 merge, physical DJ Link, and the remaining completion-flow rows remain open.
- Next action: commit and push this checkpoint, then merge the already pushed D4 Stage/Timeline/Source integration, advance the synchronized prerelease ordinal, and run the required exact-linker native build/launch/4K acceptance.
- Verified obsolete workspace artifacts remain queued for recoverable cleanup only after the owning checkpoint is committed and pushed; exact deleted paths and reclaimed bytes must be recorded separately.
