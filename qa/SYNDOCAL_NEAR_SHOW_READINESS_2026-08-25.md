# Syndocal near-show readiness gate — 2026-08-25

## Objective and boundary

The active objective is to make Syndocal safe for the next live show before
expanding the denominator to whole-product completion. This gate does not lower
the product or release requirements. It establishes the smallest real operator
path that can author, persist, recover, and run the next show without relying on
a browser fixture, a silent fallback, or an obsolete authority path.

This document is the authoritative, complete near-show objective. Chat summaries,
older alpha handoffs, green browser fixtures, and narrower tranche reports do not
replace or reduce it. A newly discovered defect remains part of the objective
until it is repaired and proven; it is not removed merely because it was absent
from the initial list.

The authoritative working branch at the start of this gate was
`codex/syndocal-v1.2` at `42339b5a41e182c4b324e80ff17882f5dac3a18e`, equal to
`origin/codex/syndocal-v1.2`. The working tree is intentionally dirty across
owned parallel tranches; no current dirty source is accepted until its owner
reports a stable checkpoint and the integrated native gate passes.

## Authoritative next-show scenario

The next-show completion test is no longer a generic smoke. It is the following
operator sequence, using
`C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` as the Daslight source project:

1. Import the same evidenced fixture patch and supported control mappings from
   `DSF2026.dvc`. Every unsupported, ambiguous, or underdetermined DVC operation
   remains explicitly `Skipped`; it must not be approximated or replaced with a
   guessed Syndocal behavior.
2. Create a small representative set of Lighting scenes, sufficient to prove
   the imported patch and control path. The operator will author the final full
   lighting design after the product path is accepted.
3. Create LED-panel and projector video layers from the two exact user-supplied
   media assets when final media becomes available. Until then, use two clearly
   distinguishable deterministic synthetic QA videos generated locally for this
   acceptance run. The production media must never be guessed from similarly
   named personal files. Run the operator surface plus the two explicit video
   outputs as the intended three-display configuration:
   - Editor/operator display: `1920x1080`;
   - LED panel output: `1920x1080`;
   - Projector output: `3840x2160`.
   Each output keeps an explicit monitor identity and native resolution; losing
   or ambiguously resolving an output fails closed instead of swapping targets.
4. Create and populate two authored Timelines named `人生オーバー` and `惑う星`
   with representative Lighting and both video outputs.
5. When rekordbox starts the Master track `人生オーバー`, the DJ-PC agent sends
   an authenticated semantic event over wired LAN. Syndocal selects the exact
   mapped Timeline, starts it, and follows the authoritative DJ playback
   position rather than merely firing an unsynchronized cue.
6. When the DJ enters an 8-beat loop, Syndocal enters the corresponding authored
   Timeline loop. Subsequent half-loop divisions update the loop deterministically
   and idempotently from absolute DJ loop state; duplicate or stale events do not
   divide the loop again.
7. The foot pedal controls the documented DJ stage transition. When DJ playback
   stops for the band handoff, Syndocal exits the loop and continues the remaining
   `人生オーバー` Timeline from the exact authored continuation point instead of
   stopping or restarting it.
8. At the authored end of `人生オーバー`, Syndocal automatically transitions to
   and plays `惑う星`, including the authored BPM/tempo-map transition. Missing,
   recursive, stale, or ambiguous targets fail closed before changing transport.
9. The show uses ASIO. The selected real driver and negotiated parameters remain
   explicit and survive the complete sequence without WASAPI/device substitution,
   callback starvation, unreported XRUN, or restart ambiguity.

This complete native, physical, two-PC scenario is the final acceptance test.
Passing smaller helper, browser, loopback, or single-output tests is necessary
diagnostic evidence but does not complete the objective.

## Complete delivery objective

Completion requires every item below; these are conjunctive, not alternatives:

1. Repair every discovered correctness, fail-closed, clean-break, native/browser
   parity, warning, persistence, hardware, and operator-path defect. Maintain an
   explicit ledger until each finding has implementation and decisive evidence.
2. Remove frontend project-snapshot mutation as an alternate production or
   acceptance authority. Browser fixtures may construct immutable test input,
   but must exercise the same production mutation contract or be labelled
   layout-only and incapable of promoting native completion.
3. Make project transaction cleanup terminal and observable. Commit/Cancel
   contention, renderer loss, retry, acknowledgement, recovery capture, Undo,
   save, restart, and next mutation must never leave an invisible permanent
   active/pending/closing residue.
4. Replace overlapping frontend/Tauri/engine/recovery mutation ownership with
   one backend project coordinator and engine authority. Split `App.tsx` and
   `main.rs` by domain only while removing the retired routes; moving duplicate
   authority into smaller files is not sufficient.
5. Complete the Lighting/Timeline Bank/Scene contract: Lighting owns complete
   CRUD/edit/play; Timeline renders the same authoritative Bank name/order/color
   and Scene number/order/type as a compact read-only placement source; empty
   Banks remain visible; child-Timeline recursion targets remain excluded.
6. Complete detached and integrated Stage/Timeline behavior: correct content,
   no duplicate Timeline, no missing Source shelf or Stage render, no unusable
   blank main-window region, exact child identity, bounded recovery, and no
   unknown-to-absent promotion.
7. Import `DSF2026.dvc` with an exact report, create representative Lighting
   scenes, generate and use the two temporary resolution-specific videos, author
   `人生オーバー` and `惑う星`, and persist/reload the complete show project.
8. Complete the authenticated DJ-PC agent and Syndocal DJ-Link machine path,
   including version identity, exact NIC/network trust, credential persistence,
   Master-track semantics, position sync, loop state, pedal state, duplicate and
   stale event rejection, disconnect/reconnect, restart, and concurrent show LAN.
9. Complete the real ASIO show path: explicit persistent device/configuration,
   negotiation, callback I/O, start/stop/free, exclusive ownership, occupied
   device, control-panel/reset/resync, XRUN/no-callback/unplug recovery, restart,
   one-hour soak, and measured latency, without fallback to WASAPI/first device.
10. Execute the full `人生オーバー` to `惑う星` physical native scenario on the
    three specified displays with the DJ PC, pedal, lighting network/output, and
    ASIO device attached. Preserve evidence sufficient to diagnose every step.
11. Remove verified obsolete artifacts and legacy paths without deleting active
    source, accepted evidence, required SDKs, or user data. Record exact targets,
    sizes, recoverability, and retained boundaries.
12. Synchronize the next SemVer prerelease, run release and warning gates, build
    with the exact VS2022 14.44 linker, launch exactly one responsive maximized
    exact-checkout Syndocal window, update all QA/handoff documents, independently
    review the integrated diff, create meaningful commits, and push them.

The user's later work is limited to final artistic Lighting scene design and
replacement of the temporary LED/projector videos with final assets. Those
creative deliverables are not used to excuse an incomplete authoring, import,
mapping, routing, playback, synchronization, persistence, or recovery path.

## Explicit non-completion cases

The objective is not complete if any of the following remains true:

- a browser fixture passes while the corresponding native path is unexecuted;
- Bank, Timeline, pane, output, DJ-Link, or ASIO state can hang indefinitely;
- an invalid/stale/missing identity is silently replaced by the first/default
  Bank, layer, output, NIC, track, timeline, audio backend, or device;
- old and new mutation/transport/credential/persistence routes are both
  reachable without an explicit bounded migration deletion gate;
- only loopback, one display, one process, short ASIO smoke, or source inspection
  has passed;
- `DSF2026.dvc` import approximates an unsupported operation instead of reporting
  it as `Skipped`;
- the exact two-song DJ-to-band transition is not reproduced end to end;
- the accepted source, built executable, version metadata, evidence, commit, and
  pushed branch do not identify the same state.

## Non-negotiable acceptance policy

- A browser harness is design and deterministic-helper evidence only. It is not
  proof for Tauri invocation, native project transaction ownership, durable
  persistence, native windows, device enumeration, network trust, hardware I/O,
  or restart recovery.
- Every mutation in the show path must use one server-authoritative route and
  return an acknowledged terminal result. A frontend-only snapshot mutation is
  an alternate authority and fails this gate.
- Unknown, stale, ambiguous, unsupported, or unverified state fails closed with
  a visible actionable reason. It must not be converted to a default Bank,
  guessed order, replacement device, restored pane, or permissive result.
- A replacement path does not complete while its legacy command, state,
  persistence, UI, adapter, fixture, or documentation route remains reachable.
  Any explicitly required data migration must be one-way, bounded, observable,
  and have a recorded deletion gate.

## Immediate blocker ledger

| ID | Live-show requirement | Current evidence | Exit evidence | Status |
| --- | --- | --- | --- | --- |
| SHOW-P0-1 | Bank/Scene authoring never leaves project mutation authority stuck | Native alpha.11 Bank Save remained pending while recovery checkpoint reported busy. Static review found a possible permanent transaction-lane residue: a failed Commit followed by one swallowed failed Cancel can leave `closing`/active state without automatic finalization. | Deterministic contention tests prove exact-identity Commit/Cancel cleanup reaches a terminal state without permanent active/pending/closing residue; native create, rename, reorder, delete, Undo, save, restart, and reload all complete. | BLOCKED BY IMPLEMENTATION |
| SHOW-P0-2 | Timeline lower-right uses the same authoritative Bank/Scene representation as Lighting, with placement-only behavior | The compact representation and required `cueLists={snapshot().cue_lists}` connection exist in the dirty tree. The shared fail-closed authority now covers Control/Edit Cue Pads, editable Touch, outer and inner Timeline placement, delayed callbacks, and orphan events without fabricated Cue/Bank IDs. The hardened Scene Matrix gate drives the production-shaped owner registration, exact three-field Program Audio state, Begin/Batch/Commit, exact operation ACK, a complete terminal `R+1` authority result, Undo, and an expected dirty-project `beforeunload`; it passes 5/5 viewport sizes. Dedicated source-shelf and external-DnD contracts also pass. Independent Ox review, authoritative backend Bank create/rename integration, and a fresh native run remain open. | Same snapshot produces identical Bank ID/order/name/color and Scene ID/order/number/name/type in Lighting and Timeline; empty Banks remain; Timeline exposes no CRUD/play path; click and drag place the same Scene Block. | FOCUSED FRONTEND PROOF PASS / OX REVIEW, BACKEND, AND NATIVE PROOF OPEN |
| SHOW-P0-3 | Detached Stage/Timeline panes never dual-render, disappear, or leave unused main-window space | The previous startup recovery could treat failed placement capture as child absence. A fail-closed exact-label child census is being implemented. | Browser state-machine proof plus a fresh exact-checkout native run covers both detachment orders, restart with detached records, child present/absent/unknown outcomes, Stage integrated rendering, Timeline source/timeline separation, no duplicate pane, and no unusable main-window void. | BLOCKED BY INTEGRATION |
| SHOW-P0-4 | `DSF2026.dvc` drives representative Lighting, LED, and projector content on the intended physical routes | Static exact derivation of the pinned file reports 46 fixtures, 12 profiles, 15 fixture groups, 2 Banks, 2 Scenes, 84/84 accepted value payloads, 234 beam records, and zero unknown fixture types. Two MIDI mappings remain Approximate because device affinity is not exact, and one DVC GOLD binding remains Unsupported. Temporary resolution-specific media and a reproducible generator exist, but the focused importer execution and physical output flow remain unexecuted. | Exact/Skipped importer report, representative Lighting scenes, generated LED/projector assets, authored `人生オーバー`/`惑う星` Timelines, and fresh alpha.12 native three-display output complete, save, restart, and reload without substitution. | STATIC INVENTORY COMPLETE / EXECUTION BLOCKED |
| SHOW-P0-5 | DJ-Link survives setup, authenticated wired operation, disconnect, and app restart | The complete peer source is committed and pushed on `beta-v1.1.2` at `1a2f3bc70507d1bb4c5ffe2eb8bbb2b24686532f`, equal to `origin/beta-v1.1.2`, with a clean tree. Full tests pass 329/329 with 2 release-gated skips separately exercised by the real pkg commitment smoke (2/2); `npm audit` reports 0 vulnerabilities and first-party warnings are 0. The release workflow's Inno Setup URL/hash mismatch was fixed by pinning the official GitHub `is-6_7_3` asset whose measured SHA-256 exactly matches the recorded pin. Final annotated tag, installer/ZIP/release manifest, and all 12 physical checks remain open by design. | Identity-bound `beta-v1.1.2` peer is committed and pushed, both PCs run the intended artifacts, and HW-4.1 through HW-4.12 are recorded over wired LAN, including pedal, loop, dedupe, reconnect, restart, and concurrent Art-Net/sACN. | SOURCE CHECKPOINT PUSHED / TAG, PACKAGE, AND HARDWARE OPEN |
| SHOW-P0-6 | The distributed artifact exactly matches the accepted source | Current source is dirty and remains version `1.2.0-alpha.11`. | Focused gates, warning ratchet, synchronized alpha.12 metadata, `check:release`, exact VS2022 14.44 linker, `pnpm --dir app tauri build --no-bundle`, exactly one responsive maximized exact-path window, evidence update, meaningful commit, and successful push. | NOT RUN |
| SHOW-P0-7 | ASIO is explicitly selected and remains truthful through start, live callback I/O, stop, fault, and restart | The isolated bridge has clean-broken to ABI/schema v2 and one canonical `syndocal_asio_bridge.dll`; independent Ox review passed its bridge boundary. Follow-up hardening bounds every channel gain to finite `[0,1]`, adds exported pre-backend invalid-gain proof, and validates all nine full C prototypes including duplicate/mutated/comment-only negative cases. SDK-free tests pass 11/11 under the exact VS 14.44 linker with zero reported first-party warnings; `distribution_approved` remains false. Default-package separation has an implementation, but independent adversarial review found common-config resource, path traversal, SDK-pin drift, renamed-byte, and installer-content bypasses, so packaging is not accepted. The application loader remains ABI v1 and incompatible until strict v2 integration; persistence/stale lock, typed lifecycle, real hardware, one-hour soak, and latency evidence remain open. | The actual show driver/rate/channels/native format/buffer are persisted and revalidated; Start/Stop/Close, callback continuity, occupied/unplug/XRUN/no-callback recovery, restart, one-hour soak, and measured latency pass without WASAPI or another-driver substitution. | BRIDGE V2 REVIEWED / PACKAGE REPAIR, APP INTEGRATION, AND HARDWARE OPEN |
| SHOW-P0-8 | `人生オーバー` follows DJ-Link position/loop/pedal handoff and auto-transitions to `惑う星` | Absolute loop/release and authored Follow/BPM primitives exist in the dirty source, but MasterTrackActive starts the Timeline without applying `position_sec` or track BPM to engine transport; StateSync also stores position diagnostically only. A nonmatching nonempty content ID can still fall through to title/artist matching. The exact authored Timelines and full sequence remain unexecuted. | Wired authenticated Master-track trigger, position follow, absolute 8-beat and half-loop updates, pedal stop/band continuation, Timeline completion, and BPM-aware automatic `惑う星` transition pass end to end with duplicate/stale/disconnect/restart cases. | BLOCKED BY IMPLEMENTATION AND AUTHORING |
| SHOW-P0-9 | Scene Settings remains usable at show-editor size and an authored Scene may intentionally contain zero owned FX | The focused browser gate now passes at 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720 with an 8-FX fixture: 4x2 family layout at 1920, no chooser descriptions, local 32px owned-FX toolbar controls, contained focusable scrollport, real wheel/PageDown/End reaching a bottom sentinel, and zero document/app scroll. Browser removal reaches 0, but native Tauri/engine guards still reject the final removal. | Exact-checkout native UI reproduces the internal scroll and zero-FX operation; acknowledged engine state, active runtime release, save/reload persistence, and publication-failure rollback all pass without browser-local authority. | FOCUSED LAYOUT PROOF PASS / NATIVE MUTATION BLOCKED |

## Discovered fail-closed and clean-break repair ledger

Every finding below remains required even if a narrower show smoke passes. Line
references are the discovery snapshot and may move as the file is decomposed;
the invariant and retirement proof remain binding.

| ID | Finding and discovery evidence | Required end state | Status |
| --- | --- | --- | --- |
| FC-01 | Generic project Begin/Commit can wait indefinitely; failed Cancel/ACK cleanup is swallowed (`App.tsx` generic invoke/recovery; `main.rs` blocking admission and lane close). | Exact-identity bounded terminal recovery, no raw mutation resend, no permanent active/pending/closing residue, visible unrecovered state, next mutation succeeds without restart. | IN IMPLEMENTATION |
| FC-02 | Browser fixture implements Bank/Scene/Timeline/Patch/FX CRUD with direct `setSnapshot`, bypassing Tauri, coordinator, engine publication, history, and persistence. | Fixtures use the production typed mutation contract or are immutable layout-only inputs that cannot promote native acceptance; production mutation functions contain no fixture-local project mutation branch. | OPEN |
| FC-03 | Bank/Scene CRUD mixes legacy renderer-ticketed Create/Rename with server-authoritative Empty/Delete/Reorder. | One versioned server-authoritative Bank/Scene API owns every CRUD operation; old route is removed from invoke registration, frontend inventories, backend, tests, and docs. | IN IMPLEMENTATION |
| FC-04 | A nonmatching DJ `content_id` falls through to title/artist matching and can trigger the wrong Timeline (`main.rs` DJ mapping resolver). | Nonempty content ID is the sole authority; mismatch is explicit `mapping_not_found`; any metadata-only policy is explicit, versioned, unambiguous, and never entered from ID mismatch. | OPEN |
| FC-05 | Project JSON is deserialized into separate project/mapping structs without rejecting unknown mapping keys, so a typo can silently delete DJ triggers. | One versioned project document owns mappings, rejects unknown/future fields, and migrates supported v1 exactly once to canonical v2. | OPEN |
| FC-06 | Missing Bank, output, layer, DMX mode, or fixture mode selection is replaced by first/default/fabricated state. | Preserve stale identity, visibly lock the operation, and require explicit reselection; exact identity mismatch changes no other object. | OPEN |
| FC-07 | Root Timeline transport uses a newer authority while child play/seek, metronome, loop, and old root commands remain fire-and-forget and registered; count-in is clamped. | One receipt/fence protocol owns root and child play/seek/loop/metronome; invalid count rejects unchanged; old commands are unregistered. | OPEN |
| FC-08 | DJ machine status/NIC errors become unavailable/empty UI, and legacy and generic wire envelopes remain accepted without a retirement gate. | Preserve typed failure reason; exact current envelope only; old wire unregistered after a bounded peer migration; wired restart evidence passes. | OPEN |
| FC-09 | When the selected ASIO backend disappears from inventory, the frontend automatically chooses WASAPI or the first backend and clears settings. | Retain the ASIO identity as stale/faulted, lock Start, expose the exact reason, and change backend only through an explicit operator action. | OPEN / SHOW P0 |
| FC-10 | Missing ASIO backend fields deserialize to WASAPI and optional backend queries default to WASAPI. | Versioned requests require backend identity, reject unknown/missing fields, and perform zero stream/catalog mutation on rejection. | OPEN / SHOW P0 |
| FC-11 | ASIO bridge missing/load/symbol/ABI errors collapse to `built:false`. | Return and display `NotPackaged`, `Ready`, or typed `Fault`; no error class triggers fallback. | OPEN / SHOW P0 |
| FC-12 | Two ASIO DLL names are searched in order, so legacy-only or both-present states select silently. | One canonical artifact name; legacy-only and ambiguity fail visibly, or a separately versioned one-way migration has an explicit deletion milestone. | OPEN / SHOW P0 |
| FC-13 | ASIO backend/driver/rate/format/channels/buffer/mix are process-local signals and do not survive restart. | Versioned machine-local persistence, exact driver identity revalidation, stale lock, restart/next-show proof, and no project-file secret/device leakage. | OPEN / SHOW P0 |
| FC-14 | Timeline layers and missing `layer_id` are synthesized/retargeted at runtime from legacy track fields indefinitely. | One-way versioned migration writes canonical layers; runtime requires exact IDs; old track synthesis and retarget paths are removed. | OPEN |
| FC-15 | Timeline Bank shelf accepts optional `cueLists`, fabricates unavailable Banks, loses authoritative order/name, and hides valid empty Banks. | `cueLists` is required; exact order/name/color and empty Banks are retained; orphans are explicit unavailable non-placeable entries; Timeline remains placement-only. | IMPLEMENTED / FOCUSED BROWSER PROOF PASS / NATIVE OPEN |
| FC-16 | UI display logic and engine snapshot sanitization rewrite the persisted default-Bank aliases `Main` / `Cue List 1` to `Bank 1`, allowing an authored saved label to change during load or diverge across Lighting/Timeline/Executor. | A new project starts with the exact label `Bank 1`; every persisted nonblank label, including `Main`, remains byte-for-byte unchanged everywhere; all alias rewrite branches and their positive tests are removed or inverted. | IN IMPLEMENTATION |
| FC-17 | New authoritative media commands coexist with transitional legacy public media IPC, allocator, adapters, and tests. | Only the authoritative media route remains registered; any saved-data migration is loader-only and one-way, not public runtime IPC. | OPEN |
| FC-18 | `.sdc` remains v1 while several legacy normalizations run on every load without writing a canonical upgraded artifact. | Independent schema version, one-way migration report, explicit upgraded save, future/invalid rejection, second load performs zero migration, old loader has a deletion milestone. | OPEN |
| FC-19 | Legacy video output commands remain registered beside v2; some mapping-preset routes still mutate Engine. | Retired output commands are absent from invoke handler, frontend types/inventory, backend, tests, and docs; only v2 succeeds. | OPEN |
| FC-20 | Browser pane `window.open()` success and localStorage persistence failures can be accepted or swallowed without proving a child/native record. | Browser path remains layout-only; native exact-label lifecycle is authoritative; popup/storage failure is visible and never hides main content. | PARTIAL FIX / NATIVE PROOF OPEN |
| FC-21 | Decoder and last-valid video fallbacks have telemetry but no explicit ownership/removal condition under the new rule. | Either remove the fallback or record bounded trigger, owner, performance/freshness limit, tests, operator state, and deletion criterion. | OPEN |
| FC-22 | Frontend sets and backend invoke registration are hand-maintained inventories, so an unclassified mutator can escape the gates. | Generate one descriptor inventory; every command is exactly one of read-only, runtime-only, backend-authoritative mutation, or retired/unregistered; unclassified is a build failure. | OPEN |
| FC-23 | Browser VJ constructs synthetic media/runtime/effects/outputs, while multiple native output operations remain explicitly unavailable. | Use real imported test media and canonical artistic renderer on exact physical outputs; unavailable native operations remain disabled until implemented, never simulated as acceptance. | IN IMPLEMENTATION / SHOW P0 |
| FC-24 | Scene Settings browser fixture can remove the final owned FX locally, but both Tauri validation and the engine reject an empty replacement, so native `x` cannot reach zero and the green browser result bypasses authority. | Empty FX replacement is an acknowledged authoritative mutation; an active effect-only Scene releases its runtime contribution, persists zero FX, survives save/reload, and rollback leaves the old FX intact on publication failure. The duplicate last-FX rejection path and its legacy acceptance test are deleted or inverted. | IN IMPLEMENTATION / NATIVE PROOF OPEN |
| FC-25 | The shared Bank authority now closes the Bank editor, shelf, executor, Control/Edit Lighting Cue Pads, editable Touch, outer and inner Timeline placement, and delayed callbacks. Fault clears armed state; orphan events remain unavailable without fabricated Cue/Bank IDs. A production-shaped fixture hook and strict terminal-authority harness expose real authority transitions without a permissive empty snapshot; its first `R+1` run exposed and then proved the immutable-clone Undo repair. | One immutable authority result gates every Bank mutation, Scene placement, executor, Control/Edit/Live/Touch action, outer and inner Timeline surface, and delayed callback before rendering and again at invocation time; fault clears armed state, sends zero IPC/runtime commands, preserves visible invalid bindings, and never fabricates Cue/Bank IDs. | FOCUSED IMPLEMENTATION AND 5-SIZE PROOF PASS / OX REVIEW AND NATIVE PROOF OPEN |
| FC-26 | A non-null selected Bank identity that disappears during polling is silently replaced by the first remaining Bank in shared helper, App selection effect, Playback Executor create state, and Scene Matrix selection. A contract test currently blesses missing Bank 1 becoming Bank 7. | `null` initial selection may explicitly adopt the first authoritative Bank once; any non-null missing/stale selection remains preserved as unavailable and locks dependent actions. Only a successful explicit delete transaction may select a documented successor. Tests must reject missing-ID-to-first-Bank fallback. | P0 REVIEW FINDING / REPAIR REQUIRED |
| FC-27 | Bank authority does not validate `CueListSummary.active_cue_id`; UI can display a Cue from another Bank as active, and engine load can carry a malformed active-Cue reference into effect activation. Existing sanitization may silently clear some malformed values instead of rejecting the document unchanged. | An active Cue reference is null or an exact unique Cue in the same exact Bank. Invalid, missing, duplicate, or cross-Bank references reject load/mutation unchanged and close every dependent UI/runtime surface; no label-only display, activation, or silent clear is permitted. | REVIEW FINDING / FRONTEND AND ENGINE REPAIR REQUIRED |
| FC-28 | Video core has a payload-less exact-black artistic admission. Independent review confirmed all six public frame-only artistic wrappers reject both `Error` and `LastValid` with typed `NonFreshArtisticOutput`; `cargo test -p video` passed 159/159 with 1 ignored and zero first-party warnings under the exact VS 14.44 linker. NDI/Spout integration is under adversarial repair after review found current-project blackout did not dominate historical Follow snapshots, route identity omitted exact kind/endpoint, Follow authority could roll over before SDK send, and revalidate-then-send lacked an honest linearization/fence. Display still uses legacy preparation/raw presentation paths and hides zero-area. | Every physical sender/presenter consumes the evidenced artistic result, rechecks complete current output/safety/project authority immediately before I/O, treats non-Fresh as typed failure or exact-black safety output, updates LastValid only from Fresh, exposes blackout telemetry, rejects zero-area unchanged, and has no reachable legacy/frame-only success path. | CORE PASS / NDI-SPOUT ADVERSARIAL REPAIR ACTIVE / DISPLAY OPEN |
| FC-29 | `App.tsx` is currently 28,159 lines / 1,199,580 bytes and `main.rs` is 123,293 lines / 4,717,427 bytes. Bank/Scene, project transaction/recovery, Timeline, external control, ASIO, video presentation, and fixture/browser routes still converge in these two files, so authority ownership is difficult to audit and native-only divergence can hide behind a frontend pass. The new `bankAuthority.ts` and `projectTransactionRecovery.ts` extractions are useful starts, not architectural completion. | Each mutation domain has one typed coordinator/controller module and one registered backend handler surface; `App.tsx` composes UI only, `main.rs` composes Tauri/state only, generated command classification rejects unowned routes, and domain tests prove no fixture/browser/recovery/legacy parallel mutation path. Extraction must preserve the accepted show contracts one domain at a time; a risky monolithic rewrite is forbidden. | STRUCTURAL DEBT CONFIRMED / INCREMENTAL EXTRACTION STARTED / COMPLETION OPEN |

Closing a row requires implementation plus a negative proof that the retired or
invalid path is unreachable. A comment, warning, disabled UI button, or green
browser layout is not row closure.

## Mandatory ASIO show blocker

The next show uses ASIO, so every applicable unchecked gate in
`qa/ASIO_INPUT_ACCEPTANCE.md` is part of this near-show run. Syndocal must not
silently substitute WASAPI or a different ASIO driver when the persisted ASIO
selection is missing, stale, occupied, or incompatible. Unsupported rate,
channel, format, buffer, ownership, callback, reset, XRUN, unplug, and restart
states must remain visible and fail closed.

## Authority and decomposition obligation

File size alone is not an acceptance failure, but the present responsibility
shape is. `App.tsx` and `main.rs` currently participate in project intent,
transaction coordination, projection, recovery, native integration, and large
amounts of unrelated UI/runtime orchestration. More importantly, some browser
fixtures and renderer code can mutate local project projections while Tauri,
engine, and recovery paths own overlapping versions of the same state. That
multi-authority topology is a correctness defect, not merely style debt.

The required clean break is:

1. the frontend emits typed intent and renders acknowledged projections; it
   never commits a project mutation by changing its local snapshot;
2. one backend project transaction coordinator owns admission, identity,
   Commit/Cancel finalization, receipts, history, and durable publication;
3. the engine is the sole author of project runtime state;
4. recovery stores and restores only acknowledged checkpoints and never acts as
   an alternate mutation authority;
5. Tauri commands are thin typed adapters into that coordinator, not raw
   alternate mutation routes;
6. `App.tsx` is split into domain controllers/stores and workspace composition,
   while `main.rs` is split into domain command modules and application wiring;
7. old raw commands, local fixture mutation branches, duplicate state helpers,
   compatibility adapters, and obsolete tests/docs are removed once callers
   migrate. No indefinite forwarding facade remains.

Decomposition is complete only when behavior tests prove the intended route and
negative tests prove the retired routes are unreachable. Moving the same
multi-authority logic into smaller files is not completion.

## Native/browser parity audit scope

The parity audit must explicitly classify these boundaries rather than infer
native success from a green browser run:

1. project mutations, transaction cleanup, persistence, recovery, Undo/Redo;
2. pane creation, placement, enumeration, restart, and close acknowledgement;
3. media selection, decoder/renderer startup, monitor identity, and output loss;
4. DMX/Art-Net/sACN device and NIC selection, bind failure, and reconnect;
5. DJ-Link NLM trust, exact GUID/IP binding, Credential Manager, listener
   replacement, authentication, and peer version identity;
6. ASIO COM/driver lifecycle, negotiated format/buffer, callback, XRUN, unplug,
   and restart recovery;
7. updater/package metadata, installed artifact identity, and clean-start
   migration/future-version rejection.

For each boundary, the final handoff must name the authoritative native evidence
or keep it explicitly unverified. Finding no browser regression is never enough.

## Temporary video evidence

`qa/harnesses/New-NearShowTestMedia.ps1` generates the two temporary clips under
the Git-ignored `target/qa/near-show-media` directory. The 2026-08-25 generation
completed successfully and `ffprobe` verified both first video streams:

| Role | Resolution | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| LED | 1920x1080 | 11,479,089 | `BCA2D8E70C142E2B4840C14424DA497ECDAE61DB255B43216CA320BE04D48E41` |
| Projector | 3840x2160 | 43,807,726 | `D51DCD2B4F55CB7A6FAE3F34E9B4D93E6BAC24B0B29A6BE500AB4B848D323E29` |

The generated `manifest.json` records absolute paths, codec, pixel format, frame
rate, duration, size, and hash. These files prove only deterministic media
availability and resolution; they do not prove Syndocal import, decoder,
timeline, monitor routing, or physical pixel output.

## Focused frontend checkpoint (2026-08-25)

The current dirty frontend passes `pnpm --dir app run build` with TypeScript
and Vite reporting no first-party warnings. Focused fail-closed/UI gates also
pass: identity color (`21` distinct hues), Timeline external DnD, Timeline
source-shelf contract and click/drag parity, localization (`3566/3566`, zero
unprotected user-data labels), right-click suppression, Scene Settings at all
five supported browser sizes, Scene Matrix full authoring at all five sizes,
and the production-shaped Scene Matrix cross-Bank transaction/Undo gate at all
five sizes. The comprehensive Scene Matrix fixture covers Bank create/custom
name/automatic name/normalized-duplicate rejection, context-menu rename/delete,
keyboard opening/escape/focus restoration, empty-Bank single `+ Scene` action,
Scene rename/duplicate/delete, Bank reorder, and Undo/Redo.

These are browser/fixture proofs only. They do not close the backend Bank
authority, native Tauri mutation, native input, persistence, or process/window
acceptance rows. In particular, the fixture's successful Bank creation must not
be cited as proof that the current native Bank Save hang is repaired.

## Connected display mode evidence

The 2026-08-25 five-display probe must distinguish DPI-virtualized
`System.Windows.Forms.Screen.Bounds` from the current physical display mode.
The earlier logical-bounds reading was incorrectly treated as physical mode
evidence. A Win32 `EnumDisplaySettings(..., ENUM_CURRENT_SETTINGS, ...)` probe
corrected that conclusion and confirmed that the 4K display is present:

| Device | Primary | DPI-virtualized bounds | Current physical mode |
| --- | --- | --- | --- |
| `DISPLAY1` | no | 2048x1152 at 1920,-364 | 2560x1440 at 320 Hz |
| `DISPLAY2` | yes | 1920x1080 at 0,0 | 1920x1080 at 360 Hz |
| `DISPLAY3` | no | 2560x1440 at -3840,-429 | **3840x2160 at 240 Hz** |
| `DISPLAY5` | no | 1280x720 at -2465,1731 | 1920x1080 at 60 Hz |
| `DISPLAY6` | no | 2560x720 at 1598,1080 | 2560x720 at 60 Hz |

Therefore the projector's required 3840x2160 physical mode is available now;
there is no mode-switch blocker. Final acceptance must bind the intended output
to the exact display identity and verify the physical client extent. Logical
desktop bounds alone are not valid evidence for output resolution.

A read-only DisplayConfig inventory was also retained at
`qa/artifacts/display-physical-mode-correction-20260825-1cdea0539eec4869af51e86e80b3d6da`.
It identifies `DISPLAY3` as `MPG321UX OLED`, stable identity ending
`MSI3DD2...UID4357`, at 3840x2160 and effective DPI 144; `DISPLAY2` is the
primary `Pixio PX259PS` at 1920x1080/DPI 96; and `DISPLAY5` is `PX160 WAVE` at
1920x1080/DPI 144. The dry-run verdict is deliberately `not-configured`: exact
editor/LED/projector role binding and the two live output IDs/labels remain for
the final native run and must not be inferred from resolution or display order.

The acceptance harnesses were re-run after this correction. The native 4K
runner self-test passed 27/27 and the three-display runner self-test passed
18/18. These results prove the runners' fail-closed identity, DPI, HWND,
process-path, hash/version/HEAD, placement, responsiveness, and stable-sampling
contracts; they are not a substitute for the later exact-build physical run.

## Recoverable build-artifact cleanup ledger

The workspace-size audit found no evidence that old authored source is the main
capacity consumer. A 2026-08-25 `du -h -d 1` measurement identified Cargo
outputs as the dominant recoverable footprint:

| Generated path | Measured size | Disposition |
| --- | ---: | --- |
| `target/debug/deps` | 83 GB | Remove after all active Rust checks finish. |
| `target/debug/incremental` | 13 GB | Remove with the debug profile; fully reproducible. |
| `target/debug/build` | 5.3 GB | Remove with the debug profile; fully reproducible. |
| entire `target/debug` | 102 GB | Run `cargo clean --profile dev --dry-run`, verify the exact checkout target, then run the same command without `--dry-run`. |
| entire `target/release` | 8.5 GB | Retain until the accepted native executable and evidence are replaced by the final exact-source release build. |
| entire `target/qa` | 249 MB | Retain current acceptance evidence; prune only named superseded runs after the final evidence set is recorded. |
| `.git` | 316 MB | Retain; `git count-objects -vH` reported only one 568 KiB temporary object, not the capacity cause. |

Do not clean `target/debug` while Cargo owners are running: doing so would race
their compiler outputs and waste the near-show critical path. Build artifacts
are recoverable by recompilation; authored `.sdc` projects, source media,
licensed SDK input, accepted QA evidence, and the final release executable are
not cleanup targets.

## Required final operator sequence

1. Start from no running exact-checkout Syndocal process and a known saved show
   project; build and launch the exact accepted source.
2. Verify one responsive maximized Syndocal operator window on the intended
   1920x1080 editor display, then bind the projector output to the exact
   `DISPLAY3` 3840x2160 identity and confirm all three show surfaces have
   explicit identities.
3. Create/rename/reorder/delete/Undo a temporary Bank and Scene, save, restart,
   and verify the restored authoritative state.
4. Verify Lighting and Timeline show the same Bank/Scene identity and ordering;
   place the Scene from Timeline without triggering playback at placement time.
5. Integrate and detach Stage and Timeline in both orders; verify Stage content,
   Timeline content, source shelf, no duplicate content, and useful main layout.
6. Run representative lighting and two-video-output cues through Timeline,
   including stop/restart and output-loss fail-closed behavior.
7. Run the complete wired DJ-Link/pedal acceptance, disconnect/reconnect, and
   both-PC restart without manually reconstructing hidden credentials or routes.
8. Run the selected ASIO device through configuration persistence, start/live
   callback/stop, occupied device, reset/XRUN/unplug/no-callback recovery,
   restart, one-hour soak, and measured latency without fallback.
9. Preserve logs, screenshots, hashes, warning counts, exact commands, and all
   deliberately unverified hardware boundaries; then commit and push.

This document is a live checkpoint, not an acceptance claim. A row changes to
accepted only after its named exit evidence exists on the current integrated
source and artifact.
