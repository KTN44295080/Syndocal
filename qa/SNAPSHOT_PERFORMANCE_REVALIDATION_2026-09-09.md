# Snapshot performance revalidation — 2026-09-09

This is a measurement-only checkpoint on current `main`. It does not change
snapshot ownership, publication order, lock scope, delta calculation, or
frontend behavior.

## Source and fixture

- HEAD: `ccd1b24cff8118a72f806421c2717e465d2ec934`
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: 21,612 bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- No application process, output device, physical output, or external client
  was started.

## Measurement

The command used the exact Windows native procedure: `vcvars64.bat
-vcvars_ver=14.44`, the absolute Build Tools MSVC `14.44.35207` linker pinned
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that linker first in
`where.exe link.exe`.

```text
cargo test -p engine --release --locked benchmark_show_video_outputs_snapshot -- --ignored --nocapture --test-threads=1
show_bytes=21612 fixtures=1 cues=1 outputs=1
median iterations=2000 full_ns=13766100 narrow_ns=142300
test result: ok. 1 passed; 0 failed; 0 ignored
```

The full path is `EngineHandle::snapshot()` followed by field access; the
narrow path is the existing dedicated readers. This confirms the already
implemented narrow-reader boundary remains materially cheaper on the fixed
fixture. It is not an application FPS, writer-wait, tick-construction,
full-snapshot serialization, deep-delta, or real-show performance claim.

The next source change, if justified by a representative measurement, must
change one ownership-preserving boundary and recheck values, ordering,
generation/epoch/revision/hold behavior, audio fence, and output ownership.
No such source change is made here. The missing-file → UI Retry → recovery
test remains unperformed and is not inferred from this measurement.
