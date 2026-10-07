# External authorization audit and canonical risk — 2026-10-07

Branch codex/showclock-review-20260912; base 79b8575940af7a0cd1acf0ede5bf33c661135e93.
The goal stays active; parent completion markers remain Open.

The authorization test first exposed a real canonical risk mismatch: Timeline
Play was R0, so its proper Live grant was refused while a Read grant could
reach a mutation. Five authored operations were also R0. The canonical source
now declares effects enable, cue reorder/rename/delete and Scene create R3;
Timeline Play R2; loop commit and Follow Abort R1. Queries remain R0 and all
R4/R5/S0 classifications and external no-individual-approval decisions remain.
Protocol validators now reject R0 authored/runtime mutation descriptors and
require the corresponding audit policy. Existing acceptance assertions changed
because the former R0 mutation contract was incorrect; no assertion was weakened.
Real exact-grant authorization succeeds in units, and all eight Read-only
mutation attempts reject before dispatch in real authenticated stdio MCP.

Every external R2/R3/R4/R5/S0 canonical authorization observation (allowed or
refused) includes bounded redacted BridgeAttempt metadata: original request
UUID hash, existing canonical argument fingerprint, principal incarnation,
adapter, risk, wall time and exact-grant/no-individual-approval policy.
Repeated checks of one UUID are separate immutable authorization observations;
these are not effect receipts and success does not claim an effect committed.
Wall time is presentation only; clock failure remains null and all expiry/rate
and credential decisions still use their existing monotonic clocks.
The existing 512-row authority ring and explicit expired-page refusal remain.
Only selected 16-row pages are cloned; full params, paths and credentials are
not retained in the new metadata. Fingerprint computation reuses the existing
authorization digest instead of serializing the request a second time.

Diagnostic format 2/audit schema 1 adds optional bridge_attempt metadata. Old
histories without it remain accepted by this build. Sanitization retains only
fixed policy/risk strings, bounded time/incarnation and SHA256 identities;
contradictory/null request identities, wrong sources and forged policy reject.
The older build's validator is not claimed to recognize the new metadata.
No stored user-project or credential migration, command/schema identity change,
product bump or unsafe Read-grant fallback is introduced. Clients must use the
correct existing capability and resnapshot current discovery, not replay a
previous Read-only grant against a mutation.

Pinned PATH-first MSVC 14.44.35207:

- Diagnostic units: 74 passed / 0 failed / 1 intentionally ignored subprocess
  helper (75 listed from the exact current test binary).
- Canonical/source and File tests: 42 passed; bridge tests: 27 passed;
  protocol registry tests: 14 passed. These filtered sets overlap; their sum
  is not reported as a count of unique tests. The extra filtered integration
  binary ran zero tests and is not acceptance evidence.
- The first diagnostic run failed 1 / passed 72 / ignored 1 and returned Cargo
  101. Its unmodified raw evidence is retained; it predates the risk correction.
- Final optimized private and ordinary no-bundle builds exited 0.
- Private executable 3cd7cfd85a976f996e14da1736e4947dbd9461afd673e221b5a9c6abd682676f: 65 actual native/stdio
  checks passed, including eight Read-grant rejections with no admitted receipt,
  unchanged project/output, and redacted R2/R3 denial metadata. The existing
  Save/Save As/template/backup/delete/management/restore, exact replay, Tap,
  revocation and lifecycle checks also passed with no native panic locations.
  Tap exercised the real App callback in the isolated native QA build; primary
  Control mouse-click interaction remains unobserved.
- Both post-delete/post-management diagnostic exports retain actual R5 metadata:
- external-diagnostic-export-after-backup_deleted-typed-success-redaction-replay-and-read-purity: 29819 bytes, 15 R5 authorization rows; 1347.28 ms for authenticated export/status/replay/state reads, not engine tick timing.
- external-diagnostic-export-after-deletion_journal_managed-typed-success-redaction-replay-and-read-purity: 29797 bytes, 15 R5 authorization rows; 1372.57 ms for authenticated export/status/replay/state reads, not engine tick timing.
- Ordinary executable fc7089e3f72846b7392ed5635db60c35bf8008010b471676f4a7fd1386b4ed88: one visible responsive maximized
  Syndocal window; unauthenticated read refused. Previous ordinary process
  closed gracefully only. No primary debugger, force termination, Computer Use,
  subagents or physical output activation.

All original private backups and prior deletion/management/retired journal facts
are preserved. The native probe adds only its own QA facts; whole journal bytes
are not claimed unchanged. Five protected dirty files remain byte-identical and
unstaged. Static bridge 16 groups and exact native 561-command admission pass.
Rust/TypeScript first-party warnings baseline/current/delta 0/0/0; existing Vite
chunk advisory 1/1/0 without threshold suppression. Self-review is not independent
review. The source freeze during the corrected compile is supplemented by the
later complete unit/optimized-build/native source hashes, including the protocol
registry and new observation module.

Current-source canonical mutations reject Read-only grants and require Authored, Live or Runtime grants. External R2-R5/S0 authorization observations retain redacted request/argument identity, principal incarnation, adapter, risk, wall time and approval policy. This does not establish all attempt/effect fields, unauthenticated ingress coverage, every adapter, crash-durable audit, all post-claim R2/R3 revocation paths, independent review, physical output or full AI8/release acceptance.

The evidence-update script initially rejected its own planned JSON because a
field-name replacement selected automated_proof instead of native_proof. No
ledger write occurred. The resume changed only the exact native_proof object,
verified existing artifacts, and restored semantic master/ledger parity. This
was a documentation automation failure, not a product-gate failure.

Next: complete remaining canonical attempt/effect, post-claim revocation and
durable retention boundaries; preserve the unobserved native/physical/release
gates. Ledger: 27 Complete / 23 Open / 8 Deferred.
