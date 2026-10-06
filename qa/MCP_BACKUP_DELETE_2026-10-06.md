# Exact MCP managed backup deletion — 2026-10-06

Branch: codex/showclock-review-20260912. Base: 8a1cc23e.
The environment-executable goal remains active; full File/AI8 remains Open.

Windows external MCP can now delete one managed artifact through
syndocal.project.backup.delete.v1 (R5, exact File grant). A strict schema-1 request
binds the operation, positive JS-safe request/backup IDs, issued project mutation
fence and lowercase original expected_artifact_sha256. Paths, owner/principal and
confirmation overrides reject. External dispatch is immutable and reauthorizes
before execution/replay and immediately before the filesystem effect; it needs no
individual human approval. The local typed wrapper retains native confirmation.

The focused deletion module opens the managed regular, non-reparse leaf with
read/delete access and exclusive sharing. The shared inspection reader validates
and hashes the same bounded read (128 MiB maximum); its full decoded image is
released before publication locks. The original open handle remains alive through
validation and Windows FileDispositionInfo, then closes under the existing
publication/admission/coordinator locks. This prevents a hash(A)/unlink(B) leaf
reopen; no pathname remove fallback, retry, ID allocation, output mutation, worker
or frame-copy path is added. Unsupported platforms fail before file access.

Issued process/window session is checked before and after preparation, preserving
query-before-coordinator lock order. Final checks bind live owner incarnation,
E/R/H/publication generation, pending edits, operator policy, installer claim and
unresolved/unacknowledged durable publication references. The latter guard now
normalizes target spellings, so an extended canonical pending path also protects
the same ordinary managed path. Missing managed directories still derive bounded
root/ID keys without creation; invalid IDs/roots and unverifiable references fail
closed. Existing backup and publication journal schemas are unchanged.

The existing bounded File admission/audit implementation is reused with a generic
serializable request and explicit server-derived request ID; prior File behavior
is covered by the same regressions. Exact terminal single-flight reuses the
existing bounded authored receipt implementation in a dedicated namespace, binding
operation/caller/window/owner incarnation/request ID and the full shape. Within
its process-local retention window (256 terminal facts, followed by bounded 10-minute retired-key tombstones), exact retries return the original success or
failure before new fence/file work; they cannot delete a recreated file. Changed
shape rejects. This is not a durable deletion transaction or authorization to
automatically retry an expired/evicted identity after a crash.

[backup-delete-protocol-tests-2026-10-06.txt](artifacts/backup-delete-protocol-tests-2026-10-06.txt), [backup-delete-protocol-tests-exit-2026-10-06.json](artifacts/backup-delete-protocol-tests-exit-2026-10-06.json), [backup-delete-native-tests-2026-10-06.txt](artifacts/backup-delete-native-tests-2026-10-06.txt), [backup-delete-native-tests-exit-2026-10-06.json](artifacts/backup-delete-native-tests-exit-2026-10-06.json), [backup-delete-native-tests-negative-fixture-2026-10-06.txt](artifacts/backup-delete-native-tests-negative-fixture-2026-10-06.txt), [backup-delete-native-tests-negative-fixture-exit-2026-10-06.json](artifacts/backup-delete-native-tests-negative-fixture-exit-2026-10-06.json), [backup-delete-backup-tests-2026-10-06.txt](artifacts/backup-delete-backup-tests-2026-10-06.txt), [backup-delete-backup-tests-exit-2026-10-06.json](artifacts/backup-delete-backup-tests-exit-2026-10-06.json), [backup-delete-file-tests-2026-10-06.txt](artifacts/backup-delete-file-tests-2026-10-06.txt), [backup-delete-file-tests-exit-2026-10-06.json](artifacts/backup-delete-file-tests-exit-2026-10-06.json), [backup-delete-registry-tests-2026-10-06.txt](artifacts/backup-delete-registry-tests-2026-10-06.txt), [backup-delete-registry-tests-exit-2026-10-06.json](artifacts/backup-delete-registry-tests-exit-2026-10-06.json), [backup-delete-registry-tests-negative-2026-10-06.txt](artifacts/backup-delete-registry-tests-negative-2026-10-06.txt), [backup-delete-registry-tests-negative-exit-2026-10-06.json](artifacts/backup-delete-registry-tests-negative-exit-2026-10-06.json), [backup-delete-unit-binary-2026-10-06.json](artifacts/backup-delete-unit-binary-2026-10-06.json), [backup-delete-native-2026-10-06.json](artifacts/backup-delete-native-2026-10-06.json), [backup-delete-native-2026-10-06.txt](artifacts/backup-delete-native-2026-10-06.txt), [backup-delete-native-exit-2026-10-06.json](artifacts/backup-delete-native-exit-2026-10-06.json), [backup-delete-native-negative-lease-read-2026-10-06.json](artifacts/backup-delete-native-negative-lease-read-2026-10-06.json), [backup-delete-native-negative-lease-read-2026-10-06.txt](artifacts/backup-delete-native-negative-lease-read-2026-10-06.txt), [backup-delete-native-negative-lease-read-exit-2026-10-06.json](artifacts/backup-delete-native-negative-lease-read-exit-2026-10-06.json), [backup-delete-source-freeze-before-lease-read-2026-10-06.json](artifacts/backup-delete-source-freeze-before-lease-read-2026-10-06.json), [backup-delete-native-negative-pressure-2026-10-06.json](artifacts/backup-delete-native-negative-pressure-2026-10-06.json), [backup-delete-native-negative-pressure-2026-10-06.txt](artifacts/backup-delete-native-negative-pressure-2026-10-06.txt), [backup-delete-native-negative-pressure-exit-2026-10-06.json](artifacts/backup-delete-native-negative-pressure-exit-2026-10-06.json), [backup-delete-source-freeze-before-pressure-2026-10-06.json](artifacts/backup-delete-source-freeze-before-pressure-2026-10-06.json), [backup-delete-qa-build-2026-10-06.txt](artifacts/backup-delete-qa-build-2026-10-06.txt), [backup-delete-qa-build-exit-2026-10-06.json](artifacts/backup-delete-qa-build-exit-2026-10-06.json), [backup-delete-normal-build-2026-10-06.txt](artifacts/backup-delete-normal-build-2026-10-06.txt), [backup-delete-normal-build-exit-2026-10-06.json](artifacts/backup-delete-normal-build-exit-2026-10-06.json), [backup-delete-normal-window-2026-10-06.json](artifacts/backup-delete-normal-window-2026-10-06.json), [backup-delete-normal-close-2026-10-06.json](artifacts/backup-delete-normal-close-2026-10-06.json), [backup-delete-source-freeze-2026-10-06.json](artifacts/backup-delete-source-freeze-2026-10-06.json), [backup-delete-profile-preservation-2026-10-06.json](artifacts/backup-delete-profile-preservation-2026-10-06.json), [backup-delete-inventory-2026-10-06.txt](artifacts/backup-delete-inventory-2026-10-06.txt), [backup-delete-e4-corrected-2026-10-06.txt](artifacts/backup-delete-e4-corrected-2026-10-06.txt), [backup-delete-ai0-2026-10-06.txt](artifacts/backup-delete-ai0-2026-10-06.txt), [backup-delete-ai1-corrected-2026-10-06.txt](artifacts/backup-delete-ai1-corrected-2026-10-06.txt), [backup-delete-ai5-2026-10-06.txt](artifacts/backup-delete-ai5-2026-10-06.txt), [backup-delete-bridge-2026-10-06.txt](artifacts/backup-delete-bridge-2026-10-06.txt), [backup-delete-frontend-invokes-2026-10-06.txt](artifacts/backup-delete-frontend-invokes-2026-10-06.txt), [backup-delete-sidecar-2026-10-06.txt](artifacts/backup-delete-sidecar-2026-10-06.txt). Raw results remain separately retained outside Git; Git text copies normalize line-end whitespace only.

Current protocol tests pass 7. Native deletion contracts pass 4, including exact
receipt replay/recreated bytes, wrong hash and competing writer, unissued/stale
fence, final revocation, installer claim, normalized durable pending reference,
and query-session retirement during preparation. Backup regressions pass 22,
File regressions 48, and registry/authored regressions 35; these counts overlap.
The application unit executable uses opt-level 0/codegen 256 with release
dependencies, not optimized native acceptance. Initial fixture setup omitted its
owned directory and failed 0/3 (exit 101); creation was repaired and negative raw
logs retained. Initial registry testing passed 34 and failed one because the new
R5 deletion was absent from two cfg(test) exact classification lists and fell into
the old R0 expectation. Those two lists gained only this ID; risk, audit, adapter,
receipt, rate, consent and capability assertions remain. Negative proof remains. The initial full-native run ended with a typed Overloaded authority observation (exit 1); owned cleanup, primary identity and credential removal passed. The deletion harness unnecessarily issued a new unrelated backup destination before each negative case. It now reuses the already-issued request fence while unchanged whole-project/authority assertions remain; no product retry, weakened assertion or backend limit change is added. Failed raw evidence and its source freeze remain. The second run passed both deletion groups and failed during a post-revocation lease observation (native exception, exit 1, specific error not retained by the old invoke wrapper). Its raw result/source freeze remain. The revocation harness now catches and records redacted query errors; only exact typed Overloaded/Unavailable with retryable=true/resnapshot_required=false permit at most eight fresh reads with bounded spacing. All final lease assertions remain; no mutation, claim, execution or replay is retried. The prior QA publication journal reached 29 of 32 retained origins. A new checked-in private backup-delete-20261006 identifier is built for the final full run rather than deleting/resetting that journal. The old managed baseline artifact remains byte-identical; only the new run profile receives owned publications. All compiled product policies and protected sources remain exact.

46 real private Windows/stdio MCP groups pass, including two added deletion
groups and claim-before-execution revocation. The final eight revocation groups needed no transient query reobservations; the bounded retry branch is source-verified, not dynamically covered by this final run. Exact R5 grants, forged fields,
invalid schema/IDs/hash, unacknowledged receipt, wrong bytes and unissued fence
reject without changing the whole project/authority/output state or owned bytes.
Successful deletion carries original metadata/hash and exact request. Missing-file
and recreated-file replays return identical results, while changed shape rejects
and preserves recreated bytes. Existing File/Backup/list/restore/restart cleanup
and shared App Tap feedback regressions remain enforced. Current shared Tap
callback observes {"bpm":80.21526,"header":"80","footer":"Tapped BPM 80.2"}; this does not establish the operator's Control click or
external clock ingress. Baseline managed backup bytes are preserved and existing journal history is retained (owned publications/acknowledgements append their normal facts);
only individually acknowledged owned probe artifacts are removed. Existing strict
scheduled GUI fixture-autosave recognition remains.

Inventory is 557 commands, fingerprint
43f489eb32cbafe96213f69f9806bf2c8fb68fd5ac911b06c2fa3d0677e1e073.
Rust/TypeScript/MCP canonical IDs increase 67 to 68; FileExport routes 27 to 28,
legacy source total 1,640 to 1,641 and canonical source total 1,673 to 1,674. R0
sets, other route classes, frontend literal invoke count 481 and all exact-set
checks remain. Inventory/E4/AI0/AI1/AI5/bridge/frontend/sidecar checks pass.
44 source hashes and five protected dirty hashes remain stable. Self-review
is not independent review. No subagent or Computer Use is used.

Private optimized artifact: 2b8c51f3b3dd6e20db86d5f294126c389b1a46d3d4ffde9cd09e01d398706453. Ordinary artifact: 904bce181f78bfbf0fcdb7544c909e4302bde0f636d20b600688609e8020e950,
PID 61564. Stable private and ordinary no-bundle builds pass with exact PATH-first
MSVC 14.44.35207. The verified ordinary process closes only gracefully before its
build; final exact artifact owns one responsive visible maximized Syndocal window.
Unauthenticated primary reads reject; no primary debugger or credential bootstrap
is used. Rust/TypeScript baseline/current/delta is 0/0/0; Vite chunk advisory
1/1/0. Unit compilation has zero current first-party warnings. Node-only checks do
not measure compiler warnings. No product/dependency/ABI/schema bump or warning
allowance is added.

Q4 gains bounded current-source evidence only. Statuses remain 27 Complete /
23 Open / 8 Deferred. The old absent QA journal is not reconstructed or re-certified.
This does not accept durable/exported audit or crash publication, atomic concurrent
root/parent/path races, all retention/revocation/storage races, non-Windows deletion,
independent review, hardware/venue/clean-install or release acceptance. Explicit
bounded retention and durable deletion reconciliation remain later work.
