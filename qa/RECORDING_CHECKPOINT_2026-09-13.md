# Recording checkpoint — 2026-09-13

This checkpoint closes `RECORDING-001` for the supported current-source
recording software slice. The runtime now exposes the authoritative recording
lifecycle and the current Video surface keeps that status visible even when
the library-only composition is mounted.

## Implemented contract verified

- Recording status is explicit and serialized as `Idle`, `Preparing`,
  `Recording`, `Finalizing`, `Complete`, or `Fault`. The existing `active`
  field remains an additive compatibility projection of the non-terminal
  states.
- Start admission publishes `Preparing`; renderer admission publishes
  `Recording`; Stop publishes `Finalizing` while ownership is retained; normal
  atomic publication publishes `Complete`; encoder, worker-panic, and
  malformed/failure paths publish `Fault` with the error retained.
- The existing target reservation, atomic artifact publication, bounded Stop
  acknowledgement, late-worker reaping, and failure-rejection contracts stay
  in the same runtime owner. A late completion cannot be overwritten by a
  second Stop request.
- The frontend bridge validates the lifecycle enum with an explicit legacy
  fallback, and the extracted status bar renders the lifecycle, metrics,
  published artifact path, fault alert, and a disabled Finalizing control.
  `VideoControlPanel` mounts that status bar in the current library-only
  Video composition, so the recording status is not hidden by the existing
  VJ-surface reachability boundary.

## Verification

The browser plugin was not available. A local Vite/Chrome CDP DOM inspection
verified the rendered recording status bar; this is browser evidence only and
does not establish native UI interaction or physical output. The Windows
native gate used `vcvars64.bat -vcvars_ver=14.44`, with the pinned Build Tools
`14.44.35207` x64 linker first in `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 recording_ -- --nocapture --test-threads=1` | PASS — 61 passed, 0 failed, 6 ignored |
| `pnpm.cmd --dir app build` | PASS — TypeScript/Vite build |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups |
| Browser DOM inspection of the current recording surface | PASS — `barFound=true`, `activeClass=true`, label `● RECORDING`, state `Recording`, Stop enabled, bridge status present, no page errors |
| `pnpm.cmd --dir app run check:vj-operator` | KNOWN BOUNDARY — current checker expects six VJ layer labels while the fixture exposes seven; the current App intentionally mounts `VideoControlPanel` with `libraryOnly`, so ISF/output-rail selectors are absent. This is the existing VJ reachability boundary, not a recording-status failure. |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — Windows `target/release/syndocal.exe`; wrapper selected the pinned linker |
| Exact release executable process smoke | PASS — one exact-path `Syndocal` process, nonzero main-window handle, title `Syndocal`, responsive; verification process then exited |
| `git diff --check` | PASS before checkpoint commit |

The browser observation was:

```text
barFound=true
activeClass=true
label="● RECORDING"
status="Recording · 2 dropped · 3600 frames · 1920x1080 @ 60fps · 1 audio"
button="Stop Recording"
buttonDisabled=false
bridge=true
isf=0
outputs=0
pageErrors=[]
```

Native artifact identity for this checkpoint:

`target/release/syndocal.exe` SHA-256
`BEF911C6D3BF83362504AC466B01459314A02F04D5073E3C158302BBE2CD4A21`

## Boundary

This closes the current-source lifecycle/status UI, publication/recovery, and
worker-ownership software contracts for `RECORDING-001`. It does not claim
real H.264/AAC codec acceptance on this checkout (the six codec-boundary tests
remain ignored), disk-full or power-loss recovery, two-PC ownership or the
encoder hardware topology, authoritative asset import, the open
`DEC-RECORD-OWN-001` recording-owner decision, native in-app recording
interaction beyond the process smoke, ASIO/DMX/MIDI, long-duration A/V,
signing, publication, venue, or product-wide acceptance. Those gates remain
open in the Q1/Q4 coverage row and their corresponding Flow markers.

## Current-source real-codec and long A/V rerun — 2026-09-15

At current source HEAD `922ebeee0be2611738ccf2ee8ccd22bc69a37f3e`, the
recording engine was rerun with the explicit external test-only FFmpeg/FFprobe
pair that provides `libx264` and AAC. The binaries were:

```text
C:\Users\janua\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-9.0.1-full_build-shared\bin\ffmpeg.exe
C:\Users\janua\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-9.0.1-full_build-shared\bin\ffprobe.exe
```

The short real-codec test passed:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 recording_command_writes_a_real_video_and_audio_mp4 -- --ignored --nocapture --test-threads=1
test video_recording_runtime_tests::recording_command_writes_a_real_video_and_audio_mp4 ... ok
test result: ok; 1 passed; 0 failed; 0 ignored
```

The 30-minute A/V boundary test also passed in 6.05 seconds of test time:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 recording_command_keeps_long_av_sync_with_first_and_last_flash_clicks -- --ignored --nocapture --test-threads=1
test video_recording_runtime_tests::recording_command_keeps_long_av_sync_with_first_and_last_flash_clicks ... ok
test result: ok; 1 passed; 0 failed; 0 ignored
```

The retained report is
`C:\TEMP\syndocal-long-av-20260915-current\long-av-sync-30m.json`:

```text
frames_written=54000; frame_rate=30; duration_seconds=1800
video_duration_seconds=1800.0; audio_duration_seconds=1800.0
start_drift_ms=0.0; end_drift_ms=0.0
first_video_luma=255.0; middle_video_luma=0.0; last_video_luma=255.0
first_audio_peak=4328; middle_audio_peak=0; last_audio_peak=4119
```

This closes the current-source real H.264/AAC file-generation and synthetic
30-minute A/V sync slice. It is not a native in-app button workflow, a
physical camera-plus-display recording, disk-full/power-loss recovery, or a
venue encoder/ownership acceptance; `RECORDING-001`'s remaining external
boundaries stay open.
