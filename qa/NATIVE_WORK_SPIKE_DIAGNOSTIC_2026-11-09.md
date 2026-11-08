# Native work-spike diagnostic — 2026-11-09

Base: `main` at `5e9fe8ad85aa38aa92ca585621e1222d0ea6731a`.
The [thirty-minute soak](NATIVE_60FPS_SOAK_2026-11-08.md) remains **FAIL**:
minute 13 retained a 35,604 µs native-work sample above the unchanged 33,333 µs
G3 bound. Its CPU/GPU/decoder/OS cause is unknown. Resident default stays OFF.

## Bounded diagnostic

The existing full trace fills its 3,072-parent capacity in about 51 seconds,
so it cannot capture a first spike thirteen minutes later. A separate coarse
recorder is compiled only with `qa-native-frame-timing` and activated only by
`SYNDOCAL_QA_NATIVE_SPIKE_OUT=<absolute-new-file>`. It is mutually exclusive
with `SYNDOCAL_QA_FRAME_TIMING_OUT`. Without the feature the hooks are absent;
with neither environment variable, the new recorder does not initialize TLS
or its clock.

The recorder counts every admitted iteration and retains the first 16 selected
records in fixed storage below 16 KiB. Further matches increment overflow.
It performs no allocation, file I/O, serialization or control-plane call in the
frame loop. Export uses the existing Stop/owner-retirement/ACK boundary and
`create_new`; existing files are never overwritten. Partial, unobserved,
unprepared and invalid observations remain explicit, rather than disappearing.

Each selected record has the exact pacing-excluded work duration and sample
Instant used by G3, a work-sample sequence and validation epoch. Last Presented
request sequence is separate; it must not be presented as the identity of that
work sample. Coarse extent, permit, retirement/configure, prepare, present and
metrics-lock intervals are CPU wall time. Present includes the separately
reported pacing subset. Neither the buckets nor their sum prove GPU execution
time or causation. A frame admitted before Stop can finish afterwards.

The retained harness patch `C5_SPIKE_DIAGNOSTIC_2026-11-09.patch` applies after
`C5_HARNESS_2026-11-08.patch` to the protected ignored C5 harness. Its
`--diagnostic-spikes off|on` modes use the same timing executable and resident
GPU ON workload, one MCP-controlled window, 15 s warmup and at most 1,800 s.
OFF/ON here controls the observer. Both always report
`DIAGNOSTIC-NOT-GATING`; success means consistent diagnostic evidence and clean
lifecycle, never C5/soak acceptance. Ordinary C5 and soak gates are unchanged.

## Verified checkpoint

Evidence lives under `target/qa/native-spike-diagnostic-20261109/`:

- Exact MSVC 14.44.35207 pin and PATH-first checks were printed and verified.
- `cargo test -p video --features qa-native-frame-timing native_frame_ --
  --test-threads=1`: 23 passed, zero failures/ignored; 702 unrelated tests
  filtered. Covers fixed capacity, 100,000 fast iterations before a spike,
  shared metric clock/pacing, disabled TLS, handoff, Stop/in-flight completion,
  explicit partials and post-ACK create-new export.
- `pnpm --dir app tauri build --no-bundle --config <isolated QA config>`:
  release build succeeded. Source hashes match `normal-source-before.json`.
  The feature-OFF EXE SHA-256 is
  `b7662f58cddb60130b9093dd2f859fe2a4f1620e3064a2dcb511b44d4b08dad9`.
- `cargo check -p syndocal --features qa-native-frame-timing,ndi`: completed
  successfully (`check-feature-crlf.log`). The first wrapper attempt rejected
  mixed LF/CRLF batch input before Cargo; it is retained as `check-feature.log`
  and does not count as a compile result. The corrected CRLF wrapper verified
  the exact pin/PATH before Cargo.
- `normal-native-proof.json`: exactly one responsive, maximized native
  `Syndocal` window at 2560×1600; rendered root populated. Video-only QA profile
  stayed effective Standby with lighting/video denied over three ownership
  polls; no live video outputs. Graceful exit 0, no forced termination, CDP
  9287 free. No physical output was requested.
- Synthetic harness tests cover diagnostic admission, malformed schema,
  clock/identity/counter consistency, Stop boundary and failure exit. Eleven
  invalid session-driver configurations reject before profile/process work.
  Existing C5/soak analyzer regressions: 46 passed.
- Stable Rust diff and harness changes independently reviewed. The review's
  Stop-boundary and unit-label findings were corrected and retested.

Compiler warnings in the completed video tests, feature-OFF release build and
feature-ON app check: zero. No new baseline compile was run, so a numerical
baseline/delta is not inferred. Feature-ON native diagnostic execution remains
pending. Observer cost and spike reproduction have not yet been measured.

Protected `target/release/syndocal.exe` remains SHA-256
`1d8e757ea8e7b1ffee8a51496d1c4ef2aa0399148a373c55313f252f1212d228`.
Unrelated Vite directories and prior failed/invalid evidence remain untouched.

Next: prepare a receipted committed timing artifact, compare short observer
OFF/ON runs, then collect a long diagnostic.
Only after attribution and any justified fix can a fresh formal acceptance
campaign be considered. Physical scanout and release gates remain separate.
