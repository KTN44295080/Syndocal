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
| SHOW-P0-2 | Timeline lower-right uses the same authoritative Bank/Scene representation as Lighting, with placement-only behavior | The compact representation exists in the dirty tree, but `cueLists` is not yet connected from `App.tsx`; fallback grouping can lose Bank name/order, and an all-empty Bank set can disappear. | Same snapshot produces identical Bank ID/order/name/color and Scene ID/order/number/name/type in Lighting and Timeline; empty Banks remain; Timeline exposes no CRUD/play path; click and drag place the same Scene Block. | BLOCKED BY INTEGRATION |
| SHOW-P0-3 | Detached Stage/Timeline panes never dual-render, disappear, or leave unused main-window space | The previous startup recovery could treat failed placement capture as child absence. A fail-closed exact-label child census is being implemented. | Browser state-machine proof plus a fresh exact-checkout native run covers both detachment orders, restart with detached records, child present/absent/unknown outcomes, Stage integrated rendering, Timeline source/timeline separation, no duplicate pane, and no unusable main-window void. | BLOCKED BY INTEGRATION |
| SHOW-P0-4 | `DSF2026.dvc` drives representative Lighting, LED, and projector content on the intended physical routes | The exact source project exists at `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`. Temporary resolution-specific media and a reproducible generator now exist, but the DVC exact-import report and physical output flow remain unexecuted. Prior native D4 and pane evidence does not prove this next-show flow. | Exact/Skipped importer report, representative Lighting scenes, generated LED/projector assets, authored `人生オーバー`/`惑う星` Timelines, and fresh alpha.12 native three-display output complete, save, restart, and reload without substitution. | IN AUDIT |
| SHOW-P0-5 | DJ-Link survives setup, authenticated wired operation, disconnect, and app restart | Current acceptance remains 0/12. The observed DJ-PC peer was an older build whose setup API returned 404; alpha.11 listener/token/NIC state did not satisfy restart reuse. | Identity-bound `beta-v1.1.2` peer is committed and pushed, both PCs run the intended artifacts, and HW-4.1 through HW-4.12 are recorded over wired LAN, including pedal, loop, dedupe, reconnect, restart, and concurrent Art-Net/sACN. | BLOCKED BY IMPLEMENTATION AND HARDWARE |
| SHOW-P0-6 | The distributed artifact exactly matches the accepted source | Current source is dirty and remains version `1.2.0-alpha.11`. | Focused gates, warning ratchet, synchronized alpha.12 metadata, `check:release`, exact VS2022 14.44 linker, `pnpm --dir app tauri build --no-bundle`, exactly one responsive maximized exact-path window, evidence update, meaningful commit, and successful push. | NOT RUN |
| SHOW-P0-7 | ASIO is explicitly selected and remains truthful through start, live callback I/O, stop, fault, and restart | Short successful TOPPING and HOTONE runs exist, but the advertised matrix, occupied-driver/control-panel/reset/resync/XRUN/unplug faults, one-hour soak, and latency evidence remain open. | The actual show driver/rate/channels/native format/buffer are persisted and revalidated; Start/Stop/Free, callback continuity, occupied/unplug/XRUN/no-callback recovery, restart, one-hour soak, and measured latency pass without WASAPI or another-driver substitution. | BLOCKED BY IMPLEMENTATION AND HARDWARE |
| SHOW-P0-8 | `人生オーバー` follows DJ-Link position/loop/pedal handoff and auto-transitions to `惑う星` | Required primitives exist in partial form, but the exact authored Timelines and full DJ-to-band-to-next-song sequence have not been executed. | Wired authenticated Master-track trigger, position follow, absolute 8-beat and half-loop updates, pedal stop/band continuation, Timeline completion, and BPM-aware automatic `惑う星` transition pass end to end with duplicate/stale/disconnect/restart cases. | BLOCKED BY IMPLEMENTATION AND AUTHORING |

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

## Required final operator sequence

1. Start from no running exact-checkout Syndocal process and a known saved show
   project; build and launch the exact accepted source.
2. Verify one responsive maximized Syndocal window on the intended real 4K
   display and confirm all three show outputs have explicit identities.
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
