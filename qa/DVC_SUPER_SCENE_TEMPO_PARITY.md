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
`ALLOWLOOP=0` remains separately reported for time-varying sources because Daslight holds
the final frame while Syndocal currently leaves the effect live for the block window.

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

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p engine`: 463 passed, 2 ignored
- `cargo test -p syndocal dvc_ -- --nocapture`: 49 passed
- `cargo test -p engine direct_tempo_driven_child_ -- --nocapture`
- `timeline_owned_tempo_child_reanchors_without_position_jump_after_bpm_change`
- `dvc_local_full_shinkan_super_scene_grid_and_dormant_conform_are_exact_when_present`
- `dvc_bpm_driven_super_scene_preserves_dynamic_conform_semantics`
- `dvc_bpm_driven_super_scene_with_invalid_division_falls_back_consistently`
- `dvc_synthetic_project_imports_patch_group_cues_and_super_scene`
