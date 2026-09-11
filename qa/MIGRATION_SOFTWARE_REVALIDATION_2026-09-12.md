# Migration and compatibility software revalidation — 2026-09-12

This checkpoint revalidates current Windows project/template/cache/recovery
compatibility and hostile-input rejection. Product source was not changed.

- Source under test: `main` at `9c53e570bd6668f9a768aa9f19eee782378a2907`.
- Native tests used the exact Build Tools MSVC `14.44.35207` x64 linker with
  the required absolute Cargo pin and PATH-first `where.exe link.exe` check.
- Evidence log: `target/qa/migration-current-main-20260912/migration-tests.log`.

## Current-source results

Static checks passed:

- project storage helpers;
- project transaction and authority contracts;
- E3 recovery production driver;
- E4 publication checks;
- project-open bootstrap authority gate;
- history preflight and keyboard controller paths;
- project transaction recovery controller — 6 scenarios;
- strict JSON duplicate-key self-test — 130 assertions.

Focused native tests passed:

```text
project_file_:        33 passed; 0 failed; 0 ignored
project_recovery_:     2 passed; 0 failed; 0 ignored
project_publication_: 18 passed; 0 failed; 0 ignored
```

The current-source set covers legacy `.sdc` defaults and round trips, custom
profile/cache-backed references, invalid version/app/duplicate IDs, broken
fixture/video/timeline/node-graph references, unsafe ISF, corrupt values,
recovery-state corruption/newer versions, serial-overflow inertness, stale
templates, reply-loss/replay, exact owner/operator fences, durable staging,
backup/publication restart reconciliation, and no-partial-write behavior.

## Remaining boundary

The software gate remains `In progress`. This does not close the golden corpus
and hostile-input fuzz matrix, cross-platform file identity decision
`DEC-FILE-ID-001`, real upgrade/downgrade machine rehearsals, or supported
non-Windows claims. No external endpoint, destructive migration, or physical
output was used.

`git diff --check`: PASS before commit.
