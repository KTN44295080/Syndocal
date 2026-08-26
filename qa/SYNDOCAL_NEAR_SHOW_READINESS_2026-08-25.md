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

The performance is **2026-08-30**. Development, acceptance, and show
preparation must be complete by the separate **2026-08-29 completion
deadline**. Neither date waives any named fail-closed hardware or native gate.

Version-state distinction (2026-08-26): the current exact committed and pushed
`1.2.0-alpha.14` runtime/standard-artifact source checkpoint is
`92122f1b148d40845b2cfe3e4618a57ce132b3df`. Its
`target/release/syndocal.exe` is `58,523,648` bytes,
ProductVersion/FileVersion `1.2.0-alpha.14`, SHA-256
`B140E9DA515741C8A6A318963C6BAB576CED62ECABEDEA50BA8DE15593AE325C`;
the native warning gate recorded zero baseline/current warnings and exactly one
responsive maximized exact-path window. The historical committed and pushed
`1.2.0-alpha.12` runtime/artifact source checkpoint is
`ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae`. Later cleanup and documentation
checkpoints do not redefine any built artifact's provenance. The historical
alpha.12 standard native artifact is `target/release/syndocal.exe`, 58,471,936 bytes,
ProductVersion/FileVersion `1.2.0-alpha.12`, SHA-256
`224F512673C8A84EAEB2557691414B2F6CA090D1201E357DCD9B38F019237680`.
The separate same-host Show-ASIO directory is
`target/show-asio-local/Syndocal_Show_ASIO_1.2.0-alpha.12_ff61a6dec6eb_x64`;
its application is 58,637,824 bytes, SHA-256
`1D313900AB94A2429BF784B7D4CCA8E8EC39FBF17E11CB257D76A19656AA2F8D`,
and its ABI-v2 bridge is 813,568 bytes, SHA-256
`40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`.
Its manifest records `distributionApproved: false`, `sameHostOnly: true`, and
`unbundled: true`. The complete warning matrix executed for `ff61a6d` recorded
zero first-party warnings. These artifact and warning results do not close the
physical DJ (0/12), native ASIO/operator, three-display Apply/output, or DSF
show-project acceptance gates.

DJ-Link update (2026-08-26): `1.2.0-alpha.13` is the committed/pushed Timeline
UI checkpoint `bbb684cee4c8b01cfc019575569bd26835dbc732`. The current product
metadata is `1.2.0-alpha.14`; its strict v3 peer is rb-output `1.1.5`, runtime
`862cf8035dfb365a7d799f820936585882d0a1e7`, clean docs tip
`71738778c8b7637c14768e02fecbc2ef14ece7f3`. Focused
software gates cover exact v3 clean break, fresh measured-loop authority,
bounded no-response prediction, late-measurement rebase, independent Release,
and the full `8 → 4 → 2 → 1 → 1/2 → 1/4 → 1/8 → 1/16 → 1/32 → 1/64`
profile. This does not change the physical DJ matrix: it remains 0/12 until the
real token, controller, Rekordbox, pedal, wired LAN, reconnect, and restart run.

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
| SHOW-P0-2 | Timeline lower-right uses the same authoritative Bank/Scene representation as Lighting, with placement-only behavior | The compact representation and required `cueLists={snapshot().cue_lists}` connection exist in the dirty tree. The shared fail-closed authority now covers Control/Edit Cue Pads, editable Touch, outer and inner Timeline placement, delayed callbacks, and orphan events without fabricated Cue/Bank IDs. The hardened Scene Matrix gate drives the production-shaped owner registration, exact three-field Program Audio state, Begin/Batch/Commit, exact operation ACK, a complete terminal `R+1` authority result, Undo, and an expected dirty-project `beforeunload`; it passes 5/5 viewport sizes. Dedicated source-shelf and external-DnD contracts also pass. Independent Ox review returned PASS with no Blocker/High/Medium; its two actionable Low findings were repaired by safety-lowering flash release across a later authority fault and visible discard messages for stale delayed look writes. The stopped Ox backend tranche now passes exact-linker engine Bank tests 16/16 and Tauri `--no-default-features` Bank tests 9/9, but remains uncommitted under independent review. The warning ratchet also found the new eight-argument `rename_cue_list` command; it must be converted to a typed request shape rather than suppressed. Fresh native proof remains open. | Same snapshot produces identical Bank ID/order/name/color and Scene ID/order/number/name/type in Lighting and Timeline; empty Banks remain; Timeline exposes no CRUD/play path; click and drag place the same Scene Block. | FOCUSED FRONTEND + OX REVIEW PASS / BACKEND TESTS PASS / REVIEW, WARNING FIX, AND NATIVE PROOF OPEN |
| SHOW-P0-3 | Detached Stage/Timeline panes never dual-render, disappear, or leave unused main-window space | The previous startup recovery could treat failed placement capture as child absence. A fail-closed exact-label child census is being implemented. | Browser state-machine proof plus a fresh exact-checkout native run covers both detachment orders, restart with detached records, child present/absent/unknown outcomes, Stage integrated rendering, Timeline source/timeline separation, no duplicate pane, and no unusable main-window void. | BLOCKED BY INTEGRATION |
| SHOW-P0-4 | `DSF2026.dvc` drives representative Lighting, LED, and projector content on the intended physical routes | Static exact derivation of the pinned file reports 46 fixtures, 12 profiles, 15 fixture groups, 2 Banks, 2 Scenes, 84/84 accepted value payloads, 234 beam records, and zero unknown fixture types. Two MIDI mappings remain Approximate because device affinity is not exact, and one DVC GOLD binding remains Unsupported. Temporary resolution-specific media and a reproducible generator exist, but the focused importer execution and physical output flow remain unexecuted. | Exact/Skipped importer report, representative Lighting scenes, generated LED/projector assets, authored `人生オーバー`/`惑う星` Timelines, and fresh alpha.14 native three-display output complete, save, restart, and reload without substitution. | STATIC INVENTORY COMPLETE / EXECUTION BLOCKED |
| SHOW-P0-5 | DJ-Link survives setup, authenticated wired operation, disconnect, and app restart | Published v1.1.3 remains immutable but blocked by its `DJ_MASTER_CHANGED` mismatch. The sole current show route is the controlled target-DJ-PC source on branch `beta-v1.1.2`, source version `1.1.5`, using only strict `syndocal-envelope-v3`; flat/v1/v2 routes are retired. The source requires a checkout-external show JSON through `DJ_AGENT_CONFIG_PATH`; no-argument `start-all.bat` is the real launch and exact lowercase `--preflight-only` is the only alternate. Passing preflight is software-only and starts no show-side process. The real launch still requires the current real Syndocal token. Current topology is FOH `192.168.50.1` / DJ PC `192.168.50.2`. No installer or preflight result is promoted to hardware acceptance. | Both PCs run the pinned controlled source/native artifact with the real token, and HW-4.1 through HW-4.12 are recorded over wired LAN, including pedal, measured loop, no-response fallback, dedupe, reconnect, restart, and concurrent Art-Net/sACN. | STRICT V3 SOFTWARE PATH / REAL TOKEN + HARDWARE 0/12 OPEN |
| SHOW-P0-6 | The distributed artifact exactly matches the accepted source | The current exact committed/pushed alpha.14 standard runtime/artifact source checkpoint is `92122f1b148d40845b2cfe3e4618a57ce132b3df`. Its `target/release/syndocal.exe` is 58,523,648 bytes, ProductVersion/FileVersion `1.2.0-alpha.14`, SHA-256 `B140E9DA515741C8A6A318963C6BAB576CED62ECABEDEA50BA8DE15593AE325C`; exactly one responsive maximized exact-path Syndocal window was recorded at the artifact checkpoint. The only built Show-ASIO artifact is the historical alpha.12 local-only artifact from `ff61a6d`; it is not distributed, and its manifest fixes `distributionApproved: false`, `sameHostOnly: true`, and `unbundled: true`. | Preserve the exact alpha.14 standard identity and historical alpha.12 Show-ASIO identity separately; complete the remaining physical DJ, ASIO, display, and DSF acceptance without substituting later cleanup/docs commits as runtime provenance. | ALPHA.14 STANDARD + HISTORICAL ALPHA.12 LOCAL SHOW-ASIO ARTIFACTS BUILT / PHYSICAL ACCEPTANCE OPEN |
| SHOW-P0-7 | ASIO is explicitly selected and remains truthful through start, live callback I/O, stop, fault, and restart | The isolated bridge and application loader source have clean-broken to ABI/schema v2 and one canonical `syndocal_asio_bridge.dll`; the old claim that the app loader remains ABI v1 is retired. The complete warning matrix executed for `ff61a6d` recorded zero first-party warnings. The exact 44.1 kHz / 2-channel i32 / 128-frame HOTONE Ampero bridge-only run completed for `3,600,031 ms` without promoting that bridge evidence to native operator acceptance. The only built Show-ASIO artifact is the historical alpha.12 local-only checkpoint `ff61a6d`: `target/show-asio-local/Syndocal_Show_ASIO_1.2.0-alpha.12_ff61a6dec6eb_x64`; its application SHA-256 is `1D313900AB94A2429BF784B7D4CCA8E8EC39FBF17E11CB257D76A19656AA2F8D`, its ABI-v2 bridge SHA-256 is `40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`, and the manifest fixes `distributionApproved: false`. No alpha.14 Show-ASIO build is implied. Formal matched 48 kHz ASIO/WASAPI, native load/UI/persistence/revalidation, occupied/reset/unplug/XRUN/no-callback recovery, restart, and latency remain open. | The actual show driver/rate/channels/native format/buffer are persisted and revalidated; Start/Stop/Close, callback continuity, occupied/unplug/XRUN/no-callback recovery, restart, formal matched 48 kHz soak, and measured latency pass without WASAPI or another-driver substitution; the local-only artifact passes its dedicated checker immediately before use. | HISTORICAL ARTIFACT + ZERO-WARNING MATRIX PASS / ALPHA.14 SHOW-ASIO BUILD, NATIVE OPERATOR, MATCHED 48 KHZ, RECOVERY, LATENCY, AND PHYSICAL ACCEPTANCE OPEN |
| SHOW-P0-8 | `人生オーバー` follows DJ-Link position/loop/pedal handoff and auto-transitions to `惑う星` | Strict wire v3 now requires fresh position/effective BPM/session/deck identity, continuous revisioned Sync, exact measured loop state, atomic engine start/sync/release ownership, and late-event fencing. F14 arms a bounded response window before local MIDI; fresh Rekordbox measurement remains primary, only true no-response emits a distinct predicted fallback, invalid/stale/contradictory response suppresses prediction, and a later fresh measurement rebases it. F13 routes Release independently of Rekordbox Stop MIDI success. Focused software proof passes, but the real-token run and exact authored Timelines/full physical sequence remain unexecuted. | Wired authenticated Master-track trigger, position follow, measured absolute loop updates and no-response fallback across `8 → 4 → 2 → 1 → 1/2 → 1/4 → 1/8 → 1/16 → 1/32 → 1/64`, independent pedal Release/stop/band continuation, Timeline completion, and BPM-aware automatic `惑う星` transition pass end to end with duplicate/stale/disconnect/restart cases. | STRICT V3 SOFTWARE CONTRACT / NATIVE, REAL RUN, AUTHORING, AND HARDWARE OPEN |
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
| FC-04 | A nonmatching DJ `content_id` falls through to title/artist matching and can trigger the wrong Timeline (`main.rs` DJ mapping resolver). | Nonempty content ID is the sole authority; mismatch is explicit `mapping_not_found`; any metadata-only policy is explicit, versioned, unambiguous, and never entered from ID mismatch. | DJ V3 IMPLEMENTATION ACTIVE / KDMX REPAIR OPEN |
| FC-05 | Project JSON is deserialized into separate project/mapping structs without rejecting unknown mapping keys, so a typo can silently delete DJ triggers. | One versioned project document owns mappings, rejects unknown/future fields, and migrates supported v1 exactly once to canonical v2. | OPEN |
| FC-06 | Missing Bank, output, layer, DMX mode, or fixture mode selection is replaced by first/default/fabricated state. | Preserve stale identity, visibly lock the operation, and require explicit reselection; exact identity mismatch changes no other object. | OPEN |
| FC-07 | Root Timeline transport uses a newer authority while child play/seek, metronome, loop, and old root commands remain fire-and-forget and registered; count-in is clamped. | One receipt/fence protocol owns root and child play/seek/loop/metronome; invalid count rejects unchanged; old commands are unregistered. | OPEN |
| FC-08 | DJ machine status/NIC errors become unavailable/empty UI, and legacy and generic wire envelopes remain accepted without a retirement gate. | Preserve typed failure reason; exact current envelope only; old wire unregistered after a bounded peer migration; wired restart evidence passes. | STRICT PEER WIRE V3 IMPLEMENTATION ACTIVE / KDMX OPEN |
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
| FC-25 | The shared Bank authority now closes the Bank editor, shelf, executor, Control/Edit Lighting Cue Pads, editable Touch, outer and inner Timeline placement, and delayed callbacks. Fault clears armed state; orphan events remain unavailable without fabricated Cue/Bank IDs. A production-shaped fixture hook and strict terminal-authority harness expose real authority transitions without a permissive empty snapshot; its first `R+1` run exposed and then proved the immutable-clone Undo repair. Independent Ox review passed with no Blocker/High/Medium. A flash activation now retains a safety-release token and releases when authority faults or the surface exits Live; delayed Store Scope writes report instead of silently discarding a stale authority capture. | One immutable authority result gates every Bank mutation, Scene placement, executor, Control/Edit/Live/Touch action, outer and inner Timeline surface, and delayed callback before rendering and again at invocation time; fault clears armed state, sends zero new activation IPC/runtime commands, permits the safety-lowering release half of an already-issued flash, preserves visible invalid bindings, and never fabricates Cue/Bank IDs. | FOCUSED IMPLEMENTATION, OX REVIEW, AND 5-SIZE PROOF PASS / NATIVE OPEN |
| FC-26 | A non-null selected Bank identity that disappears during polling is silently replaced by the first remaining Bank in shared helper, App selection effect, Playback Executor create state, and Scene Matrix selection. A contract test currently blesses missing Bank 1 becoming Bank 7. | `null` initial selection may explicitly adopt the first authoritative Bank once; any non-null missing/stale selection remains preserved as unavailable and locks dependent actions. Only a successful explicit delete transaction may select a documented successor. Tests must reject missing-ID-to-first-Bank fallback. | P0 REVIEW FINDING / REPAIR REQUIRED |
| FC-27 | Bank authority does not validate `CueListSummary.active_cue_id`; UI can display a Cue from another Bank as active, and engine load can carry a malformed active-Cue reference into effect activation. Existing sanitization may silently clear some malformed values instead of rejecting the document unchanged. | An active Cue reference is null or an exact unique Cue in the same exact Bank. Invalid, missing, duplicate, or cross-Bank references reject load/mutation unchanged and close every dependent UI/runtime surface; no label-only display, activation, or silent clear is permitted. | REVIEW FINDING / FRONTEND AND ENGINE REPAIR REQUIRED |
| FC-28 | Video core has a payload-less exact-black artistic admission. The refreshed exact-linker suite passes 159/159 with 1 ignored. A historical full dependency clippy run exposed 12 first-party large-error warnings around `VideoOutputArtisticRejection`; that warning state is superseded by the complete `ff61a6d` warning matrix, which records zero first-party warnings, and suppression remains forbidden. The NDI/Spout repair passes its focused suites and makes current-project blackout dominate historical Follow snapshots, binds exact route kind/endpoint, rechecks Follow identity, and separates pre-send revocation from SDK failure. Functional completion is still open: Display and all external transports require the engine-owned Prepared to Committed presentation token to close SDK/present TOCTOU, and Display still uses legacy preparation/raw presentation paths and hides zero-area. | Every physical sender/presenter consumes the evidenced artistic result, rechecks complete current output/safety/project authority immediately before I/O, treats non-Fresh as typed failure or exact-black safety output, updates LastValid only from Fresh, exposes blackout telemetry, rejects zero-area unchanged, and has no reachable legacy/frame-only success path. | WARNING MATRIX PASS / ENGINE TOKEN AND DISPLAY OPEN |
| FC-29 | `App.tsx` is currently 28,159 lines / 1,199,580 bytes and `main.rs` is 123,293 lines / 4,717,427 bytes. Bank/Scene, project transaction/recovery, Timeline, external control, ASIO, video presentation, and fixture/browser routes still converge in these two files, so authority ownership is difficult to audit and native-only divergence can hide behind a frontend pass. The new `bankAuthority.ts` and `projectTransactionRecovery.ts` extractions are useful starts, not architectural completion. | Each mutation domain has one typed coordinator/controller module and one registered backend handler surface; `App.tsx` composes UI only, `main.rs` composes Tauri/state only, generated command classification rejects unowned routes, and domain tests prove no fixture/browser/recovery/legacy parallel mutation path. Extraction must preserve the accepted show contracts one domain at a time; a risky monolithic rewrite is forbidden. | STRUCTURAL DEBT CONFIRMED / INCREMENTAL EXTRACTION STARTED / COMPLETION OPEN |
| FC-30 | Several browser acceptances install `__TAURI_INTERNALS__` only after App mount, after owner registration has already failed closed, then use synthetic untrusted `.click()` events that cannot re-arm registration. Live Audio, Operator VJ, Auto VJ, Control Learn, Setup I/O, Scene Settings Save, and the separate Cue Audio gate can therefore fail spuriously or exercise a fixture-only path; permissive unknown-command catch-alls can also hide missing startup IPC. | Every post-mount mock explicitly validates owner registration and Program Audio startup payloads, rejects unknown commands, and uses one real CDP pointer/keyboard action before the first native-bound operation. Focused gates distinguish fixture/layout proof from native IPC proof and no total-order assumption is imposed across independent startup effects. | ROOT CAUSE CONFIRMED / COMMON HARNESS REPAIR ACTIVE |
| FC-31 | Machine-local Live Audio/ASIO selection restore was placed inside the operator unlock action, so an ordinary restart never restores the saved exact device. The static gate only compared loose source-string ordering and falsely passed. | Restore runs once in App mount before backend discovery, never inside unlock; saved ASIO starts stale/locked, becomes ready only after one exact catalog/capability match, and remains locked on missing or ambiguous identity. Static scope checks and a restart browser acceptance prove the route. | PRODUCT BUG CONFIRMED / REPAIR ACTIVE |

Closing a row requires implementation plus a negative proof that the retired or
invalid path is unreachable. A comment, warning, disabled UI button, or green
browser layout is not row closure.

## HISTORICAL — 2026-08-26 01:57 JST parallel checkpoint

At that historical checkpoint, the branch was `codex/syndocal-v1.2`; the last pushed HEAD was
`573382543670fd129de7f90db1f92e32d990a98c`. That checkpoint contains only the
Windows PowerShell 5.1-compatible pinned-ASIO-SDK preflight repair and its
acceptance-document correction. The integrated show source described in this
section remained dirty and was not yet a distributable alpha.12 artifact.

- DJ-Link cadence/liveness: the current engine lane distinguishes ordinary
  500 ms-or-longer forward samples from a genuine locate using elapsed
  monotonic observation time, rebuilds active Scene/FX/step state on a genuine
  locate through existing recall machinery, and clears the observation on
  release/stop/source changes. Focused engine tests pass 6/6, filtered I/O
  historical strict-v2 tests passed 28/28 at that checkpoint, and exact-linker
  checks reported zero first-party
  warnings. An unrelated 75 ms performance budget failed once at 78.5 ms while
  concurrent builds were running and passed alone in 0.03 s. Outbound ownership
  truth equality, same-session ACTIVE dedupe convergence, unmapped-session
  isolation, and immutable Release receipt follow-ups are active; therefore no
  wired or authored-show acceptance is claimed.
- ASIO: an exact Ampero Mini ABI/schema-v2 60-minute run is active under
  `target/qa/asio-ampero-v2-soak-20260826-014416`, selecting only
  `asio:HOTONE AUDIO USB Audio Device` at 44.1 kHz, 2-channel i32, 128 frames.
  The first valid sample reported 5,174 callbacks with zero XRUN, terminal,
  non-finite, frame-mismatch, or callback-gap faults. Attempt 1 failed closed at
  time zero because of a QA-harness pre-callback check and is retained as
  rejected evidence; attempt 2 is the only candidate soak. Completion still
  requires the full duration, native alpha.12 UI/persistence/fault proof, and
  latency measurement.
- Three-display route: `get_video_output_window_observation_v1` now reads only
  the current Tauri app window registry and maps configured Display output IDs
  to exact `video-output-{id}` HWNDs. IDs and HWNDs are canonical positive u64
  decimal strings; missing windows are closed/null and mismatches, zero,
  precision loss, unknown fields, and overflow fail closed. Rust observation
  tests pass 3/3 and the strict frontend receiver contract passes. The
  PowerShell final-show runner's direct native command-result transport and an
  independent Ox review remain active; title-only or operator-authored JSON is
  still not acceptance.
- Timeline graph: an independent audit found that project load and Cue/Bank
  removal could admit a missing, self-referential, duplicate, cyclic, or newly
  dangling child-Timeline graph even though the single mutation route was
  strict. Global load-boundary validation and removal preflight are active.
  Valid deep acyclic nested Timelines remain required; the repair must not ban
  Super Scenes or create a parallel child transport.
- Live Audio restore: a real post-mount CDP gate reproduced a visible backend
  select reset to the first WASAPI option even while the authoritative saved
  identity and every backend request remained ASIO. Controlled select
  hydration, current-catalogue re-probe at Refresh/Start, exact ready+built
  dispatch gating, backend restore-verdict display, and a ready-to-fault
  zero-Start proof are active. No WASAPI substitution is accepted.

These items are concurrent implementation evidence, not closure of the
corresponding SHOW-P0 or FC rows. The next checkpoint must rerun integrated
gates on the final on-disk state before commit and push.

## 2026-08-26 02:30 JST parallel checkpoint

The branch remains `codex/syndocal-v1.2`. The code/doc checkpoint underlying
this report is the pushed `1aa2dbd099d85d237b75887954a67f67e4700bcb`; the
integrated alpha.12 source remains intentionally dirty under disjoint owners.
No release or native-completion claim is made from this checkpoint.

- ASIO hardware soak: the exact Ampero Mini bridge-v2 candidate remained on
  `asio:HOTONE AUDIO USB Audio Device`, 44.1 kHz, 2-channel i32, 128 frames,
  through 39.01 minutes. It reported 806,527 callbacks (latest bridge telemetry
  807,223), zero XRUN/API/terminal/warning/non-finite/frame-mismatch faults,
  16 ms maximum callback gap, 4,095 ns callback p99, and 479,600 ns callback
  maximum. The exact DLL hash and device/configuration stayed unchanged and no
  fallback occurred. This is still an in-progress show-specific soak; the
  authoritative matched 48 kHz ASIO/WASAPI acceptance and native UI/restart/
  persistence/fault legs remain open.
- Three-display observation: the runner now evaluates only the app-owned
  `window.__syndocalReadVideoOutputWindowObservationV1()` seam on every CDP
  page. That reader self-verifies the current Tauri window label is `main`, and
  the runner requires exactly one successful self-verified main result while
  every non-main page rejects. No nonexistent raw window-label command remains.
  Windows PowerShell 5.1 self-test passes 39/39, the strict observation gate
  passes, TypeScript passes, and focused first-party warning count is zero.
  Independent Ox re-review and the final physical three-display run remain
  open; the self-test is not promoted to physical acceptance.
- Project transactions (FC-01): read-only audit confirms that the dirty
  frontend no longer swallows the first failed Cancel, retains one exact frozen
  terminal recovery, and blocks the next raw mutation until exact recovery.
  The focused project-transaction gate passes. Release-blocking proof remains:
  one Rust/native sequence must cover Commit failure, Cancel failure, worker
  completion, same-ticket terminal recovery, and one succeeding next mutation.
  The audit also found that a terminal Commit/Cancel receipt whose ACK reply is
  lost can survive owner retirement and eventually exhaust the 256-receipt
  admission cap; dead-owner terminal receipt compaction is required without
  compacting Pending or indeterminate state.
- Detached Stage/Timeline (SHOW-P0-3): static and browser evidence now accounts
  for integrated Stage, Source shelf, no duplicate Timeline, main-window
  reflow, exact-label child census, and present/absent/unknown recovery. No
  current product-code route reproduced the original four symptoms. Closure is
  still blocked because the real native harness executes only Stage-first
  detachment and has no restart-with-detached-record child-present/absent leg.
  An Ox implementation lane owns those missing native sequences; unknown child
  presence stays explicitly unverified unless a real safe native seam proves it.
- Live Audio/ASIO presentation: strict backend-summary and `asio_selection`
  parsers, all-or-nothing duplicate rejection, machine-local stale-selection
  preservation, and focused tests pass. Independent Ox review found a show-P0
  circular lock: a restored ASIO verdict disabled the Start control even though
  native revalidation currently occurs inside that Start request, and the same
  verdict also disabled an explicit WASAPI request. Repair is active to keep a
  saved fast path locked while permitting an explicit operator ASIO request to
  perform exact native revalidation, scope the verdict to ASIO, surface
  serialization failure, and use one shared dispatch predicate.
- Nested Timeline authoring: engine load/removal graph validation passes the
  full engine suite (855 passed, zero failed, two ignored) with zero first-party
  warnings and independent review active. A separate browser-gate lane found a
  production bug in child editing: external placement compares only root
  Timeline event IDs after mutating the exact child Timeline, so a valid child
  placement can be persisted and then reported as failed. The App owner is
  repairing the before/after comparison against the active authored root or
  exact child Timeline before the child-recursion gate can truthfully land.
- Historical DJ strict-v2 convergence: the focused exact-linker same-session ACTIVE dedupe
  test now passes with zero first-party warnings after preserving the deliberate
  TTL test boundary. Outbound truth equality and socket shutdown tests remain
  active; the DJ-PC process remains intentionally stopped, so hardware remains
  0/12 and no LAN acceptance is claimed while the operator sleeps.

Focused gates at this checkpoint emitted zero first-party warnings. Whole-tree
`git diff --check` passes; its output contains only existing Git LF-to-CRLF
working-copy notices. The next source checkpoint must integrate the active
repairs, rerun their focused negative proofs, receive independent review, and
then create and push a meaningful code commit.

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
source-shelf contract and click/drag parity, localization (`3569/3569`, zero
unprotected user-data labels), right-click suppression, Scene Settings at all
five supported browser sizes, Scene Matrix full authoring at all five sizes,
and the production-shaped Scene Matrix cross-Bank transaction/Undo gate at all
five sizes. The comprehensive Scene Matrix fixture covers Bank create/custom
name/automatic name/normalized-duplicate rejection, context-menu rename/delete,
keyboard opening/escape/focus restoration, empty-Bank single `+ Scene` action,
Scene rename/duplicate/delete, Bank reorder, and Undo/Redo.

The latest integrated frontend rerun advances localization to `3569/3569`
with zero unprotected user-data labels. `check:scene-settings`,
`check:scene-matrix-strip-drag`, `check:cue-store-form`, and the Timeline source
shelf contract/click/drag placement gate all pass. The source-shelf static gate
was also tightened to the current Scene Matrix fixture-only snapshot seam: it
requires the local fault-injection state and the cloned snapshot hook to remain
inside the exact `viewportFixture === "scene-matrix"` block, and still requires
teardown deletion. This is test maintenance, not a relaxation of production
authority or a native acceptance claim.

After independent Ox review, the momentary Touch flash path was hardened so an
activation retains its exact safety-release half even if full Bank authority
fails before pointer/key release. `check:scene-live-viewport` passes again at
1920x1080, measured 1920x1032, 2048x1152, 1366x768, and 1280x720, including an
injected mid-press authority fault: Cue 320 is active before the fault, the
authority error is visible, and the Cue is no longer active after the fault.
Stale debounced Store Scope writes now produce a visible discard message.

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

The former single-window native 4K runner is retired. It moved the main editor
onto `DISPLAY3`, which now belongs exclusively to the projector output; retaining
that route would make the final show contract ambiguous. The three-display
runner is the only final-show entrypoint: `DISPLAY2` hosts the editor,
`DISPLAY5` hosts the LED output, and `DISPLAY3` hosts the 3840x2160 projector
output at DPI 144 (Windows 150%).

The typed, read-only output-ID-to-HWND observation route and its fail-closed
three-display harness are now implemented. The harness binds each configured
output ID to its exact label, `video-output-<ID>` native label, live-open state,
and native HWND; titles or operator-authored JSON do not count as that proof.
It also revalidates the title-selected HWND immediately before the app-owned
observation, sanitizes hostile observed labels before diagnostics, and states
truthfully that it proves only the three named show roles inside the current
five-display topology. On 2026-08-26 both Windows PowerShell 5.1 and PowerShell
7 deterministic suites passed `56/56`; an independent read-only review found
no remaining P0/P1 in the runner or companion self-test. `git diff --check` was
clean apart from Git's informational LF-to-CRLF notices.

The exact clean alpha.14 standard artifact exists at checkpoint `92122f1b`,
while the only built Show-ASIO artifact remains the historical alpha.12
local-only checkpoint `ff61a6d`; configured physical acceptance remains open.
The runner still requires the
real Apply/output/content sequence for Editor 1920x1080, LED 1920x1080, and
Projector 3840x2160, plus the exact artifact hash/version/HEAD, raw
DisplayConfig identities, exact GDI role names, exact output IDs/labels, and
full physical client/monitor-bound equality. The `56/56` result proves only
deterministic harness contracts; it is not a native visual-content or hardware
claim.

## Recoverable build-artifact cleanup ledger

Current authority correction (2026-08-26): cleanup safety is pushed through
`c40cfd8` (including predecessor `ef7b647`); this is evidence of the guard, not
evidence of deletion. The production Plan admits exactly one candidate path,
`target/debug/incremental`, but the candidate is currently blocked by
`HardlinkDetected`. Apply was not run, no path was deleted, and reclaimed bytes
are **0**. Every other path listed in the dated inventories below is currently
protected; any ASIO-named path is additionally blocked until its exact owner,
artifact identity, and evidence-retention boundary are revalidated. The older
sizes and candidate dispositions below remain inventory history only and do not
expand the current deletion allowlist.

The workspace-size audit found no evidence that old authored source is the main
capacity consumer. A 2026-08-25 `du -h -d 1` measurement identified Cargo
outputs as the dominant recoverable footprint:

The 2026-08-26 active-writer re-audit supersedes the earlier size snapshot for
deletion decisions: `target/debug` is now 147,260,456,081 B (including about
95.29 GB under `deps`, 11.74 GB under `build`, and 38.26 GB under
`incremental`); `target/release` is about 10.29 GB; `target/asio-hardware-qa`
is about 428 MB; and `target/qa` is about 256 MB. The safe-now deletion set is
empty because current Cargo/Ox writers and the Ampero soak own recent outputs.
Deleting any of them now would race proof generation and waste the show
critical path. After all writers exit and the integrated checkpoint is clean,
committed, and pushed, only `target/debug/incremental` and the separately
revalidated `target/asio-hardware-qa` are pre-accepted cleanup candidates;
`target/debug/deps` requires a new hard-link/owner review. Keep release, QA,
near-show media, the pinned SDK/archive, and `target/tmp` (which contains
user-authored test/OCR and stash/worktree material) unless a later exact audit
proves a narrower named target disposable.

| Generated path | Measured size | Disposition |
| --- | ---: | --- |
| entire `target/debug` | 133,365,652,702 B | Remove only after the integrated checkpoint is committed/pushed and all Rust writers exit; fully reproducible. |
| `target/asio-audit-*` | 1,135,276,560 B | Remove the four named audit trees after the same writer/checkpoint gate. |
| `target/asio-v2-*` | 1,255,903,975 B | Remove the four named isolated v2 build trees after the same writer/checkpoint gate. |
| entire `target/release` | 9,963,099,449 B | Retain until the accepted native executable and evidence are replaced by the final exact-source release build. |
| entire `target/qa` | 256,313,098 B | Retain current acceptance evidence; prune only named superseded runs after the final evidence set is recorded. |
| `target/asio-qa` | 1,044,918,757 B | Retain the current accepted ABI-v2 bridge evidence. |
| `.git` | 316 MB | Retain; `git count-objects -vH` reported only one 568 KiB temporary object, not the capacity cause. |

Four untracked `three-display-show-*` runs from 2026-08-25 were inspected as
`accepted:false` / `native_hardware_claim:false` failure or unconfigured
dry-runs, then their 26 reproducible files were removed. Their empty directories
contain no evidence and no longer appear in Git status. The accepted physical
display correction artifact was retained and pushed in `0d9f4d5`.

Do not clean `target/debug` while Cargo owners are running: doing so would race
their compiler outputs and waste the near-show critical path. Build artifacts
are recoverable by recompilation; authored `.sdc` projects, source media,
licensed SDK input, accepted QA evidence, and the final release executable are
not cleanup targets.

The 20:51 JST recheck initially found no immediate deletion target that passed
every ownership and writer gate. After the peer strict-v2 source and handoff
checkpoints were pushed, the incomplete `rb-output/dist` tree was rechecked as
Git ignored, tracked-file zero, reparse-point zero, process-reference zero, and
stable across two samples, then its exact directory was deleted: 32 files and
91,752,038 logical bytes, recoverable with `npm run build:dist`. `target/debug`
and the eight named ASIO audit/v2 trees remain protected while Rust writers are
active. Cargo hardlinks mean later physical reclaimed bytes may be lower. Every
future deletion still requires an exact absolute-path, reparse-point, Git
tracking, hardlink, and active-writer recheck immediately before execution.

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
