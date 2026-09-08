# Snapshot performance triage — 2026-09-08

## Purpose and boundary

This is a measurement-only checkpoint on current `main` after the bounded
snapshot live-publication repair. It selects the next safe performance unit;
it does not reimplement the existing narrow readers, four-entry sync history,
Timeline bank reductions, or live publication policy.

- Source base: `c0727d68149f90b8674525a8f905552ac07245bc`
- Product source changes in this checkpoint: none
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture identity: `21,612` bytes, SHA-256
  `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Fixture shape: `fixtures=1`, `cues=1`, `video.outputs=1`
- No app process, native output, device, or external client was started

## Measurements

All Cargo commands used the Windows native procedure from
`qa/WINDOWS_NATIVE_BUILD.md`: `vcvars64.bat -vcvars_ver=14.44`, the absolute
Build Tools MSVC `14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and `where.exe link.exe` with that
linker first.

### Existing narrow reader

`cargo test -p engine --release --locked benchmark_show_video_outputs_snapshot -- --ignored --nocapture --test-threads=1`
passed. On the fixed sample and 2,000 iterations, the median isolated read
costs were:

| Path | Median |
| --- | ---: |
| Full `handle.snapshot()` then field access | `13,980,500 ns` |
| Existing narrow readers | `141,900 ns` |

This is an isolated field-read comparison, not an application FPS, lock-wait,
or real-show performance claim. It confirms that this existing boundary is not
the next repair target.

### Existing bounded snapshot sync

`cargo test -p syndocal --release --locked benchmark_full_snapshot_sync_shared_capture -- --ignored --nocapture --test-threads=1`
passed. The current bounded-history implementation was compared with the
preserved four-entry pre-Arc algorithm using the same synthetic workloads; the
medians below are total time for 1,000 requests, with serialization excluded:

| Workload | Previous | Current bounded history | Full / delta |
| --- | ---: | ---: | ---: |
| full-heavy | `257.1931 ms` | `162.1041 ms` | `1000 / 0` |
| mixed-heavy | `142.9578 ms` | `117.7676 ms` | `33 / 967` |
| steady-heavy | `119.2836 ms` | `119.2153 ms` | `1 / 999` |
| steady-light | `975.6 µs` | `913.1 µs` | `1 / 999` |

The current result keeps the exact full/delta counts and wire-equivalence
assertions from the existing benchmark. Steady-heavy does not show a stable
delta-computation improvement, so no deep-delta rewrite is justified by this
measurement.

## Decision and remaining work

No source change is made. The remaining general snapshot capture, tick-time
construction, deep delta comparison, serialization/payload, and representative
writer-wait measurements remain open and separate. A future source change must
first obtain a representative workload with the relevant lock/writer timing,
then change exactly one ownership-preserving boundary and compare values,
ordering, E/R/H, audio fence, and output ownership.

This checkpoint does not claim product-wide performance, real-show FPS,
physical output, ASIO/NDI acceptance, Mac real-device behavior, signing,
publication, or whole-product completion. The prior real-file thumbnail
missing → UI Retry → recovery test was not rerun.
