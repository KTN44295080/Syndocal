# Bounded project and backup input diagnostics

Branch `codex/showclock-review-20260912`, base `cca0f7a5`.

## Observed failure and implemented boundary

The real Windows QA `load_project_path` command rejected a private project with
a long foreign `app` string, but returned **57,405 bytes** containing that input.
The negative probe first verified that the active checkpoint, complete recorded
project/transport/publication/mapping/recovery/path/history authority, output
ownership and source bytes remained unchanged. Its assertion prints only the
case name and returned size, never the hostile diagnostic.
[Negative evidence](artifacts/input-diagnostics-native-before-2026-10-03.json)
pins executable SHA-256
`964aa00f42c858685fe9f04a7614bfc004ac481a0873b29d034e15fe18484747`.

`input_diagnostic` now owns a **1,024-byte returned-message bound**, UTF-8-safe
truncation with an explicit marker, and control-character escaping. It formats
directly into the bounded writer rather than first creating a complete formatted
string. A formatter that propagates formatting errors stops at the limit.

Project/backup typed decoding reports a bounded field path and an instruction to
check the field's value/type/required format. It omits Serde's failing-value text
and enum-variant path values. Path keys longer than 64 bytes or containing
characters outside ASCII letters/digits/underscore become `[map-key]`; short
identifier-shaped map keys may still be shown. This is not universal secret
redaction. Raw syntax diagnostics retain line/column and duplicate-key detail.
The shared foreign-app rejection reports the expected application without
echoing the supplied app string; existing whitespace acceptance is retained.

The exact `serde_path_to_error` 0.1.20 dependency supplies the field path
([primary API documentation](https://docs.rs/serde_path_to_error/0.1.20/serde_path_to_error/fn.deserialize.html)).
The final lockfile adds only this package and its Syndocal dependency edge;
existing package versions and the vendor Tauri runtime patch are preserved.

`project_file_decode` owns version/migration/typed mapping validation before
replacement. The main API is an imported forwarding seam. ProjectFile reads a
borrowed unique-key Value, then mapping decoding consumes the original, removing
the prior whole-Value clone including ignored fields. Project load preparation
errors and backup read/path context use the bounded formatter too. Byte quotas,
schema/data versions, accepted file contents, replacement/output ownership,
authentication and unattended external R4/R5 policy are unchanged.

## Real native results

The optimized Windows QA build passes with pinned MSVC 14.44.35207 and executable
SHA-256 `f6989597979b2892b91f1e8b8b968b6ec0364fc1aae7fbe97426df2217e31fc3`.
The maintained runner adds `--input-diagnostics` only to its private project or
backup lane. Each lane tests large foreign-app, unsupported-protocol and
wrong-BPM-type strings containing Unicode and control characters.

| Input | `.sdc` returned bytes | Backup returned bytes including file context |
| --- | ---: | ---: |
| Foreign app | 101 | 248 |
| Unsupported `snapshot.output.protocol` | 139 | 294 |
| Wrong `snapshot.clock.bpm` type | 133 | 288 |

All six failures omit the sentinel value and raw control characters, identify
the rejected application or schema field, and preserve active state and source
bytes. Native assertions allow up to 2,048 bytes including command context;
the formatter itself is limited to 1,024. Successful Unicode-path project load,
valid backup restore, existing duplicate/UTF-8/version/size rejection, and the
explicit valid older restore continue to pass. No fallback is inferred.

[Project proof](artifacts/input-diagnostics-project-native-final-2026-10-03.json)
contains ten passing groups;
[backup proof](artifacts/input-diagnostics-backup-native-final-2026-10-03.json)
contains eleven. Each launch verifies one responsive maximized exact QA window,
empty initial project, closed output gates, normal-profile identity preservation,
zero panic locations, and owned file/process/credential cleanup. The QA backup
directory is empty afterward. Normal backup content is not a probe fixture.

## Validation and ordinary native identity

The six new formatter/schema tests and seven project parser tests pass. The first
backup regression run selected all ten tests and caught an obsolete raw-Serde
wording assertion: nine passed, one failed.
[The failed result](artifacts/input-diagnostic-backup-assertion-before-2026-10-03.txt)
shows the new root-schema diagnostic. The assertion now requires its exact
schema/root prefix, missing-required-field category and absence of a size error.
The exact 128 MiB boundary still reaches schema rejection; one extra byte still
requires the numeric byte-limit error. This is an intentional diagnostic contract
change, not a relaxed load/size assertion. Only that cfg(test) assertion changed
after the native builds; production code and native harnesses stayed fixed.
The corrected backup filter passes all ten selected tests. The twelve migration
corpus, two Standby checkpoint and two update-backup fence tests also pass. The
six separate maintained runner invocations therefore select **39 tests**, with
zero failures/ignored tests and zero compiler warnings. Each exact count and
real Cargo exit status is checked. Initial compile took 13m21s; the cfg(test)
assertion rerun compiled in 15m05s. These are optimized test-binary checks, not
native-window/device evidence.

Raw results:
[formatter/schema 6](artifacts/input-diagnostic-tests-2026-10-03.txt),
[project JSON 7](artifacts/input-diagnostic-project-json-tests-2026-10-03.txt),
[backup 10](artifacts/input-diagnostic-backup-tests-2026-10-03.txt),
[corpus 12](artifacts/input-diagnostic-migration-tests-2026-10-03.txt),
[Standby 2](artifacts/input-diagnostic-standby-tests-2026-10-03.txt),
[update fence 2](artifacts/input-diagnostic-update-tests-2026-10-03.txt).
The update filter performs deterministic settings/owner/operator-fence checks;
no updater endpoint, installer or real upgrade is exercised.

The maintained strict-JSON, project transaction/authority, frontend invocation
inventory (481 commands), exact Tauri linker wrapper and Node syntax checks pass.
Completion/ledger validators confirm mirror parity, Q4=168 and unchanged
27 Complete / 23 Open / 8 Deferred status markers.

`pnpm --dir app tauri build --no-bundle` passes with the exact linker and PATH-first
gate. This checkout's executable refreshed at `2026-10-02T23:52:04.6824668Z`,
after this build log's creation at `2026-10-02T23:44:17.606903Z`.
[Ordinary proof](artifacts/input-diagnostics-normal-native-2026-10-03.json)
pins SHA-256 `3b19c6009b0ffcc1d1af4c3ae09aaee450af2fda556e91ff8513a60ff79674ae`
and PID 72000: one visible responsive maximized `Syndocal` window, with
unauthenticated broker reads rejected. It remains running without a debugger.
This passive primary observation does not establish authenticated primary
Tap/loading, physical output or native Control geometry.

Completed QA/ordinary optimized builds, focused Rust filters and TypeScript checks have zero Rust
warnings/TypeScript diagnostics. Rust baseline/current/delta is 0/0/0;
the existing Vite advisory is 1/1/0. Ordinary generated assets contain no
opt-in Tap QA receiver. Protected pre-existing frontend changes are present in
the builds, so neither artifact is a clean frozen release candidate.

## Tap recheck requested during this tranche

Before these diagnostic edits, the same current-base Windows QA App Tap callback
was rechecked: 750ms taps changed 120 BPM to **80.16871**, with native header `80`
and matching footer. All eight groups passed, without a DOM/native click.
[Tap proof](artifacts/tap-native-recheck-2026-10-03.json) shares the negative
input-probe executable hash above. The exact primary process/window was also
verified [passively](artifacts/tap-normal-recheck-2026-10-03.json): PID 57260,
SHA-256 `c7e9c431f980fa68bd25e0f2e1d60c42edb7e59a7783640fea5cd0573855c9e1`.
This confirms the existing publication/readout repair at the recorded base;
it does not claim observation of the operator's physical clicks or clock source.

## Limits and next action

Returned diagnostic sizes and bounded formatting consumption are measured;
peak allocation, whole-input decode latency and total resource/time budgets are
not. Serde and the path tracker may allocate an original error/path before the
formatter runs. Existing validators may construct a large error before its
returned text is bounded. The input byte quotas do not bound total Value/typed
image or serialization allocation. Other format-specific schema readers are not
covered solely because foreign-app text is now shared.

Full O1-O4, complete independently fixed golden corpus, array/count/reference and
allocator fuzz breadth, automatic verified fallback/restart disposition, browser/
template/cache/protocol versions, actual upgrade/downgrade, non-Windows identity,
physical output and release gates remain open. No completion marker is promoted.
No Computer Use or subagents are used; self-review is not independent review.
Protected Remote PIN/localization/DJ ingress work remains outside this checkpoint.

Continue the executable golden/count/allocator/recovery gaps and stale ShowClock/
Patch risk-description audit. Source confirms explicit Manual Hold/Re-arm and
the September 12 Patch/GDTF software checkpoint; their hardware/rejoin/whole-lane
risks remain open. This tranche does not revise those separate risk rows.
