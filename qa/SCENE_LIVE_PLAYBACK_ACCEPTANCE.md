# Scene Live Playback Acceptance

- Date: 2026-07-23
- Tranche: T20-A / T20-B
- Status: software PASS; native Shinkan and external Art-Net visual confirmation remain T23 gates

## Product boundary

T20-A adds per-scene playback Direction and Cue Step Segment to the existing Scene Live speed, size, phase and flash controls. T20-B adds direct group Dimmer, Strobe and Solo to a separate lighting Live Mixer strip and to Touch. Scene controls and group controls remain distinct.

## Persistence and reset contract

- `CueLiveDirection` is `Authored`, `Forward`, `Reverse` or `Bounce`. `Authored` is the default and is omitted from JSON.
- `segment` is one-based; `0` means Auto/normal playback. Zero is omitted from JSON.
- A legacy T17 settings object serializes back to the same JSON byte shape after load.
- Authored defaults live on the Cue. The live latch remains runtime-only and is removed from engine persistence, Tauri save snapshots and frontend project comparisons.
- Release, a fresh manual trigger and project load return to the authored defaults.
- Live Mixer group strobe is runtime-only. `SubmasterSummary.strobe_hz` and `strobe_fixture_count` are additive default-zero fields omitted from legacy JSON. Engine persistence, Tauri save normalization and frontend storage comparison each remove the live fields. Project load resets every group to strobe Off.

## Runtime contract

Direction-aware Cue-owned Chaser, Move, Value, Curve, Mapping and Colour Mapping bodies are transformed while activations are built. Reverse Position Wave negates its compiled spatial vector. Static Mapping remains static. LFO and Colour have no authored direction field and therefore keep their authored traversal.

Manual Cue Step playback owns a fixed compiled sequence. Reverse reverses authored Step order. Bounce traverses the far endpoint once and avoids a duplicate endpoint when the sequence finishes. Segment selection jumps to the first compiled occurrence of that authored Step. Timeline and child-Timeline Step activations retain authored order, so Scene Live cannot mutate programmed timeline playback.

The 44 Hz path does not perform a direction branch, map lookup, sequence clone or allocation. Step evaluation now reads the fixed sequence owned by its activation and removes the former per-control Cue lookup. Segment adds only a precomputed realtime clock offset.

Group strobe resolves only canonical GDTF function attributes `Shutter...Strobe` or `Strobe` with a finite, non-zero physical frequency interval. Display names, channel names and raw DMX ranges never infer support. Pulse/random variants and ambiguous equal-distance functions fail closed. The requested 0-30 Hz rate is clamped to the nearest authored physical interval and interpolated into its DMX interval, including reversed physical ranges. Nested active groups resolve once per fixture with the highest requested rate.

Compatible targets are rebuilt only on strobe commands and patch/group topology changes, grouped by `(attribute, DMX value)` and sorted by fixture ID. The 44 Hz path performs no allocation and only checks the compact precompiled binding after Cue/effect/node-graph evaluation, making Live Mixer the final operator override. Zero clears the binding and immediately exposes the authored/effected Shutter value. Existing group Solo remains the final fixture-selection gate and is now directly accessible beside Strobe.

## Automated evidence

- Protocol legacy/additive JSON tests, including byte-identical default-zero Submaster strobe fields: PASS; 41/41.
- Engine direction transform, manual-vs-Timeline isolation, rendered Segment output, group strobe physical interpolation, nested maximum-rate arbitration, unsupported/ambiguous fail-closed behavior, reset and persistence tests: PASS; full engine suite 399 passed, 1 manual benchmark ignored.
- Tauri project-save/runtime-latch test: PASS; full Tauri suite 326 passed, 9 hardware-dependent ignored.
- Scene Live and group strobe helper tests and localization: PASS; Japanese static UI coverage 2721/2721.
- Focused Matrix/Touch viewport gate: PASS at 1920x1080, measured 1920x1032, 2048x1152, 1366x768 and 1280x720.
- Focused Live Mixer group gate: PASS at the same five resolutions. Desktop Strobe latched 0 -> 12 Hz -> 0, direct Solo toggled true -> false, and Touch Strobe latched 18 Hz -> 0 with 2/2 GDTF-compatible viewport fixtures.
- Full viewport matrix: PASS with no reported failed fixture.
- Release Cue Step stack, four simultaneous directions x eight Steps x 200 fixtures: p95 0.276 ms, p99 0.340 ms, max 0.365 ms, below the fixed 10/14/18 ms gate and inside one 22.7 ms tick.
- Release group strobe production path, 200 fixtures x Dimmer/Pan/Shutter controls: baseline p95 0.004 / p99 0.007 / max 0.017 ms; active p95 0.006 / p99 0.010 / max 0.012 ms, below the fixed 2/3/5 ms gate.
- Production frontend build: PASS; main entry 490.17 kB (145.12 kB gzip).

The first combined workspace run saw the unrelated 128,000 Scene Block scheduling test at 52.55 ms against its 50 ms debug-host gate. An immediate isolated rerun of the same test passed at 28.32 ms. This is recorded as host-load variance, not hidden as a green first run.

A later combined workspace run reached the `video` all-features test process but Windows terminated that process once with `STATUS_ACCESS_VIOLATION` (`0xc0000005`), without a Rust assertion failure. Immediate isolated `video` runs passed both the default suite (110 passed, 1 ignored) and all-features suite (114 passed, 1 ignored); a fresh `cargo test --workspace` then completed with exit code 0, including the same all-features video suite. The one-off process termination is retained here as transient host/process evidence rather than reported as a first-run green.

## Open evidence

- Native WebView2 run against the real Shinkan project for Direction, Segment, Live Mixer Strobe and Solo.
- Art-Net capture in the selected external visualizer for Cue Step direction/segment, group Strobe/Solo and representative Cue-owned FX.
- Physical node/fixture timing and venue endurance remain external dependencies and are not part of this software PASS.
