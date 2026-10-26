# Third-party notices

Syndocal dynamically links FFmpeg shared libraries for video decoding, including
H.264, H.265, HAP, DXV, ProRes, VP9 and AV1. The normal Windows variant uses
FFmpeg `n8.1.3-9-g29e619e767`, self-built by Syndocal's GitHub Actions workflow
from the BtbN FFmpeg-Builds recipe with the Chromaprint and FFTW3 recipes
removed (inventory version `n8.1.3-9-g29e619e767-20260930-syndocal-r40366564270`;
the DLLs self-report `n8.1.3-9-g29e619e767-20260930`), reporting LGPL version 3
or later. Its measured configure line enables shared libraries and version 3
licensing, and contains neither `--enable-gpl`, `--enable-nonfree` nor
`--enable-chromaprint`; none of the seven DLLs contains a Chromaprint or FFTW
string. The upstream LGPL self-description is not a completed license audit.
Publication is blocked until the exact SDK archive has durable hosting and the
actual build inputs/linkage and resulting license obligations are reconciled;
the collected notices do not grant release clearance or establish exact binary
dependency closure.

- Project: FFmpeg
- Copyright (c) 2000-2026 the FFmpeg developers
- License: GNU Lesser General Public License version 3 or later
- License texts: `licenses/FFmpeg-LGPL-3.0.txt` and `licenses/GPL-3.0.txt`
  (the LGPLv3 incorporates the GPLv3 terms)
- Dependency license texts and source provenance: `licenses/FFmpeg-DEPENDENCIES.txt`
- Exact Windows binary archive: `ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1.zip`
  inside workflow artifact https://github.com/KTN44295080/Syndocal/actions/runs/40366564270/artifacts/12674399245
  (interim, expires 2027-01-24; durable hosting is pending)
- Archive SHA-256: `81587fa070a9fa2da3d8ff3e63beb9654729782bc8e9899ee951baf5fc5d9f1a`
- Exact FFmpeg source: https://github.com/FFmpeg/FFmpeg/archive/29e619e767cde9045a75c29bc9a8278ae7b3a98b.tar.gz
- Build recipes, dependency source revisions and patches: https://github.com/BtbN/FFmpeg-Builds/tree/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b
  with the Syndocal patch, lock and workflow at https://github.com/KTN44295080/Syndocal/tree/7d0af757f73231fc0c6006d6c21c72c14cb4a31a/tools/ffmpeg-sdk
- Full measured configuration and per-DLL identities:
  `qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` in the Syndocal source distribution

FFmpeg includes code derived from the Independent JPEG Group's software.
FFmpeg and its dependencies retain their respective copyrights and licenses;
their collected license texts and notices are reproduced in
`licenses/FFmpeg-DEPENDENCIES.txt`, with the exact-binary provenance limitations
identified there. Syndocal
makes no modifications to these supplied DLLs. Source and dependency archives,
patches and applicable notices must accompany the final distribution through
its corresponding-source download; the locators above are provenance and do
not claim that this release-publication gate has been completed.

FFmpeg is not part of the Syndocal MIT-licensed source code. Recipients may
replace the dynamically linked FFmpeg libraries with a compatible build, as
permitted by the LGPL. The normal decode payload contains seven DLLs and no
recording encoder executable. A separately supplied recording encoder and the
optional ASIO bridge have their own distribution requirements.

Windows x86_64 builds include Spout2 2.007.017 through the `spout2-rs` static
binding for local inter-application video texture transport.

- Project: Spout2
- Source: https://github.com/leadedge/Spout2
- Rust binding: https://crates.io/crates/spout2-rs
- License: BSD 2-Clause
- License text: `licenses/Spout2-BSD-2-Clause.txt`

Spout2 is linked statically and does not require a separate runtime DLL.

macOS builds with the default `syphon` feature dynamically link the official
Syphon.framework through Syndocal's Objective-C/Metal output bridge.

- Project: Syphon Framework
- Source: https://github.com/Syphon/Syphon-Framework
- Pinned revision: `f4761677a45b8034a3c2069ec0f3d2553da81fba`
- Archive SHA-256: `a3766088c46588c42b087f4e5ff7ae7bb0f83292af59a0aeb8bba69eba5311c5`
- Licences: upstream BSD 3-Clause distribution licence and BSD 2-Clause
  source-header notices, including the Metal contributor notices
- Complete licence texts: `licenses/Syphon-BSD.txt`

`app/scripts/prepare-syphon.sh` verifies the pinned source archive and builds the
framework with Xcode. The macOS application bundles the framework, its Metal
shader library, and the licence texts. No Syphon binary is used on Windows or
Linux. Syphon input is not implemented.

Syndocal uses `bcdec_rs` 0.2.0 to provide a safe, pure-Rust BC7 software
fallback for HAP R frames when direct GPU BC texture sampling is unavailable or
the frame must pass through a CPU-visible effect path.

- Project: bcdec_rs
- Source: https://github.com/ScanMountGoat/image_dds/tree/main/bcdec_rs
- License: MIT
- License text: `licenses/bcdec_rs-MIT.txt`

## Open Fixture Library (bundled fixture profiles)

Syndocal bundles a converted snapshot of the Open Fixture Library so that
manufacturer fixture profiles are available offline, with no account or
download, on first launch.

- Source: https://github.com/OpenLightingProject/open-fixture-library
- Snapshot revision: `c08598f45ce5e7cff4089a65bb67b5ba2777ee32` (recorded in the
  bundle's `sourceRevision` field; regenerate with
  `node scripts/build-ofl-library.mjs <checkout> <sha>`)
- License: MIT
- Copyright (c) 2017 Florian & Felix Edelmann and OFL contributors
- License text: `licenses/open-fixture-library-MIT.txt`

The bundled data is a lossy conversion: each mode is reduced to its ordered
DMX attribute layout (the same representation Syndocal uses for its own custom
profiles). Matrix channel insert blocks are expanded before conversion.
Capability ranges, wheel slot media, and physical data are not carried over -
operators who need the full definition download the manufacturer's GDTF from
GDTF Share.

## Q Light Controller Plus fixture definitions (bundled fixture profiles)

Syndocal also bundles a converted snapshot of QLC+'s fixture definitions as a
supplemental offline source. Open Fixture Library has higher precedence: exact
normalized manufacturer/model conflicts are omitted from the QLC+ bundle and
recorded in its conversion audit.

- Source: https://github.com/mcallegari/qlcplus
- Snapshot revision: `18cf9da9934e5ea0b5026dd43b0f48be1e5b9550` (recorded in the
  bundle's `sourceRevision` field; regenerate after the OFL bundle with
  `node scripts/build-qlc-library.mjs <checkout> <sha>`)
- License: Apache-2.0
- Copyright QLC+ contributors and fixture definition authors
- License text: `licenses/qlcplus-Apache-2.0.txt`

This is the same lossy ordered-DMX-layout conversion described above. Adjacent
coarse/fine pairs are represented as 16-bit controls; unrecognized channel
semantics remain addressable as numbered generic controls so the source mode's
DMX footprint is preserved exactly.
