# Media-derived data checkpoint — 2026-09-13

This checkpoint closes `MEDIA-DERIVED-001` for the supported current-source
software slice. It adds the canonical content-identity keyed cache policy and
bounded derived-work admission state machine, then revalidates the existing
thumbnail, audio-analysis, video-prefetch, and frame-cache paths. No existing
acceptance assertion was weakened.

## Implemented contract verified

- `MediaDerivedKey` is based on content identity, derived kind, algorithm
  version, and canonical settings; it is independent of a pathname.
- The machine-local cache has bounded bytes and entries, free-space reserve
  admission, LRU eviction, pinned/in-use protection, identical-key
  deduplication, corrupt-entry quarantine, explicit rebuild release, and
  operator clear semantics. A same-key resize that would exceed the total
  budget remains degraded and preserves the previous entry.
- Derived hash/probe/thumbnail/proxy/analysis work has a bounded queue with
  Playback/Next/Background priority, active-identity deduplication, bounded
  progress, queued-cancel versus running-cancel-request distinction, terminal
  result fencing, and explicit restart after terminal failure/degradation.
- Existing audio analysis produces waveform, spectrum, beat, and BPM data for
  deterministic WAV/FFmpeg fixtures. Existing video providers retain bounded
  prefetch and frame-cache behavior with reverse-loop/BPM planning, LRU
  capacity, invalidation, and active-layer retention. Existing thumbnail
  paths retain independent-source scans, authority fencing, retry/recovery,
  and native cancellation contracts.

## Verification

The browser plugin was not available and this checkpoint makes no new UI
layout claim. The source-level application checks and the Rust release tests
were run on the current checkout.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS |
| `pnpm.cmd --dir app run check:media-asset-operations` | PASS |
| `pnpm.cmd --dir app run check:media-asset-authority` | PASS |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS |
| `pnpm.cmd --dir app run check:bundled-library` | PASS |
| `pnpm.cmd --dir app run check:vj-first-run` | PASS |
| `node app/scripts/check-vj-media-import-access.mjs` | PASS |
| `cargo test -p engine --release --locked -j 1 media_derived -- --nocapture --test-threads=1` | PASS — 13 passed, 0 failed, 0 ignored |
| `cargo test -p audio --release --locked -j 1 -- --nocapture --test-threads=1` | PASS — 22 passed, 0 failed, 0 ignored; doc-tests 0 |
| `cargo test -p video --release --locked -j 1 prefetch -- --nocapture --test-threads=1` | PASS — 4 passed, 0 failed, 0 ignored |
| `cargo test -p video --release --locked -j 1 ffmpeg_cli_decoder_ -- --nocapture --test-threads=1` | PASS — 5 passed, 0 failed, 0 ignored |
| `cargo test -p video --release --locked -j 1 still_image_cache_ -- --nocapture --test-threads=1` | PASS — 2 passed, 0 failed, 0 ignored |
| `rustfmt --check crates/engine/src/media_derived.rs` | PASS |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — TypeScript/Vite build and Windows `target/release/syndocal.exe` build |
| Exact release executable process smoke | PASS — one exact-path `Syndocal` process, nonzero main-window handle, responsive; verification process then exited |
| `git diff --check` | PASS before commit |

The engine release test was also rerun after the required Windows procedure:
`vcvars64.bat -vcvars_ver=14.44`, the Build Tools `14.44.35207` x64 linker
pinned in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that exact linker
returned first by `where.exe link.exe`.

## Boundary

This closes the current-source identity/cache/work-admission policy plus the
existing deterministic thumbnail, audio-analysis, video-prefetch, and
frame-cache software slice. It does not claim that a complete proxy/transcode
artifact pipeline, general waveform/proxy/analysis product UI, hard-stop
decoder/OS-I/O/GPU cancellation, cold/warm workload performance budgets,
MP4 missing-file recovery, native renderer/GPU/display behavior, physical or
audible devices, external clients/clocks, venue/soak operation,
cross-platform execution, signing, publication, or product-wide media
acceptance is complete. Those boundaries remain represented by the
appropriate open or deferred markers.
