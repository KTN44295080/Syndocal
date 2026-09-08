# AI2 authored bridge audit — 2026-09-08

## Scope

This is a bounded audit of the existing authored command bridge. It records
the implemented `set_effect_enabled` vertical and the existing project
transaction mutation controller without widening the supported operation set,
adding a new API, or marking the full AI2 tranche complete.

- Source base: `5c421d3b4889877c24781d5c4706715501c44427`
- Working tree before this document: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing implementation checked

`app/src/authoredEffectEnableController.ts` owns the latest-intent lane for the
canonical rendered effect-enable action. The route uses a strict server-issued
project fence, validates the terminal receipt and outcome discriminator, keeps
the stale-fence retry bounded, and prevents a superseded completion from
replaying or rolling back a newer intent. The rendered route emits one strict
`set_effect_enabled` invoke and does not reopen the generic project-transaction
wrapper.

`app/src-tauri/src/authored_control_plane.rs` already provides the corresponding
bounded backend vertical. Its tests exercise owner/session binding, receipt
single-flight and retention, shape conflict rejection, stale/future and
expired-fence handling, retirement tombstones, NoOp receipts, publication
failure without state drift, and exact authored history navigation through
Undo/Redo.

`app/src/projectTransactionMutationController.ts` keeps the existing staged
mutation lifecycle separate from the App policy layer. It preserves the exact
ticket envelope, cancels only when publication is not unresolved, recovers a
published receipt without replaying the raw command, and holds indeterminate or
unconfirmed publication instead of issuing an unsafe Cancel or retry.

## Focused verification

The Rust command ran from the repository root after
`vcvars64.bat -vcvars_ver=14.44`, with the exact Build Tools MSVC
`14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` and returned first by
`where.exe link.exe`.

| Command | Result |
| --- | --- |
| `node app/scripts/check-authored-effect-enable.mjs` | PASS — strict rendered route, receipt validation, superseded A/B, stale retry bound, and rollback-baseline scenarios |
| `node app/scripts/check-project-transaction-mutation-controller.mjs` | PASS — 6 scenarios; no native/UI side effects |
| `cargo test -p syndocal --release --locked authored -- --test-threads=1` | PASS — 34 passed, 0 failed; 1,780 other tests filtered |

The Rust run includes the authored control-plane cases for exact receipt retry,
same-key/different-shape rejection, owner retirement and ABA, cross-window
rejection, two-principal admission, terminal retention/tombstones, NoOp,
publication failure, authored commit Undo/Redo, and malformed operation/payload
rejection. No app process, physical output, device, or external client was
started.

No assertion was weakened, no old result was accepted, and no runtime or
physical-output behavior changed.

## Remaining boundary

`AI2-COMMAND-BRIDGE-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. This checkpoint proves only the existing
vertical and controller contracts above; it does not prove the full AI2
requirement across every authored operation, generated schema parity, all
external Remote/MCP/JSON-RPC adapters, consent/principal policy, or adversarial
and real-client recovery matrices. AI0/AI1/AI3-wide safety, external/hardware,
Mac real-device, signing, publication, and product-wide acceptance remain
unclaimed.

This checkpoint contains documentation only. The incomplete Channel API and
old Mac candidates were not merged, and the prior real-file recovery test was
not repeated.
