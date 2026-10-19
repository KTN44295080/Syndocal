# Video layer groups: group blend, opacity, solo/bypass and transform (Task 19)

Updated: 2026-10-20. Base: `6c13132a` (branch `task19-layer-groups`). Status: committed on the
branch for review; not merged, not natively validated (see "Not done").

Competitive review item 7 ("group blend/transform", `qa/COMPETITIVE_REVIEW_2026-10-16.md`):
Resolume and VDMX have layer groups that blend, fade and transform as one image. Syndocal
groups only scoped FX before this task.

## Model

`VideoLayerGroupSummary` (protocol) gains, all optional on disk:

| Field | Default | Meaning |
| --- | --- | --- |
| `enabled` | `true` | Disabled group removes its layers from the composition. |
| `opacity` | `1.0` | Applied once to the group's composited image. Finite, 0..1. |
| `blend_mode` | `Normal` | Blend of the group image against everything below. |
| `solo` | `false` | Like layer solo: when any live group is solo, only solo groups render. |
| `bypassed` | `false` | Group controls and group FX are skipped; children render as plain layers. |
| `transform` | none | Position, scale, rotation (and crop) of the group image. Finite, scale > 0, crops valid. |

The group's FX chain is the existing scoped `VideoEffectScope::Group` chain (up to 32
stages); it is applied to the group image before transform and opacity.

Schema: `PROJECT_FILE_VERSION` 1 -> 2. A v1 project loads with every group a pass-through
(serde defaults) and with no groups when v1 had none; the writer always emits v2; v0, v3
and non-integer versions are rejected before migration (`project_file_decode.rs`). Project
backups that embed a v1 project are accepted and normalised in memory (the stored backup file
is never rewritten). Checked-in goldens and `samples/phase1-mini-show.sdc` moved to v2; the v1
input is still exercised by programmatic v1 corpus tests.

## Rendering

Order inside one composition: children composite over transparent black (group image) ->
group FX chain -> group transform -> group opacity and blend against the backdrop. Policy
lives in `crates/video/src/layer_group_composition.rs` and is shared by CPU and GPU.

* Pass-through (default controls, no chain, no transform) allocates no intermediate: layers
  keep blending individually against the backdrop (the pre-existing behaviour).
* A group of one Normal-blend layer with no chain and no transform folds into that layer
  (`opacity *= group.opacity`, group blend), so it costs the same decode, upload and pixel
  work as a plain layer. Test: frame `Arc` identity and decode count.
* Non-trivial groups composite on the CPU into one RGBA image. The GPU path uploads that
  image as one plain layer and the presenter's existing blend and output-mapping passes do
  the rest, so the GPU/CPU contract is unchanged: `<= layer count` levels per channel (two
  outer GPU blends for a group fixture).
* Program (`render_output`), Preview and the audition snapshot all go through the same plan
  and the same `compose_scoped_plan_with_cancellation`; output pacing and monitors were not
  touched.

## Commands, undo, deletion

Group create, rename, delete, membership moves, reorder and the new controls all go through
the existing ticketed `ApplyVideoEffectCatalog` command (authoritative receipt, one history
entry, rollback of the complete A image on failure). The request gains optional
`composition_layer_orders` so that membership/reorder edits (groups stay contiguous) and the
catalog change publish and undo atomically. Deletion goes through `confirmDialog`
(`removeLayerGroupDialog`) and removes the group's chains and any transition bus that
requires it; its layers stay in the composition.

## UI

* Control > Video (Perform matrix): a 44px group strip per group above its layers, with
  collapse, name (selects the group), opacity, blend, Solo and Bypass; layers indent beneath.
* Perform Inspector FX tab: selecting a group shows `VideoGroupInspector` (enable, opacity,
  blend, transform, Group FX). Selecting a layer restores the layer Inspector.
* Edit > Video: `VideoLayerGroupsPanel` creates (contiguous range), renames, reorders,
  deletes, moves layers in and out, and opens the group inspector.
* Theme tokens and type roles only; Japanese and English strings in `uiLocalization.ts`.

### Layout trade-off at 1366x768

The pane heights are unchanged. A group strip is a matrix row: with a group above the first
layers, the matrix shows the strip plus two complete layer rows, and the rest scrolls inside
the matrix (no outer scroll). Growing the matrix by 45px was tried and rejected: it took the
monitors below the 296px contract and left one FX card in the Inspector. The check now counts
group strips as matrix rows (`fullRows + groupHeaders >= 3`).

## MIDI

Group opacity, Solo and Bypass controls carry `controlMappingTargetData(null)` plus
`data-control-map-group-id`, `data-control-map-group-property` and
`data-control-map-unsupported="video-group"`. The control-mapping action model has no group
action and the native `ControlAction` enum would have to grow in lock-step; another task is
extending that model, so group controls are tagged but NOT learnable yet. The Inspector says
"Group MIDI Learn is not available yet." (not silently mapping a child layer).

## Validation

See `task19-layer-groups-result.md` (sol-handoff) for the final run; summary in the commit
messages. Rust: `cargo check --workspace --all-targets` zero warnings; `video` 229 tests
(including `layer_group_*` and the GPU readback parity test over all ten blends, with and
without transform, three opacities); `protocol` and `engine` group tests; `syndocal` migration
corpus, backup and catalog tests. Frontend: `tsc`, `pnpm build`, `check:localization`,
`check:video-layer-groups`, `check:perform-layout`, `check:workspace-ia`,
`check-theme-browser.mjs`.

## Not done

* Native app run (single-instance rule); Claude validates after merge.
* MIDI Learn for group controls (see above).
* Group transform has no on-canvas handles; it is numeric (position, scale, rotation).

## Semantics worth knowing

* A group is "pass-through" only while every control is at its default. As soon as opacity
  is below 1, blend is not Normal, a transform is set or the group has FX, the group becomes
  an isolated image: a Multiply/Overlay child then blends against the group's own transparent
  buffer, not the layers below. This is the usual isolated-group behaviour, so a group whose
  opacity drops from 1.0 to 0.99 can look different; leave a group at defaults to keep child
  blends against the real backdrop.
* A solo group hides ungrouped layers and non-solo groups; a bypassed group is never solo.
  A disabled group removes its layers even when bypassed.
* Group opacity 0 skips the group's layers (and their frame preparation) entirely.
* A group with FX is not GPU-eligible (unchanged rule); without FX its CPU group image is
  uploaded as one layer.

## Independent review

A separate read-only reviewer read the native diff (protocol, video, engine, Tauri). Findings
and outcome:

| Finding | Outcome |
| --- | --- |
| `.sdctemplate` files saved at schema 1 would fail `validate_project_file` | Fixed: `normalize_user_template_file` upgrades a v1 project in memory; test added; v0/v3 still rejected. |
| Group transform scale only checked `> 0` | Fixed: validated to 0.001..100 (the layer bound) and clamped in `group_transform`; engine test covers 0, 0.0001, 100.5, NaN. |
| `prepare_group_layer` indexed `layers[0]` | Fixed: returns a typed error if no member is in the plan. |
| Isolated vs pass-through look at opacity 1.0/0.99 | Documented above (intended). |
| GPU attempt can build a group image then fall back to CPU | Accepted: only when a layer is over the GPU size cap or plan-ineligible; groups with stateful FX are already ineligible. |
| Preview/`render_output_plan` now apply composition-scope chains | Intended: same scoped compose as Program; covered by `layer_group_solo_enabled_and_bypass_filter_program_and_preview_identically`. |
| `present_prepared_output` ignores groups | Dead outside tests; left. |
| Layer in a transition bus and a group (existing behaviour) | Unchanged. |
