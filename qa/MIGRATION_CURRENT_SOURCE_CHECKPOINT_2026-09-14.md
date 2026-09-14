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

## Takeover rerun — 2026-09-14

The migration corpus command was rerun on current source `ddf66fc2` after the
takeover, with the same exact MSVC `14.44.35207` x64 linker check. The release
build completed in 10m12s and selected exactly 11 migration-corpus tests:
`11 passed, 0 failed, 0 ignored`; the run again reported 224 truncations, 5
malformed byte/number cases, 3 depth cases, and 128 semantic/idempotency cases.

The source audit also confirmed that the media coherence implementation keeps
Windows deny-write/delete handles and uses Unix `dev`/`ino` plus size,
mtime/ctime version CAS and a private snapshot before probing. This is a
code-side coherence implementation, not a cross-platform support decision.

## Source continuity audit — 2026-09-14

After the recorded corpus rerun, `git diff --name-only ddf66fc2..HEAD -- app
crates` returned no paths. The current product source therefore remains the
same source that produced the 11-test corpus result; subsequent changes are
QA documentation only. No new migration result is manufactured from that
unchanged source, and the external compatibility boundary remains as stated
below.

## Remaining boundary

`MIGRATION-COMPATIBILITY-001` remains **Open**. Current-source Windows
preparation, canonical save/reload, recovery/publication, golden phase-1
corpus, and bounded hostile-input checks pass. This evidence does not close
the broader hostile-input/fuzz matrix, the cross-platform file-identity
decision `DEC-FILE-ID-001`, real upgrade/downgrade machine rehearsals, or
supported non-Windows claims. No external endpoint, physical output, or
published artifact is claimed.

## Current HEAD migration-contract recheck — 2026-09-14

At current HEAD `3643c524`, the current-source migration contract set was
rerun. Storage helpers, transaction/authority, E3 recovery, E4 publication,
project-open bootstrap, history preflight, keyboard routing,
transaction-recovery controller, and strict JSON all exited `0`.

## Takeover continuation — current-source migration recheck — 2026-09-14

At current source HEAD `80a7a001`, the migration source contracts were rerun
with the existing script names:

```text
pnpm.cmd --dir app run check:project-storage
project storage helpers ok

pnpm.cmd --dir app run check:project-transaction
project transaction executable production-contract checks passed
project authority deterministic checks passed

pnpm.cmd --dir app run check:project-recovery-e3
E3 project recovery production driver passed

pnpm.cmd --dir app run check:project-publication-e4
project publication E4 checks passed

pnpm.cmd --dir app run check:project-open-bootstrap
project open bootstrap authority gate passed

node app/scripts/check-project-history-keyboard.mjs
PASS actual keyboard controller and text-editor guards

node app/scripts/check-project-transaction-recovery-controller.mjs
Project transaction recovery controller: 6 scenarios passed; no native/UI side effects.

pnpm.cmd --dir app run check:strict-json
strict JSON duplicate-key self-test passed: 130 assertions
```

All eight current-source commands exited `0`. This revalidates storage,
authority, recovery, publication, bootstrap, keyboard, transaction-recovery,
and strict JSON boundaries only. No external upgrade/downgrade machine,
cross-platform migration decision, hostile fuzz campaign, clean-machine
installation, or published artifact was exercised.

`MIGRATION-COMPATIBILITY-001` remains `Open` pending those external and
release-scope acceptance conditions.

With the exact MSVC `14.44.35207` Build Tools linker initialized and printed
first in `where.exe link.exe`, the release Rust filters passed:

- `project_file_`: 33 passed, 0 failed, 0 ignored;
- `project_recovery_`: 2 passed, 0 failed, 0 ignored;
- `project_publication_`: 18 passed, 0 failed, 0 ignored;
- `migration_corpus`: 11 passed, 0 failed, 0 ignored.

The migration corpus again covered 224 bounded truncations, 5 malformed
byte/number cases, 3 depth cases, and 128 seeded semantic/idempotency
round-trip cases, including legacy defaults/unknown fields, golden
load-save-reload, duplicate IDs, corrupt references/values, retired DJ
mapping, and embedded-profile path non-reopening.

This strengthens current-source storage/recovery/migration evidence only.
No cross-platform support decision, real upgrade/downgrade machine rehearsal,
external endpoint, hostile-input fuzz campaign, physical output, or published
artifact was performed. `MIGRATION-COMPATIBILITY-001` remains `Open`.
