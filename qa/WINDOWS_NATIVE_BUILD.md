# Windows native build and test environment

Read this when running Windows Cargo/Tauri native builds or tests. Applicability,
process ownership, and native acceptance are defined in [AGENTS.md](../AGENTS.md).

- Initialize `vcvars64.bat -vcvars_ver=14.44` before Cargo starts.
- Require the resolved linker to be exactly one of these supported local paths:
  - Community: `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.
  - Build Tools: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.
  Pin that same absolute path in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`.
- Use `VC\Auxiliary\Build\vcvars64.bat` under the selected edition root.
  The Tauri wrapper preserves a valid initialized edition. When initialization
  is needed, it uses Community if its exact linker exists, otherwise Build Tools
  if its exact linker exists. A failed batch is not retried against another edition.
  Other installation roots, editions and toolset versions are not accepted.
- Print and verify the pinned variable and `where.exe link.exe` before Cargo.
  Fail closed if the exact linker is missing or not first. Git for Windows'
  `usr\bin\link.exe` is incompatible; an attempt that invokes it is invalid
  and must restart with the correct environment.
- The only additional edition-root exception is the official GitHub-hosted `windows-2022`
  image: require the exact corresponding Visual Studio 2022 **Enterprise**
  `...\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`. Keep the same toolset,
  architecture, absolute pin and PATH-first checks. Do not apply this exception
  locally or on self-hosted runners.
- In `cmd.exe`, use `cmd /v:on` and `!PATH!` when changing PATH after vcvars.
  `%PATH%` can expand before vcvars runs and discard the MSVC additions.

The maintained Tauri wrapper is [run-tauri.mjs](../app/scripts/run-tauri.mjs);
its contract is checked by
[check-tauri-build-wrapper.mjs](../app/scripts/check-tauri-build-wrapper.mjs).
The wrapper selects `cmd` code page 65001 before vcvars and `set`, matching its
UTF-8 decoder. The Windows checker captures a real Japanese `CARGO_TARGET_DIR`
and requires exact equality; OEM/UTF-8 mismatches can otherwise build into a
different directory while leaving the intended checkout executable stale.
For an isolated release target, set `CARGO_TARGET_DIR` to an absolute path.
The wrapper verifies/stops only that target's `release/syndocal.exe`, preserving
the normal checkout instance. Relative overrides fail closed because Cargo's
invocation directory can differ from the wrapper directory. Without an override,
the existing checkout `target/release/syndocal.exe` behavior is unchanged.
Normal and isolated builds in the same checkout share `app/dist`. Run them
sequentially when their Vite environment differs; separate Cargo targets do not
isolate the frontend assets embedded by Tauri.
Direct Cargo commands must satisfy the same environment contract.
The ASIO and soak PowerShell harnesses currently retain their narrower Community
preflight; Build Tools support here applies to the Tauri wrapper and direct Cargo
commands initialized and verified against the exact paths above.

## Pinned normal Windows FFmpeg SDK

Use the LGPL shared 8.1 branch build recorded in
[FFMPEG_WINDOWS_RUNTIME_INVENTORY.json](FFMPEG_WINDOWS_RUNTIME_INVENTORY.json)
(schema 2, exact provenance, configure line and seven DLL identities):

- Version: `n8.1.3-9-g29e619e767-20260930` (BtbN).
- Archive: <https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1.zip>.
- SHA-256: `3e47bda1607740550141e37c0e49d1e5182b34699f15adfd137ee266d346811a`.
- Archive root: `ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1`.
- Exact source: <https://github.com/FFmpeg/FFmpeg/archive/29e619e767cde9045a75c29bc9a8278ae7b3a98b.tar.gz>.
- Pinned build recipes/dependency locators:
  <https://github.com/BtbN/FFmpeg-Builds/tree/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b>.

Download and verify the archive hash before extraction. Set `FFMPEG_DIR` to
that extracted root (containing `include`, `lib`, `bin`) and prepend its `bin`
to the process-local `PATH` before the MSVC/Cargo invocation. Do not substitute
the floating `latest` URL, the previous Gyan GPL DLLs or an arbitrary PATH SDK.
`avcodec_configuration()` must match the inventory; `avutil_license()` reports
`LGPL version 3 or later`, without `--enable-gpl` or `--enable-nonfree`.
`prepare-release-runtime.mjs` verifies the exact seven DLLs; it never bundles
the SDK's `ffmpeg.exe` as a recording encoder.

For the Task 53 trial, use an absolute `CARGO_TARGET_DIR` below
`target/qa/ffmpeg-lgpl-20261026/`, run `pnpm --dir app tauri build --no-bundle`,
and place only the hash-verified seven candidate DLLs beside that isolated
executable before launching it. Preserve `target/release/syndocal.exe` and its
runtime DLLs. The regular bundling hook still stages `target/release`; an
isolated **installer** build needs separate resource-path isolation and is not
established by this no-bundle trial.

BtbN retains monthly releases for two years. Before publication, preserve and
serve the exact binary and complete corresponding source/dependency closure,
including patches, build environment, license texts and notices. A fixed
recipe commit still uses upstream moving build-image tags; it is not complete
reproducibility evidence. This SDK selection does not close signing,
clean-machine, ASIO or recording-encoder gates. See
[the alignment evidence](FFMPEG_LICENSE_ALIGNMENT_2026-10-26.md).
