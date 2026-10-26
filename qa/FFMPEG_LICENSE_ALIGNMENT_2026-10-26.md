# FFmpeg license alignment - Task 53 - 2026-10-26

Status: pin switched to the BtbN LGPL 8.1 shared build and committed. Focused
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

Do not bundle a recording encoder in this task. Recommend a separate pinned
encoder exe in a dedicated directory with its own hash/source/license manifest.
`app/src-tauri/src/video_recording.rs:255` resolves SYNDOCAL_FFMPEG then PATH
ffmpeg; command constructor :112, libx264 :146, optional built-in aac :140.
libx264 requires the GPL encoder distribution path; never --enable-nonfree.
Built-in aac is not libfdk_aac. The pinned LGPL CLI's encoder list
offers LGPL-compatible H.264 paths: `libopenh264`, `h264_amf`, `h264_mf`,
`h264_nvenc`, `h264_qsv` and `h264_d3d12va`. It offers no `libx264`. The
recording lane must choose between two options. One is the GPL route: a separate
GPL encoder exe with libx264 and a GPL source offer. The other is to switch
`video_recording.rs:146` to an LGPL encoder with a hardware-first fallback chain.
Recording lane must add bounded codec preflight,
actionable unavailable errors, exact process/dependency paths, installer/inventory
integration and clean-install recording proof. Do not replace LGPL decode DLLs
with GPL encoder DLLs. No encoder was bundled.

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
