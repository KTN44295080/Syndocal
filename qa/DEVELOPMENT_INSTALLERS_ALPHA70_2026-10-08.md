# Alpha.70 development installer distribution

Base: `97b9c6d3151a049e78ee92ac29254de5b7346e8f`. Branch: `codex/distribution-alpha70-20261008`.

The user requested installers usable on another PC and a Mac version. This isolated checkout packages committed Control/header/Tap and MCP improvements. Uncommitted native Video BO work and unrelated dirty owner files remain in the original checkout and are excluded. No data, project or external protocol schema changes. Default updater remains disabled. The separately licensed ASIO bridge remains excluded from the default package.

Windows x64: NSIS EXE and Japanese MSI, with pinned LGPL FFmpeg runtime and notices. macOS 12+: Apple Silicon arm64 DMG, bundled LGPL FFmpeg, generated and inspected on the existing macOS CI workflow. Intel Mac is not included.

Windows build and validation completed: `pnpm --dir app run check:release` exit 0; optimized `tauri build --no-bundle` exit 0, then NSIS/MSI build exit 0. Both the initial and final packaged executable were launched with exactly one responsive, visible, maximized window; unauthenticated broker reads were rejected. The original alpha.69 executable (SHA-256 `0abce29e600c60b6e60202d3ef74aed10a68f3ba0df799d0337483c507330ab8`) was closed gracefully and restored after each bounded probe. No UI interaction or physical output was requested. Final Windows executable SHA-256: `fdf08428c2919b7f77f4bdf586f7fc82574fd59a4f85296d85f1bd2966570aed`.

Pinned/PATH-first MSVC 14.44.35207 Build Tools was verified by the maintained wrapper. First-party Rust/TypeScript compiler warnings: baseline/current/delta 0/0/0 for these successful builds; Vite large-chunk advisory 1/1/0. Tauri recreated its incomplete NSIS tool cache, with one tool-cache warning; no first-party warning suppression. The first launch helper rejected its own UTF-8-without-BOM Japanese path in Windows PowerShell; correcting its script encoding preserved the exact original identity checks.

The release checker originally truncated the production handler registry at 20,000 characters. It now reads to the actual build boundary and requires exactly one registry; all required registered-command assertions remain. A preliminary bracket-only correction failed on an existing `#[cfg(...)]` attribute, and was replaced with the full boundary. The verified release aggregate passed. Initial missing runtime staging, SDK DLL pin mismatch, and unquoted PowerShell bundle-list failures are recorded as setup/launcher failures, with successful corrected gates. The pinned runtime was copied only after verifying its exact approved hashes; SDK include/import libraries were kept separate from the distributed DLL pin.

Raw evidence: [artifact directory](artifacts/development-installers/2026-10-08-alpha70). Frozen build manifests record source byte hashes and timestamps; checker-only and documentation edits after the initial no-bundle build do not change product source. macOS CI dispatch and final DMG verification are the next action. Installer installation on a clean PC remains unobserved.

Windows payload validation passed using the maintained two-root installer validator: extracted NSIS normal payload and MSI administrative-extraction payload both contain the exact executable, all seven pinned DLLs and all four common resources. The six NSIS installer-only plugin files are recorded separately before copying the exact normal payload for validation; no generic allowance was added to the validator. The MSI log records `MainEngineThread is returning 0`; administrative extraction is not installation. Artifacts are copied with checksum equality to `C:\Users\janua\Downloads\Syndocal-1.2.0-alpha.70`.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Syndocal_1.2.0-alpha.70_x64-setup.exe` | 82430046 | `744ffc3bfa934829e9db969e1e84d7a0e209ec749d7fec4d7d4e7c4711c7a776` |
| `Syndocal_1.2.0-alpha.70_x64_ja-JP.msi` | 123179008 | `64685b25715557826508479acba726d18011e5dc3d6f321526e1964b57eaa667` |

These are unsigned development installers. Apple Developer ID/notarization, Windows Authenticode, another physical PC, actual macOS 12 hardware, venue devices and full release acceptance are not proven. Independent review is not claimed; the user prohibited subagents. Ledger acceptance markers are unchanged.
