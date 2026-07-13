# Source-Linked Scene Blocks Acceptance

Updated: 2026-07-14

## Implemented boundary

A Scene Block is a timeline placement that keeps a live `cue_id` reference. It does not copy the Cue body and it does not create a separate preset file. Editing the source Cue therefore changes every placement the next time that placement triggers. The only persisted representation is the existing timeline event inside the project `.sdc`.

| Field | Scene Block contract | Legacy point contract |
|---|---|---|
| `cue_id` | Existing source Cue; resolved at trigger time | Existing source Cue |
| `time_ms` | First trigger position | Point trigger position |
| `duration_ms` | Greater than zero; interval between loop starts | `0` |
| `loop_count` | `1..=256` | `1` |
| `jump_to_event_id` | Optional existing timeline placement | `None` |

For a block with start `S`, duration `D` and loop count `N`, the latest source Cue triggers at `S + kD` for `k = 0..N-1`. The block ends at `S + ND`. Under internal Timeline Play, crossing that final end with a jump target lands on the target placement start and triggers its latest source Cue exactly once. A landed Cue is not triggered again on the next tick, and jump targets do not form a same-tick chain. While MTC, LTC or MIDI SPP owns the timeline position, the external source remains authoritative: local clock advancement and local jumps are suppressed, while crossed or landed loop boundaries still trigger exactly once. Pause/resume preserves that external ownership and does not replay a consumed boundary; an explicit internal Play or Seek releases it, and Seek rearms an exact landing boundary once.

The runtime timeline duration includes the complete block span. Add, Set and Remove reject missing source Cues, missing jump targets, zero-duration blocks, loop counts outside `1..=256` and arithmetic overflow. Removing a referenced event or Cue clears incoming jump references. Ordinary point-event editing preserves block-only fields instead of silently converting a block into a point.

## Publication and persistence safety

- Add, Set and Remove have a two-second execution deadline and wait for the published engine snapshot before acknowledging success.
- Snap Items publishes all Scene Block placements, lighting automation and video automation as one operation and one Undo transaction; late validation cannot leave a partial snap.
- Snapshot contention restores the prior timeline events, lighting/video automation, timeline position, boundary/jump-landing guards and last error.
- Project load validates the same source, duration, loop, jump and overflow constraints before mutation.
- Legacy `.sdc` events that omit the new fields deserialize as `duration_ms = 0`, `loop_count = 1`, `jump_to_event_id = None`.
- Scene Blocks round-trip only as part of the project `.sdc`; there is no `.scene-block` preset and no destructive Cue duplication.
- Undo/Redo history stores the linked placement fields together with the rest of the project state.

## Operator surface and viewport contract

Control > Timeline provides a dedicated Scene Blocks workspace with source Cue, start, duration, loop count, visual lane and Continue/Jump controls. A lane only controls timeline organization: every trigger recalls the complete linked Cue, including its lighting, video, node-graph and Effect targets. Each row identifies the live link and exposes the current source summary. The overview represents the full `duration x loop_count` span rather than drawing a point marker, while legacy events remain point markers. Drag, nudge, snap and save operations use the block-aware Set path; removal selects the matching block or point command.

The primary automated browser gate is 1920x1080. The Scene Blocks composer, linked-source explanation, timeline overview, block rows and edit actions are judged there first. The browser harness additionally runs a 1920x1032 measured-work-area fixture; that fixture is still a browser run and is not native evidence. Native Windows acceptance is a separate path: this workstation's 1920x1032 work area produces a 1920x1009 decorated Tauri client while maximized, F11 fullscreen is exactly 1920x1080 and Esc must restore the exact 1920x1009 client. The 2048x1152 browser run is an extended ceiling regression. The 1366x768 and 1280x720 runs are containment fallbacks that must keep every control reachable through pane-local scrolling; passing only those compact layouts is not visual sign-off.

## Large-show browser evidence

- `check:scene-block-hour-viewport` passed at the 1920x1080 primary browser gate and the 1920x1032 measured-work-area browser fixture with 500 placements across a 3,600,000 ms show. Fit All, bounded 1-2-5 ruler ticks, zoom, pan, terminal selection reveal, playhead reveal, absolute-time drag and visible-window projection passed. The overview remained below 3,500 DOM nodes in this 500-placement fixture.
- `check:scene-block-overlap-viewport` passed at the 1920x1080 primary browser gate and the 1920x1032 measured-work-area browser fixture with 500 coincident placements represented by separate Lighting and Video clusters of 250 members. Mouse activation, real Tab/Enter activation, accessibility-tree discovery, time-span metadata, bounded paging/search, Lighting-to-Video filter reconciliation, pan/live-follow persistence, clear-to-500 restoration and zero outer overflow passed. This overlap-track test also owns the partially clipped drag regression: it preserved the source timestamp instead of snapping the block to the visible edge.
- The complete general `check:viewport` suite also passed at all five browser sizes. The 1920x1080 primary gate kept a 1015x323 Show Timeline workspace and the 1920x1032 measured-work-area fixture kept 1015x288; both exposed all 9 viewport controls and all 9 row actions. The compact runs remain reachability/containment evidence only.
- On this host on 2026-07-13, the observed 2–7 ms values were handler-plus-microtask state-activation measurements. The automated regression gate requires both handler and microtask state activation to remain `<100 ms`. Two-animation-frame reads separately prove that the expected DOM state is present; they are correctness checks, not timed end-to-end painted-response measurements.
- The 10,000-item helper benchmark currently measures interval clustering only. A worst-case 10,000 simultaneously visible block surface is not yet DOM-windowed or browser-gated, so no 10,000-item UI responsiveness claim is made.
- These fixtures validate DOM, CSS and input behavior. They do not invoke the Rust/Tauri backend.

## Native desktop evidence

On 2026-07-13 a separate QA-identifier desktop build (`Syndocal QA - Native Acceptance`) loaded `samples/phase1-mini-show.sdc` through the real Tauri backend, then added a linked placement and returned `Added linked Scene Block 3 at 0 ms (1000 ms × 1)`. The earlier note labeled the 1920x1032 Windows work area as a client size. The repeatable native gate now distinguishes it from the decorated Tauri content client: 1920x1009 while maximized, exact 1920x1080 after F11 and 1920x1009 again after Esc. The already-running release Syndocal and Daslight 5 processes remained separate and unchanged. This proves native command connectivity and window-mode behavior; it does not prove physical-rig output or combine the browser's 500-placement load with a native large-show run.

## Automated evidence

- `pnpm --dir app run check:release-ui` is the required release UI aggregate. It runs the existing general `check:viewport` suite and the focused `check:timeline-viewport` suite as two mandatory gates; the focused timeline evidence does not replace full-application containment, and the general suite is not lengthened with the timeline-only fixtures.
- `cargo test -p protocol --locked`: 16/16 passed, including legacy defaults, Scene Block field round-trip and atomic Snap request round-trip.
- `cargo test -p engine --locked scene_block -- --nocapture`: 10/10 passed, covering a large tick across every loop start, preservation through the legacy Set path, latest source-Cue resolution, one-shot jump landing without duplicate or same-tick chaining, exact-end Seek/load versus pause/resume, aligned-grid/fallback ordering equivalence, every pre-wait/follow occurrence, end-to-end 2,000-block dispatch, publication rollback, incoming-jump cleanup, bounded 2,000-block maximum-loop scanning and overflow rejection.
- `cargo test -p engine --locked`: 254/254 passed. Focused boundary, authoritative external timecode and MIDI SPP ownership, pause/resume versus explicit Seek, point rollback, Cue-removal rollback including queued multiplicity, atomic Snap, 4,000-event cleanup and mixed-Cue lane tests also pass.
- `cargo test -p syndocal --locked scene_block -- --nocapture`: 4/4 passed, covering project/legacy/history round-trip and strict source/timing/loop/jump/overflow validation.
- Full backend verification also passes `cargo test -p syndocal --locked` at 220 passed / 9 hardware-only ignored, `cargo check -p syndocal --locked`, Rust formatting and `git diff --check`.
- The pending trigger scheduler run-length encodes only adjacent identical Cue/due-time/source occurrences. It applies the Cue body once for each adjacent run and carries the occurrence multiplicity in `repeat_count`, so pre-wait/follow reservations are preserved without a valid-trigger drop cap. Non-adjacent A/B/A runs remain A/B/A and are not merged. Zero-due ticks allocate no Cue lookup table, while due batches resolve Cue, pre-wait and next-Cue indices once in O(C+N).
- A common aligned Scene Block grid now emits directly in chronological/event-id order through an O(K) fast path, where K is the number of emitted occurrences. Mixed or unsorted grids retain the general collect-and-sort fallback; an equivalence regression proves identical time/event/A-B-A ordering between the two paths.
- On this host, the final 128,000-occurrence debug runs scheduled in 2.66-2.75 ms and performed the coalesced pre-wait/body/follow dispatch in 0.048-0.077 ms. Repeated release runs measured 0.378-0.409 ms scheduling and 0.021-0.030 ms dispatch. Mixed 2,000-placement/128-Cue release runs measured 0.077-0.086 ms scheduling and 0.406-0.422 ms pre-wait dispatch. The earlier 56.3854 ms scheduling / 183.1903 ms dispatch result was the pre-coalescing, pre-aligned-fast-path baseline, not the final path. Each regression retains its explicit bounded-latency ceiling.
- Frontend sign-off requires block-aware atomic snap/nudge/drag/save/remove, source-Cue update visibility, full-span overview rendering, bounded/stable large-show DOM and localization coverage. The current 500-placement gate passes; worst-case 10,000-visible-item DOM windowing remains open.
- Browser sign-off requires 1920x1080 primary and 1920x1032 measured-work-area fixtures first, then a 2048x1152 ceiling run and 1366x768/1280x720 fallback containment with zero document/app horizontal overflow and all terminal actions reachable. Native maximized/fullscreen/restore acceptance is recorded separately above.

The software contract above closes the destructive-copy gap in source-linked timeline composition. It is not evidence that Syndocal exceeds Daslight 5 overall. Final production acceptance still requires an operator rehearsal with a representative physical rig: exercise loops/jumps under internal Play, exact-once loop boundaries under external timecode, source-Cue edits, Undo/Redo and `.sdc` save/reload in the same show.
