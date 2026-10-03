# Authored Control project and Timeline persistence — 2026-10-03

Branch: `codex/showclock-review-20260912`. Base: `ad07f7fb`.
Requirement: `MIGRATION-COMPATIBILITY-001`, bounded O2 follow-up.
The complete O1–O4 requirement remains **Open**.

## Problem and resulting behavior

The application writer cleared root `playing` but retained root position and
bank transport state. Project preparation also admitted a saved root transport
to the Engine, which could resume at that position and alter fixture values.
The Engine's persistence contract already treats Timeline playback/position as
runtime state and writes stopped/zero. The application now uses that same
policy at both preparation and save, for the root and every bank entry.

The focused protocol module `timeline_persistence.rs` owns the policy. Engine
bank preparation/persistence, protocol bank normalization and the application's
bank authoring seam forward to it. Authored root/bank agreement is validated
**before** application cleanup; a conflicting authored entry is still rejected.
No frame-loop work, polling thread, transport adapter or fixture-value shadow
copy is added.

Opening a project, including a version-1 file carrying old playing/position
values, starts stopped at zero. Saving strips previous transport, count-in,
audio revision, epoch/generation, child/loop/follow/Guide/click runtime state.
Authored content, tempo/audio policy, loop/follow configuration and duration
remain. Opening never rewrites the original source file. There is no new schema
or product version. The old application test expecting saved position `12345`
is changed to `0` to enforce the already-established Engine persistence policy;
the negative corpus expectations are not weakened.

## Independent whole-project fixture

[The frozen fixture](migration/authored-control-project.json) has **1,265 scalar
leaves**, in addition to all keys/container shapes and ordered array entries.
It adds two nonempty Timeline banks, one five-node/three-edge Graph, an enabled
LFO Effect, Palette, two Touch pages/eight controls and a second Video clip slot
to the independently authored Phase 1 image. New floating-point literals use
exact binary fractions. The current-schema Cue explicitly declares number `1`;
legacy empty-number behavior remains in the separate Phase 1 oracle.

[The manifest](migration/authored-control-project-oracle.json) pins the fixture
and base identities. This fixture was drafted from fixed literals and protocol
declarations before candidate runs, never from a production save/checkpoint.
Its identity is checked in Rust and in the native helper. There is no fixture
update mode. The existing whole-JSON comparator checks every leaf/type/key/array
position exactly; first/second production save bytes must also agree.

The [authoring audit](artifacts/migration-authored-control-oracle-audit-2026-10-03.json)
checks both fixed-document identities and every original base key/array entry.
All 858 base scalar leaves remain: 853 are unchanged and five deliberately
author the current Cue number, Unicode note, two event lane IDs and group color.
No base key or array entry is removed. Sixteen added subtree roots account for
the new class; no candidate writer/binary/checkpoint is used by the audit.

Six new application corpus tests cover full load/save/reload, nonempty Graph
runtime removal, root and bank transport cleanup, **prepared ingress** before
serialization, and nine corrupt-reference cases. The latter include active
projection disagreement, duplicate bank identity, missing Graph node/fixture,
Touch Cue/group/grid, missing Cue Effect and missing Video-slot media reference.
Failure must retain input bytes. The protocol test compares complete structs,
including runtime fields omitted by JSON, and proves idempotent cleanup.

## Native adaptation and causal separation

The optional `--project-json --authored-controls` harness invokes real Tauri
commands through the isolated QA native backend, without DOM gestures or
Computer Use. It removes all Video sources/outputs and their references; keeps
an explicit empty Main composition; and directs the former Graph Video output
to fixture ColorRed. No file-media/audio path, device-enable or output-arm
command is issued. Both output ownership gates must remain closed.

Stopped Timeline automation intentionally evaluates at its current position,
as existing paused Seek tests require. The frozen pure-writer fixture contains
fixture Dimmer `0` and a position-zero keyframe `65535`; those different values
cannot both describe the native post-tick image. The native fixture therefore
**independently authors Dimmer `65535`**, matching that fixed zero keyframe.
It does not copy an observed candidate value or remove/disable the automation,
Effect or Graph to obtain a pass. The first native neutral load checks the
entire adapted expectation with Effect and Graph enabled. The second injects
root/bank playback/positions and Graph telemetry, and must yield the same whole
checkpoint. A direct `get_snapshot` read must show stopped/zero, unchanged over
250 ms of Engine ticks; canonical reload must retain the complete expectation
and its JSON hash. This is not proof of separately preserving fixture values
against arbitrary running automation or of Video/audio playback.

## Negative reproduction

Before the production fix, the optimized corpus actually selected 21 tests:
**19 passed, 2 failed, 0 ignored**. Root cleanup first differed at
`/snapshot/timeline/position_ms`; bank cleanup first differed at
`/snapshot/timeline_bank/0/playing`.

The pre-fix native artifact (`f6989597979b2892b91f1e8b8b968b6ec0364fc1aae7fbe97426df2217e31fc3`)
passed the complete stopped adapted fixture. Loading the otherwise-identical
transport-contaminated file then produced Dimmer `51969` instead of `65535`.
That value is an observation, not a golden. Source tracing confirmed root
playback/position restoration and per-tick Timeline automation writes; Effect
modulation operates on the render path and was not the direct cause.

## Verification

The exact MSVC Build Tools `14.44.35207` absolute linker pin and PATH-first
checks passed. [The optimized Rust run](artifacts/migration-authored-control-tests-2026-10-03.txt)
selected **26 distinct tests: 22 corpus + one writer + one protocol + two Engine**;
all passed, zero failed/ignored. The existing 224 truncations, five malformed
byte/number cases, three depth cases, 4096 bounded hostile byte cases and 128
seeded whole-image variants still ran. The protocol and Engine selections use
`--lib`, retaining exact test counts and rejecting empty selections. The
[initial target-selection run](artifacts/authored-control-rust-target-selection-before-2026-10-03.txt)
compiled the application/protocol and passed their selected tests, but its
one-binary accounting rejected Cargo's extra zero-test integration target;
that runner issue was corrected by pinning the library target, not ignoring a
failure or changing product assertions.

[The maintained corpus entry point](artifacts/migration-authored-control-maintained-runner-2026-10-03.txt)
also selected/passed 22, and [the parser/writer runner](artifacts/authored-control-writer-runner-2026-10-03.txt)
selected/passed the existing one writer test. These reruns validate the updated
runner selections; duplicate cases are not counted again in the 26. Storage,
open bootstrap, transaction/authority and strict-JSON checks passed; the latter
ran 130 assertions. Both oracle JSON files and the authoring audit passed the
strict parser. Focused new-module Rust formatting and all changed Node syntax
checks passed.

[The QA native build](artifacts/authored-control-qa-build-2026-10-03.txt) succeeded
and produced SHA-256
`8b5c383d846ad71256ecb29d3cc7bd701bedd530c59238eb799073933e7ed412`.
Its modification time was verified newer than the build log's creation time,
and its identity differs from the pre-fix artifact.
[Native evidence](artifacts/authored-control-native-2026-10-03.json) passed
**11 groups**: two default JSON admission/preservation, two rich authored
checkpoint cases and seven existing lifecycle/authentication/restart groups.
Live transport was stopped/zero after contaminated load, stayed identical
across 250 ms of ticks, and was stopped/zero after canonical reload. Full adapted
checkpoint equality, unchanged original source bytes and closed ownership
gates passed. QA credentials/process were cleaned; the ordinary instance's
descriptor was unchanged. No native panic location was observed.

[The ordinary build](artifacts/authored-control-normal-build-2026-10-03.txt)
passed `pnpm.cmd --dir app tauri build --no-bundle`. The wrapper verified/stopped
only PID 72000 at this checkout's exact executable path immediately before
building. The new executable's modification time was verified newer than the
build log's creation time. It was launched without a debugger, then backend
window maximization and [the ordinary native gate](artifacts/authored-control-normal-window-2026-10-03.json)
verified PID 193772: exactly one visible, responsive, maximized
`Syndocal` window. Unauthenticated broker reads remain rejected.
Ordinary SHA-256: `c55736c821838bcf32153d2bb4928a09998486c0f998d3a25216ae2ccd6feb0b`.

First-party Rust warnings: preceding optimized/native baseline **0**, current
optimized tests/QA/ordinary builds **0**, delta **0**. The existing Vite chunk
warning is baseline/current **1/1**, delta **0**, measured against the preceding
input-diagnostics native build. The unchanged protected App bundle remains
506.76 kB; no warning threshold or suppression was changed. Native artifacts
include the preserved unrelated dirty frontend; they are not clean frozen
release artifacts. Other-owner file identities were checked unchanged and are
excluded from commit.

Both completion and Q1–Q4 ledger validators passed after the evidence update:
**27 Complete / 23 Open / 8 Deferred**, 32 Q1 rows covering 29 domains and 10
source contracts, 15 decisions, 14 risks and **170 evidence records**. Master and
mirror semantic parity passed. Only the migration residual/evidence link, risk
mitigation and new Q4 evidence row/count change; no requirement or risk status
is promoted. Owned staged diff inspection and `git diff --cached --check` passed.

## Remaining boundary

This adds one rich current-schema authoring class and a bounded native
post-install/coordinator checkpoint/reload case. Larger DVC/GDTF, file media,
relink/missing-file, audio/recording, recovery/template/cache/protocol classes,
allocation/count/decompression limits, coverage-guided fuzz, verified fallback
and restart, historical upgrade/downgrade and non-Windows identity remain
incomplete. No physical output, venue, distribution, broad UI or complete
release acceptance is established. User-required no-subagent operation was
preserved; no separate agent review is claimed. Other-owner dirty work is
excluded from the checkpoint.
