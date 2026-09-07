# macOS installer runbook

## Current route

The repository does not require a local Mac for the development installer. Run
the `macOS Installer` workflow manually from GitHub Actions. It uses the
standard `macos-15` runner, builds the app for arm64 with a macOS 12.0 minimum,
bundles the LGPL FFmpeg runtime, creates a drag-to-Applications DMG, launches
the app for a smoke check, validates the DMG structure, and uploads the DMG
with a SHA-256 file for seven days.

The normal `Cross-platform` workflow intentionally excludes macOS packaging.
This keeps every push and pull request from consuming a macOS runner; macOS
packaging is an explicit, one-job action only when requested.

## Verified run

The repository is public at
<https://github.com/KTN44295080/Syndocal>. Run
<https://github.com/KTN44295080/Syndocal/actions/runs/34139676339> completed
successfully from `main` at commit `33c7da89727fdf636822f45ae2882fe1f1a09af7`.
The application bundle, LGPL FFmpeg runtime bundling, DMG creation, launch
smoke test, `hdiutil imageinfo`, SHA-256 generation, and artifact upload all
passed. The uploaded artifact is
`syndocal-macos-ARM64-33c7da89727fdf636822f45ae2882fe1f1a09af7` (29,965,371
bytes, retained for seven days).

## Distribution boundary

The current DMG is a development artifact. The bundling script uses an
ad-hoc signature because no Apple Developer ID certificate is available. It
must not be presented as a public Gatekeeper-ready release. Public
distribution requires the Apple Developer ID signing, hardened runtime,
notarization, stapling, and clean-Mac acceptance steps recorded in the release
roadmap.

The `macos-15` route produces arm64 output. Intel output needs a separate
Intel runner and an architecture-specific FFmpeg build; it is not silently
claimed by this workflow.

## Billing recovery

This separation reduces future Actions use but cannot restore a consumed quota
or override an account spending block. If the manual workflow is blocked before
its first step, inspect the account billing and Actions usage page, wait for the
billing-cycle reset, or add a valid payment method and an appropriate budget.
