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
