# Cue-owned Effect transition acceptance (T19-D)

- Date: 2026-07-23
- Product boundary: lighting/VJ Cue recall; no internal 3D visualizer work
- Persistence contract: `CueEffectTarget.transition_ms: Option<u64>` with `serde(default)` and omission when unset

## Implemented contract

- Each Cue-owned Effect can independently select `Snap` (field absent) or a 0–600000 ms transition in Effect Recall.
- A direct Cue List/Scene GO captures the currently active instance with the same Effect ID before replacing it. The incoming runtime starts at the outgoing live result and advances to the incoming Cue-owned parameter state.
- Continuous DMX16 attributes interpolate linearly. Gobo, shutter, strobe, prism, macro, control, reset, mode and wheel-slot controls switch at the midpoint so the fade does not emit invalid in-between DMX ranges.
- Lighting transitions work for all independent saved Effect bodies because both precompiled runtimes are evaluated through the common Effect stack. LFO and Position Wave video targets use the same transition without cloning variable-length `VideoLayerState` data.
- The transition is activation-local. It survives command-time activation rebuilds, expires after its duration, and disappears with release/project reload. Unmatched Effect IDs and targets without Cue-owned params retain the existing immediate behavior.
- A Capture/Update Look refresh preserves an already-authored transition for the same Effect. Clearing the field in Save Recall restores the legacy JSON shape.
- If another GO arrives while a transition is active, the new transition starts from the previous Cue target runtime rather than retaining an unbounded nested blend. This keeps retrigger cost deterministic; continuity under rapid repeated GO remains part of the T23 operator/Art-Net rehearsal.

This is an output-domain crossfade between two Cue-owned parameter states. It deliberately does not invent a structural interpolation between incompatible generator bodies, path point counts, palettes or image rasters.

## Hot-path structure

- Previous and next runtimes, duration and source rate are resolved at Cue trigger time.
- The 44 Hz lighting path adds no fixture lookup, allocation, string normalization or parameter-body rebuild. It receives the already-known control's continuous/discrete policy and caches one transition progress value per activation/tick.
- The video Effect path uses a fixed-size Copy state for transition arithmetic. The pre-existing final state sanitizer remains unchanged.
- Completed transitions are removed once per engine tick, so steady-state Effect evaluation returns to the ordinary single-runtime path.

## Automated evidence

- protocol: 38/38 pass, including transition round-trip and absent-field legacy shape.
- engine: 389 pass + 1 manual benchmark ignored. Focused tests cover lighting midpoint, video midpoint, discrete midpoint switch, bounds, activation rebuild survival and expiration.
- Tauri: 325 pass + 9 hardware-dependent ignored. Strict project validation, save/history round-trip, absent-field persistence and Look recapture preservation are covered.
- frontend: TypeScript, Effect Recall helper gate, terminology, localization 2701/2701 and production build pass. Clearing a transition removes the property rather than emitting a new legacy-field shape.
- five viewport gate: 1920x1080, measured 1920x1032, 2048x1152, 1366x768 and 1280x720 render all eight Cue-owned Effect rows, eight contained transition controls and the authored 750 ms value with zero app/document scroll.
- operation-count regression: all 13 established tasks pass; Daslight-unmeasured tasks remain labelled `未計測`.
- release 64 mixed Effects x 200 fixtures x 6 attributes:
  - ordinary stack: p95 2.654 ms / p99 2.955 ms / max 3.460 ms;
  - deliberate worst case with all 64 Effects simultaneously transitioning: p95 9.576 ms / p99 10.596 ms / max 10.878 ms.

The ordinary 5/8/12 ms gate remains unchanged. The all-transition stress gate is separate at 12/16/20 ms and remains inside one 22.7 ms DMX tick. These are local software measurements, not physical fixture or Daslight same-host evidence.

## Remaining external/next gates

- Timeline Scene Blocks keep their independent block fade-in/fade-out and overlapping activation model; T19-D's same-Cue-List replacement transition applies to direct Scene/Cue GO.
- Physical fixtures, external Art-Net visualization and same-host Daslight comparison remain T23 evidence gates.
- Calibrated Amber/Lime/UV/multi-emitter policy remains T19-E.
