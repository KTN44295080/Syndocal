# Snapshot clone and payload benchmark — 2026-09-10

This checkpoint adds an opt-in measurement only. It does not change the
product snapshot, synchronization, lock ownership, publication ordering,
audio fence, output ownership, or frontend wire format.

## Scope and ownership

- Source base before this checkpoint: `1614caf6c4544ecbc65b5a17bc5c81775a8d33d3`
- Owned source file: `crates/engine/src/snapshot_output_read_benchmark.rs`
- Added item: ignored benchmark for one preserved representative `.sdc`
- No application process, native output, device, or external client was used

The benchmark measures three separate boundaries on the same decoded public
snapshot: public clone, JSON serialization of the retained public image, and
clone plus serialization. It asserts that the serialized bytes are identical
between the retained public image and a freshly cloned public image.

## Fixture and environment

- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: 21,612 bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- Release Cargo run used `vcvars64.bat -vcvars_ver=14.44`, the absolute Build
  Tools MSVC `14.44.35207` linker pin, and that linker first in
  `where.exe link.exe`.

## Results

Command:

```text
cargo test -p engine --release --locked benchmark_show_snapshot_clone_and_payload -- --ignored --nocapture --test-threads=1
```

Final run: exit 0, `1 passed; 0 failed; 0 ignored`.

```text
median iterations=1000 clone_ns=3461800 encode_ns=10238400 clone_encode_ns=14246800 payload_bytes=12224
```

The existing release snapshot regression also passed: `snapshot_public` had
10 passed, 0 failed, 1 ignored. The existing representative narrow-reader
benchmark passed with median `full_ns=13885500` and `narrow_ns=139500` over
2,000 iterations.

## Interpretation and limits

On this small representative fixture, serialization is a larger isolated
cost than public cloning. This does not measure writer wait, tick-time
snapshot construction, deep-delta comparison under multiple clients, real
show FPS/CPU, or end-to-end IPC. Therefore no lock move, shadow cache, dirty
tracking, or wire change is justified by this checkpoint alone. A future
optimization must first add the missing writer/tick or multi-client workload,
change one ownership-preserving boundary, and recheck values, ordering,
E/R/H, audio fence, and output ownership.

`rustfmt --check --edition 2021` passed for the owned benchmark file and
`git diff --check` passed. Whole-workspace `cargo fmt --all -- --check`
continues to report a pre-existing formatting difference in the unowned
`app/src-tauri/src/fixture_profile_contract.rs`; that file was not changed.

The real-file missing → UI Retry → recovery flow remains unperformed by
explicit user boundary and is not inferred from this benchmark.
