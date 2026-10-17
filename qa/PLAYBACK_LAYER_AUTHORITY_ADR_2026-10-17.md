# Playback layer and project authority - ADR - 2026-10-17

- Decision: `DEC-PLAYBACK-LAYER-001`, registered **Open** in `qa/SYNDOCAL_Q1_Q4_LEDGER.json`.
  It is waiting for the product owner. No runtime code is changed by this document.
- Problem: Cue playback rewrites the persisted project image. Each Cue trigger and each Timeline
  Scene Block therefore advances the project revision, clears Undo/Redo and retires exact
  authority tokens.
- Follows [`qa/LIVE_TRAFFIC_AUTHORITY_DRIFT_2026-10-17.md`](https://github.com/KTN44295080/Syndocal/pull/3)
  (open PR KTN44295080/Syndocal#3), which fixed the live-tempo half and left this half open.
- Branch `dev/charming-johnson-drv08x`, base `88c5861` (`main`). Linux cloud session: no Windows
  native gate for this document.
- Evidence: a temporary engine probe (source and output in
  [`artifacts/playback-layer-probe-2026-10-17.txt`](artifacts/playback-layer-probe-2026-10-17.txt)),
  plus code reading. Every claim below is labelled measured, code or derived.

## 1. Product question: should reopening a show restore the last active Cue look?

**Recommendation: no, but only for opening a file.** These contexts would start with no Cue live
and show the authored state: Open, recent, template, backup restore and browser recovery. The
operator then fires the Cue or plays the Timeline. Continuing a running show is a different
context and is handled separately below:

| Context | Today | Recommended |
| --- | --- | --- |
| Open file, recent, template | Restores lighting levels and active Cue marks only (partial, see 1.1) | Authored state; no Cue live |
| Backup restore, browser recovery | Same as Open | Authored state; no Cue live |
| Undo / Redo | Cleared by any playback. When reachable, it installs the historic image, including that image's look | Replaces authored state and keeps the current playback layer |
| Two-PC Standby Take Over | B applies the Primary's full checkpoint every 2 s, so it mirrors the look behind blackout (not accepted on two PCs) | Carry the playback layer in the Standby generation, applied behind blackout and Re-arm (sub-decision Q3) |

No requirement or decision record asks for reopen-at-last-Cue, and none forbids it. The docs search
covered `qa/`, the README, the release/completion plans, HOTKEYS, frontend strings and the
historical handoff log. The only statement is the open question in the drift document. FC-27
(`qa/SYNDOCAL_NEAR_SHOW_READINESS_2026-08-25.md:431`) is an integrity rule: an active reference is
null or exact, or load is rejected. It does not require restoring the reference. The reasons for
the recommendation follow.

### 1.1 Today's restore is partial and depends on when the image was captured (measured)

- Reopen restores `fixtures[].attribute_values` and the active Cue ids. It does not restore the
  Cue's video layer state: layers load from `authored_state`. The fade is not restored either:
  `active_fade` is stripped on save and set to `None` on load. The Timeline loads stopped at 0.
- A save, backup or recovery capture taken mid-fade reopens frozen at the intermediate level.
  Probe: Cue 2's Pan reopened at `25003` of `50000`, with Cue 2 marked active and no fade running.
- Opening never energizes anything. Open retires output into Standby and keeps S0 until an
  explicit Arm and blackout release (`qa/PROJECT_REPLACEMENT_PREFLIGHT_2026-10-04.md:55-58`).
  So the only effect of the restore is that the first Arm of the next session shows whatever look
  was live when the file, backup or recovery capture was written.

### 1.2 Cue references alone cannot reproduce the look (measured)

- Cues track by default (`tracking: true`) and recall in Coexist mode. The live look depends on the
  trigger history, not only on the active Cue.
  - Probe: Cue 1 (Dimmer 40000), then Cue 2 (Pan only). Active Cue is 2, but Dimmer stays at 40000
    from Cue 1.
  - Re-applying Cue 2 over the authored base on load would lose that Dimmer.
- Other writers also leave residue that no Cue reference describes: move-in-black, Scene Block
  fade-out bakes, the End snap of a Scene Block, and Timeline automation lanes.
  - Probe: a Pan automation leaves Pan at 30000 after Stop.
- A faithful "yes" therefore means persisting the live look itself. That is exactly the playback
  data that moves the authority hash today.

### 1.3 Saving after playback overwrites authored data (measured)

Trigger a Cue, release it, then save. The saved image differs from the authored one:

- A fader-authored Dimmer of 1000 becomes 0. Release returns to fixture defaults
  (`cue_default_release_values`), not to the previous value.
- A legacy param-less effect target leaves `effects[].enabled` true.
- A video output target leaves `authored_video.outputs[].enabled/opacity/blackout` changed. This
  includes output *enabled*.
- Timeline automation leaves its last value.

With "no", the saved file holds only what was authored. A show's file no longer drifts each time it
is played and then saved.

### 1.4 Precedent in this product (code and docs)

Every other playback state already resets on open:

- Timeline transport ("a saved transport could resume at that position and alter fixture values",
  `qa/AUTHORED_CONTROL_PROJECT_PERSISTENCE_2026-10-03.md`);
- Cue live modifiers, strobe and FX activation instances;
- programmer and Blind, fades, pending Cues, and clip runtime.

The active Cue is the outlier. `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md:76-82` says a runtime
command "never reshapes persistence, and never enters Undo/history". Cue transport violates the
first half today.

### 1.5 What "no" gives up

- After a single-PC crash or a plain reopen, the operator re-fires the current Cue before Arm. A
  recovery capture can be up to 10 s old and a backup up to 60 s, so today's restore can also bring
  back the previous Cue. Timeline-driven shows need Play or a locate in either case.
- Two-PC continuity is kept only if Q3 carries the playback layer through Standby generations.
- An explicit, operator-invoked "Resume last Cue" after a crash could be a separate later decision.
  It is not proposed here.

**Alternative answer: yes.** Section 4 option (a2) describes what a faithful "yes" costs.

## 2. Measured write set (probe, engine persistence image)

| Playback | Persisted fields changed | Reaches `.sdc`? |
| --- | --- | --- |
| `TriggerCue` (fixture + legacy effect + video layer target) | `active_cue_id`, `cue_lists[].active_cue_id`, `fixtures[].attribute_values`, `effects[].enabled`, `video.layers[].state` | Yes, except `video.layers` (the writer uses `authored_video`; layers persist `authored_state`) |
| `TriggerCue` (video output target) | `authored_video.outputs[].enabled/opacity/blackout`, active ids | Yes; outputs have no authored/live split (`authored_video_snapshot_from_rendered`) |
| `TriggerCue` (group Cue) | `active_group_cue_ids`, active ids, levels | Yes |
| Node-graph Cue targets (code, `apply_node_graph_targets` L41002) | `node_graphs[].enabled` | Yes |
| Mid-fade sample | levels at the interpolated value; `active_fade` | Levels yes; `active_fade` stripped |
| `ReleaseCue` | active ids cleared; levels to defaults. Effect and video output flags are **not** restored (node graphs: code, same) | Yes |
| Timeline 500 ms (Scene Block + Pan automation), then Stop | Same as trigger, plus automation level; Stop restores nothing | Yes |
| `SetTimelinePlaying(true)` alone | telemetry only | No |
| `SetCueLiveModifier` | none | No |

Engine writers of `values` (code, `crates/engine/src/lib.rs`):

- Authored: `SetAttribute` (L23816), `SetFixtureAttributeBatch` (L23829), `SetGroupAttribute`
  (L23847), `ApplyAttributeValues` / presets (L24217), `ApplyPalette{programmer:false}` (L27254),
  programmer commit (L32424), patch and remove (L23574, L21889, L32436), and load (L22358).
- Playback:
  - Cue start snap (L40702) and fade tick (L41669);
  - move-in-black (L40877) and release plans (L37917, L37955);
  - Scene Block fade-out and End (L36640, L36690);
  - Timeline automations (L47270), child and Follow automations (L42841, L42906).
- Rollback after failed publication assigns `values` wholesale: L31908, L31981, L32024, L32270.

Video layers already have the split this ADR needs elsewhere. `RuntimeVideoLayer` keeps
`authored_state` (persisted) and `state` (written by Cues and clip transport) (L20044-20047,
L54825).

## 3. Why fail-closed is the correct result today (code)

- `reconcile_project_checkpoint_metadata` (`app/src-tauri/src/main.rs:52561-52588`) SHA-256 hashes
  the save image. On a difference it does `revision += 1`, clears Undo and Redo ("intentionally
  non-Undoable"), and the frontend marks the project dirty. Exact tokens are (epoch, revision,
  hash), so every held token stops matching.
- Undo and Redo are a full project replacement (`navigate_project_history_with_coordinator`
  L60345 -> `prepare_project_load` -> `replace_prepared_project_snapshot_with_coordinator_and_platform`
  L67016). It retires output into Standby until Arm (`complete_project_swap_disarmed`,
  L67514-67523; `qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md:30`). It stops media and the
  Timeline, ends a recording take, and reinstalls the historic image's levels and active Cue.
- Because Cue fades and fader edits share `values`, no hash projection can tell playback from an
  authored edit. Clearing Undo is the only result that never lets Undo rewind the live look.

What fail-closed costs today (code and derived):

- Every Cue, Scene Block or automation step clears Undo and retires tokens. VJ Preview staging
  fails with "Project changed before VJ Preview staging" while a Timeline with Scene Blocks plays.
  The frontend polls authority every 1 s.
- During a fade, each poll sees new levels, so the revision advances about once per second
  (derived from the fade writing `values` every tick).
- The project turns dirty after any playback. Close then prompts, browser recovery captures every
  10 s, and desktop backups write up to once per 60 s during the show. Recovery captures are
  superseded by moving revisions ("Project changed before recovery capture"). A Save during a fade
  can go stale ("Project changed while Save was preparing").

## 4. Options

### (c) Keep today's behaviour and document it

- No risk and no format change. The doc comment and contract wording were already corrected in PR
  KTN44295080/Syndocal#3.
- Leaves every cost in section 3, and the authored-data overwrite in 1.3, in place.
- Recommended only if the product accepts Undo and exact tokens being unusable during any
  rehearsal or show that fires Cues.

### (b) Undo/Redo keeps the current playback layer, as a standalone change

- **Not viable alone.** `values` records no owner for a level, so there is nothing to separate the
  playback layer from: history images carry the merged look.
- The hash also still moves on every trigger. Undo is cleared before (b) could act, so (b) alone
  gains nothing for tokens or Undo.
- (b) is therefore the Undo half of (a), not an alternative to it.

### (a) Authored base plus playback layer (recommended: variant a1)

The engine work is the same for both product answers. What differs is where the playback layer is
written.

Common engine model:

- `values` stays the live map. All playback writers, the render path and live behaviour stay
  bit-identical: release still goes to defaults, a fader over a live Cue still wins
  last-writer-takes-key, and capture still sees the live look.
- A parallel authored map holds the last *authored* value per key. Authored writers and load write
  both maps. Playback writers write only `values`, and mark the key playback-owned when they
  install the fade, snap, release plan, move-in-black or automation lane. This is never done per
  tick (AGENTS performance rule).
  - An authored write to a playback-owned key takes the key back (today's LTP result).
  - Rollbacks restore both maps.
- The same authored/live split is added for:
  - `effects[].enabled`, for legacy param-less targets;
  - `node_graphs[].enabled`;
  - video output `enabled/opacity/blackout`.
  Video layers already have it.
- One engine type, `PlaybackLayerSnapshot`, holds the playback layer:
  - playback-owned levels;
  - effect, node-graph and output overrides;
  - active Cue, list and group ids.
  It is the single implementation used for the Undo carry and, if chosen, for Standby and the
  `.sdc` playback section.
- `build_persistence_snapshot` reports the authored projection. The `mutates_persistence_snapshot`
  false set then becomes literally true. A permanent invariant test replaces the probe: every
  playback command and tick path leaves the persistence image byte-identical.
- App capture (Create Scene from Current, Update from Current, at main.rs L33966, L35131 and
  L35188) reads `persistence_snapshot()` today. It must switch to the live look, or the split would
  change what record captures.
- Undo and Redo (the (b) half): history navigation loads with "keep playback layer". The engine
  captures its own playback layer at load, so there is no cross-thread race, and re-applies it over
  the new authored state.
  - A live Cue that the target image removes is released, exactly as Remove Cue does today.
  - Overrides for removed fixtures, effects or outputs are dropped.
  - FC-27 validation still applies.
  - Output retirement, the media and Timeline stop, and the end of a recording take stay
    **unchanged** (Q4).

**(a1) Playback layer never in files (matches "no").**

- `.sdc`, backup, recovery and the hash carry only authored state. Active Cue ids are runtime-only.
- Old version-1 files with active ids or a baked look go through a one-way, observable migration:
  - `attribute_values` load as the authored base, so the file still opens with the look it was
    saved with;
  - active references are still FC-27-validated, so a malformed file is still rejected;
  - valid references are then dropped, with a load notice ("Saved live Cue X was not restored;
    shows open with no Cue live");
  - the next save omits them.
- The shape is unchanged, so `PROJECT_FILE_VERSION` stays 1. Old builds open new files like a show
  saved with no Cue live, and new builds open old files through the migration above. Proposed
  without a version bump, because a bump would make new files unreadable by older builds for no
  safety gain.

**(a2) Playback layer as a separate `.sdc` section (matches "yes").**

- Needs a new section outside the hash, which means `PROJECT_FILE_VERSION` 2: an old build would
  otherwise read the base and silently ignore the playback data. It also needs a v1-to-v2
  migration and rejection of future versions.
- Because playback no longer dirties the project, the section holds the look as of the last
  authored save or backup. That is an arbitrary moment, not "the last Cue". Making playback dirty
  the project again (separately from the authority hash) brings back the close prompt and the
  backup and recovery churn.
- Same engine work as (a1), plus the format change. Not recommended.

## 5. Questions for the product owner

1. **Reopen:** opening a file (Open, recent, template, backup, recovery) starts with no Cue live,
   authored state only. *Recommended: yes (a1).* Alternative: restore the last look (a2).
2. **Architecture:** (a) authored base plus playback layer, with Undo/Redo keeping the playback
   layer. *Recommended.* Alternative: (c) keep fail-closed and document it.
3. **Two-PC Take Over:** carry the playback layer in Standby generations so B resumes the
   Primary's live Cue state behind blackout and Re-arm. *Recommended: yes*, as its own phase with
   its own Standby-format compatibility evidence. Alternative: Take Over starts with no Cue live
   and the operator re-fires.
4. **Undo/Redo output:** Undo stays a full replacement that retires output, as today. *Recommended:
   unchanged here.* An in-place Undo that keeps output armed changes the output-ownership contract
   and its checkers, and would be a separate decision.

A related open item that this ADR does not decide: release returns to fixture defaults, so after a
release the live level can differ from the saved authored base until reopen or Undo. Whether release
should instead reveal the authored base would be a separate product decision. It is not needed for
(a), which keeps live behaviour identical.

## 6. Implementation plan (only after the decision)

1. **Engine split.**
   - Authored maps and flags, playback ownership, `PlaybackLayerSnapshot`.
   - Persistence reports the authored projection.
   - Rollbacks restore both maps.
   - The invariant test covers trigger, toggle, next/previous, release, fade tick, move-in-black,
     Scene Block trigger, fade-out and End, automation, child, Follow and DJ Link automation, and
     executor transport.
   - Live-output equivalence test: identical DMX for a scripted sequence before and after the
     split.
2. **Undo/Redo carry.**
   - Engine load mode and app history navigation.
   - Tests: authored edit -> Cue -> Undo keeps the look and reverts only the edit; Undo that
     removes a live Cue releases it; Redo symmetric; FC-27 still rejects.
3. **Compatibility.**
   - Migration and load notice (a1).
   - New golden: a version-1 file saved with an active group Cue mid-fade, a flipped legacy
     effect, a video output target and automation residue, plus its expected canonical image.
   - Round-trip stability; malformed references and future versions rejected.
4. **App and tests flip.**
   - Capture reads the live look.
   - `project_snapshot_for_save_drops_volatile_runtime_state` changes to "active Cue dropped".
   - The drift branch's `cue_trigger_rewrites_the_saved_look_and_fails_closed` becomes "Cue and
     Timeline playback keep tokens and Undo".
   - The classification test at engine L69170 stays, and is now true.
5. **Standby carry** (if Q3 is yes): generation sidecar and Take Over apply, with Standby
   compatibility evidence.
6. **Windows native gate** (`qa/WINDOWS_NATIVE_BUILD.md`, `pnpm --dir app tauri build --no-bundle`).
   - One responsive `Syndocal` window.
   - Fire a Cue and play a Timeline with Scene Blocks, then stage VJ Preview and Undo an earlier
     edit: the look is kept and the token accepted.
   - Reopen the saved file: no Cue live.

Validation per `AGENTS.md`: runtime and persistence row (focused regressions plus the Windows native
gate), compatibility proof for the migration (success plus invalid and retired-path rejection), and
warnings baseline/current/delta for the configurations run.

Remaining drift outside this decision: Follow settlement and DJ Link start install the active
Timeline (`timeline.id`, plus the project tempo after PR KTN44295080/Syndocal#3) and still retire
tokens by design. A legacy Timeline video clip on a layer without a default slot may leak
`layer.source` into the saved image (code reading, L47451, not measured).

## 7. Validation of this document

- Documentation only: no product build, UI launch, version bump or hardware QA.
- The probe was run on Linux with `cargo test -p engine --lib zz_probe -- --nocapture --test-threads=1`
  (3 passed) and then removed. `crates/engine/src/lib.rs` is unchanged in this commit.
- Ledger: `node app/scripts/check-q1-q4-ledger.mjs` and
  `node app/scripts/check-completion-ledger.mjs`; `git diff --check`.
