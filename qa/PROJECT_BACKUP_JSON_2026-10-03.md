# Project backup JSON admission and identity

Branch `codex/showclock-review-20260912`, base `2a9d9c0e`.

## Reproduced problem

The real Windows QA `load_project_backup` command accepted a canonical filename
whose envelope carried a different ID. The private valid backup first restored
89 BPM with output gates closed. The next load returned `{name: "id-mismatch",
ok: true}` where rejection was required. The
[negative evidence](artifacts/backup-json-native-before-2026-10-03.json) pins the
old QA executable SHA-256
`edd4dc3dfdfaca9ea1bbf1df4026d86603a38081951a45ac7b69ec96acc7baca`,
valid restore and failure; the assertion payload above was captured in the runner
output. Normal-profile identity and QA credential/process/file cleanup passed.

The old reader also used unbounded `read_to_string`. Its typed serde parser
already rejected duplicate recognized fields, but ignored additive fields could
contain duplicates. List/prune consumed the envelope's ID without checking it
against the filename, so an aliased/corrupt entry could name a different backup.

## Implemented boundary

`project_backup_json` owns byte/JSON/envelope validation and serialization. The
main reader is a forwarding seam; listing, restore, publication verification and
update-backup receipt verification share it. Publication/transaction/retention
ownership remains in the existing modules and commands.

The reader now shares the canonical unique-key parser and opened-handle bounded
reader with `.sdc`/Standby. Duplicate keys at any depth, including ignored fields
and escape-equivalent keys, fail before preparation. The unique Value is consumed
by typed envelope deserialization; there is no second raw JSON parse or persisted
image clone. Existing serde defaults and unambiguous additive-field behavior remain.

Backup JSON has a separate **128 MiB** byte limit, leaving room for envelope
metadata and added indentation around the 64 MiB `.sdc` format. This is a new
backup admission boundary, not a claim that old backups already had a quota.
The writer uses the same limit and validates the envelope before staging any
file, so it cannot publish a backup that its reader rejects solely for size.
Metadata is only a preflight; actual input consumption is capped to limit plus
one byte using the existing shared reader. Input bytes are bounded; total Value,
typed-image or allocator memory and serialization allocation are not claimed
to fit inside 128 MiB.

An envelope must have a positive ID and a canonical `backup-{id}.json` filename
matching that ID exactly. Zero, mismatched and noncanonical names are rejected.
`u64::MAX` remains a valid codec ID; this does not expand the JavaScript MCP ID
range. Both backup/project version and application validation remain in place.
No schema/product version bump, legacy fallback, rename or source rewrite occurs.
Over-limit or malformed older files remain on disk; listing continues to omit
invalid entries and does not automatically load a different generation.

Only normal-profile backup metadata was inspected: 8 files, largest 47401 bytes.
No normal backup was explicitly read or restored by the probe. The private
QA backup directory was empty before and after the reproduction. Native probes
create and remove individually owned files in that exact QA directory, with
exclusive create and path checks; no profile-wide or recursive deletion is used.

## Software verification

The exact MSVC 14.44.35207 linker is pinned and first in PATH. Four final optimized
release selections pass, with no failed/ignored test and zero compiler warnings:

- [Backup codec/reader/writer and existing backup regressions](artifacts/backup-json-tests-2026-10-03.txt):
  10 passed; seven new cases plus existing retention/mapping/unique-ID cases.
- [Shared project parser regression](artifacts/backup-json-project-regression-tests-2026-10-03.txt):
  7 passed; raw-value parity, duplicates, 64 MiB and bounded-reader boundaries.
- [Standby regression](artifacts/backup-json-standby-regression-tests-2026-10-03.txt):
  2 passed; integrity/pruning and mapping round trip through the shared reader.
- [Update settings and owner/operator fence regression](artifacts/backup-json-update-fence-tests-2026-10-03.txt):
  2 passed; no update endpoint or installer is invoked. These are not upgrade or
  whole update-backup artifact acceptance.

The final backup filter is `project_backup`, which includes the existing plural
`project_backups_` test. The initial underscore filter ran nine actual passing
tests but omitted that plural case. The count gate correctly rejected its result
against the initial ten-test expectation. [That negative selection log](artifacts/backup-json-selection-before-2026-10-03.txt)
is preserved; the corrected filter runs all ten rather than lowering the count.
The initial optimized compilation took 14m13s. No reduced-optimization override
or timeout restart is used.

Existing project storage, transaction/authority, E3 recovery and E4 publication
checks pass. Node syntax and focused Rust-module formatting pass.

## Real Windows QA evidence

The optimized isolated QA executable is written at
`2026-10-02T22:45:15.6932493Z` (UTC), SHA-256
`1ef46463eab3046fa12c9000ba93b99c4334770d8c7c3f0d9860512cb4604f1f`.
[Ten native groups pass](artifacts/backup-json-native-final-2026-10-03.json):
three backup groups and seven existing authentication/lifecycle groups.
The real private restore publishes 89 BPM and `unsaved_replacement` with null
current path, and both runtime output gates stay closed.

Ten corrupt files fail: ID mismatch, zero ID, future envelope/project version,
foreign app, duplicates in ignored envelope/project fields, truncation, invalid
UTF-8 and a 134217729-byte file. Each rejected restore preserves the canonical
checkpoint and every required publication/mapping/disposition/recovery/path/
history/transport counter, current path, full ownership state and original SHA.
Missing observation fields are rejected by the helper. Only the valid older
backup is listed. A subsequent explicit restore selects that exact ID and
advances the project epoch while retaining the expected unsaved disposition,
89 BPM and closed gates. It does not silently fall back after a failed load.

Each QA launch has exactly one responsive maximized QA window. Normal-profile
identity remains unchanged; owned processes, credentials and backup files are
cleaned up, with no native panic. The
[same-artifact `.sdc` regression](artifacts/project-json-native-backup-regression-2026-10-03.json)
also passes all nine groups. It proves the shared-reader refactor retains the
previous actual-path JSON rejection/state-preservation behavior.

## Normal checkout build/window and ledger

The exact normal `pnpm --dir app tauri build --no-bundle` exits 0. The wrapper
verifies and stops only PID 53260 at this checkout's `target/release/syndocal.exe`.
The refreshed executable has SHA-256
`c7e9c431f980fa68bd25e0f2e1d60c42edb7e59a7783640fea5cd0573855c9e1`.
Its UTC write time `2026-10-02T22:58:35.9533631Z` is after the build log's UTC
creation time `2026-10-02T22:55:39.1022819Z`.

Plain launch without a debugger produces PID 57260, one visible responsive
maximized `Syndocal` window, and unauthenticated broker-read rejection.
[The passive native proof](artifacts/backup-json-normal-native-2026-10-03.json)
does not establish authenticated primary-profile backup acceptance or geometry.

The [QA](artifacts/backup-json-qa-build-2026-10-03.txt) and
[normal](artifacts/backup-json-normal-build-2026-10-03.txt) optimized build logs
retain the exact MSVC/linker/PATH and artifact paths. Against `2a9d9c0e`, Rust
compiler warning baseline/current/delta is 0/0/0 and the Vite advisory is 1/1/0
for both native configurations. TypeScript compilation succeeds. No warning
allowance or chunk threshold changes are made. Builds include the preserved
pre-existing protected frontend changes and are internal checkpoints.

Q1 records the bounded backup/`.sdc` native proof and Q3 mitigation is refreshed.
Q4 is 167; all completion statuses remain 27 Complete, 23 Open and 8 Deferred.
This is not a promotion of migration, security, output or release acceptance.
The maintained completion and Q1-Q4 validators pass with tracked evidence and
exact master parity. The reviewed staged diff has no whitespace errors and
contains only the explicitly owned files.

## Acceptance boundary

This change does not complete O1–O4, full golden migration, every count/allocator/
diagnostic resource bound, browser recovery/template/cache/protocol compatibility,
crash/upgrade/downgrade or cross-platform identity. A private explicit older backup
restore is distinct from automatic corrupt-newest fallback or restart disposition
acceptance. No physical output, Enable/Arm, media/device probe, distribution,
Computer Use or subagent is involved. Protected Remote PIN/DJ ingress work remains
outside this change; source review is by the implementing agent.
