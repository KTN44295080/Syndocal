# First-show Preview settlement - 2026-10-17

- Competitive review queue item 2 ([review](COMPETITIVE_REVIEW_2026-10-16.md#one-prioritized-improvement-queue)):
  first automatic Preview after "Choose Media & Set Up" failed; P retry succeeded; the old error stayed.
- Branch `claude/beautiful-ptolemy-g2dlki`, base `59fbfa7b`. Linux cloud session: no Windows native gate,
  no native first-show rerun.

## Root cause

- The engine derives `timeline.duration_ms` from video layer source metadata (engine `timeline_duration_ms`).
- `media_asset_transaction_candidate_snapshot` projected the committed layers but not that derived duration.
  For any File layer with a known duration, the stored checkpoint hash therefore missed the published image.
- The first exact-authority action after the commit reconciles, finds the hash changed, advances the revision
  and clears Undo/Redo as if a live external edit had happened. First-run staging is that action, so
  `stage_vj_preview_layer` refused its own Bootstrap token ("Project changed before VJ Preview staging").
  The manual P path carries no token, which is why the retry succeeded.
- Ordinary authoritative video-layer imports share the projection, so they also lost their Undo entry at
  the next reconcile.
- The existing projection test used `metadata: None`, so it never exercised the derived duration.

## Changes

- Projection: mirror the engine duration with the existing `normalize_timeline_duration_for_engine`, and copy
  it to the active Timeline bank entry.
- Preview refusal names expected and current epoch/revision. Automatic first-run staging now reports the
  backend reason; it was previously discarded with `showError: false`.
- The first-run Preview failure footer is keyed. A confirmed staging of the requested layer replaces it with
  "Preview staged" and clears the first-run error; other footer messages are untouched.
- `stage_vj_preview_layer_for_authority` extracts the command body unchanged so tests drive the production path.

## Evidence

- New `media_asset_authoritative_bootstrap_terminal_authority_stages_first_preview`: real Bootstrap command,
  then staging with the returned token succeeds; a live BPM change afterwards is refused with the moved revision.
- New `media_asset_authoritative_video_layer_commit_keeps_its_authority_and_undo_after_reconcile`: after a
  real layer commit, reconcile keeps revision, hash and the Undo entry.
- Both fail without the projection fix (revision 1 -> 2; diff was only `timeline.duration_ms` and
  `timeline_bank[0].duration_ms`, stored 0, live 1000) and pass with it.
- `tsc --noEmit`, `check:vj-first-run`, `git diff --check` pass. `cargo fmt -p syndocal -- --check`: 0 diffs in
  `main.rs`; 126 pre-existing diffs elsewhere, identical with and without this change.
- `check:localization` fails on `59fbfa7b` with and without this change (4003/4009; six strings in
  `VideoPerformDesk.tsx`). Not changed here.

## Boundaries and next action

- Not observed natively. Next: on Windows, build, run the native gate, rerun the first-show path from an empty
  show and confirm the first clip reaches Preview without retry and the footer stays clean.
- Conditional drift remains by design: live MIDI clock, Link or Timeline Follow BPM and cue triggers rewrite
  persisted `clock.bpm` / `active_cue_id` and retire exact tokens. `MidiClockPulse` and cue triggers are
  classified as not touching persistence although those fields are persisted; not changed here.
- The fix covers every `MediaAssetTransaction` projection (Import, BootstrapVjShow, Update/Relink,
  UpdateGenerator); only Bootstrap and single-layer commit have new tests. Projections outside this
  function that change source metadata were not audited.
