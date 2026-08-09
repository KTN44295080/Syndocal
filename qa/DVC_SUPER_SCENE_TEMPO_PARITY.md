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

For a BPM-driven parent plus `CONFORM_TO_TEMPO=1`, the importer intentionally keeps the
fixed-ms block and reports one explicit approximation: output is exact at the authored
grid tempo, but later global BPM changes do not yet retime the imported block content.
`ALLOWLOOP=0` is separately reported for time-varying sources because Daslight holds the
final frame while Syndocal currently leaves the effect live for the block window.

## Real-project proof

`C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc` contains two Super Scenes:

- `Shin`: `PLAY_TRIGGER=0`, `GRID_BPM=120`, 77 conform-marked Scene Blocks.
- `Unr`: `PLAY_TRIGGER=1`, `GRID_BPM=120`, 152 conform-marked Scene Blocks.

All 229 conform flags are dormant under the documented Daslight rule. The golden importer
test requires all 229 blocks to remain fixed-time, the imported clock to be 120 BPM, and
zero `CONFORM_TO_TEMPO` approximation details. The full project audit changed from 234 to
5 approximations; the remaining five are the already-disclosed eight-beam target omissions
on five dynamic FX and are unrelated to Timeline timing.

Synthetic coverage also fixes the opposite boundary: a `PLAY_TRIGGER=2` Super Scene with
`GRID_BPM=96` and audio `BPM=143` must seed 96 BPM and retain an explicit dynamic-tempo
approximation instead of silently claiming parity.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p syndocal dvc_ -- --nocapture`: 48 passed
- `dvc_local_full_shinkan_super_scene_grid_and_dormant_conform_are_exact_when_present`
- `dvc_super_scene_conform_is_only_a_boundary_when_parent_uses_bpm_driving`
- `dvc_synthetic_project_imports_patch_group_cues_and_super_scene`

