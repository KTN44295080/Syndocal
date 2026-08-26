# Signed Application Update Runbook

Updated: 2026-08-26

Syndocal uses Tauri's signed updater. A normal local build deliberately has no
update endpoint and shows that state in Project -> Application Updates. A
release build enables checking and installation only when the endpoint and
public key below are embedded at compile time.

## One-time key setup

Generate the updater keypair on a trusted offline or release host:

```powershell
npx tauri signer generate -w C:\secure\syndocal-updater.key
```

Store the private key and password in the release secret manager. Never commit
the private key. Keep an offline recovery copy: losing it prevents existing
installations from accepting future updates. The `.pub` file contents are not
secret and are embedded into the app as `SYNDOCAL_UPDATE_PUBKEY`.

## Release build

Set all variables in the same release job that runs the Tauri build:

```powershell
$env:SYNDOCAL_UPDATE_ENDPOINT = "https://releases.example.invalid/syndocal/stable/{{target}}/{{arch}}/{{current_version}}"
$env:SYNDOCAL_UPDATE_PUBKEY = Get-Content -Raw C:\secure\syndocal-updater.key.pub
$env:SYNDOCAL_UPDATE_CHANNEL = "stable"
$env:TAURI_SIGNING_PRIVATE_KEY = "C:\secure\syndocal-updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<release-secret>"
npm run tauri build -- --config src-tauri/tauri.updater.conf.json
```

Allowed channel values are `stable`, `beta`, and `nightly`. Use a separate
endpoint path and separately built application for each channel; the operator
cannot redirect a signed build to another endpoint at runtime.

Publish the generated updater artifact and its `.sig`, then return either the
Tauri dynamic manifest or static `platforms` manifest. Manifest artifact URLs
must use HTTPS, `version` must be SemVer, and `signature` must contain the
generated signature content rather than a path.

## Local-only show-ASIO is not an updater channel

The proposed non-default `show-asio` feature/overlay exists only for the
controlled local 2026-08-30 performance path, whose development, acceptance,
and show preparation must complete by 2026-08-29. The normal signed updater
never governs that artifact: it must not discover, install, update, repair, replace, select,
or attest a local-only show-ASIO build. Do not create a `stable`, `beta`, or
`nightly` endpoint that serves it, and do not interpret updater metadata or a
normal MIT package check as show-ASIO provenance.

Checkpoint `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` built and
manifest-verified the exact same-host artifact
`target/show-asio-local/Syndocal_Show_ASIO_1.2.0-alpha.12_ff61a6dec6eb_x64`.
Its manifest fixes `distributionApproved: false`, `sameHostOnly: true`, and
`unbundled: true`. The artifact is not an updater payload or public-distribution
approval, and physical native/operator acceptance remains open. It must pass
its dedicated checker immediately before use. If it changes, rebuild and rerun
that dedicated proof; never fall back to an older DLL, the normal WASAPI
artifact, another driver, or the signed updater.

## Windows candidate package-content boundary

The normal MIT artifact never contains an ASIO bridge. Before a Windows release
candidate can be accepted, run the normal release metadata and ASIO packaging
checks, then prove the installed/extracted contents of all three Windows
delivery surfaces: NSIS, MSI, and updater payload.

The proof must use a repository-owned deterministic extractor, record an
immutable inventory, and reject canonical ASIO bridge names, retired names,
third-party ASIO names, known bridge hashes under any filename, and every
FFmpeg name/size/SHA-256/PE mismatch. The default CI already rechecks the
materialized NSIS and MSI directories after its smoke steps.

No repository-owned deterministic extractor currently exists for safe NSIS and
updater payload extraction. Therefore release-candidate package acceptance is
intentionally fail-closed. Do not execute or unpack an arbitrary candidate
installer as a substitute. Implement and independently review that extractor
and its extraction-inventory schema before changing this state.

## Inventory authority, target scope, reparse policy, and the NDI signal contract

The approved runtime values in `qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` are
mirrored as independent code anchors in
`app/scripts/windows-runtime-inventory.mjs`
(`canonicalPinnedRuntimeIdentity`, `canonicalPinnedCommonResourceIdentity`,
`canonicalKnownAsioBridgeSha256`, `anchoredAmd64PeIdentity`). Loading the
inventory enforces exact equality against these anchors; editing only the JSON
can never redefine what is approved, and mutation self-tests prove each edited
field fails. The anchors are part of the packaging ABI: change them only in a
reviewed tranche that also refreshes the pinned SDK source evidence.

The packaging target triple is exactly `x86_64-pc-windows-msvc`. ARM64
(`aarch64-pc-windows-msvc`) is rejected by name, and every ingested DLL must
match the anchored PE machine `0x8664`; no route copies AMD64 binaries to an
ARM64 layout. Windows aliasing is rejected at attribute level: symlinks,
junctions, mount points, cloud/on-demand placeholders (every reparse tag), and
hard-link aliases (`nlink != 1`) fail closed on all staged, bundled, and
extracted surfaces. Alias-coverage self-test cases that cannot be created on a
host (for example symlink creation without privilege) are reported as explicit
`SKIP` lines and are never counted as executed passing assertions; an EPERM or
EACCES during fixture creation is never treated as a pass.

NDI remains excluded from normal packaging. The sanctioned default feature set
is exactly `libav + spout`. Detection is layered and repository-owned: (1) the
four explicit feature-signal environment variables reject any NDI value;
(2) any other NDI-named environment variable is rejected as an unknown bypass,
except the reviewed SDK-path allowlist `WINDIR`, `NDI_SDK_DIR`, and
`NDI_RUNTIME_DIR_V2`..`V6` (owner: release engineering; revisit when the NDI
overlay route is formalized); (3) `app/package.json`,
`.github/workflows/cross-platform.yml`, both Tauri conf overlays, and
`app/src-tauri/Cargo.toml` are scanned for tauri/cargo invocations carrying an
NDI feature flag (`--features …ndi…`, `--features=ndi`, `-F ndi`); (4) at
bundle time (`beforeBundleCommand`) the ancestor process command-line chain is
audited so a direct CLI `tauri/cargo … --features ndi` bypass fails closed.
Any future NDI distribution requires the separately licensed overlay proof
already described above; extending this contract requires its own review.

## Acceptance

1. Install version N from the same channel and signing key.
2. Publish a signed N+1 test artifact and manifest.
3. Start N and confirm Project -> Application Updates announces N+1.
4. Start with an unsaved edit, install N+1, and confirm a `before update N+1`
   project backup exists before the installer runs.
5. Confirm a modified artifact, wrong signature, wrong public key, HTTP
   endpoint, channel mismatch, and changed version between check/install are
   all rejected.
6. Confirm Windows Authenticode and macOS signing/notarization independently;
   updater minisign verification does not replace platform code signing.
7. Retain the signed version N installer before publishing N+1. After the N+1
   acceptance run, reinstall N, restore the automatic `before update N+1`
   project backup, and confirm both the application version and restored
   project hash. Record this as the supported rollback procedure; replacing an
   executable without restoring the matching project backup is not a pass.
8. Return the machine to N+1 through the signed production channel and confirm
   the same project opens again. Attach the N installer hash, backup hash,
   restored-project hash, version screenshots, and install/update logs.

The public release endpoint, signing key custody, Authenticode certificate, and
Apple notarization credentials are external release-operations inputs and are
not stored in this repository.
