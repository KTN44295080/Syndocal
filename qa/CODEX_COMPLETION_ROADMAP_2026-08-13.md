# Syndocal complete product roadmap and frozen checkpoint

> **Operational notice (updated 2026-08-25):** this file remains a subordinate
> detailed product-requirement and final-gate record. Its frozen checkpoint,
> 75.5% planning roll-up, and Media A1 resume instructions are historical. The
> post-alpha.10 pause request was rescinded before promotion on 2026-08-25; use
> `AGENTS.md` and `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` for active work.
> Consult `qa/SYNDOCAL_POST_ALPHA10_PAUSE_HANDOFF_2026-08-24.md` for exact
> alpha.10 checkpoint evidence and residual cross-checking. Do not compare
> percentages with different denominators.

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

The active dependency-ordered flow now has an additive machine-readable open-item
index at `qa/SYNDOCAL_COMPLETION_LEDGER.json`. Its focused checker verifies the
stable markers on current sections 6-9 rows, required ownership/dependency/
non-claim/evidence fields, and tracked evidence paths. It does not replace this
master Q1-Q5 contract, complete an unchecked row, or convert native, hardware,
external, soak, or distribution acceptance into a source-only claim.
At this checkpoint it reports exactly 50 `Open` rows and 8 `Deferred` rows; the
six section-9 distribution rows remain frozen/out of scope until distribution
becomes a product goal.

The Q1-Q4 coverage contract additionally carries an additive machine-readable
mirror at `qa/SYNDOCAL_Q1_Q4_LEDGER.json`, enforced fail-closed by
`app/scripts/check-q1-q4-ledger.mjs` (`pnpm --dir app run check:q1-q4-ledger`)
with isolated-fixture negative self-tests
(`pnpm --dir app run check:q1-q4-ledger:self-test`). The canonical in-master
copy of that mirror is the single fenced `json syndocal-q1-q4-ledger` block in
this section's Q4 area; the checker fails closed on master-to-JSON drift,
missing Q0 domain/source-contract representation, unreferenced Flow markers,
unknown enum values, orphan decisions/risks/evidence, invalid evidence paths,
and acceptance claims that outrun recorded proof. This infrastructure does not
close `COMP-Q1-Q4-001`, does not treat the Flow document as complete
requirement coverage, does not convert any Flow reference into closure or
acceptance, and claims no native, hardware, external, soak, or distribution
evidence.

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

Machine-readable Q1-Q4 coverage mirror (canonical master copy). The fenced
object below must remain exactly equal to `qa/SYNDOCAL_Q1_Q4_LEDGER.json`;
`pnpm --dir app run check:q1-q4-ledger` enforces master-to-JSON field/ID
parity, complete Q0 domain and source-contract representation, reference of
every current Flow open/deferred marker without closure claims, closed enum
values, per-row automated/native/hardware proof and nonclaim rules,
orphan-free decision/risk/evidence linkage, AI control-plane linkage, and
repository-relative tracked evidence paths. The Markdown tables above remain
authoritative whenever any conflict appears; this mirror never completes an
unchecked row.

```json syndocal-q1-q4-ledger
{
  "schema_version": 1,
  "ledger_kind": "SYNDOCAL_Q1_Q4_COVERAGE_MIRROR",
  "generated_by": {
    "tranche": "COMP-Q1-Q4-001 coverage infrastructure",
    "owner": "Ox-alpha primary implementation owner",
    "date": "2026-08-25"
  },
  "authority": {
    "master_document": "qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md",
    "master_sections": ["21"],
    "master_is_authoritative": true,
    "mirror_fence_tag": "syndocal-q1-q4-ledger",
    "flow_document": "qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md",
    "completion_ledger": "qa/SYNDOCAL_COMPLETION_LEDGER.json"
  },
  "nonclaims": {
    "overall": "This ledger mirrors traceability state only. It does not claim any domain, native build, hardware device, external client, soak, distribution, or release acceptance, and it does not close COMP-Q1-Q4-001.",
    "flow_reference": "Referencing a completion-ledger marker from a Q1 row records traceability only; the Flow document is not treated as complete requirement coverage and no referenced marker is closed, accepted, or completed by this ledger.",
    "historical_evidence": "Historical evidence rows record past tranche proof only; they cannot be used or cited as current-source acceptance."
  },
  "expected_counts": {
    "q0_domains": 29,
    "q0_source_contracts": 10,
    "flow_markers": { "Open": 50, "Deferred": 8 },
    "q1_rows": 32,
    "q2_decisions": 12,
    "q3_risks": 14,
    "q4_evidence": 4
  },
  "q0_registry": {
    "mirrored_from": "master section 21 Q0 tables; the master Markdown tables remain authoritative",
    "domains": [
      { "domain_id": "MEDIA-T1", "scope_summary": "MediaAsset schema, import, relink, authority", "primary_source_section": "A", "frozen_status_summary": "A1-A8 accepted for current Windows tranche" },
      { "domain_id": "MEDIA-DERIVED", "scope_summary": "thumbnails, proxies, waveform, analysis, cache", "primary_source_section": "M", "frozen_status_summary": "thumbnail and hover-preview foundation accepted on Windows" },
      { "domain_id": "VIDEO-SLOT", "scope_summary": "clip banks/slots, queue/Take", "primary_source_section": "B", "frozen_status_summary": "B1-B4 accepted for current Windows tranche" },
      { "domain_id": "VIDEO-FX", "scope_summary": "scoped effects and transition buses", "primary_source_section": "C", "frozen_status_summary": "C1 scoped CPU/authority/UI tranche accepted" },
      { "domain_id": "VIDEO-SOURCE", "scope_summary": "camera, screen, NDI, Spout/Syphon, generators", "primary_source_section": "L/K", "frozen_status_summary": "Partial" },
      { "domain_id": "AUDIO-AUTHORED", "scope_summary": "audio asset/clip/timeline/waveform", "primary_source_section": "L", "frozen_status_summary": "Partial foundation" },
      { "domain_id": "AUDIO-LIVE", "scope_summary": "WASAPI/ASIO analysis and reactive paths", "primary_source_section": "L/K", "frozen_status_summary": "Advanced but externally incomplete" },
      { "domain_id": "RECORDING", "scope_summary": "A/V recording and finalization", "primary_source_section": "L", "frozen_status_summary": "Partial legacy capability" },
      { "domain_id": "PATCH-GDTF", "scope_summary": "fixture catalog/cache/Patch/Repair", "primary_source_section": "D", "frozen_status_summary": "Partial implementation" },
      { "domain_id": "STAGE", "scope_summary": "Stage import and mutation", "primary_source_section": "D", "frozen_status_summary": "alpha.11 D4 accepted; fixture boundary open" },
      { "domain_id": "PROJECT-TX", "scope_summary": "generic Begin/Commit/Cancel/history", "primary_source_section": "E", "frozen_status_summary": "E1-E4 accepted on Windows through alpha.11" },
      { "domain_id": "PROJECT-AUTH", "scope_summary": "authority bundle, mappings, disposition", "primary_source_section": "E", "frozen_status_summary": "Advanced" },
      { "domain_id": "RECOVERY-SAVE", "scope_summary": "recovery journal, backup, Save/As", "primary_source_section": "E/O/P", "frozen_status_summary": "Advanced" },
      { "domain_id": "INPUT", "scope_summary": "MIDI/OSC/DMX/manual lifecycle and feedback", "primary_source_section": "F/K", "frozen_status_summary": "Advanced" },
      { "domain_id": "OUTPUT-LOCAL", "scope_summary": "DMX/NDI/Spout/Display roles/fences", "primary_source_section": "F/K", "frozen_status_summary": "Advanced local tranche" },
      { "domain_id": "REMOTE-TOUCH", "scope_summary": "Web Remote, Touch, TouchOSC", "primary_source_section": "N/K", "frozen_status_summary": "Security foundation exists" },
      { "domain_id": "RDM", "scope_summary": "RDM/TOD device operation", "primary_source_section": "N/K", "frozen_status_summary": "Partial" },
      { "domain_id": "SHOWCLOCK", "scope_summary": "shared time, actions, two-PC failover", "primary_source_section": "G", "frozen_status_summary": "Architecture draft" },
      { "domain_id": "TIMELINE-MUSICAL", "scope_summary": "phases, Guide, loop, linked media, group editing, Follow", "primary_source_section": "L-TL", "frozen_status_summary": "Foundation and linked editing implemented/reviewed" },
      { "domain_id": "UI-SHELL", "scope_summary": "Setup/Edit/Control and shared shell", "primary_source_section": "H", "frozen_status_summary": "Staged implementation ongoing" },
      { "domain_id": "ACCESSIBILITY", "scope_summary": "keyboard, AT, contrast, DPI, IME", "primary_source_section": "H", "frozen_status_summary": "Basic static coverage" },
      { "domain_id": "MIGRATION", "scope_summary": ".sdc, recovery, backup, template compatibility", "primary_source_section": "O", "frozen_status_summary": "Per-feature tests exist" },
      { "domain_id": "SECURITY", "scope_summary": "IPC, remote, LAN clock, parser, update threat model", "primary_source_section": "N/J", "frozen_status_summary": "Partial" },
      { "domain_id": "PERFORMANCE", "scope_summary": "latency/frame/tick/resource/soak budgets", "primary_source_section": "M/K", "frozen_status_summary": "Domain-specific evidence exists" },
      { "domain_id": "OBSERVABILITY", "scope_summary": "statuses, logs, diagnostics, runbooks", "primary_source_section": "P", "frozen_status_summary": "Partial" },
      { "domain_id": "AI-CONTROL", "scope_summary": "backend operation registry, MCP, JSON-RPC/REST/WS, consent and audit", "primary_source_section": "R", "frozen_status_summary": "Requirements accepted; selected seams exist" },
      { "domain_id": "RELEASE", "scope_summary": "packages, signing, legal, updater, clean machine", "primary_source_section": "J", "frozen_status_summary": "Partial CI/package foundation" },
      { "domain_id": "COMPARE-VIDEO", "scope_summary": "SynapseRack pinned benchmark", "primary_source_section": "I", "frozen_status_summary": "Acceptance plan only" },
      { "domain_id": "COMPARE-LIGHTING", "scope_summary": "Daslight task/parity boundary", "primary_source_section": "I", "frozen_status_summary": "Partial measured evidence" }
    ],
    "source_contracts": [
      { "path": "RELEASE_STATUS.md", "carried_by_sections": ["J", "K", "L", "N", "O", "P", "22", "25"] },
      { "path": "qa/SYNDOCAL_UI_PRODUCT_VISION.md", "carried_by_sections": ["B", "C", "G", "H", "L", "M"] },
      { "path": "qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md", "carried_by_sections": ["H", "22.1"] },
      { "path": "qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md", "carried_by_sections": ["A", "B", "C", "L", "M"] },
      { "path": "qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md", "carried_by_sections": ["A", "B", "C", "L", "M", "I"] },
      { "path": "qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md", "carried_by_sections": ["I", "K", "L", "M", "Q4"] },
      { "path": "qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md", "carried_by_sections": ["F", "G", "K", "L", "N"] },
      { "path": "qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md", "carried_by_sections": ["R", "E", "F", "G", "J", "K", "N", "P", "Q"] },
      { "path": "qa/DASLIGHT_PARITY_COMPLETION_PLAN.md", "note": "contract includes the recorded comparison verdict documents", "carried_by_sections": ["D", "I", "K", "N", "O"] },
      { "path": "AGENTS.md", "carried_by_sections": ["4.5", "22.1", "22.2"] }
    ]
  },
  "q1_requirements": [
    {
      "id": "COV-Q1Q4-INFRA-001",
      "requirement": "Every Q0 domain and source contract carries at least one maintained Q1 requirement row, every current Flow Open/Deferred marker is referenced by traceability without closure, and the master fenced mirror equals qa/SYNDOCAL_Q1_Q4_LEDGER.json exactly; drift fails closed through check:q1-q4-ledger.",
      "failure_behavior": "Missing domain/source/marker coverage, duplicate IDs, unknown enum values, orphan decisions/risks/evidence, untracked evidence paths, or master-to-JSON drift makes check:q1-q4-ledger exit nonzero.",
      "source": "Master roadmap section 21 Q1-Q5 contract plus the additive completion-ledger paragraph in Q1.",
      "source_contracts": ["AGENTS.md", "RELEASE_STATUS.md", "qa/SYNDOCAL_UI_PRODUCT_VISION.md", "qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md", "qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md", "qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md", "qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md", "qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md", "qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md", "qa/DASLIGHT_PARITY_COMPLETION_PLAN.md"],
      "domains": ["MEDIA-T1", "MEDIA-DERIVED", "VIDEO-SLOT", "VIDEO-FX", "VIDEO-SOURCE", "AUDIO-AUTHORED", "AUDIO-LIVE", "RECORDING", "PATCH-GDTF", "STAGE", "PROJECT-TX", "PROJECT-AUTH", "RECOVERY-SAVE", "INPUT", "OUTPUT-LOCAL", "REMOTE-TOUCH", "RDM", "SHOWCLOCK", "TIMELINE-MUSICAL", "UI-SHELL", "ACCESSIBILITY", "MIGRATION", "SECURITY", "PERFORMANCE", "OBSERVABILITY", "AI-CONTROL", "RELEASE", "COMPARE-VIDEO", "COMPARE-LIGHTING"],
      "flow_refs": ["COMP-Q1-Q4-001"],
      "scope": "Supported",
      "owner_files": "qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md (Q1-Q4 sections), qa/SYNDOCAL_Q1_Q4_LEDGER.json, app/scripts/check-q1-q4-ledger.mjs, app/scripts/test-check-q1-q4-ledger.mjs, app/package.json scripts",
      "dependencies": ["Master roadmap section 21 Q1-Q5 contract", "qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md sections 6-9 markers", "qa/SYNDOCAL_COMPLETION_LEDGER.json schema v1"],
      "automated_proof": { "status": "passing", "command": "pnpm --dir app run check:q1-q4-ledger && pnpm --dir app run check:q1-q4-ledger:self-test", "expected_count": 46 },
      "native_proof": { "status": "not-applicable", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Documentation and Node-script coverage infrastructure involves no native UI or runtime surface." },
      "hardware_external_proof": { "status": "not-applicable", "device_topology_duration": "", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No physical device, external client, or venue is involved in this documentation/tooling row." },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "non-blocking", "disposition": "Coverage mirror is additive; stale future rows are caught by the fail-closed checker." },
      "nonclaim": "This row does not claim COMP-Q1-Q4-001 is closed, nor that any mirrored domain is implemented or accepted.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": ["EV-Q1Q4-COVERAGE-INFRA-2026-08-25"]
    },
    {
      "id": "COV-MEDIA-T1-001",
      "requirement": "MediaAsset import, relink, authority, history atomicity, and reply-loss truth behave observably per the A1-A8 contract; non-Windows file identity coherence either works or narrows the support claim (DEC-FILE-ID-001).",
      "failure_behavior": "Corrupt, missing, or replaced media fails import/relink visibly without publishing partial project state; raw IPC with wrong authority/hash is rejected.",
      "source": "Master roadmap critical path A and Q0 MEDIA-T1 row; RELEASE_STATUS P0 list.",
      "source_contracts": ["qa/SYNDOCAL_UI_PRODUCT_VISION.md", "qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md", "qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md", "RELEASE_STATUS.md"],
      "domains": ["MEDIA-T1"],
      "flow_refs": [],
      "scope": "Supported",
      "owner_files": "Media asset backend/engine owners per critical path A assignments",
      "dependencies": ["Project transaction authority E1-E4", "DEC-FILE-ID-001 disposition for non-Windows identity"],
      "automated_proof": { "status": "not-passing", "command": "Planned: focused Media Asset gate equivalent to the historical A7 evidence command; rerun required on current source.", "expected_count": null },
      "native_proof": { "status": "recorded", "artifact_hash": null, "raw_evidence_paths": ["qa/MEDIA_ASSET_T1_A8_NATIVE_EVIDENCE_2026-08-13.md"], "na_reason": null },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Real storage/media corpora on the operator PC; macOS/Linux execution pending.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Windows tranche accepted historically; macOS/Linux execution proof remains outside any acceptance claim." },
      "nonclaim": "This row does not claim cross-platform media safety or that historical A7/A8 evidence proves current-source acceptance.",
      "decision_ids": ["DEC-FILE-ID-001"],
      "risk_ids": ["R-MEDIA-TERM-001", "R-MEDIA-COMPAT-001", "R-FILE-ABA-001"],
      "evidence_ids": ["EV-MEDIA-T1-A7-2026-08-13", "EV-MEDIA-T1-A8-2026-08-13"]
    },
    {
      "id": "COV-MEDIA-DERIVED-001",
      "requirement": "Thumbnails, proxies, waveform, analysis, cache eviction, bounded cancellable background work, degraded operation, and cold/warm performance budgets behave per master section 17 with identity-keyed correctness.",
      "failure_behavior": "Worker overrun or cache miss degrades visibly without blocking playback or corrupting project state; cancellation during Begin copy leaves no partial artifact.",
      "source": "Flow Phase 5 MEDIA-DERIVED-001 marker; master sections 17 and Q0 MEDIA-DERIVED.",
      "source_contracts": ["qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md", "qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md"],
      "domains": ["MEDIA-DERIVED"],
      "flow_refs": ["MEDIA-DERIVED-001"],
      "scope": "Supported",
      "owner_files": "Media-derived-data owner per flow Phase 5",
      "dependencies": ["COV-MEDIA-T1-001 media authority"],
      "automated_proof": { "status": "not-passing", "command": "Planned: focused thumbnail/waveform/proxy/cache harness with nonzero selected assertions; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Current-PC media corpus plus long-duration budget runs.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Foundation accepted on Windows; general cache/eviction/budget envelope open." },
      "nonclaim": "This row does not claim cache, proxy, waveform, analysis, eviction, or performance-budget acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-VIDEO-SLOT-001",
      "requirement": "Clip banks/slots, queue, Take, and Clip Take integrate through authoritative transactions per B1-B4 and C2, including mapping/Timeline integration (C4).",
      "failure_behavior": "Invalid slot/Take operations fail closed with no project or history delta; transition-duration execution follows authored policy.",
      "source": "Master critical path B; Flow VIDEO-FULL-GATE-001 and VIDEO-C2-C4-001 markers.",
      "source_contracts": ["qa/SYNDOCAL_UI_PRODUCT_VISION.md", "qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md", "qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md"],
      "domains": ["VIDEO-SLOT"],
      "flow_refs": ["VIDEO-FULL-GATE-001", "VIDEO-C2-C4-001"],
      "scope": "Supported",
      "owner_files": "Video runtime owner per flow Phase 5",
      "dependencies": ["Accepted Clip Slot/Layer Bus/FX tranches", "Timeline arrangement authority"],
      "automated_proof": { "status": "passing", "command": "node app/scripts/check-video-clip-slot-bank.mjs && pnpm --dir app run check:timeline-follow-runtime && pnpm --dir app run check:timeline-follow-hold-ui && pnpm --dir app run check:video-runtime-polling && pnpm --dir app run check:video-output-routing-runtime && cargo test -p engine --release --locked -j 1 video_full_gate_engine_path -- --test-threads=1 && cargo test -p engine --release --locked -j 1 video_sample_clip_take_queue -- --test-threads=1 && cargo test -p engine --release --locked -j 1 video_sample_follow_admission -- --test-threads=1", "expected_count": 8 },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Multi-display VJ topology and venue conditions.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "B1-B4 accepted for Windows tranche; C2/C4 integration and full gate remain." },
      "nonclaim": "This row does not claim Clip Take, C4 integration, or full-gate acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-VIDEO-FX-001",
      "requirement": "Scoped effects and transition buses execute per authored policy across C1-C3 scope with authority checks and GPU/hardware breadth still bounded by later tranches.",
      "failure_behavior": "Unsupported effect or bus targets fail closed without silent fallback; authority violations are rejected before publication.",
      "source": "Master critical path C; Flow VIDEO-C2-C4-001 marker.",
      "source_contracts": ["qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md", "qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md"],
      "domains": ["VIDEO-FX"],
      "flow_refs": ["VIDEO-C2-C4-001"],
      "scope": "Supported",
      "owner_files": "Video transition owner per flow Phase 5",
      "dependencies": ["COV-VIDEO-SLOT-001 clip authority"],
      "automated_proof": { "status": "passing", "command": "node app/scripts/check-authored-effect-enable.mjs && node app/scripts/check-effect-draft-helpers.mjs && node app/scripts/check-value-effect-generator.mjs && node app/scripts/check-cue-effect-recall.mjs && pnpm --dir app run check:fx-visual && pnpm --dir app run check:fx-palettes && cargo test -p engine --release --locked -j 1 video_transition_bus_c3_is_typed -- --test-threads=1", "expected_count": 7 },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "GPU breadth matrix and real output devices.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "C1 accepted; C3 Transition Bus and hardware breadth open." },
      "nonclaim": "This row does not claim Transition Bus completion or GPU/hardware breadth acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-VIDEO-SOURCE-001",
      "requirement": "Live camera/screen/NDI/Spout/Syphon/generator sources expose stable identity, availability, permission/fault/reconnect truth, old-worker retirement, and physical fault matrices.",
      "failure_behavior": "Source loss or permission denial reports truthfully without silent fallback or stale worker reuse.",
      "source": "Flow VIDEO-LIVE-SOURCES-001 and VIDEO-PHYSICAL-001 markers; master Q0 VIDEO-SOURCE row.",
      "source_contracts": ["qa/SYNDOCAL_VIDEO_MODEL_GAP_AUDIT_2026-08-12.md", "RELEASE_STATUS.md"],
      "domains": ["VIDEO-SOURCE"],
      "flow_refs": ["VIDEO-LIVE-SOURCES-001", "VIDEO-PHYSICAL-001"],
      "scope": "Supported",
      "owner_files": "Live-video source owner per flow Phase 5",
      "dependencies": ["Output ownership F2 fence"],
      "automated_proof": { "status": "not-passing", "command": "Planned: live-source identity/retirement harness; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Cameras, capture devices, NDI/Spout peers, one-hour matrices.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Authored/runtime identity, replacement fence, and physical faults all remain." },
      "nonclaim": "This row does not claim NDI/Spout availability or any live-source physical acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-AUDIO-AUTHORED-001",
      "requirement": "Authored audio schema/migration/history and explicit ShowClock/audio/PTS master-clock, resampling/slew/seek/loop/underrun/device-fault policy behave deterministically.",
      "failure_behavior": "Underrun or device fault fails to the authored policy without unbounded drift or silent format fallback.",
      "source": "Flow AUDIO-AUTHORED-001 marker; master critical path L and DEC-CLOCK-MASTER-001.",
      "source_contracts": ["qa/SYNDOCAL_UI_PRODUCT_VISION.md"],
      "domains": ["AUDIO-AUTHORED"],
      "flow_refs": ["AUDIO-AUTHORED-001"],
      "scope": "Supported",
      "owner_files": "Audio model owner per flow Phase 5",
      "dependencies": ["ShowClock master-clock decision", "Timeline arrangement authority"],
      "automated_proof": { "status": "passing", "command": "pnpm --dir app run check:timeline-cue-audio && pnpm --dir app run check:timeline-audio-output-bus && pnpm --dir app run check:timeline-loop-runtime && pnpm --dir app run check:timeline-transport-runtime && cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 timeline_audio -- --test-threads=1 && cargo test -p engine --release --locked -j 1 timeline_fractional_ticks -- --test-threads=1", "expected_count": 6 },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Real audio devices and long-duration clocks.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Canonical authored/runtime model and migration remain; audio ownership risk shared with recording/live audio." },
      "nonclaim": "This row does not claim authored-audio migration or device-fault policy acceptance.",
      "decision_ids": ["DEC-CLOCK-MASTER-001"],
      "risk_ids": ["R-AUDIO-OWN-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-AUDIO-LIVE-001",
      "requirement": "Windows ASIO input meets the full qa/ASIO_INPUT_ACCEPTANCE.md gate (license/artifact decision, advertised rate/buffer/format/channel matrix without fallback, fault/XRUN/disconnect recovery, matched one-hour soak, latency measurement, persistence/package identity) while WASAPI stays the MIT default.",
      "failure_behavior": "Driver faults, occupied devices, or unsupported formats fail closed with explicit telemetry and never silently fall back.",
      "source": "Flow section 7 ASIO markers; AGENTS.md Windows ASIO product gate; DEC-ASIO-001.",
      "source_contracts": ["AGENTS.md", "RELEASE_STATUS.md"],
      "domains": ["AUDIO-LIVE"],
      "flow_refs": ["ASIO-LICENSE-001", "ASIO-FORMAT-MATRIX-001", "ASIO-FAULT-MATRIX-001", "ASIO-SOAK-001", "ASIO-LATENCY-001", "ASIO-PERSISTENCE-PACKAGE-001"],
      "scope": "Supported",
      "owner_files": "ASIO bridge/product owner; tools/asio-bridge lane kept separately licensed",
      "dependencies": ["Explicit GPLv3-separated artifact or Steinberg SDK decision", "Second-vendor driver cycle already passed"],
      "automated_proof": { "status": "not-passing", "command": "Planned: deterministic ASIO loader/negotiation suites plus qa/ASIO_INPUT_ACCEPTANCE.md gates; real-device rows cannot be automated.", "expected_count": null },
      "native_proof": { "status": "recorded", "artifact_hash": null, "raw_evidence_paths": ["qa/ASIO_INPUT_ACCEPTANCE.md"], "na_reason": null },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "HOTONE second-vendor cycle passed historically; advertised matrix, fault injection, matched one-hour soak, and TouchDesigner trials pending on current source.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "License decision and most acceptance checkboxes remain unchecked per the authoritative ASIO document." },
      "nonclaim": "This row does not claim ASIO release completion while any qa/ASIO_INPUT_ACCEPTANCE.md gate remains unchecked, and does not claim the historical HOTONE cycle as current-source acceptance.",
      "decision_ids": ["DEC-ASIO-001"],
      "risk_ids": ["R-AUDIO-OWN-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-RECORDING-001",
      "requirement": "Recording moves Idle -> Preparing -> Recording -> Finalizing -> Complete|Fault with target reservation, disk/crash/encoder/timeout recovery, verified atomic artifacts, two-PC ownership, and authoritative asset import.",
      "failure_behavior": "Crash or disk-full during recording yields a truthful Fault with no partially promoted artifact imported as authoritative.",
      "source": "Flow RECORDING-001 marker; master critical path L and DEC-RECORD-OWN-001.",
      "source_contracts": ["qa/SYNDOCAL_UI_PRODUCT_VISION.md"],
      "domains": ["RECORDING"],
      "flow_refs": ["RECORDING-001"],
      "scope": "Supported",
      "owner_files": "Recording runtime owner per flow Phase 5",
      "dependencies": ["Audio master-clock decision", "Output ownership decisions"],
      "automated_proof": { "status": "not-passing", "command": "Planned: recording state-machine crash/disk-full harness; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Two-PC ownership topology and encoder hardware.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Terminal machine, recovery, artifact verification, and two-PC ownership all remain." },
      "nonclaim": "This row does not claim recording state-machine or two-PC ownership acceptance.",
      "decision_ids": ["DEC-RECORD-OWN-001"],
      "risk_ids": ["R-AUDIO-OWN-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-PATCH-GDTF-001",
      "requirement": "Fixture catalog/cache purity and PATCH/GDTF Repair publish atomically through the authoritative path with D-lane integrity; standalone-hardware programming stays out of scope per DEC-STANDALONE-001.",
      "failure_behavior": "Profile corruption or repair failure rolls back allocator/cache/output state with one truthful history result.",
      "source": "Master critical path D and Q0 PATCH-GDTF row.",
      "source_contracts": ["qa/DASLIGHT_PARITY_COMPLETION_PLAN.md", "AGENTS.md"],
      "domains": ["PATCH-GDTF"],
      "flow_refs": [],
      "scope": "Supported",
      "owner_files": "Patch/GDTF owner per critical path D",
      "dependencies": ["Engine publication authority"],
      "automated_proof": { "status": "not-passing", "command": "Planned: rerun of fixture-catalog/profile-library checkers plus atomic-repair regression on current source.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Real fixtures for repaired-profile output verification.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Cache purity and atomic publication risk R-PATCH-ATOMIC-001 remains open." },
      "nonclaim": "This row does not claim D-lane completion or fixture-level visual parity with pinned comparators.",
      "decision_ids": ["DEC-STANDALONE-001"],
      "risk_ids": ["R-PATCH-ATOMIC-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-STAGE-001",
      "requirement": "Stage import/mutation runs through the authoritative Published transaction with stale-dialog fencing; alpha.11 accepted stage-object/preset/undo flows stay reproducible on current source.",
      "failure_behavior": "Stale dialogs and concurrent mutations are rejected without corrupting stage state; undo/redo restores exact objects.",
      "source": "Master Q0 STAGE row; flow sections 37-39 alpha.11 D4 acceptance boundary.",
      "source_contracts": ["AGENTS.md"],
      "domains": ["STAGE"],
      "flow_refs": [],
      "scope": "Supported",
      "owner_files": "Stage/D4 owner; fixture-transform boundary tracked here",
      "dependencies": ["Project transaction authority E1-E4"],
      "automated_proof": { "status": "not-passing", "command": "Planned: rerun of engine D4 6/6 and backend D4 7/7 focused tests on current source.", "expected_count": null },
      "native_proof": { "status": "recorded", "artifact_hash": "1B010C40242A5C7DD7A2797EAC1ECA2D31BCACE4455BA57C7F935611075B582B", "raw_evidence_paths": ["qa/SYNDOCAL_ALPHA11_MAIN_INTEGRATION_CHECKPOINT_2026-08-25.md"], "na_reason": null },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Fixture transform A/B step requires patched fixtures; the launched Untitled project had zero patched fixtures at alpha.11 QA.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "non-blocking", "disposition": "Software/native integration accepted at alpha.11; fixture/hardware-specific behavior explicitly unverified." },
      "nonclaim": "This row does not claim fixture/hardware Stage behavior acceptance, and the recorded alpha.11 hash is historical tranche evidence, not current-source acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": ["EV-ALPHA11-D4-2026-08-25"]
    },
    {
      "id": "COV-PROJECT-TX-001",
      "requirement": "Generic Begin/Commit/Cancel with receipts, authority bundles, durable restart recovery, and Save/Save As publication keep every mutation on one recoverable authority path (E1-E4).",
      "failure_behavior": "Reply loss, crash, stale delayed commits, owner ABA, and same-ID shape changes fail closed with durable, replay-safe receipts.",
      "source": "Master critical path E and Q0 PROJECT-TX row; flow Phase 1 accepted rows.",
      "source_contracts": ["RELEASE_STATUS.md", "AGENTS.md"],
      "domains": ["PROJECT-TX"],
      "flow_refs": [],
      "scope": "Supported",
      "owner_files": "Project transaction authority owners (Phase 1)",
      "dependencies": ["Engine publication authority", "Control-plane admission"],
      "automated_proof": { "status": "not-passing", "command": "Planned: rerun of project_transaction/project-publication/recovery checkers on current source with nonzero selections.", "expected_count": null },
      "native_proof": { "status": "recorded", "artifact_hash": "1B010C40242A5C7DD7A2797EAC1ECA2D31BCACE4455BA57C7F935611075B582B", "raw_evidence_paths": ["qa/SYNDOCAL_ALPHA11_MAIN_INTEGRATION_CHECKPOINT_2026-08-25.md"], "na_reason": null },
      "hardware_external_proof": { "status": "not-applicable", "device_topology_duration": "", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Transaction authority is host-local software behavior; no external device participates in this contract." },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "non-blocking", "disposition": "Phase 1 accepted through alpha.11; cross-platform execution remains deferred scope." },
      "nonclaim": "This row does not claim macOS/Linux execution or that alpha.11 hashes constitute current-source acceptance.",
      "decision_ids": [],
      "risk_ids": ["R-TX-BEGIN-001"],
      "evidence_ids": ["EV-ALPHA11-D4-2026-08-25"]
    },
    {
      "id": "COV-PROJECT-AUTH-001",
      "requirement": "Authority bundles, mappings, and dispositions stay generation-consistent across every mutation with frozen re-audit and integration proof.",
      "failure_behavior": "Generation mismatch or mapping drift is rejected before any publication or output effect.",
      "source": "Master Q0 PROJECT-AUTH row and critical path E2.",
      "source_contracts": ["qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md"],
      "domains": ["PROJECT-AUTH"],
      "flow_refs": [],
      "scope": "Supported",
      "owner_files": "Project authority owner (E2 lane)",
      "dependencies": ["E1 transaction receipts"],
      "automated_proof": { "status": "not-passing", "command": "Planned: rerun of project-authority checker bundle on current source.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-applicable", "device_topology_duration": "", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Authority consistency is host-local software behavior; no external device participates." },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Frozen re-audit and integration proof remain." },
      "nonclaim": "This row does not claim the frozen re-audit has been performed.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-RECOVERY-SAVE-001",
      "requirement": "Recovery journal, backup reservation, Save/Save As atomic replacement, upgrade compatibility, and fault evidence hold across restarts with full route/latch audit.",
      "failure_behavior": "Corrupt journals/backups fail closed without resurrecting stale authority or losing acknowledged checkpoints.",
      "source": "Master Q0 RECOVERY-SAVE row; critical paths E/O/P.",
      "source_contracts": ["RELEASE_STATUS.md"],
      "domains": ["RECOVERY-SAVE"],
      "flow_refs": [],
      "scope": "Supported",
      "owner_files": "Recovery/save owner (E3/E4 lanes)",
      "dependencies": ["E4 publication receipts"],
      "automated_proof": { "status": "passing", "command": "pnpm --dir app run check:project-recovery-e3 && pnpm --dir app run check:project-publication-e4 && pnpm --dir app run check:project-storage && node app/scripts/check-project-history-preflight.mjs && node app/scripts/check-project-history-keyboard.mjs", "expected_count": 5 },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Power-loss/disk-fault simulation hardware or VM matrices.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Full route/latch audit, upgrade compatibility, and fault evidence remain." },
      "nonclaim": "This row does not claim upgrade-compatibility or power-fault acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-INPUT-001",
      "requirement": "MIDI/OSC/DMX/manual input lifecycle uses monotonic generations with stale callback retirement on mapping/project replacement, and passes the physical MIDI/OSC/Remote latency matrix.",
      "failure_behavior": "Stale callbacks after replacement are retired before side effects; unknown ingress policies fail closed.",
      "source": "Flow F1-INPUT-GENERATIONS-001 and INPUT-PHYSICAL-001 markers; master Q0 INPUT row.",
      "source_contracts": ["qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md", "RELEASE_STATUS.md"],
      "domains": ["INPUT"],
      "flow_refs": ["F1-INPUT-GENERATIONS-001", "INPUT-PHYSICAL-001"],
      "scope": "Supported",
      "owner_files": "Input-runtime owner per flow Phase 4",
      "dependencies": ["Project replacement authority", "Ingress canonicalization already landed code-side"],
      "automated_proof": { "status": "not-passing", "command": "Planned: generation/retirement regression suite; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Generated source inventory is not physical ingress acceptance." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Physical MIDI/OSC controllers, TouchOSC/iPad/Android over wired/Wi-Fi with p50/p95/p99/max latency.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Generation re-audit and the whole physical matrix remain." },
      "nonclaim": "This row does not claim physical MIDI/OSC/Remote acceptance or that source inventories substitute for hardware proof.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-OUTPUT-LOCAL-001",
      "requirement": "Lighting/Video/Both/Standby ownership spans DMX, NDI, Spout, Display, native windows, SDK resources, Take Over, teardown ACK, and explicit Arm; Art-Net/sACN nodes, fixtures, USB DMX timing, and RDM/TOD pass their physical matrices.",
      "failure_behavior": "Ownership conflicts, ambiguous overlap, or teardown loss fail closed with S0 Blackout independent and intact.",
      "source": "Flow F2-OUTPUT-OWNERSHIP-001, DMX-ARTNET-001, DMX-USB-RDM-001 markers; DEC-3D-001 external visualizer boundary.",
      "source_contracts": ["qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md", "RELEASE_STATUS.md"],
      "domains": ["OUTPUT-LOCAL"],
      "flow_refs": ["F2-OUTPUT-OWNERSHIP-001", "DMX-ARTNET-001", "DMX-USB-RDM-001"],
      "scope": "Supported",
      "owner_files": "Output-ownership and lighting hardware acceptance owners",
      "dependencies": ["F1 input generations", "AI3 lease authority", "One-click local Enable path"],
      "automated_proof": { "status": "not-passing", "command": "Planned: output-ownership/control-runtime checker reruns on current source; physical matrices are not automatable.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Historical four-display Add Display acceptance is recorded in the flow document but is not current-source acceptance for this row." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Real Art-Net/sACN nodes/fixtures, Enttec/DMXKing long run with logic analyzer, multi-universe 44 Hz continuity.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Replacement fence risk R-OUTPUT-SWAP-001 and all listed physical matrices remain open." },
      "nonclaim": "This row does not claim DMX/NDI/Spout/display hardware acceptance or distributed/venue completion.",
      "decision_ids": ["DEC-3D-001"],
      "risk_ids": ["R-OUTPUT-SWAP-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-REMOTE-TOUCH-001",
      "requirement": "Web Remote/Touch exposure keeps authenticated pairing, Origin/Host, grants, bounds, RDM/TOD cancellation ownership, and DJ Link setup/hardware workflows (NIC bind, token rotation, CAS mapping, Pedal/Agent acceptance) truthful under audit.",
      "failure_behavior": "Unauthenticated or malformed remote traffic fails closed; DJ Link sessions recover from disconnect without phantom triggers.",
      "source": "Flow REMOTE-SECURITY-001, DJ-LINK-SETUP-001, DJ-LINK-HARDWARE-001 markers; REKORDBOX acceptance authority.",
      "source_contracts": ["qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md", "RELEASE_STATUS.md"],
      "domains": ["REMOTE-TOUCH"],
      "flow_refs": ["REMOTE-SECURITY-001", "DJ-LINK-SETUP-001", "DJ-LINK-HARDWARE-001"],
      "scope": "Supported",
      "owner_files": "Remote/security and DJ Link peer owners",
      "dependencies": ["AI4/AI5 consent and sidecar policy", "Authenticated /dj-link ingress implementation"],
      "automated_proof": { "status": "not-passing", "command": "Planned: parser/path/archive fuzz and remote security suites; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "The 2026-08-25 live-LAN preflight observed no TCP 9100 listener and checked none of HW-4.1..HW-4.12." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Wired DJ-PC Agent, Stream Deck Pedal, rekordbox Master Track, TouchOSC/iPad clients; DJ/Pedal submatrix currently 0/12.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Restart token/bind restoration gap (P1) and NIC ambiguity (P2) recorded in flow section 40." },
      "nonclaim": "This row does not claim LAN exposure safety, DJ hardware acceptance, or completed peer-artifact identity binding.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-RDM-001",
      "requirement": "RDM/TOD discovery, correlation, timeout/cancellation, and removal behave within ownership boundaries against physical devices.",
      "failure_behavior": "Timeouts cancel cleanly without leaking queued TOD work into replaced mappings.",
      "source": "Flow DMX-USB-RDM-001 marker (RDM portion); master Q0 RDM row.",
      "source_contracts": ["qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md"],
      "domains": ["RDM"],
      "flow_refs": ["DMX-USB-RDM-001"],
      "scope": "Supported",
      "owner_files": "DMX/RDM hardware acceptance owner",
      "dependencies": ["F2 output ownership"],
      "automated_proof": { "status": "not-passing", "command": "Planned: RDM timeout/cancellation unit harness; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Physical RDM devices with discovery/timeout captures.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Physical, timeout/cancellation, and ownership proof all remain." },
      "nonclaim": "This row does not claim RDM/TOD physical operation.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-SHOWCLOCK-001",
      "requirement": "Shared ShowClock transport/discovery/authentication/key rotation/replay, master-clock/slew/Hold, witness/fence/physical-interlock, mixed-version, implementation, and two-machine venue rehearsal freeze decisions first, then prove partition/rejoin cannot create simultaneous output.",
      "failure_behavior": "Partition, crash, or rejoin fails toward zero simultaneous output; automatic failover is never claimed without the accepted witness/fence.",
      "source": "Flow SHOWCLOCK-* markers; master critical path G; DEC-SHOW-* decisions.",
      "source_contracts": ["qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md", "RELEASE_STATUS.md"],
      "domains": ["SHOWCLOCK"],
      "flow_refs": ["SHOWCLOCK-DECISIONS-001", "SHOWCLOCK-IMPLEMENTATION-001", "SHOWCLOCK-VENUE-001"],
      "scope": "Supported",
      "owner_files": "ShowClock architecture and implementation owners",
      "dependencies": ["F2 local ownership boundary", "Audio/recording clock semantics"],
      "automated_proof": { "status": "not-passing", "command": "Planned: schema/simulator/estimator deterministic suites after decision freeze; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Two-machine real-switch crash/partition/rejoin/device-loss rehearsal.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "Blocked",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Split-brain risk R-SHOW-SPLIT-001 blocks any automatic failover claim until witness/fence decisions close." },
      "nonclaim": "This row does not claim transport, fencing, implementation, or two-machine soak acceptance.",
      "decision_ids": ["DEC-SHOW-TRANSPORT-001", "DEC-SHOW-FENCE-001", "DEC-CLOCK-MASTER-001"],
      "risk_ids": ["R-SHOW-SPLIT-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-TIMELINE-MUSICAL-001",
      "requirement": "Timeline phases, Guide voice, musical loop, linked media, Follow/crossfade with BPM slew and failure policy, persistence focus/selection, fixed Guide device routing, and native A/V/Lighting synchronization meet the sample-frame scheduler contracts including the pinned two-song material.",
      "failure_behavior": "Discontinuities, meter changes, loop wraps, and device restarts cancel stale scheduled frames by generation; missing Guide devices fail closed without double speech.",
      "source": "Flow TIMELINE-FOLLOW-001 and TIMELINE-PERSISTENCE-001 markers; master L-TL path and click/Guide contracts.",
      "source_contracts": ["qa/SYNDOCAL_UI_PRODUCT_VISION.md", "qa/SYNDOCAL_VIDEO_LAYER_MEDIA_TRANSITION_MODEL.md"],
      "domains": ["TIMELINE-MUSICAL"],
      "flow_refs": ["TIMELINE-FOLLOW-001", "TIMELINE-PERSISTENCE-001"],
      "scope": "Supported",
      "owner_files": "Timeline runtime/editor owners",
      "dependencies": ["Sample-frame scheduler acceptance (alpha.4 era)", "Guide vocabulary assets"],
      "automated_proof": { "status": "not-passing", "command": "Planned: rerun of timeline-follow-runtime/timeline-cue-audio checkers and Engine scheduler suites on current source.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Audible-device and final three-screen passes remain; browser/sample-frame proofs do not establish them." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Audible click/Guide routing on real outputs plus MTC/DJ discontinuity drills.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Follow/persistence/native-sync boundaries remain; pre-rendered stems prove authored material only." },
      "nonclaim": "This row does not claim audible-device acceptance, native synchronization proof, or that pre-rendered WAV evidence replaces the runtime scheduler requirement.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-UI-SHELL-001",
      "requirement": "Every supported feature has a discoverable operator path (H1) inside one truthful shared shell (H2), with Setup (H3 incl. DJ Link setup), Edit (H4), and Control (H5) completions proven natively.",
      "failure_behavior": "Dead/hidden routes, stale status/error/selection contracts, or unreachable workflows fail the reachability/shell gates instead of shipping.",
      "source": "Flow UI-H1..H5 markers; UI product vision and implementation roadmap.",
      "source_contracts": ["qa/SYNDOCAL_UI_PRODUCT_VISION.md", "qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md"],
      "domains": ["UI-SHELL"],
      "flow_refs": ["UI-H1-REACHABILITY-001", "UI-H2-SHELL-001", "UI-H3-SETUP-001", "UI-H4-EDIT-001", "UI-H5-CONTROL-001"],
      "scope": "Supported",
      "owner_files": "UI workspace owners per flow Phase 6",
      "dependencies": ["Supported-feature inventory", "Backend-first registered command validation rule"],
      "automated_proof": { "status": "not-passing", "command": "Planned: routing/reachability static gates extended to H1 dead-route detection; existing viewport checkers cover only staged surfaces.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Native 2560x1440 and 1280x720 placements remained unverified even at alpha.7; no H1-H5 native acceptance exists." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Editor + LED panel + projector three-screen operating state.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Staged workspaces pass scoped browser/native checks; H1/H2 completeness and H3-H5 completion remain." },
      "nonclaim": "This row does not claim reachability completeness or native H3-H5 acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-ACCESSIBILITY-001",
      "requirement": "NVDA, High Contrast, color-independent states, 125/150/200% scaling, keyboard-only safety workflows, IME, dialog/popout focus, and reduced motion pass native accessibility acceptance.",
      "failure_behavior": "Accessibility failures block the release gate rather than being waived as cosmetic.",
      "source": "Flow ACCESSIBILITY-NATIVE-001 marker; UI implementation roadmap.",
      "source_contracts": ["qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md"],
      "domains": ["ACCESSIBILITY"],
      "flow_refs": ["ACCESSIBILITY-NATIVE-001"],
      "scope": "External acceptance",
      "owner_files": "Accessibility acceptance owner",
      "dependencies": ["H1-H5 native UI implementation", "Available accessibility environments"],
      "automated_proof": { "status": "not-passing", "command": "Planned: static keyboard-path/label audits; AT behavior needs native environments.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Only basic static coverage exists today." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "NVDA + High Contrast + scaling matrix on the operator PC.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Whole native matrix unmeasured." },
      "nonclaim": "This row does not claim any screen-reader, contrast, scaling, or IME acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-MIGRATION-001",
      "requirement": "Supported .sdc/template/cache/protocol version matrix holds with golden migration corpus, hostile/corrupt input rejection, fuzz, backup/recovery/upgrade compatibility; non-Windows file identity follows DEC-FILE-ID-001.",
      "failure_behavior": "Unknown or hostile versions/inputs are rejected with truthful errors and no partial migration.",
      "source": "Flow MIGRATION-COMPATIBILITY-001 marker; master critical path O and R-MIGRATION-001.",
      "source_contracts": ["RELEASE_STATUS.md"],
      "domains": ["MIGRATION"],
      "flow_refs": ["MIGRATION-COMPATIBILITY-001"],
      "scope": "Supported",
      "owner_files": "Migration and recovery owner",
      "dependencies": ["Version-support decisions", "Durable save/recovery boundaries"],
      "automated_proof": { "status": "not-passing", "command": "Planned: unified migration/corruption/golden/fuzz matrix; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row in the owned evidence manifest." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Upgrade/downgrade rehearsals on real machines.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2 release-blocking", "disposition": "Unified compatibility/corruption proof blocks supported-upgrade claims." },
      "nonclaim": "This row does not claim any supported-upgrade or corruption-resistance acceptance.",
      "decision_ids": ["DEC-FILE-ID-001"],
      "risk_ids": ["R-MIGRATION-001", "R-FILE-ABA-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-SECURITY-001",
      "requirement": "Unified IPC/remote/LAN-clock/parser/update threat model reaches release-blocking proof; AI adapter bypass and unconsented disruptive automation stay impossible via dedicated registry linkage.",
      "failure_behavior": "Parser, path, archive, and update attacks fail closed; any GUI/adapter mutation bypassing the registry/authority/receipt path fails AI0 admission.",
      "source": "Flow REMOTE-SECURITY-001 marker; master R-SECURITY/R-AI risks; DEC-AI-CONSENT-001.",
      "source_contracts": ["qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md", "RELEASE_STATUS.md"],
      "domains": ["SECURITY"],
      "flow_refs": ["REMOTE-SECURITY-001"],
      "scope": "Supported",
      "owner_files": "Security and AI-safety owners",
      "dependencies": ["AI registry authority", "Consent service design"],
      "automated_proof": { "status": "not-passing", "command": "Planned: unified threat-model fuzz/red-team suite; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native security acceptance run is recorded for this row." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Adversarial LAN/remoting exercises against real peers.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2 release-blocking", "disposition": "Unified threat model and bypass-absence proof remain; AI risks tracked jointly with COV-AI-CONTROL-001." },
      "nonclaim": "This row does not claim public-network safety or absence-of-bypass proof.",
      "decision_ids": ["DEC-AI-CONSENT-001"],
      "risk_ids": ["R-SECURITY-001", "R-AI-BYPASS-001", "R-AI-SAFETY-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-PERFORMANCE-001",
      "requirement": "Fixed whole-product latency/frame/tick/resource envelopes and maximum-condition venue soaks meet fixed budgets on the pinned reference machine.",
      "failure_behavior": "Budget breaches fail the gate with measured evidence instead of being reinterpreted.",
      "source": "Flow VENUE-SOAK-001 marker; master Q0 PERFORMANCE row.",
      "source_contracts": ["qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md", "RELEASE_STATUS.md"],
      "domains": ["PERFORMANCE"],
      "flow_refs": ["VENUE-SOAK-001"],
      "scope": "External acceptance",
      "owner_files": "Venue performance acceptance owner",
      "dependencies": ["Integrated video/lighting/output/recording paths"],
      "automated_proof": { "status": "not-passing", "command": "Planned: cold/warm performance gates; venue one-hour run is inherently physical.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No whole-product envelope run is recorded." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Venue GPU maximum ISF/layer/Preview/Program/output/recording one-hour run with resource logs.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Fixed envelope and current-source soaks remain." },
      "nonclaim": "This row does not claim any soak or envelope acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-OBSERVABILITY-001",
      "requirement": "Generation-stamped status truth, redacted diagnostic bundles, updater failure behaviors, and operator/support runbooks reach support acceptance.",
      "failure_behavior": "Status lies, unredacted diagnostics, or wrong-channel/downgrade updater acceptance fail closed.",
      "source": "Flow OBSERVABILITY-SUPPORT-001 marker; master critical path P.",
      "source_contracts": ["RELEASE_STATUS.md"],
      "domains": ["OBSERVABILITY"],
      "flow_refs": ["OBSERVABILITY-SUPPORT-001"],
      "scope": "Supported",
      "owner_files": "Observability and support owner",
      "dependencies": ["Generation-stamped runtime state", "Release/update policy"],
      "automated_proof": { "status": "not-passing", "command": "Planned: diagnostic-bundle redaction/updater negative suites; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No current-source native acceptance run is recorded for this row." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Operator startup/failure/takeover/recovery/shutdown runbook drills.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Cross-domain generation truth and support acceptance remain." },
      "nonclaim": "This row does not claim diagnostics or updater resilience acceptance.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-AI-CONTROL-001",
      "requirement": "AI0-AI8 deliver the complete typed operation registry with fail-closed unclassified mutations (AI0), canonical query/event schemas with gap/resnapshot (AI1), authored command bridge with E/R/H/owner/receipts/Undo (AI2), durable runtime/output lease recovery and native ingress/physical re-Arm acceptance (AI3), principals/grants/revocation/kill-switch/consent (AI4), authenticated localhost sidecar MCP/JSON-RPC/REST/WebSocket (AI5), administration UI (AI6), adversarial parity/security/rate/reply-loss/fuzz/saturation proof (AI7), and release-native external acceptance (AI8) per qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md sections 2-10.",
      "failure_behavior": "Unclassified mutations, cached-authority replay, unauthenticated sidecars, and consent bypass fail closed; adapters contain no domain authority.",
      "source": "Flow Phase 3 AI0-AI8 markers; AI control-plane roadmap architecture/risk/capability model; DEC-AI-ARCH-001 and DEC-AI-CONSENT-001.",
      "source_contracts": ["qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md", "AGENTS.md"],
      "domains": ["AI-CONTROL"],
      "flow_refs": ["AI0-COVERAGE-001", "AI1-SCHEMAS-001", "AI2-COMMAND-BRIDGE-001", "AI3-DURABLE-RECOVERY-001", "AI3-NATIVE-INGRESS-001", "AI3-PHYSICAL-REARM-001", "AI3-DURABLE-ACCEPTANCE-001", "AI4-CONSENT-001", "AI5-SIDECAR-001", "AI6-ADMIN-UI-001", "AI7-ADVERSARIAL-PROOF-001", "AI8-EXTERNAL-ACCEPTANCE-001"],
      "scope": "Supported",
      "owner_files": "AI control-plane owner plus AI3 lease, AI4 consent, AI5 sidecar, AI6 admin, AI7 adversarial, AI8 acceptance lanes",
      "dependencies": ["AI3 remaining durable/physical items before Phase 3 exit", "Retired Raw Input challenge must not be restored"],
      "automated_proof": { "status": "not-passing", "command": "Planned: registry coverage/schema-parity/adversarial generated suites per roadmap section 7 non-vacuous evidence rules; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No AI0-AI8 native or external-client acceptance run is recorded." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Real external clients, clean install, hardware output, crash/restart drills.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P1", "disposition": "Registry/risk/schema/capability proof remains linked to this dedicated row and must not be flattened into generic security claims." },
      "nonclaim": "This row does not claim AI0-AI8 acceptance, external R4/R5 enablement, or that existing seams satisfy roadmap section 7 evidence.",
      "decision_ids": ["DEC-AI-ARCH-001", "DEC-AI-CONSENT-001"],
      "risk_ids": ["R-AI-BYPASS-001", "R-AI-SAFETY-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-RELEASE-001",
      "requirement": "check:release gains tag/previous-version/updater/artifact gates; P0-Release hardware/legal risks and the DEC-P2-001 blocking policy hold for any candidate.",
      "failure_behavior": "Version/tag/updater/artifact mismatches fail the release metadata gate; unresolved P0-Release/P1/blocking-P2 items stop candidacy.",
      "source": "Flow RELEASE-METADATA-GATE-001 marker; master Q2/Q3 release rows; UPDATE_RELEASE_RUNBOOK.",
      "source_contracts": ["RELEASE_STATUS.md", "AGENTS.md"],
      "domains": ["RELEASE"],
      "flow_refs": ["RELEASE-METADATA-GATE-001"],
      "scope": "Supported",
      "owner_files": "Release metadata owner; app/scripts/check-release-metadata.mjs lane",
      "dependencies": ["Synchronized product metadata surfaces", "Warning ratchet zero before RC"],
      "automated_proof": { "status": "not-passing", "command": "Planned: extended check:release with tag/previous-version/updater/artifact checks (the open marker itself); current check:release covers synchronized metadata only.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No release candidate exists; nothing to verify natively." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Required physical acceptance per R-RELEASE-HW-001.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P0-Release", "disposition": "Hardware and legal P0-Release risks block public release." },
      "nonclaim": "This row does not claim beta/RC/release readiness or any signed/publication outcome.",
      "decision_ids": ["DEC-P2-001"],
      "risk_ids": ["R-RELEASE-HW-001", "R-RELEASE-LEGAL-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-DISTRIBUTION-DEFERRED-001",
      "requirement": "The six frozen distribution rows (platform/package matrix, Authenticode/Apple signing, BOM/SBOM/notices, clean-machine install/upgrade/uninstall, signed updater, public tag/artifact/evidence publication) stay explicitly deferred/out-of-scope until distribution becomes a product goal.",
      "failure_behavior": "Any premature distribution claim contradicts the active Windows-local denominator and fails review; reopening requires an explicit product goal.",
      "source": "Flow section 9 frozen Deferred markers; master critical path J.",
      "source_contracts": ["RELEASE_STATUS.md"],
      "domains": ["RELEASE"],
      "flow_refs": ["DIST-PLATFORM-PACKAGE-001", "DIST-SIGNING-001", "DIST-NOTICES-001", "DIST-CLEAN-MACHINE-001", "DIST-UPDATER-001", "DIST-PUBLICATION-001"],
      "scope": "Out of scope",
      "owner_files": "Distribution owners (frozen)",
      "dependencies": ["A future distribution product goal"],
      "automated_proof": { "status": "not-passing", "command": "Not applicable until reopened; completion-ledger checker enforces the six markers stay Deferred-kind.", "expected_count": null },
      "native_proof": { "status": "not-applicable", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Frozen out-of-scope rows have no scheduled native surface in the active Windows-local target." },
      "hardware_external_proof": { "status": "not-applicable", "device_topology_duration": "", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Frozen out-of-scope distribution work schedules no hardware acceptance." },
      "status": "Not started",
      "commit": "",
      "residual_risk": { "classification": "non-blocking", "disposition": "Frozen rows are excluded from the 71-item active denominator by the flow authority." },
      "nonclaim": "This row does not claim any signing, clean-machine, updater, or publication capability, and reopening is not scheduled.",
      "decision_ids": [],
      "risk_ids": ["R-RELEASE-LEGAL-001"],
      "evidence_ids": []
    },
    {
      "id": "COV-CROSS-PLATFORM-DEFERRED-001",
      "requirement": "macOS/Linux real-machine display/media/audio/save-reload/package acceptance and macOS warning enforcement stay deferred outside the Windows target while remaining recorded follow-on work.",
      "failure_behavior": "Cross-platform claims are rejected unless the deferral is lifted with its own acceptance evidence.",
      "source": "Flow WARN-MACOS-001 and PLATFORM-MAC-LINUX-001 Deferred markers; broader Windows-only reference scope.",
      "source_contracts": ["RELEASE_STATUS.md", "AGENTS.md"],
      "domains": ["RELEASE", "MIGRATION", "VIDEO-SOURCE", "AUDIO-LIVE"],
      "flow_refs": ["WARN-MACOS-001", "PLATFORM-MAC-LINUX-001"],
      "scope": "Deferred",
      "owner_files": "Cross-platform acceptance owner (deferred)",
      "dependencies": ["Windows-only product scope decision", "requiredMatrixComplete=false acknowledgment"],
      "automated_proof": { "status": "not-passing", "command": "macOS warning rows are pending external evidence; no cross-platform checker is enforced in the active target.", "expected_count": null },
      "native_proof": { "status": "not-applicable", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Deferred platform rows schedule no native run in the active Windows-only target." },
      "hardware_external_proof": { "status": "not-applicable", "device_topology_duration": "", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "Deferred platforms schedule no physical acceptance in the active target." },
      "status": "Not started",
      "commit": "",
      "residual_risk": { "classification": "non-blocking", "disposition": "Two macOS warning rows keep requiredMatrixComplete false without blocking the Windows target." },
      "nonclaim": "This row does not claim macOS/Linux display, media, audio, save/reload, package, or warning enforcement.",
      "decision_ids": [],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-COMPARE-VIDEO-001",
      "requirement": "SynapseRack pinned benchmark produces measured V01-V17 evidence with pinned build/license/hardware, preserving first failures and unmeasured rows.",
      "failure_behavior": "Parity is never inferred from counts or loopback; unmeasured tasks remain explicitly unmeasured.",
      "source": "Flow COMPARE-PINNED-001 marker; SynapseRack benchmark document; DEC-COMPARE-001.",
      "source_contracts": ["qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md"],
      "domains": ["COMPARE-VIDEO"],
      "flow_refs": ["COMPARE-PINNED-001"],
      "scope": "External acceptance",
      "owner_files": "Comparative acceptance owner",
      "dependencies": ["DEC-COMPARE-001 pinning", "Verified Windows executable"],
      "automated_proof": { "status": "not-passing", "command": "Comparison capture tooling planned with the benchmark protocol; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No pinned-build comparison run is recorded." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Pinned SynapseRack version/license/reference hardware task comparisons.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "Not started",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Acceptance plan only; measurements absent." },
      "nonclaim": "This row does not claim any SynapseRack parity result.",
      "decision_ids": ["DEC-COMPARE-001"],
      "risk_ids": [],
      "evidence_ids": []
    },
    {
      "id": "COV-COMPARE-LIGHTING-001",
      "requirement": "Daslight task/parity comparisons at pinned version/license/hardware/content preserve first failures and unmeasured rows; external Art-Net visualizer boundary (DEC-3D-001) and no-standalone-programming boundary (DEC-STANDALONE-001) hold.",
      "failure_behavior": "Semantic/profile or physical-output differences are recorded as failures, never averaged away.",
      "source": "Flow COMPARE-PINNED-001 marker; DASLIGHT_PARITY_COMPLETION_PLAN and verdict documents.",
      "source_contracts": ["qa/DASLIGHT_PARITY_COMPLETION_PLAN.md"],
      "domains": ["COMPARE-LIGHTING"],
      "flow_refs": ["COMPARE-PINNED-001"],
      "scope": "External acceptance",
      "owner_files": "Comparative lighting acceptance owner",
      "dependencies": ["DEC-COMPARE-001 pinning", "Real fixture/serial DMX availability confirmed 2026-08-22"],
      "automated_proof": { "status": "not-passing", "command": "Comparison capture tooling planned with the parity plan; none authored for this row yet.", "expected_count": null },
      "native_proof": { "status": "not-run", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": "No pinned Daslight comparison run is recorded on current source." },
      "hardware_external_proof": { "status": "not-run", "device_topology_duration": "Pinned Daslight task comparisons over real serial/Art-Net DMX with synchronized output evidence.", "artifact_hash": null, "raw_evidence_paths": [], "na_reason": null },
      "status": "In progress",
      "commit": "",
      "residual_risk": { "classification": "P2", "disposition": "Partial measured evidence exists; unmeasured tasks and semantic/profile evidence remain." },
      "nonclaim": "This row does not claim Daslight parity from counts or loopback evidence.",
      "decision_ids": ["DEC-COMPARE-001", "DEC-3D-001", "DEC-STANDALONE-001"],
      "risk_ids": [],
      "evidence_ids": []
    }
  ],
  "q2_decisions": [
    { "id": "DEC-3D-001", "decision": "Built-in 3D versus external visualizer: external Art-Net visualizer is the product boundary.", "state": "Accepted", "date": "2026-08-13", "approver": "Operator/product owner via master roadmap section 21 initial decision state", "alternatives": ["Built-in 3D stage visualizer rendering pipeline"], "consequences": "No internal 3D renderer is shipped; visualizer interoperability rides the Art-Net output contract and its acceptance matrices.", "linked_q1_ids": ["COV-OUTPUT-LOCAL-001", "COV-COMPARE-LIGHTING-001"] },
    { "id": "DEC-STANDALONE-001", "decision": "Standalone hardware scene programming stays out of current PC-software scope unless a separate hardware product is approved.", "state": "Out of scope", "date": "2026-08-13", "approver": "Operator/product owner via master roadmap section 21 initial decision state", "alternatives": ["Shipping embedded standalone programming modes"], "consequences": "Lighting authoring targets the PC show workflow; comparator parity excludes standalone-console features.", "linked_q1_ids": ["COV-PATCH-GDTF-001", "COV-COMPARE-LIGHTING-001"] },
    { "id": "DEC-FILE-ID-001", "decision": "Media file identity on non-Windows: implement equivalent coherence or narrow the support claim.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending cross-platform scope decision", "alternatives": ["Content-hash identity everywhere", "Path-plus-metadata identity with narrowing"], "consequences": "Non-Windows media safety claims stay blocked; Windows target unaffected.", "linked_q1_ids": ["COV-MEDIA-T1-001", "COV-MIGRATION-001"] },
    { "id": "DEC-ASIO-001", "decision": "ASIO license and artifact: GPLv3-separated artifact or Steinberg proprietary SDK agreement; normal MIT distribution remains WASAPI-only.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending ASIO licensing owner", "alternatives": ["GPLv3-separated bridge artifact", "Signed Steinberg SDK agreement", "Remain WASAPI-only forever"], "consequences": "ASIO stays a separately licensed, non-default artifact; installer/updater separation and notices depend on this decision.", "linked_q1_ids": ["COV-AUDIO-LIVE-001"] },
    { "id": "DEC-SHOW-TRANSPORT-001", "decision": "ShowClock transport/discovery/versioning policy.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending ShowClock architecture owner", "alternatives": ["Auth UDP multicast discovery", "TCP brokered pairing", "Hybrid with slew tiers"], "consequences": "Implementation and any two-PC claim stay blocked until frozen.", "linked_q1_ids": ["COV-SHOWCLOCK-001"] },
    { "id": "DEC-SHOW-FENCE-001", "decision": "Witness/shared-lease/physical-interlock/manual-Hold fencing against split-brain output.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending ShowClock architecture owner", "alternatives": ["Strongly consistent witness service", "Physical interlock line", "Manual Hold-only policy"], "consequences": "Automatic failover claims remain forbidden while open.", "linked_q1_ids": ["COV-SHOWCLOCK-001"] },
    { "id": "DEC-CLOCK-MASTER-001", "decision": "ShowClock/audio/video/device master-clock policy.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending audio architecture alignment", "alternatives": ["Audio PTS master", "ShowClock master with device slew", "Per-domain masters with declared arbitration"], "consequences": "Authored audio, recording, and Timeline scheduling semantics inherit this policy once frozen.", "linked_q1_ids": ["COV-AUDIO-AUTHORED-001", "COV-SHOWCLOCK-001"] },
    { "id": "DEC-RECORD-OWN-001", "decision": "Recording ownership and two-PC behavior/format.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending recording owner", "alternatives": ["Single-owner recording with takeover receipts", "Shared ownership with arbitration"], "consequences": "Recording terminal machine and artifact import rules stay provisional.", "linked_q1_ids": ["COV-RECORDING-001"] },
    { "id": "DEC-P2-001", "decision": "Which P2s block release: security/migration/legal/platform/recovery/advertised-claim P2s block per master section 4.7.", "state": "Accepted", "date": "2026-08-13", "approver": "Operator/product owner via master roadmap section 21 initial decision state", "alternatives": ["All P2s blocking", "Only P0/P1 blocking"], "consequences": "Release candidacy math and Q5 query use the enumerated blocking classes deterministically.", "linked_q1_ids": ["COV-RELEASE-001"] },
    { "id": "DEC-COMPARE-001", "decision": "Pinned Daslight/SynapseRack versions, licenses, tiers, and reference hardware before comparative measurement.", "state": "Open", "date": "2026-08-13", "approver": "Unassigned pending comparative acceptance owner", "alternatives": ["Latest consumer tiers", "Pinned production licenses"], "consequences": "All comparison rows stay unmeasured until pins are recorded.", "linked_q1_ids": ["COV-COMPARE-VIDEO-001", "COV-COMPARE-LIGHTING-001"] },
    { "id": "DEC-AI-ARCH-001", "decision": "AI control architecture: one in-process authoritative registry with optional localhost-only MCP/API sidecar; adapters contain no domain authority.", "state": "Accepted", "date": "2026-08-13", "approver": "Operator/product owner via AI control-plane roadmap architecture section", "alternatives": ["Distributed adapter authority", "Out-of-process authority broker"], "consequences": "AI0-AI8 lanes inherit single-registry admission, receipts, and audit; sidecar exposure stays localhost-bounded until AI4/AI5 accept otherwise.", "linked_q1_ids": ["COV-AI-CONTROL-001"] },
    { "id": "DEC-AI-CONSENT-001", "decision": "Unattended disruptive automation: external principals start safe; R4/R5 require locally approved, fingerprint-bound, short-lived single-use consent unless a later explicit product decision narrows the class.", "state": "Accepted", "date": "2026-08-13", "approver": "Operator/product owner via AI control-plane roadmap risk/capability/consent model", "alternatives": ["Standing grants for trusted hosts", "Per-action manual approval only"], "consequences": "Kill switch, revocation, and fail-safe controller-loss behavior are mandatory AI4 gates; the retired Raw Input challenge stays retired.", "linked_q1_ids": ["COV-AI-CONTROL-001", "COV-SECURITY-001"] }
  ],
  "q3_risks": [
    { "id": "R-MEDIA-TERM-001", "severity": "P1", "escalation": null, "reproduction": "Media publish/history path with lost command reply during Begin-window mutations.", "affected_scope": "Media project data, published thumbnails, operator history truth", "likelihood": "undetermined", "owner": "Media authority owner", "mitigation": "Closed for the current Windows Media T1 tranche via client-operation receipts and terminal-truthful history.", "blocking_milestone": "Closed for Media T1 Windows tranche; revisit for cross-platform scope.", "proof_needed": "Focused transaction tests plus A8 native workflow matrix.", "status": "Closed", "closing_commit": "aad9172", "closing_note": "Closing evidence includes companion commit 14eeeb2 and final reviews per master Q3 register.", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-MEDIA-T1-001"] },
    { "id": "R-TX-BEGIN-001", "severity": "P1", "escalation": null, "reproduction": "Begin side effect whose reply is lost before renderer acknowledgement.", "affected_scope": "Any project mutation, history entries, retry semantics", "likelihood": "undetermined", "owner": "Project transaction owner", "mitigation": "Client-operation receipt/query/adopt path with owner-incarnation ABA rejection closes the Windows E1 tranche.", "blocking_milestone": "Closed for E1; reopen criteria tied to new mutation surfaces.", "proof_needed": "project_transaction focused tests plus independent P0/P1/P2-zero review.", "status": "Closed", "closing_commit": "", "closing_note": "Closed by reviewed E1 implementation checkpoint rather than a single commit hash; see flow sections 23 and 30.", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-PROJECT-TX-001"] },
    { "id": "R-MEDIA-COMPAT-001", "severity": "P1", "escalation": null, "reproduction": "Old raw IPC carrying wrong authority/hash attempting atomic media publication.", "affected_scope": "Media authority, cache, engine patch state", "likelihood": "undetermined", "owner": "Media authority owner", "mitigation": "Command-bound authority/hash checks reject legacy raw IPC; closed for current Windows Media T1.", "blocking_milestone": "Closed for Media T1 Windows tranche.", "proof_needed": "Command tests and A8 acceptance record.", "status": "Closed", "closing_commit": "4173e35", "closing_note": "See master Q3 register closure entry.", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-MEDIA-T1-001"] },
    { "id": "R-FILE-ABA-001", "severity": "P1", "escalation": "Conditional on non-Windows support", "reproduction": "Path/file image coherence violation when a file changes identity between stat and read on case-insensitive or remapped filesystems.", "affected_scope": "Media assets, relink correctness, migrated projects", "likelihood": "undetermined", "owner": "Media/file-identity owner", "mitigation": "Code-side coherence implementation closed by 8cb0459; macOS/Linux execution and product-support decision pending.", "blocking_milestone": "Blocks non-Windows support claims only.", "proof_needed": "Cross-platform execution proof plus DEC-FILE-ID-001 disposition.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-MEDIA-T1-001", "COV-MIGRATION-001"] },
    { "id": "R-PATCH-ATOMIC-001", "severity": "P1", "escalation": null, "reproduction": "Profile ancillary load or Engine Patch/Repair failing mid-publication.", "affected_scope": "Fixture profiles, allocator, live output, cache", "likelihood": "possible", "owner": "Patch/GDTF owner", "mitigation": "Whole-batch prevalidation and rollback path implemented in D2-era lanes; residual audit continues.", "blocking_milestone": "Blocks D completion.", "proof_needed": "Atomicity regression suite plus engine repair focused tests.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-PATCH-GDTF-001"] },
    { "id": "R-OUTPUT-SWAP-001", "severity": "P1", "escalation": null, "reproduction": "Project or Take Over replacement racing external outputs (NDI/Spout/DMX/display windows).", "affected_scope": "Live lighting/video output, leases, downstream venues", "likelihood": "possible", "owner": "Output-ownership owner", "mitigation": "Candidate/orphan receipts, generation fences, explicit re-Arm implemented code-side; physical proof outstanding.", "blocking_milestone": "Blocks distributed/venue completion.", "proof_needed": "Physical replacement/reconnect matrices across all output kinds.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-OUTPUT-LOCAL-001"] },
    { "id": "R-SHOW-SPLIT-001", "severity": "P1", "escalation": null, "reproduction": "Two-node partition causing dual output after rejoin.", "affected_scope": "Both machines' full output, venue audiences", "likelihood": "possible", "owner": "ShowClock architecture owner", "mitigation": "Not implemented; witness/fence decision must precede any distributed claim.", "blocking_milestone": "Blocks automatic failover claim entirely.", "proof_needed": "Real-switch partition/rejoin rehearsal with zero simultaneous output.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-SHOWCLOCK-001"] },
    { "id": "R-AUDIO-OWN-001", "severity": "P1", "escalation": "Product boundary risk", "reproduction": "Audio/MIDI/recording ownership conflict during device swap or two-PC operation.", "affected_scope": "Live audio, recordings, MIDI routing", "likelihood": "possible", "owner": "Audio architecture owner", "mitigation": "Master-clock and ownership decisions tracked as DEC-CLOCK-MASTER-001/DEC-RECORD-OWN-001.", "blocking_milestone": "Blocks two-PC/full-output claims.", "proof_needed": "Decision freeze plus device-swap fault drill.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-AUDIO-AUTHORED-001", "COV-AUDIO-LIVE-001", "COV-RECORDING-001"] },
    { "id": "R-RELEASE-HW-001", "severity": "P0-Release", "escalation": null, "reproduction": "Attempting public release without the required physical acceptance matrix.", "affected_scope": "Public release truthfulness, operator venues", "likelihood": "near-certain", "owner": "Release/hardware acceptance owner", "mitigation": "Q5 completion query forbids completion while any physical row is unaccepted.", "blocking_milestone": "Blocks public release.", "proof_needed": "Full K-matrix physical evidence bound to exact artifacts.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-RELEASE-001"] },
    { "id": "R-RELEASE-LEGAL-001", "severity": "P0-Release", "escalation": null, "reproduction": "Signing/notarization/license obligations unmet at publication time.", "affected_scope": "Distribution legality, ASIO separation, notices", "likelihood": "near-certain", "owner": "Release legal owner", "mitigation": "Distribution rows frozen/out-of-scope keep publication blocked; ASIO license decision pending.", "blocking_milestone": "Blocks public release.", "proof_needed": "Signing/notarization/SBOM results plus ASIO artifact decision.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-RELEASE-001", "COV-DISTRIBUTION-DEFERRED-001"] },
    { "id": "R-MIGRATION-001", "severity": "P2 release-blocking", "escalation": null, "reproduction": "Unified compatibility/corruption proof absent across .sdc/template/cache/protocol versions.", "affected_scope": "User projects, upgrades, support escalations", "likelihood": "possible", "owner": "Migration owner", "mitigation": "Per-feature tests exist; unified corpus/fuzz/golden matrix pending.", "blocking_milestone": "Blocks supported-upgrade claim.", "proof_needed": "Golden migration corpus plus hostile-input fuzz results.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-MIGRATION-001"] },
    { "id": "R-SECURITY-001", "severity": "P2 release-blocking", "escalation": null, "reproduction": "Unified remote/parser/update threat model gaps exploited via LAN or crafted files.", "affected_scope": "Network-exposed surfaces, project files, updater trust", "likelihood": "possible", "owner": "Security owner", "mitigation": "Foundation controls exist per-domain; unified model and release-blocking proof pending.", "blocking_milestone": "Blocks public-network/security claims.", "proof_needed": "Unified threat-model proof and adversarial exercises.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-SECURITY-001"] },
    { "id": "R-AI-BYPASS-001", "severity": "P1", "escalation": "P0-Code when externally reachable", "reproduction": "Adapter or GUI mutation bypassing the registry/authority/receipt path.", "affected_scope": "Every project/output mutation, audit truth", "likelihood": "undetermined", "owner": "AI control-plane owner", "mitigation": "Route-proof scans and EngineCommand classifier constrain known sinks; AI0 inventory gate will fail-close unclassified mutations.", "blocking_milestone": "Blocks full AI-driven claim and external R2+ release.", "proof_needed": "AI0 coverage gate plus adversarial parity proof.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-AI-CONTROL-001", "COV-SECURITY-001"] },
    { "id": "R-AI-SAFETY-001", "severity": "P0-Code", "escalation": null, "reproduction": "External disruptive/output/file action lacking bound consent, revocation, or fail-safe controller-loss behavior.", "affected_scope": "Operator venues, files, physical output", "likelihood": "undetermined", "owner": "AI safety owner", "mitigation": "DEC-AI-CONSENT-001 fixes safe-by-default external principals; AI4 must implement grants/revocation/kill switch before enablement.", "blocking_milestone": "Blocks external R4/R5 enablement.", "proof_needed": "Consent service proof including revocation and controller-loss fail-safes.", "status": "Open", "closing_commit": "", "closing_note": "", "last_reviewed": "2026-08-25", "linked_q1_ids": ["COV-AI-CONTROL-001", "COV-SECURITY-001"] }
  ],
  "q4_evidence": [
    { "id": "EV-MEDIA-T1-A7-2026-08-13", "scope": "Ordered automated Media Asset A7 gate and current native build hash", "kind": "historical-evidence", "status": "accepted-historical", "date": "2026-08-13", "command": null, "exit_code": null, "assertion_count": null, "ignored_count": null, "warning_count": null, "artifact_hash": null, "artifact_na_reason": "Exact artifact hashes are recorded inside the linked evidence document.", "raw_evidence_paths": ["qa/MEDIA_ASSET_T1_A7_EVIDENCE_2026-08-13.md"], "currency_nonclaim": "Historical A7 tranche evidence only; it cannot be used as current-source acceptance.", "linked_q1_ids": ["COV-MEDIA-T1-001"] },
    { "id": "EV-MEDIA-T1-A8-2026-08-13", "scope": "Full native Media Asset workflow matrix plus rebuilt thumbnail/hover supplement", "kind": "historical-evidence", "status": "accepted-historical", "date": "2026-08-13", "command": null, "exit_code": null, "assertion_count": null, "ignored_count": null, "warning_count": null, "artifact_hash": null, "artifact_na_reason": "Exact artifact hashes are recorded inside the linked evidence document.", "raw_evidence_paths": ["qa/MEDIA_ASSET_T1_A8_NATIVE_EVIDENCE_2026-08-13.md"], "currency_nonclaim": "Accepted for the declared historical Windows tranche only; it cannot be cited as current-source acceptance and non-Windows execution remains separate.", "linked_q1_ids": ["COV-MEDIA-T1-001"] },
    { "id": "EV-ALPHA11-D4-2026-08-25", "scope": "Alpha.11 D4 Stage software/native integration acceptance and main-checkout launch parity", "kind": "historical-evidence", "status": "accepted-historical", "date": "2026-08-25", "command": null, "exit_code": null, "assertion_count": null, "ignored_count": null, "warning_count": null, "artifact_hash": "1B010C40242A5C7DD7A2797EAC1ECA2D31BCACE4455BA57C7F935611075B582B", "artifact_na_reason": null, "raw_evidence_paths": ["qa/SYNDOCAL_ALPHA11_MAIN_INTEGRATION_CHECKPOINT_2026-08-25.md"], "currency_nonclaim": "Bound to the identified alpha.11 executable; this historical record cannot be promoted to current-source acceptance without a fresh exact-process native gate.", "linked_q1_ids": ["COV-STAGE-001", "COV-PROJECT-TX-001"] },
    { "id": "EV-Q1Q4-COVERAGE-INFRA-2026-08-25", "scope": "Fail-closed Q1-Q4 coverage infrastructure: validator, isolated-fixture self-test, master mirror parity, and full Q0/Flow reference coverage", "kind": "current-source-automated", "status": "accepted-current", "date": "2026-08-25", "command": "node app/scripts/check-q1-q4-ledger.mjs && node app/scripts/test-check-q1-q4-ledger.mjs", "exit_code": 0, "assertion_count": 46, "ignored_count": 0, "warning_count": 0, "artifact_hash": null, "artifact_na_reason": "Deterministic script evidence has no binary artifact; the ledger, validator, and self-test sources are the inspectable artifacts and are re-runnable byte-for-byte.", "raw_evidence_paths": ["qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md", "qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md", "qa/SYNDOCAL_COMPLETION_LEDGER.json", "app/scripts/check-completion-ledger.mjs", "app/scripts/test-check-completion-ledger.mjs"], "currency_nonclaim": "Automated documentation/tooling proof only; it is not current-source acceptance of any domain and does not constitute native, hardware, external, soak, or distribution acceptance, nor does it close COMP-Q1-Q4-001.", "linked_q1_ids": ["COV-Q1Q4-INFRA-001"] }
  ]
}
```

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

## 24. Historical resume protocol — do not execute

This protocol records the 2026-08-13 checkpoint only. It is not a current
resume entry and grants no implementation authority. Follow `AGENTS.md` and the
active completion flow; consult the post-alpha.10 snapshot for checkpoint evidence
and frozen ownership only. Their current model hierarchy, ownership, validation,
cleanup, and continuation rules supersede the historical instructions below.

At the next session or agent handoff at that historical checkpoint:

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

This section records the active-train override as it stood on 2026-08-21; it is
not a current entrypoint. The post-alpha.10 pause request was later rescinded
before promotion. `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` is the active work
authority; the post-alpha.10 snapshot and 2026-08-19 handoff are checkpoint and
historical evidence logs.

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

## 35. 2026-08-23 alpha.7 D2/UI/Guide integration checkpoint

The next distributed development artifact is `1.2.0-alpha.7`; alpha.6 remains an
immutable accepted E4 artifact. D2 PATCH/GDTF Repair is independently green at
P0/P1/P2 zero through its registered production transaction path, including
whole-batch prevalidation, Published ACK classification, allocator/runtime
rollback, reply-loss recovery, exact lifecycle truth, and one history result.
The final backend hashes are main
`5B91759B9B4789AF17B0376220F094B08603D0AD271DEF9B5CD67AA46103215D`, Engine
`03685241C9FC35B12F7FDA6A880ADEB64D3EF092F60B63717E8325ACEC0C0C6F`, and
control plane
`A0864AB959688F5B857F5F49BF985F82F64036F145C3445FDCFA113E9B480750`.
The focused backend/frontend matrix and no-default warning gate are green with
zero first-party warnings.

Lighting/Video/Timeline geometry and screenshot evidence passed four viewports;
Timeline operator passed five viewports; Timeline Sources retained zero outer/body
scroll; and the Guide perceptual boundary passed at 150 ms. These checks remain
automated evidence only and do not replace the final native maximized-window,
display, audible-device, DMX/MTC/DJ, ASIO, hardware, or soak acceptance.

D2 advances the current Windows denominator to 19/71 (26.8%). D3 is the next
dependency-ordered project-mutation tranche. No alpha.7 native build or GUI
verification is claimed here; root owns the synchronized native build and final
integrated acceptance.

Release metadata self-check, the locked no-default Rust check, the Windows
frontend warning ratchet, and diff-check are green. First-party warnings are zero
for both configurations run; no native warning or UI claim is inferred because
the alpha.7 release build has not yet run.

## 36. 2026-08-23 alpha.7 Control upper-workspace native/UI closure

The reported blank/clipped Control workspaces are repaired without a density
reduction. Lighting again exposes its populated Scene Matrix upper owner; Video's
Media Library occupies the upper grid and scrolls within its body; Timeline uses
a fixed header plus remaining-height upper-content row; and Sources retains a
full-height outer shelf with body-only scrolling. Tools/Live Mixer disclosure,
Timeline expansion, and the ordered Escape stack are preserved.

The dedicated browser acceptance is green at exactly 3840x2160, 2560x1440,
1920x1080, and 1280x720 across 28 captured states. 960x640 remains the product
minimum configuration only. 860x520 and 1366x768 are historical supplemental
inputs and do not replace the current four-size matrix. Browser evidence remains
separate from native evidence.

The exact alpha.7 no-bundle executable, SHA-256
`00253F26A8D3A933172D7B07923E430B455CA18F98E439CE45C5B62405F86EF4`,
launched as exactly one responsive maximized `Syndocal` window. Native 1920-class
screenshots cover Lighting, Video, Video Import, Timeline/Sources, Tools,
expanded mode, and ordered Escape restoration. 2048x1104 is supplemental. Native
2560x1440 and 1280x720 were not accepted because a safe verified move to those
active monitors was unavailable, while 3840x2160 was connected but not active as
a desktop mode. Those native matrix rows remain open.

Guide timing is fixed and proven at click target minus 7,200 frames (150 ms at
48 kHz). D2 remains complete and the exercised Windows warning rows remain at
zero first-party warnings. The two failures in supplemental `check:edit-live`
(scene-identity badge color and the old <=86 px mode-tab expectation) predate and
are unrelated to this upper-workspace change; they are not closed by shrinking
shared controls.

The next dependency-ordered completion work is physical multi-display VJ, DMX,
MIDI, DJ Link plus Stream Deck Pedal, audible click/Guide, the remaining ASIO
matrix/licensing/fault paths, and the integrated long soak. This checkpoint does
not advance those hardware denominators or claim whole-product completion.

## 37. Historical 2026-08-23 pause/resume point — do not execute

The then-current remaining-task inventory and dependency-ordered resume sequence
were in `qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-23.md`. That file and the later
post-alpha.10 requested-pause snapshot are historical checkpoint evidence after
the operator's 2026-08-25 continuation instruction. Their old DJ/D3/alpha.8
resume orders must not be used. The accepted denominator at that historical
checkpoint remained 19/71 (26.8%).

## 38. 2026-08-24 pre-alpha.8 software/source closure checkpoint

Branch `codex/syndocal-v1.2` remains based at
`23f350c366ede2fdffcfbf3232e18112eada51ea`; product metadata is still
`1.2.0-alpha.7`. D3 is now accepted at the software/source boundary. Its final
fixed-hash review returned P0 0 / P1 0 / release-blocking P2 0, with full Engine
822 passed / 2 ignored, I/O 148 passed / 1 ignored, and Syndocal no-default 951
passed / 5 ignored. The frozen source hashes are main
`2CC94D3E5307E6BBC026F1EC815ABC02BFF47E987D62526446AFD1059984E6C7`, Engine
`F2DCE4D4F6E4B9CA4E15DB722608461D9D0ADBD9948FB41D5796FD4B94E1B2F7`, and I/O
`E552FEA70D017BBAE40B534A6D854BE1C1B20AFFE111B9E733E0C19FA7A1E8B4`.

The Tauri flat-payload adapter covers 51 commands (39 common + 11 video + 1
repair) with six passing focused tests and fail-closed Raw/invalid-body handling.
The warning-ratchet library/test hashes are
`55D2388C3EA244F3C58B207E651E56F33DB10B6E76BD404A2943497E384926AA` and
`3391F2F149ADCED3B5641E48E326680686B8D243389EAFC7E2DCC3799AEA7E55`;
independent review found no release-blocking issue and the exercised warning counts
are total/first-party 0/0.

The frozen frontend chain passed routing 133/29/33/402, 417 invokes, backend
478/311/133, project transaction/authority, E3/E4, output ownership/control/runtime,
the Scene Matrix strip plus full five-viewport matrix, DVC DMX 35 assertions,
localization 3556/3556, and worktree/cached diff checks. The Scene Matrix test now
measures Playback Executor Bank labels on the actual Live/Playback surface instead
of the intentionally unmounted Edit surface; both it and the DVC DMX strengthening
received fixed-hash reviews without a release-blocking finding.

The companion DJ Agent `Beta` commit
`6c4f4328a6866d9d48022bd8ee20a7887c9de851` is pushed and clean with 54 tests,
16 syntax checks, zero warnings, and packaged `server.exe` SHA-256
`339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`.
This does not accept physical rekordbox/DJ Link/State Sync/Pedal operation.

The accepted denominator remains exactly 19/71 (26.8%): no source-only result is
promoted to native or physical acceptance. The intended source manifest is
`qa/artifacts/source-freeze/2026-08-24-alpha7-pre-alpha8-source-freeze.sha256`.
Next is one synchronized alpha.8 version change plus `check:release`, then the exact
native/WER/reload, current-PC hardware, ASIO/DJ, and one-hour soak gates.

## 39. 2026-08-24 Step 6 alpha.8 product-metadata checkpoint

The current product train is now `1.2.0-alpha.8` at all 20 authoritative
locations. An exact coordinate audit passed 20/20 while retaining the historical
alpha.7 evidence. Release metadata, the eight-assertion Tauri build-wrapper gate,
and locked Cargo metadata passed; all eight first-party workspace packages resolve
to `1.2.0-alpha.8`. The Windows frontend TypeScript/Vite warning ratchet also
rebuilt successfully with total/first-party/third-party warnings 0/0/0 and all
three output markers present.

This is a product-metadata-only checkpoint. No project/schema/API/ABI boundary
changed, no native alpha.8 executable has yet been built or launched, and no
WER/reload, display, physical hardware, ASIO/DJ, or soak row is inferred. The
accepted denominator remains exactly 19/71 (26.8%); next is the required exact-path
native build and native/physical acceptance sequence.

## 40. 2026-08-24 alpha.8 native crash-closure checkpoint

The branch remains `codex/syndocal-v1.2` at base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. Product metadata is
`1.2.0-alpha.8`, synchronized at 20/20 authoritative points. The frozen frontend
chain is 25/25 green; final checker
`app/scripts/check-viewport-containment.mjs` is
`E0FA2A5E0FA27A97CCCDCAE842BF6E6104270E85EBE8C1BE4FA572FE41915118`, with Gate
12 and Gate 13 passing twice across all five viewports. Independent Sol review is
P0=0/P1=0/release-blocking P2=0. The App TDZ repair and native reload harness
hashes are `E62439B070BC210CDE4FE08DBDBAABBEF3E16C738A14F3925B3D9DB9AB60F241`
and `BBFDF9D65742964625DF127D581AA78E54F7B6638DCED67CCD08ADBFADBB2E42`.

Native build attempt 1 at 10:20 failed because Cargo selected
`C:\Program Files\Git\usr\bin\link.exe` despite the parent PATH showing MSVC;
this was a toolchain-selection failure before source compilation, not a source
failure. Setting `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to the absolute
VS2022 14.43.34808 linker fixed it. The required
`pnpm --dir app tauri build --no-bundle` passed in 4m09s with zero
warning-shaped lines. The 57,491,456-byte alpha.8 executable hash is
`86423537C3F1DC8B3BBD602E7CC51F58F63DBC4CA05AEE2DAA3BCA485E8F6882`; its
19,582,976-byte PDB hash is
`776DB6E8099DA6F996E2411CD81F3DDB1D3112EF09B9587582E9D352595A451B`.
FileVersion and ProductVersion both read `1.2.0-alpha.8`.

Launch produced exact PID `126188`, exactly one responsive maximized Syndocal
window at 1920x1032, and CDP port 9333. With WER baseline
`2026-08-24 10:27:02 JST`, the fixed harness artifact
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-final/native-reload-stress-100.json`
(`99694A08414B52C8C35B567146E3E98006EF8BD2FF81CF1ED835F6B5697C004D`) passed
100/100 reloads, 100 unique origins, exit 0, total 45.884s, min 319ms, p50
439.1ms, p95 595.7ms, max 717.3ms, runtime exceptions 0, and log issues 0.
Only the two expected Tauri reload warning messages appeared (2,134 each).
Post-run PID/window responsiveness remained intact; WER Event and Reliability
deltas were zero. The measured process snapshot was WS 83,218,432 bytes, private
52,027,392 bytes, 538 handles, and 48 threads.

Five active physical displays are confirmed. D3 `MSI MPG321UX OLED` is the
physical 3840x2160 4K display and is logically 2560x1440 at 150% scaling. The
required native route `D5 -> D2 -> D3 -> D1 -> D6` remains open. Daslight PID
72476/UDP 6454 and Ableton PID 103764/UDP 20909 were not touched. DJ/rekordbox,
physical MIDI/Pedal, ASIO, fault/recovery matrices, and one-hour soak remain
pending or fail-closed where not physically proven; this checkpoint does not
advance the 19/71 denominator to whole-product completion.

The disk audit measured 343.777 GiB for the workspace, 343.04 GiB for `target`,
238.15 GiB for `target/debug` (149 GiB incremental), and 90.36 GiB for the old
named QA target. No cleanup was performed. If subsequent source/document changes
invalidate the frozen frontend evidence, rerun the final frontend chain; otherwise
the next actions are checkpoint commit/push and verified five-display native UI
acceptance. Full hardware, ASIO, DJ, release, and soak claims remain gated.

## 41. 2026-08-24 alpha.8 current-source build/reload and physical-route checkpoint

The current branch is `codex/syndocal-v1.2` at pre-commit base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`, with product metadata synchronized
at `1.2.0-alpha.8`. The current-source final frontend chain is 25/25 green in
`target/qa/alpha8-current-source-final-gates-20260824-114918`; the independent
fixed-hash review is P0=0/P1=0/release-blocking P2=0. Current source identifiers
are CSS `4974a2f828b8b8bd1c9fbe43390d97d5d6702179`, upper-workspace checker
`73a43ecfb2c6f10c07fb638f84f50375de5213a0`, and Scene Matrix checker SHA-256
`F164CD5B5C6C130E1D27B21C6A04CB1C361CEE3346F08FA9DFF77DE522C5FE11`. Exercised
warning counts are total/first-party/third-party 0/0/0.

The authoritative staged-source inventory is
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`.
It contains 103 payload records and excludes only its own manifest envelope from
the payload to avoid recursive self-hashing. The alpha.7 pre-alpha.8 manifest is
retained as historical evidence and is not current alpha.8 source authority.

The exact required native command, run from the Visual Studio Developer Shell,
was `pnpm --dir app tauri build --no-bundle`; it passed using MSVC
14.43.34808/Windows SDK 10.0.26100.0. The current
`target/release/syndocal.exe` is 57,491,456 bytes, SHA-256
`627BE88032774C7FA0A4C3CD3510A7BFB52E8ED0E76884ADD414B9BFD101F459`, and both
file/product versions are alpha.8. The matching PDB is 19,582,976 bytes,
SHA-256 `5548E4F4B2C3CBB38F1881AAA6C9299AE42211616A8A05E9189C3019838F56AB`.
Launch verification found exactly one responsive maximized Syndocal window from
the exact checkout executable.

The current-source reload artifact is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-reload-stress-100-current-source.json`,
SHA-256 `6D12AA14371B1C837DF67DDE80AB44CB1C6B1329F093567B81524E35076FB621`:
100/100 reloads, 100 unique origins, exit 0, total 40.1424 s, min 232.5 ms,
p50 407.3 ms, p95 479 ms, max 530.1 ms, runtime/log issues 0, and WER
Application Error/Reliability deltas 0. The earlier
`2026-08-24-alpha8-final` evidence remains immutable historical pre-CSS output;
the current-source evidence is under `2026-08-24-alpha8-current-source-final`.

The native five-display pane route is complete via the supported
`open_pane_window` placement path: `D5 -> D2 -> D3 -> D1 -> D6`. Every pane was
maximized and each step recorded zero document scroll. Measurements and exact
positions are D5 monitor 1920x1080/scale 1.5/viewport 1280x650 at `{-2465,1731}`;
D2 1920x1080/scale 1/viewport 1920x1009 at `{0,0}`; D3 real physical
3840x2160 4K/scale 1.5/viewport 2560x1370 at `{-3840,-429}`; D1 2560x1440/
scale 1.25/viewport 2048x1082 at `{1920,-364}`; and D6 2560x720/scale 1/
viewport 2560x649 at `{1598,1080}`.

Current-source evidence under
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final`
is `D5-timeline-pane-route.jpg` SHA-256
`37EDB56EA3F81BA014C23A96A1C78F32EEBB68C6B3E4C3D7C6692FD30E2FD0E1`,
`D2-timeline-pane-route.jpg` SHA-256
`0A037F7E6359FBC22C1FC16F297D9A3860157544777764679FABD2C194C20CFD`,
`D3-4K-timeline-pane-route.jpg` SHA-256
`55E83F6AFC32A32DD9BD5AAB187DA659DC0221DB0EF82EAEAA65215327F591CD`,
`D1-timeline-pane-route.jpg` SHA-256
`C22918826714D16E0E02DB5190AFB1E2724CEB387943A0D3FA4FABCDE5DD645E`, and
`D6-timeline-pane-route.jpg` SHA-256
`2336172DB637B9E4F9D4906B3E7D6D781408CD0A4B11D18229974FC92E820340`.
The canonical machine-readable route record is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`,
SHA-256 `41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`.
The expanded D5 evidence `D5-timeline-tools-expanded-fixed.jpg` has SHA-256
`B780CD4CCDA35CC8A8F1148A64B26C91065EB15E33D57A38FDCC04F8B0A3724D` and
records native popup client/scroll 345/345, nested surfaces 335/335, and the
deepest 44px target inside the viewport/popup with hit/focus proof and Escape
focus return. The pane was closed afterward; final verification had one exact
responsive PID 123952, CDP page 1, and a maximized D5 main viewport of 1280x672.

All five detached Timeline pane screenshots still show the non-secret status six
seconds after each pane opened:
`Window 'pane-timeline' has no current project transaction owner registration`.
At the same six-second point, `get_project_authority_bundle` from the pane succeeds
with epoch 0/revision 1, but that separate read does not prove transaction-owner
registration or make the persisted status stale. This evidence accepts placement,
maximize, and containment only; it does not accept transactional pane operation,
warning-clean pane startup, or completed owner registration. The main D5 Timeline
Tools proof remains clean. Keep the owner-registration ordering/status behavior as
an alpha.9 P1 investigation boundary rather than promoting it to an accepted row.

This closes native multi-display pane placement/containment only. It does not
claim display-output playback, fullscreen playback, GPU reset/recovery, physical
DJ Link/rekordbox, Stream Deck Pedal/MIDI, ASIO, real DMX, fault/recovery
matrices, or integrated one-hour soak; the accepted denominator remains exactly
19/71 (26.8%).

The non-destructive capacity audit measured about 343.777 GiB for the workspace,
343.04 GiB for `target`, 238.15 GiB for `target/debug` including about 149 GiB
incremental, and 90.36 GiB for the old named QA target directories. The
`.claude/worktrees` copies are only about 5.3 MiB each and `vendor` is about
0.29 MiB, so the size is overwhelmingly generated Rust build and QA cache output,
not historical source/worktree copies. No cleanup was performed because the
deletion/rebuild-cost tradeoff was not authorized; generated `target` remains
excluded from Git.

## 42. 2026-08-24 alpha.8 push and alpha.9 owner-registration tranche

The reviewed alpha.8 checkpoint is committed and pushed at
`ec9fca4887e079fa61950056d94aca5ab5d65da9` on
`origin/codex/syndocal-v1.2`. The immediate post-push state was HEAD=upstream,
ahead/behind 0/0, staged/unstaged tracked paths 0/0, with only the 19 intentional
historical evidence exclusions left untracked.

The active product train is now synchronized at `1.2.0-alpha.9` across all 20
authoritative coordinates. Release metadata and locked Cargo metadata pass. The
next required dependency is the detached-pane transaction-owner registration
barrier: registration must precede owner-bound startup work, only the main window
may consume startup/queued project opens, failure must re-arm without a retry
loop, and a fresh native pane must be free of the prior six-second owner status.
This row remains in progress and does not change 19/71.

## 43. 2026-08-24 alpha.9 owner-registration accepted

The detached-WebView registration barrier is implemented and independently
reviewed: one sticky fail-closed owner registration precedes owner-bound work,
failure re-arms only from later trusted activity, disposed late results cannot
publish, main-only startup/open consumption is preserved, and latest desired
selection/audio state is serialized. Concurrency, retry, disposal, bootstrap,
transaction, authority, routing, runtime, workspace, localization, format, and
focused Rust gates pass. Frontend warning evidence is 0 first-party and 0
third-party warnings.

The required `pnpm --dir app tauri build --no-bundle` completed with the exact
pinned MSVC Hostx64/x64 linker. Native evidence in
`qa/artifacts/native-owner-registration/2026-08-24-alpha9/` proves all seven
detached panes register before their first owner-gated selection, remain usable
after a main reload, show no owner-registration status/error/crash, and close
back to one responsive maximized main window. The accepted EXE SHA-256 is
`BD4375D09EA09E099E6F24D74DC57B014E60F0C4C124401D1CBA6ACB1EE207FF`.
Five displays were enumerated, including 3840x2160, but no playback claim follows.

Alpha.9 therefore closes its registration-order P1 without changing the
whole-product denominator of 19/71. Alpha.10 owns the now-visible layout P1:
when Stage and Timeline are both detached, remove the unused lower workspace
area, let the upper workspace use the freed height in restored/maximized/
fullscreen modes, retain explicit rejoin routes, and restore saved split ratios
without shrinking existing targets. Stale generated build/cache trees are now a
recurring checkpoint cleanup obligation after the owning commit/push.

## 44. 2026-08-24 alpha.10 pane semantic correction — accepted and paused

The preceding alpha.10 layout-P1 wording is **historical and superseded** for
acceptance: it treated the upper workspace as the survivor and did not encode the
real Stage/Sources topology. Alpha.7 and alpha.8 are immutable historical train
evidence. Alpha.10 was subsequently committed/pushed as
`5c7e19a72a97e20f5ece553594841103990a78a9`; its exact final evidence and pause
state are recorded in the post-alpha.10 handoff.

The current contract is exact. Integrated Control/LIVE has real Timeline above,
real Stage/Groups lower-left, and Sources lower-right. Timeline detachment
removes Timeline only, leaving real Stage/Groups plus Sources at full main-window
height. Stage detachment removes Stage/Groups only, leaving Timeline plus Sources.
With both detached, Sources fill the entire main workspace. Do not accept an
empty lower band or a Timeline Preview as a Stage substitute.

Required proof covers both detach orders and both reverse rejoin orders, native
child-titlebar `X` closure, expand-before-detach and expand-before-rejoin,
outer document/app scroll zero in every state, and preservation of Setup/Patch
Groups. Setup, Edit, and Mixer must remain non-regressed. The final native
measurement rows for maximized/restored/F11 classes and physical 3840x2160 at
150% scaling passed on the corrected committed alpha.10 source with independent
review. The later rebuild's distinct binary hash is recorded separately and is
not used to relabel the same-source physical captures as proof of that exact hash.
