# Snapshot performance revalidation — 2026-09-09

## Earlier measurement retained

This earlier measurement was recorded on source HEAD
`ccd1b24cff8118a72f806421c2717e465d2ec934` using the same preserved
`samples/phase1-mini-show.sdc` fixture: 21,612 bytes,
SHA-256 `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`,
with `fixtures=1`, `cues=1`, and `video.outputs=1`. No application process,
output device, physical output, or external client was started.

The exact MSVC procedure and the release benchmark recorded:

```text
cargo test -p engine --release --locked benchmark_show_video_outputs_snapshot -- --ignored --nocapture --test-threads=1
show_bytes=21612 fixtures=1 cues=1 outputs=1
median iterations=2000 full_ns=13766100 narrow_ns=142300
test result: ok. 1 passed; 0 failed; 0 ignored
```

This was a fixed-fixture comparison, not an application FPS, writer-wait,
tick-construction, full-snapshot serialization, deep-delta, or real-show
performance claim.

## Current-source revalidation

This is a read-only revalidation of the remaining snapshot-performance
measurement boundary in the LUNA handoff. It uses the current `main` source,
the existing opt-in engine benchmarks, and a preserved show fixture. No
product source, persistence schema, IPC wire, lock ordering, or output path was
changed. Physical output and native UI interaction were not used.

## Fixed inputs

- Source checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`
- Source HEAD: `2789ac3765d46ec37078f2124c797252ee180743`
- Fixture: `phase1-mini-show.sdc` from the clean snapshot-profile worktree
- Fixture bytes: 21,612
- Fixture SHA-256: `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Fixture shape: 1 fixture, 1 cue, 1 video output, 1 video layer
- MSVC: Build Tools 14.44.35207, absolute linker pin and `where.exe link.exe`
  first-match check passed

The fixture was read only. It was not copied over, edited, or used to start a
physical-output path.

## Command and result

With the exact MSVC environment initialized:

```text
cargo test -p engine --locked --lib snapshot_output_read_benchmark -- --ignored --nocapture --test-threads=1
```

Result: **5 passed, 0 failed**. The invocation ran the existing ignored
measurement tests; it does not constitute a release-performance or show-FPS
acceptance gate.

| Measurement | Result |
| --- | ---: |
| Public snapshot clone, 1,000 iterations | median 9,148,200 ns |
| Public snapshot JSON encode, 1,000 iterations | median 305,307,500 ns |
| Clone + encode, 1,000 iterations | median 320,555,400 ns |
| Full snapshot read under a 2 ms writer hold, 100 iterations | median 2,504,200 ns |
| Narrow read under the same writer hold, 100 iterations | median 2,511,700 ns |
| Try-read under the same writer hold | 7,600 ns; 0 successful reads while held |
| `build_snapshot`, 1,000 iterations | median 49,339,100 ns |
| Rendered video surface, 1,000 iterations | median 4,605,100 ns |
| Authored-from-rendered surface, 1,000 iterations | median 7,098,000 ns |
| Rendered + authored surface, 1,000 iterations | median 7,128,600 ns |
| Full video-output snapshot read, 2,000 iterations | median 35,800,200 ns |
| Narrow video-output read, 2,000 iterations | median 324,100 ns |

The full-vs-narrow and clone-vs-encode comparisons are local measurement
boundaries. They are not converted into end-to-end FPS, CPU, network, or
operator-latency claims.

## Disposition

The current evidence does not justify changing lock semantics, moving the
engine publication guard, adding a shadow cache, or changing the snapshot
wire. In particular, the writer-contention result shows that the try-read
path correctly refuses the held writer rather than manufacturing a stale
answer; the full and narrow reads both wait for the same writer boundary.

The following remain separate, uncompleted measurements or design units:

- representative-show writer wait under real tick load;
- tick-time construction and authored/rendered ownership at a larger show
  shape;
- deep delta comparison and serialization/payload under a fixed multi-window
  workload;
- any optimization that changes ownership, lock order, wire shape, or
  observable snapshot semantics.

Those items require a fixed workload and before/after evidence before a
product change is justified. This checkpoint therefore closes measurement
revalidation only; it does not claim snapshot-performance completion, native
hardware acceptance, or whole-product completion.

## Current-main revalidation

The read-only measurement was repeated on current `main` at source HEAD
`045f3c8f8483ea85e5a2bfa85548415db446f701`. The preserved fixture remained
unchanged at 21,612 bytes with SHA-256
`45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`.

The exact Windows native procedure used Build Tools 14.44.35207, set
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to the absolute
`...14.44.35207...\\link.exe`, and verified that `where.exe link.exe` returned
that linker first. The engine benchmark also received the required preserved
fixture path through `SYNDOCAL_SNAPSHOT_BENCH_PROJECT`.

```text
cargo test -p engine --release --locked snapshot_output_read_benchmark -- --ignored --nocapture --test-threads=1
test result: ok. 5 passed; 0 failed; 0 ignored
clone_ns=3364500 encode_ns=9773300 clone_encode_ns=13746400
writer_contention full_median_ns=2522300 narrow_median_ns=2518800 try_median_ns=12200 full_successes=100 narrow_successes=100 try_successes=0
build_snapshot_ns=11151800
rendered_ns=1242000 authored_from_rendered_ns=1997400 both_ns=2010000
full_ns=13933600 narrow_ns=142200
```

The focused application and engine regressions also passed under the same
linker procedure:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 snapshot_sync -- --nocapture --test-threads=1
test result: ok. 9 passed; 0 failed; 2 ignored

cargo test -p engine --release --locked snapshot_read -- --test-threads=1
test result: ok. 4 passed; 0 failed; 2 ignored
```

An initial invocation without `SYNDOCAL_SNAPSHOT_BENCH_PROJECT` failed all
five opt-in benchmark cases at their explicit-fixture guard. This was a
command-input omission, not a product result; the corrected invocation above
passed without changing source or weakening assertions.

This current-main checkpoint remains measurement evidence only. It does not
close the larger snapshot-performance design boundary, native hardware
acceptance, or whole-product completion claims.
