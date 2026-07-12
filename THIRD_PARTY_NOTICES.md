# Third-party notices

Syndocal dynamically links FFmpeg 8.1 shared libraries for H.264, H.265, and
ProRes decoding. Release bundles use an LGPLv3-compatible FFmpeg build with GPL
components disabled.

- Project: FFmpeg
- Source: https://ffmpeg.org/
- License: GNU Lesser General Public License version 3
- License text: `licenses/FFmpeg-LGPL-3.0.txt`

FFmpeg is not part of the Syndocal MIT-licensed source code. Recipients may
replace the dynamically linked FFmpeg libraries with a compatible build, as
permitted by the LGPL.

Windows x86_64 builds include Spout2 2.007.017 through the `spout2-rs` static
binding for local inter-application video texture transport.

- Project: Spout2
- Source: https://github.com/leadedge/Spout2
- Rust binding: https://crates.io/crates/spout2-rs
- License: BSD 2-Clause
- License text: `licenses/Spout2-BSD-2-Clause.txt`

Spout2 is linked statically and does not require a separate runtime DLL.

Syndocal uses `bcdec_rs` 0.2.0 to provide a safe, pure-Rust BC7 software
fallback for HAP R frames when direct GPU BC texture sampling is unavailable or
the frame must pass through a CPU-visible effect path.

- Project: bcdec_rs
- Source: https://github.com/ScanMountGoat/image_dds/tree/main/bcdec_rs
- License: MIT
- License text: `licenses/bcdec_rs-MIT.txt`
