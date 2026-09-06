# Project mutation lifecycle extraction

Base `c212f08f7f53fc529946b084e96336c77bd2e70a`, branch
`codex/syndocal-v1.2`, alpha.69 unchanged.

`projectTransactionMutationController.ts` owns the post-Begin lifecycle of one
renderer-ticketed authored operation: cancellation after Begin, one raw dispatch,
published-receipt recovery, Commit settlement, and conditional Cancel. App keeps
registration, operator policy, command classification, mapping flush, authority
comparisons, Begin and server-authoritative routes. The existing terminal recovery
controller remains unchanged. This is not a relocation of all App state.

Execution receives the exact ticket, identity, command arguments and lifecycle
callbacks. Injected ports are raw Tauri transport, owner identity and the existing
four recovery/settlement functions. Cancellation retains one promise, and the
same six publication/terminal uncertainty errors prevent unsafe cancellation.
The two publication errors are exported from the new module so callers retain
the same constructor identity for `instanceof` checks. The raw dispatch is not
retained for retry or duplicated in App.

The strict nested envelope for single/group fixture transforms and cross-Bank
moves remains unchanged. No command, schema, worker or per-frame work is added;
this extraction does not claim measured runtime performance improvement.

## Evidence

Independent review accepted module equivalence and final composition wiring.
The full project-transaction production contract and exact frontend inventory
(455 commands) pass after moving post-Begin assertions to the module and adding
explicit checks for App delegation and absence of a duplicate implementation.
Evidence logs are `project-contract-extracted.log` and
`invoke-inventory-extracted.log` in `target/qa/mapping-group-undo-20260906/`.

The old raw-port regex expected `invoke: tauriInvoke` while the existing production
composition used `invoke: (command, args) => tauriInvoke(command, args)`. The check
now requires that exact raw forwarding function rather than accepting arbitrary
ports. It passed before the extraction as well; this resolves the previously
recorded checker mismatch without suppressing a product assertion.

The new executable controller checker passes six scenarios (including two hold
variants): strict group happy path, ordinary failure, not-published failure,
recovered publication, indeterminate/unconfirmed hold, and post-Begin abort.
It loads the actual production factory and dependencies. The existing recovery
controller checker and TypeScript no-emit check also pass. Independent review
accepted the test evidence; cleanup rejection and terminal exception coverage
continue to rely on unchanged shared recovery checks and source equivalence.
App's current combined diff removes 119 lines and adds 26, including the pending
group Undo composition. The new controller does not retain global application
state. No new compiler-warning measurement is claimed from this TS-only tranche.

Native release build and startup passed with the group Undo checkpoint.
See [native evidence](MAPPING_GROUP_UNDO_2026-09-06.md). Hardware and GUI gesture
acceptance are not inferred from startup/read-only MCP evidence.
