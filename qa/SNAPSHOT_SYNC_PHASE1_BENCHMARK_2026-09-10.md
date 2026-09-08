# Snapshot sync phase1 benchmark — 2026-09-10

This checkpoint measures the existing bounded four-entry snapshot sync using
the preserved phase1 fixture. It does not change `SnapshotSyncState`, delta
semantics, history capacity, frontend merge behavior, lock ownership, or the
IPC wire contract.

## Scope and fixture

- Source base before this checkpoint: `df356be21390c34e87cd5b89c2c45b773e73a897`
- Owned source file: `app/src-tauri/src/snapshot_sync_tests.rs`
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: 21,612 bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- No application process, WebView, external client, physical output, or
  device was started.

The ignored benchmark alternates two logical clients for 1,000 requests. Each
request changes only the clock BPM, and each client presents its exact prior
revision to the bounded four-entry history. It reports publication alone and
publication plus JSON serialization separately.

## Result

The release test used `vcvars64.bat -vcvars_ver=14.44`, the absolute Build
Tools MSVC `14.44.35207` linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that linker first in
`where.exe link.exe`.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 benchmark_phase1_snapshot_sync_delta_and_serialization -- --ignored --nocapture --test-threads=1
median iterations=1000 publish_ns=4305900 publish_serialize_ns=4621600 full/delta=2/998
```

The cumulative serialized response bytes for the 1,000-request run were
`195047`. The existing snapshot-sync regression was also rerun on the same
release test binary:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 snapshot_sync -- --test-threads=1
test result: ok. 9 passed; 0 failed; 2 ignored
```

The ignored counts are reported separately and are not included as passes.

## Interpretation and limits

The representative fixture preserves each alternating client's exact base:
only the first request from each client is full, followed by 998 deltas. This
confirms the existing bounded-history behavior on this fixture; it is not a
claim about network throughput, WebView IPC latency, CPU/FPS, writer wait,
real multi-window load, or physical output.

The next performance boundary remains writer contention or a larger
representative multi-client/deep-delta workload. No lock move, shadow cache,
dirty tracking, or wire change is justified by this measurement alone.
`rustfmt --check --edition 2021` for the owned test file and `git diff --check`
passed. The real-file missing → UI Retry → recovery flow remains unperformed
by explicit user boundary and is not inferred here.
