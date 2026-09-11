# AI2 authored bridge audit — 2026-09-08

## Scope

This is a bounded audit of the existing authored command bridge. It records
the implemented `set_effect_enabled` vertical and the existing project
transaction mutation controller without widening the supported operation set,
adding a new API, or marking the full AI2 tranche complete.

- Source base for this revalidation: `1441f6379ca4b67c09c5b22bf735aeb4bab8b77e`
- Working tree before this revalidation: clean; `main` matched `origin/main`
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

## Current-main software revalidation

The existing authored vertical was revalidated on current `main` at source
HEAD `818235abe6c17f5571bd8a4ee2ad2132564e1ed0`. No authored operation set,
receipt schema, or mutation controller was changed.

| Check | Result |
| --- | --- |
| `node app/scripts/check-authored-effect-enable.mjs` | PASS — strict rendered route, receipt, superseded A/B, stale retry and rollback scenarios |
| `node app/scripts/check-project-transaction-mutation-controller.mjs` | PASS — 6 scenarios; no native/UI side effects |
| `cargo test -p syndocal --release --locked authored -- --test-threads=1` | PASS — 34 passed, 0 failed; 1784 filtered |

The Rust run used the exact MSVC 14.44.35207 x64 linker pin and
`where.exe link.exe` first-match check. This strengthens only the existing
vertical evidence; `AI2-COMMAND-BRIDGE-001` remains Open for all authored
families, external adapters, consent policy, and adversarial or real-client
recovery matrices.

## Current-main authored-bridge revalidation — 2026-09-10

The bounded authored bridge was rerun against current `main` at source HEAD
`50a0ed98e3f501d1676cd1809a8921e3fad76489`. Product source, operation sets,
and receipt schemas were unchanged. The exact MSVC 14.44.35207 linker was
pinned and verified first in `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-authored-effect-enable.mjs` | PASS — strict route, receipt validation, superseded intents, bounded stale retry, rollback baseline |
| `node app/scripts/check-project-transaction-mutation-controller.mjs` | PASS — 6 scenarios, no native/UI side effects |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 authored -- --test-threads=1` | PASS — 34 passed, 0 failed, 0 ignored |

The run retains rejection of wrong operation/payload, owner/window mismatch,
stale/future/expired fences, same-key shape conflict, retired-owner replay,
publication failure drift, and superseded results. `AI2-COMMAND-BRIDGE-001`
remains Open for all authored families, generated schema parity, external
adapters, consent policy, and adversarial or real-client recovery. The
real-file thumbnail recovery trial was not rerun.

## Current-main authored-bridge revalidation — 2026-09-11

The bounded AI2 checks were rerun after the current output, snapshot, and
input-generation checkpoints at source HEAD
`9c4a5e9f6cc1886b475f55a8536bbee1754f72e7`. Evidence is preserved under
`target/qa/ai2-current-main-20260911-01/`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-authored-effect-enable.mjs` | PASS — strict rendered route, receipt validation, superseded A/B, bounded stale retry, rollback baseline |
| `node app/scripts/check-project-transaction-mutation-controller.mjs` | PASS — 6 scenarios; no native/UI side effects |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 authored -- --test-threads=1` | PASS — 34 passed, 0 failed, 0 ignored |

The Rust run retained rejection of wrong operation/payload, owner/window
mismatch, stale/future/expired fences, same-key shape conflicts,
retired-owner replay, publication failure drift, and superseded results. It
also retained exact authored commit Undo/Redo and byte-identical terminal
retry behavior.

`AI2-COMMAND-BRIDGE-001` remains Open. This is bounded local evidence for
the existing authored vertical; it does not prove all authored families,
generated schema parity, external Remote/MCP/JSON-RPC adapters, consent
policy, adversarial matrices, or real-client recovery. Physical, Mac,
signing, publication, and product-wide acceptance remain unclaimed.
