# Syndocal complete product roadmap and frozen checkpoint

> **Current execution notice (2026-08-19):** this file remains the detailed product
> requirement and final-gate authority, but its frozen checkpoint, 75.5% planning
> roll-up, and Media A1 resume instructions are historical. Use
> `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` for the current dependency order,
> `1.2.0-alpha.6` version train, warning ratchet, E4 resume point, and checkpoint
> workflow. Do not compare old and current percentages without identical denominators.

> **OutputControl supersession (2026-08-21):** physical six-digit/Raw Input/Enter
> consent and its challenge IPC are removed from the product. Normal Enable is one
> local v2 click for exact Both; dangerous advanced output mutations use a parented
> native Warning/Yes-No confirmation. Older physical-consent acceptance items are
> historical. Owner/fence/durable/S0 requirements remain in force.

> **Windows-only completion scope (2026-08-22):** the active product target is
> this operator's Windows PC. macOS/Linux control and native acceptance, and the
> distribution/legal/signing/SBOM/clean-machine/updater/publication tranche, are
> deferred and removed from the current completion denominator. Windows native,
> warning-zero, the final editor + LED panel + projector topology, ASIO, hardware, crash recovery, security, and soak
> requirements remain. Use 71 active items; E1 leaves progress at 14/71 (19.7%).

> **Windows verification workflow (2026-08-22):** routine validation drives the
> same registered Tauri/control-plane production commands through backend
> drivers. Do not repeat Computer Use during each implementation loop. Perform
> one maximized release-executable UI/hardware pass after the integrated bundle
> passes implementation, adversarial review, warning, build, and document gates.

Status: implementation resumed on 2026-08-13 (JST). Media Asset A1-A8 are accepted for the current Windows tranche, including the rebuilt Media Library thumbnail/hover supplement; the whole-product Q5 completion query still fails and Critical Path B is next.
Roadmap revision: v4, current Windows Media Asset A8 evidence integrated on 2026-08-14 (JST).

This document is the detailed continuation contract from the current checkout to a release candidate that can truthfully be compared with Daslight 5 and SynapseRack. It records what is committed, what is only present in the dirty worktree, what has been tested, what is still unsafe, the dependency order, file ownership, required evidence, commit boundaries, and the native completion gate.

It does **not** claim that Syndocal is complete. It does **not** replace:

- `qa/SYNDOCAL_UI_PRODUCT_VISION.md`
- `qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md`
- `qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md`
- `qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md`
- `qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md`
- `RELEASE_STATUS.md`
- `qa/DASLIGHT_PARITY_COMPLETION_PLAN.md`
- `qa/DASLIGHT_COMPARISON_VERDICT.md`
- `qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md`

Those files remain detailed evidence and domain specifications. This file is the master route from the frozen implementation checkpoint to the target. A requirement is not allowed to disappear merely because its detailed evidence lives in another file; every release-blocking requirement must also appear in the traceability ledger defined below.

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

The original frozen table above is retained as historical context. The resumed implementation has since produced these focused checkpoints:

| Commit | Meaning | Completion claim |
| --- | --- | --- |
| `aad9172 feat: complete authoritative media backend` | Backend-owned terminal receipt/history/Published transaction | A1 implementation checkpoint |
| `682b908 fix: fence authoritative preview staging` | Expected-authority CAS for automatic Preview staging | Media authority regression checkpoint |
| `14eeeb2 feat: wire authoritative media frontend` | Direct authoritative frontend commit/query path | A2 implementation checkpoint |
| `4173e35 fix: preserve atomic legacy media compatibility` | Old IPC names routed through authoritative hash/publication/history | A3 implementation checkpoint |
| `bb6aef7 test: prove complete media bootstrap rollback` | Complete Bootstrap output/fade/runtime rollback proof | First A6 proof checkpoint |
| `8cb0459 fix: bind cross-platform media file coherence` | Windows retained handle plus Unix stable snapshot/version/CAS | A4 implementation checkpoint; non-Windows execution proof still open |
| `68a983d feat: harden media availability lifecycle` | Reserved inspection, cancellation linearization, reaper/drop lifecycle | A5 implementation checkpoint |
| `bdb7008 test: prove authoritative media command idempotency` | Six real authoritative command concurrency/retry tests | Second A6 proof checkpoint |
| `dd91db8 test: add deterministic media hash pause` | Native first-hash cancellation/replacement QA seam | A8 testability checkpoint |
| `10fdf05 feat: complete media library UI and align Edit navigation` | Media Library, availability/relink UI, operation Cancel/progress | A8 UI-readiness checkpoint |
| `be9c456 fix: align Video with the shared Edit grid` | Lighting/Video navigation and shared Video grid correction | Focused UI checkpoint, not whole-product completion |
| `2f77981 fix: localize shared Video grid labels` | Eight missed shared-grid Japanese labels | A7 localization correction |

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

There is no longer one ambiguous “overall” percentage. Report these dimensions independently after every checkpoint. The release-readiness estimate is a planning roll-up weighted as software implementation 50%, automated proof 20%, current-source native/hardware proof 20%, and distribution/legal readiness 10%. A P0 release blocker overrides the numeric result.

| Area | Frozen estimate | Basis |
| --- | ---: | --- |
| Software implementation toward the planned product target | 92% | Media T1, authored/runtime Clip Slots, scoped Video FX C1, Timeline Guide/loop/link/group editing through authoritative Split and lane-valid reorder, and their authority paths are implemented; Follow/transition buses, derived data, recording closure, distributed ShowClock, AI Control Plane, and final cross-domain boundaries remain |
| Product-surface coverage | 90% | Media Library, shared Edit/Control Clip Bank, scoped FX, Timeline phases/Guide/loop and linked editing through lane-valid reorder exist; Follow/transition-duration, AI administration, distributed operation, and several recording/recovery surfaces remain incomplete |
| Automated proof coverage | 90% | Media, Clip Slot, scoped FX, Timeline linked Split/lane editing, real-browser pointer/fresh-ID/full-bank mounting, backend authority, and full-crate gates have substantial coverage; unified AI registry, migration/security/performance, and external/hardware seams remain |
| Current-source native and hardware proof | 40% | Media/Clip/FX/Timeline Windows release builds and focused native workflows are accepted; most physical DMX/control/audio/video/venue, two-machine, clean-machine, non-Windows, and AI external-client gates remain open |
| Distribution/legal/clean-machine readiness | 35% | Packaging/CI foundations exist; signing, notarization, license disposition, release artifact publication, and clean-machine acceptance remain |
| Planning roll-up to public release candidate | 75.5% | `92*0.5 + 90*0.2 + 40*0.2 + 35*0.1`; unresolved P0-Release/P1 gates override the number |
| Media Asset T1 overall | 100% for the declared Windows tranche | A1-A8 are implemented/reviewed; A7 and the native A8 matrix plus targeted rebuilt thumbnail/hover supplement are green |
| Protocol Media Asset schema/migration/validation | 100% for T1 scope | Focused Protocol tests passed before commit |
| Engine Media Asset runtime/allocator/Published rollback | 100% for A6 scope | Complete Bootstrap A restoration and allocator proof pass in A7 |
| Frontend Media Asset orchestration | 100% for A2 scope | Direct authoritative Import/Add/Bootstrap/Relink with exact recovery/cancellation is implemented and the focused gate passes |
| Backend Media Asset prepare/finalize/registry | 100% for A1-A5 implementation scope | Authoritative terminal transaction, compatibility bridge, coherence, availability, and reaper are implemented |
| Media Asset adversarial/final review | Accepted for current Windows tranche | Final thumbnail/session review returned P0=0/P1=0; remaining notes are proof/resource P2 only |
| Native release/UI/hardware acceptance for this tranche | Accepted for current Windows tranche | Fifteen workflow rows plus rebuilt 3-column grid, explicit thumbnail authorization, moving single preview, Still fail-safe, and no-touch evidence are recorded |
| Clip Slot T2 | 100% for the declared Windows B1-B4 tranche | Protocol, authored/runtime Engine split, backend authority, shared Edit/Control UI, focused/browser/native proof, and independent review are accepted; C2 transition-duration execution remains separate |
| Scoped Video FX C1 | 100% for the declared CPU/authority/UI tranche | Stable scope schema, Engine lifecycle/rollback, CPU renderer, backend authority, UI, transport/preview integration, and focused/native proof are accepted; Clip Take and transition buses remain C2/C3 |
| Timeline L-TL | 83% for the requested editing/runtime surface | Phase/Guide and musical loop foundation, linked A/V import/grouping, selection, delete/duplicate/nudge/quantize/copy-paste/ripple/trim/direct resize/Split, and five-domain lane-valid reorder are implemented; Follow/crossfade and full Undo/Redo/save/reload focus closure remain |
| AI Control Plane | 10% requirements/foundation | Full registry/MCP/API contract is accepted and authoritative/Remote building blocks exist; AI0-AI8 implementation and external-client acceptance have not begun |
| Distributed ShowClock/2PC | architecture requirements drafted, implementation 0% | Local clock and machine-local output fence exist; authenticated distributed authority does not |

After every checkpoint report at least: software implementation, automated proof, native/hardware proof, distribution/legal readiness, affected domain percentage, and the next blocking gate. Do not report a percentage without its denominator and evidence boundary.

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

### 4.6 Supported-product boundary

- Record the supported OS, architecture, GPU/API, media codec, audio backend, lighting interface, remote-client, and package matrix before calling a release candidate complete.
- A capability that is intentionally unsupported is listed as `Out of scope` with a reason and operator-facing fallback; it is not silently omitted.
- Windows-only file-identity guarantees may not be generalized to macOS/Linux. Either implement and test an equivalent proof or narrow the supported platform claim.
- Built-in 3D and standalone-hardware programming remain explicit product decisions: the accepted current direction is external Art-Net visualization and no standalone-hardware claim unless separately approved.
- Commercial comparison uses pinned product versions, license tiers, fixtures/media, hardware, and task definitions.

### 4.7 Definition of done and severity

`Done` for one tranche means all of the following:

1. requirement and non-goals are written;
2. schema/API is backward compatible or has a tested migration;
3. all fallible work precedes irreversible publication, or a durable terminal recovery protocol exists;
4. focused automated gates pass with nonzero selected tests;
5. adversarial review on frozen files reports no open P0/P1 in that tranche;
6. a scoped commit exists with exact files, hash, evidence, warnings, and non-claims;
7. any native/runtime-affecting tranche has a current-source native release smoke before the next dependent tranche;
8. hardware or external acceptance is recorded separately when required.

Severity policy:

- `P0-Code`: corrupts data, violates physical-output safety, prevents launch/core workflow, or creates an uncontrolled security boundary.
- `P0-Release`: mandatory signing/legal/clean-machine/hardware/venue gate that is not yet satisfied. It blocks public release even when no P0-Code is known.
- `P1`: realistic data loss, split-brain, false success/failure, authority bypass, indefinite liveness loss, or major operator workflow failure.
- `P2`: bounded degradation, proof gap, maintainability risk, or noncritical UX defect. A P2 blocks release when it affects security, migration, legal compliance, supported-platform correctness, recovery, or an advertised comparison claim.

### 4.8 Global dependency order

The authoritative order is:

1. finish Media Asset terminal authority and native smoke;
2. close generic Begin reply-loss or give each dependent mutation a backend-owned authoritative operation;
3. close PATCH/Repair/Stage mutation integrity;
4. close project replacement/Takeover output fencing, including Audio/MIDI/recording ownership decisions;
5. implement media-derived cache/thumbnail/proxy foundations;
6. implement Clip Slots and Video transitions;
7. close Audio/recording/live-source product paths;
8. freeze ShowClock platform, transport, authentication, and fencing decisions, then implement distributed synchronization;
9. finish shared UI and accessibility;
10. run comparative, migration, security, hardware, performance, clean-machine, and distribution acceptance.

Independent pure/read-only work may proceed earlier, but no dependent mutation may ship on an unresolved authority or ownership boundary.

## 5. Critical path A: finish Media Asset T1

Media Asset T1 blocked Clip Slot T2 and the final Video operator comparison. A1 through A8 are now accepted for the declared Windows tranche, so Critical Path B may begin from this checkpoint.

### A1. Backend-owned authoritative Media Asset terminal transaction

Priority: highest. Status: implemented and committed in `aad9172`; the ordered A7 proof, final frozen reviews, and Windows A8 acceptance pass.

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

Status: implemented and committed in `682b908` and `14eeeb2`. The direct authoritative command/query path passes the A7 frontend gate.

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

Status: implemented and committed in `4173e35`. Compatibility names/shapes are retained, production tests cover authoritative owner, atomicity, hash identity, and history, and the declared Windows A8 tranche is accepted.

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

Status: implementation committed in `8cb0459`. Windows retained handles and Unix stable private snapshots/version/CAS are present. Windows focused tests pass; execution on macOS/Linux and the supported-platform release decision remain open.

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

Status: implemented and committed in `68a983d`; the current A7 operation and availability filters pass, and native first-hash cancellation/project-replacement rows pass in A8.

- Add reserved operation identity to verified availability when the inspection can be long.
- Allow cancellation before first read and between chunks/probe entries.
- Do not change project revision/history during inspection.
- Treat reaper thread spawn failure as setup failure or a visible degraded state; do not silently `.ok()` and leave retained file locks indefinitely.
- Test TTL cleanup without another registry call.
- Test app shutdown joins/unparks the one reaper thread promptly.

### A6. Fill Engine proof gaps

Status: implemented and committed in `bb6aef7` and `bdb7008`; the current A7 Engine and authoritative filters pass.

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

Status: PASS on 2026-08-13 for the current resumed checkpoint. Exact commands, nonzero counts, warnings, and non-claims are recorded in `qa/MEDIA_ASSET_T1_A7_EVIDENCE_2026-08-13.md`. The gate found eight missing Japanese labels; they were fixed in `2f77981`, after which localization was 3174/3174.

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

Status: PASS for the current Windows tranche. All fifteen workflow rows remain recorded against executable SHA-256 `41FE507275C21916F36894A4D4E5DF4D9CEF537E5223A778302B83CCE93FAE07`. The subsequent Media Library thumbnail-grid/backend-preview and authority-retry change was rebuilt as executable SHA-256 `27D1CD18A810AD960B771B800B32E4C7353539C6E6DEF5E2B567387983CE36AD` and passed targeted native grid/hover/focus/Still/no-touch acceptance, including automatic restart and 13/13 publication after a genuine same-project authority change during the initial batch. Exact evidence and claim boundaries are in `qa/MEDIA_ASSET_T1_A8_NATIVE_EVIDENCE_2026-08-13.md`.

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

Status (2026-08-14): implemented and accepted. Stable authored/runtime bus IDs, explicit
layer/group membership, all nine transition kinds, reversible active identity, deterministic
conflict rejection, scoped LayerBus FX, authoritative retry/read recovery, renderer/NDI/Spout/
Program/recording integration, Edit+Control UI, 5-viewport browser proof, full Rust suites,
release build, and maximized native Control > Video mount are green. Layer duplication now
updates bus topology atomically and publication failure restores the exact pre-command image.

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

Dependency rule: D1 machine/session read purity may proceed independently. D2-D4 must wait for E1 generic transaction recovery, or each command must use an equivalent backend-owned authoritative terminal operation with its own reply-loss receipt. Frontend `Begin` wrapping alone is not an acceptable server-side authority boundary.

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

### E1. Generic Begin reply-loss and transaction lifetime — accepted on Windows

Accepted boundary:

- Begin-side effects survive a dropped reply through an idempotent receipt/query/adopt path.
- exact owner/window incarnation, terminal recovery, renderer retirement, and stale delayed Commit/Cancel are enforced backend-side.

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

### G8. Decisions required before implementation

Freeze these decisions in Q2 before writing the network protocol:

- canonical show-time unit, range, precision, and long-duration/wrap behavior;
- mapping between monotonic clocks and display/wall time;
- master-clock policy for free-run, audio playback/device, MTC/LTC, MIDI Clock, Ableton Link, OSC, MSC, ArtTimeCode, and ArtSync;
- concrete slew limit, step/Hold threshold, lock-acquisition window, loss timeout, holdover duration, and resume policy;
- transport, discovery, ports, multicast/unicast policy, firewall behavior, and protocol versioning;
- authentication, key provisioning, rotation, revocation, replay window, and downgrade rejection;
- witness/shared strongly consistent fence/physical interlock choice, or an explicit manual-Hold-only two-node product boundary;
- rolling upgrade and mixed-version refusal policy;
- action scheduling horizon, per-action late tolerance, dedupe lifetime, and queue limit;
- supported network/load envelope and whether Wi-Fi is diagnostic-only or supported.

### G9. ShowClock tranche order

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

Native accessibility acceptance additionally includes:

- NVDA or another supported Windows screen reader through the primary Setup/Edit/Control workflows;
- Windows High Contrast and color-independent state/error indication;
- OS display scaling and text scaling at 125%, 150%, and 200% without clipped critical controls;
- keyboard-only Import, Patch, GO/Back/Release, Take, Blackout, Save, recovery, and output Arm paths;
- IME composition and long Japanese/English strings without premature submit or lost text;
- focus ownership across native file dialogs, popouts, disclosures, and error recovery;
- reduced-motion behavior that preserves truthful Take/transition state rather than hiding progress.

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

Lighting/Daslight comparison must also retain:

- every task in the accepted Daslight operation-count set, with unmeasured tasks remaining explicitly `Unmeasured`;
- synchronized same-show output captures from Syndocal and the pinned Daslight build where an equivalence claim is made;
- DVC exact/imported/fail-closed coverage by effect family and source evidence;
- GDTF/OFL semantic coverage, including capability ranges, wheel slots, geometry, matrix/pixel semantics, and calibrated emitters rather than catalog count alone;
- physical controller/feedback, real LAN node/fixture, external visualizer, and venue rehearsal evidence;
- the explicit external-Art-Net visualizer boundary for 3D;
- the explicit no-standalone-hardware-programming boundary unless a separate hardware product is approved.

Never convert a loopback packet capture, fixture count, internal schema, or three measured tasks into a claim of whole-product parity.

## 14. Critical path J: release, distribution, legal, and clean-machine delivery

This is a release-blocking path, not post-release paperwork. Open items are `P0-Release` until evidence is attached.

### J1. Supported artifact matrix

Freeze and publish one table containing:

- Windows x86_64: portable/release executable, NSIS, MSI, file association, updater channel;
- macOS supported architectures: signed app/DMG, hardened runtime, entitlements, notarization, Gatekeeper launch;
- Linux supported architectures/distributions: AppImage/deb and required system libraries;
- normal MIT/WASAPI build versus any separately licensed ASIO build;
- optional NDI/Spout/Syphon/features and what happens when their runtime/SDK is absent;
- exact version, commit, build profile, feature set, and SHA-256 for every artifact.

Unsupported combinations must fail as an unavailable capability, not prevent the whole application from starting.

### J2. Signing and notarization

- Windows Authenticode-sign executable, installer, and updater payload; validate signature and timestamp on a clean machine.
- Sign macOS app and DMG with Apple Developer ID, enable the required hardened-runtime entitlements, notarize, staple, and verify with Gatekeeper.
- Define key/certificate custody, rotation, expiry, CI secret access, and emergency revocation.
- Prevent unsigned or wrong-channel updater payloads from being treated as trusted releases.

### J3. Third-party licensing and notices

Produce a release-component bill of materials and legal decision for:

- FFmpeg/libav dynamic linking, LGPL obligations, license text, notices, and corresponding-source access;
- NDI SDK/runtime redistribution;
- Spout/Syphon dependencies;
- ISF shader licenses and bundled examples;
- GDTF/OFL/catalog data licensing and attribution;
- fonts, icons, sample projects, images, audio, and video;
- ASIO: keep normal distribution WASAPI-only unless the GPL/proprietary agreement and separate artifact workflow are approved;
- Rust/npm transitive licenses and prohibited-license scan.

No artifact is published until the manifest, notice bundle, and actual packaged files agree.

### J4. Clean install, upgrade, repair, and removal

For each supported package:

- download from the actual release endpoint;
- verify hash/signature;
- install as a standard user where supported;
- first launch, project open, save, media import, and output-disabled startup;
- file association/deep-link behavior;
- upgrade from the previous supported release while retaining projects, backups, settings, and recovery state;
- failed/interrupted upgrade rollback;
- repair/reinstall;
- uninstall without deleting user projects; clearly document retained app data;
- launch after reboot and without developer SDKs, PATH entries, or source checkout.

### J5. Release publication and rollback

- Freeze version/tag/commit and release notes.
- Publish SHA-256 and signed update manifest for stable/beta/nightly channels.
- Test wrong-channel, downgrade, expired signature, corrupted download, partial download, offline, and endpoint-unavailable behavior.
- Define rollback artifact, database/project compatibility, and emergency channel disable.
- Archive build logs, dependency lockfiles, SBOM, licenses, test reports, and native/hardware evidence under one release evidence ID.

Acceptance: one operator can start from a clean machine, verify the artifact, install, launch, complete the declared smoke workflow, update, rollback where supported, and uninstall, with no checkout or development tool present.

## 15. Critical path K: physical hardware and protocol acceptance matrix

Loopback, mocks, SDK-free tests, and browser UI are prerequisites only. The matrix records device identity, firmware/driver, topology, cable/switch, duration, operator, date, commit/artifact hash, measurement source, and raw evidence path.

### K1. Lighting output

- Art-Net node plus real fixtures: address, RGB/color wheel, Pan/Tilt direction/range, intensity, strobe where supported, Blackout, reconnect, 44 Hz continuity, and multi-universe routing.
- sACN multicast node across the supported network-switch topology: universe routing, priority, join/leave, reconnect, and sustained output.
- Enttec USB PRO and DMXKing: discovery, serial identity, reconnect, device removal, queue/backpressure, and long run.
- Enttec Open DMX: logic-analyzer proof of Break 176 microseconds, MAB 16 microseconds, frame period, and failure behavior; recommend PRO-class hardware for critical venues.
- RDM/TOD: discovery, response correlation, timeout, cancellation boundary, ownership fencing, and device removal.
- Blackout and Standby must dominate all paths, including direct test, stale worker, reconnect, and project replacement.

### K2. Control input and feedback

- physical MIDI Note/CC/Clock/MTC input;
- MIDI feedback and All Notes Off on the declared controllers;
- wired `rekordbox-DJ-Link-ForPCDJ` Agent integration: Master Track Active drives project-owned
  Track-to-Timeline mappings, absolute Loop State converges the authored Timeline loop, and Release is
  idempotent. Pedal/global-hotkey and all rekordbox MIDI/filter/stop/reset behavior remain on the DJ PC;
- OSC over wired and Wi-Fi paths;
- TouchOSC and iPad/Android Web Remote round trip;
- Learn, conflict, disconnect/reconnect, stale reply, and project replacement;
- measure input-to-engine, engine-to-DMX/pixel, and feedback round-trip p50/p95/p99/max.

### K3. Video and displays

- HDMI/Display 1 and 2 mapping, fullscreen, unplug/replug, monitor reorder, DPI, refresh-rate change, and GPU reset behavior;
- NDI discovery/send/receive, color/alpha, fractional frame rate, resize, disconnect/reconnect, and one-hour stability;
- Spout/Syphon send/receive and sender restart;
- physical camera format negotiation and unplug/replug;
- screen capture display/window loss and permission denial;
- Preview/Program/Take truth during every route fault;
- project replacement retires removed endpoints before a new snapshot becomes active.

### K4. Audio hardware

- WASAPI system default and explicit stable device ID;
- at least two representative device families where available;
- ASIO only for the separately approved artifact;
- sample-rate/channel/sample-format/fixed-buffer negotiation;
- unplug/replug, exclusive-device conflict, callback gap, XRUN, worker panic, and safe-zero recovery;
- capture-to-analysis, capture-to-engine, and capture-to-pixel latency percentiles;
- one-hour WASAPI and approved-ASIO soak with no silent fallback.

### K5. Venue and two-machine rehearsal

- real LAN switch, realistic cable lengths, multicast policy, firewall profile, sleep/power policy, and clock-source topology;
- Primary crash, Standby restart, network partition/rejoin, witness/fence loss, output device removal, and explicit Arm after recovery;
- zero simultaneous physical output during the complete rehearsal;
- operator runbook for startup, sound/light/video check, failure, takeover, recovery, and shutdown.

## 16. Critical path L: Audio, recording, and live capture product completion

Audio is not merely an analysis input. The product target includes authored audio, monitoring, synchronized playback, recording, and failure truth.

### L1. Authored Audio model

- stable audio asset/clip IDs and reference integrity;
- source, duration, trim, gain, mute, routing, fade, waveform/analysis reference, and missing state;
- additive migration and strict validation;
- authored state separated from runtime playhead, device, level meter, and decoder state;
- save/recovery/history/Undo/Redo and project replacement coverage.

### L2. Audio runtime and ShowClock relationship

- declare whether the ShowClock, audio device clock, or decoded-media PTS is master for each mode;
- define resampling/slew, seek, loop, preroll, late-start, underrun, and discontinuity behavior;
- bounded A/V drift and no backward show-time step;
- audio-device replacement and sample-rate change enter a truthful Hold/Fault state;
- safe-zero affects reactive modulation without overwriting authored values.

### L3. Live sources

- camera, screen capture, NDI, Spout/Syphon, and generators have stable authored source identity plus machine-local availability;
- permission denial, unplug, format change, sender disappearance, and reconnect are explicit states;
- a source fault cannot mutate the authored project or replace another source through a stale reply;
- project replacement retires old capture workers before new workers publish frames.

### L4. Recording transaction

Recording requires an explicit state machine such as `Idle -> Preparing -> Recording -> Finalizing -> Complete | Fault`.

- target reservation and free-space estimate before start;
- one canonical A/V timeline and timestamp policy;
- temporary file plus atomic final publication where the format permits;
- explicit behavior for disk full, permission loss, encoder failure, app crash, output loss, and Stop timeout;
- no UI success until the playable artifact is finalized and verified;
- recovery inventory for interrupted temporary recordings;
- recording ownership and two-PC policy are explicit: local recording must not accidentally imply permission to emit external output;
- successful recording appears as a reusable asset only through an authoritative import transaction.

### L5. Audio/recording acceptance

- V15 two-output program recording with audio produces a playable artifact;
- seek/loop/BPM change/Take remain synchronized under the defined envelope;
- one-hour maximum-condition recording with Preview/Program, NDI/Spout, effects, and audio input;
- inspect duration, frame count, audio samples, drift, corruption, and finalization time;
- crash/fault tests preserve the original project and produce either a recoverable partial artifact or an explicit nonrecoverable report.

## 16A. Critical path L-TL: Timeline Phases, musical loop, linked media, and follow transitions

This is an explicit completion tranche, not optional polish. It extends the one shared
Lighting/Video/Audio/Automation Timeline without creating a second transport or a VJ-only path.

### L-TL1. Phase model and Guide voice

Status (2026-08-15): authored Phase/runtime cue generation and the native offline Guide monitor
path are implemented. The engine publishes Phase/`Looping`/`Break`/`Trans` cues under the same
Timeline transport snapshot; native playback uses embedded deterministic English WAV assets on
an independent session-local gain/device bus, cancels stale generations, and reports unsupported
custom labels as text-only faults. Browser fixtures retain Web Speech only outside Tauri to prevent
double announcements. Protocol/engine/backend decoding tests, localization, production build, and
the real-browser 1920/1366/1280/860 containment proof are green. The fixed English vocabulary is the
accepted show scope; physical multi-device routing acceptance remains part of the final L-TL7 boundary.

- Add stable authored `TimelinePhaseId` and ordered, non-overlapping Phase ranges on a Timeline.
- A Phase has a canonical label and a typed role. Built-in roles include `Intro`, `Verse`,
  `PreChorus`, `Chorus`, `Bridge`, `Breakdown`, `Outro`, and `Custom`; Custom retains its authored label.
- Phase boundaries use the same ShowClock/time domain as Timeline events. Moving or resizing a Phase is
  one authoritative history mutation and cannot silently move its child events.
- When Guide is enabled, entering a Phase speaks its operator label exactly once per forward boundary
  crossing. Seeking into a Phase speaks it once after the seek settles; pausing/resuming inside the same
  Phase does not repeat it.
- Guide speech is a dedicated monitor/cue bus, never Program audio and never recorded unless that cue bus
  is explicitly routed into the recording. Click and Guide have independent enable, gain, device/routing,
  and fault truth.
- Built-in English guide vocabulary is recorded and deterministic. Custom labels fail visibly to text-only
  guidance; they do not require TTS, an additional language pack, or a live network service.
- Project replacement, timeline switch, seek, loop wrap, follow transition, and clock discontinuity reset
  the guide de-duplication fence explicitly; stale speech from the previous generation is cancelled.

### L-TL2. Musical A-B loop

- Add authored/runtime loop truth with stable owner Timeline ID, A and B positions, enabled/armed state,
  and a musical length expressed against the ShowClock grid. A must be before B and the interval must
  resolve to at least one engine tick.
- The loop may be armed while the playhead is outside `[A,B)`. It begins only when normal playback first
  enters the range; arming must not seek or jump the playhead into it.
- At B, playback wraps sample/tick-exactly to A without double-triggering lighting events, video launches,
  audio samples, Phase announcements, or Timeline automation at either boundary.
- `1/2` halves the musical loop length and `x2` doubles it, anchored to the active loop start and quantized
  to the current beat/bar grid. Both are available before entry and during looping. Limits, rounding, and
  the maximum project/timeline boundary are fail-closed and visible.
- Disable/Break is available at any time. Disabling does not seek: playback continues linearly from the
  current position. When Guide is enabled, every actual loop wrap speaks `Looping`; disabling an armed or
  active loop speaks `Break` once. Merely editing A/B while disabled does not speak.
- BPM changes do not rewrite authored A/B. The runtime resolves the next wrap against the current
  ShowClock generation, with discontinuity Hold semantics and no retroactive boundary firing.
- Commands exist for Set A, Set B, Enable/Disable, Clear, `1/2`, and `x2`; all have configurable shortcuts
  and use the shared command/mapping system so keyboard, MIDI, OSC, DMX, Touch, and Remote can target the
  same typed action. Default keyboard bindings must not shadow text input or existing safety controls.
- The required DJ Link integration reuses this authored Loop and Disable/Resume path, but receives an
  absolute `DJ_LOOP_STATE` rather than a relative Half command. `rekordbox-DJ-Link-ForPCDJ` is the existing
  DJ Agent; Syndocal must not create another bridge or send rekordbox MIDI. Project mapping and wired
  physical acceptance are normative in `qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`.

### L-TL3. Independent channel lanes and Media Library placement

- The Timeline continues to render independent Lighting, Video, Audio, and Automation lanes under one
  playhead. Lighting accepts existing Scene/Scene Block placement unchanged.
- A visual MediaAsset can be dragged/click-inserted from Media Library onto a Video lane. An audio asset
  can be placed onto an Audio lane. Placement creates stable clip IDs that reference the catalog asset;
  it does not copy a pathname into the Timeline or route through Clip Slot/VJ Take runtime.
- A video asset with an audio stream creates one Video clip and one Audio clip at the same start, with
  matched trim/duration, in one atomic authoritative mutation. Stream probing/preparation must finish
  before publication; failure leaves both lanes and history unchanged.
- Missing/relinked media, proxies, waveform/thumbnail data, decoder readiness, and runtime playheads are
  machine/runtime truth and cannot overwrite the authored clips.

### L-TL4. Linked groups and selection/edit semantics

Status (2026-08-15): in progress. Stable cross-domain group membership, automatic linked A/V
placement, linked selection, group/ungroup, relative group move, grid nudge, grid quantize, ripple move, and
reference-scoped copy/paste are implemented. Delete now
expands the captured authored group closure on the backend and removes Lighting events, Video clips,
Audio clips, and both Automation domains in one authoritative publication/Undo; stale references
fail before mutation and exact request retry returns the stored terminal result. Nudge earlier/later is
available from the item context menu and shifts the complete captured group by one current grid step;
the backend preflights every Lighting, Video, Audio, and Automation member, updates both millisecond and
musical beat positions, preserves the selected logical group, and provides one history entry with exact
terminal recovery. Quantize uses the earliest temporal point in the captured selection/group as its
anchor, rounds it to the nearest current grid boundary, and moves every member by the same delta so
cross-domain relative timing and musical beat positions remain intact; zero/invalid grids reject before
mutation. Copy stores the current logical selection in a session-local clipboard scoped to the project
epoch and Timeline. Paste captures those references from the current authored A, expands their complete
group closure, shifts the earliest temporal anchor to the snapped playhead (including an earlier target),
allocates fresh item/group IDs, rewires internal jump targets, and returns the fresh logical selection in
one exactly recoverable history mutation. Replacement or Timeline change clears the clipboard; stale or
deleted source references reject before mutation. Ripple move uses the earliest point in the selected
logical group as its boundary, shifts every authored Lighting, Video, Audio, and Automation item at or
after that boundary, expands any intersected group to its complete membership, and preserves the original
logical selection; negative underflow rejects before mutation. Grouped Trim is now authoritative across all
five item domains: one primary absolute edge becomes one signed group edge delta, media source offsets and
fades remain coherent, Scene source time/tempo/whole-loop constraints fail closed, and Step/Linear automation
  is evaluated at the new boundary while unsupported Bezier cuts reject before mutation. The item menu exposes
  both playhead trim directions with the same exact receipt/history path. Direct edge gestures for linked Scene,
  Audio, Video, Lighting Automation, and Video Automation items now enter that same authoritative Trim path;
  unlinked items retain their existing resize behavior, `Alt` isolates one member for the captured gesture, and
  the returned logical selection is restored. A real Chromium fixture performs pointer resize on both linked and
  unlinked representatives across all five domains and verifies both edges plus `Alt` isolation. Split is now one
  authoritative mutation across Scene, Audio, Video, Lighting Automation, and Video Automation. The primary
  playhead cut becomes one relative offset for every grouped member; left items retain their IDs, right items get
  fresh IDs, non-isolated groups are recreated on the right, `Alt` leaves only the new right member ungrouped, and
  internal Scene jump references are rewired. Scene/Audio/Video fade envelopes do not invent a fade at the cut;
  cuts through an existing fade ramp fail closed. ACK results mount the returned bank before best-effort refresh,
  so fresh right-side selection and focus survive refresh failure. Rust authority/persistence/retry tests, a real
  Chromium fresh-ID mount/focus gate, independent adversarial review, and an Opus read-only audit are green.
  Lane-valid reorder is now authoritative across Scene, Audio, Video, Lighting Automation, and Video Automation:
  one transaction applies horizontal time delta plus an exact per-item destination map, expands the full group
  closure unless `Alt` isolates the primary, preflights source and destination locks, and rejects incompatible or
  cross-Timeline targets before mutation. Scene may cross Lighting and Video lanes while synchronizing its track;
  Audio remains invalid, while clips and Automation remain strict to their own lane kinds. Legacy implicit
  Lighting/Video lanes and Automation ownership are materialized only in the candidate or pinned during explicit
  layer mutations, preserving fail-closed rollback. The ACK mounts the full returned bank before best-effort
  refresh, restores all five selection domains and primary focus, and real CDP gestures prove nonzero time movement,
  cross-kind Scene movement, `Alt` isolation, keyboard/context routing, and mounted post-ACK times. Protocol,
  Engine, backend, TypeScript, localization, browser, viewport, Vite, native release build, independent review, and
  Opus audit are green. Remaining grouped edit work is full selection/focus restoration across Undo/Redo and
  save/reload. Normal horizontal Audio/Video clip
drag now routes a linked member through the authoritative group nudge path, while holding `Alt` selects
and edits only that member for the current gesture. Closing the item action menu restores focus to its
invoking clip; the real-browser gate proves both the modifier selection and focus return.
Duplicate is now authoritative across all five item domains: it allocates fresh item/group IDs,
preserves group-relative offsets and musical beat placement, rewires duplicated internal jump targets,
returns the fresh logical selection, and is exactly recoverable without a second allocation/history entry.

- Add stable `TimelineItemGroupId` and an authored group table. A group may contain two or more compatible
  Timeline items across Lighting, Video, Audio, and Automation lanes; membership is explicit, unique, and
  validated against the same Timeline.
- Auto-split A/V placement creates one group containing its Video and Audio clips.
- Selecting any group member selects the complete group by default. Move, ripple move, duplicate, delete,
  trim, split, nudge, quantize, copy/paste, and lane-valid reorder operate atomically on all members while
  preserving relative offsets. A modifier can temporarily select/edit one member without silently
  dissolving the group.
- Multiple selected items can be grouped from the context menu and keyboard command; grouped items can be
  ungrouped the same way. Invalid cross-Timeline membership and incompatible target lanes reject without
  partial movement or history entry.
- Group/ungroup and every grouped edit are one Undo/Redo mutation. Focus/selection returns to the same
  logical items after success, failure, Undo, Redo, save/reload, and project replacement.

### L-TL5. Timeline Follow and crossfade

- Add an authored ordered Timeline bank. `Follow` optionally starts the next Timeline (the item directly
  below the current one in canonical bank order) when the current Timeline reaches its natural end.
- Follow does not fire on Stop, manual seek to end, project replacement, loop wrap, fault, or an explicitly
  aborted transition. The next Timeline is captured by stable ID when the transition is admitted; reorder
  races cannot redirect an in-flight Follow.
- A Follow transition has typed duration (milliseconds/beats/bars), audio fade curve, video transition,
  lighting fade policy, BPM destination, and optional preroll. Cut uses zero duration; all other modes use
  nonzero bounded timing resolved once at transition admission.
- During the transition, outgoing and incoming Timelines run concurrently under one authoritative
  transition generation. Audio crossfades without double monitoring, video uses the canonical output
  transition path, and lighting merges/fades by the authored policy without destroying either Timeline's
  Scene state.
- Click BPM slews monotonically from outgoing to incoming BPM across the exact transition duration. Beat
  phase remains continuous; it does not reset at admission or completion.
- When Guide is enabled, `Trans` is spoken on a configurable musical cadence during the transition.
  For the pinned two-song show chart, it is exactly every two measures at targets 149, 151, 153,
  and 155. After and only after successful Follow settlement, the fixed one-word cue `Complete` is
  spoken exactly once. At the same settlement boundary, any destination Phase/Intro cue is suppressed;
  `Complete` is the only Guide cue there, preventing a collision. Abort, fault, stale generation, or
  failed settlement must never speak `Complete`. `Trans` and `Complete` cues use the same monitor-only
  guide bus and generation cancellation rules as Phase/Loop cues.
- Missing media, unavailable audio device, late decoder, clock discontinuity, or next-Timeline validation
  failure follows an explicit Hold/Cut/Fault policy selected before publication. It must never leave both
  Timelines emitting indefinitely or advance history twice.

### L-TL6. UI and interaction contract

- Phase blocks and names are visible on a dedicated Phase ruler above the channel lanes; they do not
  consume a Lighting/Video/Audio lane.
- Click, Guide, loop A/B, `1/2`, `x2`, Break, Follow, and transition status remain reachable at all required
  viewports using reflow/disclosure/internal scrolling. Existing typography, 44 px targets, and Timeline
  editing density are not globally reduced.
- Video/Audio asset placement has exact lane drop targets plus keyboard/click fallback. Context menus expose
  Group/Ungroup only when the selection is eligible, and announce the result accessibly.
- Runtime badges distinguish `Loop armed`, `Looping`, `Follow pending`, `Transitioning`, `Held`, and `Fault`;
  UI must not infer them from authored fields or playhead position.

### L-TL7. Required proof before completion

- protocol migration/roundtrip and malformed Phase/loop/group/follow rejection;
- engine unit tests for boundary de-duplication, outside-range arming, wrap, `1/2`, `x2`, Break, seek,
  BPM/discontinuity, Phase/Guide generation fences, and one-history rollback;
- atomic A/V split placement and group move/trim/delete/Undo/Redo/save/reload tests;
- follow bank ordering, captured-ID race, Cut and timed crossfade, BPM slew, `Trans` cadence, fault/abort,
  and exactly-once next-Timeline start tests;
- audio sample and video-frame continuity measurements plus lighting output assertions at transition
  boundaries; no event/sample double fire;
- browser and native UI tests for drag/click placement, multi-select context Group/Ungroup, shortcuts,
  Phase ruler, loop controls/status, Guide speech routing, and required viewport containment;
- release native build and maximized-window QA using representative Lighting+Video+Audio Timelines,
  including an A/V file that auto-splits and follows into the next Timeline.

This tranche is complete only when authored state, runtime truth, authoritative history/terminal recovery,
renderer/audio execution, shared mappings, UI, accessibility/localization, and native evidence are all green.

## 17. Critical path M: media-derived data, cache, and performance envelope

Project-authored MediaAsset identity remains separate from machine-local derived data.

### M1. Derived-data model

- thumbnail/contact sheet;
- codec/container/stream detail;
- waveform;
- proxy/transcode variants;
- BPM, beat markers, onset, energy, spectral/color/section features where product-visible;
- decoder capability/health and last verified source fingerprint.

Derived records are keyed by content identity plus algorithm/version/settings, never only by pathname. They do not dirty `.sdc` unless the user explicitly authors derived markers or overrides.

### M2. Background work and cancellation

- hash/probe/thumbnail/proxy/analysis run outside project transactions;
- bounded worker pools and priority queues;
- exact operation identity, progress, cancellation, restart, and terminal result;
- project replacement/unmount prevents stale UI application but does not corrupt reusable cache entries;
- process exit either cancels cleanly or leaves a versioned resumable artifact.

### M3. Cache policy

- configured size limit and minimum free-space reserve;
- LRU or another documented eviction policy;
- pinned/in-use entries cannot be evicted underneath active decoders;
- orphan/temp cleanup, corrupt-entry quarantine, and version migration;
- concurrent readers/writers and identical-content dedupe;
- removable/network source behavior;
- operator-visible clear/rebuild/status without changing project authority.

### M4. Predecode/prefetch and degraded operation

- queued/Next/Take media receives priority over background thumbnails;
- define decoder warm-up and preroll budget;
- slow disk, cache miss, unsupported codec, and memory pressure produce truthful readiness/fault state;
- no unbounded layer/effect claim without a tested envelope and warning policy;
- resource-pressure degradation must protect DMX/ShowClock/control responsiveness before preview quality.

### M5. Fixed performance budgets

For the pinned reference machine and each supported minimum machine, record thresholds for:

- Engine 44 Hz tick p95/p99/max;
- render frame and Take scheduling p95/p99/max;
- dropped/late video frames and audio XRUNs;
- decode and command queues;
- command-to-DMX and input-to-pixel latency;
- CPU/GPU/RAM/VRAM, file handles, threads, and cache size;
- Save/Recovery/Backup/project-load pause;
- cold-cache and warm-cache V01/V02/V13/V17 runs;
- one-hour and extended soak leak slope.

Every performance gate names workload, build profile, machine, driver, sample count, warm-up, percentile method, threshold chosen before measurement, and raw result artifact.

## 18. Critical path N: Remote, Touch, RDM, and security boundary

### N1. Web Remote and Touch

- default bind remains localhost; trusted-LAN exposure is explicit and visibly unencrypted unless behind an approved TLS/VPN boundary;
- pairing PIN/token, Origin, Host, connection, message-size, and rate limits are server-enforced;
- credentials remain session/machine local and never enter `.sdc`, template, backup, diagnostic bundle, or logs;
- client list, revoke/disconnect, stale session expiry, and project/authority replacement are exact;
- Touch and Web Remote commands use the same operator lock, target IDs, validation, and terminal authority as the desktop UI;
- real iPad/Android/TouchOSC acceptance records Wi-Fi topology and latency.

### N2. RDM/TOD and blocking device work

- RDM/TOD work retains the physical-output permit through the blocking call;
- cancellation semantics distinguish “request abandoned” from “driver call stopped”;
- timeouts, duplicate responses, malformed packets, device removal, and late completion cannot cross an ownership generation;
- discovery/cache is machine/runtime state unless explicitly authored;
- no RDM operation can bypass Standby/Blackout/output fencing.

### N3. Security threat model

Document assets, trust zones, entry points, and abuse cases for:

- local Tauri IPC and raw command invocation;
- WebSocket/HTTP remote;
- LAN discovery and distributed ShowClock;
- update manifests/downloads;
- project, DVC, GDTF, media, preset, shader, and archive parsing;
- filesystem paths, symlinks/junctions, network shares, and archive traversal;
- logs/diagnostic packages and personal/project data;
- dependency/supply-chain compromise.

Required proof includes malformed/fuzz inputs at parser boundaries, size/count/depth limits, path traversal rejection, secret redaction, dependency audit/SBOM, and authenticated protocol replay/downgrade tests. Security-critical P2 issues block release.

## 19. Critical path O: compatibility, migration, and corruption recovery

### O1. Version support matrix

Publish the exact project versions accepted by the release:

- oldest supported `.sdc` version;
- each additive migration step;
- current canonical version;
- future-version fail-closed behavior;
- whether downgrade/reopen in an older Syndocal is supported;
- platform path/Unicode/case behavior.

### O2. Golden migration corpus

Maintain representative projects for:

- empty/new project;
- large lighting show and DVC imports;
- GDTF/custom profiles/groups/mappings;
- Timeline, Cue, effects, Stage, Touch, Node Graph;
- Video layers, MediaAssets, Clip Slots, outputs, effects, missing/relinked assets;
- audio, recording references, recovery, backup, template;
- legacy fields, unknown additive fields, maximum IDs, and cross-platform paths.

For every corpus item prove load, validate, migrate, save, reload, semantic equality, and second-save idempotency. Migration is clone-then-validate-then-commit; failure leaves the original bytes and current project unchanged.

### O3. Corruption and hostile-input matrix

- truncated/invalid JSON and invalid UTF-8/path encoding where applicable;
- duplicate/zero/MAX IDs and allocator overflow;
- cyclic/deep/oversized graph, group, timeline, and composition structures;
- missing fixture/profile/media/output references;
- impossible durations, NaN/infinity, negative sizes, and oversized counts;
- corrupt DVC/GDTF/archive/media/preset files;
- partial Save/Recovery/Backup/upgrade journal states.

Add deterministic property/fuzz tests to the pure parsers/normalizers. Bound time, allocation, recursion, decompression, and diagnostics. Never fuzz through live physical output.

### O4. Recovery/backup compatibility

- old browser recovery, desktop backup, and template formats either migrate or are rejected with actionable non-destructive messaging;
- startup serial/tag handshake is compatible across supported upgrades;
- upgrade/downgrade cannot silently suppress the last valid recovery image;
- corrupt newest generation falls back only to a verified older generation and reports which image was used;
- clean save, unsaved replacement, history navigation, and recovery acknowledgement retain their exact disposition across restart.

## 20. Critical path P: observability, updater, diagnostics, and supportability

### P1. Runtime observability

Expose bounded, generation-stamped status for:

- project authority/history/recovery/save;
- input workers and feedback;
- DMX/output ownership/Blackout;
- NDI/Spout/Display/camera/screen/audio/recording;
- decoder/cache/background operations;
- ShowClock peer/source/action queues.

Statuses distinguish configured, preparing, running, held, degraded, failed, cleanup pending, and retired. A stale status cannot overwrite a newer generation.

### P2. Logging and diagnostic bundle

- structured severity/domain/generation/timestamp with bounded rotation;
- redact tokens, pairing credentials, paths where configured, and media/project content;
- crash log plus last safe authority/output/clock state;
- diagnostic ZIP manifest and integrity validation;
- operator preview of included data before export;
- support runbook mapping visible error codes to safe next actions.

### P3. Updater operation

- stable/beta/nightly endpoints and signed manifest;
- exact mapping flush/save/recovery fence before install;
- download resume/cancel/hash/signature;
- install failure rollback and no project/recovery loss;
- channel switch and downgrade policy;
- offline/unreachable endpoint does not degrade show operation;
- update cannot start during unsafe recording/output/transaction state without an explicit safe transition.

### P4. Operational runbooks

Produce operator-facing and support-facing procedures for:

- pre-show health check;
- output Arm/Standby/Blackout;
- audio/video/DMX device loss;
- Primary/Standby takeover;
- project corruption/recovery;
- missing media/profile relink;
- recording recovery;
- diagnostic export;
- safe update/rollback;
- emergency shutdown and post-incident evidence preservation.

## 20A. Critical path R: AI Control Plane, MCP, and external API

The product target includes full backend operability by AI and automation clients.
The canonical detailed contract is `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`.

Completion requires:

- one generated typed Command/Query Registry covering every non-presentational
  GUI, shortcut, MIDI/OSC, Remote, project, runtime, output, file, and
  administrative operation;
- strict authored/runtime/query/output/file separation, with existing E/R/H,
  owner-incarnation, shape, terminal receipt, Undo/history, runtime generation,
  and output-ownership invariants applied centrally;
- thin Tauri, MCP, JSON-RPC/REST, and WebSocket adapters with no independent
  domain logic, defaulting, retries, or direct Engine route;
- MCP and external API discovery generated from the same Rust schemas and typed
  machine errors;
- scoped principals and risk classes `R0`-`R5`, default external safe mode,
  local human-present single-use consent for disruptive/output/file/replacement
  operations, audit/redaction, rate limits, revoke-all, and a priority kill switch;
- a localhost-only optional MCP/API sidecar whose crash or overload cannot affect
  realtime output or prevent Syndocal startup;
- non-vacuous reply-loss, stale-writer, schema parity, consent, rate/DoS, event-gap,
  sidecar-crash, file/recording, output/hardware, native, and security evidence.

DOM automation is a QA aid, not a supported backend control path. Until AI0-AI8
in the dedicated roadmap are accepted, do not claim full AI operability or safe
unattended high-risk automation.

## 21. Master traceability, decision, risk, and evidence ledger

This section is the mechanism that prevents future omissions. Before the product may be called complete, create and maintain the following tables in this file. A generated companion may mirror them for CI, but it may not replace or hide rows from this master index.

### Q0. Current master coverage registry

This is the minimum domain-level inventory. Each row must be decomposed into Q1 requirement rows before implementation of that domain is declared complete.

| Domain ID | Scope | Primary section/source | Frozen status | Release-blocking remainder |
| --- | --- | --- | --- | --- |
| `MEDIA-T1` | MediaAsset schema, import, relink, authority | A | A1-A8 accepted for current Windows tranche | macOS/Linux execution proof before any cross-platform safety claim |
| `MEDIA-DERIVED` | thumbnails, proxies, waveform, analysis, cache | M | Asset-keyed thumbnail and immutable-copy hover-preview foundation accepted on Windows | general cache model, cancellation during Begin copy, eviction, proxies/waveform/analysis, performance |
| `VIDEO-SLOT` | clip banks/slots, queue/Take | B | B1-B4 accepted for current Windows tranche | C2 transition-duration execution and broader hardware/venue acceptance |
| `VIDEO-FX` | scoped effects and transition buses | C | C1 scoped CPU/authority/UI tranche accepted | C2 Clip Take, C3 Transition Bus, C4 mapping/timeline and GPU/hardware breadth |
| `VIDEO-SOURCE` | camera, screen, NDI, Spout/Syphon, generators | L/K | Partial | authored/runtime identity, replacement fence, physical faults |
| `AUDIO-AUTHORED` | audio asset/clip/timeline/waveform | L | Partial foundation | canonical authored/runtime model and migration |
| `AUDIO-LIVE` | WASAPI/ASIO analysis and reactive paths | L/K | Advanced but externally incomplete | ownership, extended soak, devices, end-to-end latency, license decision |
| `RECORDING` | A/V recording and finalization | L | Partial legacy capability | terminal state machine, crash/disk-full recovery, artifact verification |
| `PATCH-GDTF` | fixture catalog/cache/Patch/Repair | D | Partial implementation | cache purity and atomic project/engine publication |
| `STAGE` | Stage import and mutation | D | Partial implementation | authoritative Published transaction and stale-dialog fence proof |
| `PROJECT-TX` | generic Begin/Commit/Cancel/history | E | E1 accepted on Windows | E2 authority-bundle integration, E3 durable restart recovery, and E4 Save/Save As |
| `PROJECT-AUTH` | authority bundle, mappings, disposition | E | Advanced | frozen re-audit and integration proof |
| `RECOVERY-SAVE` | recovery journal, backup, Save/As | E/O/P | Advanced | full route/latch audit, upgrade compatibility, fault evidence |
| `INPUT` | MIDI/OSC/DMX/manual lifecycle and feedback | F/K | Advanced | generation/reply-loss re-audit and physical matrix |
| `OUTPUT-LOCAL` | DMX/NDI/Spout/Display roles/fences | F/K | Advanced local tranche | project/Takeover replacement fence and real hardware |
| `REMOTE-TOUCH` | Web Remote, Touch, TouchOSC | N/K | Security foundation exists | current-source audit, physical clients, latency and stale-session proof |
| `RDM` | RDM/TOD device operation | N/K | Partial | physical, timeout/cancellation, ownership proof |
| `SHOWCLOCK` | shared time, actions, two-PC failover | G | Architecture draft | decisions, implementation, witness/fence, two-machine soak |
| `TIMELINE-MUSICAL` | phases, Guide, loop, linked media, group editing, Follow | L-TL | Foundation and linked editing through authoritative five-domain lane-valid reorder implemented/reviewed | Follow/crossfade, persistence focus/selection, representative A/V native Split/reorder proof |
| `UI-SHELL` | Setup/Edit/Control and shared shell | H | Product vision/roadmap exists | staged implementation and native/accessibility acceptance |
| `ACCESSIBILITY` | keyboard, AT, contrast, DPI, IME | H | Basic static coverage | native screen-reader/contrast/scaling matrix |
| `MIGRATION` | `.sdc`, recovery, backup, template compatibility | O | Per-feature tests exist | unified version/corruption/golden/fuzz matrix |
| `SECURITY` | IPC, remote, LAN clock, parser, update threat model | N/J | Partial | unified threat model and release-blocking proof |
| `PERFORMANCE` | latency/frame/tick/resource/soak budgets | M/K | Domain-specific evidence exists | fixed whole-product envelope and current-source soaks |
| `OBSERVABILITY` | statuses, logs, diagnostics, runbooks | P | Partial | cross-domain generation truth and support acceptance |
| `AI-CONTROL` | complete backend operation registry, MCP, JSON-RPC/REST/WS, consent and audit | R | Requirements accepted; selected backend/Remote seams exist | AI0-AI8 implementation, parity, native/external client, security and hardware acceptance |
| `RELEASE` | packages, signing, legal, updater, clean machine | J | Partial CI/package foundation | all P0-Release items |
| `COMPARE-VIDEO` | SynapseRack pinned benchmark | I | Acceptance plan only | pinned build/license and measured V01-V17 evidence |
| `COMPARE-LIGHTING` | Daslight task/parity boundary | I | Partial measured evidence | unmeasured tasks, semantic/profile and physical output evidence |

Source-to-roadmap coverage:

| Source contract | Master sections that carry its release path |
| --- | --- |
| `RELEASE_STATUS.md` | J, K, L, N, O, P, 22, 25 |
| `qa/SYNDOCAL_UI_PRODUCT_VISION.md` | B, C, G, H, L, M |
| `qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md` | H and the per-tranche native gate in 22.1 |
| `qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md` | A, B, C, L, M |
| `qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md` | A, B, C, L, M, I |
| `qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md` | I, K, L, M, Q4 |
| `qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md` | F, G, K, L, N |
| `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md` | R, E, F, G, J, K, N, P, Q |
| `qa/DASLIGHT_PARITY_COMPLETION_PLAN.md` and comparison verdict | D, I, K, N, O |
| `AGENTS.md` | section 4.5, 22.1, and 22.2 native completion procedure |

### Q1. Requirements traceability table

One row per requirement:

| Field | Required content |
| --- | --- |
| Requirement ID | Stable domain-prefixed ID, for example `MEDIA-AUTH-001` |
| Requirement | Observable behavior and failure behavior |
| Source | Product vision/spec/release-status/user decision |
| Scope | Supported, deferred, external acceptance, or out of scope |
| Owner/files | Exclusive implementation ownership and affected files |
| Dependencies | Required earlier IDs/decisions |
| Automated proof | Exact test/harness and nonzero expected count |
| Native proof | Build/hash/window/workflow evidence |
| Hardware/external proof | Device/topology/duration/raw artifact or `N/A` with reason |
| Status | Not started/In progress/Implemented/Reviewed/Accepted/Blocked |
| Commit | Focused commit SHA; never `HEAD` only |
| Residual risk | P0/P1/P2/non-blocking and disposition |

The table must include every item from the referenced product vision, UI roadmap, media transition model, Video benchmark, Output Ownership document, Release Status P0 list, Daslight comparison/parity documents, and this master roadmap.

### Q2. Decision log

Record irreversible or scope-changing choices with date, approver, alternatives, and consequences:

- supported OS/architecture matrix;
- Windows-only versus cross-platform file identity;
- external 3D visualizer boundary;
- no standalone-hardware programming boundary;
- ASIO licensing/artifact decision;
- ShowClock transport/authentication/discovery/fencing/witness;
- audio/video/ShowClock master-clock policy;
- recording ownership and recovery format;
- P2 release-blocking policy;
- comparison product versions/license tiers and reference hardware.

Initial decision state:

| Decision ID | Decision | State at this checkpoint |
| --- | --- | --- |
| `DEC-3D-001` | Built-in 3D versus external visualizer | Accepted: external Art-Net visualizer is the product boundary |
| `DEC-STANDALONE-001` | Standalone hardware scene programming | Out of current PC-software scope unless a separate hardware product is approved |
| `DEC-FILE-ID-001` | Media file identity on non-Windows | Open: implement equivalent coherence or narrow support claim |
| `DEC-ASIO-001` | ASIO license and artifact | Open; normal MIT distribution remains WASAPI-only |
| `DEC-SHOW-TRANSPORT-001` | ShowClock transport/discovery/versioning | Open |
| `DEC-SHOW-FENCE-001` | witness/shared lease/physical interlock/manual Hold | Open and blocks split-brain-safe automatic failover claim |
| `DEC-CLOCK-MASTER-001` | ShowClock/audio/video/device master policy | Open |
| `DEC-RECORD-OWN-001` | recording ownership and two-PC behavior | Open |
| `DEC-P2-001` | Which P2s block release | Accepted by section 4.7; security/migration/legal/platform/recovery/advertised-claim P2s block |
| `DEC-COMPARE-001` | pinned Daslight/SynapseRack versions/licenses/hardware | Open before comparative measurement |
| `DEC-AI-ARCH-001` | AI control architecture | Accepted: one in-process authoritative registry with optional localhost-only MCP/API sidecar; adapters contain no domain authority |
| `DEC-AI-CONSENT-001` | unattended disruptive automation | Accepted: external principals start safe; R4/R5 require locally approved, fingerprint-bound, short-lived single-use consent unless a later explicit product decision narrows the class |

### Q3. Risk register

Each open risk records ID, severity, reproduction, affected data/output/operator, likelihood, owner, mitigation, blocking milestone, proof needed, current status, and last review date. Closed risks remain in the ledger with the closing commit/evidence.

Initial high-severity register:

| Risk ID | Severity | Boundary | Status / blocking milestone |
| --- | --- | --- | --- |
| `R-MEDIA-TERM-001` | P1 | media publish/history/reply loss | Closed for the current Windows Media T1 tranche by `aad9172`/`14eeeb2`, final reviews, and A8 evidence |
| `R-TX-BEGIN-001` | P1 | Begin side effect with lost reply | Closed for the Windows E1 tranche by the client-operation receipt/query/adopt path, owner-incarnation ABA rejection, focused tests, and independent P0/P1/P2-zero review |
| `R-MEDIA-COMPAT-001` | P1 | old raw IPC authority/hash/atomicity | Closed for current Windows Media T1 by `4173e35`, command tests, and A8 acceptance |
| `R-FILE-ABA-001` | P1 if non-Windows supported | path/file image coherence | Code implementation closed by `8cb0459`; macOS/Linux execution and product support decision pending |
| `R-PATCH-ATOMIC-001` | P1 | profile ancillary and Engine Patch/Repair | Open; blocks D completion |
| `R-OUTPUT-SWAP-001` | P1 | project/Takeover replacement and external outputs | Open; blocks distributed/venue completion |
| `R-SHOW-SPLIT-001` | P1 | two-node partition and dual output | Not implemented; blocks automatic failover claim |
| `R-AUDIO-OWN-001` | P1 product boundary | Audio/MIDI/recording ownership | Open before two-PC/full-output claim |
| `R-RELEASE-HW-001` | P0-Release | required physical acceptance | Open; blocks public release |
| `R-RELEASE-LEGAL-001` | P0-Release | signing/notarization/licenses | Open; blocks public release |
| `R-MIGRATION-001` | P2 release-blocking | unified compatibility/corruption proof | Open; blocks supported-upgrade claim |
| `R-SECURITY-001` | P2 release-blocking | unified remote/parser/update threat model | Open; blocks public-network/security claims |
| `R-AI-BYPASS-001` | P1/P0-Code when externally reachable | adapter or GUI mutation bypasses the registry/authority/receipt path | Open; blocks full AI-driven claim and external R2+ release |
| `R-AI-SAFETY-001` | P0-Code | external disruptive/output/file action lacks bound consent, revocation, or fail-safe controller-loss behavior | Open; blocks external R4/R5 enablement |

### Q4. Evidence manifest

Each release candidate receives a stable evidence directory containing:

- commit/tag/artifact hashes and feature matrix;
- exact commands, exit codes, selected/pass/fail/ignored counts, warnings, and logs;
- browser/native screenshots or recordings with build identity;
- performance raw samples and summary method;
- hardware topology/device/firmware/driver and captures;
- comparison recordings and unfavorable first runs;
- signing/notarization/license/SBOM results;
- clean-machine install/update/uninstall results;
- known failures, waivers, and non-claims.

Current focused evidence records:

| Evidence ID | Scope | Record | Status |
| --- | --- | --- | --- |
| `MEDIA-T1-A7-2026-08-13` | Ordered automated Media Asset A7 gate and current native build hash | `qa/MEDIA_ASSET_T1_A7_EVIDENCE_2026-08-13.md` | Accepted for A7 only |
| `MEDIA-T1-A8-2026-08-13` | Full native Media Asset workflow matrix plus rebuilt thumbnail/hover supplement | `qa/MEDIA_ASSET_T1_A8_NATIVE_EVIDENCE_2026-08-13.md` | Accepted for the declared Windows tranche; non-Windows execution remains separate |

### Q5. Completion query

The product may be called complete for the declared release only when a generated or manually audited query shows:

- no open P0-Code or P0-Release;
- no open P1;
- no release-blocking P2;
- every Supported requirement has Implemented, Reviewed, and required Accepted evidence;
- every Deferred/External/Out-of-scope row is explicitly documented in release notes and does not contradict an advertised claim;
- all focused commits are contained in the frozen release tag;
- final native/hardware/clean-machine evidence refers to that exact tag and artifact hashes.

## 22. Global final verification order

Verification is two-tiered.

### 22.1 Per-tranche gate

After each native/runtime-affecting focused commit, before starting a dependent tranche:

1. focused format/check/tests/static/localization/viewport gates for the changed domain;
2. adversarial read-only review on frozen files;
3. resolve every P0/P1 or explicitly stop the dependency chain;
4. build the current source through the repository native gate;
5. launch the exact artifact, verify one responsive Syndocal window, maximize it, and smoke the changed workflow plus one neighboring regression path;
6. record artifact hash, screenshot/recording, result, warnings, and non-claims;
7. create the focused commit and update Q1-Q4 plus percentages.

### 22.2 Final integrated release gate

Only after all implementation tranches are committed, per-tranche native smoke is complete, and P0/P1 review is clean:

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
12. native UI and accessibility acceptance at all supported viewports/DPI/text-scale modes.
13. the full K physical hardware/protocol matrix, without substituting loopback for physical evidence.
14. cold/warm-cache performance gates and one-hour maximum-condition soak on the pinned reference machine.
15. extended leak/stability soak at the duration fixed in M5/K5.
16. two-PC ShowClock/output-ownership fault and soak matrix after that tranche exists.
17. O migration/corruption corpus and supported upgrade/downgrade matrix.
18. N security and remote exposure tests.
19. J signed/notarized clean-machine install/update/rollback/uninstall matrix.
20. I pinned Daslight/SynapseRack comparison, preserving first failures and unmeasured rows.
21. final Q5 completion query against the exact release tag and artifact hashes.

No final completion claim may be based only on browser harnesses, TypeScript, Vite, or Rust unit tests.

## 23. Planned commit cadence

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
5. `test: accept media asset tranche in native release`
   - current-source native smoke/evidence only; no unrelated implementation.
6. `fix: make project transactions recoverable after reply loss`
   - generic Begin/terminal receipt/liveness only.
7. `fix: make patch repair and stage publication atomic`
    - PATCH/GDTF/Stage authority and tests, split further if file ownership requires it.
8. `fix: fence project replacement across all outputs`
    - project/Takeover output retirement/rearm and ownership tests.
9. `feat: add media derived-data cache`
    - thumbnail/proxy/analysis/cache model and bounded workers.
10. `feat: add authored video clip slots`
    - Protocol/migration.
11. `feat: publish atomic clip slot operations`
    - Engine/runtime.
12. `feat: bridge clip slot project authority`
    - backend.
13. `feat: add video clip authoring and control surfaces`
    - frontend.
14. Video effect/transition work remains separate by scoped model, runtime, authority, and UI.
15. Audio/recording/live-source work remains separate by schema, runtime, backend, and UI.
16. ShowClock work remains separate by decision record, protocol/simulator, peer transport, authority/fencing, UI, and hardware acceptance.
17. Remote/RDM/security, migration/corruption, observability/updater, UI/accessibility, and release/distribution remain separate focused commits.
18. AI Control Plane remains separate by registry inventory/schema, read surface,
    authored/runtime bridge, principal/consent, sidecar adapters, administration UI,
    adversarial proof, and native/external acceptance.

After each commit report:

- hash and subject;
- exact files;
- exact passing tests/counts;
- warnings;
- what the commit does not prove;
- updated overall and per-domain percentages;
- next blocking boundary.

## 24. Resume protocol

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

## 25. Known open risks at this pause

### P0-Code

No confirmed P0-Code at the last frozen reviewed checkpoint. Current dirty `main.rs` has not received a final full GO.

### P0-Release

- real Art-Net/sACN/serial DMX and fixture acceptance is incomplete;
- physical MIDI/Touch/Web Remote feedback and latency acceptance is incomplete;
- maximum-condition GPU/audio/video/recording soak is incomplete;
- current-source cross-platform native acceptance is incomplete;
- signing, notarization, third-party distribution review, release artifact publication, and clean-machine install/update/uninstall are incomplete.

### P1

- Generic Begin reply-loss/liveness is closed for the accepted Windows E1 tranche; E2-E4 authority, restart durability, and Save/Save As remain open.
- Media terminal reply-loss/history and old compatibility atomicity remain closed for the accepted Windows Media T1 tranche.
- Non-Windows file coherence is implemented, but macOS/Linux execution proof and the supported-platform decision remain open.
- Pending broader PATCH/GDTF atomic project/engine commit and raw mutation admission.
- Remaining generic project/input/output authority boundaries identified in prior reviews must be re-audited before release.
- Distributed ShowClock/split-brain-safe two-PC operation is not implemented.
- Audio/recording/live-source ownership and authoritative completion are not yet a closed product tranche.
- Project/New/Recovery/Backup/Takeover replacement is not yet proven under one complete output retirement/rearm fence.
- Web Remote/Touch/RDM/security and compatibility/migration matrices are not represented by a final accepted ledger.

### P2 / proof gaps

- Media availability first-hash cancellation, reaper spawn failure, terminal receipt recovery, complete Bootstrap rollback, and all-six command idempotency have automated implementation proof; first-hash cancellation/replacement also have Windows native evidence, while non-Windows and external-platform rows remain separate.
- non-Windows execution tests;
- non-Windows Media A8 execution and supported-platform evidence;
- comparative benchmark evidence.
- media thumbnail/proxy/cache/analysis lifecycle and performance proof;
- full migration/corruption/fuzz corpus;
- native screen-reader/high-contrast/DPI/text-scale acceptance;
- observability/support/update failure matrix;
- requirements traceability and evidence manifest completion.
- AI Control Plane inventory, registry, MCP/API sidecar, scoped consent, parity,
  adversarial security, and real external-client/native acceptance.

## 26. Claims that must not be made yet

Do not claim:

- cross-platform Media Asset T1 complete (the current acceptance is explicitly Windows-scoped);
- whole-product reply-loss idempotency complete (E1 is closed, but E3 and other subsystem-specific durable boundaries remain open);
- native verification complete;
- Clip Slot model implemented;
- ShowClock distributed synchronization implemented;
- automatic two-node failover is split-brain safe;
- Daslight 5 or SynapseRack has been beaten without measured benchmark evidence;
- cross-platform file identity safety unless non-Windows is implemented and tested;
- hardware NDI/Spout/dual-HDMI/DMX correctness from SDK-free/unit tests alone.
- public-release readiness while any J/K P0-Release row is open;
- recording/audio/live-source product completion without L acceptance;
- cross-platform project/media/migration safety without O and supported-platform evidence;
- secure internet exposure of Web Remote or ShowClock without the declared TLS/authentication/fencing boundary;
- whole-product Daslight parity from loopback, fixture count, internal schema, or the currently measured subset of tasks.

The next truthful milestone is: **Critical Path B / Clip Slot T2 protocol schema and migration, followed by its Engine/backend/frontend/gates checkpoints; Media T1 remains the accepted Windows dependency baseline**.

## 27. 2026-08-21 active-train override

The historical resume sentence above is not the current train entrypoint. Current
authority is `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` and the latest operational
state is `qa/CODEX_HANDOFF_2026-08-19.md`.

- DJ Link integration on the Syndocal side is implemented and independently
  reviewed P0/P1/P2=0; the external DJ-Link repository and physical wired
  rekordbox/Pedal acceptance remain separate.
- Output Lease/Output Control are integrated enough that `checked_deadline` has
  real production callers and no longer represents suppressible dead code.
- The always-visible Lighting/Video topbar master sliders were removed without
  shrinking adjacent UI; non-header master controls remain.
- Windows native release and frontend warning output are currently zero. Promote
  the seven locally measurable P2 warning rows, keep both macOS rows pending, then
  run one final three-screen editor + LED panel + projector VJ exercise.

## 28. 2026-08-21 warning-P2 promotion closure

- The seven locally measurable warning rows have been promoted through an
  inventory-only, host/toolchain-bound zero-warning audit. All seven are at zero
  first-party warnings with exact artifact/output coverage.
- The structured warning matrix is now 11 enforced and 2 pending. macOS dev and
  release remain the only pending rows; `requiredMatrixComplete` remains false.
- The immediate next gate is the exact-executable, one-window, maximized native VJ
  exercise in the requested retained three-screen editor + LED panel + projector
  state. The earlier five-display exercise is historical capacity evidence and is
  not repeated in the final pass.

## 29. 2026-08-21 Windows release-integrity and durable-R4 closure

- Release-candidate evidence is schema-bound and fail-closed. Executable metadata
  and runtime updater identity are inspected only from a unique fixed copy of the
  hashed bytes, and a genuine Tauri signer fixture proves actual signature
  validation plus tamper rejection.
- Output Lease terminal/origin/pending state is durable without restoring
  authority. Take Over now uses the same prepare-before-physical and
  record-after-ACK boundary, while terminal and in-doubt retries cannot repeat a
  physical publication.
- The journal is atomic, capped at 8 MiB, rejects unknown fields, and validates
  nested semantics. Independent review found P0 0, P1 0, and code P2 0.
- Full Windows no-default Rust evidence is 806 passed / 0 failed / 5 ignored,
  followed by warning-zero check. Release self-tests are 65 groups and frontend
  warnings remain zero.
- The final no-bundle executable SHA-256 is
  `E28FF0260A9A2781BA6F072057C67DB18B53BA1139CD3065497BF3FB28D58ABF`.
  Exactly one responsive exact-path Syndocal window was launched and explicitly
  maximized before UI operations.

The physical-input consent sequence in the preceding historical checkpoint is
retired and must not be executed. The remaining Windows acceptance sequence is:
single-click local `enable_output_control_v2`; native Warning/Yes-No only for the
advanced dangerous mutations; stable monitor-identity persistence and
revalidation; five canonical fullscreen output-window origin checks; and the
retained editor + LED panel + projector layout. No six-digit, Raw Input, Enter, or
15-second challenge remains in production or acceptance.

## 30. 2026-08-22 E1 generic transaction closure

- Generic Begin/Commit/Cancel now uses a strict client operation identity and
  exact backend owner/window incarnation, with Pending/Committed/Cancelled
  receipt query, adopt, acknowledgement, and terminal replay behavior.
- Same-ID shape conflict, same-label owner ABA, delayed stale Commit/Cancel,
  live-pane stealing, renderer retirement races, partial/no-change history, and
  bounded owner/receipt capacity are covered by the production-path checker and
  focused Rust tests. Independent read-only review reports P0 0 / P1 0 / P2 0.
- Frontend invoke inventory is exactly 410 and localization is 3538/3538. Native
  `tauri build --no-bundle` passed with zero first-party warnings. The resulting
  executable SHA-256 is
  `0AACEA71AC666329A56DFBD57515650621E68DCD4D1B54E3ABA37918636C818C`;
  exactly one responsive 1920x1032 maximized `Syndocal` window was verified.
- Product progress is 14/79 (17.7%). E2 authority-bundle/generation consistency
  is next. E3 restart durability, E4 Save/Save As, and all later roadmap items
  remain open; this is not a whole-product or cross-platform completion claim.

## 31. 2026-08-22 alpha.4 cue-audio integration checkpoint

The next distributed development artifact is `1.2.0-alpha.4`. This is a
product-version-only advance; it is not a beta/RC/release claim and does not
alter project, command, API, ABI, or asset schema versions. The checkpoint is on
`codex/syndocal-v1.2`; its pre-commit parent is
`9b0bd7e1755571037c9ff552e880467f6ee47f8d`.

The independently frozen scheduler evidence is protocol
`48FF684AFD66680DC97F1AB5447F5FAFF920E63A5609D9ABF1FC6F6CF502BD1B` and Engine
`0031D9EC1AB5004779A9C28A2665DF2BC14D6122B1FAE32B40DFCB15D5735F66`.
The final plan/apply scheduler review reported P0/P1/P2 zero. The final
independently accepted native integration hashes are main
`E69A8989E7AE0D030C7AB3FE0AA31B2D36075CB22FADA0382EDDFE99D5214750`, cue core
`D6C1A18FD09E98AB7CA849EE439AFEF93C67C700E42466CB5B4C6854F90C1927`, and DVC
`B81913413A4796A37B4F6FE5A146CB75114AB20F0BE1A74B700A4A3C226F3546`; its
review reported P0/P1/P2 zero.

The two-song authored material remains exact: 33 Guide events, natural
`playback_rate_milli = clamp(round((1 + (BPM - 170) / 600) * 1000), 920, 1080)`
(170 BPM = 1000; 194 BPM = 1040), `Trans` at measures 149/151/153/155 every
two measures, and `Complete` as the sole Guide cue at the settlement boundary
with the destination Intro suppressed. These values are backed by the
pre-render manifest. Runtime integration and focused native UI smoke are now
accepted; audible playback and physical display/hardware acceptance remain open.

Automated gates passed: Cue Audio 41/41, DVC import 105/105, protocol 143/143,
frontend TypeScript/build, 411 invokes, 3541/3541 localization, Cue Audio
runtime/browser, the four-viewport Timeline matrix, and Windows default/release
warning ratchets with zero first-party warnings. The final no-bundle build
produced a 56,342,016-byte `1.2.0-alpha.4` executable with SHA-256
`AB98EA14F8439E23CC2E3BD80A4041C2B9CC82244A68B6938A3E9B2515A87297`.
Exactly one responsive window was maximized; Edit > Timeline rendered Click,
Guide, Follow waiting, lanes, and the source shelf without a fault. This is a
native UI smoke, not audible playback or physical editor + LED + projector,
fixture/DMX, MTC/DJ Link, audio-device, soak, beta, RC, or whole-product proof.

## 32. 2026-08-22 alpha.5 E3 recovery durability checkpoint

The active artifact ordinal advances to `1.2.0-alpha.5` without changing any
project or wire schema. E2 authority-bundle consistency and D1 cache/read purity
are reconciled as accepted from their alpha.3 independent review and current
focused reruns. E3 now has one shared App/driver publication, startup, and ACK
consumer plus one shared registered Rust lifecycle service. Two independent
reviews report P0/P1/P2 zero, and all focused frontend/Rust/build/warning gates
are green with zero first-party warnings.

E3 is still pending the single final native operation. That pass must use the
real Tauri dispatch and browser localStorage, kill/relaunch after B publication
with reply loss, kill/relaunch after durable ACK before browser cleanup, and then
record the alpha.5 release executable identity and one responsive maximized
Syndocal window. No static or headless result is substituted for this evidence.

## 33. 2026-08-23 alpha.6 E4 automated checkpoint

E4 Save durability is implemented across Save, Save As, template, backup, and
update preflight through one versioned durable publication service. Exact restart
owner adoption, server-side owner/operator fences, staging cleanup truth, backup
receipt retention, updater claims, and reply-loss query/ACK convergence have
independent P0/P1/P2-zero reviews. Focused Rust/frontend/build/warning gates are
green, including project-publication 15/15 and exact 415 invokes.

The product metadata is advancing to `1.2.0-alpha.6`, and the E4 checker is being
added to cross-platform CI. E4 is not checked complete until the alpha.6 native
build, one responsive maximized window, and scratch Save/Save As/template/backup
operation are recorded. AI4 remains blocked until that acceptance finishes.

## 34. 2026-08-23 alpha.6 E4 native acceptance

E4 is accepted for the current Windows tranche. The exact-path native build
passed with executable SHA-256
`687E7BFD8B9A74C7A3F91493C2FFAF56D97B60B32B0D0EDF6D45C2209EA563F7`, size
57,085,440 bytes, and synchronized ProductVersion/FileVersion
`1.2.0-alpha.6`. Launch verification found exactly one responsive, maximized
`Syndocal` window.

The first native pass found a real contract defect: successful User Template
receipts carried project authority and were truthfully rejected by the frontend
parser. Commit `d707872` fixes the native response and adds direct/restart proof
that User Template and Backup receipts never expose project authority while Save
and Save As retain it. Independent review returned P0/P1/P2 zero.

The rebuilt executable then passed scratch Save, Save As, User Template, and
autosave Backup. The current autosave is
`backup-1787421898035.json`, SHA-256
`01CCCBC4F99ECCC038FB80788ECBA774CDC2F1E6E1E9DBA65C7425E8A98C0EC3`; it names
`qa-final-save-as.sdc` as source and contains the `QA Backup` fixture group.
The broader current-Windows denominator is 18/71 (25.4%). This acceptance does
not close remaining show-core hardware, ASIO, security, or soak work.
