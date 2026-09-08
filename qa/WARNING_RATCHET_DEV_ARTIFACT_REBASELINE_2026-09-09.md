# Warning ratchet dev artifact rebaseline — 2026-09-09

## Scope

This checkpoint repairs one warning-ratchet artifact-coverage mismatch for the
existing `windows-default-all-targets` command. It changes no product source,
warning assertion, suppression, compiler configuration, or release artifact.

## Reproduced failure

At trusted base `21b403dc6d03af745cd11e6fcd8281cfc675cf10`, the exact Windows
warning command completed successfully but the checker failed closed because
Cargo emitted the existing integration-test artifact below while the dev
artifact inventory did not list it:

```text
Cargo artifact coverage mismatch:
missing=[]
unexpected=[{"package":"protocol","target":"dj_link_v3_sender_contract","targetKinds":["test"],"crateTypes":["bin"]}]
```

The target is an existing source test at
`crates/protocol/tests/dj_link_v3_sender_contract.rs`. The release artifact
inventory already contained the same target; the dev all-targets inventory was
the stale entry.

## Bounded correction

Commit `6e121193e0545c088da7de21331ab1b8823128f7` adds that exact artifact only
to `windows-default-all-targets` and records the evidence commit as the trusted
base. No diagnostics, external warning allowances, immutable configuration
fields, or other configuration inventories changed.

The explicit audit was run with the exact MSVC 14.44.35207 Build Tools linker
pinned and first in `where.exe link.exe`:

```text
pnpm.cmd --dir app exec node scripts/check-warning-ratchet.mjs \
  --rebaseline-artifacts \
  --base-ref 21b403dc6d03af745cd11e6fcd8281cfc675cf10 \
  --head-ref 6e121193e0545c088da7de21331ab1b8823128f7 \
  --configuration windows-default-all-targets
```

Result: `artifact coverage: 12`, `changed files: 1`, `artifact rebaseline audit
ok; inventory was not written`.

Evidence log: `target/qa/warnings-current-20260909-artifact-rebaseline.log`.

## Boundary

This closes only the observed dev all-targets artifact-coverage drift. It does
not claim the complete warning matrix, macOS warning configurations, native
release warning run, physical output, signing, publication, or product-wide
completion. The normal B→C gate is run after this evidence-only QA checkpoint;
the intentional B-only metadata immutability failure is retained as evidence,
not hidden.
