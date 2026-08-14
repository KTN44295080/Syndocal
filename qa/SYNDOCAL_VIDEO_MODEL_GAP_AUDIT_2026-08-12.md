# Syndocal Video Model Gap Audit — 2026-08-12

Status: static current-state audit. This is deliberately separate from the target model in `SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md`.

## Verdict

The current product is still fundamentally `one VideoLayerSummary = one embedded VideoSourceSummary + one layer state/effect stack`. UI labels such as Clip, Take, and A/B Deck must not be treated as proof of a reusable media catalog, per-layer clip slots, or a first-class transition system.

| Product target | Current state | Required change |
| --- | --- | --- |
| Reusable MediaAsset library | Missing. Import probes a path and creates one new layer per file. No asset id, content hash, availability, or relink state. | Add authored MediaAsset catalog, machine-local thumbnail/decode caches, background hash/metadata state, and relink transaction. |
| Multiple clip slots per layer | Missing. A layer owns one source; UI launch/take targets layer id. | Add stable clip-slot ids, ordered per-layer banks, authored default, runtime active/queued state, and acknowledged slot commands. |
| Clip FX | Missing because clip scope does not exist. | Add clip-local ordered chain/overrides independent of the layer chain. |
| Layer FX | Partial. Built-in transform/color/FX and an ordered ISF stack exist. | Preserve exact legacy render order while generalizing to a common scoped chain and presets/mappings. |
| Transition FX | Missing. | Add an ordered transition-only chain with preset, matte, curve, timing, and error isolation. |
| Clip Take Transition | Missing as a first-class object. Current Take targets a layer and supports only cut/linear fade duration. | Add active/queued slot transition on one layer with ms/beat/bar timing and interruption policy. |
| Layer Transition Bus | Missing. Exclusive Take fades every other active layer; A/B is non-persistent layer opacity control. | Add explicit opt-in bus membership, from/to, group support, non-member preservation, curve/matte/FX, quantization, reverse/interruption, and stable ids. |
| Group/Composition FX | Composition layer routing exists; group object and chain do not. | Add authored groups or scoped composition chain without conflating output mapping with FX. |
| Import by picker/drop/direct layer drop | Picker/batch only; current desktop drop routes Timeline audio or `.sdc`, otherwise invalid. | Add target-aware media routing and import-and-assign transactions. |
| Missing media/relink/hash | Missing. A path may survive load, but render/decode failure can fence/close output. | Add visible availability state, content identity, relink, last-valid/fallback policy, and non-global failure isolation. |
| BPM binding | Layer playback loop sync exists. | Extend to slot launch quantization and transition timing. |
| Audio Reactive / Auto VJ | Rich layer VideoParam modulation and safe-zero runtime exist; Auto VJ selects layers with a fade. | Bind stable slot/bus/effect ids and expose complete authoring/status; prove native and long-run behavior. |

## Existing strengths to preserve

- Multiple authored video layers and compositions.
- Transform, opacity, blend, color, fixed FX, and ordered ISF stages.
- Shared BPM clock phase and layer loop synchronization.
- Audio feature extraction and loss-to-safe-zero behavior.
- Auto VJ state/status and clock/onset triggers.
- Two physical Display output configuration, mapping, and output-ownership fencing.

These are foundations. They do not by themselves prove the target media workflow or SynapseRack superiority.

## SynapseRack official comparison boundary

Official current material documents:

- layer compositing, 100+ real-time effects, nodes, MIDI/OSC/DMX, group/layer nodes, Audio/Tempo, and unlimited layers in Pro: https://synapserack.com/en/
- direct file/folder drag-and-drop import: https://synapserack.com/en/docs/load-video/
- Layer playback, crossfade loop, Beat Sync, transform, blend, and source types: https://synapserack.com/en/docs/window/layer
- GroupLayer and layer hierarchy: https://synapserack.com/en/docs/window/group-layer/
- source directory, ContentsBrowser thumbnails, two-layer opacity mixing, FX buttons, and BPM StepSequencer workflow: https://synapserack.com/en/docs/tutorial/monotone-2layer/

The official documents inspected do not establish that SynapseRack has the exact first-class Layer Transition Bus proposed for Syndocal. That is an absence of documentation, not proof that the product can never implement an equivalent workflow.

## Claim discipline

Until implementation and native acceptance land:

- Do not call current layer objects MediaAssets or clip slots.
- Do not call current Exclusive Take or A/B opacity a Transition Bus.
- Do not call output mapping Output FX.
- Do not call path persistence relink or missing-media recovery.
- Do not call an uncapped schema unlimited-layer performance.
- Do not claim current video capability exceeds SynapseRack. Media workflow, grouping, effect breadth, and native operator proof remain behind the official SynapseRack surface.
