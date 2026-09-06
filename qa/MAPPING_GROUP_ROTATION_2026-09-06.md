# Selected fixture rotation

Branch `codex/syndocal-v1.2`, base
`8213f4a8072a814cd72d223bb6b8ec3f727fda6e`; product alpha.69 unchanged.

The yaw handle previously retained only its anchor fixture ID and persisted only
that fixture. It now captures the selected fixture IDs and their starting
rotations. Preview and persistence apply the same yaw delta to every selected
fixture, preserving relative yaw, pitch, roll and position. Clicking a rotate
target also applies the anchor delta to the selection. This does not orbit fixture
positions; the existing explicit group-orbit operation is separate.

The existing transform batch applies per-fixture backend transactions and checks
one refreshed authoritative snapshot. It stops on rejection and reports possible
partial application. This change does not provide atomic group Undo or rollback.
No additional per-frame IPC or snapshot read is introduced.

## Evidence and pending boundary

`app/scripts/check-mapping-rotation-persistence.mjs` exercises controller writes,
selection preservation, orientation and position preservation, duplicate events,
rejection, cancellation and stale completion. Final focused script and
`pnpm --dir app exec tsc --noEmit` passed. Added deferred-ACK cases cover project
replacement in both paths, a newer interaction that starts and ends before the
old ACK, and the unselected-anchor single-fixture fallback. Each dispatch and
completion checks the captured project epoch and operation ownership; starting
another mapping interaction permanently invalidates an older stage operation.
Independent review accepted the final production diff. The App change is one
authority-accessor wiring line; controller tests supply that required accessor.
`git diff --check` passed for the owned mapping files.

`app/scripts/check-mapping-group-rotation-browser.mjs` renders the production
mapping render model and beam layer. Starting yaws 20/350/120 become 90/60/120
when the first two fixtures are selected. Both selected SVG beams change, the
unselected beam remains unchanged, cancellation restores the original geometry,
and confirmed snapshot geometry persists after clearing the preview.
Evidence: `target/qa/mapping-group-rotation-20260906/browser.log`,
`selected-preview.png`, `confirmed.png`. Browser rendering passed and the preview
screenshot was inspected. This harness does not perform native pointer gestures
or physical output acceptance.

After the user's restart confirmation, `pnpm --dir app tauri build --no-bundle`
passed with the pinned MSVC linker (release 1m47s, Vite 6.21s). Previous accepted
native build warnings: 0; current first-party warnings: 0; delta: 0.
`target/qa/mapping-group-rotation-20260906/native-build.log` and
`native-launch.json` record the evidence. Executable SHA256:
`9A121FEEF7529D75E9A6EDDE8925F164294D39AFC67EA773E33A628568961296`.
One responsive maximized Syndocal main window was verified. The existing
`target/qa/spout-output-snapshot-20260906/DSF2026-before-output-read.sdc` was opened
through the single-instance file ingress; read-only MCP confirmed 46 fixtures
and a healthy runtime (`loaded-runtime.json`). No output was armed by this check.
Native multi-selection pointer gesture acceptance remains for the user; startup
and MCP evidence do not substitute for that observation.
