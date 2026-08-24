# M6 Release Validation

> Historical evidence notice: the v1.0.0 artifacts below prove the named 2026-07
> baseline only. The current product train is `1.2.0-alpha.10`; its release cannot
> inherit these hashes, version metadata, signatures, clean-machine, ASIO, or current
> source evidence. The post-alpha.10 pause request was rescinded before promotion;
> follow `AGENTS.md` and the active
> `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`. The post-alpha.10 snapshot and this
> file remain detailed checkpoint/release-gate evidence records.

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

## Full-window UI acceptance

`pnpm --dir app run check:release-ui` is the release UI aggregate. The browser
suite judges the full desk at 1920x1080 first and retains 2048x1152 plus the
1366x768/1280x720 fallbacks. On Windows the same command then launches an
isolated real Tauri build, proves maximized content of at least 1920x1000,
toggles F11 to an exact 1920x1080 client, and verifies that Esc restores the
exact starting maximized dimensions. Screenshots and a JSON report are written
outside the source tree. See `qa/NATIVE_WINDOW_ACCEPTANCE.md`.

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

## Signed updater readiness (v1.1)

The application now embeds Tauri's minisign-verified updater and supports
compile-time `stable`, `beta`, and `nightly` channels. Builds without both an
HTTPS endpoint and a valid public key remain safely disabled. The Project menu
checks in the background, reports available/current/error state, rechecks the
selected version before install, and creates a verified project backup before
downloading. Updater artifact generation is isolated in
`app/src-tauri/tauri.updater.conf.json`; key custody, manifest publication, and
tamper acceptance are documented in `qa/UPDATE_RELEASE_RUNBOOK.md`.

Software configuration validation and disabled-build UI pass locally. A real
N to N+1 install cannot be accepted until release operations supplies the
public HTTPS endpoint, minisign private key secret, and platform signing
credentials. This external gate is not represented as a completed signed
release.

## User template portability (v1.1)

Project menu export/import uses `.sdctemplate` v1 to carry the validated
Syndocal project, embedded fixture profiles, and MIDI/OSC/DMX control mappings.
Imports are bounded to 64 MiB, reject the wrong application/version or invalid
project references and mappings, and create an unsaved project. Every legacy
and routed DMX output plus every video output is disabled, with lighting and
video blackout enabled, before the template reaches the engine. Existing
`.midimap` and `.oscmap` sharing remains available independently.

DMX mappings use the same validated action surface but retain a typed
universe/channel source. Imported or learned mappings round-trip through the
template and open with every output disarmed. Input listener configuration and
physical device selection remain machine-local Setup > I/O state.

Rust tests cover round-trip mappings, omitted legacy mapping fields, identity
and extension rejection, and complete output disarming. The project menu and
its template actions remain part of the multi-viewport containment check.

## UI locale validation (v1.1)

English remains the default source locale and Japanese can be selected from
the Project menu. The device-local preference is independent from `.sdc` and
workspace layout state. Static operator labels, dynamic project/template/update
status, and title/ARIA/placeholder attributes share one reversible translation
boundary; unknown text falls back to English. Mutation handling is scoped to
changed nodes rather than rescanning the application for every telemetry tick.

`pnpm --dir app run check:localization` covers locale normalization, whitespace,
dynamic patterns, English fallback, persistence, storage denial, and an AST
inventory of static TSX labels/attributes. The Japanese static coverage is
1913/1913 (100%), including explicitly classified shared units, protocol names,
coordinates, and input examples; any new unclassified static text fails the
check. A second AST gate requires user-authored labels to opt out of translation;
the browser fixture names `Video`, `Save`, and `Output` remain unchanged in the
Japanese locale to prove dictionary collisions cannot rename show data. The real
browser viewport gate switches to Japanese, verifies `html[lang=ja]` and
translated Project menu controls, then checks 1366x768 containment before
returning to English.

The inventory at this historical v1.1 checkpoint was 1913/1913 static labels and attributes (100%).
