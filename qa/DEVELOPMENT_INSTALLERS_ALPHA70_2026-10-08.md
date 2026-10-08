# Alpha.70 development installer distribution

Base: `97b9c6d3151a049e78ee92ac29254de5b7346e8f`. Branch: `codex/distribution-alpha70-20261008`.

The user requested installers usable on another PC and a Mac version. This isolated checkout packages committed Control/header/Tap and MCP improvements. Uncommitted native Video BO work and unrelated dirty owner files remain in the original checkout and are excluded. No data, project or external protocol schema changes. Default updater remains disabled. The separately licensed ASIO bridge remains excluded from the default package.

Windows x64: NSIS EXE and Japanese MSI, with pinned LGPL FFmpeg runtime and notices. macOS 12+: Apple Silicon arm64 DMG, bundled LGPL FFmpeg, generated and inspected on the existing macOS CI workflow. Intel Mac is not included.

Windows build and validation completed: `pnpm --dir app run check:release` exit 0; optimized `tauri build --no-bundle` exit 0, then NSIS/MSI build exit 0. Both the initial and final packaged executable were launched with exactly one responsive, visible, maximized window; unauthenticated broker reads were rejected. The original alpha.69 executable (SHA-256 `0abce29e600c60b6e60202d3ef74aed10a68f3ba0df799d0337483c507330ab8`) was closed gracefully and restored after each bounded probe. No UI interaction or physical output was requested. Final Windows executable SHA-256: `fdf08428c2919b7f77f4bdf586f7fc82574fd59a4f85296d85f1bd2966570aed`.

Pinned/PATH-first MSVC 14.44.35207 Build Tools was verified by the maintained wrapper. First-party Rust/TypeScript compiler warnings: baseline/current/delta 0/0/0 for these successful builds; Vite large-chunk advisory 1/1/0. Tauri recreated its incomplete NSIS tool cache, with one tool-cache warning; no first-party warning suppression. The first launch helper rejected its own UTF-8-without-BOM Japanese path in Windows PowerShell; correcting its script encoding preserved the exact original identity checks.

The release checker originally truncated the production handler registry at 20,000 characters. It now reads to the actual build boundary and requires exactly one registry; all required registered-command assertions remain. A preliminary bracket-only correction failed on an existing `#[cfg(...)]` attribute, and was replaced with the full boundary. The verified release aggregate passed. Initial missing runtime staging, SDK DLL pin mismatch, and unquoted PowerShell bundle-list failures are recorded as setup/launcher failures, with successful corrected gates. The pinned runtime was copied only after verifying its exact approved hashes; SDK include/import libraries were kept separate from the distributed DLL pin.

Raw evidence: [artifact directory](artifacts/development-installers/2026-10-08-alpha70). Frozen build manifests record source byte hashes and timestamps; checker-only and documentation edits after the initial no-bundle build do not change product source. Completed macOS CI and final DMG verification are recorded below. Installer installation on a clean PC remains unobserved.

Windows payload validation passed using the maintained two-root installer validator: extracted NSIS normal payload and MSI administrative-extraction payload both contain the exact executable, all seven pinned DLLs and all four common resources. The six NSIS installer-only plugin files are recorded separately before copying the exact normal payload for validation; no generic allowance was added to the validator. The MSI log records `MainEngineThread is returning 0`; administrative extraction is not installation. Artifacts are copied with checksum equality to `C:\Users\janua\Downloads\Syndocal-1.2.0-alpha.70`.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Syndocal_1.2.0-alpha.70_x64-setup.exe` | 82430046 | `744ffc3bfa934829e9db969e1e84d7a0e209ec749d7fec4d7d4e7c4711c7a776` |
| `Syndocal_1.2.0-alpha.70_x64_ja-JP.msi` | 123179008 | `64685b25715557826508479acba726d18011e5dc3d6f321526e1964b57eaa667` |

These are unsigned development installers. Apple Developer ID/notarization, Windows Authenticode, another physical PC, actual macOS 12 hardware, venue devices and full release acceptance are not proven. Independent review is not claimed; the user prohibited subagents. Ledger acceptance markers are unchanged.

## Another-PC prerequisites

The PE dependency inspection found `MSVCP140.dll` in the Windows executable; the seven FFmpeg DLLs import only the bundled FFmpeg set and Windows system libraries. The delivery folder includes the original Microsoft-signed `VC_redist.x64.exe` (14.51.36247.0, 18731856 bytes, SHA-256 `843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c`), acquired from the official [Microsoft x64 permalink](https://aka.ms/vc14/vc_redist.x64.exe). Its Authenticode status was `Valid`, with Microsoft Corporation as signer. [Microsoft documentation](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) requires a compatible x64 runtime at least as recent as the build tools.

The version-specific [setup helper](../tools/distribution/alpha70/Install-Windows.ps1) verifies the exact Syndocal EXE hash and the exact signed Microsoft prerequisite before opening either. It handles prerequisite failure, restart-required status, and already-newer-runtime verification; it does not execute installers during `-ValidateOnly`. That read-only check passed on the delivered folder. The helper's actual installation path remains unexecuted. WebView2 may require network installation on a machine without that runtime. The Microsoft binary is an additional prerequisite, not a change to Syndocal's approved normal bundle payload or its immutable alpha.70 installers.

The macOS workflow was dispatched at source commit `c509b642006472fb2735cc61356b3dbf72c8a5a6`: [run 37742560103](https://github.com/KTN44295080/Syndocal/actions/runs/37742560103). Final DMG validation and workflow upload succeeded. Repository log copies trim trailing whitespace/blank lines to satisfy `git diff --check`; original Temp evidence bytes remain unchanged.

## Final macOS delivery

Run 37742560103 completed successfully with its successful final-DMG validation, separate upload acceptance, and DMG upload steps at source `c509b642006472fb2735cc61356b3dbf72c8a5a6`. The downloaded artifact and checksum match the successful report: `Syndocal_1.2.0-alpha.70_arm64.dmg`, 33429558 bytes, SHA-256 `09a512b68d931adad24724a3ee0ae6b009ed1c0c7219b26e9c5f4ffe07e9954a`. It is copied to the same Downloads folder. All ten report checks passed, and the validator suite passed 147/147 with no skipped tests. This proves the arm64/macOS 12 deployment target and self-contained dylib graph, final image integrity, ad-hoc signature integrity, extraction, eight seconds of process survival and cleanup on the macOS 15.7.9 CI host. It does not prove native UI responsiveness, actual macOS 12 execution, Gatekeeper, hardware, Intel Mac or notarization.

The Windows setup-helper lifecycle tests passed six cases against the real helper with process/registry seams replaced: success, application cancellation, prerequisite cancellation, restart required, already-newer runtime, and too-old runtime rejection. No installer was executed. The initial test runner inherited PowerShell 7 modules into Windows PowerShell and normalized nested exit codes; explicit Windows PowerShell modules and forwarding the child exit code corrected the harness, without changing product/helper conditions.

macOS build warnings measured from the completed workflow: first-party Rust compiler current 0; Vite large-chunk advisory current 1; third-party FFmpeg compiler/linker current 26. Their macOS baseline/delta are unmeasured; Windows measurements above are not a macOS baseline. No warning allowance or suppression was added. This is a development artifact and does not close release warning debt or FFmpeg upstream acceptance.

## Preserved original checkout

The original checkout remains at `97b9c6d3` with its protected five owner hashes unchanged and its native Video BO/output authorization tranche uncommitted. That tranche's units/static checks passed, but the real native legacy Video BO request returned `native_operation_not_supported`; the method admission route still needs investigation. Failure evidence is `%TEMP%/syndocal-native-output-guard-native-20261007.json` and `.txt`. Its QA process and credential were cleaned up. The shared private acceptance target now contains this alpha.70 distribution executable, so future Video BO native work must rebuild its exact QA identifier and verify the resulting executable identity before running. Do not reuse this distribution executable as QA evidence. No original source, index or protected work was staged in this distribution branch.

## Files to give another computer

- Windows: `Syndocal_1.2.0-alpha.70_Windows-x64-install.zip` (101168009 bytes, SHA-256 `8f2a7ecda32464c62dc546074fcaa4d48658cbcb466f4ecfdabcab3b48edd8af`). Extract the complete ZIP and run `Install-Windows.cmd`; do not copy only the helper. The ZIP contains the EXE installer, its checksum, the signed Microsoft prerequisite, the verified helper and English/Japanese instructions. 7-Zip archive integrity check passed. MSI remains an alternative in the delivery folder.
- Mac: `Syndocal_1.2.0-alpha.70_arm64.dmg` and its `.sha256`; drag the app into Applications. Apple Silicon only, macOS 12 minimum; the CI survival proof was on macOS 15.7.9.

Both are local in `C:\Users\janua\Downloads\Syndocal-1.2.0-alpha.70`. No public release/tag or updater endpoint was published. Windows and Mac package bytes remain immutable; the final ZIP is a delivery wrapper around the accepted EXE and Microsoft prerequisite. The preliminary wrapper is preserved in Temp and is not the delivered file. A new public/modified product build must advance the ordinal again.
