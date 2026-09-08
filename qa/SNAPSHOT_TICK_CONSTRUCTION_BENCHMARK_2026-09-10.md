# Snapshot tick construction benchmark — 2026-09-10

This is a second measurement-only checkpoint for the snapshot performance
boundary. It does not modify `EngineRuntime::build_snapshot`, publication
ordering, lock scope, audio fences, output ownership, or the wire format.

## Scope and fixture

- Source base before this checkpoint: `6a34619dee3adf81d7abe972ddc93947679c4baf`
- Owned source file: `crates/engine/src/snapshot_output_read_benchmark.rs`
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: 21,612 bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- The fixture was loaded into a test-only `EngineRuntime` with DMX disabled.
  No application process, output device, physical output, or external client
  was started.

## Result

The release command used the exact Windows native procedure: `vcvars64.bat
-vcvars_ver=14.44`, the absolute Build Tools MSVC `14.44.35207` linker pin,
and that linker first in `where.exe link.exe`.

```text
cargo test -p engine --release --locked benchmark_show_tick_snapshot_construction -- --ignored --nocapture --test-threads=1
round=0 iterations=1000 build_snapshot_ns=11349300
round=1 iterations=1000 build_snapshot_ns=10933700
round=2 iterations=1000 build_snapshot_ns=11188700
round=3 iterations=1000 build_snapshot_ns=11504100
round=4 iterations=1000 build_snapshot_ns=11269600
median iterations=1000 build_snapshot_ns=11269600
test result: 1 passed; 0 failed; 0 ignored
```

The benchmark loads the preserved snapshot and measures only repeated
`EngineRuntime::build_snapshot(0)` construction. The load error remained
`None` and the benchmark does not call the engine worker or output path.

## Interpretation and limits

This is an isolated small-fixture tick-construction baseline, not a real-show
tick budget, FPS, CPU, writer-wait, lock-contention, multi-client delta, or
payload/IPC result. It does not justify moving a lock, adding a shadow cache,
or changing authored/rendered video ownership. A future optimization must
first establish the missing representative workload, change one ownership-
preserving boundary, and verify values, ordering, E/R/H, audio fence, and
output ownership.

`rustfmt --check --edition 2021` passed for the owned benchmark file and
`git diff --check` passed. The existing whole-workspace formatter difference
in unowned `app/src-tauri/src/fixture_profile_contract.rs` remains untouched.
The real-file missing → UI Retry → recovery flow remains unperformed by
explicit user boundary.
