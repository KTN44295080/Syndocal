# Syndocal Samples

This directory contains small local assets for repeatable handoff and smoke checks.

## Phase 1 Mini Spot

File: `phase1-mini-spot.fixture`

This is a custom fixture profile, not a GDTF profile. It is intentionally tiny so Phase 1 patch/control checks do not depend on external downloads.

### Expected Footprint

Patch at Universe 0, Address 1:

| Channel | Attribute | Width |
| --- | --- | --- |
| 1 | Dimmer | 8-bit |
| 2 | ColorRed | 8-bit |
| 3 | ColorGreen | 8-bit |
| 4 | ColorBlue | 8-bit |
| 5-6 | Pan | 16-bit |
| 7-8 | Tilt | 16-bit |

### Manual Smoke Flow

1. Open Syndocal.
2. Go to Setup -> Profiles or the custom fixture profile loader.
3. Load `samples/phase1-mini-spot.fixture`.
4. Patch one fixture at Universe 0, Address 1.
5. Go to Control.
6. Set Dimmer to full.
7. Set RGB values and move Pan/Tilt.
8. Confirm DMX Raw for Universe 0 updates channels 1-8 as expected.
9. If an Art-Net receiver or loopback test is available, send output and confirm the same channel values arrive.

### Targeted Check

The sample is covered by this Rust test:

```powershell
cargo test -p syndocal phase1_smoke_fixture_sample_is_valid
```

## Phase 1 Mini Show

File: `phase1-mini-show.sdc`

This is a self-contained Syndocal project file using the same mini spot profile. It contains one patched fixture, one cue, one Art-Net route, one display output with a 16:9 projector mapping preset, one 2D stage map preset, and three 2D stage reference objects:

- `Main Deck` (`Stage`)
- `Front Truss` (`Truss`)
- `Projection Screen` (`Screen`)

Use the Setup -> Mapping view to confirm that fixtures, the projector surface, the `Mini Venue` stage map preset, and stage reference objects load together. Stage objects can be moved, resized, and rotated directly in the 2D map, saved with stage map presets, and used to fit the selected projector for a quick projection-mapping starting point.

Use it when the project loader itself needs a quick handoff smoke check:

```powershell
cargo test -p syndocal phase1_smoke_project_sample_is_valid
cargo test -p syndocal phase1_smoke_project_sample_loads_into_engine_and_renders_cue
cargo test -p syndocal phase1_smoke_project_sample_sends_cue_to_artnet_loopback
```

## Effect Presets

Files:

- `front-dimmer-pulse.effect`
- `front-dimmer-wave.effect`

These are small reusable effect presets for the `Front` group used by `phase1-mini-show.sdc`. The pulse preset is an LFO-driven dimmer look. The wave preset is a position-wave dimmer look that uses the fixture's 2D/3D patch position as the phase source.

Targeted check:

```powershell
cargo test -p syndocal sample_effect_presets_are_valid_for_phase1_mini_show
```
