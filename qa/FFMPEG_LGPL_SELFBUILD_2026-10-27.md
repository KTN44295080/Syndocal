# Task 56 - FFmpeg LGPL self-build candidate

Original checkout `main`, initial base
`512f0a7e0d7aab229c76e673f0bfe1a2db665819`.
Ownership: `.github/workflows/ffmpeg-lgpl-sdk.yml`, `tools/ffmpeg-sdk/**` and
this note. Concurrent UI/checker/video-test changes are preserved. No product
runtime, pin, notices, release/tag or distribution changes are made at setup.

The manual-only workflow fixes recipe
`6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b` and FFmpeg source
`29e619e767cde9045a75c29bc9a8278ae7b3a98b`, with source archive SHA-256
`8e4699bcb6a311772e8f513cc72ce27328891b82a598b82e7cff3dfe580875a0`.
Chromaprint/FFTW recipes and the final Chromaprint edge are removed in the
committed patch. Toolchain/download base images are pinned by digest. The
previous mutable rav1e cc update is replaced by a complete checksum-pinned
lock and locked build, with a retained lock equality check.

Setup checks: patch applies cleanly to the retained exact upstream source;
Bash syntax passes for build/inspection and patched upstream scripts;
`actionlint` passes the new workflow; independent stable-diff review finds no
dispatch blocker. These are not remote build, binary, native or GPU evidence.

Candidate execution, local binary acceptance and regression results will be
recorded below. Source/tag/build timestamp and complete license/source closure
limitations are documented in `tools/ffmpeg-sdk/README.md`. Durable SDK hosting
remains an owner decision; the 90-day workflow artifact cannot serve as the
product pin's durable download endpoint.
