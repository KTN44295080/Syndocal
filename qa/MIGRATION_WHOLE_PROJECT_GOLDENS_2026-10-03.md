# Whole-project migration expectations — 2026-10-03

Branch: `codex/showclock-review-20260912`. Base: `71428c91`.
Requirement: `MIGRATION-COMPATIBILITY-001`, critical path O2; the wider O1–O4
requirement remains **Open**.

## Change and oracle independence

The previous Phase 1 oracle selected 22 pointers and 10 array lengths. It could
miss a value lost outside those selections while the first and second saves
still agreed with one another. The active oracle now compares the **entire
decoded JSON object**, including all keys, array positions, types and leaves.
The old partial oracle is removed from the active test assets.

Two fixed expectations are checked in:

- [Phase 1](migration/phase1-current-canonical.json): 858 scalar leaves, plus
  every container/key/array position. This includes embedded custom profiles,
  fixture geometry and controls, group registry, Cue targets, Timeline
  events/automation, NDI source declaration, migrated media/slot identities,
  composition/output/mapping presets and Stage objects.
- [Empty new project](migration/empty-current-canonical.json): 118 scalar
  leaves, plus its complete container shape. The actual new-project
  `EngineSnapshot::default()` is an input under test, never the expected image.

The expectations were authored **before the candidate test run**, from the
legacy sample and explicit reviewed version-1 defaults/normalization. A local
drafting script used JSON plus literals; it did not run or read the candidate
writer, Cargo binary or native app. There is no test mode that overwrites an
expectation with production output.

The [manifest](migration/whole-project-oracles.json) pins the legacy sample's
LF-normalized SHA-256 and the leaf counts. Changing that sample requires an
independent oracle review. LF normalization makes this identity independent of
checkout CRLF settings.

The [authoring audit](artifacts/migration-whole-oracle-authoring-audit-2026-10-03.json)
checks all 630 original scalar leaves and every original key/array position:
629 leaves retain their authored values, with declared `f32` decimal values
represented by their exact IEEE-754 single-precision image; the one intentional
string change is the fixture profile identity to `snapshot://fixture/1`.
57 original decimal spellings change to that exact `f32` image. Integer IDs
are never converted through floating point. No original container or array
entry is removed. The audit enumerates 94 added subtree roots, accounting for
the 228 additional scalar leaves. It checks preservation, not the correctness
of every new default; those defaults were separately reviewed against the
following contracts:

| Expected addition / normalization | Reviewed contract |
| --- | --- |
| Bank 1 Cue list/executor, playback level, Cue/Timeline/event defaults, programmer, clock, telemetry, Auto VJ, mapping/mask defaults | `crates/protocol/src/lib.rs` serde declarations and explicit defaults |
| Legacy Front group registry and stable embedded fixture identity | `reconcile_project_fixture_groups` and `prepare_project_load` in `app/src-tauri/src/main.rs` |
| NDI asset ID 2 and default slot ID 3, loop 0..4000, Start/Drop cue points | `normalize_legacy_video_media_assets`, `normalize_legacy_video_clip_slots`, and `legacy_video_layer_transport_to_clip_slot` in protocol |
| Clock phase/counters, programmer, previews, telemetry, Auto VJ status and live strobe cleared on save | `app/src-tauri/src/project_snapshot_persistence.rs` |
| Root profile/group ordering and optional root mappings | `project_file_for_save_from_parts` and project writer in `main.rs` |

The comparator uses exact decoded `serde_json::Value` equality for leaves,
including numeric types, with no epsilon or ignored paths. Object key order
is irrelevant; array order is checked. Failures report a JSON pointer instead
of dumping the entire project. First/second production save **bytes** must
also remain equal through the existing round-trip helper.

## Coverage added

- Legacy Phase 1 → production decode/prepare/save → reload/save → full oracle.
- Current canonical Phase 1 and empty images → the same full round trip.
- New empty project defaults → independently fixed full expectation.
- All 128 fixed-seed Unicode label/address/BPM variants now compare their
  entire authored output, with only those explicitly authored values changed.
- All three embedded Windows/case/Unicode/non-Windows profile path variants
  compare the whole project, including the complete embedded geometry.
- A runtime-contaminated Phase 1 image checks the complete expected authored
  result after clock, playing, programmer, preview, telemetry, live strobe and
  Auto VJ status cleanup.
- Eight deliberate whole-oracle mutations outside the former selected set,
  plus adjacent `u64::MAX` identities, prove detection of changed leaves,
  missing/extra keys, reordered/shortened arrays and integer differences.

The focused test-only helper is `project_migration_golden.rs`. The existing
corpus remains in memory: no `AppState`, `EngineHandle`, device/network access
or project-file publication. Its pre-preparation File/StillImage/non-null-path
guard is preserved. NDI is a declaration; no peer or native output is opened.

Here "canonical" names the current pure preparation/writer's saved image.
These fixtures retain the admitted legacy single-Timeline representation;
they do not prove the current engine's nonempty multi-Timeline bank admission,
post-install normalization, coordinator publication, or native loaded state.

## Verification

The maintained exact-MSVC runner passed **16 tests; 0 failed; 0 ignored**
(the original 12, with stronger assertions, plus four whole-oracle tests).
Its exact Build Tools `14.44.35207` linker pin and PATH-first checks passed;
the optimized release test binary compiled in 11m 14s and selected 16 of 1961
tests. The fixture run took 0.24s. See the
[raw run](artifacts/migration-whole-golden-tests-2026-10-03.txt).

The [alternate parser/corpus runner](artifacts/migration-whole-golden-alternate-runner-2026-10-03.txt)
also passed its synchronized selection of 16 with zero failed/ignored tests.
This is a second entry point to the same test selection, not 32 independent
tests. Both runners reject an empty/changed selection and compiler warnings.
The existing 224 truncations, five malformed byte/number cases, three depth
cases and 4096 bounded fixed-seed hostile byte cases also ran.

First-party Rust warnings for this optimized test configuration:
baseline 0 in the preceding
[12-test run](artifacts/input-diagnostic-migration-tests-2026-10-03.txt),
current 0, delta 0. This checkpoint did not run or measure frontend/native
package compiler warnings. Node syntax and focused Rust formatting checks
passed. All three oracle JSON documents and the authoring audit passed the
repository's duplicate-key-rejecting JSON parser; its existing self-test
passed 130 assertions. Owned `git diff --check` passed. Both completion-ledger
validators passed: 27 Complete / 23 Open / 8 Deferred, 169 evidence records,
and master/mirror semantic parity. No requirement or risk status was closed.

This change is test/fixture/QA-only. It changes no runtime, UI, IPC, schema,
product version or native package configuration. A product build/window gate
is therefore not required for this checkpoint. No new native or physical
acceptance is claimed.

## Remaining work

These are complete expectations for **two project classes**, not a complete
O2 corpus. Independently authored larger DVC/GDTF, nonempty Node Graph/effect/
Touch, multiple Timeline banks, media-file/relink/missing-file, audio/recording,
backup/recovery/template and older-version classes remain. The empty arrays
in this fixture do not prove those features' authored semantics. Count/time/
allocation/decompression limits, broader fuzzing, historical upgrade/downgrade,
O4 automatic verified fallback/restart and non-Windows filesystem identity
remain separate incomplete acceptance boundaries. This checkpoint cannot
close the migration requirement, associated risk, or broad release markers.
