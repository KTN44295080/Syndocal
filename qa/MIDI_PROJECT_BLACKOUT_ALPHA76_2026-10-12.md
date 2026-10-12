# MIDI Learn, new project and blackout — alpha.76 candidate

Branch `codex/midi-project-blackout-repair-20261012`, base `9e6f5d57645c9533afe87bd3cc3aec31b1ac4e4c`.
The primary dirty checkout and installed alpha.75 are protected. No subagent or Computer Use is used.

The user reproduced mapping synchronization and SAFE-release errors on installed alpha.75 and reported project New doing nothing. Control's pink-target MIDI Learn is the confirmed failing workflow; Setup also needed clarification and verification.

Changes: retain a local mapping edit across a known revision CAS conflict only when the server's mapping baseline and replacement generation are unchanged; reject foreign mappings, epoch/history/owner changes and non-CAS failures. A clean-but-still-settling mapping ACK is joined. The native request allocator now includes admitted request IDs which have no durable lease receipt, while replay rejection remains intact.

New and MIDI Learn run blocking device/lifecycle work off the UI event loop. New clears the temporary retirement latch only for the exact new empty project with every output disabled. A newer project cannot be altered. The independent SAFE header control is removed; its runtime zero state is reflected and released through the existing DMX BO control. Hardware fault shutdown and operator Full Lock emergency admission remain internal.

Learn is single-flight across Control and Setup. Setup releases its existing Control input before Learn and restores it after success/timeout. Control captures pointerdown before a fader can start its ordinary mutation. Waiting is advertised only after the MIDI port is open, using a request/epoch-bound event subscribed before dispatch; that listener is removed on every terminal path. The IO callback retains one first sample; disconnect is a distinct failure. Mapping ACK still precedes reconnect and stale-project results never create mappings or reconnect inputs.

Setup now exposes its input selector and direct Control Learn entry; Clock/MTC and manual mapping are disclosures. User channels are 1–16 with All channels, retaining the existing 0–15/null wire representation. Selecting the placeholder now clears the selected device instead of selecting port 0. Busy Learn disables duplicate capture/add operations.

Final source checkpoint: `check:release` and TypeScript passed. Windows native tests: 2,056 passed, 0 failed, 21 ignored; focused native/IO regressions: 15 distinct passed. Mapping CAS/in-flight ACK checks passed (13 assertions). Actual Solid header rendering passed at six widths (960 through 3840), retains hit sizes, has no overlap and no independent SAFE button. Setup rendering and production Control capture passed at 1280/1920 with no browser errors. Normal alpha.76 no-bundle build passed; its exact executable launched as one responsive maximized Syndocal window, rejected unauthenticated reads, and closed gracefully. Windows packages, isolated native New/BO operations, hosted Win/Mac checks and physical hardware acceptance remain pending.

The source checkpoint is staged before hosted packaging so its source revision can be tested. Native tests/build emitted no first-party compiler warnings; the existing Vite chunk-size advisory remains baseline 1/current 1/delta 0. No independent subagent review is performed, following the explicit user prohibition.

The header/viewport test contract changes follow the requested UI: input is always visible, optional Clock controls are disclosed, and a runtime DMX blackout uses the existing DMX control. Assertions for visibility/containment and blackout state remain. Physical MIDI reception on the user's controller, actual USB signal continuity and Mac GUI are unobserved; do not claim these software results prove them.
