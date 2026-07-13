# Lighting FX Acceptance

Updated: 2026-07-13

## Implemented tranche

The existing ordered effect stack now has a seven-family quick-start library aligned to common lighting workflows:

| Library family | Current presets | Runtime primitive |
|---|---|---|
| Colour | Colour Spectrum | Multi-stop Color engine with continuous RGB/HSV interpolation |
| Chaser | Chase | Independent ordered-step Chaser engine |
| Move | Fan, Circle | Static Position Wave fan; paired Pan/Tilt LFO bundle |
| Value | Pulse, Shared, Flash, Random | LFO shapes and shared lighting/video target |
| Curve | Perlin, Curve Saw | Smooth noise and saw LFO curves |
| Mappings | Wave, Ball | Directional and radial Position Wave |
| Colour Mappings | Colour Chase | Multi-stop Color engine with ordered Sequence steps and deterministic fixture spread |

The rail uses eight keyboard-focusable buttons (`All` plus seven families) with `aria-pressed` state. The full library contains 13 cards. Colour presets cannot load their embedded compatibility target: both frontend and Tauri require an explicit current fixture, group or map selection. Color is a whole-fixture target, does not ask the operator to choose one scalar attribute, and rejects video targets.

`EffectKind::Color` is an independent production runtime, preset and project body. It supports two to eight ordered stops, Cycle/Bounce/Sequence/seeded Random algorithms, RGB/HSV-shortest/HSV-longest interpolation, free or beat-synced period, phase, fixture spread and the existing ordered Override/Add/Multiply stack semantics. Add and Update wait until the published snapshot contains the mutation; failed publication rolls the change back instead of allowing Undo/history to race an unpublished effect.

Fixture bindings are compiled when fixtures are patched. A color is evaluated once per effect/fixture/tick and reused for all its components; RGBW/CMY/HSV conversions are enabled per binding so ordinary RGB and wheel fixtures skip unused transforms. Supported software mappings are complete RGB/ColorAdd triplets, RGBW with minimum-channel white extraction, complete CMY, complete HSV/HSB, and color wheels with at least two parseable hex or named-color physical slots (nearest slot in OKLab). Static/shake ranges for the same physical slot remain addressable but count once for support/ranking. Secondary wheels use only an explicit Open/Clear/White or near-white slot as neutral. Mixed groups retain supported fixtures and skip unsupported ones; a new target with zero compatible fixtures is rejected, while an existing group-target effect survives a temporary zero-target state and rebinds after group/patch recovery. Amber, Lime, UV and warm/cold-white outputs not covered by a calibrated model are deliberately held at zero rather than receiving guessed values.

`EffectKind::Chaser` is also an independent production runtime, preset and project body. Its ordered steps can address fixtures, hierarchical groups or explicit gaps and can scale one to sixteen independently ranged feature attributes. Forward, Reverse, Bounce and seeded Random traversal share deterministic engine/UI ordering; Pixels on, Wings, duty, Fading/Overlap, Size, Phase and fixture spread are live-editable, with free-time or beat-synced step clocks and the existing Override/Add/Multiply stack modes. Mixed fixture selections retain each feature on compatible fixtures, while unsupported fixture/feature pairs are skipped. The runtime precompiles fixture/step levels and feature bindings, caches one normalized step level per fixture/effect/tick, and avoids allocation in the per-attribute hot path.

The Chaser inspector keeps a reactive 24-cell preview, ordered step editor, gap insertion, feature coverage, min/max ranges and movement/replacement actions in the center pane. Step editing is paged at eight rows and the live rack at ten effects so large drafts and stacks do not expand the desktop. Changing Type while editing intentionally starts a new draft rather than replacing a different effect kind in place; engine and Tauri guards independently reject wrong-kind updates.

Cues now store a selective list of `CueEffectTarget { effect_id, enabled }` values. GO, Back, direct trigger, pre-wait completion and follow progression apply each listed state once and leave unlisted Effects unchanged. `Effects Only` can create a pure Effect Cue; scoped Lighting, Video, fixture and group updates replace only eligible Effect entries while preserving out-of-scope Recall. Deleting an Effect removes its Cue references, legacy files default to an empty list, and the user-facing `.sdc` loader rejects unknown or duplicate references instead of silently changing a show. Effect enable, Cue creation, Details, Look update and Recall-only save acknowledge the published snapshot before the UI commits Undo history; Details applies metadata, Parts, Mark and MIB atomically. The Cue editor deliberately separates `Save Details`, `Save Recall` and `Update Look` so a metadata edit cannot recapture a live look and a Recall edit cannot overwrite fixture/video targets.

The former vertically serialized Library/Form/Graph/Stack view has also been replaced with a dedicated full-window production desk. Effects mode now hides the unrelated live overview and generic stage, and keeps `Effect Library | Inspector | Live Rack` visible as three continuous panes. Recipe selection/actions and effect range/apply actions are fixed to the bottom of their panes; only pane contents scroll. Stack/Graphs are explicit rack tabs. The visual system uses neutral graphite surfaces, thin dividers and the Syndocal orange state accent rather than copying Daslight colours or controls.

Automated evidence:

- every new embedded `.effect` parses and passes the production preset validator;
- spectrum and colour chase contain only a Color body, retarget the complete fixture, and preserve every palette/algorithm/interpolation/spread field;
- project validation accepts mixed RGBW/wheel/unsupported groups, but rejects a Color target with zero compatible fixtures;
- GDTF `ColorAdd_*`, `ColorSub_*`, `ColorRGB_*` and `HSB_*` aliases are fixed by tests against the canonical [AttributeDefinitions.xml](https://github.com/open-stage/python-gdtf/blob/master/AttributeDefinitions.xml) naming;
- engine tests cover validation, all four algorithms, opposite HSV hue paths, RGBW/CMY/HSV/wheel conversion, fixture spread and group changes, project round-trip, stack order, publication rollback and a 200-fixture/64-effect evaluation regression;
- Cue tests cover selective GO/Back recall, pre-wait/follow timing, unlisted-state preservation, pure Effect bodies, scoped merge/clear, deletion cleanup, project/legacy round-trip, strict loader validation and publication rollback;
- backend target-required aliases reject an absent override; Chase is the third target-required recipe and a group is expanded in patch order for the built-in fixture-index sample;
- localization remains 100%;
- 1280x720, 1366x768 and 2048x1152 browser acceptance requires the three-pane desk, both fixed action docks, Stack/Graphs switching, 8 family buttons, 13 total cards, 3 target-required cards and the filtered Colour view. It also switches to Multi-color, adds/removes a stop, changes Random/HSV-longest, verifies the gradient and asserts that scalar Attribute/waveform/video controls are absent with zero horizontal overflow. The target/draft/status summaries must all identify the current whole-fixture Color edit and must not retain stale LFO/Position Wave text;
- the same browser run opens Chaser, verifies 6 ordered steps, 2 features, Reverse, 2 active pixels, Size/Fading edits, six step replacement actions, the fixed editor footer and zero outer/inner horizontal overflow at all three reference sizes. A 500-effect rack fixture renders only the current 10-row page;
- Cue browser acceptance opens the compact live editor at all three reference widths, verifies the three distinct save actions, Recall selection/state controls, Effects Only capture and node-graph-only Store availability with zero outer overflow; a 500-Effect/12-Cue fixture requires zero rows for closed Cue editors and at most 48 rows per open/global editor;
- local generated screenshots are under `app/target/qa/chaser-fx/`, including `control-edit-effects-chaser-editor-1366x768.png`.

## Competitive boundary

The Color and Chaser tranches are real independent engines, not single-attribute aliases. The other five families still share the existing LFO/Position Wave primitives, so the full library must **not** be described as Daslight 5 FX parity.

Remaining engine work:

1. Colour fidelity: fixture colorimetry/calibration matrices, additive Amber/Lime/UV emitters, CTO/CTB and measured RGBW/CMY/wheel output on representative physical fixtures.
2. Cue modulation: Effect ON/OFF recall is complete, but per-Cue Effect parameter morph/fade and source-linked reusable scene blocks are not.
3. Move: shape library, focus point, pan/tilt calibration, rotation and per-fixture phase tools.
4. Curve/Value: editable curve points, channel functions, relative/absolute modes and live modulation rack controls.
5. Mappings: 2D image/text/video sampling, matrix cells, UV transforms and spatial preview. The current Colour Chase is palette sequencing, not image/video RGB sampling.
6. Rack operation: named racks, multi-select bypass, reset/hold, macro controls, rack virtualization and release-build 44 Hz/p95/p99 Color/Chaser stack benchmarks and soaks.
7. Chaser field acceptance: the software path is complete, but real-fixture direction/order, beat timing, mixed-attribute output and one-hour DMX load still require representative rigs. The editor preview is deterministic parameter visualization, not an engine-telemetry playhead.

Only after these are implemented and real fixtures confirm color/movement output can the seven-family gap be considered closed.
