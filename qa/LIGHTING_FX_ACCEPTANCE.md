# Lighting FX Acceptance

Updated: 2026-07-13

## Implemented tranche

The existing ordered effect stack now has a seven-family quick-start library aligned to common lighting workflows:

| Library family | Current presets | Runtime primitive |
|---|---|---|
| Colour | Colour Spectrum | Multi-stop Color engine with continuous RGB/HSV interpolation |
| Chaser | Chase | Position Wave with square shape and stage direction |
| Move | Fan, Circle | Static Position Wave fan; paired Pan/Tilt LFO bundle |
| Value | Pulse, Shared, Flash, Random | LFO shapes and shared lighting/video target |
| Curve | Perlin, Curve Saw | Smooth noise and saw LFO curves |
| Mappings | Wave, Ball | Directional and radial Position Wave |
| Colour Mappings | Colour Chase | Multi-stop Color engine with ordered Sequence steps and deterministic fixture spread |

The rail uses eight keyboard-focusable buttons (`All` plus seven families) with `aria-pressed` state. The full library contains 13 cards. Colour presets cannot load their embedded compatibility target: both frontend and Tauri require an explicit current fixture, group or map selection. Color is a whole-fixture target, does not ask the operator to choose one scalar attribute, and rejects video targets.

`EffectKind::Color` is an independent production runtime, preset and project body. It supports two to eight ordered stops, Cycle/Bounce/Sequence/seeded Random algorithms, RGB/HSV-shortest/HSV-longest interpolation, free or beat-synced period, phase, fixture spread and the existing ordered Override/Add/Multiply stack semantics. Add and Update wait until the published snapshot contains the mutation; failed publication rolls the change back instead of allowing Undo/history to race an unpublished effect.

Fixture bindings are compiled when fixtures are patched. A color is evaluated once per effect/fixture/tick and reused for all its components; RGBW/CMY/HSV conversions are enabled per binding so ordinary RGB and wheel fixtures skip unused transforms. Supported software mappings are complete RGB/ColorAdd triplets, RGBW with minimum-channel white extraction, complete CMY, complete HSV/HSB, and color wheels with at least two parseable hex or named-color physical slots (nearest slot in OKLab). Static/shake ranges for the same physical slot remain addressable but count once for support/ranking. Secondary wheels use only an explicit Open/Clear/White or near-white slot as neutral. Mixed groups retain supported fixtures and skip unsupported ones; a new target with zero compatible fixtures is rejected, while an existing group-target effect survives a temporary zero-target state and rebinds after group/patch recovery. Amber, Lime, UV and warm/cold-white outputs not covered by a calibrated model are deliberately held at zero rather than receiving guessed values.

The former vertically serialized Library/Form/Graph/Stack view has also been replaced with a dedicated full-window production desk. Effects mode now hides the unrelated live overview and generic stage, and keeps `Effect Library | Inspector | Live Rack` visible as three continuous panes. Recipe selection/actions and effect range/apply actions are fixed to the bottom of their panes; only pane contents scroll. Stack/Graphs are explicit rack tabs. The visual system uses neutral graphite surfaces, thin dividers and the Syndocal orange state accent rather than copying Daslight colours or controls.

Automated evidence:

- every new embedded `.effect` parses and passes the production preset validator;
- spectrum and colour chase contain only a Color body, retarget the complete fixture, and preserve every palette/algorithm/interpolation/spread field;
- project validation accepts mixed RGBW/wheel/unsupported groups, but rejects a Color target with zero compatible fixtures;
- GDTF `ColorAdd_*`, `ColorSub_*`, `ColorRGB_*` and `HSB_*` aliases are fixed by tests against the canonical [AttributeDefinitions.xml](https://github.com/open-stage/python-gdtf/blob/master/AttributeDefinitions.xml) naming;
- engine tests cover validation, all four algorithms, opposite HSV hue paths, RGBW/CMY/HSV/wheel conversion, fixture spread and group changes, project round-trip, stack order, publication rollback and a 200-fixture/64-effect evaluation regression;
- backend target-required aliases reject an absent override;
- localization remains 100%;
- 1280x720, 1366x768 and 2048x1152 browser acceptance requires the three-pane desk, both fixed action docks, Stack/Graphs switching, 8 family buttons, 13 total cards, 2 target-required cards and the filtered Colour view. It also switches to Multi-color, adds/removes a stop, changes Random/HSV-longest, verifies the gradient and asserts that scalar Attribute/waveform/video controls are absent with zero horizontal overflow. The target/draft/status summaries must all identify the current whole-fixture Color edit and must not retain stale LFO/Position Wave text;
- local generated screenshots are under `app/target/qa/color-fx-final/`, including `control-edit-effects-color-editor-1366x768.png`.

## Competitive boundary

The Color tranche is a real independent engine, not the earlier single-attribute alias. The other six families still share the existing LFO/Position Wave primitives, so the full library must **not** be described as Daslight 5 FX parity.

Remaining engine work:

1. Colour fidelity: fixture colorimetry/calibration matrices, additive Amber/Lime/UV emitters, CTO/CTB and measured RGBW/CMY/wheel output on representative physical fixtures.
2. Cue recall: normal Effect enabled state is not yet stored per Cue; Color must not be called cue-programmable until `CueEffectTarget` GO/Back recall and deletion cleanup exist.
3. Chaser: explicit ordered fixture steps, grouping, wings, direction/bounce, duty cycle and live size/phase controls.
4. Move: shape library, focus point, pan/tilt calibration, rotation and per-fixture phase tools.
5. Curve/Value: editable curve points, channel functions, relative/absolute modes and live modulation rack controls.
6. Mappings: 2D image/text/video sampling, matrix cells, UV transforms and spatial preview. The current Colour Chase is palette sequencing, not image/video RGB sampling.
7. Rack operation: named racks, multi-select bypass, reset/hold, macro controls, rack virtualization and a release-build 44 Hz/p95/p99 Color-stack benchmark/soak.

Only after these are implemented and real fixtures confirm color/movement output can the seven-family gap be considered closed.
