# Playback layer and project authority - ADR - 2026-10-17

- Decision: `DEC-PLAYBACK-LAYER-001`, **Accepted 2026-10-17** by the product owner, who chose the
  recommended answer to all four questions in section 5. Recorded in `qa/SYNDOCAL_Q1_Q4_LEDGER.json`.
  The proposal commit changed no runtime code. Section 8 records the implementation as built.
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

Owner answers (AskUserQuestion, 2026-10-17). The recommended option was chosen for each question:

1. Opening a file starts with no Cue live (a1).
2. Architecture (a): authored base plus playback layer, and Undo/Redo keeps the playback layer.
3. Standby Take Over carries the playback layer.
4. Undo/Redo output behaviour stays as today.

The questions as asked:

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

## 8. Implementation as built (2026-10-17)

Branch `dev/charming-johnson-drv08x`, base `88c5861`. Linux cloud session: the Windows native gate
was **not run** (section 8.5).

### 8.1 Engine (`crates/engine`, `crates/protocol`)

- **Fixture levels.** `values` is now `FixtureValueLayers` (`playback_layer.rs`).
  - Reads see the live map through `Deref`, so render, snapshot and Cue code are unchanged.
  - The type has no mutable access, so the compiler listed every writer, 23 in all. Each now
    calls `set_authored` or `set_playback`.
  - Authored writers:
    - `SetAttribute`, `SetFixtureAttributeBatch`, `SetGroupAttribute`;
    - `ApplyAttributeValues`, `ApplyPalette{programmer:false}`;
    - a plain programmer commit;
    - patch default seeding, fixture removal, load.
  - Playback writers:
    - Cue snap and fade tick, move-in-black, both release plans;
    - Scene Block fade-out and End;
    - Timeline, child and Follow automation;
    - a programmer commit into a live scene. Its values release with that scene.
  - The first playback write to a key keeps the authored level under it.
  - An authored write takes the key back. The live result is last-writer-wins, as before.
  - The fade-tick write no longer clones the key String on every tick.
- **Flags.**
  - Effect enables: an id-keyed underlay map.
  - Node-graph enables: an underlay on `RuntimeNodeGraph`.
  - Video-output `enabled/opacity/blackout`: a per-field underlay on `RuntimeVideoOutput`.
  - Whole-vector rollbacks therefore restore underlays with their entries. The
    `SetEffectEnabledPublished` rollback restores its effect's entry.
  - Playback writers: Cue effect, node-graph and output targets, and the Cue fade's output part.
  - Authored writers: the explicit enable/opacity/blackout commands, the authored opacity fade
    and Show Spout pair replacement, plus any `update_video_output` that changes a field.
  - A node-graph upsert or output config that resends the live flag unchanged keeps the Cue's
    override.
- **Images.**
  - `build_persistence_snapshot` is the authored projection: authored levels and flags, no
    active Cue references, no running fade.
  - `build_live_look_persistence_snapshot` is the former image. Capture reads it, so "from
    current" still records what is on stage.
  - `RequestPersistenceSnapshot` carries the chosen `PersistenceProjection`.
- **Playback layer.** `protocol::PlaybackLayerSnapshot` holds:
  - active, Bank and group Cue ids;
  - releasable Cues, so a Coexist Cue no Bank names still releases;
  - live modifiers;
  - owned levels with their Bank (executor and playback-master scaling);
  - effect and node-graph flags;
  - per-field output overrides.
  - `RequestPlaybackLayerSnapshot` is a new internal read, inventoried as Engine source 285.
- **Load policy.** `PlaybackLoadPolicy` has three values:
  - `Discard`: Open, recent, template, backup, recovery, New and Standby entry.
  - `KeepCurrent`: Undo/Redo.
    - A running fade is carried at its targets, since a load cannot resume it.
    - A live Cue the target image removes is released as Remove Cue releases it, to its exact
      release targets. With several removed at once, a key returns to its release target
      unless a surviving Cue still holds it.
    - A live Cue the target keeps stays live under the target's Bank and group for that Cue.
    - A level whose Bank the target removes stays live without Bank scaling.
    - Overrides for removed items are dropped.
    - Release targets travel inside the process only (`#[serde(skip)]`).
  - `Apply(layer)`: Standby. It is strict: a reference the image cannot satisfy, including a
    level's Bank, rejects the load unchanged.
  - Saved active references are still validated (FC-27) and then never installed.

### 8.2 App (`app/src-tauri`)

- **Open.** `prepare_project_load` validates current-schema references as before, then removes
  the active references.
  - A version-1 file saved with a live Cue opens with a load warning: `Saved live Cue "<label>"
    was not restored; a show opens with no Cue live`.
  - `project_snapshot_for_save` also drops the references.
- **Undo/Redo.** `PreparedProjectLoad.playback` reaches the single publication seam.
  `navigate_project_history_with_coordinator` sets `KeepCurrent`. Output retirement, the media
  and Timeline stop, and the end of a recording take are unchanged.
- **Capture.** Create Scene from Current, Update from Current and its batch variant read
  `live_look_snapshot()`. Their history checkpoint and candidate stay on the authored image.
- **Standby.**
  - Every generation from this version writes a `syndocal-standby-{session}-{generation}.playback`
    sidecar (`version: 1`) before the manifest that commits it.
  - The manifest names the sidecar in optional `playback_file`, `playback_bytes` and
    `playback_checksum` fields, so `STANDBY_SYNC_VERSION` stays 1.
  - The sidecar is not `.json`, so the exact Take Over scanner ignores it. Retention prunes it
    with its generation.
  - A manifest may name only its own generation's sidecar. Retention computes the sidecar name
    instead of trusting the manifest's.
  - The worker reloads when the authored hash changes or the sidecar checksum differs from
    the last one it synced. Before that, every Cue trigger moved the hash and reloaded.
    - The worker's first generation compares with the Standby's own live layer, so a
      Standby already showing the same project and no Cue does not reload.
    - A layer the Standby could not apply counts as synced once the load succeeded with its
      warning, so it does not reload on every heartbeat.
    - An older Primary's live Cue references are dropped before the hash, as the load drops
      them, so they are not a change.
  - Warm apply and Take Over use `Apply`. A sidecar that does not match its generation falls
    back to `Discard` with a warning, so it never blocks authored sync or Take Over.
  - A damaged, missing or incompletely named sidecar rejects the generation, like a damaged
    project file. An intact sidecar of an unknown version or shape (a newer Primary) drops only
    the live layer, with the warning `The Primary's live Cue state was not applied: ...`.
- **Effect toggle.** The authored control-plane `set_effect_enabled` still decides on authored
  state. When the request equals the authored flag but a Cue holds the live flag the other
  way, the same engine write takes the effect back from playback. The receipt is `NoOp`, and
  the token, revision and history do not move.
- **Not changed, by design.** The frontend legacy `projectComparableSnapshot` reads the live
  snapshot and is used only before an authority baseline exists.

### 8.3 Compatibility (old behaviour, new behaviour, reason)

| Boundary | Old | New | Reason |
| --- | --- | --- | --- |
| Saved image | Live look, active Cue ids, mid-fade levels, Cue-written flags | Authored state only | Playback is not project data (decision Q1) |
| Opening a v1 file with active ids | Lighting levels and Cue marks restored | Levels open as authored state, no Cue live, load warning | One-way, observable migration; nothing the file holds is lost |
| Malformed active ids (current schema) | Rejected (FC-27) | Still rejected before removal | No silent clear |
| `PROJECT_FILE_VERSION` | 1 | 1 | Same shape; older builds open new files as "no Cue live" |
| Future project version | Rejected | Rejected (unchanged) | Existing strict equality |
| Standby manifest | v1 | v1 plus optional sidecar fields | Older readers ignore them and sync authored state |
| Standby sidecar | none | v1; damage, a missing file, missing fields or a foreign file name reject the generation | Fail closed on integrity |
| Standby sidecar from a newer Primary | none | Intact but unknown version or shape: authored state syncs, no live Cue state, load warning | An older Standby keeps syncing and can still Take Over |
| Undo/Redo | Cleared by any playback; when reachable, reinstalled the historic look | Survives playback; keeps the live look | Decision Q2 |

### 8.4 Evidence (Linux, debug)

- **New engine tests** (`playback_layer_tests.rs`, 11):
  - Cue, Timeline, release, toggle, group and automation playback never move the authored image.
  - The live look is still driven.
  - Authored edits over live playback reach the image.
  - Opening a live-Cue file starts with no Cue live.
  - Undo/Redo keeps the layer and continues exactly like a show that never undid anything.
  - An Undo that removes a live Cue matches Remove Cue.
  - An Undo whose target puts the live Cue in another Bank and group keeps it live there.
    **Mutation check:** with the Bank remap reverted, it fails (`left: None, right: Some(3)`).
  - An Undo during a fade lands on the fade targets and then releases like a show that let
    the fade finish.
  - Standby apply installs the layer, and rejects a mismatched one unchanged.
  - A live-scene programmer commit stays playback; a plain commit is authored.
  - **Mutation check:** with the fade-tick writer switched to `set_authored`, the invariant
    test fails (`.fixtures[0].attribute_values[1].value: 25004 -> 0`).
- **New app tests:**
  - `cue_trigger_keeps_the_exact_token_and_undo`, on the real headless harness and production
    reconcile. After a Cue the VJ Preview exact token still matches, the revision and hash are
    unchanged and Undo has one entry. Exact-token Undo then reverts the edit, keeps the Cue
    live at its level, and the reconciled hash matches.
  - `opening_a_file_saved_with_a_live_cue_starts_with_no_cue_live_and_says_so`, including
    FC-27 rejection.
  - `project_file_saved_with_a_live_cue_reopens_as_its_authored_image`: old writer bytes
    through production ingress, publication and re-save. The re-saved image equals the
    prepared image.
  - `effect_toggle_takes_a_cue_held_flag_back_without_moving_the_token`.
  - `standby_judges_playback_by_what_it_last_synced`: the first generation compares with the
    live layer, later ones with the last synced checksum; an older Primary's live references
    are not a change; an authored change still reloads.
  - `standby_generation_carries_and_verifies_the_playback_sidecar`: damage and incomplete
    fields reject; unknown version and unknown fields drop only the layer with the warning; a
    manifest naming a foreign sidecar is rejected; retention prunes sidecars.
  - `standby_applies_only_a_playback_layer_that_matches_its_generation`.
- **Changed assertions, because the contract changed:**
  - `project_snapshot_for_save_drops_volatile_runtime_state`: `active_cue_id` is now `None`.
  - Engine `current_schema_load_rejects_invalid_bank_global_and_group_active_references`: the
    valid saved reference is no longer installed, so the test fires it live to keep its
    atomicity check.
  - `project_load_does_not_carry_direct_child_transport_runtime`.
  - Inventory pins: AI0 checker and `control_plane.rs` tests, Engine 284 to 285 and the derived
    totals.
- **Suites, single-threaded as CI runs them:** see 8.6.
- `pnpm --dir app run check:release:static`: exit 0.
  - It includes `check:q1-q4-ledger`, `check:ai0-source-coverage` (Engine 285),
    `check:project-transaction`, `check:project-storage` and `check:output-ownership`.
- Warnings: see 8.6.

### 8.5 Boundaries and next action

- **Not observed natively.** Next action, the Windows native gate:
  1. Follow `qa/WINDOWS_NATIVE_BUILD.md`, then run `pnpm --dir app tauri build --no-bundle`, and
     confirm one responsive window.
  2. Make an authored edit, fire a Cue, and play a Timeline with Scene Blocks.
  3. Stage VJ Preview and check the exact token is accepted.
  4. Undo the edit and check the look is kept (output goes to Standby as before).
  5. Save, reopen, and check no Cue is live.
  6. Run a two-PC Standby drill: Cue on A, then Take Over on B shows A's Cue state behind
     Re-arm.
- **Unchanged.** Undo/Redo still retires output and stops media and the Timeline (decision
  Q4). Chaser and effect activation phases and video layer playback restart on Undo, as on
  any load.
- **Hidden authored base.** Release still goes to fixture defaults live, so after a release the
  stage can differ from the authored base until reopen or Undo shows it. Release-to-base would
  be a separate product decision.
- **Undo of a level edit made over a live Cue.** The edit took the key back from playback (last
  writer wins), so Undo shows the reverted authored level, not the Cue's. Before this change
  Undo reinstalled the image from before the edit, which held the Cue's level. Re-firing the
  Cue restores it.
- **Release targets across machines.** They travel within one process (Undo/Redo). A Standby
  that applies a Primary's layer uses each live Cue's default release values, as any load
  did before. A release there can differ from the Primary's when the Primary's targets came
  from earlier playback.
- **Local playback on a Standby.** A Cue fired on the Standby machine stays live until the
  Primary's project or playback layer next changes. Before this change it moved the hash and
  the next heartbeat overwrote it.
- **Stale effect overrides.** Remove Effect leaves the effect's playback entry. Nothing reads
  it: entries are read only for effects that exist, effect ids are allocated monotonically,
  and every load clears the map.
- **Remaining drift outside this decision.** Follow settlement and DJ Link start still install
  the active Timeline and retire tokens.
- **PR KTN44295080/Syndocal#3.** Both change `crates/engine/src/lib.rs`, including
  `build_persistence_snapshot` and the `mutates_persistence_snapshot` doc, so whichever merges
  second resolves those. Its `cue_trigger_rewrites_the_saved_look_and_fails_closed` test then
  flips to this contract; `cue_trigger_keeps_the_exact_token_and_undo` here is the
  replacement.

### 8.6 Suite runs and warnings (Linux debug, single-threaded as CI runs them)

- `cargo test -p engine --lib -- --test-threads=1`: 1143 passed, 1 failed, 14 ignored.
  - The failure was `control_plane::tests::exact_engine_command_inventory_is_generated_once_and_conservative`,
    whose pin of 284 Engine commands predates `RequestPlaybackLayerSnapshot`. Raised to 285; it
    passes on rerun with the inventory and playback tests (11 passed).
  - An earlier full run made while a Tauri build competed for the 4 cores failed two
    time-budget tests. Both passed 3/3 alone and in this idle run.
- `cargo test -p syndocal --bin syndocal -- --test-threads=1`: 1735 passed, 0 failed, 13 ignored.
- Warnings for the configurations run (engine lib and tests, syndocal bin and tests): baseline 1,
  current 1, delta 0. The one warning is the existing unused `safety_write_gate` in
  `crates/io/src/serial_dmx.rs`.
- After the review fixes (8.7), on an idle machine:
  - `cargo test -p engine --lib -- --test-threads=1`: 1146 passed, 0 failed, 14 ignored.
  - `cargo test -p syndocal --bin syndocal -- --test-threads=1`: 1737 passed, 0 failed,
    13 ignored.
  - `pnpm --dir app run check:release:static`: exit 0.
  - Warnings: still only the existing `safety_write_gate` warning. No `#[allow]` or `#[expect]`
    added.
  - `rustfmt`: new files format clean; `main.rs` and `authored_control_plane.rs` gained no
    unformatted hunks.
- `rustfmt` (copies, counted per file against `88c5861`): `crates/engine/src/lib.rs` 66 hunks
  before and after; `app/src-tauri/src/main.rs` 229 before and after; touched child files
  unchanged. The new files format clean. `git diff --check` clean.

### 8.7 Independent review and fixes

An independent review of `eb62381` raised the findings below. Each fix is in this branch and
named by its evidence in 8.4.

| # | Finding | Outcome |
| --- | --- | --- |
| 1 | Undo whose target puts a live Cue in another Bank or group released it | Fixed: the Cue stays live under the target's Bank and group |
| 2 | An effect toggle to the authored value did nothing while a Cue held the live flag the other way | Fixed: the toggle takes the effect back from playback; `NoOp` receipt, token unchanged |
| 3 | The Standby compared the sidecar with a fresh engine capture, so a layer it could not apply reloaded the project on every heartbeat | Fixed: it compares the sidecar checksum with the last one it synced |
| 4 | Retention removed whatever sidecar file a manifest named | Fixed: a manifest may name only its own sidecar; retention computes the name |
| 5 | Undo during a fade froze intermediate levels that no Cue owns | Fixed: a running fade is carried at its targets |
| 6 | Carried Cues lost their exact release targets | Fixed within one process; across machines see 8.5 |
| 7 | A level whose Bank the target removes made the carried layer invalid | Fixed: Undo keeps it without Bank scaling; Standby `Apply` rejects it |
| 8 | Removing several live Cues at once released keys a surviving Cue still held | Fixed: a key returns to its release target only when no surviving Cue holds it |
| 9 | An older Primary's live Cue references reloaded the Standby on every heartbeat | Fixed: they are dropped before the hash |
| 10 | An unknown sidecar version blocked an older Standby from syncing a newer Primary | Fixed: only the live layer is dropped, with a warning |
| 11 | Undo of a level edit made over a live Cue shows the authored level | Kept and documented in 8.5 |
| 12 | Remove Effect leaves a stale playback entry | Kept: nothing reads it (8.5) |
| 13 | Bank origins kept for authored keys | No change: an authored write already clears its origin |
