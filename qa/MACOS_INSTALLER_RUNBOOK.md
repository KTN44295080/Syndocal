# macOS installer runbook

## Current route

The repository does not require a local Mac for the development installer. Run
the `macOS Installer` workflow manually from GitHub Actions. It uses the
standard `macos-15` runner, builds the app for arm64 with a macOS 12.0 minimum,
bundles the LGPL FFmpeg runtime and creates a drag-to-Applications DMG. The
candidate final-artifact gate extracts the app from that DMG into a private
temporary directory, verifies its metadata, Mach-O dependencies and signature
integrity, then observes eight seconds of process survival and reaps its child.
It uploads the DMG/checksum only after validation and a separate report/file
acceptance step both succeed. This is not UI, hardware or Gatekeeper acceptance.

The normal `Cross-platform` workflow intentionally excludes macOS packaging.
This keeps every push and pull request from consuming a macOS runner; macOS
packaging is an explicit, one-job action only when requested.

## Previously verified packaging run (not the new final-DMG gate)

The repository is public at
<https://github.com/KTN44295080/Syndocal>. Run
<https://github.com/KTN44295080/Syndocal/actions/runs/34139676339> completed
successfully from `main` at commit `33c7da89727fdf636822f45ae2882fe1f1a09af7`.
The application bundle, LGPL FFmpeg runtime bundling, DMG creation, launch
smoke test, `hdiutil imageinfo`, SHA-256 generation, and artifact upload all
passed. The uploaded artifact is
`syndocal-macos-ARM64-33c7da89727fdf636822f45ae2882fe1f1a09af7` (29,965,371
bytes, retained for seven days).

## Final-DMG gate candidate

Branch: `chatgpt/macos-final-gate-20260908`, based on `cfac3c3`.
Windows Node tests passed 147/147 with no skipped tests. The new macOS CI run
has NOT been executed; the older run above cannot be used as its proof.
See [the current checkpoint](MACOS_FINAL_GATE_2026-09-08.md) for evidence and limits.

The report records the exact source SHA, run ID/attempt and DMG SHA-256.
`report.json` alone does not establish success: require the matching non-cancelled
workflow/job and successful validation/acceptance steps. Pending files are not
accepted reports. Any remaining staging alias, unexpected upload candidate,
failed check, stale run identity or changed DMG fails upload acceptance.
Evidence is uploaded on failure as well; the installer is not uploaded as a
successful result when validation or acceptance fails.

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
