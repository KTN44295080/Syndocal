# Lighting FX Acceptance

Updated: 2026-07-13

## Implemented tranche

The existing ordered effect stack now has a seven-family quick-start library aligned to common lighting workflows:

| Library family | Current presets | Runtime primitive |
|---|---|---|
| Colour | Colour Spectrum | Saw LFO retargeted to a selected colour/hue/wheel/emitter attribute |
| Chaser | Chase | Position Wave with square shape and stage direction |
| Move | Fan, Circle | Static Position Wave fan; paired Pan/Tilt LFO bundle |
| Value | Pulse, Shared, Flash, Random | LFO shapes and shared lighting/video target |
| Curve | Perlin, Curve Saw | Smooth noise and saw LFO curves |
| Mappings | Wave, Ball | Directional and radial Position Wave |
| Colour Mappings | Colour Chase | Position Wave retargeted to a selected colour attribute |

The rail uses eight keyboard-focusable buttons (`All` plus seven families) with `aria-pressed` state. The full library contains 13 cards. Colour-only presets cannot load their embedded compatibility target: both frontend and Tauri require an explicit current target, and the frontend rejects non-colour attributes. This prevents a preset named “Colour” from silently driving a dimmer.

The former vertically serialized Library/Form/Graph/Stack view has also been replaced with a dedicated full-window production desk. Effects mode now hides the unrelated live overview and generic stage, and keeps `Effect Library | Inspector | Live Rack` visible as three continuous panes. Recipe selection/actions and effect range/apply actions are fixed to the bottom of their panes; only pane contents scroll. Stack/Graphs are explicit rack tabs. The visual system uses neutral graphite surfaces, thin dividers and the Syndocal orange state accent rather than copying Daslight colours or controls.

Automated evidence:

- every new embedded `.effect` parses and passes the production preset validator;
- spectrum and colour chase retarget to `ColorRed` while preserving Saw and mapping parameters;
- backend target-required aliases reject an absent override;
- localization remains 100%;
- 1280x720, 1366x768 and 2048x1129 browser acceptance requires the three-pane desk, both fixed action docks, Stack/Graphs switching, 8 family buttons, 13 total cards, 2 target-required cards and the filtered Colour view;
- the checked screenshots are under `target/qa/fx-desk-redesign/`, including `control-edit-effects-1366x768.png`.

## Competitive boundary

This is a discoverability and one-click workflow improvement over the former unclassified sample list. It is **not** seven independent engine implementations and must not be described as Daslight 5 FX parity.

Remaining engine work:

1. Colour: multi-stop palettes, shortest/longest hue paths, random/sequence algorithms and fixture color-space conversion.
2. Chaser: explicit ordered fixture steps, grouping, wings, direction/bounce, duty cycle and live size/phase controls.
3. Move: shape library, focus point, pan/tilt calibration, rotation and per-fixture phase tools.
4. Curve/Value: editable curve points, channel functions, relative/absolute modes and live modulation rack controls.
5. Mappings: 2D image/text/video sampling, matrix cells, UV transforms and spatial preview.
6. Colour Mappings: true RGB sampling mapped through each fixture's available emitters, not a single attribute wave.
7. Rack operation: named racks, multi-select bypass, reset/hold, macro controls and measured large-stack latency.

Only after these are implemented and real fixtures confirm color/movement output can the seven-family gap be considered closed.
