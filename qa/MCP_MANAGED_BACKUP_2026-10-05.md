# Typed MCP managed backup — 2026-10-05

Branch: `codex/showclock-review-20260912`. Base: `5cb4e474`.
The environment-executable goal remains active; broader File/AI8 acceptance is Open.

Authenticated external MCP can query a server-issued managed backup target and
create the complete current project backup without individual human approval.
The routes are `syndocal.query.project.backup.authority.v1` (R0 Read) and
`syndocal.project.backup.create.v1` (R5 File). Exact principal grants, incarnation,
window owner, revocation, process/session/E/R/H/publication fence and
path/disposition generations remain required. The local canonical writer retains
native confirmation. Existing GUI backup behavior is preserved.

Authority accepts only `{schema_version:1}`. It returns the existing file
authority shape: full fence, generations, next sequential request ID, canonical
managed destination and null target hash. Submit these as the existing
`ProjectFileRequestV1`, with the create operation ID. The server binds the entire
request hash to its derived origin in a process-local table of 256 issued targets
with a five-minute expiry. Authority reads neither reserve an ID nor create the
directory. Generic file authority does not mint a backup target. Arbitrary
managed IDs, changed bodies and another caller's issued target cannot authorize
creation. Exact stored terminal replay/status/ack remain independent of that
issuance expiry. The ordinary allocator's high-water mark advances on an accepted
explicit target, preserving its next-ID behavior.

Managed targets must be canonical `backup-<positive safe integer>.json` paths in
this profile's managed backup directory. Existing targets cannot be replaced.
The adapter reuses durable reservation/staging/terminal/ack and the existing
no-replace publication primitive. Full public request identity remains in native
journal metadata; the envelope's human-readable reason is `MCP backup`, and
source path comes from the authoritative save ticket. Receipt status now includes
optional backup summary and warning fields, omitted when absent. No persisted
journal schema or old input request shape changed.

Directory creation and final publication repeat current authority and issued
target validation immediately before their effects. Retention holds the existing
publication/admission/coordinator locks while reading protected journal targets
and removing eligible backups, avoiding a stale protected-target capture.
Authorization repeats per deletion. A backup successfully written with failed
retention remains a success with the stored warning. This implementation review
does not replace a live retention-race or mid-publication revocation proof.

Native inventory is 553, SHA-256
`5290a720424c808fd63dd6dcf1a8c4df313a176e66de659ad43f69a36c9fe22c`.
There are 64 canonical operations, 1,637 legacy sources and 1,670 canonical
sources; frontend sources remain 481. The writer reuses existing rate and
bounded append-only process-memory audit admission. Audit is not yet durable or
exported.

[Protocol](artifacts/managed-backup-protocol-tests-2026-10-05.txt): four pass. [Native file units](artifacts/managed-backup-native-tests-2026-10-05.txt): 47 pass. [Registry/authored units](artifacts/managed-backup-registry-tests-2026-10-05.txt): 35 pass, including an intentional caught lock-poison fixture panic. [Publication regressions](artifacts/managed-backup-publication-tests-2026-10-05.txt): 18 pass. [Backup regressions](artifacts/managed-backup-backup-regressions-2026-10-05.txt): ten pass. [Real stdio MCP](artifacts/managed-backup-native-2026-10-05.json): 28 native groups pass; [probe log](artifacts/managed-backup-native-2026-10-05.txt), [optimized QA](artifacts/managed-backup-qa-build-2026-10-05.txt), [ordinary build](artifacts/managed-backup-normal-build-2026-10-05.txt), [ordinary window](artifacts/managed-backup-normal-window-2026-10-05.json), [source freeze](artifacts/managed-backup-source-freeze-2026-10-05.json), [inventory](artifacts/managed-backup-inventory-2026-10-05.txt), [bridge](artifacts/managed-backup-bridge-2026-10-05.txt), [E4](artifacts/managed-backup-e4-contract-2026-10-05.txt), [AI0](artifacts/managed-backup-ai0-2026-10-05.txt), and [nine asynchronous Tap regressions](artifacts/managed-backup-tap-tempo-2026-10-05.txt) passed. Superseded [46-unit run](artifacts/managed-backup-before-issued-target-native-tests-2026-10-05.txt) and [QA build](artifacts/managed-backup-before-issued-target-qa-build-2026-10-05.txt) precede issued-target binding.

The native fixture uses actual authenticated stdio MCP and native durable
receipts. It verifies complete project and mapping bytes/hash/source metadata,
unchanged authored/path/disposition/output state, rejected forged/future/replace/
outside/stale requests, exact replay/status/ack/re-ack/next sequence, protected
delete rejection before ack, and revocation after claim before native execution.
Only the known f32 BPM serialization scalar is normalized; every other project
leaf remains exact. It avoids retention against unowned backups by requiring
fewer than ten existing verified summaries before any write, and checks all
existing managed filenames, bytes and summaries after individual owned-ID
cleanup. The source-path string is preserved; this does not prove the referenced
source file exists. Outside-target rejection now occurs at full issued-request
binding before filesystem policy; pure managed-policy tests separately prove the
directory/ID boundaries. No assertion was removed to accept a weaker contract.
The 6,037-byte native backup creation plus full byte/mapping/authority checks took
1,039.5 ms in this harness; this is not isolated disk latency or output-loop
performance. Managed target policy is extracted from the central native file.
Retention still holds the publication/admission/coordinator locks across directory
inspection and deletion, so slow storage can delay competing project mutations.

The first 46-unit run and optimized QA build preceded final issued-target binding
and allocator high-water changes; they are superseded and retained separately.
Final source was compiled and the focused regressions rerun. Selected native
unit runs use test-only opt-level 0/codegen 256 with release dependencies;
optimized app gates are separate. Selected test counts overlap and are not summed.

Optimized QA artifact: `0fe7ef0e94ba86b10a35afd188d8e0b4965e055f5e1e22a69054b7f36f284fea`.
Ordinary artifact: `32c7be9ff04815eb923f1c15e1c8e4114ed13bfce5589008738455661632055a`, PID 180240. Exactly one responsive visible
maximized `Syndocal` window passed the passive gate and unauthenticated requests
reject. The normal app uses no debugger or authenticated primary-window mutation.
Automatic approval review rejected force termination of the old ordinary PID;
the exact owned window accepted a graceful close request and exited before the
build. Force termination was not retried.
Optimized normal/QA Rust/TypeScript warning/diagnostic baseline/current/delta is
0/0/0; existing Vite advisory is 1/1/0. Selected unit runs have zero current
warnings; no baseline is inferred for a test-only configuration. Node-only
checks have no compiler-warning measurement. No warning allowance or version
changed.

The same native slice rechecks the actual App Tap callback and header:
120 -> 80.18727 BPM, readout 80, with an enabled unobstructed
40 x 40 Tap target and a 42 px single-row header. The first Tap begins measurement,
the second derives BPM, and a gap greater than two seconds restarts history.
This batch changes no Tap product code. Current source also shows that continuing
external synchronization clears Tap history; the operator's input/external-clock
symptom is not reproduced by the isolated manual-clock fixture.

Five protected file fingerprints remain unchanged; their dirty frontend work is
included in the builds, so these are not clean frozen distributions. No Computer
Use or subagents were used. Stable self-review is not independent review.

Canonical managed backup restore/delete/list, durable/exported audit, retired
owner or restart receipt access, full retention races, external filesystem
parent/reparse races, mid-publication revocation/crash, update/security/release/
venue and physical output acceptance remain open. The previous File checkpoint's
unclassified intermittent authority-read rejection has no proven cause or fix.
Q4 adds one bounded evidence entry; statuses stay 27 Complete / 23 Open /
eight Deferred. Continue the remaining backend File/audit operations and their
native proof without converting software checks into hardware/release acceptance.
