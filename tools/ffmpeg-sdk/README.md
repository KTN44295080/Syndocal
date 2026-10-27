# FFmpeg LGPL SDK candidate build

Run the manual-only `FFmpeg LGPL SDK candidate` GitHub Actions workflow on main.
It never publishes a release, tag, container, installer or product update.
The public GitHub-hosted Ubuntu runner builds the patched BtbN recipe; local
Docker/WSL installation and paid runners are not required.

`pin.json` fixes the upstream recipe, FFmpeg source/archive hash and two
container digests. The committed patch removes Chromaprint and FFTW recipes,
removes the final Chromaprint dependency, fixes the source checkout/date and
container inputs, and skips disabled variant downloads. Optional Chromaprint
is absent rather than emitting `--disable-chromaprint`, because the candidate
acceptance rejects **all** Chromaprint/FFTW strings in the seven runtime DLLs.

The original rav1e recipe performs an unversioned `cargo update cc`. Its
replacement overlays the complete reviewed `rav1e.Cargo.lock` and uses
`cargo cinstall --locked`. The lock was resolved from the exact rav1e source
with `cargo update -p cc --precise 1.10.0`; relative to its original lock,
cc 1.2.26 becomes 1.10.0, find-msvc-tools 0.1.15 and shlex 2.0.1 are added.
The built dependency image retains a copy, which must equal the committed lock.

The build uses `win64 lgpl-shared 8.1` and rebuilds the enabled dependencies.
Only the two digest-pinned toolchain/download base images are reused. It prints
the configure line, license and muxers using Wine, scans all seven DLLs, and
retains SDK ZIP, exact FFmpeg source archive, hashes, patch, lock, Dockerfile,
image/toolchain receipts and verbose build log for 90 days. Failure diagnostics
are uploaded too; an artifact alone is not proof of a successful candidate.

The fixed source/recipe/environment inputs improve repeatability. Byte-identical
rebuilds have not been demonstrated: ZIP timestamps, dependency build timestamps
and upstream recipe references to named source tags still require provenance
review. The digest-pinned base image is not independently reconstructed here.
This is not complete corresponding-source/license closure or publication approval.

Download only this workflow's run, verify ZIP/DLL identities and capabilities
against the current pin with `verify-candidate.py` (separate Python processes
per DLL set; it also runs the Task 53 framecrc matrix), then run isolated
Windows regressions before adoption. Workflow artifacts expire and are not an
approved durable SDK download URL.

Run 40366564270 (commit `7d0af757`) produced the adopted candidate, archive
SHA-256 `81587fa070a9fa2da3d8ff3e63beb9654729782bc8e9899ee951baf5fc5d9f1a`.
Its build and DLL scan passed; the step failed only on the literal `LGPL`
grep, fixed in `inspect-candidate.sh`. That exact ZIP is hosted with its
source, patch, provenance and `SHA256SUMS` as the build-dependency prerelease
`ffmpeg-sdk-lgpl-n8.1.3-r40366564270` (tag on `7d0af757`). The inventory,
resolver, notices, build documentation and CI download now name it. Rebuilds
are not byte-identical (rerun 40377168832 differs), so never substitute a
rebuild for the hosted archive. CI keeps a release-packaging guard that rejects
the old BtbN Chromaprint `avformat-62.dll`.

## Corresponding source

The manual-only `FFmpeg LGPL SDK corresponding source` workflow
(`ffmpeg-lgpl-sdk-source.yml`) leaves the build job untouched. It re-runs the
patched recipe's `download.sh`/`generate.sh` for `win64 lgpl-shared 8.1` with
the digest-pinned download image and requires the regenerated Dockerfile to be
byte-identical to the `dependency.Dockerfile` hosted for run 40366564270. Every
download stage that Dockerfile mounts is extracted under `dependencies/<stage>`.
The sources the build fetched itself are added: freetype's `dlg` and kvazaar's
`greatest` submodules, dav1d's `checkasm` wrap, and the rav1e crates. The crates
are vendored with the build image's cargo from the committed lock.

`pack-corresponding-source.sh` records every resolved Git commit and then
removes `.git`/`.svn`. It checks each commit and submodule printed by the hosted
build log against the resolved trees. Moving tags are proved only by their
abbreviated commit prefix. It then writes a sorted, fixed-mtime, root-owned GNU
tar compressed with `xz -6 -T4 --block-size=64MiB`. A second tar pass must
reproduce the uncompressed tar SHA-256. `corresponding-source-manifest.py`
writes the per-component manifest, with refs, resolved commits, build-log
evidence, licence summary (`component-licenses.json`), licence files and tree
digest. `verify-corresponding-source.py` re-verifies a downloaded upload set
independently. Toolchain runtime sources (GCC/MinGW runtime, Rust std) and the
base images themselves are identified, not preserved.
