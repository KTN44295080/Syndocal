# M6 Release Validation

Date: 2026-07-12  
Release: Syndocal 1.0.0  
Publisher: Seraf()のKTN

## Release metadata

`pnpm --dir app run check:release` passes and enforces:

- Product/package/workspace version `1.0.0`
- Product name `Syndocal`
- Publisher `Seraf()のKTN`
- `.sdc` project association
- Active Tauri bundling and all configured desktop icon files
- Cargo package version inheritance across every workspace crate

The original 1x1 placeholder icon was replaced with the Syndocal synchronization mark and generated into PNG, ICO, and ICNS desktop assets.

## Windows bundles

Built locally from the v1.0.0 release executable:

| Bundle | Bytes | SHA-256 | Signature |
| --- | ---: | --- | --- |
| `Syndocal_1.0.0_x64-setup.exe` | 49,704,383 | `DC0FED347732305ADFFC7374F453A0D05FAA20EA4C802EA1674EB2989430F29D` | NotSigned |
| `Syndocal_1.0.0_x64_ja-JP.msi` | 66,654,208 | `BD8DB78B306609D15E21A250DA9A4C74E2D303128F1399B6D0E73CD62597A77F` | NotSigned |

The MSI uses `ja-JP` so the required publisher name can be represented by WiX without code-page loss. NSIS includes Japanese and English UI languages.

## Windows install smoke

Result: **PASS**

1. MSI administrative extraction contained `syndocal.exe` plus seven FFmpeg runtime DLLs. File metadata reported `Syndocal`, version `1.0.0`, company `Seraf()のKTN`.
2. The default-feature release executable was built with in-process libav enabled. Both installers place seven FFmpeg DLLs directly beside `syndocal.exe`; neither format depends on PATH or an installer-specific copy hook.
3. The installed executable launched successfully and rendered the complete one-screen Setup/Patch/2D Mapping workspace with the new application icon.
4. The installer registered `.sdc` as `Syndocal Project`; the open command was `syndocal.exe "%1"` and the executable icon was registered.
5. Silent uninstall removed the test installation and its association. No Syndocal process remained.

## Cross-platform release CI

Final workflow: [Cross-platform run 29179218727](https://github.com/Seraf0-org/Rayard/actions/runs/29179218727), commit `1f04fd3`, result: **PASS** on all three jobs.

`.github/workflows/cross-platform.yml` builds, package-smokes, and uploads:

- Windows: NSIS + MSI
- macOS: `.app` + DMG
- Ubuntu 22.04: `.deb` + AppImage

The matrix installs LGPL FFmpeg development/runtime dependencies and tests the default in-process libav path. On Windows, CI silently installs NSIS, starts the installed app for eight seconds, verifies the local DLL set, uninstalls it, then administratively extracts MSI and verifies the same executable-adjacent DLL layout. On macOS it starts the bundled `.app` without `DYLD_LIBRARY_PATH`. On Ubuntu it starts the generated AppImage with `--appimage-extract-and-run` under Xvfb.

The macOS bundle contains five FFmpeg dylibs addressed through `@rpath`, declares macOS 12.0 as its minimum version, and has no Homebrew or runner-home dependency. All platform artifacts include `THIRD_PARTY_NOTICES.md` and the LGPLv3 text. The workflow fails when a bundle is absent or its packaged application does not remain alive for the smoke interval.

## Release disposition

The v1.0 software release gate is complete. Physical Art-Net/sACN/serial waveform checks remain an external rig acceptance activity documented in `qa/M4_IO_VALIDATION.md`; unavailable hardware is not represented as a software implementation pass.

## Signing decision

v1.0 personal distribution is intentionally unsigned. Windows SmartScreen and macOS Gatekeeper instructions must remain in the user README. A public trusted release requires an Authenticode certificate and an Apple Developer ID/notarization secret; neither credential is stored in this repository.
