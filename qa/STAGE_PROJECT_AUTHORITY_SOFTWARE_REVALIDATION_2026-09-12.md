# Stage and project authority software revalidation — 2026-09-12

This checkpoint advances deterministic software evidence for
`COV-STAGE-001`, `COV-PROJECT-TX-001`, and `COV-PROJECT-AUTH-001` on `main` at
`974d8801ed7f1baccf1adb81b6d8c4ed9275b500`. No product source was changed.

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:stage-labels` | PASS — label anchoring, eligibility, priority and overlap contracts |
| `pnpm --dir app run check:mapping-stage-geometry` | PASS — coordinate/grid-layer/authoritative drag contracts |
| `node app/scripts/check-patch-transaction-d2.mjs` | PASS — atomic PATCH/Repair transaction controller |
| `node app/scripts/check-project-transaction-mutation-controller.mjs` | PASS — 6 scenarios, no native/UI side effects |
| Engine `fixture_transform_batch` | PASS — 5 passed, 0 failed, 0 ignored |
| Syndocal `fixture_transform` | PASS — 4 passed, 0 failed, 0 ignored |
| Syndocal `stage_fixture_transform_batch` | PASS — 2 passed, 0 failed, 0 ignored |
| `pnpm --dir app run check:project-transaction` | PASS — project transaction and authority deterministic contracts |
| `pnpm --dir app run check:project-recovery-e3` | PASS — durable recovery and stale/late-ACK handling |
| `pnpm --dir app run check:project-publication-e4` | PASS — publication receipts and lost/malformed reply handling |
| `node app/scripts/check-project-history-preflight.mjs` | PASS — stale/epoch fence, CAS and redo invalidation |

All Rust checks used the documented MSVC 14.44.35207 linker and verified it
first with `where.exe link.exe`. The old alpha.11 native hash remains
historical evidence only. Fixture hardware, current native UI launch,
cross-platform execution and product-wide acceptance are not claimed; all
three rows remain `In progress`.

## Current-main project authority rerun — 2026-09-12

The bounded project-authority checks were rerun against current `main` at
`b0af6a052bc22ed9c51484af2b76689f0ab0fd5b`:

- `pnpm.cmd --dir app run check:project-transaction` — PASS;
- `node app/scripts/check-project-transaction-mutation-controller.mjs` — PASS,
  6 scenarios with no native/UI side effects.

This is deterministic software evidence only. It does not claim the frozen
re-audit, native UI, external client, physical output, or product-wide
acceptance.
