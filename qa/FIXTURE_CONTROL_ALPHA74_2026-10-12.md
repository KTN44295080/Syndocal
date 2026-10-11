# Control fixture faders and bulk repatch — alpha.74 candidate

Branch `codex/patch-bulk-alpha74-20261012`, base
`6f4adfe91651b21b94b541c8c0d6fdac3060465e`.
The primary dirty OneDrive checkout and distributed alpha.72 remain protected.
The user identified Windows alpha.72 Control fixture faders; FTDI USB-DMX is
connected. Physical-device timing has not been measured in this checkpoint.

## Changed behavior

Fixture and fixture-type/group base faders send intermediate values through one
bounded history ticket instead of opening, capturing and committing the entire
project for each input. The latest pending value for each control replaces older
unsent values, preserving chronological order between overlapping selections.
Tickets rotate after two seconds or 512 samples and close after 120 ms idle.
Native admission borrows the existing checkpoint, caches validated targets and
enforces exact window/owner/epoch/sequence. Each sample uses the existing engine
batch; physical USB scheduling is unchanged. Programmer controls keep their
existing runtime path. Failed writes clear only their matching optimistic value.

Commit produces one Undo entry per bounded ticket. Cancel retains already-live
values as an interrupted Undo entry, matching existing history policy. Project
replacement cancels the old input generation; it cannot write into the new
project. Long uninterrupted input may produce several bounded Undo entries.

Patch grid pointer dragging now preserves DOM cells during live snapshot updates.
Ctrl/Cmd toggles fixture selection; Shift selects a range; Move Selected works in
the list or grid. Bulk dragging retains the grabbed channel offset and relative
fixture address spacing. Escape/pointer cancellation applies nothing. A single
published backend operation validates all expected addresses/footprints and all
destination collisions before changing any fixture. Invalid/stale changes fail
as a whole. Values, profiles and groups are preserved, with one Undo/Redo entry.

The default DJ title filter is empty. Explicit mappings saved by a user remain
valid; an extension manager or optional integration installer is not implemented.
The Windows NSIS hook was regenerated from the complete-removal source, including
the exact product HKCU preferences key. Updates retain user settings and their
existing installer format. Full removal preserves external projects/media and
shared Microsoft/WebView2 runtimes.

## Evidence and current boundaries

- TypeScript and the alpha.74 release aggregate passed, including 484 frontend,
  564 native and 282 engine inventory contracts, stream controller regressions,
  production history-controller scenarios and twelve pure repatch-policy cases.
- Windows focused native tests passed 53 cases (stream/history 4, bulk DTO/history 2, inventory/coordinator 42, exact renderer envelopes 5). Engine repatch passed 3; first-party compiler warnings observed were 0. The Mac artifact validator passed 169 tests.
- Headless Edge rendered the actual Solid patch component. Single drag, multiple
  selection/drag, snapshot refresh during drag, grabbed offset, pointer cancel
  and list movement passed. The old HTML drag also worked in this harness;
  the originally reported native drag failure was not reproduced.
- The existing patch viewport matrix passed at 1920x1080, 1920x1032, 2048x1152,
  1366x768 and 1280x720. Two cold-start harness attempts failed to mount the shell;
  no cause is inferred. Failure diagnostics were added without weakening checks.
- Windows uninstall/removal filesystem and registry-seam tests passed 16 cases;
  they did not uninstall the user's application or touch its real credentials.
- The exact MSVC14.44 no-bundle build passed. Rust compiler warnings observed in
  that configuration were 0; the existing Vite chunk advisory remained 1.
- The user saved and closed the installed alpha.72. The ordinary alpha.74
  executable at source `3ae8d7818d2eba7733e29f680a77ba58ef523604` passed one
  visible, responsive, maximized native window and unauthenticated broker-rejection
  checks, then closed gracefully. No unrelated process was terminated.

The private native executable built from `5a70a371bafbff04d83e2bf396a60b8b698ada56`
passed 32 backend checks: DMX/VID BO, replay and stale rejection, receipt
retention, restart/ownership retirement, 60 intermediate fixture-fader writes,
one Undo/Redo and atomic bulk repatch. All physical outputs were disarmed.
Mean fader IPC acceptance was 1.754 ms, maximum 3.147 ms; these are queue-acceptance
measurements, not physical USB timing or actual Control DOM interaction.
The exact-case target correction at `3ae8d781` subsequently passed four focused
native stream/history tests.

Linux full-workspace CI found one regression: the canonical repatch validator
had changed the existing single-patch collision error prefix. Restored the
`DMX address conflict in universe` contract in product code, preserving the
atomic rejection and adding both source/destination ranges. No assertion was
weakened. Three bulk repatch and three existing invalid-patch tests passed
locally after the correction. Updated the private acceptance helper to read
published runtime snapshots while a history ticket is active; full checkpoint
and history reads still reject during that ticket. Bounded observation waits
require the exact intermediate/final engine value; mutations are never retried.

Multi-universe boundary: the engine stores/render-sends multiple routes, but
`createOutputDiagnosticsController.applyDmxOutputRoutes` currently rejects UI
route changes. The managed USB route is fixed to internal U0 and requires its
exact Unity loopback mirror. General routing and multiple USB ports are not
complete product features and have not been released or hardware verified.

Win/Mac packaging, hosted installer lifecycle and artifact hashes will be
recorded here when observed.
No package is delivered yet. Apple Silicon GUI, minimum-OS execution, Developer
ID signing/notarization, USB signal continuity and venue acceptance remain open.
