# Typed project Open canonical state — 2026-10-05

Branch: `codex/showclock-review-20260912`. Base: `57c64c27`.
Follow-up to [typed New/Open](MCP_PROJECT_REPLACEMENT_2026-10-04.md).
The wider AI8/File/migration requirements remain **Open**.

An accepted legacy Open could name a prepared checkpoint which differed from
the Engine's acknowledged persistence image. The new full-image regression
reproduced that fault: 22 passed, one failed. An empty legacy Timeline bank,
Main composition and runtime preview defaults were not all represented by the
prepared image. The prior typed New fix manually filled its known defaults;
typed Open still used the older application preparation result.

Typed New/Open now prepares the candidate with the Engine's existing checked
load and persistence primitives, then applies the application's existing save
projection before replacement/hash capture. The new focused
`crates/engine/src/project_load_preparation.rs` creates an unstarted private
runtime with permanent Standby output ownership. It owns no worker, tick,
renderer, input or device. Enabled primary/additional DMX declarations remain
in the candidate, but no sender is created. The original ingress decides
legacy/current-schema once inside the existing Engine loader; filling a legacy
bank first cannot accidentally change that decision. Invalid current reference
conflicts still reject before confirmation, retirement or publication.

Existing file byte bounds, same-read SHA/parse, exact grants, immutable native
dispatch, owner/process/session/E/R/H/publication fences, local confirmation
policy, external no-individual-approval policy and exact terminal replay remain.
The source `.sdc` is never rewritten. No schema/product version, handler count,
adapter policy or retirement primitive changes. Other local/raw load paths are
unchanged; this correction is scoped to the typed New/Open preparation.

The cost is one candidate-only runtime and persistence snapshot per typed
replacement, before active output retirement. No frame-loop copying, polling
or detached worker is introduced. The measured empty-legacy stdio MCP Open is
755.74 ms end to end, including broker/polling overhead; this is not an
isolated CPU/allocation benchmark or a large-project performance guarantee.

Evidence:

- [Retained failure](artifacts/legacy-open-negative-2026-10-05.txt): the original
  full prepared/ACK comparison fails without weakening its equality.
- [Engine tests](artifacts/legacy-open-engine-tests-2026-10-05.txt): three pass;
  enabled routes/no senders, raw-current conflict rejection, canonical default
  idempotence. Test-only Engine package opt-level 0/codegen 256; dependencies
  retain release optimization. This is not the optimized application gate.
- [Native units](artifacts/legacy-open-native-tests-2026-10-05.txt): 24
  pass. The exact prepared image matches real EngineHandle ACK persistence for
  New, empty legacy, checked-in Phase 1 legacy and the 1,265-leaf authored Control
  fixture. The original fixture/source bytes remain unchanged. Invalid current
  root/bank conflict never reaches confirmation/publication. Existing replacement
  tests remain selected; there is no physical/device I/O in these cases.
  A retained [test guard failure](artifacts/legacy-open-fixture-guard-negative-2026-10-05.txt)
  found that Phase 1 has enabled DMX declarations. That run stopped before
  publication. The final test preserves exact fixture bytes and sets the real
  worker to acknowledged Standby before loading, asserting both capabilities
  stay denied before/after ACK. Full persistence equality is unchanged. This
  test-only correction is excluded from optimized app builds by `cfg(test)`
  and recorded in the source freeze; it does not change the validated product.
- [Actual external MCP](artifacts/legacy-open-native-2026-10-05.json):
  29 groups pass. Authenticated typed New/Open, grant/origin/fence/
  file rejection, exact replay and pre-start revocation regressions remain. The
  added old-format Open at 91 BPM has epoch/publication +1, revision 0, and its
  terminal hash equals the complete readback. The expected canonical image is
  specified before execution from the initial default by removing only the bank
  and Main fields from the source; candidate readback is not copied into the
  expectation. Both gates stay closed and Japanese source bytes remain exact.
- [Tap regression](artifacts/legacy-open-tap-2026-10-05.json): 8 groups
  pass on the same optimized QA artifact; real App callback updates Engine and
  header BPM, with the enabled unobstructed target and one-row header retained.
  This does not reproduce the operator's physical button press/external-clock
  state. The first Tap has no interval and keeps the current BPM.
- [QA build](artifacts/legacy-open-qa-build-2026-10-05.txt): normal optimization,
  separate maintained MCP-lifecycle profile, QA hash `d799978ebb5c02d8e2bc6a14d6b489e8cb311f75b90b96f94466072c79d43121`.
- [Ordinary build](artifacts/legacy-open-normal-build-2026-10-05.txt): required
  `pnpm --dir app tauri build --no-bundle`, normal hash `c08efc0d20c41a63e7b1f4fbfc36dad4e22501ad5a4084127df9132a5d4d84e9`.
  [Window proof](artifacts/legacy-open-normal-window-2026-10-05.json): exactly
  one responsive visible maximized `Syndocal`, PID 8776; unauthenticated
  reads reject. No primary debugger/authenticated mutation is used.
- [Source freeze](artifacts/legacy-open-source-freeze-2026-10-05.json): six owned
  source/harness files, five protected fingerprints unchanged. Native inventory
  remains 545 / 18 hostile fixtures; frontend agent-bridge 15 groups pass.

Rust/TypeScript warning/diagnostic baseline/current/delta is 0/0/0 for the optimized normal/QA configurations;
existing Vite chunk advisory is 1/1/0. Node-only checks have no
compiler-warning measurement. Selected native/Engine unit runs have zero current
warnings; the new Engine test-only configuration has no measured pre-change
baseline. No warning allowance or chunk threshold changed.
No Computer Use/subagent was used; self-review is not independent review.
Protected dirty frontend work remains included, so this is not a clean release.

This does not establish all legacy/current migration shapes, all file operations,
durable/queryable audit, mid-parser/final-CAS revocation races, simultaneous output
retirement atomicity, crash/release/venue or physical device acceptance. Q4 adds
one bounded record; status counts remain 27 Complete / 23 Open / eight Deferred.
Next implement the remaining backend file-operation and audit gaps with their
own native proof. The complete environment-executable goal remains active.
