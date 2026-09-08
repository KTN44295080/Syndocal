# Snapshot multi-client deep-delta regression — 2026-09-09

## Scope

This checkpoint runs the existing bounded snapshot synchronization regression
against a larger synthetic authored payload. It does not change snapshot
construction, delta semantics, history ownership, serialization, IPC, or
frontend merge behavior.

- Source: `main` at `05bbafcba1403248c0477f04a7116bae1b0bff36`
- Product source changes: none
- Workload: 256 authored cues, 16 group colors, 512-channel DMX preview, two
  alternating clients, 100 successive updates
- Native/hardware scope: no application process, device, external client, or
  physical output was started

## Verification

The command used the exact Windows native procedure: `vcvars64.bat
-vcvars_ver=14.44`, with the Build Tools `14.44.35207` x64 linker pinned and
returned first by `where.exe link.exe`.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 snapshot_sync -- --nocapture --test-threads=1
```

Result: **PASS — 9 passed, 0 failed, 2 ignored**. The two ignored tests are
existing opt-in measurement benchmarks and are not counted as executed
acceptance.

The larger payload regression reported:

```text
old full=100 delta=0 bytes=16036282
bounded full=2 delta=98 bytes=325412
retained=4
```

The same run passed exact-base selection for alternating clients, unknown and
future revision resynchronization, immutable history retention, nullable and
empty-collection clearing, revision exhaustion fail-closed behavior, and
independent authored/safety blackout deltas.

## Interpretation and boundary

This confirms the existing bounded four-entry history and top-level delta wire
contract on the synthetic multi-client/deep-payload workload. It is not a
real-show FPS/CPU, WebView latency, network throughput, writer-contention,
physical-output, or venue acceptance result. No optimization or wire change is
justified by this test alone.

The real-file missing → UI Retry → restore → recovery flow remains unperformed
by explicit user boundary and is not inferred from this checkpoint.
