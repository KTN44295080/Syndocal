# Migration and compatibility current-source checkpoint — 2026-09-14

## Checkpoint identity

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `fdc42556` (`Add combined Lighting and Video control overview`)
- Scope: rerun the current Windows project storage, transaction, recovery,
  publication, strict-JSON, and in-memory migration corpus gates on the
  current source tree.
- No project data, external endpoint, device, or destructive migration was
  used. This checkpoint adds evidence only; it does not close
  `MIGRATION-COMPATIBILITY-001`.

## Verification evidence

The command was run from the repository root after initializing the exact
Build Tools MSVC `14.44.35207` x64 environment and confirming the absolute
linker with `where.exe link.exe`:

```text
pnpm.cmd --dir app run check:project-storage
pnpm.cmd --dir app run check:project-transaction
pnpm.cmd --dir app run check:project-recovery-e3
pnpm.cmd --dir app run check:project-publication-e4
pnpm.cmd --dir app run check:project-open-bootstrap
node app/scripts/check-project-history-preflight.mjs
node app/scripts/check-project-history-keyboard.mjs
node app/scripts/check-project-transaction-recovery-controller.mjs
pnpm.cmd --dir app run check:strict-json
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_file_ -- --nocapture --test-threads=1
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_recovery_ -- --nocapture --test-threads=1
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_publication_ -- --nocapture --test-threads=1
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 migration_corpus -- --nocapture --test-threads=1
```

All command groups exited zero. The focused results were:

- project storage, transaction/authority, E3 recovery, E4 publication,
  project-open bootstrap, history, and transaction-recovery checks passed;
- strict JSON duplicate-key self-test: 130 assertions;
- `project_file_`: 33 passed, 0 failed, 0 ignored;
- `project_recovery_`: 2 passed, 0 failed, 0 ignored;
- `project_publication_`: 18 passed, 0 failed, 0 ignored;
- `migration_corpus`: 11 passed, 0 failed, 0 ignored;
- the migration corpus recorded 224 truncated-input cases, 5 malformed
  byte/number cases, 3 depth cases, and 128 seeded semantic/idempotency
  round trips. It also covered phase-1 golden load/save/reload, legacy
  defaults and unknown fields, mapping preservation, version rejection before
  migration, duplicate IDs, corrupt references/values, retired DJ mapping,
  and embedded-profile path non-reopening.

## Remaining boundary

`MIGRATION-COMPATIBILITY-001` remains **Open**. Current-source Windows
preparation, canonical save/reload, recovery/publication, golden phase-1
corpus, and bounded hostile-input checks pass. This evidence does not close
the broader hostile-input/fuzz matrix, the cross-platform file-identity
decision `DEC-FILE-ID-001`, real upgrade/downgrade machine rehearsals, or
supported non-Windows claims. No external endpoint, physical output, or
published artifact is claimed.
