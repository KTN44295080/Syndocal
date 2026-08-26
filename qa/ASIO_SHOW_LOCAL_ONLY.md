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

The earlier `SHOW_ASIO_FEATURE_MISSING` and pre-build prerequisite blocks were
resolved for exact committed/pushed checkpoint
`ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae`.
`app/src-tauri/Cargo.toml` defines the exact `show-asio = ["asio"]` feature while
the normal defaults remain unchanged. That checkpoint built and
manifest-verified
`target/show-asio-local/Syndocal_Show_ASIO_1.2.0-alpha.12_ff61a6dec6eb_x64`.
Its application `syndocal-show-asio.exe` is 58,637,824 bytes, SHA-256
`1D313900AB94A2429BF784B7D4CCA8E8EC39FBF17E11CB257D76A19656AA2F8D`;
its ABI-v2 `syndocal_asio_bridge.dll` is 813,568 bytes, SHA-256
`40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`;
and `show-asio-local-manifest.json` has SHA-256
`DCDFA0D381C851483D9E206637E803920ADDD6CC5B0C313780604FFC1D0EAAC4`.
The manifest records `distributionApproved: false`, `sameHostOnly: true`, and
`unbundled: true`. This closes the exact build/manifest checkpoint only. The
dedicated checker must still pass immediately before use, and current physical
native/operator, driver/recovery, formal matched 48 kHz, and measured-latency
acceptance remain open. No installer, updater, copy, archive, or public
distribution is approved.

## Alpha.14 same-host physical evidence (2026-08-26 JST)

The current local-only artifact is bound to source commit
`6b4cd1afb4d228158d04a15dbe3e4a73c922baeb`:
`target/show-asio-local/Syndocal_Show_ASIO_1.2.0-alpha.14_6b4cd1afb4d2_x64`.
Its application SHA-256 is
`CC2D1E28B9063250E86106F04DD082A5C860A840EB721C8208EEFEE009BD0599`; its
bridge SHA-256 is `40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`;
and its manifest SHA-256 is
`BBEA830122B999A7F985A8F0E88330361E74D3EEE9C5DF2E982EB932A4B687DD`.
The dedicated checker passed with `files=14` and `distributionApproved=false`.
The official `node app/scripts/build-windows-show-asio.mjs` route passed with
the exact VS Community 14.44 x64 linker pinned and first, `check:release` PASS,
and first-party build warnings `0`.

The explicit `HOTONE AUDIO USB Audio Device` opened ASIO native `i32`, two
channels, 44.1 kHz, fixed requested/applied `128f`. First run telemetry was
callback `128/128/128`, `OVR 0/0f`, `XRUN 0`, capture-to-worker `4.4/4.5 ms`,
and `Q 0/1/4`; Start held five seconds stable, then Stop had no pending work and
Close left application/bridge process counts `0`. Restart restored the persisted
ASIO selection as `ASIO RESTORED`, locked and not auto-started. Explicit Start
then reported `ASIO REVALIDATED` and ACTIVE with the same configuration;
callback/overrun/XRUN/queue were unchanged, capture-to-worker was `4.4/4.7 ms`,
and final Stop/Close again left both process counts `0`.

This is not unplug, XRUN/fault, TOPPING, long-duration, matrix, or latency
threshold evidence. The artifact remains `distributionApproved: false`. After
a documentation commit changes HEAD, reuse requires a clean checkout detached
at source `6b4cd1a`, or a rebuild from the new HEAD; no cross-source reuse is
approved.

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
