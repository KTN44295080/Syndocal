# Warning ratchet workspace-test artifact rebaseline — 2026-09-09

## Scope

This checkpoint repairs the same existing Cargo integration-test artifact
drift for the separately enforced `windows-workspace-tests` warning command.
It changes no product source, warning assertion, suppression, compiler
configuration, or release artifact.

## Reproduced failure

At trusted base `d79cbce254fd767ac9da1542b610dcc77b05d233`,

```text
cargo test --workspace --locked --no-run --message-format=json
```

completed its test build but the warning gate rejected the unlisted existing
artifact:

```text
Cargo artifact coverage mismatch:
missing=[]
unexpected=[{"package":"protocol","target":"dj_link_v3_sender_contract","targetKinds":["test"],"crateTypes":["bin"]}]
```

## Bounded correction

Commit `c7fc3f9d0a1e9f3253c86a324bac8121b035321f` adds the exact artifact only
to `windows-workspace-tests` and binds its evidence to the trusted base. The
other configuration inventories, diagnostics, external warning allows, and
immutable command fields are unchanged.

The explicit audit used the exact MSVC 14.44.35207 Build Tools linker, pinned
and first in `where.exe link.exe`:

```text
pnpm.cmd --dir app exec node scripts/check-warning-ratchet.mjs \
  --rebaseline-artifacts \
  --base-ref d79cbce254fd767ac9da1542b610dcc77b05d233 \
  --head-ref c7fc3f9d0a1e9f3253c86a324bac8121b035321f \
  --configuration windows-workspace-tests
```

Result: `artifact coverage: 12`, `changed files: 1`, `artifact rebaseline
audit ok; inventory was not written`.

Evidence log: `target/qa/warnings-current-20260909-workspace-artifact-rebaseline.log`.

## Boundary

This closes only the observed Windows workspace-test artifact-coverage drift.
It does not claim the complete warning matrix, Linux or macOS execution,
native release warning run, physical output, signing, publication, or
product-wide completion. The normal gate is run after this evidence-only QA
checkpoint; the intentional B-only metadata immutability failure remains
retained as evidence.
