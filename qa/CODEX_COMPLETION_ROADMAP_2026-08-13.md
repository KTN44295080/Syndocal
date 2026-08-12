# Syndocal completion roadmap and frozen checkpoint

Status: implementation paused at the user-requested checkpoint on 2026-08-13 (JST).

This document is the detailed continuation contract from the current checkout to a release candidate that can truthfully be compared with Daslight 5 and SynapseRack. It records what is committed, what is only present in the dirty worktree, what has been tested, what is still unsafe, the dependency order, file ownership, required evidence, commit boundaries, and the native completion gate.

It does **not** claim that Syndocal is complete. It does **not** replace:

- `qa/SYNDOCAL_UI_PRODUCT_VISION.md`
- `qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md`
- `qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md`
- `qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md`
- `qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md`

Those files define the product target. This file defines the route from the frozen implementation checkpoint to that target.

## 1. Stop condition and exact current checkpoint

Implementation was intentionally stopped after these independently useful checkpoints:

1. Protocol and Engine Media Asset schema/runtime were committed and focused tests were green.
2. Frontend staged Media Asset orchestration was committed as a partial checkpoint.
3. Additional uncommitted frontend work completed Start/Reserved Prepare, exact phase continuity, real AbortControllers, post-Begin cancellation, and reserved relink wiring; focused frontend gates were green and a read-only adversarial review found no frontend-local P0/P1 in Import/Add/Bootstrap, then confirmed reserved relink closed the remaining first-hash frontend gap.
4. Additional uncommitted backend work completed Start reservation, cancellation/admission CAS, Windows retained-file guards, availability identity checks, TTL reaping, finalize idempotency, per-key single-flight, Start-authority binding, and reserved relink. Focused Rust gates were green.
5. Work stopped before implementing the next backend-owned authoritative Media Asset commit boundary. Two Claude Opus attempts implemented the earlier backend registry work successfully. A later broad authoritative-commit attempt and a narrower phase-A attempt produced no file changes and were terminated rather than counted as progress.

No new feature tranche should begin before the current dirty state is re-anchored against this document.

### 1.1 Committed checkpoints

| Commit | Meaning | Completion claim |
| --- | --- | --- |
| `c90e5a2 feat: harden atomic show runtime and media assets` | Protocol and Engine schema/runtime/allocator/publication/rollback checkpoint | Focused Protocol/Engine checkpoint only |
| `ed3f473 feat: stage media asset authority workflows` | Frontend staged Prepare/Finalize/Commit workflow and static proof checkpoint | Partial frontend wiring only; not reply-loss complete |

The last observed branch was `codex/syndocal-v1.0`.

### 1.2 Uncommitted Media Asset files owned by the current tranche

- `app/src-tauri/src/main.rs`
- `app/src/App.tsx`
- `app/src/types.ts`
- `app/src/createVideoRuntimeController.ts`
- `app/src/mediaAssetAuthority.ts`
- `app/scripts/check-media-asset-authority.mjs`
- `app/scripts/check-vj-first-run.mjs`

These files contain valid progress but are **not** one reviewed completion unit yet. Do not stage them together with unrelated dirty files merely because they are already modified.

### 1.3 Shared/user-owned dirty state that must be preserved

The checkout contains many other modified and untracked files, including release notes, QA specifications, output ownership work, project authority work, PATCH/GDTF work, native video transports, UI components, localization, and reference captures. They must not be reset, checked out, or silently folded into a Media Asset commit.

Before every future commit:

1. Run `git status --short`.
2. Stage only an explicit file list.
3. Run `git diff --cached --name-only` and compare it with the intended ownership list.
4. Run `git diff --cached --check`.
5. Never stage `RELEASE_STATUS.md`, `qa/CODEX_*`, visual references, or another tranche's files unless that exact commit explicitly owns them.

## 2. Current progress estimate

These percentages are planning estimates, not release evidence. They must move backward when review invalidates an assumption.

| Area | Frozen estimate | Basis |
| --- | ---: | --- |
| Overall route to the currently planned Syndocal product target | 81% | Core runtime and several authority foundations exist; Media Asset T1, Clip Slots, distributed ShowClock, comparative/native acceptance remain |
| Media Asset T1 overall | 85% | Schema/runtime/frontend staging are advanced; terminal reply-loss/history closure and compatibility/cross-platform boundaries remain |
| Protocol Media Asset schema/migration/validation | 100% for T1 scope | Focused Protocol tests passed before commit |
| Engine Media Asset runtime/allocator/Published rollback | 94% | Focused tests passed; Bootstrap output/fade rollback proof remains |
| Frontend Media Asset orchestration | 98% for current API | Import/Add/Bootstrap/Relink sequencing and cancellation are wired; it must be changed once authoritative commit API is frozen |
| Backend Media Asset prepare/finalize/registry | 94% | Focused registry, commit, relink tests passed; authoritative terminal transaction is missing |
| Media Asset adversarial/final review | 78% | Multiple P1s found and fixed; current backend still has known P1s and no final GO |
| Native release/UI/hardware acceptance for this tranche | 0% | No native release build or maximized-window acceptance was run for the current tranche |
| Clip Slot T2 | design complete, implementation 0% | Transition model is accepted; code has not begun |
| Distributed ShowClock/2PC | architecture requirements drafted, implementation 0% | Local clock and machine-local output fence exist; authenticated distributed authority does not |

## 3. Evidence already obtained

### 3.1 Protocol and Engine

Previously reported green evidence:

- `cargo test -p protocol media_asset --locked -- --nocapture`: 4 passed.
- `cargo test -p engine media_asset --locked -- --nocapture`: 8 passed.
- `cargo check -p protocol --locked`: passed.
- `cargo check -p engine --locked`: passed with the known pre-existing dead-code warnings.

Implemented foundations include:

- stable `MediaAssetId` and authored media catalog;
- additive legacy layer-to-asset migration;
- strict hash/size pairing and asset metadata validation;
- engine catalog-only imports;
- atomic paired asset/layer compatibility transactions;
- relink/update of all referencing layer source projections;
- allocator maximum observation and collision handling;
- definitive `ProjectSnapshotLoadAdmission` ACK behavior;
- rollback of Media Asset transaction state, including output/fade state in implementation.

### 3.2 Backend registry/finalize checkpoint

The most recent supervisor rerun of the frozen backend produced:

- `cargo test -p syndocal media_asset_operation --locked -- --nocapture`: 10 passed.
- `cargo test -p syndocal media_asset_commit_ --locked -- --nocapture`: 9 passed.
- `cargo test -p syndocal media_asset_relink_ --locked -- --nocapture`: 6 passed.
- `cargo check -p syndocal --locked`: passed with known warnings.
- `git diff --check -- app/src-tauri/src/main.rs`: passed, apart from the repository LF-to-CRLF notice.

The current backend includes:

- `start_media_asset_operation`;
- `prepare_reserved_media_assets`;
- `prepare_reserved_media_asset_relink`;
- exact Start authority stored in the reservation;
- `Cancelable -> Admitted | Cancelled` CAS;
- per-exact-key single-flight and a second receipt check inside the lane;
- finalize exact-retry handling;
- Windows retained deny-write/delete file guard and path identity verification;
- availability verification bound to the current path on Windows;
- a single TTL reaper owned by `AppState`;
- legacy staged fail-fast rejection for Failed inputs while retaining duplicate Skip compatibility.

This evidence does **not** close the terminal history/reply-loss boundary described below.

### 3.3 Frontend frozen checkpoint

The most recent frontend owner and supervisor runs reported green:

- `pnpm --dir app exec tsc --noEmit`;
- `node app/scripts/check-media-asset-authority.mjs`;
- `node app/scripts/check-vj-first-run.mjs`;
- `node app/scripts/check-vj-media-import-access.mjs` at the prior committed checkpoint;
- `node app/scripts/check-backend-operator-contract.mjs`, most recently 394 backend commands / 317 literal frontend calls / 163 transactional mutations;
- `pnpm --dir app run check:localization`, previously 3149/3149;
- `node app/scripts/check-project-storage-helpers.mjs` at the prior committed checkpoint;
- scoped `git diff --check`.

The frozen frontend currently provides:

- Start -> Reserved Prepare -> Finalize -> staged commit for import, local layer add, first-run bootstrap, and relink;
- exact request/generation/E/R/H continuity checks;
- mapping-flush and picker-era epoch fences;
- AbortControllers for add/import/bootstrap/relink;
- abort on project authority replacement and component unmount;
- a pre-Begin and post-Begin abort barrier;
- ticket cancellation before command dispatch when abort arrives during Begin wait;
- catalog-only normal import, with zero layer creation;
- first-run availability requiring empty media catalog/layers/outputs/non-Main compositions;
- first-run mixed-invalid fail-fast before Finalize/Commit;
- no production calls to the old raw media routes.

The frontend harness explicitly states that it does not prove Begin reply-loss or a terminal history receipt.

## 4. Non-negotiable implementation principles

Every remaining tranche must preserve these invariants.

### 4.1 Authority

- One project identity is `{epoch, revision, checkpoint_hash}` plus the relevant monotonic metadata generations.
- No asynchronous result may apply to another project identity.
- One operation must have one server identity that survives IPC response loss.
- A successful external effect must never be reported as a failed/no-op UI operation merely because a reply was lost.
- A retry must discover the already-completed terminal result; it must not create a new layer, history entry, save, cue, output, or recovery transition.

### 4.2 File I/O

- Hashing/probing is cancellable, runs outside a pending project transaction, and is executed on blocking workers.
- Hash and probe metadata must describe one stable file image.
- A project commit performs only bounded identity/metadata checks.
- Missing, unreadable, changed, hash-mismatched, and legacy-unverified states remain distinct.
- Machine-local availability/cache state is not serialized into `.sdc` unless it is explicit authored state.

### 4.3 Engine publication

- Multi-object operations use one definitive Published ACK.
- Queue admission is not success.
- Cancellation before admission means zero mutation.
- After admission, cancellation loses and the caller receives the definitive terminal result.
- A publication failure restores the complete A image, including derived caches, outputs, fades, selections, and allocator invariants where required.

### 4.4 UI

- Do not shrink typography, controls, icons, spacing, or hit targets to make features fit.
- Use reflow, disclosure, pagination, and internal scrolling.
- One dominant action per region.
- Import remains reachable in empty, populated, compact, mixer, paginated, and full-bank states.
- Operator lock checks remain server-authoritative; changing a command wrapper must not accidentally classify a mutation as a read.

### 4.5 Git and checkpoints

- Commit only a frozen, reviewed, focused unit.
- Report commit hash and percentages after every committed checkpoint.
- A green build with zero selected tests is not evidence.
- Static, browser, Rust, native, and hardware evidence must be named separately.

## 5. Critical path A: finish Media Asset T1

Media Asset T1 blocks Clip Slot T2 and the final Video operator comparison. Complete A1 through A8 in order.

### A1. Backend-owned authoritative Media Asset terminal transaction

Priority: highest. Status: not implemented at this stop point.

Problem:

- The existing staged commit receipt includes `project_transaction_id`.
- If the staged command reply is lost after engine publication, the generic frontend wrapper cancels the old ticket and records a truthful `Interrupted:` history entry.
- A retry obtains a new transaction ID, cannot find the old receipt, and either finds the token consumed or the operation already admitted.
- The UI reports failure although B exists. A user retry with a fresh media operation can add a duplicate layer.
- If `begin_project_transaction` itself commits its pending reservation but its reply is lost, the renderer has no transaction ID to cancel or query; the live owner can remain pending and block mutations.

Required design:

1. Introduce a backend-owned Media Asset authoritative commit command family.
2. Do not expose a frontend project transaction ID on the new path.
3. One backend call must own:
   - exact operation receipt lookup;
   - registered-owner/operator admission;
   - mapping/project epoch validation;
   - internal baseline capture;
   - bounded file identity CAS;
   - Media Asset engine Published ACK;
   - project revision/hash/publication/history preflight and commit;
   - terminal result recording;
   - authoritative bundle/history result construction.
4. Use one shared internal transaction helper for Import, Relink, File Layer, Still Layer, Batch Layers, and Bootstrap.
5. Terminal receipt identity must be stable across IPC reply loss and must not include an ephemeral frontend transaction ID. Minimum identity:
   - command kind;
   - prepared token;
   - request ID;
   - operation generation;
   - owner ID;
   - exact Start authority;
   - shape fingerprint where a label/output/policy can change semantics.
6. Terminal receipt payload must include:
   - the command-specific result;
   - authoritative `ProjectHistoryMutationResult` or equivalent paired bundle;
   - terminal status sufficient for exact query/retry.
7. Receipt lookup must occur before token lookup, so token cleanup cannot erase success evidence.
8. Concurrent exact callers must single-flight. A different key for the same operation must not republish and must be able to discover the canonical terminal receipt.
9. All fallible history/generation/hash preparation must occur before engine ACK. Post-ACK coordinator work must be assignment-only or otherwise recoverably terminal.
10. Every branch clears internal pending/active state exactly once.

Suggested additive API names:

- `commit_prepared_media_assets_authoritative`
- `commit_prepared_media_asset_relink_authoritative`
- `commit_prepared_video_file_layer_authoritative`
- `commit_prepared_still_image_layer_authoritative`
- `commit_prepared_local_media_layers_authoritative`
- `commit_prepared_bootstrap_vj_show_authoritative`
- `get_media_asset_operation_terminal_result` only if an exact retry of the commit command is insufficient.

Suggested result shapes:

```text
MediaAssetAuthoritativeImportResult {
  report,
  mutation
}

MediaAssetAuthoritativeLayerResult {
  layer_id or layer_ids,
  mutation
}

MediaAssetAuthoritativeBootstrapResult {
  setup,
  mutation
}

MediaAssetAuthoritativeRelinkResult {
  report,
  mutation
}
```

Required deterministic tests:

- response lost after engine ACK -> same operation retry returns identical payload/history;
- history response lost -> same terminal query returns identical authority/history;
- two concurrent exact calls -> one engine publication, one revision change, one history entry;
- same operation with a different transient request shape -> zero second publication and canonical result discovery;
- cancel wins before admit -> zero engine/project/history mutation;
- cancel arrives after admit -> cancel reports loss and commit returns definitive success;
- engine reject -> zero project revision/history and no pending state;
- post-publication bookkeeping injection -> no false failure or unrecoverable B;
- no branch leaves `project_transaction_active=true` or an internal pending record;
- receipt expiry cannot occur while a legitimate in-flight response/recovery is still possible.

Checkpoint commit A1 should own only `app/src-tauri/src/main.rs` unless an additive DTO must live in a shared protocol file. It must not include frontend wiring.

### A2. Frontend authoritative commit wiring

Start only after A1 API and return DTOs are frozen and reviewed.

Required changes:

1. Keep Start -> Reserved Prepare -> Finalize unchanged.
2. Flush mapping authority before authoritative commit.
3. Capture and pass the exact picker/start project epoch.
4. Preserve abort-before-dispatch after the mapping flush.
5. Invoke the new authoritative command directly, without the generic frontend Begin/Commit wrapper.
6. Preserve operator-lock classification explicitly. Removing a command from `projectMutationCommands` must not make it read-only; add a dedicated server-authoritative mutation classification if needed.
7. Apply the paired authoritative bundle/history result with the existing monotonic application helpers.
8. On lost reply, retry/query with the same operation identity. Never generate a new request ID as a recovery strategy.
9. Show a truthful terminal message only after terminal result resolution.
10. Retain exact cancel behavior; before backend admission cancel may win, after admission the UI waits for the terminal result.

Required frontend tests:

- lost authoritative commit response -> same operation query -> success shown once, one layer;
- authority A -> B during picker/prepare/flush -> zero B mutation from A intent;
- abort before dispatch -> zero backend commit call;
- abort while backend admitted -> terminal result applied, no false Cancelled;
- Full/Partial operator modes block the same command server-side and in UI;
- event/reply/poll duplicates apply one result and one history status;
- stale B reply after project C is a no-op;
- Import remains catalog-only; File/Still adds exactly one layer; Bootstrap remains all-or-nothing; Relink updates all references.

Checkpoint commit A2 should own only the six frozen frontend files and any focused harness that directly proves the new API.

### A3. Preserve old IPC names while replacing their internals

Open P1 even though current production frontend calls are zero.

Affected compatibility commands:

- `add_video_file_layer`
- `add_still_image_layer`
- `add_local_media_layers`
- `bootstrap_vj_show`
- also verify `refresh_video_layer_metadata` compatibility semantics.

Current risk:

- legacy File/Still paths enqueue unacknowledged legacy commands;
- batch loops per file and can leave a committed prefix;
- byte hash/content identity is not established;
- raw IPC callers bypass project authority/history;
- Bootstrap remains a legacy probe/identity route.

Required result:

- exact old command names/arguments/return shapes remain compatible;
- internals call the same hash/finalize/Published/authoritative history boundary;
- mixed invalid batch means zero mutation for the legacy all-or-nothing contract;
- direct/raw callers cannot bypass operator lock, project epoch, or history truth;
- no compatibility command returns success on queue admission alone.

Required tests:

- old IPC deserialization shapes still work;
- mixed valid+missing/corrupt -> zero layer/catalog/history;
- QueueFull/timeout/expiry/publication failure -> zero mutation;
- success -> content-hashed catalog + exact layer refs + one history entry;
- raw invocation under Full/Partial operator policy rejects server-side;
- duplicate input path behavior matches the historical contract exactly.

### A4. Cross-platform file coherence decision

Windows is the current native target and its retained handle/identity path is statically strong. Non-Windows remains open if supported.

Risk on non-Windows:

- prepare may observe hash A, probe metadata B, then restored A during final rehash;
- availability may hash an already-open A while the path now names B.

Choose one explicit product contract:

1. Cross-platform support: implement a stable file identity/descriptor strategy or a complete metadata/hash/probe envelope that detects A -> B -> A, and prove it on each supported OS.
2. Windows-only guarantee for this release: fail closed or mark unsupported behavior on other platforms and state that scope in release documentation. Do not silently imply the Windows guarantee is portable.

Tests:

- deterministic A -> B -> A during probe;
- atomic rename during availability hash;
- same length/restored mtime replacement;
- hard-link/same-file relink;
- write/delete denied during finalized Windows token lifetime;
- retained handle released at cancel/commit/expiry/app shutdown.

### A5. Availability cancellation and reaper hardening

P2, but complete before release.

- Add reserved operation identity to verified availability when the inspection can be long.
- Allow cancellation before first read and between chunks/probe entries.
- Do not change project revision/history during inspection.
- Treat reaper thread spawn failure as setup failure or a visible degraded state; do not silently `.ok()` and leave retained file locks indefinitely.
- Test TTL cleanup without another registry call.
- Test app shutdown joins/unparks the one reaper thread promptly.

### A6. Fill Engine proof gaps

- Add a forced publication-failure Bootstrap test that asserts restoration of:
  - `video_outputs`;
  - `video_output_fades`;
  - catalog;
  - layers;
  - compositions;
  - layer fades;
  - last error;
  - allocator invariants where relevant.
- Add real command-level concurrent tests for all six Media Asset commit shapes, not only the registry closure.
- Test different-key/same-operation recovery.
- Test retained output IDs and layer IDs are identical on exact retry.

### A7. Media Asset T1 focused gate

Run from a frozen tree, in order:

1. `cargo fmt --check --package protocol --package engine --package syndocal`
2. `cargo test -p protocol media_asset --locked -- --nocapture`
3. `cargo test -p engine media_asset --locked -- --nocapture`
4. `cargo test -p syndocal media_asset_operation --locked -- --nocapture`
5. `cargo test -p syndocal media_asset_hash --locked -- --nocapture`
6. `cargo test -p syndocal media_asset_finalize --locked -- --nocapture`
7. `cargo test -p syndocal media_asset_availability --locked -- --nocapture`
8. `cargo test -p syndocal media_asset_relink_ --locked -- --nocapture`
9. `cargo test -p syndocal media_asset_authoritative --locked -- --nocapture`
10. `cargo test -p syndocal media_asset_commit_ --locked -- --nocapture`
11. `cargo check -p syndocal --locked`
12. `pnpm --dir app exec tsc --noEmit`
13. `node app/scripts/check-media-asset-authority.mjs`
14. `node app/scripts/check-vj-first-run.mjs`
15. `node app/scripts/check-vj-media-import-access.mjs`
16. `node app/scripts/check-backend-operator-contract.mjs`
17. `pnpm --dir app run check:localization`
18. `git diff --check` for the exact tranche file list.

If any test filter selects zero tests, correct the command and rerun; do not count it.

### A8. Media Asset native acceptance

After focused/static review is P0/P1 clean:

1. Verify the exact path of any running `target/release/syndocal.exe` from this checkout.
2. Force-terminate only that exact process before linking.
3. Run `pnpm --dir app tauri build --no-bundle` successfully.
4. Launch this checkout's exact `target/release/syndocal.exe`.
5. Verify exactly one responsive `Syndocal` window.
6. Maximize it before all UI operations.
7. Run:
   - empty import;
   - populated import;
   - 12-file catalog-only import;
   - single File/Still add;
   - mixed failure;
   - cancel during first hash;
   - project replacement during prepare;
   - missing/hash mismatch/relink all references;
   - first-run bootstrap;
   - restart/load/no file touch;
   - explicit save and reload.
8. Record that native evidence separately from TypeScript/static/Rust evidence.

## 6. Critical path B: Clip Slot T2

Do not start until Media Asset T1 is committed and native-smoke clean.

### B1. Protocol schema and migration

Add:

- `VideoClipSlotId`;
- ordered authored slots per layer;
- stable `media_asset_id` reference;
- in/out points;
- loop mode;
- speed;
- cue points;
- launch quantization;
- clip-local effect overrides;
- serde defaults for old projects.

Migration rule:

- each tranche-1 legacy layer gets exactly one default slot referencing its existing asset ID;
- never create a second MediaAsset;
- preserve layer ID/source projection/effects/transform/blend/routing;
- clone-then-validate-then-commit;
- load does not rewrite disk; only explicit save serializes the new representation.

Tests:

- byte-idempotent normalization;
- invalid second layer leaves the entire project unchanged;
- no asset duplication;
- legacy save/load semantics preserved;
- slot IDs and allocator maxima never collide.

### B2. Engine authored/runtime split

Authored:

- slot banks and ordering;
- slot parameters;
- layer membership and default selection.

Runtime-only:

- active slot;
- queued slot;
- playhead/decoder state;
- pending quantized launch;
- transition progress;
- decode/cache availability.

Commands must be definitive Published transactions:

- assign asset to slot;
- create/remove/reorder/duplicate slot;
- queue/cancel queue;
- launch/take;
- seek;
- import-and-assign direct drop.

Tests:

- 32 visible slots, paging/full bank;
- queue/launch/cancel exactness;
- project replacement cancels old queued actions;
- missing asset never exposes unintended content;
- runtime selection never dirties the project;
- authored changes create one Undo/history entry.

### B3. Direct drop routing

- `.sdc` opens only on the project surface.
- Audio routes to Timeline Audio lanes.
- Visual media routes to library or the exact slot/layer target.
- Direct layer/slot drop is one guarded `import -> create slot -> assign` transaction.
- Multi-file ordering is deterministic.
- Failure never changes the currently live slot.

### B4. Edit/Control UI

Edit > Video:

- Media Library and slot bank in the upper region;
- active/queued truth;
- composition/output canvas;
- layer/clip/transform/composite/color/effects/mapping inspector;
- same geometry as accepted Edit > Lighting;
- no control shrink.

Control > Video:

- persistent Import;
- clip grid;
- layer selection;
- Preview/Program;
- one dominant Take;
- transition/duration/quantization;
- output truth.

Acceptance:

- 1920x1080, 1920x1032, 1366x768, 1280x720;
- zero app outer scroll/clipping/overlap;
- Import reachable in all bank states;
- keyboard and coarse-pointer paths;
- accessible names and focus return;
- no normal clip pad exposes more than three primary actions.

### B5. Commit sequence

1. Protocol schema/migration commit.
2. Engine slot runtime/Published transactions commit.
3. Backend authority bridge commit.
4. Frontend Edit/Control wiring commit.
5. Native acceptance evidence commit/documentation update.

Never combine all five into one unreviewable commit.

## 7. Critical path C: Video effects and transitions

After Clip Slots are stable:

### C1. Effect scopes

Generalize the existing effect chain into explicit scopes:

- Clip FX;
- Layer FX;
- Transition FX;
- Composition/Group FX;
- Output FX.

Preserve legacy built-in color/FX and ISF ordering. Add enable/bypass, reorder, reset, preset, automation, MIDI/OSC/DMX, BPM and audio-reactive bindings without changing legacy render order.

### C2. Clip Take

- Cut and crossfade first.
- Then wipe/luma/matte/displacement/blur/glitch/custom.
- Duration in milliseconds, beats, or bars.
- Explicit late-trigger policy.
- Interrupt/reverse behavior deterministic.
- Missing incoming content keeps last valid/fallback frame.

### C3. Layer Transition Bus

- stable bus IDs;
- explicit member layers/groups;
- scoped from/to;
- unrelated overlays remain visible;
- multiple non-conflicting buses allowed;
- conflicts reject deterministically;
- no layer playhead/FX destruction.

### C4. Mapping/timeline integration

- stable IDs, never UI indices;
- one Timeline playhead for Lighting/Video/Audio/Automation;
- MIDI/OSC/DMX target slots/buses/effects;
- ShowClock beats/bars are the timing authority;
- Audio Reactive/Auto Operator have Armed/Running/Hold/Fault and bounded loss behavior.

## 8. Critical path D: PATCH/GDTF and Stage integrity

This work is largely designed and partly implemented, but must be resumed from a fresh static audit because `main.rs`, Engine, App, and panels have moved.

### D1. Machine/session cache purity

- `list_gdtf_fixture_cache` is pure list only.
- Share cache writes machine cache only.
- import/load/verified/custom selection is session preview only.
- opening PATCH never changes project hash/revision/history/dirty.
- same-value reads are strict no-ops.

### D2. Atomic Patch/Repair

- resolve profile without project insertion;
- validate identity/mode/layout/footprint/group/address/conflict for the full batch first;
- use Engine `PatchFixturesPublished` and `repair_fixture_profile_published` definitive ACKs;
- commit required embedded profiles and fixture snapshot as one project image/history entry;
- reject/cancel/timeout/publication failure leaves Engine, ancillary, token, and history unchanged;
- batch second-item failure applies zero fixtures.

### D3. Server-side mutation admission

- new mutation commands require owner/project transaction/expected epoch, or use an equivalent backend-owned authoritative operation;
- raw component `tauriInvoke` cannot bypass operator policy/history;
- component calls use injected command facade;
- static scan covers all `app/src/**/*.{ts,tsx}`, not App alone.

### D4. Stage import and mutation

- file dialog/read/parse remains transaction-free;
- capture pre-dialog project identity;
- after dialog, reject if identity changed;
- transactional upsert uses Published ACK;
- missing apply/remove/set target is Err, not silent engine `last_error` success;
- one successful import = one Undo entry.

Focused acceptance matrix:

- catalog mount leaves checkpoint bit-identical;
- Share batch cancel changes machine cache only;
- invalid profile/footprint/group/conflict leaves full project unchanged;
- Patch N success = N fixtures + required profiles in one publication/Undo;
- Repair reject restores full fixture state;
- stage invalid/reject unchanged, success one history entry;
- operator Partial/Full and stale epoch/raw direct call reject server-side.

## 9. Critical path E: generic project authority, transaction, recovery, and save

Media-specific authoritative commit removes the generic Begin boundary only from media operations. The generic system still needs its own completion tranche.

### E1. Generic Begin reply-loss and transaction lifetime

Open P1:

- Begin can install pending/active state before the renderer receives the ticket.
- owner liveness recovery handles renderer retirement, but not every live-owner reply-loss case.

Required design:

- client operation ID on Begin;
- idempotent Begin receipt/query;
- exact owner/window generation;
- no fixed TTL that can expire during a valid long-running command;
- renderer/window retirement cancels only that retired owner;
- in-flight command accounting prevents late publication after retirement;
- partial B becomes one `Interrupted:` Undo entry;
- no-change orphan produces no history entry;
- retry does not steal a live pane's transaction.

Tests:

- Begin side effect + dropped reply -> query/adopt/cancel;
- pane close -> recover exactly once;
- main reload does not steal live pane;
- owner replacement cancel-preflight failure retains old mapping and retries;
- late old-owner Commit/Cancel rejects;
- callback admission reopens after cleanup.

### E2. Project authority bundle/application

Retain and re-verify:

- atomic staged Solid batch application;
- exact duplicate event/reply idempotency;
- stale event/reply rejection;
- replacement/mapping-replacement generations;
- history/path/disposition/recovery/input runtime generations;
- poll liveness under runtime-only DMX;
- dirty mapping rebase for ordinary mutation but forced hydrate for identity/history replacement;
- no post-load raw refresh races;
- no stale A snapshot/group/policy/runtime response after B bundle.

### E3. Recovery durability

Retain and re-verify:

- backend durable recovery serial/tag startup handshake;
- source serial + unique request ID + target hash CAS;
- exact recovery intent consumer;
- event/reply/poll equivalence;
- ACK preserves an acknowledged checkpoint until coherent B capture or CleanSave;
- storage failure never blocks live authority application;
- localStorage crash boundary is stated truthfully;
- draft generation CAS around ACK/clear;
- latest-wins browser recovery capture.

### E4. Save durability

Retain and re-verify:

- per-target publication serialization;
- pre-dialog reservation for Save As/template;
- stale ticket rejected before journal advance/file replace;
- CleanSave journal/recovery serial ordering;
- post-replace journal failure advances live provisional truth correctly;
- indeterminate rollback failure latches all mutation paths fail-closed;
- backend server-side mutation admission prevents catalog/raw bypass of the latch;
- delayed Save response cannot mark a replacement clean;
- mapping debounce flushed before Save/Save As/template/backup/update install.

## 10. Critical path F: input/runtime/output truth

### F1. Input runtime generations

- separate worker callback epochs from observable runtime-state generations;
- connect/disconnect/start/stop/self-failure increments exactly once;
- status/event/poll application is monotonic;
- delayed raw DMX status cannot resurrect retired UI;
- mapping retirement reply loss converges by bundle/poll;
- Learn allows its own revision advance but rejects identity/other-author changes;
- MIDI/OSC/DMX Learn success reconnects the new worker and shows success.

### F2. Output ownership tranche closure

The machine-local role/fence exists, but re-verify:

- startup all-deny;
- explicit Arm;
- exact Lighting/Video/Both/Standby matrix;
- physical permits fence NDI/Spout/Display/DMX creation and presentation;
- failure retains teardown lease until acknowledged;
- Standby project swap remains fenced for the full replacement interval;
- Take Over reloads the latest checkpoint and still requires explicit Arm;
- no raw output startup bypasses the ownership gate.

Native SDK/hardware evidence remains mandatory for final completion.

## 11. Critical path G: distributed ShowClock and two-PC operation

### G1. What ShowClock is

ShowClock is the authoritative show-time domain shared by Lighting, Video, Audio, Timeline automation, and two-PC failover. It is not merely the formatted timecode in the top bar.

One ShowClock image includes at least:

- show position;
- BPM;
- beat and bar phase;
- transport state;
- chosen external source;
- source/clock quality;
- monotonic clock generation;
- session/fencing generation;
- last accepted sequence/action ID.

### G2. Input authority

MIDI Clock, MTC, LTC, Ableton Link, OSC timecode, future ArtTimeCode, MSC, and ArtSync are sources into one selected Primary authority. Primary and Standby must not independently choose and apply competing sources.

Source selection requirements:

- explicit priority/configuration;
- quality/lock status;
- holdover policy;
- source loss timeout;
- bounded slew for small correction;
- Hold on backward jump or large step;
- explicit operator resume/relock.

### G3. Peer synchronization

Primary publishes authenticated samples containing:

- session ID;
- project hash;
- media manifest hash;
- clock generation;
- fencing generation;
- sequence;
- monotonic send timestamp;
- show position/BPM/phase/transport;
- expiry/nonce/MAC or mutually authenticated transport identity.

Standby estimates:

- offset;
- RTT/delay;
- drift;
- jitter;
- freshness;
- confidence.

It slews within a bounded correction envelope and enters `HOLD`, `STALE`, or `FAULT` rather than stepping backward or silently following an unauthenticated/stale peer.

### G4. Timestamped actions

GO, STOP, BACK, RELEASE, BLACKOUT, TAKE, clip launch, transition start, and timeline jumps carry:

- stable action ID;
- target ShowClock time;
- fencing generation;
- project/media/clock generation where relevant;
- late policy;
- dedupe lifetime.

Every action is exactly-once within the session. Duplicate/reordered packets do not double GO/Take. Late behavior is explicit per action type: execute immediately, drop, or Hold; never implicit.

### G5. Failover and split brain

Standby remains physically disarmed while synchronizing.

Take Over requires:

- stale/dead Primary determination under the chosen fencing model;
- latest project/media/clock generations;
- cancellation of old queued actions;
- invalidation of the old lease/fence;
- explicit operator Arm after takeover state is coherent.

Two nodes alone cannot prove that the other stopped during a network partition. Safe automatic failover therefore requires at least one of:

- a third witness/quorum;
- a shared strongly consistent lease/fence;
- physical output exclusion/interlock.

Without that, both peers enter Hold and takeover is guarded/manual. Never market two-node heartbeat timeout alone as split-brain-safe auto failover.

### G6. ShowClock UI

Persistent top bar shows only concise state:

- `LOCKED`
- `ACQUIRING`
- `HOLD`
- `STALE`
- `FAULT`

Expanded diagnostics show source, offset, RTT, drift, jitter, generation, peer identity, last sample/action, project/media match, and takeover readiness. Do not turn the persistent shell into a diagnostics dashboard.

### G7. ShowClock acceptance targets

Proposed wired-LAN targets:

- clock offset p95 <= 1 ms;
- clock offset maximum <= 3 ms under the defined normal-load envelope;
- timestamped action execution delta p95 <= 2 ms;
- timestamped action execution delta maximum <= 5 ms;
- zero duplicate GO/Take under duplicate/reorder tests;
- backward OS clock step causes Hold, never show-time reversal;
- project/media hash mismatch blocks Arm;
- one-hour two-PC test produces zero dual physical output.

Fault tests:

- packet loss, duplication, reordering, burst delay;
- Primary process crash;
- Standby process crash/restart;
- network partition/rejoin;
- OS wall-clock step;
- external timecode loss/relock;
- project replacement during sync;
- media mismatch/missing asset;
- stale action after takeover;
- witness/shared-fence unavailable.

### G8. ShowClock tranche order

1. Protocol/authenticated clock sample and action schema.
2. Pure estimator/slew/Hold state machine and deterministic simulator tests.
3. Primary publisher/Standby receiver without output authority.
4. Action dedupe/scheduling.
5. Project/media/clock generation coupling.
6. Witness/shared fencing decision and implementation.
7. Takeover integration with machine-local output ownership.
8. UI status/detail.
9. two-process loopback tests.
10. two-machine hardware soak and evidence.

## 12. Critical path H: UI product roadmap

Follow `qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md` in order.

### H1. UI-0 reachability

- Import Media always reachable.
- Display Close/Sync/Open truthful and reachable.
- Scene/group creation uses one Group Picker contract.
- branch-specific reachability tests.

### H2. UI-1 shared shell

- only Setup/Edit/Control peer workspaces;
- ShowClock/save/output health/Blackout persistent;
- authoring controls leave global chrome;
- deep-link/session restore preserves selection/context.

### H3. UI-2 Setup

- Fixture Library -> Patch -> Stage -> Inspector visible workflow;
- same selection/inspector model for Video outputs and I/O;
- contextual diagnostics and closed advanced protocol settings;
- machine output ownership never rewrites authored configuration.

### H4. UI-3 Edit

- Edit has Lighting and Video domains;
- Lighting preserves accepted executable geometry;
- Video matches that geometry with Media/Slots, active/queued, canvas, layer/clip inspector;
- one Timeline and one Undo/selection model;
- shared Group Picker;
- no density-by-shrinking.

### H5. UI-4 Control

- Lighting/Video/Both emphasis;
- Current/Next/GO/Back/Release/master intensity;
- Preview/Program/Take/clip grid/output truth;
- compact read-only show progress;
- automation truth and manual override.

### H6. Viewport/accessibility completion

At 1920x1080, 1920x1032, 1366x768, and 1280x720:

- no application outer scroll;
- no clipping/overlap/unreachable primary action;
- existing typography/control/hit-target sizes preserved;
- keyboard route and visible focus;
- accessible icon names;
- focus return after dialogs/disclosure;
- reduced motion;
- paste/input compatibility;
- Japanese localization coverage.

## 13. Critical path I: comparative and production acceptance

Use the pinned tasks in `qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md` and the corresponding Daslight task set.

For every task record:

- exact product build/version/license;
- start project/hash and media manifest/hash;
- elapsed time;
- clicks/taps/keystrokes/drags/modal confirmations;
- wrong navigation and recovery;
- accidental live-output changes;
- output drops/audio discontinuity/DMX telemetry;
- screen recording with timestamps and input overlay;
- first failed run and retest, without deleting unfavorable evidence.

Syndocal wins only when it is measurably no slower/no more complex, or one additional confirmation has a documented safety/clarity advantage.

Priority Video tasks:

- V01 12 files -> 12 reusable assets, zero unrelated layers;
- V02 assign assets to slots without file dialog/live replacement;
- V04 queue and Take on next bar;
- V07 scoped Layer Bus transition without hiding overlay/logo;
- V09 deterministic BPM change during queued transition;
- V11 dual-HDMI routing truth;
- V13 missing/relink all references;
- V16 full-bank Import reachability at all viewports;
- V17 decoder/effect fault isolation.

## 14. Global final verification order

Only after all implementation tranches are committed and P0/P1 review-clean:

1. `cargo fmt --check` for all Rust packages.
2. Focused Protocol/Engine/Syndocal tests for every changed domain.
3. `cargo test --workspace --locked` if the repository's accepted release process permits the full workspace run.
4. `cargo check --workspace --locked`.
5. TypeScript.
6. all focused static harnesses.
7. localization.
8. supported viewport harnesses, unless a user instruction explicitly forbids a full viewport run for that tranche.
9. scoped and repository `git diff --check`.
10. adversarial read-only review on frozen files.
11. exact native release process:
    - resolve running process executable paths;
    - terminate only this checkout's exact `target/release/syndocal.exe`;
    - `pnpm --dir app tauri build --no-bundle`;
    - launch exact release executable;
    - verify exactly one responsive Syndocal window;
    - maximize before UI actions.
12. native UI acceptance at all supported viewports.
13. physical DMX, HDMI 1/2, audio input, MIDI/OSC, NDI/Spout as available.
14. one-hour maximum-condition soak on RTX 5090 / 13900KF / 128 GB.
15. two-PC ShowClock/output-ownership soak after that tranche exists.

No final completion claim may be based only on browser harnesses, TypeScript, Vite, or Rust unit tests.

## 15. Planned commit cadence

Commit after each frozen green unit and report percentages.

Recommended future commits:

1. `fix: make media asset commits terminal and idempotent`
   - backend authoritative commit helper/API/tests only.
2. `feat: use authoritative media asset commits`
   - frontend direct wiring/types/harness only.
3. `fix: preserve atomic legacy media IPC compatibility`
   - old IPC internals and exact compatibility tests.
4. `fix: close media file identity and lifecycle gaps`
   - cross-platform/availability/reaper/proof gaps.
5. `feat: add authored video clip slots`
   - Protocol/migration.
6. `feat: publish atomic clip slot operations`
   - Engine/runtime.
7. `feat: bridge clip slot project authority`
   - backend.
8. `feat: add video clip authoring and control surfaces`
   - frontend.
9. PATCH/GDTF, Stage, generic authority/recovery, input/output, ShowClock, and UI roadmap commits remain separate by domain.

After each commit report:

- hash and subject;
- exact files;
- exact passing tests/counts;
- warnings;
- what the commit does not prove;
- updated overall and per-domain percentages;
- next blocking boundary.

## 16. Resume protocol

At the next session or agent handoff:

1. Read this file completely.
2. Read `AGENTS.md` completely.
3. Run `git status --short` and `git log -3 --oneline`.
4. Confirm that commits `c90e5a2` and `ed3f473` are present.
5. Verify no other process/agent is editing the intended files.
6. Re-run the frozen Media Asset focused gates before editing if the checkout changed.
7. Re-anchor current `main.rs` because line numbers in review reports may have moved.
8. Start with A1 only: backend-owned authoritative Media Asset terminal transaction.
9. Assign exclusive `main.rs` ownership to one implementation agent. Keep the reviewer read-only until a stable checkpoint.
10. Prefer Claude Opus for difficult implementation when available, as requested by the user, but independently inspect and test its output. Opus output is not evidence by itself.
11. Do not start Clip Slot T2, ShowClock, UI redesign, or legacy IPC cleanup concurrently with A1 in the same files.
12. Stop and report if the intended staged file list contains unrelated user/shared changes that cannot be separated safely.

## 17. Known open risks at this pause

### P0

No confirmed P0 at the last frozen reviewed checkpoint. Current dirty `main.rs` has not received a final full GO.

### P1

- Media terminal reply-loss/history closure.
- Generic Begin reply-loss/liveness.
- Old raw compatibility Media IPC internal atomicity/authority/hash identity.
- Non-Windows file ABA/current-path identity if non-Windows is supported.
- Pending broader PATCH/GDTF atomic project/engine commit and raw mutation admission.
- Remaining generic project/input/output authority boundaries identified in prior reviews must be re-audited before release.
- Distributed ShowClock/split-brain-safe two-PC operation is not implemented.

### P2 / proof gaps

- availability first-hash cancellation;
- reaper spawn failure visibility;
- post-engine-ACK/pre-receipt insertion model;
- Bootstrap output/fade rollback test coverage;
- production-level all-six concurrent/reply-loss Media command tests;
- non-Windows tests;
- native Tauri release build and UI/hardware acceptance;
- comparative benchmark evidence.

## 18. Claims that must not be made yet

Do not claim:

- Media Asset T1 complete;
- reply-loss idempotency complete;
- native verification complete;
- Clip Slot model implemented;
- ShowClock distributed synchronization implemented;
- automatic two-node failover is split-brain safe;
- Daslight 5 or SynapseRack has been beaten without measured benchmark evidence;
- cross-platform file identity safety unless non-Windows is implemented and tested;
- hardware NDI/Spout/dual-HDMI/DMX correctness from SDK-free/unit tests alone.

The next truthful milestone is: **backend-owned terminal Media Asset commit, reviewed and committed as a focused checkpoint**.
