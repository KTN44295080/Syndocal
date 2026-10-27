# FFmpeg license alignment - Task 53 - 2026-10-26

Current status (Task 54, 2026-10-27): **publication blocked**. The pinned
Chromaprint recipe selects GPL FFTW3; the DLL LGPL self-description is not
proof of the transitive license boundary. Exact Rust build lockfiles are also
unavailable. See the Task 54 checkpoint below.

Task 53 historical status: pin switched to the BtbN LGPL 8.1 shared build and committed. Focused
decode regressions pass on an isolated `git archive HEAD` copy built against the
new SDK; real D3D11VA cases pass on the local GPU; the notice/inventory checks
pass. The release-wide gates that still fail (`check-release-metadata`,
`check:ai0-source-coverage`) fail identically on pristine HEAD and are owned by
other lanes (see "Notice / inventory / checks"). No encoder or installer was
bundled or published. Normal source stays MIT with LGPL decode DLLs; ASIO
remains a separate boundary. This does not close DIST-NOTICES-001 (see
"Source / recording remaining work").

Work split: Codex (Task 53) gathered the evidence, edited the seven owned files,
ran the first regression pass on the shared tree and the isolated native build,
then stopped at its usage limit. Claude completed the review, reran the
regressions on an isolated HEAD copy, added the real-GPU and decode-equivalence
evidence, reran the checks on three trees and committed.

## Ownership and provenance

Original checkout, main; initial base `059277a744762b6c8a7e50d564f293969162b949`.
Observed concurrent main during validation: `d471cfdac610c1a4586488a21a1d1d15378301cc`. UI and 60fps lanes
are active; this tree is not a frozen release candidate. Only the eight task-owned
files were edited. Other dirty files and normal target/release exe/DLLs are
preserved. Local ignored evidence: `target/qa/ffmpeg-lgpl-20261026/`.

Probe loads each build in a separate Python process using the exact DLL path,
calls avcodec_configuration(), avutil_license(), codec iteration and per-codec
hardware configurations. Old DLL hashes match the original inventory. Both
archives were fetched/hashed; old ffmpeg.exe is from that exact Gyan archive and
runs against matching old DLLs. Both CLIs' -version/-buildconf/-L/-decoders/
-hwaccels exited 0.

| Evidence | Old Gyan | Candidate BtbN |
| --- | --- | --- |
| Version | `8.1.2-full_build-www.gyan.dev` | `n8.1.3-9-g29e619e767-20260930` |
| Actual DLL license | GPL version 3 or later | LGPL version 3 or later |
| --enable-gpl | Present | Absent |
| --enable-nonfree | Absent | Absent |
| --enable-version3 / --enable-shared | Both present | Both present |
| Archive bytes | 105925481 | 80733362 |
| Archive SHA-256 | `274923c68904a9b76c73b908f57923dafba81155856cd742138515ded570d066` | `3e47bda1607740550141e37c0e49d1e5182b34699f15adfd137ee266d346811a` |

Old archive: <https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-full_build-shared.zip>.
Candidate: <https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08/ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1.zip>.

Old measured configure:

```text
--enable-gpl --enable-version3 --enable-shared --disable-w32threads --disable-autodetect --enable-cairo --enable-fontconfig --enable-iconv --enable-gnutls --enable-lcms2 --enable-libxml2 --enable-gmp --enable-bzlib --enable-lzma --enable-libsnappy --enable-zlib --enable-librist --enable-libsrt --enable-libssh --enable-libzmq --enable-avisynth --enable-libbluray --enable-libcaca --enable-libdvdnav --enable-libdvdread --enable-sdl2 --enable-libaribb24 --enable-libaribcaption --enable-libdav1d --enable-libdavs2 --enable-libopenjpeg --enable-libquirc --enable-libuavs3d --enable-libxevd --enable-libzvbi --enable-liboapv --enable-libqrencode --enable-librav1e --enable-libsvtav1 --enable-libvvenc --enable-libwebp --enable-libx264 --enable-libx265 --enable-libxavs2 --enable-libxeve --enable-libxvid --enable-libaom --enable-libjxl --enable-libsvtjpegxs --enable-libvpx --enable-mediafoundation --enable-libass --enable-frei0r --enable-libfreetype --enable-libfribidi --enable-libharfbuzz --enable-liblensfun --enable-libvidstab --enable-libvmaf --enable-libzimg --enable-amf --enable-cuda-llvm --enable-cuvid --enable-dxva2 --enable-d3d11va --enable-d3d12va --enable-ffnvcodec --enable-libvpl --enable-nvdec --enable-nvenc --enable-vaapi --enable-libshaderc --enable-vulkan --enable-libplacebo --enable-opencl --enable-libcdio --enable-openal --enable-libgme --enable-libmodplug --enable-libopenmpt --enable-libopencore-amrwb --enable-libmp3lame --enable-libshine --enable-libtheora --enable-libtwolame --enable-libvo-amrwbenc --enable-libcodec2 --enable-libilbc --enable-libgsm --enable-liblc3 --enable-libopencore-amrnb --enable-libopus --enable-libspeex --enable-libvorbis --enable-ladspa --enable-libbs2b --enable-libflite --enable-libmysofa --enable-librubberband --enable-libsoxr --enable-chromaprint --enable-whisper
```

Candidate measured configure (also schema 2 anchored):

```text
--prefix=/ffbuild/prefix --pkg-config-flags=--static --pkg-config=pkg-config --cross-prefix=x86_64-w64-mingw32- --arch=x86_64 --target-os=mingw32 --enable-version3 --disable-debug --enable-shared --disable-static --disable-w32threads --enable-pthreads --enable-iconv --enable-zlib --enable-libxml2 --enable-libvmaf --enable-fontconfig --enable-libharfbuzz --enable-libfreetype --enable-libfribidi --enable-vulkan --enable-libshaderc --enable-libdav1d --enable-libvorbis --enable-librav1e --enable-librsvg --disable-libxcb --disable-xlib --disable-libpulse --enable-gmp --enable-lzma --enable-liblcevc-dec --enable-opencl --enable-amf --enable-libaom --disable-avisynth --enable-chromaprint --disable-libdavs2 --disable-libdvdread --disable-libdvdnav --disable-libfdk-aac --enable-ffnvcodec --enable-cuda-llvm --disable-frei0r --enable-libgme --enable-libkvazaar --enable-libaribb24 --enable-libaribcaption --enable-libass --enable-libbluray --enable-libjxl --enable-libmp3lame --enable-libopus --enable-libplacebo --enable-librist --enable-libssh --enable-libtheora --enable-libvpx --enable-libwebp --enable-libzmq --enable-lv2 --enable-libvpl --enable-openal --enable-liboapv --enable-libopencore-amrnb --enable-libopencore-amrwb --enable-libopenh264 --enable-libopenjpeg --enable-libopenmpt --disable-librubberband --enable-schannel --enable-sdl2 --enable-libsnappy --enable-libsoxr --enable-libsrt --enable-libsvtav1 --enable-libtwolame --enable-libuavs3d --disable-libdrm --enable-vaapi --disable-libvidstab --enable-libvvenc --disable-whisper --disable-libx264 --disable-libx265 --disable-libxavs2 --disable-libxvid --enable-libzimg --enable-libzvbi --extra-cflags=-DLIBTWOLAME_STATIC --extra-cxxflags= --extra-libs=-lgomp --extra-ldflags=-pthread --extra-ldexeflags= --cc=x86_64-w64-mingw32-gcc --cxx=x86_64-w64-mingw32-g++ --ar=x86_64-w64-mingw32-gcc-ar --ranlib=x86_64-w64-mingw32-gcc-ranlib --nm=x86_64-w64-mingw32-gcc-nm --extra-version=20260930
```

All seven old/new PE identities: PE32+ AMD64, signature PE\0\0,
machine 34404, characteristics 8750, optional magic 523.

| DLL | Old bytes / SHA-256 | Candidate bytes / SHA-256 |
| --- | --- | --- |
| `avcodec-62.dll` | 97454080 / `34f5b1baac01c4be3edf464309c79db05ffbd4a9c905c94b4a4651cd15370296` | 90992128 / `4f817870cfaeaced3e34a083c50a1b4be0389202d7fb821632d47cf9b3f4d82a` |
| `avdevice-62.dll` | 6323200 / `d213d6cad9f3a526f7664ebb3f93d6882669540db7164daecd03f52e0f5288cc` | 4947456 / `34bc695cfdd3106de29551c6868f1c2cdc0fa36903494506f1708251e6a3075e` |
| `avfilter-11.dll` | 124344320 / `e318cac83d648869180d0b57c45f21aad1e4db34a467000c157da2938ff7f63f` | 31212544 / `28e202431dc0643af97c307dc6437da13f8c08aa016fc28d1f9ec8fd0336e172` |
| `avformat-62.dll` | 20179968 / `c04e6ed2f9f36d42325d4f4df5babb5d6ce7c55dbffeb7ef1007e25e97bcb716` | 22722560 / `bcf514a394dc5ec43fe64e4f1624ba63b23e9c25967081ea2d03d95b0dc359ef` |
| `avutil-60.dll` | 3148288 / `6f172b5d10224fcc3f729c8baa6fd36a97bb58042bb3b1417078d77d2da59b87` | 3014144 / `481620c7b5cd8e831a50762fae55699c821091e96c42226d04fee725a4aae2f5` |
| `swresample-6.dll` | 486912 / `72e2721672c11fd37d983b05cc2370f612784e4e3218362a0cb4315d08c917fb` | 734720 / `3baad8ac0c8f0050cfd5742f5104603a27127397521409bf7d1742e6d02f955b` |
| `swscale-9.dll` | 12748288 / `3d07972cada6ba38c492e92b0f6c025a6835607fe00cdf83afc604c2fbdfe550` | 12900864 / `d217a08bcd92b3a4b7ecb02e841724460a5c94a103d957370cfe75e892ac3dc2` |

## Decoder / hardware comparison

| Required capability | Old | Candidate |
| --- | --- | --- |
| `h264` | Present | Present |
| `hevc` | Present | Present |
| `hap` | Present | Present |
| `dxv` | Present | Present |
| `prores` | Present | Present |
| `vp9` | Present | Present |
| `av1` | Present | Present |
| `libdav1d` | Present | Present |
| `d3d11va` | Present | Present |
| `dxva2` | Present | Present |

Both -hwaccels lists: cuda, vaapi, dxva2, qsv, d3d11va, opencl, vulkan,
d3d12va, amf. D3D11VA is present in H264/HEVC/VP9/AV1 configurations; DXVA2
in H264/HEVC/VP9. Enumeration proves build support, not physical-device execution.

Decoder counts: old 558, candidate 542.
Removed: `adpcm_circus`, `adpcm_ima_escape`, `adpcm_ima_hvqm2`, `adpcm_ima_hvqm4`, `adpcm_ima_magix`, `adpcm_ima_pda`, `adpcm_n64`, `adpcm_psxc`, `ahx`, `evc`, `libcodec2`, `libdavs2`, `libgsm`, `libgsm_ms`, `libilbc`, `liblc3`, `libspeex`, `libsvtjpegxs`.
Added: `libopenh264`, `librsvg`.
No required Task 53 decoder is missing. This does not establish every previous
media format's compatibility.

## Regressions and native build

Initialized Build Tools vcvars64.bat -vcvars_ver=14.44; pinned the exact
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.
where.exe link.exe returned this path first. FFMPEG_DIR selects the verified
candidate SDK; its bin is first on process-local PATH. CARGO_TARGET_DIR is an
absolute path to target/qa/ffmpeg-lgpl-20261026/cargo-target in this checkout.

Commands: `cargo test -p video --features libav --release --lib <filter>`.

| Filter | Exit | Passed | Failed | Ignored | Filtered out |
| --- | --- | --- | --- | --- | --- |
| `libav_` | 0 | 44 | 0 | 6 | 542 |
| `hap` | 0 | 13 | 0 | 1 | 578 |
| `dxv` | 0 | 0 | 0 | 0 | 592 |
| `parallel_decode_tests` | 0 | 18 | 0 | 1 | 573 |
| `hardware` | 0 | 4 | 0 | 3 | 585 |

The table above is Codex's first pass, compiled from the shared working tree
(which then carried other lanes' uncommitted `crates/video` test edits).

Clean rerun (Claude): `git archive HEAD` (d471cfda) into
`target/qa/ffmpeg-lgpl-20261026/src-head-d471cfda/`, same pinned MSVC 14.44.35207
environment (`where link.exe` first hit is the pinned linker), `FFMPEG_DIR` = the
candidate SDK, candidate `bin` first on PATH (`where avcodec-62.dll` first hit is
the candidate), `CARGO_TARGET_DIR=target/qa/ffmpeg-lgpl-20261026/cargo-target-head`.
Command: `cargo test --manifest-path <copy>/Cargo.toml -p video --features libav --release --lib <filter>`.
Cargo log shows `Compiling video v1.2.0-alpha.82 (...src-head-d471cfda\crates\video)`;
every filter ran `cargo-target-head\release\deps\video-18d7cd16daf234a5.exe`.
Compiler warnings: 0.

| Filter | Exit | Passed | Failed | Ignored | Filtered out | Listed |
| --- | --- | --- | --- | --- | --- | --- |
| `libav_` | 0 | 44 | 0 | 6 | 542 | 50 |
| `hap` | 0 | 13 | 0 | 1 | 578 | 14 |
| `dxv` | 0 | 0 | 0 | 0 | 592 | 0 |
| `parallel_decode_tests` | 0 | 18 | 0 | 1 | 573 | 19 |
| `hardware` | 0 | 4 | 0 | 3 | 585 | 7 |

Executed names include `libav_decoder::tests::decodes_h264_h265_and_prores_in_process`,
`libav_decoder::tests::ten_bit_yuv444_decode_reuses_scaler_across_seek_and_loop`,
`hap_decoder::tests::decodes_hap_mov_samples_in_process_at_native_size`,
`hap_decoder::tests::decodes_hap_r_bc7_movie_in_process_with_alpha`,
`libav_decoder::hardware::windows::tests::format_negotiation_never_accepts_software`
(full lists: `head-tests/test-results.json`).

Real-GPU ignored cases, run with `-- --ignored --test-threads=1` on the same
build (AMD Radeon 8060S, driver 32.0.31032.1003). The local fixtures are 1080p30
8 s `testsrc2` clips encoded with the old CLI's libx264/libx265. They are test
input only and never distributed.

| Run | Exit | Passed | Failed |
| --- | --- | --- | --- |
| H.264 fixture, filter `real_` (skip benchmark, invalid-adapter) | 0 | 5 | 0 |
| HEVC fixture, filter `real_` (skip benchmark, invalid-adapter) | 0 | 5 | 0 |
| `d3d11va_invalid_adapter` with `SYNDOCAL_D3D11VA_ADAPTER=2147483647` | 0 | 1 | 0 |

The five `real_` cases are `catch_up_real_gpu_two_outputs_350ms_cadence_preserves_sessions_and_seek_parity`,
`real_gpu_loop_wraps_rewind_the_hardware_session_in_place_fifty_times`,
`d3d11va_real_video_frames_seek_reuse_and_teardown`,
`seek_projection_real_video_matches_eager_oracle_with_fewer_transfers` and
`output_preview_sessions_real_video_paused_and_forward_requests_reuse_two_sessions`.
Only `task20_decode_demand_benchmark` stays unexecuted; it is a perf benchmark.
Evidence: `head-tests-gpu/results.json`.

DXV: the crate has no DXV-specific code or tests (DXV goes through the generic
libav path), so the `dxv` filter selects zero tests. Two substitute pieces of
evidence were collected instead:

1. Codex: FFmpeg FATE samples decoded by old and candidate CLIs, with identical
   `framecrc` for `dxv-na`, `dxv-wa`, `dxv3-hqna` and `dxv3-hqwa`. The
   `dxv3-nqna` download timed out and `dxv3-nqwa` was not attempted
   (`dxv-cli-results.json`).
2. Claude: a local decode-equivalence matrix with no downloads. `testsrc2`
   640x360, 60 frames, was encoded once, then decoded by the old CLI (on the
   old pinned DLLs, hashes re-verified against the old inventory, buildconf has
   `--enable-gpl`) and by the candidate CLI (buildconf has no `--enable-gpl`).
   `framecrc` was compared (`decode-equivalence-v2/results.json`).

| Fixture (encoder) | Encode | Old decode | Candidate decode | Frames | Identical |
| --- | --- | --- | --- | --- | --- |
| DXV3 normal quality / DXT1 (`dxv`) | 0 | 0 | 0 | 60 | yes |
| HAP (`hap`) | 0 | 0 | 0 | 60 | yes |
| HAP Alpha (`hap`) | 0 | 0 | 0 | 60 | yes |
| HAP Q (`hap`) | 0 | 0 | 0 | 60 | yes |
| ProRes HQ (`prores_ks`) | 0 | 0 | 0 | 60 | yes |
| H.264 (`libopenh264`) | 0 | 0 | 0 | 60 | yes |
| H.264 (`libx264`, old CLI) | 0 | 0 | 0 | 60 | yes |
| HEVC 8-bit (`libx265`, old CLI) | 0 | 0 | 0 | 60 | yes |
| HEVC 10-bit (`libx265`, old CLI) | 0 | 0 | 0 | 60 | yes |
| VP9 (`libvpx-vp9`) | 0 | 0 | 0 | 60 | yes |
| AV1 (`libsvtav1`) | 0 | 0 | 0 | 60 | yes |

Together these cover the DXV3 NQ variant that the FATE run missed. No DMX,
audio-device or 60fps acceptance is claimed.

Native build (Codex): `pnpm --dir app tauri build --no-bundle` with
`CARGO_TARGET_DIR=target/qa/ffmpeg-lgpl-20261026/cargo-target`. It built from
the shared working tree, so the exe includes other lanes' uncommitted UI edits.
Exit 0, 0 warning lines in `native-build.log`, 7m20s. The candidate exe
(74875392 bytes, SHA-256
`8926786ac670fd1102063421d74804cb612be537c58e3162f2c4abcbb4d371f0`) was
launched with only the seven hash-verified candidate DLLs beside it. PID 57968
showed the window "Syndocal", Responding=True, and loaded `avcodec-62`,
`avformat-62`, `avutil-60`, `swscale-9` and `swresample-6` from the isolated
`cargo-target\release` directory (`native-loaded-modules.json`). Claude closed
it with CloseMainWindow after checking its path; it exited cleanly. The protected
`target/release/syndocal.exe` is unchanged
(`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`, the same
before and after), and `target/release/avcodec-62.dll` is still the old pinned
one. No operator-level playback acceptance was done in the window.

## Notice / inventory / checks

Schema 1 lacked provenance and anchored GPL DLLs while notice claimed LGPL.
Schema 2 pins archive/version/config/license/source/recipe. Legacy/future schema,
missing/extra fields and drift fail closed. No product-version or user-data
migration change. Seven DLLs/four resources and strict JSON remain enforced.

Retired notice anchor: 3318 bytes,
`9c194c6b24445d8e9dbe39898b7aa20ee7b6e2a75972dd458c6011f79b47f175`.
Current notice: 5750 bytes, `6eb708d0392132a3783e65939bc6f2afc2d3d4f8b6047c0c76a689b224f59194`.

Direct anchored inventory + four actual common resources + seven actual candidate
DLL verification: exit 0. Added artifact self-test rejection fixtures for GPL,
nonfree, floating URLs, altered identities, retired notice and old GPL avcodec.

| Baseline command | Exit | Evidence |
| --- | --- | --- |
| node app/scripts/check-windows-release-artifacts.mjs --self-test | 1 | Notice size mismatch |
| node app/scripts/prepare-release-runtime.mjs --self-test | 0 | 98 assertions |
| node app/scripts/check-release-metadata.mjs | 1 | Default feature checker excludes current syphon feature, :195-196 |
| npm --prefix app run check:release | 1 | AI0 frontend invoke count 513 vs 511, check-ai0-source-coverage.mjs:117 |

Current checks (same existing failures retained):

| Command | Exit | Evidence |
| --- | --- | --- |
| node app/scripts/check-windows-release-artifacts.mjs --self-test | 0 | 156 assertions; extractor 43 + materialization 4 also passed |
| node app/scripts/prepare-release-runtime.mjs --self-test | 0 | 98 assertions |
| node app/scripts/check-release-metadata.mjs | 1 | Same pre-existing default feature mismatch |
| npm --prefix app run check:release | 1 | Same pre-existing AI0 513 vs 511 |
| node --check app/scripts/windows-runtime-inventory.mjs | 0 | Syntax |
| node --check app/scripts/check-windows-release-artifacts.mjs | 0 | Syntax |

No package gate was weakened to bypass an existing failure.

Three-tree check rerun (Claude). Trees: the shared working tree; `chk-head`, a
pristine `git archive HEAD` of d471cfda; and `chk-owned`, the same archive plus
only the eight owned files. Logs and results are in `final-checks/`.

| Check | worktree | head | head+owned |
| --- | --- | --- | --- |
| `node app/scripts/check-windows-release-artifacts.mjs --self-test` | 0 (156 + 43 + 4 assertions) | **1**, notice 4162 B vs anchored 3318 B | 0 (156 + 43 + 4) |
| `node app/scripts/prepare-release-runtime.mjs --self-test` | 0 (98) | 0 (98) | 0 (98) |
| `npm --prefix app run check:strict-json` | 0 (130) | 0 (130) | 0 (130) |
| Inventory anchors + 4 common resources + 7 candidate SDK DLLs | 0 | **1**, notice byte size | 0 |
| `node app/scripts/check-release-metadata.mjs` | 1 | 1 | 1 |
| `node app/scripts/test-check-release-metadata.mjs` | 1 | 1 | 1 |
| `node app/scripts/check-ai0-source-coverage.mjs` | 1 (513 vs 511) | 1 (513 vs 511) | 1 (513 vs 511) |
| `npm --prefix app run check:release` | 1 at ai0 | 1 (*) | 1 (*) |

(*) The archive copies are not Git checkouts, so `check:q1-q4-ledger` stops
earlier on "evidence path is not tracked". The ai0 row shows the same failure
directly.

Reading: this change fixes the one HEAD failure in its own scope (the notice
identity drift) and introduces no new failure. The remaining failures exist on
pristine HEAD and belong to other lanes:

- `check-release-metadata.mjs:195-196` requires default features to be exactly
  `libav + spout + asio`. `app/src-tauri/Cargo.toml:13` has had `syphon` since
  9a6836f1 (2026-10-20). `test-check-release-metadata.mjs` fails for the same
  reason.
- `check-ai0-source-coverage.mjs:117` counts 513 frontend invokes, against 511
  pinned.

Focused Cargo regression compiler warnings: 0 emitted, in both the first pass and
the isolated HEAD rerun. Baseline compiler warnings were not remeasured, so no
numeric delta is available. Native no-bundle build: 0 warning lines. Git
line-ending advisories are not compiler warnings.

## Source / recording remaining work

Exact FFmpeg source fetched: <https://github.com/FFmpeg/FFmpeg/archive/29e619e767cde9045a75c29bc9a8278ae7b3a98b.tar.gz>; 16922506 bytes;
SHA-256 `8e4699bcb6a311772e8f513cc72ce27328891b82a598b82e7cff3dfe580875a0`; 10259 entries.
Build recipes: <https://github.com/BtbN/FFmpeg-Builds/tree/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b>.

Independent stable-diff review found no introduced P0/P1 defect. Notice includes
FFmpeg copyright and IJG credit. `licenses/FFmpeg-LGPL-3.0.txt` matches the
BtbN archive's `LICENSE.txt` (LGPLv3) apart from one trailing newline.

Claude review: the pin is consistent across the CI workflow
(`cross-platform.yml:68-84`), the inventory (`FFMPEG_WINDOWS_RUNTIME_INVENTORY.json:2-12`),
the code anchor (`windows-runtime-inventory.mjs:45-53`), the build doc
(`WINDOWS_NATIVE_BUILD.md:48-60`) and the notice (`THIRD_PARTY_NOTICES.md:13-17`).
All five carry the same URL, version and SHA-256. The notice anchor (5750 bytes,
`6eb708d0...`) matches the tracked file. The following are open before
publication. They are outside this task's owned files and are not regressions
introduced here:

1. P1, GPLv3 text not shipped. LGPLv3 section 4(b) requires the GNU GPL text
   alongside the LGPL text. The normal payload's common resource set
   (`windows-runtime-inventory.mjs:30` and the Tauri resource list) carries only
   `FFmpeg-LGPL-3.0.txt`. Adding `licenses/GPL-3.0.txt` needs a coordinated
   change to the canonical resource set, the inventory and `tauri.conf` resources.
2. P1, attribution for statically linked dependencies. The DLLs embed
   BSD/MIT/zlib-style libraries (dav1d, libvpx, libaom, openh264, libwebp, zlib,
   SVT-AV1, rav1e, kvazaar and others). Their binary-redistribution attribution
   is only pointed to ("consult the pinned source and build recipes"), not
   reproduced. Ship an aggregated dependency license file built from the pinned
   BtbN recipe commit.
3. P2, local harness fallback to a GPL SDK. `app/scripts/check-native-workspace-operator.ps1:167-201`
   and `app/scripts/check-native-window-acceptance.ps1:2139-2173` still fall back
   to the WinGet Gyan `full_build-shared` SDK when `FFMPEG_DIR` is unset. Local
   acceptance builds can then silently link GPL DLLs. Point them at the pinned
   BtbN SDK, or make the fallback fail closed.
4. Info: `RELEASE_STATUS.md:533` and older QA handoffs still describe the
   8.1.2 SDK. They are historical, but the release status should be refreshed.
5. Info: the build compiles in `libopenh264` from source. Cisco's patent
   licence covers only Cisco-distributed binaries. This is a patent and policy
   note, not a copyright licence issue. Source/recipes are precise provenance locators;
full source closure is unproved. Before publication, preserve and serve complete
dependency sources, patches, licenses/notices (including incorporated GPLv3
terms), exact build images and reproducible instructions. Recipes still use
moving release/8.1 and Docker tags; freeze exact commits/environment. Monthly
upstream retention is two years, so preserve/mirror exact bytes before expiry.
This does not close DIST-NOTICES-001 or prove clean-machine distribution.

The Task 58 owner decision supersedes the earlier separate-GPL-encoder proposal:
recording now uses the **same pinned self-built LGPL SDK's `ffmpeg.exe`** as the
decode DLLs. No GPL encoder executable, libx264 or `--enable-nonfree` is admitted.
Bundled `ffmpeg.exe` beside the application wins, followed by the development
`SYNDOCAL_FFMPEG` override and explicit PATH discovery. Every newly admitted
executable must complete bounded `-L` and `-version` checks, prove the Lesser GPL
grant and reject GPL/nonfree flags or an unverifiable response. It does not
silently continue to a different executable after an invalid selected path.

Real two-frame 256x144 encode preflights try exactly
`h264_nvenc -> h264_amf -> h264_qsv -> h264_mf -> libopenh264`; first success
wins and is cached for the process session by canonical executable path and
SHA-256. Each process has an 8-second deadline, stop cancellation, a two-second
termination/reap window and retained process-tree ownership if the OS cannot
confirm termination. License output uses disk capture rather than an undrained
pipe. The chosen encoder is logged in existing diagnostics; no IPC/status
schema was extended. Unavailable/executable/license errors use three exact,
actionable English source keys with Japanese dictionary translations.

All encoder options share `-pix_fmt yuv420p` and a two-second GOP (`-g 2*fps`).
The old implicit x264 CRF 23 / veryfast quality intent is mapped as follows;
these settings do not claim perceptually identical output across encoders:

| Encoder | Recording and preflight options |
| --- | --- |
| `h264_nvenc` | `-preset p2 -rc vbr -cq 23 -b:v 0` |
| `h264_amf` | `-quality speed -rc cqp -qp_i 23 -qp_p 23` |
| `h264_qsv` | `-preset veryfast -global_quality 23` |
| `h264_mf` | `-rate_control quality -quality 75 -scenario archive` (MF chooses its available implementation) |
| `libopenh264` | `-rc_mode bitrate -b:v 8M -allow_skip_frames 0` |

MP4 and built-in AAC at 192 kbps are unchanged. Hardware availability and
requested output dimensions remain runtime constraints; a successful tiny
preflight does not prove every resolution or sustained recording budget.
The options were checked against the pinned executable's `-h encoder=...`;
[FFmpeg's encoder documentation](https://ffmpeg.org/ffmpeg-codecs.html) describes
the per-encoder controls. No paid dependency was introduced.

The canonical Windows inventory is now schema **3**, with a distinct
`recording_encoder` entry: `ffmpeg.exe`, **540,672 bytes**, SHA-256
`cc737ff68b38ba9228ee422b1c49ad0943e9132a17ff9a2b958aa47a24598d97`,
PE32+ AMD64 executable (characteristics `0x022e`). The SDK ZIP/source/configure,
seven DLL hashes remain unchanged. `THIRD_PARTY_NOTICES.md` corrects the former
no-encoder statement: 7,101 bytes, SHA-256
`d39a587a9cc3ae55e24567c0d6b338967594d5a591bd4853255cd3b852761bd1`;
the other five notice hashes are unchanged. Resolver, staging,
Tauri Windows resources, normal-artifact verification and optional Show-ASIO
runtime consumers all require the encoder. Schema 2 and future inventory
versions fail closed. This is build/distribution metadata, not a user-project
schema migration. The normal extracted tree grows from 14 to 15 files; the
optional Show-ASIO tree requires 17 files including its existing six notices.

Task 58 validation evidence and remaining acceptance boundaries are recorded
in the dated Task 58 section below. **Clean-install recording proof and the
self-built OpenH264 patent-policy note remain open.** Cisco-distributed binary
patent coverage is not inferred for this source-built library; see the
[Cisco-provided binary conditions](https://www.openh264.org/BINARY_LICENSE.txt).
Corresponding
source/dependency closure, signing, hardware acceptance and full release
readiness remain separate gates.

## Checkpoint / first safe resume

Codex's checkpoint held the commit. It cited failing release gates and
unproved DXV decode. Claude's resolution:

- The release gates fail identically on pristine HEAD, belong to other lanes,
  and are not caused or masked by this change.
- DXV decode, including the DXV3 NQ variant, is covered by framecrc equivalence
  between the old and candidate builds.

Exactly the eight owned files were staged and committed. Remaining owners:

- Default-feature/syphon drift (`check-release-metadata.mjs`).
- AI0 invoke count.
- GPLv3 text and the dependency-attribution bundle (DIST-NOTICES-001).
- The local `.ps1` harness fallback.
- Corresponding-source mirroring.
- The recording encoder decision.

Do not change other owners' work or the protected normal exe/DLLs.
Isolated installer resource paths need separate proof: the regular bundling hook
still stages target/release; no-bundle trial does not exercise it.


## Task 54 - release notices and checker drift - 2026-10-27

Original checkout `main`, initial base `0a40de3e3c3b69700dcae119d34d2645ba376882`.
Owned work: common license resources and exact byte anchors; two native harness
SDK resolvers; release checker drift; current SDK clarification in release status.
Concurrent video/UI files remain outside this task. No codec, product version,
Cargo feature, encoder selection or normal executable change. The seven normal
staged DLLs were refreshed to the existing pin for release checks after
confirming no Syndocal process was running; prior DLLs/hashes are preserved in
`target/qa/task54-release-notices/normal-runtime-before/` and its JSON receipt.
Local evidence: `target/qa/task54-release-notices/`.

The GPLv3 full text comes verbatim from `COPYING.GPLv3` in the hash-verified
FFmpeg source archive at `29e619e767cde9045a75c29bc9a8278ae7b3a98b`:
35,147 bytes, SHA-256 `8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903`.
It accompanies LGPLv3 under section 4(b). The dependency notice collection is a
bounded collection of pinned source candidates, with full license texts and
provenance; it is explicitly **not** an exact binary SBOM or publication clearance.
Collection: 104 source candidates and 577 Cargo.lock-checksum-verified crate
candidates; 1,399 notice records, 522 unique full documents and 877 in-file
references to identical texts. Aggregate: 2,692,678 bytes, SHA-256
`0f2facc3e4f3e388a6ccafc5c3e568424e8e9338be250f53f5fd301d3f34ec26`.
The source-only JSON records per-source revisions and hashes.
`generate-ffmpeg-dependency-notices.py --check` deterministically verifies the
retained evidence offline. Both license texts are common Tauri resources,
pinned by independent code/inventory identities
and LF Git attributes. Common resources increase from four to six; runtime DLLs
remain the same seven. Artifact tests require both new files and reject missing
or same-size corrupted copies.

### Blocking findings and publication policy

- P1, transitive GPL conflict: pinned recipe
  [`50-chromaprint.sh`](https://github.com/BtbN/FFmpeg-Builds/blob/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b/scripts.d/50-chromaprint.sh)
  enables Chromaprint on FFmpeg >=6, builds it with `-DFFT_LIB=fftw3`, and adds
  `-lfftw3`. [`25-fftw3.sh`](https://github.com/BtbN/FFmpeg-Builds/blob/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b/scripts.d/25-fftw3.sh)
  selects `93ed4c786934aec9946f8dda4b4e3eb08f8be41c`, static-only.
  That source grants GPL-2.0-or-later. This supports a blocking provenance/license
  conflict, not proof of every actual linker input. Do not claim that avutil's
  `LGPL version 3 or later` string resolves it. Obtain the actual build link map
  and inputs or select and reaccept a demonstrably suitable SDK in a separate
  authorized change. Merely shipping GPL text does not resolve the boundary.
- P1, exact dependency attribution remains pending: rav1e's pinned recipe runs
  `cargo update cc`; the final build lockfiles and link map are unavailable.
  Source-tree licenses can include unused code. Four source candidates have no
  named license document (three NV codec headers plus simd_helpers). Sixteen
  crate archives have no named license document; pinned upstream supplements
  cover eight, while `block`, `difflib`, `malloc_buf`, `objc-foundation`,
  `objc_id`, `simd_helpers` and both winapi GNU archive crates remain unresolved.
  Embedded source-header notice closure is also pending. Preserve exact binary/source
  inputs, dependency sources, patches, toolchain/build images and notices before
  publication; the recipe's moving image tags do not prove reproducibility.
- Policy: the recipe compiles OpenH264 from source. Cisco's
  [FAQ](https://www.openh264.org/faq.html) limits Cisco-paid AVC patent royalties
  to its own distributed binary module under its conditions, and excludes
  third-party source-built redistribution. Release owner must resolve applicable
  patent policy before publication. No codec was changed by this task.
- `DIST-NOTICES-001` stays open: this is the FFmpeg portion only; whole-product
  BOM/SBOM, ASIO/NDI boundaries, corresponding-source hosting and final artifact
  license/extraction verification are separate required evidence.

### Harness and checker behavior

Both PowerShell harnesses now delegate to one shared SDK resolver. When
`FFMPEG_DIR` is unset, only the exact pinned SDK root under
`target/qa/ffmpeg-lgpl-20261026/sdk/` is eligible. Explicit roots pass the same
seven DLL SHA-256/size/PE checks and required nonempty headers/import libraries.
Missing, incomplete, GPL, changed or unverifiable SDKs fail closed, with a link
to `WINDOWS_NATIVE_BUILD.md`; no WinGet, Gyan, arbitrary directory or PATH SDK
fallback remains. The inherited floating local SDK is rejected as intended.

`9a6836f1` intentionally added the macOS-only Syphon bridge/default feature.
The checker and explicit-feature approval set now include Syphon; Cargo defaults
are untouched and NDI/unknown feature rejection remains. `git log -S` traces
`query_video_authoring_lease_authority_v1` and `replace_video_authoring_lease_v1`
to `2907a2ea` (authoritative lease turnover after project Open). They are valid
read/runtime-mutation routes, so AI0 now expects 513 with explicit presence
assertions and its existing exact registration/uniqueness checks.

### Validation and first safe resume

All tests below use stable notice bytes. No installer, source offer, publication,
clean-machine, hardware or 60fps acceptance is claimed.

| Final command | Exit | Observed evidence |
| --- | --- | --- |
| `node app/scripts/check-release-metadata.mjs` | 0 | Full current metadata and seven staged pinned DLLs |
| `node app/scripts/test-check-release-metadata.mjs` | 0 | 147 assertion groups |
| `node app/scripts/check-windows-release-artifacts.mjs --self-test` | 0 | 160 + extractor 43 + materialization 4 assertions |
| `node app/scripts/prepare-release-runtime.mjs --self-test` | 0 | 99 assertions |
| `node app/scripts/check-ai0-source-coverage.mjs --self-test` | 0 | 3 parser/duplicate cases |
| `node app/scripts/test-resolve-pinned-ffmpeg-sdk.mjs` | 0 | 23 checks, including 3 PowerShell parses; independently rerun |
| `npm --prefix app run check:strict-json` | 0 | 130 duplicate-key assertions; source-only dependency JSON separately strict-parsed |
| `python app/scripts/generate-ffmpeg-dependency-notices.py --check` | 0 | All retained source/notice hashes and deterministic outputs |
| `node app/scripts/prepare-release-runtime.mjs` | 0 | Seven exact pinned DLLs staged; prior bytes retained |
| `npm --prefix app run check:release` | 1 | Stops at AI4 -> Agent Bridge projection equality; see below |

The two requested checker drifts are fixed. The final release run gets past
completion/Q1-Q4, AI0 (Tauri 593, Engine 287, Remote 116, MIDI/OSC/DMX 240,
Frontend 513, Keyboard 33, UI literal invokes 349; six authority gates), AI1
and AI2. It then exposes another HEAD drift: Agent Bridge's Rust operation
inventory is 75, TypeScript 76, Node MCP 72. The checker count now correctly
pins 75 with explicit T33 operation assertions. `ad5fe649` introduced the three
external-add/configure/remove operations, which Node MCP still lacks.
TypeScript alone contains `syndocal.query.video.ndi_sources.v1` from `d8befdc3`.
Exact equality assertions remain enforced. Resolving product projections is
outside this task's checker/resource ownership; the release aggregate remains
**pending**, and no later chained gates are claimed to have run.

Preliminary attempts are excluded from final evidence: a no-bundle build began
before the in-progress dependency text existed and failed at that missing
resource; the complete resource set was then used for the rerun. The runtime
preparation wrapper initially rejected inherited
`CODEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS` as an unknown NDI-named environment
signal (the substring in ENDINGS). Final runtime preparation/release checks
unset only this unrelated key in the process environment; the fail-closed
product guard was not weakened. No persistent environment was changed.

Independent reviews accepted the bounded notice/resource propagation, SDK
resolver and checker drift, while explicitly withholding exact-closure and
publication approval. No compiler runs in the script-only checks, so they have
no compiler-warning measurement. Native build/window evidence follows below.


Native gate: `pnpm --dir app tauri build --no-bundle`, exit 0; release Cargo
phase 5m34s (Vite 14.79s). Wrapper initialized Build Tools vcvars64 with
`-vcvars_ver=14.44`, pinned
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and printed/verified that exact linker first. `FFMPEG_DIR` selected the exact
pinned SDK; its bin was first on process-local PATH. Absolute isolated target:
`target/qa/ffmpeg-lgpl-20261026/cargo-target`, reusing the Task 53 build cache.
This is an internal no-bundle acceptance artifact, not a distributed prerelease.

Executable: Product/FileVersion `1.2.0-alpha.82`, 74,898,432 bytes, SHA-256
`e4581465bd9462596b7a296be6c91926e8e4278286f6468a39dda3fe610a9961`.
Seven actual DLLs and all six actual common resources in that output directory
match their inventory size/SHA-256 identities (`native-artifact-identities.json`).
Exactly one responsive `Syndocal` window, PID 67084, HWND 4003236, was verified
against that exact executable path, observed rendered and maximized (native
Restore control), then closed cleanly. Five loaded FFmpeg DLLs were observed
from the isolated directory: avcodec, avformat, avutil, swscale and swresample
(`native-window-final.json`). No hardware controls were exercised.

The Computer Use launch resolver initially opened the installed
`C:\Program Files\Syndocal\syndocal.exe` instead of the requested path.
That task-opened instance was identified and closed without product interaction;
it is excluded from acceptance. Direct absolute-path launch then established
the isolated artifact identity above. Native-window inspection used the
`computer-use:computer-use` skill
through the available CUA runtime; the installed tool path is environment-owned,
not a repository dependency.

Final native compiler warning count: 0. No pre-change native warning baseline
was compiled in Task 54, so numeric delta is unavailable. Git line-ending
advisories and the preliminary failed-build summary are not compiler warnings.
The normal `target/release/syndocal.exe` SHA-256 remains
`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.
The staged normal runtime DLLs now match the existing pin; their previous bytes
are retained locally as described above.

Stop boundary: pending exact binary/source/license closure and Agent Bridge
projection reconciliation. First safe resume is to resolve those owner decisions
and evidence, then rerun `check:release` and affected package/source checks.
Do not infer publication readiness from the passing notice, native smoke or
individual metadata checks. No installer/publication or product version bump
was performed; unrelated video test dirty markers were preserved.

## Task 55 - Agent Bridge equality and binary license proof - 2026-10-27

Original checkout `main`, base `0f3a4c0364ec441d1921ae2616dc1874780be995`.
Owned scope is Agent Bridge projections/tests, release-only AI6/AI7 checker
drift, and this evidence section. Concurrent video, native entry point and UI
implementation/checker lanes are preserved. Local evidence is retained under
`target/qa/task55-agent-bridge/`. No runtime DLL, codec, Cargo feature, Tauri
command, product version, installer or distribution change is made.

### Part A - intentional membership retained

`git show d8befdc3` proves that NDI discovery intentionally added
`syndocal.query.video.ndi_sources.v1`, its `list_ndi_sources_v1` native command,
reviewed runtime-read registry route and TypeScript bridge adapter.
`git show ad5fe649` proves that T33 intentionally added
`syndocal.output.external.add.v2`, `syndocal.output.video.configure.v2` and
`syndocal.output.video.remove.v2` to Rust and TypeScript, with existing typed
OutputControl transactions. None is an accidental product operation.

Rust's finite bridge wire projection now includes the missing NDI query;
Node MCP includes that query and the three missing T33 operations. TypeScript
already contains all four and is unchanged. All three projections contain
exactly **76 unique operations**. Exact set equality remains enforced, with
explicit presence and duplicate rejection assertions. Tauri registration and
the frozen **593-command** control-plane inventory remain unchanged.

The wire schema is unchanged: `control_plane.execute` carries `operationId`
and an object-valued `request`; Node also requires the request UUID. Rust
retains strict envelope deserialization and finite admission; TypeScript
retains its static operation-to-typed-command map. Operation-specific request
and response schemas remain backend-owned, with no second schema or permissive
fallback introduced. New native-wire and mock-MCP tests preserve nested request
contents, classify NDI as read/T33 as mutations, and reject array payloads;
the mock rejection test proves the broker receives no malformed request.
These envelope tests do not claim actual typed output execution or LAN discovery.

Release checker-only corrections preserve their existing acceptance:

- AI6 follows the actual App import/mount to `AppWorkspaceView`, then checks
  its authority route, panel import and mount. `2b0424df` intentionally extracted
  this JSX. Three mutation self-tests reject removed App mount, authority route
  and authority panel; the original security assertions remain.
- AI7 pins the existing AI0/routing assertions at 513, replacing its stale 511
  markers. `2907a2ea` intentionally added query/replace video-authoring lease
  routes after project Open; no product inventories were changed here.

Independent stable-diff review found no blocking issue in these seven source
and checker files; authorization, finite admission, mutation classification,
request identity and unchanged schema boundaries were inspected.

### Part B - P0: actual Chromaprint and FFTW payload in the pinned LGPL DLL

This supersedes the source-only P1 suspicion in Task 54 with binary evidence.
`binary-identities.json` verifies the exact pinned archive SHA-256
`3e47bda1607740550141e37c0e49d1e5182b34699f15adfd137ee266d346811a`
and all seven DLL sizes/SHA-256 values against the runtime inventory.
The relevant exact DLLs are:

| DLL | Bytes | SHA-256 |
| --- | ---: | --- |
| `avformat-62.dll` | 22,722,560 | `bcf514a394dc5ec43fe64e4f1624ba63b23e9c25967081ea2d03d95b0dc359ef` |
| `avcodec-62.dll` | 90,992,128 | `4f817870cfaeaced3e34a083c50a1b4be0389202d7fb821632d47cf9b3f4d82a` |

Observed evidence (enumeration only, no media/device/output opened):

1. SDK `ffmpeg.exe -hide_banner -buildconf` includes `--enable-chromaprint`.
   Direct exact-path `avformat_configuration()` independently confirms the
   same flag in the DLL (`binary-buildconf.txt`, `avformat-direct-probe.json`).
2. `ffmpeg.exe -hide_banner -muxers` lists `E chromaprint Chromaprint`.
   Direct `av_muxer_iterate()` on the exact DLL independently returns that
   muxer, demonstrating its presence beyond a configure string.
3. `avformat-62.dll.strings.txt` contains **1,033** matched printable strings:
   Chromaprint muxer/error messages, FFTW assertions, an `fftw-3.3.11`
   wisdom identifier and hundreds of `fftw_codelet_*`/`fftw_dft_*` names,
   including SSE2/AVX/AVX2 implementations. `avcodec-62.dll` has only the common
   configure-line match; equivalent FFTW payload was not observed there.
4. MSVC `dumpbin /imports` and `/exports` for both DLLs contain no external
   Chromaprint/FFTW imports or exports. This does not exclude static linkage.
   Static incorporation in avformat is the evidence-supported conclusion,
   consistent with the pinned static Chromaprint and FFTW recipes.
5. Direct `avformat_license()` reports `LGPL version 3 or later`; this compiled
   self-description does not reconcile its transitive FFTW payload.

All three CLI enumerations and all four dumpbin commands exited 0. The retained
`probe-binaries.py` completed after explicitly decoding localized dumpbin output
as UTF-8; the initial decoding failure is not passing evidence. Relevant primary
sources: pinned [Chromaprint recipe](https://github.com/BtbN/FFmpeg-Builds/blob/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b/scripts.d/50-chromaprint.sh)
uses `BUILD_SHARED_LIBS=OFF`, `FFT_LIB=fftw3` and private `-lfftw3`;
pinned [FFTW recipe](https://github.com/BtbN/FFmpeg-Builds/blob/6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b/scripts.d/25-fftw3.sh)
uses static-only FFTW revision `93ed4c786934aec9946f8dda4b4e3eb08f8be41c`.
The pinned [Chromaprint README](https://github.com/acoustid/chromaprint/blob/aed8eba2202dd9d7b3b0a56c77904cc805490d72/README.md#fft-library)
expressly states that its FFTW-backed binary becomes GPL licensed.

**P0: the pinned DLLs cannot be accepted as the product's LGPL-only distribution
boundary.** Chromaprint and extensive identifiable FFTW implementation payload
are present in avformat. Leaving the muxer unused or adding GPL text does not
close this blocker. Exact final linker maps/input closure remain unavailable;
this is not a complete binary SBOM or publication clearance. No build is switched.

Concrete free options for a separately authorized follow-up:

- Pin another immutable LGPL shared build whose actual binaries and sources
  prove absence of Chromaprint/FFTW, or a suitable non-GPL FFT backend; reaccept
  ABI/features/decode/native/artifact/source/notices. No suitable replacement
  asset was verified here. The pinned recipe enables Chromaprint for every
  FFmpeg >=6 target without excluding LGPL variants, so merely selecting
  another ordinary >=6 BtbN variant using this recipe does not fix the issue.
- Self-build FFmpeg `29e619e767cde9045a75c29bc9a8278ae7b3a98b` with the pinned
  BtbN recipe, disabling Chromaprint in recipe enablement so generated configure
  emits `--disable-chromaprint`; also omit the unused FFTW dependency stage.
  Pin the build script's moving FFmpeg branch to the exact commit, retain image
  digests, dependency source/patches/final locks, verbose link inputs and notices.
  The recipe uses Bash plus Docker/buildx (`makeimage.sh`/`build.sh`, target
  `win64 lgpl-shared 8.1`). Estimate: 1-2 hours for the policy patch/configuration
  proof, and **1-2 engineer days** for an initial pinned candidate plus focused
  reacceptance if infrastructure cooperates. Cold dependency builds may take
  hours; Docker readiness, build duration and disk capacity were not measured.
  Exact source/license closure may take longer.
- If Chromaprint is required, rebuild it with its bundled permissive KissFFT
  backend and remove FFTW dependencies/private linkage; independently verify
  resulting binary closure. This entails more change than disabling the muxer.

Other Task 54 dependency/source, embedded-notice and OpenH264 policy boundaries
remain open. This task provides no publication, physical-output or 60fps proof.

### Release continuation boundary

After the owned AI4/AI6/AI7 fixes, F1 reaches the unrelated UI checker
`check-dvc-midi-shortcuts.mjs:66`, which still searches `App.tsx` for MIDI
mapping JSX now in `SetupIoWorkspaceView.tsx:534`. These files belong to the
active UI lane; no assertions are bypassed or weakened to manufacture a pass.
The remaining release sequence was also run independently through metadata
(`remaining-release-gates.json`): **39 of 46 commands exit 0**, seven exit 1.
These independent passes do not imply the aggregate itself passed.

| Remaining failing command | Cause observed |
| --- | --- |
| `check:f1-input-generations` | Delegates to MIDI UI checker above; extracted mapping JSX |
| `check:dmx-universe-state` | `check-dmx-universe-state.mjs:83` counts zero DMX panels in App; actual mount is `SetupIoWorkspaceView.tsx:301` |
| `check:safety-blackout-runtime` | `check-safety-blackout-runtime.mjs:83` searches App for safety engage; actual JSX is `AppWorkspaceView.tsx:2206` |
| `check:live-audio` | `check-live-audio-input.mjs:1250` searches App for graph getter; actual getter is `AppWorkspaceView.tsx:2102` |
| `check:timeline-transport-runtime` | `check-timeline-transport-runtime.mjs:558` searches App for cue-panel callbacks; actual JSX is `PersistentWorkspaceView.tsx:1239` |
| `check:video-display-target` | Helper slice at `check-video-display-target.mjs:34-43` runs through `export default function App`, now also including `createAppView` and its JSX; its TypeScript data-URL fixture fails with `SyntaxError: Invalid left-hand side in assignment`. The checker itself passes `node --check`; this is not proof of a product parse error |
| `check:camera-input` | Camera profile check passes, then `check-video-transport-status-polling.mjs:98` searches App for capture fault getter now in `AppWorkspaceView.tsx:1913` |

The six missing-JSX assertions match the intentional `2b0424df` extraction.
The UI lane owns updating those checks and diagnosing the display fixture.
No other lane was waited on, polled, edited or resumed. First safe resume is
those UI checker repairs, then an unmodified full `check:release` rerun.
The pinned SDK replacement remains a separate P0 follow-up; passing release
scripts alone would not close the binary license blocker.

### Final owned-scope validation

| Check | Exit | Observed evidence |
| --- | --- | --- |
| `node app/scripts/check-agent-bridge.mjs` | 0 | 15 processor/runtime groups; exact 76/76/76 parity and uniqueness |
| `node tools/syndocal-mcp/check.mjs` | 0 | 20 mock adapter groups, 128 hostile stdio rejections; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | 0 | Four deferred lifecycle groups |
| AI6 checker and `--self-test` | 0 / 0 | Actual extracted route; six self-test assertions |
| AI7 checker and `--self-test` | 0 / 0 | Exact current count markers; three self-test assertions |
| `cargo test --release -p syndocal agent_bridge -- --test-threads=1` | 0 | 35 selected tests passed, zero failed; 2,285 filtered out; new wire test explicitly ran. Compile 6m36s, execution 0.20s |
| `pnpm --dir app tauri build --no-bundle` | 0 | TypeScript/Vite passed (Vite 12.20s); Cargo release phase 4m45s |
| Native artifact identity verification | 0 | Seven exact pinned DLLs and six notice resources match inventory |
| `npm --prefix app run check:release` final rerun | 1 | Passes AI4/AI5/AI6/AI7; stops at F1 MIDI checker above |
| Independent continuation through release metadata | 1 overall | 39/46 commands pass; seven failures enumerated above; metadata itself exits 0 |
| `git diff --check` | 0 | Owned diff inspected; unrelated edits preserved |

Native tests and build initialized vcvars64 with `-vcvars_ver=14.44`, pinned
MSVC **14.44.35207** to the absolute Build Tools linker, printed it and verified
it first in `where.exe link.exe`. `FFMPEG_DIR` selected the exact pinned SDK;
its bin was prepended to the process-local PATH. Absolute isolated Cargo target
is `target/qa/ffmpeg-lgpl-20261026/cargo-target`. The build wrapper verified that
this exact target executable was not running before compilation. Only the
unrelated inherited `CODEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS` environment key
was removed process-locally, as in Task 54; no product guard was weakened.

Resulting internal executable: version `1.2.0-alpha.82`, **74,898,432 bytes**,
SHA-256 `2f03ea5d5984a29eea79aa15f49ab8a88b8dd7d071e89f0904279c53598232e8`.
Exact-path launch produced one responsive `Syndocal` window, PID **6476**,
HWND **9770078**. Computer Use inspection verified the intended executable,
rendered Ready state and maximized state (native Restore button). Five loaded
FFmpeg modules came from the isolated output: avcodec, avformat, avutil,
swscale and swresample. The task-owned process was closed cleanly and its exit
verified; no hardware controls or product mutations were exercised. Receipts:
`native-artifact-identities.json`, `native-window-final.json`,
`native-window-cua.json` and build/test logs in the evidence directory.
The `computer-use:computer-use` skill supplied the native inspection workflow.

Native test/build compiler warnings: **0 / 0**. No pre-change compiler baseline
was built in Task 55, so a numeric warning delta is not measured. Git CRLF
advisories are not compiler warnings. Normal `target/release/syndocal.exe`
remains unchanged at SHA-256
`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.

Concurrent UI files became dirty during native compilation; they are outside
this checkpoint. The built artifact and observed window are evidence for the
stable owned bridge change, not acceptance of those later UI edits or the
entire final shared working tree. No rebase, stash, reset, checkout, unrelated
staging, hardware output, paid dependency or publication was performed.

## Task 56 - LGPL self-built FFmpeg SDK without Chromaprint/FFTW3 - 2026-10-27

Closes the Task 55 Part B binary blocker for the **local pin**. Codex built the
candidate (`f23cc7e2`, `7d0af757`); Claude completed verification and the pin
switch after Codex hit its usage limit. Evidence is retained under
`target/qa/ffmpeg-selfbuild-20261027/` and the downloaded artifact under
`%LOCALAPPDATA%\SyndocalDev\ffmpeg-lgpl-selfbuild-40366564270\`.

### Workflow runs

| Run | Commit | Result |
| --- | --- | --- |
| [40366564270](https://github.com/KTN44295080/Syndocal/actions/runs/40366564270) | `7d0af757` | Build **succeeded** (1h05m). Wine inspection and the seven-DLL `chromaprint\|fftw` scan passed; the step then failed only on `grep -F 'LGPL' license.txt`, because `ffmpeg -L` prints "GNU Lesser General Public License". Artifact `ffmpeg-lgpl-sdk-7d0af757...-40366564270` (id 12674399245, 116,996,507 bytes, expires 2027-01-24) uploaded |
| [40377168832](https://github.com/KTN44295080/Syndocal/actions/runs/40377168832) | `3e390aae` | Rerun after the licence-check fix (see "Rerun status" below) |

`3e390aae` makes `inspect-candidate.sh` fail on any `GNU General Public License`,
`nonfree` or `not legally redistributable` wording, then require the
Lesser GPL grant and `either version (2.1|3) of the License`. It was tested
against the real run-40366564270 `license.txt` (exit 0) and a GPL-substituted
copy (exit 1).

### Candidate identity (adopted pin)

- Inventory version `n8.1.3-9-g29e619e767-20260930-syndocal-r40366564270`;
  `av_version_info()` reports `n8.1.3-9-g29e619e767-20260930` (fixed
  `--extra-version`), `git describe` of the built source `n8.1.3-9-g29e619e767`.
- SDK ZIP `ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1.zip`,
  79,353,186 bytes, SHA-256
  `81587fa070a9fa2da3d8ff3e63beb9654729782bc8e9899ee951baf5fc5d9f1a`
  (equal in the CI receipt and recomputed locally).
- FFmpeg source tarball SHA-256
  `8e4699bcb6a311772e8f513cc72ce27328891b82a598b82e7cff3dfe580875a0` (pinned).
- Recipe: BtbN `6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b` plus
  `tools/ffmpeg-sdk/0001-lgpl-without-chromaprint-fftw.patch`; base images
  `base@sha256:91eaaaf3...` and `base-win64@sha256:8a3d189e...`
  (created 2026-10-17); mingw GCC 16.2.0, rustc 1.99.1; built rav1e lock equals
  the committed `rav1e.Cargo.lock`.

| DLL | Old BtbN bytes | Candidate bytes | Candidate SHA-256 |
| --- | ---: | ---: | --- |
| `avcodec-62.dll` | 90,992,128 | 90,940,416 | `0954bd519a2c7a1dbe596440b8d84abfd0de45472143612a9a4ad72c3ca672d1` |
| `avdevice-62.dll` | 4,947,456 | 4,947,456 | `69a0646a626b1c60b25f2435c42cd62bd9266264235a72de252a9931367e482e` |
| `avfilter-11.dll` | 31,212,544 | 31,212,544 | `bec4350bb5bc9196a344fcdccec095017b2d28f4bf3439cf00fe325fee75bac5` |
| `avformat-62.dll` | 22,722,560 | 19,531,264 | `6dc37e85d82a112bef743b7d07e965723250c17a4407068cb80cf05beaa3166b` |
| `avutil-60.dll` | 3,014,144 | 3,013,632 | `df1408834ab806417f2314400c12287af875c07a1c74a2c3f29fd32c6e6bcf1c` |
| `swresample-6.dll` | 734,720 | 734,720 | `235ef70cc3d9faa72a42504ae7c1deef39bded6ba4af5cb247886ac57f1a60c8` |
| `swscale-9.dll` | 12,900,864 | 12,900,864 | `19873bf3978f3891c5c92f278115ea444a4dbf7502881127d6f37b953f472070` |

All seven are PE32+ AMD64 DLLs with the anchored identity (characteristics
0x222e). avformat shrinks by 3,191,296 bytes (Chromaprint + FFTW3 payload).
The small avcodec/avutil differences come from rebuilding all dependencies with
the newer digest-pinned toolchain image; byte-identical reproduction of the
BtbN pin was not a goal and is not claimed.

### Absence proof (native Windows, `tools/ffmpeg-sdk/verify-candidate.py`)

`verify-candidate.py` (committed with this checkpoint) probes each SDK in its
own Python process via `ctypes`, runs the SDK CLI, and fails closed on any
mismatch. It exited **0** (`status: PASS`). Evidence: `evidence/`.

- Configure (`avcodec_configuration()` == `avformat_configuration()` ==
  `-buildconf`): identical to the old pin except `--enable-chromaprint` removed;
  no `--enable-gpl`, `--enable-nonfree` or `--enable-chromaprint`; nothing added.
- `avutil_license()` = `avformat_license()` = `LGPL version 3 or later`;
  `ffmpeg -L` states the GNU Lesser General Public License, version 3 or later.
- Muxers: 184 to 183; only `chromaprint` removed (CLI and `av_muxer_iterate()`).
- Strings: **0** case-insensitive ASCII and **0** UTF-16LE `chromaprint|fftw`
  matches in each of the seven DLLs; also 0 in `ffmpeg.exe`, `ffprobe.exe`,
  `ffplay.exe`, `include/` and `lib/`. Old pin for contrast: avformat 1,034
  ASCII matches, each other DLL 1 (the configure string). The CI scan agrees.

### Decoder / hwaccel diff

- Decoders: 541 old, 541 candidate; **none removed, none added**. Required
  `h264`, `hevc`, `hap`, `dxv`, `prores`, `vp9`, `av1`, `libdav1d` present.
- Per-decoder `avcodec_get_hw_config()` (device, methods, pix_fmt): **equal**
  for every decoder.
- `-hwaccels`: identical, `cuda vaapi dxva2 qsv d3d11va opencl vulkan d3d12va amf`.
- No Chromaprint-related decoder exists, so the only capability change is the
  muxer above.

### Regressions (isolated)

Source: `git archive 3e390aae` into
`target/qa/ffmpeg-selfbuild-20261027/src-3e390aae/` (excludes the concurrent
uncommitted video-test edits). `CARGO_TARGET_DIR` =
`target/qa/ffmpeg-selfbuild-20261027/cargo-target`; vcvars64
`-vcvars_ver=14.44` with the absolute MSVC **14.44.35207** linker first in
`where.exe link.exe`; `FFMPEG_DIR` = the candidate SDK, its `bin` prepended to
the process PATH. Every run executed
`cargo-target\release\deps\video-18d7cd16daf234a5.exe` (SHA-256
`a1dc916255bc21eb74739691dd881997624f741db0c932bc04386114514e7094`, 600 tests),
as printed by Cargo in each log (`tests/*.log`, `tests/results.json`).

| `cargo test -p video --features libav --release --lib ...` | Exit | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: | ---: |
| `libav_` | 0 | 44 | 0 | 6 |
| `hap` | 0 | 13 | 0 | 1 |
| `parallel_decode_tests` | 0 | 18 | 0 | 1 |
| `hardware` | 0 | 4 | 0 | 3 |
| `real_ -- --ignored` (H.264 1080p30 fixture, real D3D11VA GPU) | 0 | 5 | 0 | 0 |
| `real_ -- --ignored` (HEVC 1080p30 fixture, real D3D11VA GPU) | 0 | 5 | 0 | 0 |
| `d3d11va_invalid_adapter -- --ignored` (adapter 2147483647) | 0 | 1 | 0 | 0 |

The real-GPU runs skip only `task20_decode_demand_benchmark` and the separately
run invalid-adapter test, as in Task 53. They cover catch-up two-output
cadence, fifty in-place loop rewinds, D3D11VA seek/reuse/teardown, seek
projection against the eager oracle and output-preview session reuse. GPU
fixtures are the Task 53 `gpu-h264-1080p30-8s.mp4` (SHA-256 `3a5907c8...`) and
`gpu-hevc-1080p30-8s.mp4` (`b5cb26f7...`).

DLL provenance: no FFmpeg DLL exists in the Cargo target or System32. The same
test binary run with PATH = candidate `bin`;System32;Windows passed `libav_`
44/0/6; with PATH = System32;Windows only, it exited `0xC0000135`
(`STATUS_DLL_NOT_FOUND`). The tests therefore loaded the candidate DLLs
(`tests/dll-provenance.txt`).

### Framecrc equivalence

The 11 Task 53 fixtures (`decode-equivalence-v2/`, fixture SHA-256s verified)
were decoded with `-threads 1 -map 0:v:0 -f framecrc` by the old pin and the
candidate: **11/11 identical**, 60 frame records each (DXV DXT1, HAP, HAP Alpha,
HAP Q, ProRes HQ, H.264 OpenH264, H.264 x264, HEVC x265, HEVC 10-bit x265, VP9,
AV1); all decodes exited 0. Both outputs also match the Task 53 old-pin
`.old.framecrc` records byte-for-byte (22/22).

### Pin decision: switched (local)

All acceptance gates passed, so the local pin now names the candidate:

- `qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` and the independent anchors in
  `app/scripts/windows-runtime-inventory.mjs` (provenance, configure, seven DLL
  sizes/SHA-256, notice resource identities). `archive_url` is the Actions
  artifact URL, an interim locator.
- `app/scripts/resolve-pinned-ffmpeg-sdk.mjs` default root
  `target/qa/ffmpeg-selfbuild-20261027/sdk/ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1`.
  The old Chromaprint SDK is now rejected as an `FFMPEG_DIR` override (exit 1).
- `qa/WINDOWS_NATIVE_BUILD.md`, `THIRD_PARTY_NOTICES.md` (6,899 bytes,
  `e94a15bc2bcdd46626873a5227ce0b58b1f96174a6c32c5acb3011c8fda93f11`),
  `RELEASE_STATUS.md` pin sentence and `tools/ffmpeg-sdk/README.md`.
- `generate-ffmpeg-dependency-notices.py` excludes the deleted `chromaprint` and
  `fftw3` recipes (fail-closed if their receipts are not exactly present) and
  records them in `excluded_disabled_recipes`; limitations updated for the
  removed recipes and the locked rav1e build. Regenerated aggregate: sources
  104 to 102, notices 1,399 to 1,390, `licenses/FFmpeg-DEPENDENCIES.txt`
  2,692,678 to 2,662,745 bytes, SHA-256
  `dc70e08c2a19eb52b3a7e5b6faf6ac1d9ab30bd44e72f7936f28e8d1c8ef8875`; no
  Chromaprint/FFTW text remains apart from the header limitation.
- `target/release` was restaged from the candidate with
  `pnpm --dir app run prepare:runtime-libs` (exit 0, MSVC 14.44 environment).
- CI (`.github/workflows/cross-platform.yml`) still downloads the old BtbN
  archive because the candidate has no durable URL. A
  `TODO(ffmpeg-sdk-hosting)` block marks it. The new "Reject Chromaprint FFmpeg
  DLLs before release packaging" step refuses the old avformat SHA-256
  (`bcf514a3...`) in `target/release` or `FFMPEG_DIR`, and any
  `chromaprint|fftw` string in the seven staged DLLs, before
  `Bundle Windows installers`. Because the anchors changed, the old SDK already
  fails earlier at `Stage Windows runtime libraries`. **The hosted Windows job
  is therefore expected to fail closed until CI downloads the hosted
  candidate.** No GitHub release or tag was created.

### Checks

| Check | Exit | Observed |
| --- | ---: | --- |
| `verify-candidate.py` (capabilities + 11 framecrc) | 0 | PASS |
| `python app/scripts/generate-ffmpeg-dependency-notices.py --check` | 0 | 102 sources, 577 crates, 1,390 notices, 0 unresolved |
| `node app/scripts/resolve-pinned-ffmpeg-sdk.mjs` (FFMPEG_DIR unset) | 0 | resolves the candidate root |
| same, `FFMPEG_DIR` = old BtbN SDK | 1 | rejected (expected) |
| `node app/scripts/test-resolve-pinned-ffmpeg-sdk.mjs` | 0 | 23 checks |
| `node app/scripts/check-asio-packaging.mjs` | 0 | 175 assertions |
| `node app/scripts/check-windows-release-artifacts.mjs --self-test` | 0 | 160 + 4 assertions |
| `pnpm --dir app run prepare:runtime-libs` (candidate `FFMPEG_DIR`) | 0 | seven candidate DLLs staged |
| `npm --prefix app run check:release` | 0 | 55 package scripts; ends `release metadata ok: Syndocal 1.2.0-alpha.82`; verifies the seven staged candidate DLLs and new notice identities. Run on the shared working tree, which also held uncommitted UI-lane edits |

The user environment sets a global `FFMPEG_DIR` to an unrelated
`ffmpeg-n8.1-latest` SDK; the resolver correctly rejects it, so the checks
above unset it or point it at the candidate explicitly.

### Rerun status

Run 40377168832 (`3e390aae`) **completed successfully**, finishing at
2026-10-26T19:32:29Z. Every step passed, including the corrected Wine
inspection, which accepted the Lesser GPL v3 `-L` wording, and the seven-DLL
scan. Its outputs are **not byte-identical** to run 40366564270, although the
source tarball matches (`8e4699bc...`). The rerun ZIP SHA-256 is
`f84ae76001eb0882c423724936b3337b5e949a06dd6437368f7afdacecb9dffb`, and all
seven DLL hashes differ (avcodec `ab23c191...`, avformat `c6d0de23...`). The
cause was not analysed; embedded build or ZIP timestamps are the likely source.
The pin therefore stays the natively verified run-40366564270 bytes. A rebuild
is not a substitute for preserving that exact archive.

Cross-platform run 40380328003 for `3bb0cac0` failed on both jobs:

- **Windows:** failed at `Stage Windows runtime libraries`, as designed. The
  downloaded BtbN SDK no longer matches the anchored inventory.
- **Ubuntu:** failed in `Test Rust workspace`. The cause is outside the release
  lane: `tests::synchronous_project_runtime_routes_hold_the_identity_fence_through_dispatch`
  panicked at `app/src-tauri/src/main.rs:99567` (1900 passed, 1 failed).

The GitHub Release hosting the SDK was **not** created in this task. A relayed
coordinator message said the owner had chosen a prerelease in this repository.
Publishing public content needs the owner's own confirmation, so release
assets and notes were only staged locally (see Remaining work).

### Remaining work

1. **Durable hosting** of the candidate ZIP and its corresponding source
   (owner decision). Then switch the CI download URI/hash and `archive_url`,
   and keep the release-packaging guard as a regression check. The Actions
   artifact expires 2027-01-24; copy it before then.
2. Until (1), the hosted Windows CI job fails closed; local builds use the
   candidate.
3. Task 54 limitations still apply: exact binary dependency closure, crate
   notices for the rav1e lock deltas (cc 1.10.0, find-msvc-tools, shlex),
   embedded per-file notices, OpenH264 patent policy, and complete
   corresponding-source preservation. Byte-reproducibility of the self-build is
   not demonstrated.
4. A native `tauri build` and launch with the candidate DLLs beside the
   executable were not repeated in this task; the video-crate regressions and
   framecrc matrix are the acceptance evidence.

### Hosting addendum - 2026-10-27

The owner chose GitHub Release hosting. The supervisor created build-dependency
prerelease
[`ffmpeg-sdk-lgpl-n8.1.3-r40366564270`](https://github.com/KTN44295080/Syndocal/releases/tag/ffmpeg-sdk-lgpl-n8.1.3-r40366564270),
an annotated tag on `7d0af757` (not latest). It carries the 14 staged assets
plus `SHA256SUMS`. All 15 assets were re-downloaded into an empty directory:
`sha256sum -c SHA256SUMS` passed for all 14, and that `SHA256SUMS` is identical
to the locally staged one. The SDK ZIP still hashes to `81587fa0...`.

Commit `cbd8cd55` points the CI download, `archive_url` in the inventory and
anchors, the native build doc and the third-party notice locator at the
release asset. `THIRD_PARTY_NOTICES.md` is now 6,976 bytes, SHA-256
`703bc4fddc6fad1b11522bd10a00d7f0b5c060f177a977266eed914454f8ac91`. The
`TODO(ffmpeg-sdk-hosting)` block is removed; the old-avformat and
Chromaprint/FFTW string guard stays as a regression check before Windows
packaging.

Local checks all exit 0: notice `--check`, resolver default and self-test
(23), ASIO packaging (175), Windows artifact self-test, `prepare:runtime-libs`
and `check:release`.

CI results:

- **Windows, runs 40442259482 (`cbd8cd55`) and 40446728698 (`9c58e96a`).**
  The SDK download hash check, `Stage Windows runtime libraries`, the
  staged-runtime assertion, ASIO packaging and the warning ratchets passed;
  staging had failed in every completed run inspected back to 2026-10-24.
  Both runs were then cancelled
  by newer pushes (`cancel-in-progress`) during `Test Rust workspace`.
  The packaging guard and bundling were not reached.
- **Ubuntu, both runs.** Failed in `Test Rust workspace` on
  `tests::synchronous_project_runtime_routes_hold_the_identity_fence_through_dispatch`:
  `app/src-tauri/src/main.rs:99567` asserts 182 runtime mutation routes but
  183 are registered. The first CI failure is run 39543079797 on `2907a2ea`
  (2026-10-21, adds the video authoring lease routes). Its parent `7b32ad68`
  passed that step. The failure appears in every completed run since.
  It reproduced **5/5** locally (Windows, debug, isolated `git archive
  cbd8cd55`), with left 183 vs right 182 each time. It is a deterministic
  stale count, not a flake, and belongs to the native lane. The Windows
  `Test Rust workspace` step runs the same test, so the Windows job cannot
  turn green until that count is reconciled.

<a id="task-58--lgpl-recording-encoder--2026-10-27"></a>

## Task 58 - LGPL recording encoder - 2026-10-27

Base: `main` at `c353e172f1b82ef3d029b1d9446b05a9709ff77f`. The owner accepted
the LGPL recording route described in [Source / recording remaining work](#source--recording-remaining-work).
The implementation, exact options, runtime executable identity and remaining
publication boundaries are recorded there. `main.rs`, `crates/**` and frontend
components were not edited; the only UI change is three Japanese source-key
entries. Existing unrelated dirty video-test markers were preserved.

Evidence root: `%LOCALAPPDATA%/SyndocalDev/task58-recording-lgpl/`. `head.tar`
is a `git archive HEAD` snapshot of the base; `src/` overlays only the owned
files listed in `owned-files.txt`. Python's UTF-8 tar extraction recovered
Japanese distribution filenames after Windows tar rejected them. No extra
branch/worktree or shared-index mutation was needed. Git attribute checks use
the existing `.git` read-only with `GIT_WORK_TREE` pointing at the archive.
Dependency installation is reused via the `app/node_modules` junction; frontend
output and both Cargo targets are isolated from the shared checkout.

`run.mjs` resolves and verifies the pinned SDK, initializes vcvars64 with
`-vcvars_ver=14.44`, pins the exact Build Tools MSVC **14.44.35207** linker and
prints/verifies it first in `where.exe link.exe` before each native command.
`FFMPEG_DIR` and process-local PATH select that SDK; `SYNDOCAL_FFMPEG` and
`SYNDOCAL_FFPROBE` select its exact CLIs. The unrelated inherited
`CODEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS` key is removed process-locally.
Debug tests use `cargo-target/`; the no-bundle native build uses a separate
`native-target/`. The Tauri wrapper verifies/stops only that isolated release
executable. No physical DMX, audio device or external application is exercised.

The first release-check run detected an affected static contract: recording
now has **six** early failure paths that must close a started receipt, rather
than four. `check-output-ownership.mjs` now includes LGPL/encoder admission and
fallible command construction alongside lease, process, stdin and renderer
startup. This tightens coverage of the new failure paths; it does not relax
receipt ownership. Its standalone rerun passed.

| Check | Exit | Evidence |
| --- | ---: | --- |
| Base `cargo build -p syndocal --locked` | 0 | `baseline-build.log`; 4m01s; compiler warnings 0 |
| Candidate `cargo build -p syndocal --locked` | 0 | `candidate-build.log`; 2m51s; compiler warnings 0 |
| `cargo test -p syndocal --locked recording -- --test-threads=1` | 0 | 95 passed, 0 failed, 8 ignored, 2,225 filtered; all six new selection tests ran |
| `cargo test -p syndocal --locked -- --test-threads=1` | 0 | 2,303 passed, 0 failed, 25 ignored; 184.54s, no tests filtered |
| `cargo test -p syndocal --locked recording_command_writes -- --ignored --nocapture --test-threads=1` | 0 | 3 passed, 0 failed, 0 ignored, 2,325 filtered; automatic AMF, forced MF and forced OpenH264 |
| Localization contract | 0 | `localization-check.log`; 4,214/4,214 static Japanese entries, 0 unprotected user-data labels |
| `npm --prefix app run check:release` | 0 | `release-check.log`; all 54 static commands plus metadata (55 total); new encoder and notice identities verified |
| Pinned SDK resolver fixtures (provisioned shared checkout) | 0 | 25 checks: existing 23 plus missing/corrupt executable rejection; copied SDK fixture includes the encoder |
| ASIO packaging / Show-ASIO artifact / Show-ASIO preparation | 0 / 0 / 0 | 183 / 155 / 16 assertions; no bridge build, device or process-stop action |
| Windows artifact final self-test | 0 | `artifact-selftest.log`; 162 assertions, extractor 43, materialization 4; exact 15-file normal tree (8 FFmpeg runtime files, 6 notices, app) |
| Runtime preparation self-test / actual isolated stage | 0 / 0 | 99 assertions / all 7 DLLs plus encoder verified and staged |
| `pnpm --dir app tauri build --no-bundle` | 0 | `native-build.log`; TypeScript/Vite passed, Vite 19.14s; Cargo release 8m13s |
| Native identities / window | 0 | All 8 FFmpeg files and 6 notices verified; one responsive Ready window, maximized; task-owned app closed cleanly |

The base build first required staging the pinned DLL resources into the
archive's `target/release`; this setup failure was corrected before the
successful baseline above. Initial native-launcher setup also corrected the
local pnpm entry point to Corepack's installed `pnpm.js`; no product change or
guard bypass was involved.

Direct CLI probe receipts (`direct-chain.json`) show NVENC unavailable
(`nvcuda.dll` missing), AMF successful, QSV unavailable (unsupported MFX
implementation), and Media Foundation/OpenH264 successful. The Rust session
automatically selected **`h264_amf`**. All three runtime MP4 tests passed in
4.71 seconds: raw RGBA plus a PCM tone produces H.264/AAC MP4, FFprobe sees both
streams, all 30 submitted frames decode, the previous target remains unchanged
until atomic publication, and staging disappears on success. The tests use
256x144 rather than the former 16x16 fixture so hardware encoders can accept
the same input shape as the preflight; frame-count assertions grow accordingly.
This is local encode/mux/publication proof, not clean-install or live A/V soak
acceptance.

Native artifact: `native-target/release/syndocal.exe`, **74,956,800 bytes**,
SHA-256 `277620b580fe8e3228c6b9ff7b9edaa6b277a38f57386fe02005621941c53806`.
The exact-path process was PID **15372**, HWND **115607300**, title `Syndocal`,
`Responding=true`. Computer Use verified the rendered empty-project Ready
state and maximization: the Windows system menu enabled Restore and disabled
Maximize. A successful earlier accessibility snapshot also reported Ready and
the native Restore control; later accessibility snapshots intermittently
returned null, so the final receipt distinguishes visual evidence from that
earlier accessibility evidence. No product-data edits or output actions were
performed. Alt+F4 closed the task-owned process, and absence of its window and
process was verified. Five loaded modules (avcodec, avformat, avutil, swscale,
swresample) came from the isolated release directory. Receipts:
`native-artifact-identities.json`, `native-window.json`,
`native-window-cua.json`, `native-launch.json`, `native-exit.json`.

Compiler warnings for the matched non-test debug configuration are
**baseline 0 / current 0 / delta 0**. The candidate release build and all three
Cargo test invocations also report **0** compiler warnings; no pre-change
release/test baseline was built, so those profile deltas are unmeasured. No
warning suppression or acceptance assertion was weakened. Git line-ending
advisories are not compiler warnings. The application version remains
`1.2.0-alpha.82`; this is an internal checkpoint, not a distributed artifact.

Remaining: clean-install recording proof; self-built OpenH264 patent-policy
decision; requested-resolution/GPU availability and sustained live A/V soak;
the wider corresponding-source/dependency, signing and release acceptance
gates. The local encode tests and native window do not close those gates.

Independent stable-source review found no actionable defect, including the
exact option-array assertions, bounded teardown, error localization, inventory
anchors, normal/Show-ASIO consumers and corrected notice. This review is source
evidence and does not establish clean-install or patent-policy acceptance.
