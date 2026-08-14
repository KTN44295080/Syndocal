# Syndocal Video: Media, Layer, Effects, and Transition Model

Status: product target accepted on 2026-08-12. This document defines the Video model behind Edit > Video and Control > Video. It is not a claim that the current implementation already satisfies the model.

The audited current-state gap is recorded separately in `qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md`.

## Product rule

Media, clips, layers, effects, transitions, compositions, and physical outputs are different objects. The UI may put related controls close together, but the persistence and runtime model must not collapse them into a single "video layer with a file path" object.

```text
Media Library
  MediaAsset (mp4, mov, still, camera, NDI, Spout, generator, composition)
      |
      +--> VideoClipSlot ------> VideoLayer ------> Composition ------> Output 1 / Output 2
             clip FX              layer FX
             loop/cues            transform/blend
             BPM/audio bindings   opacity/mask
                                      |
                                      +--> LayerTransitionBus <------ another VideoLayer
                                              transition FX
                                              beat quantize
                                              duration / curve / matte
```

## 1. Media Library

`MediaAsset` is imported once and reused from any layer.

- Stable asset id; original path is provenance, not identity.
- Content hash, size, codec, dimensions, duration, frame rate, audio presence, and availability state.
- Generated thumbnail and optional proxy/cache metadata.
- Optional audio analysis: BPM, beat grid, onsets, energy, color summary, and section markers.
- Sources include file video, still image, camera, screen capture, NDI, Spout, generator, and nested composition.
- Import routes are one coherent operation: file picker, drag-and-drop into the library, or dropping a file directly onto a layer. Direct layer drop performs `import -> create clip slot -> assign` as one guarded transaction.
- Desktop file-drop routing is target-aware: `.sdc` opens only on the project surface, audio clips route to Timeline Audio lanes, and supported visual media routes to the Media Library or a specific Video layer/slot. The current global first-match drop handler must not guess based only on extension.
- Multi-file import reports imported / skipped / failed counts, preserves every successful item, and never replaces the currently live slot. A direct multi-file layer drop appends slots in deterministic input order.
- Missing or hash-mismatched media remains visible and relinkable; it never silently becomes another file.

The media rail remains reachable when banks are empty, populated, paginated, or full. Import is not hidden by the presence of existing clips.

## 2. Layer and clip slots

`VideoLayer` is a persistent render lane. It is not the media file itself.

- Unlimited authored layers within the measured runtime envelope.
- Each layer owns an ordered bank of `VideoClipSlot` references to MediaAssets.
- A slot stores in/out points, loop mode, speed, cue points, launch quantization, and optional clip-local effect overrides.
- Each layer has `active_slot`, `queued_slot`, playback state, transform, mask, opacity, blend mode, and an ordered layer effect chain.
- The same MediaAsset may be assigned to multiple layers with different slot settings.
- Layers support duplicate, reorder, group, solo, hide, lock, rename, and routing to one or more compositions.
- Selecting a media tile does not unexpectedly replace a live layer. Edit assigns; Control stages/previews/takes according to the selected launch policy.

## 3. Effect scopes

Effects use one common ordered-chain editor but have explicit scopes.

1. Clip FX: follows a specific clip slot.
2. Layer FX: applies after the active clip is rendered and remains when clips change.
3. Transition FX: applies only while a transition is active.
4. Group/Composition FX: applies after multiple layers are composited.
5. Output FX: calibration/mapping-safe final processing for a physical output.

Every chain supports enable/bypass, reorder, reset, preset save/load, automation binding, MIDI/OSC/DMX mapping, BPM binding, and audio-reactive modulation. Common controls stay visible; advanced shader/node parameters remain behind contextual disclosure.

## 4. Two transition systems

Two transitions are deliberately distinct so operators do not have to build common show behavior from opacity automation.

### Clip Take Transition

Changes the active clip within one layer.

- Cut, crossfade, dip, wipe, luma/matte, displacement, blur, glitch, and custom ISF/node transition.
- Duration may be milliseconds, beats, or bars.
- Optional beat/bar quantization with deterministic late-trigger policy.
- Preview shows the outgoing and incoming slot before Take.

### Layer Transition Bus

Transfers visual prominence between layers or layer groups.

- Explicit `from` and `to` layer/group.
- Membership is opt-in and scoped to the bus. Other simultaneously composited layers remain untouched; a logo, mask, camera, or overlay must not disappear merely because two background layers transition.
- Independent transition curve, duration/beat length, matte source, and effect chain.
- Does not destroy either layer's playback position or layer FX.
- Can be manual, cue/timeline driven, MIDI/OSC/DMX triggered, or Auto Operator controlled.
- Failure or missing media never exposes an unintended layer; the last valid program frame/defined fallback wins.

This first-class bus is a usability advantage over workflows that require users to assemble every transition from raw opacity, grouping, and node primitives.

## 5. Edit > Video layout

Edit > Lighting in the current executable remains the geometry baseline.

| Current Lighting region | Edit > Video equivalent |
| --- | --- |
| Upper Scene Matrix | Media Library and clip-slot bank |
| Active Cue / Next Cue rail | Active Clip / Queued Clip and output truth |
| Lower-left editable Stage | Composition / Output Canvas with layer bounds and mapping guides |
| Lower-right Attributes/Faders | Layer stack and contextual Clip/Transform/Composite/Color/Effects/Mapping inspector |
| Expandable Timeline | Same shared Timeline, filtered to Video when opened |

The default inspector shows the selected layer and only its common controls. Clip FX, Layer FX, Transition, audio reactive, mapping, and node details are contextual tabs/disclosures, not permanent panels.

## 6. Control > Video

Control is performance, not authoring.

- Persistent but single Import Media route remains available for emergencies.
- Clip grid, layer selection, Preview, Program, one dominant Take, transition choice, duration/beat quantization, master blackout, and Output 1/2 truth.
- Active and queued clips are unambiguous per layer.
- Editing a deep effect graph routes back to Edit > Video without changing the live program frame.
- Auto VJ and audio reactive show Armed / Running / Hold / Fault truth; automation never silently takes control.

## 7. Compatibility and migration

The current `VideoLayerSummary` embeds one `VideoSourceSummary`. Migration must be additive and lossless. The final migrated model contains one default Clip Slot for every legacy layer, but that migration is staged so the asset tranche does not invent a temporary slot schema.

- In tranche 1, create one MediaAsset for each legacy layer source and bind the layer to it with an additive compatibility asset reference while retaining the embedded source projection.
- In tranche 2, create one default VideoClipSlot from that stable asset reference for each layer; do not create a second MediaAsset or change the asset id.
- Preserve layer id, label, blend, state, transform, color, built-in FX, and ISF stack.
- Do not rewrite the project on load. Serialize the new schema only after an explicit save.
- Media identity and hashes exclude machine-specific absolute-path spelling while preserving relink provenance.
- A failed migration leaves the old project unopened and reports the exact unsupported field; it never partially loads.

## 8. Acceptance boundary

Completion requires more than schema and static UI.

- Import via picker, library drop, and direct layer drop; cancel/failure/relink cases.
- At least 8 layers, 32 visible clip slots per bank, pagination/full-bank, and Output 1/2 at the target show resolution.
- Per-clip and per-layer effect ordering, bypass, persistence, clone, and error isolation.
- Clip Take and Layer Transition Bus: cut/crossfade/wipe/luma/custom effect, millisecond and beat-quantized timing, interrupted/reversed transitions, missing-media fallback.
- BPM changes and external ShowClock discontinuities remain deterministic.
- Audio-reactive modulation has bounded loss-to-safe-state behavior.
- MIDI/OSC/DMX mappings and Timeline cues target stable ids rather than UI positions.
- 1920x1080/1032 and 1366x768/1280x720 have zero outer scroll, clipping, overlap, or unreachable operations without shrinking established controls.
- RTX 5090 / 13900KF / 128 GB production acceptance includes HDMI Output 1 + Output 2, Serial DMX, audio input, and one-hour maximum-condition evidence.

## 9. Implementation tranches

The runtime may evolve internally, but each tranche must land as a coherent additive contract rather than a UI-only approximation.

1. **Asset catalog and migration**
   - Add stable `MediaAssetId` and authored `media_assets` to `VideoSnapshot` with `serde(default)`.
   - Add content hash/size/metadata/availability/relink state. Thumbnail and decoded-frame caches stay machine-local, not in `.sdc`.
   - Normalize every legacy `VideoLayerSummary.source` into one asset and an additive layer-to-asset compatibility reference while retaining the legacy source field for old-reader/render compatibility. This tranche does not create a placeholder Clip Slot.
2. **Clip slots per layer**
   - Add stable `VideoClipSlotId`, an ordered slot list, authored default slot, and runtime active/queued slot state.
   - Migrate each tranche-1 layer asset reference into exactly one default slot without duplicating or renumbering the MediaAsset.
   - Implement assign, remove, reorder, duplicate, queue, cancel queue, launch, seek, and direct-drop-as-import-and-assign as acknowledged engine transactions.
3. **Effect scopes**
   - Generalize the current ISF stack into an ordered effect-chain value shared by clip, layer, transition, composition/group, and output scopes.
   - Keep legacy built-in layer color/FX and `isf_effect` semantics during migration; do not silently change render order.
4. **Clip Take**
   - Replace source mutation during performance with an acknowledged active/queued slot transition on one layer.
   - Cut and crossfade first, then wipe/luma/custom chain; timing supports milliseconds and ShowClock beats/bars.
5. **Layer Transition Bus**
   - Add stable bus ids, explicit member layers/groups, authored defaults, and runtime from/to/progress state.
   - Multiple buses may coexist when their member sets do not conflict. Conflicting active transitions are rejected deterministically rather than last-writer-wins.
6. **Operator UI and mappings**
   - Edit > Video authors assets, slots, layers, chains, and transition buses in the current Lighting Edit geometry.
   - Control > Video stages/queues/takes without exposing deep authoring by default.
   - Timeline, MIDI, OSC, DMX, Auto VJ, and Audio Reactive target stable asset/slot/layer/bus/effect ids.
7. **Performance and native acceptance**
   - Predecode/prefetch, cache eviction, proxy policy, dual-HDMI presentation, failure isolation, and target-machine soak close the feature.
