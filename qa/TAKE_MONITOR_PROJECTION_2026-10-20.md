# Take monitor projection fix (2026-10-20)

## Symptom

After Take on a clip slot whose media asset differs from the layer's default slot, the
Program monitor went offline with
`InvalidLayerTransition("Video layer N media asset projection does not match its default
clip slot")`. The clip runtime kept playing. Taking the default clip again recovered it.
First recorded as "Native findings left unchanged" item 1 in
`qa/VJ_PERFORM_UI_2026-10-18.md`.

The native reproduction below shows it is wider than the monitor: the Display output's
health row also disappeared after the first non-default Take. Program output, the
monitors and recording all render through the same function.

## Root cause

- The engine publishes two shapes of the video snapshot. The authored/persisted shape
  projects the default slot into `layer.media_asset_id`/`source`. The rendered (live)
  shape deliberately projects the active slot, so the decoder follows the Take
  (`video_layer_summary` versus `authored_video_layer_summary`,
  `activate_runtime_video_clip_slot`, `complete_runtime_video_clip_take`).
- `video::apply_video_layer_transition_weights` is the single gate for every render
  entry point (`render_output*_with_effects_and_transitions*`, the artistic, display and
  cancellable recording variants). It called `protocol::validate_video_layer_transition_runtime`
  on the rendered snapshot. That validator chains into `validate_authored_video_effect_chains`
  and `validate_video_clip_slots`, which only accept the authored form (default-slot
  projection and an exact legacy ISF projection of the canonical layer chain).
- The engine itself validates against `authored_video_snapshot_from_rendered(...)`, which
  is why only the render side failed.
- A second, latent instance of the same mismatch was found in review: the rendered layer
  applies the active slot's effect overrides to its legacy ISF control values
  (`apply_active_video_clip_slot_effect_overrides`), so a layer with a canonical chain
  and an overriding active slot would also fail `validate_layer_effect_chain_projection`.

## Fix

`crates/protocol/src/lib.rs`

- New `validate_rendered_video_layer_transition_runtime` (rendered/live form). The
  existing `validate_video_layer_transition_runtime` and every authored validator keep
  their strict behaviour (`live_slot_projection = false`).
- `validate_video_clip_slots` in live mode requires `layer.media_asset_id` to be the asset
  of some slot in the layer's own bank (and the catalog check still requires
  `layer.source` to equal that asset's source). Distinct message for that case:
  "live media asset projection does not match any clip slot in its bank".
- `validate_layer_effect_chain_projection` in live mode ignores only ISF control
  `value`s; structure (labels, sources, stages, controls) must still match the chain.

`crates/video/src/lib.rs`: `apply_video_layer_transition_weights` now calls the rendered
validator. This covers Program output, Program/Preview monitors, recording (cancellable
variants), NDI/Spout and Display paths. The FX audition render path never reached the
transition validator.

## Tests added

- `protocol`: `rendered_transition_runtime_validation_accepts_a_non_default_live_slot_projection`
  (authored rejects, rendered accepts, asset outside the bank still rejected) and
  `rendered_transition_runtime_validation_accepts_active_slot_effect_override_values`
  (override values accepted, structural drift still rejected).
- `video`: `transition_render_paths_accept_a_non_default_live_clip_slot_projection` drives
  four public render entry points with a non-default live projection; it fails with the
  exact production error when the old validator is restored (checked).
- `engine`: `rendered_transition_runtime_validates_after_non_default_cut_and_crossfade_takes`
  takes slot 21 with Cut, then Crossfades 20 and 21 (start, middle, completion), checking
  the real rendered snapshot with the rendered validator and the authored snapshot with
  the strict one.

## Validation

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | 0 warnings |
| `cargo test -p protocol --lib` | 274 passed |
| `cargo test -p video --lib` | 222 passed, 6 ignored |
| `cargo test -p engine --lib` | 1151 passed; 2 timing flakes (`tap_snapshot_publication_ack_waits_for_shared_clock_and_stops_command_drain`, `max_loop_pre_wait_and_follow_dispatch_128000_occurrences_without_loss`) pass in isolation |
| `cargo test -p syndocal --bin syndocal` | 2140 passed; 1 flake (`project_backup_listing_missing_directory_pagination_exact_bytes_and_no_allocation`, allocation counter under parallel load) passes in isolation |
| `build-nobundle.cmd` | exit 0, only the existing Vite chunk-size advisory |
| `native-gate.ps1` | PASS: one responsive maximized `Syndocal` window plus the 13x13 helper, `CloseMainWindow` exit 0, no process left |
| Frontend checks | not run: no frontend or Tauri route change (no pinned manifest counts touched) |

## Native walkthrough (release build, copy of `vj-perform-final.sdc`)

Fixture: `target/qa/task18e/take-monitor.sdc`, a copy of the QA fixture with the Display
output enabled; the source fixture and user shows were not touched. Harness:
`target/qa/task18e/walk.mjs` (real Perform desk clicks for queue and Take, then
`get_live_video_monitor_frame` for Program and a staged Preview, plus
`get_live_video_output_health`). Evidence: `walk-prefix.json`, `walk-fixed2.json`,
`prefix-monitor-offline.png`, `fixed-monitors-healthy.png`, `idle-health.json`.

Sequence: Cut to layer 1 clips 4 and 6; Crossfade (1 s) layer 2 clip 5; Crossfade (0.5 s)
layer 3 clip 3; Cut layer 2 clip 9; Crossfade (1 s) layer 1 clip 2; Cut layer 3 clip 8;
Cut back to layer 1 default.

| | Before (executable built 04:15) | After |
| --- | --- | --- |
| Program monitor frame | 12 of 13 samples failed with the exact error (the 13th was before the first Take) | 13 of 13 OK, distinct pixel hash per Take |
| Preview (staged after each Take) | not exercised in this run (nothing staged) | 12 of 13 OK; one sample had been unstaged by design after a layer source change |
| Monitor/header text | `Monitor offline`, error banner with Retry | `PROGRAM LIVE`, no error text anywhere |
| Display output health | row vanished after the first non-default Take (output stopped reporting frames) | 58.4 to 60.0 fps through the sequence (peak load from the probes), 59.9 fps idle for 8 s, p95 about 18.6 ms, dropped frames 17 and flat afterwards |

Not covered natively: physical Display pixels (no capture of the output window), native
recording, and an effect override on a canonical chain (covered by the protocol test).
Preview sessions are cleared by design when their layer's source identity changes
(`VjPreviewTransport::reconcile_at`); this is unrelated to the bug and was left alone.

## Cleanup

Role restored to Standby (persisted and effective), localStorage restored byte-identical
(Recent list entry for the QA copy removed; verified again after a relaunch), only
processes whose path is `target\release\syndocal.exe` were stopped, none remain.

## Review

Independent read-only review (separate agent) confirmed the root cause and that every
other authored-validator caller receives authored data; it found the effect-override
instance above, which is fixed and tested.
