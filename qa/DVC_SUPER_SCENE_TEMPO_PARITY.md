# DVC Super Scene tempo parity

## Source contract

Daslight 5 manual v1.4 defines a Scene Block's `Conform to tempo` toggle as active only
when the owning Super Scene uses BPM driving mode. With any other parent driving mode,
the Scene Block runs at its source speed. The adjacent `Loop` toggle controls whether
extended content repeats or holds its final source value.

Official source: [Daslight 5 manual v1.4](https://eu-litterature.n-g.co/Release/daslight_5_manual_en.pdf),
section 4.3, pages 47-49.

The DVC representation used by the available projects is:

- `SCENE@PLAY_TRIGGER=2`: BPM driving mode.
- `RACK@GRID_BPM`: the Super Scene authoring grid tempo.
- audio `BLOCK@BPM`: detected/source audio metadata, not the Super Scene grid or project clock.
- Scene `BLOCK@CONFORM_TO_TEMPO`: content follows tempo only under a BPM-driven parent.
- Scene `BLOCK@ALLOWLOOP`: extended content loops when enabled.
- Scene `BLOCK@SPEED`: authored content playback rate.

## Current import rule

Syndocal keeps DVC Scene Blocks in the existing fixed-ms child-timeline path whenever
the parent is not BPM-driven. This is exact for block placement, duration, fade,
signed source position, and `SPEED`; the dormant `CONFORM_TO_TEMPO` bit is therefore
not reported as an approximation.

The imported project clock is seeded from `RACK@GRID_BPM`. The former behavior used the
first audio block's detected `BPM`, which incorrectly changed `Shinkan2026.dvc` from its
120 BPM Super Scene grid to the audio analysis value 157.861 BPM.

For `PLAY_TRIGGER=2`, Syndocal stores `PLAY_DIVISION` as the owning Scene's authored beat
duration and marks its child Timeline as tempo-driven. The complete authored millisecond
grid then advances at:

`parent rate = authored child duration ms / (PLAY_DIVISION * 60000 / live BPM)`

Block placement therefore follows later global BPM changes without rewriting the imported
millisecond coordinates. A live BPM rebuild re-anchors both direct and root-Timeline-owned
child transports at the current authored position, so the playhead does not jump.

Within that moving parent grid, `CONFORM_TO_TEMPO=1` multiplies the Scene Block source rate
by the parent rate. With the flag off, the engine cancels the parent rate for the source
clock, leaving the block at its authored `SPEED` while its placement still follows the
parent. Rebuilds preserve the wall-clock phase of a non-conforming source and preserve the
authored phase of a conforming source while changing its future rate.

Invalid or unsupported `PLAY_DIVISION` values fail closed to the fixed-time child path,
disable the otherwise unusable conform bit, and emit one explicit approximation.

`ALLOWLOOP` is preserved on the child Scene Block's existing `loop_fill` field. Loop On
keeps the source oscillator or Cue Step sequence wrapping for the block window. Loop Off
clamps every owned FX evaluation clock to the last millisecond of that FX's authored free-run
period; Cue Steps clamp at sequence end. The block remains active for its full placement and
fade window, so the held value participates in normal DMX/video blending until block release.
The resolved Loop On/Off state is copied into each runtime activation when the block starts;
the 44 Hz fixture/video evaluation path does not search child transports or authored events.

## Recursive nested Timeline rule

A DVC Scene Block may reference another Super Scene. Syndocal imports that reference as the
same typed child-Timeline event instead of dropping it at depth 1. At project validation and
runtime rebuild, the complete Cue graph is walked recursively: acyclic descendants are accepted,
while self references, cycles, and missing Cue references still fail closed.

Every descendant reuses the existing child-transport lifecycle. Parent source position,
signed source offset, `SPEED`, BPM parent rate, Conform behavior, Loop On/Off, fade boundaries,
FX, Cue Steps, lighting/video automation, pause/resume, seek, and release are therefore composed
through one transport tree rather than a parallel playback path. Direct Scene Matrix playback
propagates the root Cue generation to every descendant; root release recursively tears down the
subtree and restores values only when no other direct tree still owns the Cue.

Child audio no longer reconstructs position from the authored summary. The engine publishes the
exact position of every active child transport plus a collision-free identity consisting of its
root activation and complete nested Scene Block path. The native audio worker includes that path
in its sink key, so equal audio clip ids in sibling or repeated nested branches cannot stop or
steal one another's playback. This runtime-only identity is excluded from `.sdc` and UI JSON.

## Static implementation evidence

Daslight 5.0.6.2 (`Daslight 5.exe`) was inspected without UI automation. The
`PLAY_TRIGGER=2` dispatcher reads `PLAY_DIVISION` and forms a target duration from
`60000 / global BPM`. Its Super Scene rate function divides authored total duration by
that target duration. Runtime block dispatch reads `CONFORM_TO_TEMPO`; conforming content
inherits the parent effective rate, while non-conforming content divides its source delta
by that rate. `ALLOWLOOP` selects modulo source time; Loop Off uses the non-modulo hold path.

## Real-project proof

`C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc` contains two Super Scenes:

- `Shin`: `PLAY_TRIGGER=0`, `GRID_BPM=120`, 77 conform-marked Scene Blocks.
- `Unr`: `PLAY_TRIGGER=1`, `GRID_BPM=120`, 152 conform-marked Scene Blocks.

All 229 conform flags are dormant under the documented Daslight rule. The golden importer
test requires all 229 blocks to remain fixed-time, the imported clock to be 120 BPM, and
zero `CONFORM_TO_TEMPO` approximation details. The full project audit changed from 234 to
5 approximations; the remaining five are the already-disclosed eight-beam target omissions
on five dynamic FX and are unrelated to Timeline timing.

Synthetic coverage fixes the opposite boundary: a `PLAY_TRIGGER=2` Super Scene with
`GRID_BPM=96` and audio `BPM=143` must seed 96 BPM, store `PLAY_DIVISION`, enable the
tempo-driven parent, and retain the block's active conform semantics without an
approximation. A second invalid-division fixture proves the fixed-time fallback.

The available Shinkan golden contains no nested Super Scene reference, so it cannot prove this
structure from real content. A synthetic DVC therefore adds a Super Scene that references another
Super Scene and requires import, project validation, direct trigger, recursive transport dispatch,
and leaf DMX rendering to succeed without a nested-reference skip. Engine tests independently
cover root/direct nested source offsets, direct-generation propagation, recursive release,
pause/resume FX continuity, Step Loop On/Off, exact child-audio position/path, and audio sink-key
separation.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p engine`: 470 passed, 2 ignored
- `cargo test -p protocol`: 44 passed
- `cargo test -p syndocal`: 377 passed, 9 ignored
- `cargo test -p syndocal dvc_ -- --nocapture`: 51 passed
- `cargo test -p engine direct_tempo_driven_child_ -- --nocapture`
- `timeline_owned_tempo_child_reanchors_without_position_jump_after_bpm_change`
- `direct_child_loop_off_holds_final_fx_frame_while_loop_on_wraps`
- `direct_child_loop_off_holds_final_step_while_loop_on_wraps`
- `direct_nested_child_transport_reaches_leaf_and_parent_release_tears_down_subtree`
- `timeline_nested_child_transport_composes_parent_source_offset_to_leaf`
- `direct_nested_owned_fx_uses_root_generation_and_freezes_across_pause`
- `direct_nested_step_loop_off_holds_and_loop_on_wraps`
- `nested_child_audio_uses_exact_runtime_path_position_and_direct_generation`
- `child_timeline_audio_sink_keys_isolate_parent_activation_and_clip_id`
- `cargo test -p engine --release child_timeline_budget_16_by_200_uses_preallocated_tick_buffers -- --nocapture`:
  16 timeline transports × 200 child events, 0 transport-tick reallocations
- `dvc_local_full_shinkan_super_scene_grid_and_dormant_conform_are_exact_when_present`
- `dvc_bpm_driven_super_scene_preserves_dynamic_conform_semantics`
- `dvc_bpm_driven_super_scene_with_invalid_division_falls_back_consistently`
- `dvc_synthetic_project_imports_patch_group_cues_and_super_scene`
- `dvc_nested_super_scene_reference_imports_and_reaches_leaf_dmx`
