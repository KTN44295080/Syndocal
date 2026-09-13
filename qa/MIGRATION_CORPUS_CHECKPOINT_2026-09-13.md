# Bounded migration corpus — 2026-09-13

## Identity and boundary

- Branch: `codex/showclock-review-20260912`.
- Base: `b31f21babc1915a178b587a52a70d471f5b04a3e`.
- Related requirement: `MIGRATION-COMPATIBILITY-001`, critical path O1–O3.
- Owned changes are the corpus test module, its test-only registration in
  `project_snapshot_persistence.rs`, the partial golden expectations, this note,
  and `app/scripts/check-migration-corpus.mjs`.
- No runtime implementation, project schema, product version, or shared
  completion-ledger status is changed by this tranche.

## Exact tested contract

The current project admission accepts `.sdc` version **1**. Version 0, future
versions, missing/non-integral/negative versions, and a foreign application name
are rejected. This is not a claim that every historical application build using
version 1 can reopen every file written by the current build.

Optional legacy fields receive the existing production defaults. Unknown
additive fields are not preserved on save. The retired `dj_transition` mapping
is discarded with a load warning; it is not silently converted into a new DJ
Link trigger. The corpus exercises those existing decisions without adding a
compatibility fallback.

The native test module calls the actual production chain:

`project_and_control_mappings_from_value` → `prepare_project_load` →
`project_file_for_save_from_parts` →
`project_json_for_write_with_control_mappings_and_dj`.

It then reloads and saves the canonical image again. The two canonical JSON
values and byte strings must be identical. MIDI/OSC/DMX mappings must retain
their complete normalized values. Runtime counters must not enter the authored
save image.

The checked-in Phase 1 sample supplies embedded fixture geometry, controls,
groups, Cue targets, Timeline events/automation, an NDI source declaration,
composition/display configuration, and Stage objects. The golden file fixes
**22 exact JSON pointers and 10 array lengths**, not every authored field.
The NDI declaration is data only; no transport or engine is started.

The fixed-seed property loop performs 128 label/address/BPM/runtime-counter
variants of that same sample. This is bounded deterministic coverage, not a
coverage-guided fuzz campaign or a substitute for the other O2 project classes.

## Side-effect and evidence limits

The corpus creates no `AppState` or `EngineHandle` and publishes no project file.
Preparation normally probes local file-backed media. A test-only guard rejects
`File`, `StillImage`, and non-null `path` fields before production preparation,
including nested fields. The current non-local media availability path returns
`LiveSource` without opening a device or contacting an NDI peer.

Embedded fixture `profile_source_path` strings are separately exercised with
Windows/case/Unicode and non-Windows-style spellings. Production preparation
canonicalizes them to `snapshot://fixture/1` while retaining embedded controls
and geometry. These are path-string cases executed on Windows, not macOS/Linux
filesystem identity acceptance.

Input-clone equality verifies only the supplied in-memory candidate. It does
**not** prove preservation of a running project's state, crash recovery,
rollback, or original on-disk bytes. The malformed byte/UTF-8/depth cases call
the configured `serde_json` parser directly; they do not prove the full project
file ingress, byte/count quotas, duplicate-key policy, or archive decompression
limits.

## Verification

Run from the repository root:

```powershell
node app/scripts/check-migration-corpus.mjs
```

The runner reuses `run-tauri.mjs`'s exact vcvars/MSVC/PATH-first environment
contract and prints the absolute Cargo linker plus `where.exe link.exe` before
Cargo starts. It does not invoke Tauri or stop/start a product window. It checks
Cargo's exit code, one actual test-binary summary, exact selected-test count,
and zero ignored tests, preventing empty selections from passing.

Initial run before adding the explicit I/O guard:

- `10 passed; 0 failed; 0 ignored; 1884 filtered out`.
- 224 truncated prefixes, 5 malformed byte/number inputs, 3 depth-limit inputs.
- 128 semantic/idempotency cases, seed `0x5344435f20260913`.
- MSVC Build Tools `14.44.35207`, exact pinned linker first on PATH.
- Cargo release test configuration emitted no compiler warnings. A separate
  baseline compiler run was not measured; no warning delta is asserted.

Final run including the I/O guard:

- `11 passed; 0 failed; 0 ignored; 1884 filtered out`, Cargo exit 0 and runner
  exit 0. The same 224/5/3 malformed-input cases and 128 seeded round-trips ran.
- The additional guard test rejects File/StillImage declarations and nested
  local/UNC-style path candidates before production preparation.
- The exact MSVC Build Tools linker and PATH-first checks passed again.
- No compiler warnings were emitted by this release test invocation. No
  separately measured baseline or cross-configuration warning delta is claimed.

Independent read-only review found no runtime change in the original four-file
diff and confirmed the runner's toolchain/selection checks. It identified the
partial-golden, in-memory-only, and parser-only limitations documented above.
The additional I/O guard addresses the future-fixture side-effect boundary.

## Still open

`MIGRATION-COMPATIBILITY-001` remains **Open**. Outstanding work includes the
full independently fixed authored golden corpus; larger DVC/GDTF/Node Graph,
Audio/recording/template/backup/recovery specimens; invalid-reference/MAX-ID
and allocator/structure limits across all domains; production byte-ingress
fuzz/property coverage; failure-preserving disk/active-project tests; and
supported upgrade/downgrade and native restart rehearsals.

Cross-platform file identity and external acceptance remain separate. This
checkpoint does not claim full O1–O4, supported upgrade, native UI, device,
physical-output, venue, signing, publication, or product-wide completion.
