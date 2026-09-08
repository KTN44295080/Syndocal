# Backend operator checker repair — 2026-09-08

## Scope

This checkpoint repairs a stale checker assertion after the project mutation
facade was extracted from `App.tsx` into
`app/src/projectTransactionMutationController.ts`. Product runtime code was
not changed. The existing transaction, owner, approval, stale-authority, and
old-result rejection contracts remain checked; this change only points those
checks at their current responsibility boundaries.

The previously completed PNG missing-file → Retry → recovery trial was not
rerun. No file was moved, no native output was enabled, and no physical or
device acceptance was claimed.

## Reviewed change

- `app/scripts/check-backend-operator-contract.mjs`
  - reads the extracted mutation controller;
  - verifies that `App.tsx` passes the transaction and identity to the shared
    controller;
  - verifies the controller adds the exact backend ticket (`transaction_id`,
    `project_epoch`, and registered owner) before invoking the command;
  - verifies Stage published-command recovery and Cancel/Commit recovery at
    the extracted controller boundary;
  - retains the existing exact owner, approval, stale-authority, terminal,
    permission, and recovery assertions.

Repository base before this checkpoint: `1ed0df4f9ec321af916319bcfa292601412d1bbc`.

## Evidence

| Check | Result |
| --- | --- |
| `node --check app/scripts/check-backend-operator-contract.mjs` | PASS |
| `node app/scripts/check-backend-operator-contract.mjs` | PASS — 516 backend commands, 334 literal frontend calls, 133 transactional mutations |
| `pnpm.cmd --dir app exec tsc --noEmit` | PASS |
| `pnpm.cmd --dir app run check:frontend-invokes` | PASS — 457 commands |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer mutations / 31 server-authoritative mutations / 28 raw dispatches / 464 facade dispatches |
| `pnpm.cmd --dir app run check:project-transaction` | PASS — executable production contract and deterministic authority checks |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; no native/device calls |
| `pnpm.cmd --dir app run check:dj-link` | PASS — track-mapping policy and frontend contract |
| `git diff --check` | PASS |

The independent Astra review attempt was unavailable because the account
usage limit was reached. The supervising agent completed a read-only review of
the stable checker diff and the extracted controller, with no product-source
change. No native rebuild or application launch was required for this
checker-only change.

## Boundary

This checkpoint does not close the handoff. The ledger remains `50 Open + 8
Deferred`; Mac, physical ASIO/NDI/DMX/video, signing/publication, and other
documented gates remain separate. The existing EXE evidence is not a rebuild
of this checker-only change.
