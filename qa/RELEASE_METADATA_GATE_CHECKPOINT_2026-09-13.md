# Release metadata gate checkpoint — 2026-09-13

## Scope

This checkpoint closes the Flow marker `RELEASE-METADATA-GATE-001` only. The
current release checker already contains the normal metadata gate and the
release-candidate path for exact tag, previous-version, updater-manifest,
signature, artifact hash, executable identity, and clean-worktree checks. This
checkpoint re-runs that implementation and records the result in the current
source tree; it does not claim a signed candidate, published artifact, clean
machine, updater service, legal approval, or physical acceptance.

## Evidence

Base source before this checkpoint: `bad6ed8b73d2f9c7e55e83e0b809d412df6993f5`.

The following commands passed in the Windows checkout:

```text
pnpm.cmd --dir app run check:release
pnpm.cmd --dir app run check:release:self-test
pnpm.cmd --dir app run check:q1-q4-ledger
pnpm.cmd --dir app run check:q1-q4-ledger:self-test
pnpm.cmd --dir app run check:completion-ledger
pnpm.cmd --dir app run check:completion-ledger:self-test
```

Observed results included:

- `release metadata ok: Syndocal 1.2.0-alpha.69 / .sdc / signed updater overlay / Seraf() / KTN`
- release metadata self-tests: `137 assertion groups`
- Windows release artifact self-test: `144 assertions`
- strict JSON duplicate-key self-test: `130 assertions`
- Q1-Q4 coverage: `32` Q1 rows, `29/29` domains, `10/10` source contracts, and `58/58` Flow markers referenced
- completion ledger: `49 Open + 8 Deferred + 1 Complete` authority rows
- completion-ledger self-tests: `19 negative cases`
- Q1-Q4 self-tests: `46 negative cases + 1 positive fixture baseline + 1 real-repository baseline`

## Result and boundary

The marker is now checked in the Flow document and represented as
`Complete` in `qa/SYNDOCAL_COMPLETION_LEDGER.json`. The completion and Q1-Q4
validators accept completed markers while continuing to fail closed on source,
status, marker, count, mirror, and accepted-row drift.

The linked Q1 release row remains `In progress` because the separate signed,
hardware, legal, clean-machine, updater, and publication gates remain open or
deferred. Those boundaries are not silently closed by this software-gate
checkpoint.
