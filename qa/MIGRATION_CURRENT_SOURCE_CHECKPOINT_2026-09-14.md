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

## Continuation — current Video-repair source migration recheck — 2026-09-14

At current source HEAD `a88877b3`, after the Video upper-desk repair and
security-contract recheck, the current storage/recovery/publication set was
rerun:

```text
check:project-storage: PASS
check:project-transaction: PASS
check:project-recovery-e3: PASS
check:project-publication-e4: PASS
check:project-open-bootstrap: PASS
check-project-history-preflight: PASS
check-project-history-keyboard: PASS
check-project-transaction-recovery-controller: PASS (6 scenarios)
check:strict-json: PASS (130 assertions)
```

All nine commands exited `0`. This confirms the existing Windows source-side
save/reload, transaction/authority, recovery, publication, bootstrap, history,
and strict-parser safeguards. No project upgrade/downgrade machine, cross-
platform compatibility decision, hostile fuzz campaign, clean-machine
installation, external endpoint, physical output, or published artifact was
used. The previously recorded 11-test native migration corpus remains the
applicable code-side corpus evidence because no migration Rust source changed.

`MIGRATION-COMPATIBILITY-001` remains `Open` pending the supported-version and
file-identity decisions, broader hostile-input/fuzz matrix, and real
upgrade/downgrade compatibility rehearsal.

## Continuation — deterministic hostile JSON corpus — 2026-09-14

The migration corpus was extended in implementation commit `c42bb98e` with a
bounded deterministic hostile-byte run around the production in-memory project
preparation path. It generates `4096` cases from a fixed seed, caps each random
case at `2048` bytes, includes valid JSON scalar controls, and checks parser
rejection, preparation panic-freedom, and byte-for-byte input immutability.
It performs no file, device, network, or publication I/O.

With the exact MSVC `14.44.35207` x64 linker selected and confirmed first by
`where.exe link.exe`, the focused release command completed successfully:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 migration_corpus_ -- --nocapture --test-threads=1
test result: ok. 12 passed; 0 failed; 0 ignored; 1915 filtered out
migration hostile corpus: seed=0x5344435f20260913, 4096 cases,
4091 parser-rejected, 5 parsed, 0 prepared, max_bytes=2048
```

The existing corpus remained green: `224` truncation cases, `5` malformed
byte/number cases, `3` depth cases, and `128` semantic/idempotency cases. The
Windows native warning-ratchet was rerun after the test change and passed with
output markers `2/2`, warning-shaped output `none`, baseline/current warnings
`0/0`, and identity removals `0`.

This closes only the new deterministic hostile-input source slice. It does not
close the supported-version/file-identity decision, broader fuzz campaign,
cross-platform claims, real upgrade/downgrade or clean-machine rehearsal, or
published-artifact compatibility. `MIGRATION-COMPATIBILITY-001` remains
`Open`.

## Continuation — current-source migration corpus recheck — 2026-09-15

At current source HEAD `0f846c7a`, the migration storage, transaction,
recovery, publication, bootstrap, history, strict-parser, and bounded hostile
corpus checks were rerun with the exact MSVC `14.44.35207` x64 linker. The
following nine source/preflight commands exited `0`:

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
```

The focused release filters also exited `0`:

- `project_file_`: 33 passed, 0 failed, 0 ignored;
- `project_recovery_`: 2 passed, 0 failed, 0 ignored;
- `project_publication_`: 18 passed, 0 failed, 0 ignored;
- `migration_corpus_`: 12 passed, 0 failed, 0 ignored.

The migration corpus again recorded 224 truncations, 5 malformed byte/number
cases, 3 depth cases, and 128 semantic/idempotency cases. The hostile corpus
used seed `0x5344435f20260913`, 4096 cases, 4091 parser-rejected cases, 5
parsed cases, 0 prepared cases, and a 2048-byte cap. No file, device,
network, or publication I/O was performed.

This is a current-source Windows recheck only. It does not close the broader
hostile-input/fuzz matrix, cross-platform file-identity decision
`DEC-FILE-ID-001`, real upgrade/downgrade or clean-machine rehearsals,
supported non-Windows claims, external clients, physical output, signing,
publication, or product completion. `MIGRATION-COMPATIBILITY-001` remains
**Open**.

## Continuation — current-source migration recheck and runner repair — 2026-09-18

At the current product source after the Remote/Security checkpoint, the exact
MSVC `14.44.35207` x64 environment was initialized and `where.exe link.exe`
resolved the pinned linker first. The migration source contracts and focused
release filters passed:

```text
check:project-storage: PASS
check:project-transaction: PASS
check:project-recovery-e3: PASS
check:project-publication-e4: PASS
check:project-open-bootstrap: PASS
check-project-history-preflight: PASS
check-project-history-keyboard: PASS
check-project-transaction-recovery-controller: PASS (6 scenarios)
check:strict-json: PASS (130 assertions)
project_file_: 33 passed; 0 failed; 0 ignored
project_recovery_: 2 passed; 0 failed; 0 ignored
project_publication_: 18 passed; 0 failed; 0 ignored
migration_corpus_: 12 passed; 0 failed; 0 ignored
```

The maintained `check-migration-corpus.mjs` runner first exposed a stale
expectation (`12 !== 11`) after the current corpus selected twelve tests. The
runner was repaired to expect the actual maintained selection of twelve, then
rerun with the same pinned linker. The final runner result was:

```text
migration corpus: 224 truncations, 5 malformed byte/number cases, 3 depth cases
migration hostile corpus: seed=0x5344435f20260913, 4096 cases,
4091 parser-rejected, 5 parsed, 0 prepared, max_bytes=2048
migration corpus: seed=0x5344435f20260913, 128 semantic/idempotency cases
migration corpus gate passed: 12 Rust tests, none failed or ignored
```

No product migration schema or runtime behavior was changed by this repair;
only the stale acceptance count in the QA runner was corrected. This remains
current-source Windows evidence. It does not close the broader hostile-input
fuzz matrix, `DEC-FILE-ID-001`, supported non-Windows claims, real
upgrade/downgrade or clean-machine rehearsals, external clients, physical
output, signing, publication, or product completion. Therefore
`MIGRATION-COMPATIBILITY-001` remains **Open**.
