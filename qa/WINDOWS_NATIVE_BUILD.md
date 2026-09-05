# Windows native build and test environment

Read this when running Windows Cargo/Tauri native builds or tests. Applicability,
process ownership, and native acceptance are defined in [AGENTS.md](../AGENTS.md).

- Initialize `vcvars64.bat -vcvars_ver=14.44` before Cargo starts.
- Require the resolved linker to be exactly
  `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.
  Pin that same absolute path in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`.
- Print and verify the pinned variable and `where.exe link.exe` before Cargo.
  Fail closed if the exact linker is missing or not first. Git for Windows'
  `usr\bin\link.exe` is incompatible; an attempt that invokes it is invalid
  and must restart with the correct environment.
- The only edition-root exception is the official GitHub-hosted `windows-2022`
  image: require the exact corresponding Visual Studio 2022 **Enterprise**
  `...\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`. Keep the same toolset,
  architecture, absolute pin and PATH-first checks. Do not apply this exception
  locally or on self-hosted runners.
- In `cmd.exe`, use `cmd /v:on` and `!PATH!` when changing PATH after vcvars.
  `%PATH%` can expand before vcvars runs and discard the MSVC additions.

The maintained Tauri wrapper is [run-tauri.mjs](../app/scripts/run-tauri.mjs);
its contract is checked by
[check-tauri-build-wrapper.mjs](../app/scripts/check-tauri-build-wrapper.mjs).
Direct Cargo commands must satisfy the same environment contract.
