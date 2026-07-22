# Scene Live Playback Acceptance

- Date: 2026-07-23
- Tranche: T20-A
- Status: software PASS; native Shinkan and external Art-Net visual confirmation remain T23 gates

## Product boundary

T20-A adds per-scene playback Direction and Cue Step Segment to the existing Scene Live speed, size, phase and flash controls. It does not relabel Live Mixer group controls as scene controls. Group strobe and the already implemented group solo path are T20-B.

## Persistence and reset contract

- `CueLiveDirection` is `Authored`, `Forward`, `Reverse` or `Bounce`. `Authored` is the default and is omitted from JSON.
- `segment` is one-based; `0` means Auto/normal playback. Zero is omitted from JSON.
- A legacy T17 settings object serializes back to the same JSON byte shape after load.
- Authored defaults live on the Cue. The live latch remains runtime-only and is removed from engine persistence, Tauri save snapshots and frontend project comparisons.
- Release, a fresh manual trigger and project load return to the authored defaults.

## Runtime contract

Direction-aware Cue-owned Chaser, Move, Value, Curve, Mapping and Colour Mapping bodies are transformed while activations are built. Reverse Position Wave negates its compiled spatial vector. Static Mapping remains static. LFO and Colour have no authored direction field and therefore keep their authored traversal.

Manual Cue Step playback owns a fixed compiled sequence. Reverse reverses authored Step order. Bounce traverses the far endpoint once and avoids a duplicate endpoint when the sequence finishes. Segment selection jumps to the first compiled occurrence of that authored Step. Timeline and child-Timeline Step activations retain authored order, so Scene Live cannot mutate programmed timeline playback.

The 44 Hz path does not perform a direction branch, map lookup, sequence clone or allocation. Step evaluation now reads the fixed sequence owned by its activation and removes the former per-control Cue lookup. Segment adds only a precomputed realtime clock offset.

## Automated evidence

- Protocol legacy/additive JSON test: PASS.
- Engine direction transform, manual-vs-Timeline isolation, rendered Segment output, reset and persistence tests: PASS; full engine suite 395 passed, 1 manual benchmark ignored.
- Tauri project-save/runtime-latch test: PASS; full Tauri suite 326 passed, 9 hardware-dependent ignored.
- Scene Live helper test and localization: PASS; Japanese static UI coverage 2713/2713.
- Focused Matrix/Touch viewport gate: PASS at 1920x1080, measured 1920x1032, 2048x1152, 1366x768 and 1280x720.
- Full viewport matrix: PASS with no reported failed fixture.
- Release Cue Step stack, four simultaneous directions x eight Steps x 200 fixtures: p95 0.276 ms, p99 0.340 ms, max 0.365 ms, below the fixed 10/14/18 ms gate and inside one 22.7 ms tick.

The first combined workspace run saw the unrelated 128,000 Scene Block scheduling test at 52.55 ms against its 50 ms debug-host gate. An immediate isolated rerun of the same test passed at 28.32 ms. This is recorded as host-load variance, not hidden as a green first run.

## Open evidence

- T20-B: direct group strobe plus group solo acceptance in the Live Mixer boundary.
- Native WebView2 run against the real Shinkan project for Direction and Segment.
- Art-Net capture in the selected external visualizer for Cue Step direction/segment and representative Cue-owned FX.
- Physical node/fixture timing and venue endurance remain external dependencies and are not part of this software PASS.
