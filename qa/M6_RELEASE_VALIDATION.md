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
| `Syndocal_1.0.0_x64-setup.exe` | 71,463,799 | `6E2244E4EAB4B3377FDF826B9055924FF0C2BB325623CD1FDCE9777824EFF210` | NotSigned |
| `Syndocal_1.0.0_x64_ja-JP.msi` | 108,032,000 | `D2A53F0457F2C49C49ADAB4E087CF019D09AE6CA83B3703B44B510D7F46C839D` | NotSigned |

The MSI uses `ja-JP` so the required publisher name can be represented by WiX without code-page loss. NSIS includes Japanese and English UI languages.

## Windows install smoke

Result: **PASS**

1. MSI administrative extraction contained `syndocal.exe` plus seven FFmpeg runtime DLLs. File metadata reported `Syndocal`, version `1.0.0`, company `Seraf()のKTN`.
2. NSIS silent install placed `syndocal.exe`, seven FFmpeg DLLs, and `uninstall.exe` in one application directory. Its post-install hook copies staged runtime libraries beside the executable and removes the staging directory.
3. The installed executable launched successfully and rendered the complete one-screen Setup/Patch/2D Mapping workspace with the new application icon.
4. The installer registered `.sdc` as `Syndocal Project`; the open command was `syndocal.exe "%1"` and the executable icon was registered.
5. Silent uninstall removed the test installation and its association. No Syndocal process remained.

## Cross-platform release CI

`.github/workflows/cross-platform.yml` builds and uploads:

- Windows: NSIS + MSI
- macOS: `.app` + DMG
- Ubuntu 22.04: `.deb` + AppImage

The matrix installs FFmpeg development/runtime dependencies and tests the default in-process libav path. On Windows, `prepare-release-runtime.mjs` stages the shared FFmpeg DLLs immediately before Tauri bundling. The Windows platform config packages those DLLs for both MSI and NSIS.

Hosted macOS/Linux artifact execution remains external CI evidence; it cannot be produced on the Windows workstation. The workflow fails when an expected bundle directory is missing, so absent artifacts are not silently accepted.

## Signing decision

v1.0 personal distribution is intentionally unsigned. Windows SmartScreen and macOS Gatekeeper instructions must remain in the user README. A public trusted release requires an Authenticode certificate and an Apple Developer ID/notarization secret; neither credential is stored in this repository.
