# Snapshot video surface benchmark — 2026-09-09

This is a measurement-only checkpoint for the existing rendered/authored
video separation. It does not change `EngineRuntime::build_snapshot`, video
ownership, Timeline Follow projection, publication ordering, audio fences, or
the persisted wire shape. The file-missing → UI Retry → recovery flow was not
rerun.

## Source and fixture

- Source base: `0e9efaf6b3d6f069cd8ae9f05b7ea7edeab6b3c0`
- Owned source file: `crates/engine/src/snapshot_output_read_benchmark.rs`
- Product source changes: none; the owned change is an ignored benchmark and
  this QA record.
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: 21,612 bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- No application process, output device, physical output, or external client
  was started.

## Measurement

The release test used the exact Windows native procedure: `vcvars64.bat
-vcvars_ver=14.44`, the absolute Build Tools MSVC `14.44.35207` linker pinned
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that linker first in
`where.exe link.exe`.

```text
cargo test -p engine --release --locked benchmark_show_tick_video_surface_construction -- --ignored --nocapture --test-threads=1
median iterations=1000 rendered_ns=1253400 authored_from_rendered_ns=2031900 both_ns=2009200
test result: ok; 1 passed; 0 failed; 0 ignored
```

The benchmark separately measures `EngineRuntime::video_snapshot()`, the
authored projection derived from that rendered image, and the same two
surfaces retained together. The result confirms that authored projection
construction is a material slice of this fixed fixture's video-surface work.

The existing snapshot reader regression was rerun previously on the same
release engine test binary after the preceding measurement checkpoint; this
new benchmark does not alter runtime code or its assertions. A focused
regression rerun for this checkpoint is recorded with the commit.

## Decision and limits

The authored and rendered surfaces have different semantics: renderer-only
Follow layers must not enter authored persistence, while native/UI consumers
need the rendered image. This isolated measurement therefore does not justify
collapsing the surfaces, adding a cache, moving the publication lock, or
changing the snapshot contract. A future optimization must preserve values,
ordering, E/R/H, audio-fence behavior, and output ownership on a representative
show workload.

This is not a real-show tick budget, FPS/CPU, writer-wait, deep-delta, WebView
IPC, native-window, physical-output, device, Mac, signing, publication, or
product-completion result.
