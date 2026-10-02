# Project JSON admission and bounded file reads

Branch `codex/showclock-review-20260912`, base `bacd1f18`.

## Problem and compatibility decision

The actual Windows QA `load_project_path` command accepted a file containing
`version: 0` followed by `version: 1`. The Value parser silently chose the latter,
so unsupported/ambiguous project state entered the publication path. The
[negative native evidence](artifacts/project-json-native-before-2026-10-03.json)
records the old executable identity and successful valid-file setup; the runner
then failed its expected rejection with `{name: "duplicate-version", ok: true}`.
The assertion payload was captured in the runner output. Normal-profile identity
and QA credential/process cleanup passed.

The existing `check:strict-json` is the Node release-metadata parser's self-test;
it did not establish strict native .sdc admission. The old migration hostile-byte
loop also called the generic Value parser rather than the real project JSON
entry. Those are distinct evidence scopes.

Native .sdc and Standby project images now use one pure `project_file_json`
module. It constructs the canonical Value while rejecting repeated object keys
at every depth, including identical values, ignored additive fields, and keys
which become equal after Unicode escape decoding. It keeps the existing numeric,
UTF-8, trailing-data and serde recursion behavior for unambiguous JSON. Equal keys
in separate sibling objects, differently cased keys and key-like text inside
strings remain valid.

This intentionally rejects ambiguous files which previously used last-value-wins.
The application does not choose a winning value or rewrite the source. Errors
identify line/column and ask the operator to remove duplicate keys, without
echoing an arbitrarily large or sensitive key. Project version 1, legacy migration,
unknown additive-field policy and product/schema versions are unchanged.

## Reading and ownership boundary

The existing 64 MiB .sdc limit is unchanged. An opened file handle supplies both
metadata and the bytes, and its read is capped to limit plus one byte. Metadata
is a preflight optimization, not authority to read an unlimited growing file.
An oversized actual read fails before UTF-8 decoding or project preparation.
Normal path loads and Standby checkpoint reads share this bound. Standby retains
its exact byte-count/checksum validation before the project is prepared.

The parser owns no engine, media/device, transaction or publication lifecycle.
The existing preparation/validation/commit owner remains in place; no rendering
hot path, output behavior, file writer or migration adapter is added. The new
corpus calls the actual production parser for raw bytes and canonical reloads.

The bounded-reader unit case injects a source which declares two bytes but
continues producing bytes: a 16-byte test limit consumes exactly 17 bytes and
rejects it. This is a deterministic test of the production read function, not a
timing-dependent real concurrent file-growth observation. The production byte
limit bounds input consumption; JSON Value allocation and allocator overhead
are not claimed to fit inside 64 MiB.

## Software verification

The exact MSVC 14.44.35207 Build Tools linker was pinned and first in PATH.
The maintained focused runner executes the optimized release test binary and
requires the exact selected count, no failed/ignored tests and no compiler
warnings. Final raw results are preserved separately:

- [Project JSON](artifacts/project-json-tests-2026-10-03.txt): 7 passed, 0 failed,
  0 ignored; duplicate keys, valid JSON parity, UTF-8/numbers/depth/trailing input,
  exact 64 MiB boundary and bounded-reader growth/metadata rejection.
- [Production-parser migration corpus](artifacts/project-json-migration-tests-2026-10-03.txt):
  12 passed, 0 failed, 0 ignored; 224 truncations, 5 malformed byte/number cases,
  3 depth cases, 4096 seeded hostile inputs (4091 parser-rejected, 5 scalar values
  parsed, none prepared), and 128 semantic/idempotency cases.
- [Standby checkpoints](artifacts/project-json-standby-tests-2026-10-03.txt):
  2 passed, 0 failed, 0 ignored; integrity/pruning and control-mapping round trip
  through the changed reader/parser.

An attempted package-specific optimization environment override did not take
effect: the observed rustc command used `-C opt-level=3`. The initial run was
allowed to finish; the ineffective settings and misleading runner footer were
removed. The final results above use the corrected runner and make no reduced
optimization claim. They prove test-binary behavior, not a native window.

The existing project-storage, transaction/authority, recovery E3 and publication
E4 focused checks pass. Node syntax, focused Rust-module formatting and
`git diff --check` pass. A read-only strict scan found unique object keys in all
three checked-in `.sdc` files: `samples/phase1-mini-show.sdc` (21612 bytes),
`qa/specimens/show-structural-preflight.sdc` (3349 bytes), and
`qa/specimens/DSF2026-show-alpha10-reference-audio.sdc` (1128562 bytes).
The latter two were not loaded/prepared and no media
was opened. This scan does not replace full golden migration proof.

## Real Windows QA load acceptance

The optimized isolated QA executable SHA-256 is
`edd4dc3dfdfaca9ea1bbf1df4026d86603a38081951a45ac7b69ec96acc7baca`,
written at `2026-10-02T22:14:13Z` (UTC). Its separate application identifier is
`jp.seraf.ktn.syndocal.qa.mcp-lifecycle`; no Tap test-only configuration is used.
The exact process-tree-bound backend session calls production
`load_project_path` with its registered native-window owner and fresh E/R/H fence.
This is backend acceptance, not an external MCP project-load operation.

[Nine native groups pass](artifacts/project-json-native-final-2026-10-03.json):
two project-file groups plus seven existing authentication/lifecycle groups.
A private empty project loads from `valid-日本語.sdc` and publishes 97 BPM.
Eight corrupted files are rejected with the intended errors: duplicate version,
identical duplicate, escaped duplicate, nested BPM duplicate, truncation,
invalid UTF-8, future version 2 and a 67108865-byte file. Every rejection preserves
the canonical checkpoint, current path, E/R/H, publication/mapping/disposition/
recovery/path/history and transport counters, and full output ownership. The
helper requires every observed counter to be present rather than allowing missing
fields to compare equal. Original source SHA-256 values are unchanged.

Both runtime output gates stay closed, with no fixture, Video layer/output or
audio clip in the private project. No Enable/Arm, dialog or DOM action runs.
Each QA launch verifies one responsive maximized QA window. The normal app
descriptor is unchanged; owned QA processes, credential and prefixed temporary
files are cleaned up, with no native panic. The build has zero Rust/TypeScript
warnings and the existing single Vite chunk-size advisory.

## Normal checkout build/window checkpoint

`pnpm --dir app tauri build --no-bundle` exits 0 with the exact MSVC pin and
PATH-first checks. The wrapper stops only the existing exact-checkout executable,
PID 149208; the intended `target/release/syndocal.exe` is refreshed to SHA-256
`8e19cb2d1b2aa8e3633cadd79d78f59a44661adffb7b7b780e34faf135dd7f29`.
Its UTC write time `2026-10-02T22:19:33.6566299Z` is later than the normal build
log's UTC creation time `2026-10-02T22:16:32.0566630Z`.

Plain launch, with no debugger enabled, produces PID 53260. The exact executable
owns one visible responsive `Syndocal` window, maximized through the backend;
the [passive window/broker proof](artifacts/project-json-normal-native-2026-10-03.json)
also records unauthenticated-read rejection. It does not measure native Control
geometry or claim authenticated normal-profile project-load acceptance.

The [QA build log](artifacts/project-json-qa-build-2026-10-03.txt) and
[normal build log](artifacts/project-json-normal-build-2026-10-03.txt) retain actual
optimized build and linker evidence. Compared with the immediately preceding
`bacd1f18` native checkpoint, compiler warning baseline/current/delta is Rust
0/0/0 and Vite advisory 1/1/0 for these two native configurations. TypeScript
compilation succeeds. The three selected Rust test runs also emit no compiler
warnings. No warning allowance or chunk threshold changes are made.

Builds use the live working tree, including pre-existing protected frontend
changes. Those Remote PIN/DJ ingress files remain unstaged and are not attributed
to this change. This is an internal checkpoint; no product version is changed.
Q1 records this bounded native proof, Q3 migration mitigation is refreshed, and
Q4 has 166 evidence records. All Flow and requirement/risk completion statuses
are preserved: 27 Complete, 23 Open and 8 Deferred.
The maintained completion-ledger and Q1-Q4 validators pass, including exact master
mirror parity and tracked evidence paths; the staged diff has no whitespace errors.

## Acceptance boundary

This tranche repairs project byte/JSON admission. It does not complete O1–O4,
the full independently fixed authored golden corpus, every collection/allocator
limit, DVC/GDTF/archive/media/preset ingress, backup-envelope reading, browser
recovery JSON parsing, real upgrade/downgrade, supported non-Windows identity,
clean installation, physical output or release acceptance.

No subagents or Computer Use are used. Protected Remote PIN/DJ ingress work is
excluded; review is by the implementing agent.
