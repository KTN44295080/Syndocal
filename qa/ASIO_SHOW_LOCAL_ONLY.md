# Windows Show ASIO local-only artifact

## Authority and purpose

This path exists only to run the imminent show on the Windows workstation that
builds it. It is a separate, unbundled, same-host artifact. It does not change
or relax the normal MIT/WASAPI installer, updater, or release artifact. The
normal Tauri default features remain exactly `libav` and `spout`, and the normal
ASIO packaging rejection remains authoritative.

The local artifact is always `distributionApproved: false`. It must not be
installed, signed, archived, copied to another machine or checkout path,
uploaded, published, attached to a release, served by the updater, or combined
with NDI. Resolving ASIO distribution licensing is a separate product decision;
this route is not that decision and contains no fallback that pretends it is.

## Exact clean-break layout

- Application Cargo target: `target/show-asio-build/app`
- Bridge Cargo target: `target/show-asio-build/bridge`
- Final directory:
  `target/show-asio-local/Syndocal_Show_ASIO_<version>_<commit12>_x64`
- Final executable: `syndocal-show-asio.exe`
- Bridge: `syndocal_asio_bridge.dll`, ABI v2, exact nine v2 exports
- Feature union: `libav,spout,show-asio`
- Manifest: `show-asio-local-manifest.json`, schema v1

The final directory is never overwritten. A pre-existing directory is a hard
failure and must be investigated; it is not deleted or reused automatically.
The normal `target/release` directory is never a source or destination of this
route.

The manifest binds the artifact to the Windows MachineGuid and the canonical
checkout path using a SHA-256 digest. It also records the exact source-file
identity, version, full Git commit, feature union, target directories, runtime
hashes, PE identities, bridge ABI/exports, and fixed prohibitions. The checker
must pass immediately after manifest creation and again immediately before use.
Any changed, missing, extra, linked, reparse-backed, ambiguous, or unverified
entry fails closed.

The source identity contains each trusted first-party helper in the complete
local import closure, not a directory hash: `check-release-metadata.mjs`,
`prepare-release-runtime.mjs`, `run-tauri.mjs`, `strict-json.mjs`, and
`windows-runtime-inventory.mjs`. A post-build change to any one of those helpers
invalidates the artifact before runtime payload inspection or acceptance.

## Commands

Read-only plan (never invokes Cargo, Tauri, or process termination):

```powershell
node app/scripts/build-windows-show-asio.mjs --plan
```

Deterministic hostile self-tests (temporary fixtures only; never invokes Cargo,
Tauri, or process termination):

```powershell
node app/scripts/check-show-asio-artifact.mjs --self-test
node app/scripts/prepare-show-asio-runtime.mjs --self-test
node app/scripts/build-windows-show-asio.mjs --self-test
```

Actual local build, only after every prerequisite below is satisfied:

```powershell
node app/scripts/build-windows-show-asio.mjs
```

Verification immediately before local use:

```powershell
node app/scripts/check-show-asio-artifact.mjs
```

The actual build accepts no feature, target, bundle, signing, output, archive,
or publication arguments. It initializes and verifies the exact MSVC 14.44 x64
toolchain through the existing Tauri wrapper primitives before each Cargo
entry, applies the standard exact-path `target/release/syndocal.exe` process
gate immediately before each native release build, passes `--locked` to both
Cargo runners, runs the existing ASIO SDK provenance preflight, uses the two
isolated target directories, stages only the exact pinned files, writes the
manifest last, and performs post-manifest verification. The process gate does
not make normal `target/release` an input or output of this artifact.

## Prerequisites and typed block

The actual route requires all of the following, with no guessing:

- Windows x64 on the same host and canonical checkout path;
- a completely clean worktree, including no untracked files;
- `HEAD` exactly equal to its configured upstream;
- synchronized Cargo, lockfile, frontend, and Tauri product versions;
- exact MSVC 14.44 x64 linker pin and `where.exe link.exe` first resolution;
- explicit `FFMPEG_DIR`, `CPAL_ASIO_DIR`,
  `SYNDOCAL_ASIO_SDK_ARCHIVE_PATH`, and `LIBCLANG_PATH`;
- the pinned FFmpeg seven-DLL inventory with no missing, extra, mutated,
  hard-linked, symlinked, or reparse-backed entry;
- the pinned ASIO SDK provenance and one exact bridge DLL with ABI v2, exact
  exports, AMD64 PE32+ DLL identity, and unchanged build hash;
- application integration feature `show-asio = ["asio"]`, while
  `default = ["libav", "spout"]` remains unchanged.

The earlier `SHOW_ASIO_FEATURE_MISSING` block is resolved:
`app/src-tauri/Cargo.toml` now defines the exact `show-asio = ["asio"]` feature
while the normal defaults remain unchanged. No real Show-ASIO artifact has been
built yet. The current observed route remains blocked before Cargo by the
missing explicit `FFMPEG_DIR` and by the requirement for a completely clean,
pushed worktree whose `HEAD` exactly equals its upstream. These are hard
preconditions, not fallback invitations.

## Unsupported cases

Installers, updater payloads, signatures, archives, copied artifacts, network
shares, removable/alternate checkout paths, NDI, additional Cargo features,
future or legacy manifests, dirty/unpushed commits, ordinary release-directory
reuse, SDK downloads, default-device guessing, and post-manifest mutation are
deliberately unsupported. There is no migration or legacy compatibility path.

The tracked JSON Schema is the exact structural manifest mirror and is executed
by the checker. The same checker additionally enforces role-specific PE bit
semantics and the independently pinned FFmpeg inventory, which JSON Schema
cannot express. Any runtime-inventory change therefore requires a same-tranche
schema, checker, fixture, and documentation update; silent inventory expansion
is intentionally impossible.

`FFMPEG_DIR/bin` may also contain ordinary non-DLL tools such as `ffmpeg.exe`
and `ffprobe.exe`; they are neither runtime candidates nor staged files. The
exactness rule applies to the DLL inventory: all seven pinned DLLs must exist
and any eighth DLL is rejected. The final artifact tree remains an exact
14-file manifest set, so no non-DLL tool or other stray input can propagate.
