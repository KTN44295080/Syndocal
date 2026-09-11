# Snapshot performance revalidation on current main — 2026-09-12

## Scope

This checkpoint re-runs the existing isolated snapshot/output reader benchmark against the checked-out current `main` source. It does not add product behavior, change a performance budget, or claim whole-product/native/venue acceptance.

## Source and fixture identity

- Source branch: `main`
- Source HEAD at measurement: `8da8a431172dc6a7aef444182734b9eeccd20629`
- Fixture: `samples/phase1-mini-show.sdc`
- Fixture size: `21,612` bytes
- Fixture SHA-256: `45FBBFC1165C8A79BAEDBCBA6C44C99F101120E1814AB5E9D3AD14C4589F99DB`
- Build environment: MSVC Build Tools `14.44.35207`, x64; absolute Cargo linker pinned to `...\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`; `where link.exe` resolved that path first.
- Cargo: `1.97.1 (c980f4866 2026-06-30)`

## Command and result

The existing ignored benchmark module was run with the fixture path set explicitly:

```text
cargo test -p engine --release --locked benchmark_show_ -- --ignored --nocapture --test-threads=1
```

Result: **5 passed, 0 failed, 0 ignored, 0 measured; 1078 filtered out; exit code 0**.

Observed medians from the five benchmark cases:

```text
clone_ns=3762500 encode_ns=10327500 clone_encode_ns=14302000 payload_bytes=12224
writer_contention full_median_ns=2511200 narrow_median_ns=2523100 try_median_ns=11500 full_successes=100 narrow_successes=100 try_successes=0
build_snapshot_ns=11377000
rendered_ns=1163000 authored_from_rendered_ns=2043000 both_ns=2049000
full_video_read_ns=13968100 narrow_video_read_ns=138300
```

The benchmark process remained local to the engine test. No Syndocal GUI, native executable, external client, hardware device, physical output, or venue soak was started.

## Interpretation and nonclaims

This is current-source automated evidence for the isolated snapshot construction, serialization, reader-width, and writer-contention benchmark. It does not close `COV-PERFORMANCE-001`, because the ledger requirement is a fixed whole-product latency/frame/tick/resource envelope and maximum-condition venue soak on the pinned reference machine. It also does not establish app FPS, native-window behavior, GPU/resource ceilings, recording/output timing, cross-platform behavior, or physical acceptance.
