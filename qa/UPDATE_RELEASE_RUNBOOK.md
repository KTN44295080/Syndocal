# Signed Application Update Runbook

Updated: 2026-07-13

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
