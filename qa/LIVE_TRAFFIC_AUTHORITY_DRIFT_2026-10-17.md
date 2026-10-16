# Live traffic and project authority drift - 2026-10-17

- Scope: live engine traffic rewrote the persisted project image outside a project transaction. The
  next reconcile (`reconcile_project_checkpoint_metadata`) advanced the revision, cleared Undo/Redo and
  retired every exact epoch/revision/hash token, for example `stage_vj_preview_layer`'s first-run
  authority ("Project changed before VJ Preview staging").
- Follows the boundary recorded by [KTN44295080/Syndocal#2](https://github.com/KTN44295080/Syndocal/pull/2)
  (`qa/FIRST_SHOW_PREVIEW_2026-10-17.md` on that branch), which fixed a different drift.
- Branch `dev/pensive-galileo-sqij2h`, base `59fbfa7b`. Linux cloud session: no Windows native gate.

## What live traffic writes into the saved image

Measured with a temporary engine probe (`build_persistence_snapshot` JSON diff, then the app save
projection, which keeps only `clock.bpm` from the clock and strips previews and telemetry):

| Traffic | `mutates_persistence_snapshot` | Saved fields changed |
| --- | --- | --- |
| 30 MIDI Clock pulses | false | `clock.bpm` 120 -> 124.87 (an estimate that moves on every pulse) |
| `SyncExternalClock` (Link from the frontend, OSC, Web Remote) | true | `clock.bpm` |
| Timeline Follow ramp (44 Hz tick, no command) | - | `clock.bpm` on every tick of the transition |
| `TriggerCue` (fixture level + legacy effect target) | false | `active_cue_id`, `cue_lists[].active_cue_id`, `active_group_cue_ids`, `fixtures[].attribute_values`, `effects[].enabled` |
| `ReleaseCue` | false | the three active-Cue fields |

The classification doc said the false set was limited to state the persistence projection removes.
That was untrue for MIDI Clock (tempo) and for every Cue transport command (recalled look).

## Decision

### Live tempo is not project data

- `project_snapshot_for_save` already said project load restores only the authored BPM, and load
  builds `BpmClock::new(snapshot.clock.bpm)`.
- Manual BPM is the renderer-ticketed, Undoable path; Tap and external sync stayed runtime
  (`qa/SNAPSHOT_CLEANUP_PENDING_2026-09-05.md`, manual `set_bpm` entry).
- The MIDI estimate changes on every pulse (24 PPQN) and the Follow ramp changes it from the tick with
  no command, so no command classification can keep it out of the hash.

Change: `BpmClock` keeps a separate `project_bpm`. Project load, `SetBpm` and Tap write it (both
re-conform authored Timeline blocks at that tempo). A clock-owning Timeline Follow writes it once at
settlement, together with the destination Timeline that is persisted at the same moment.
MIDI Clock, `SyncExternalClock` and the Follow ramp change only the live tempo.
`build_persistence_snapshot` now reports `ClockSnapshot { bpm: project_bpm, ..default }`.

### The recalled Cue look stays in the hash and fails closed

- It is saved on purpose. Load restores and validates the active Cue references (engine
  `load_project_snapshot_checked`, FC-27). `project_snapshot_for_save_drops_volatile_runtime_state` asserts
  that `active_cue_id` is kept.
- A trigger also writes fixture levels. Cue fades write the same `values` map that fader edits write
  and Undo tracks (`apply_active_fade`, `fixture_attribute_values`). A legacy, param-less effect target
  flips the authored `effects[].enabled` (`apply_cue_effect_targets`). Excluding these from the hash
  would let real authored edits through unchecked.
- Undo/Redo replaces the whole image (`navigate_project_history` -> `prepare_project_load`).
  Excluding only the active-Cue fields would let an Undo silently rewind the active Cue and its effect
  activations. The current result (new revision, Undo cleared, token refused) is the only one that
  cannot revert the live look by accident.
- Reclassifying Cue transport as persisting would not help. `send` would make every live trigger wait
  on a held persistence submission gate (`persistence_submission_gate_holds_follower_through_route_rollback`
  shows a persisting command waiting through a route transaction). External Cue traffic would starve
  read-only authority polls, which is why the classification exists. Tick-driven Timeline playback
  writes the same fields with no command at all.

Change: classification unchanged. The doc comment now states the real contract. The false set never
touches coordinator-owned state. It changes only engine state that is either removed from the saved
image or is the deliberately saved playback look, and checkpoint-hash reconciliation detects that look
and fails closed.

## Changes

- `crates/engine/src/lib.rs`: `BpmClock.project_bpm` and `settle_project_bpm_preserving_beat_position`;
  the Follow settlement uses the latter; `build_persistence_snapshot` normalizes the clock to the
  project tempo; `mutates_persistence_snapshot` doc rewritten.
- `app/src-tauri/src/project_snapshot_persistence.rs`: comment only; the writer keeps its local clock
  normalization.
- Tests: engine `external_clock_traffic_never_rewrites_the_persisted_project_tempo` (new);
  `timeline_follow_captures_next_id_runs_dual_video_slews_bpm_and_completes_once` now asserts the
  persisted tempo stays at the project tempo mid-ramp and becomes the destination tempo at settlement;
  app `live_traffic_authority_tests.rs` (new, three tests on the real headless harness: authored commit
  -> live traffic -> production reconcile + VJ Preview exact-authority check + exact-token Undo).

## Evidence

- Before the fix (engine unchanged, new app tests only):
  - `live_midi_clock_and_link_keep_the_committed_token_and_undo` failed: saved BPM `128.0` (Link)
    where `120.0` was expected.
  - `manual_tempo_outside_a_transaction_still_retires_the_token` failed right after MIDI Clock alone:
    `"Project changed before VJ Preview staging; nothing was staged"`.
  - `cue_trigger_rewrites_the_saved_look_and_fails_closed` passed. That behaviour is intended.
- After: all three pass. `bpm_history_tests` (2) and `project_snapshot_for_save_drops_volatile_runtime_state`
  pass.
- Engine lib suite, run in parallel while the app built: 1135 passed, 1 failed, 14 ignored. The failure
  was `max_loop_pre_wait_and_follow_dispatch_128000_occurrences_without_loss` at 84 ms against its
  75 ms debug budget; alone it passes at 67 ms.
- App subset (history, authority, checkpoint, persistence, clock, DJ Link, Follow, Cue, tempo; parallel):
  396 passed, 1 failed. The failure was `timeline_cue_source_admission_precedes_callback_progress`,
  which uses a process-global activation hook (CI runs single-threaded); alone it passes.
- Full app suite single-threaded as CI runs it (`cargo test -p syndocal --bin syndocal -- --test-threads=1`):
  1731 passed, 0 failed, 13 ignored.
- Warnings (Linux debug, `cargo test -p engine --lib`, `cargo test -p syndocal --bin syndocal`):
  baseline 1, current 1, delta 0. The one warning is the existing unused `safety_write_gate` in
  `crates/io/src/serial_dmx.rs`.
- `rustfmt --check`: no diffs in any touched Rust file. `git diff --check` clean.

## Compatibility

- File format unchanged (`snapshot.clock.bpm`); every existing file loads exactly as before.
- Old: a save while following MIDI Clock or Link, or during a Follow ramp, stored the momentary live
  tempo. New: it stores the project tempo (last load, `SetBpm`, Tap or settled clock-owning Follow).
  Reason: the live value is not authored, jitters per pulse, and moved the authority identity with no
  edit. Reload after a settled Follow still restores the destination tempo with the destination Timeline.
- Rejection kept: `SetBpm` outside a transaction still retires tokens and Undo (new test; PR #2's
  `SetBpm` refusal test uses the same path). Cue recall still fails closed (new test).
- Persisted-image readers now get the project tempo. Timeline item edits (nudge/ripple/quantize/
  duplicate/paste) shift `time_beats` with `before.clock.bpm`, which is now the tempo the blocks were
  conformed at rather than an external estimate. DJ Link bar math reads the published live snapshot and
  is unchanged.

## Boundaries and next action

- Not observed natively. Next: Windows native gate; with MIDI Clock running, run the first-show path
  from an empty show and confirm the Bootstrap token stages Preview and Undo survives.
- A Cue trigger, Timeline playback that fires Scene Blocks, and Follow settlement still retire exact
  tokens and Undo by design. Keeping authoring tokens alive across playback needs the recalled look
  separated from authored base values (engine model) and a product decision on reopening at the last
  Cue. Not done here.
- `SyncExternalClock` stays classified as persisting (conservative). It now changes only live clock
  state and could move to the false set; not changed here.
