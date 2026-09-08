# Snapshot writer contention benchmark — 2026-09-09

This is a measurement-only checkpoint on `main`. It does not change snapshot
ownership, publication order, lock scope, reader semantics, or frontend
behavior. The file-missing → UI Retry → recovery flow was not rerun.

## Source and fixture

- Source base: `732d8f96246d109fd632dc75b793097b77be752b`
- Product source changes: none; the owned change is an ignored test benchmark
  and this QA record.
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: 21,612 bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Fixture shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- No application process, WebView, external client, physical output, or
  device was started.

## Measurement

The command used the exact Windows native procedure: `vcvars64.bat
-vcvars_ver=14.44`, the absolute Build Tools MSVC `14.44.35207` linker pinned
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that linker first in
`where.exe link.exe`.

```text
cargo test -p engine --release --locked benchmark_show_snapshot_reads_under_writer_contention -- --ignored --nocapture --test-threads=1
writer_contention iterations=100 hold_ms=2 full_median_ns=2514800 narrow_median_ns=2518500 try_median_ns=13000 full_successes=100 narrow_successes=100 try_successes=0
test result: ok; 1 passed; 0 failed; 0 ignored
```

The benchmark holds the shared publication writer lock for a controlled 2 ms
per iteration. Under that condition the blocking public snapshot and the
existing narrow video-output reader both wait for the writer. The
`try_snapshot` path returns without waiting and returned `None` for all 100
observed reads; it therefore requires a retained last-good image or another
caller-owned fallback and is not a transparent replacement for every reader.

The existing snapshot-reader regression was rerun on the same release binary:

```text
cargo test -p engine --release --locked snapshot_read -- --test-threads=1
test result: ok; 4 passed; 0 failed; 2 ignored
```

## Decision and limits

This controlled writer-hold result does not justify moving the publication
lock, adding a shadow cache, or changing all readers to non-blocking reads.
No product runtime source was changed. A future latency-sensitive caller may
be evaluated separately when it can preserve the retained-image/fail-closed
contract of `try_snapshot`.

This is not an application FPS, real tick-budget, multi-window, deep-delta,
WebView IPC, native-window, physical-output, or real-show acceptance result.
Mac real-device/signing/notarization and external hardware gates remain
outside this local checkpoint.
