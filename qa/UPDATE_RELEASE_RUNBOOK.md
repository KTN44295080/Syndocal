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

The proof uses `app/scripts/windows-candidate-extractor.mjs`, a repository-owned
deterministic extractor, and its revalidated/transient `inventory.json` (it is
not a cryptographic signature or an immutable release record). The checker then
re-reads the exact installer identities, verifies the three extracted roots, and
rejects canonical ASIO bridge names, retired names, third-party ASIO names,
known bridge hashes under any filename, and every FFmpeg name/size/SHA-256/PE
mismatch. No installer is executed as part of this proof. The only candidate
executable diagnostic (`syndocal.exe --print-updater-release-identity`) is
reachable only after the updater payload, its exact adjacent `.sig`, the
base64-wrapped minisign public key, and the updater manifest have passed the
same Ed25519 verification used by the RC evidence gate.

Run the release build first, then create the candidate inventory from the exact
versioned Tauri outputs. The output must be a new directory below the
repository-owned `target/qa/windows-release-candidate` tree:

```powershell
$version = "1.2.0-rc.1"
$nsis = (Resolve-Path "target/release/bundle/nsis/Syndocal_${version}_x64-setup.exe").Path
$msi = (Resolve-Path "target/release/bundle/msi/Syndocal_${version}_x64_ja-JP.msi").Path
# createUpdaterArtifacts=true emits a direct signed installer; use that exact
# NSIS or MSI payload for the updater role.
$updater = $nsis
$out = Join-Path (Resolve-Path .).Path "target/qa/windows-release-candidate/$version"
# Required crypto preflight: do this before any archive extraction can make an
# extracted syndocal.exe available to the candidate inspector.
pnpm --dir app run check:release
node app/scripts/check-release-metadata.mjs `
  --release-candidate --manifest qa/release/release-evidence.json
node app/scripts/windows-candidate-extractor.mjs `
  --nsis $nsis --msi $msi --updater $updater `
  --product-version $version --output $out
$env:SYNDOCAL_WINDOWS_ARTIFACT_INVENTORY = Join-Path $out "inventory.json"
node app/scripts/check-windows-release-artifacts.mjs --require-candidate-extraction
```

This is the only acceptance order: static release checks, RC metadata/key/
manifest/adjacent-signature/payload Ed25519 preflight, deterministic candidate
extraction, then the candidate artifact gate. `pnpm --dir app run
check:release:candidate` is the same safe verification tail (static checks,
metadata crypto preflight, candidate gate) when `SYNDOCAL_WINDOWS_ARTIFACT_
INVENTORY` already identifies the just-created inventory. Do not run the
extracted `syndocal.exe` diagnostic manually before that command completes, and
never run either installer as a substitute for extraction evidence.

NSIS and updater ZIP payloads require an explicitly trusted 7-Zip executable
(`SYNDOCAL_WINDOWS_7ZIP_PATH`, or the reviewed `Program Files\7-Zip\7z.exe`
location; on the current Windows host this is `C:\Program Files\7-Zip\7z.exe`);
PATH lookup and `Expand-Archive` are deliberately unsupported. The current
Tauri v2 updater configuration (`bundle.createUpdaterArtifacts=true`) accepts
only the direct signed NSIS/MSI payload with its adjacent `.sig`; legacy
`.nsis.zip`/`.msi.zip` forms are accepted only when the config explicitly uses
`v1Compatible`. The adjacent signature, release-evidence updater manifest,
canonical decoded HTTPS channel/filename URL, normalized public-key fingerprint,
and extracted executable identity must all match the same evidence manifest.
MSI extraction uses only the exact `%SystemRoot%\System32\msiexec.exe` path.
The extractor validates archive entry paths, rejects symlinks/reparse points
and hard links, verifies archive listing-to-tree correspondence, materializes
each tool input from verified bytes using a fresh `wx` file, rehashes it before
and after tool use, and removes that input through the revalidated tombstone
path before publication. It publishes only the three approved role directories
plus the inventory through a same-volume pathname rename and binds every input
artifact SHA-256 across listing, extraction, and publication. Cleanup removes
only its revalidated temporary staging directory after moving it to a
same-parent tombstone. The tombstone's parent, directory identity, and reparse
status are rechecked; on any mismatch or cleanup failure it is visibly left in
place rather than recursively deleted.
A pre-existing or swapped final output is never recursively deleted. A
missing tool, future/non-RC version, stale artifact, or hostile path remains a
visible fail-closed error. Run `node app/scripts/windows-candidate-extractor.mjs
--self-test` before the release checker self-test.

The pathname rename/tombstone fence is not handle-atomic in Node, and Node does
not provide a portable proof of the Windows owner/DACL of the temporary
directory. This remains a visible P1 operational boundary: use only the
repository-owned, fresh, operator-controlled single-writer staging/output tree;
do not treat this script as an ACL or hostile-concurrent-writer guarantee.

The alpha/prerelease release gate remains fail-closed: the candidate inventory
requires an explicit `X.Y.Z-rc.N` version and cannot be used to turn an alpha
artifact into a release candidate. The older CI smoke extraction is diagnostic
only; it is not a substitute for this inventory proof.

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
