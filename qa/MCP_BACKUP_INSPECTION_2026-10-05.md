# Typed MCP backup inspection — 2026-10-05

Branch: `codex/showclock-review-20260912`. Base: `a97af07e`.
The environment-executable goal remains active; broader File/AI8 stays Open.

Authenticated external MCP can inspect one managed backup through
`syndocal.query.project.backup.inspect.v1` (R0, exact Read grant). Its strict
request is `{schema_version:1, backup_id:<positive JS-safe integer>}`; arbitrary
paths, owners, principals and confirmation fields reject. The native executor
uses the immutable broker dispatch and checks exact current principal authority
before reading and again before returning. Local typed queries retain the same
bounded observation without confirmation. This query neither issues a managed
write target nor reserves a publication ID or creates a directory.

The response includes schema 1, exact backup metadata/byte count, SHA-256 of the
bytes consumed by the same decode, and the existing restore policy's projected
`.sdc` source path. Metadata is bounded (reason/source at most 4,096 UTF-8 bytes,
safe timestamp/ID, lowercase digest); invalid metadata rejects without
truncating its identity. The projected source is not proof that it exists or
authorization to write to it. The response is an observation, not a reservation
or promise that the file will stay unchanged; a future restore must compare its
expected digest during its own preparation.

The new focused inspection module owns filesystem observation. Managed root/ID
policy, canonical unique-key envelope decode and the bounded reader are shared
with existing implementations. The legacy reader still checks the canonical
filename before opening a file, and its parser/error contract is preserved.
Inspection rejects missing/nonregular/symlink leaves and enforces the existing
128 MiB backup bound from opened-handle metadata and actual limit+1 reading.
On Windows the opened leaf uses OPEN_REPARSE_POINT, rejects reparse attributes,
and permits only shared readers while consuming the file. A competing writer
fails explicitly; callers can issue a new read after it closes. There is no
automatic retry. Root/parent reparse races, hostile OS filesystem races and
non-Windows execution are not accepted by these Windows checks.

[Protocol](artifacts/backup-inspection-protocol-tests-2026-10-05.txt), [Native file units](artifacts/backup-inspection-native-tests-2026-10-05.txt), [Registry/authored units](artifacts/backup-inspection-registry-tests-2026-10-05.txt), [Publication regressions](artifacts/backup-inspection-publication-tests-2026-10-05.txt), [Backup regressions](artifacts/backup-inspection-backup-regressions-tests-2026-10-05.txt), [Real stdio MCP/native evidence](artifacts/backup-inspection-native-2026-10-05.json), [Native probe log](artifacts/backup-inspection-native-2026-10-05.txt), [Optimized QA build](artifacts/backup-inspection-qa-build-2026-10-05.txt), [Ordinary build](artifacts/backup-inspection-normal-build-2026-10-05.txt), [Ordinary window](artifacts/backup-inspection-normal-window-2026-10-05.json), [Source freeze](artifacts/backup-inspection-source-freeze-2026-10-05.json), [Inventory](artifacts/backup-inspection-inventory-2026-10-05.txt), [E4](artifacts/backup-inspection-e4-2026-10-05.txt), [AI0](artifacts/backup-inspection-ai0-2026-10-05.txt), [Bridge](artifacts/backup-inspection-bridge-2026-10-05.txt), [Frontend inventory](artifacts/backup-inspection-frontend-invokes-2026-10-05.txt). Raw results are retained separately.

Protocol tests pass five. Native file tests pass 48; selected registry/authored
tests pass 35, publication regressions 18 and backup regressions 12 (including
the two inspection tests). The missing-directory application-path MockRuntime
test now calls the production inspect wrapper too. Exact original whitespace
and Japanese metadata bind the digest, invalid input retains its bytes, and
the Windows competing-writer test rejects then succeeds after explicit close.
Selected unit counts overlap and are not summed. Test-only application opt-level
0/codegen 256 and release dependencies do not replace optimized application
acceptance. Registry/authored tests include an existing intentional caught
lock-poison fixture panic.

32 real authenticated stdio MCP/native groups pass. New groups prove
the exact R0 policy/grant, strict schema and ID-only ingress, byte digest and
metadata equality, unchanged complete project/authority/closed output state,
competing-writer rejection/recovery, and missing/mismatched/future/duplicate-key/
UTF-8/unsafe-metadata/oversize rejection without writes. Corrupt/sparse inputs
replace only this run's individually owned acknowledged backup and are restored
in `finally`; all baseline managed filenames, bytes and summaries survive the
owned cleanup. Existing Save/template/backup replay/status/ack/revocation and
App Tap checks also pass. Inspection observed 6038 bytes in
1012.8 ms including MCP/harness overhead, not an isolated filesystem
benchmark or output-loop measurement. It adds no frame copies, engine workers,
polling or project locks. Its bounded filesystem/parse work runs on the existing
blocking executor; simultaneous large reads and slow storage remain an
unmeasured pressure boundary.

Optimized QA artifact: `7c0249e77b46f0662675e835578aaab610145bfb608fa52c6eaa252cdbadecf3`. Ordinary artifact: `b4fa28756d63c049051f79cf4f5e3408151e96fa4d405ad59f514ee00f72bcaf`,
PID 57652. Both builds pass the pinned PATH-first MSVC 14.44.35207 gate.
The exact ordinary checkout has one responsive visible maximized `Syndocal`
window, and unauthenticated requests reject. Ordinary launch has no debugger or
authenticated primary-window mutation. The previous exact ordinary process
was closed gracefully before its build; no force termination is retried.
Rust/TypeScript warning/diagnostic baseline/current/delta is 0/0/0 in optimized
QA/normal configurations; existing Vite advisory is 1/1/0. Selected units have
zero current compiler warnings; no baseline is inferred for a new test fixture.
Node-only gates have no compiler-warning measurement. No product/dependency
version or persisted backup/journal/input schema changes.

Inventory is 554 native routes, SHA-256
`6267c381b5e9c0a57af1cb7bee41802ea0eae68f8b7662bc6d905e82dc7f4e9c`,
65 canonical operations, 1,638 legacy / 1,671 canonical source entries and 481
frontend sources. The exact reviewed delta is one R0 query/route; existing
mutation classifications are unchanged. Five protected fingerprints remain
unchanged, and their dirty frontend work is included in both artifacts. No
Computer Use or subagent was used; stable self-review is not independent review.

Q4 adds bounded current evidence; authority statuses remain 27 Complete /
23 Open / eight Deferred. Canonical backup list/restore/delete, durable/exported
audit, full retention/filesystem/revocation/restart races, clean-install/security/
release/venue and physical gates remain unaccepted. This does not reproduce the
operator's physical Tap/external-clock symptom or change Tap behavior. Continue
the remaining canonical File operations without promoting unobserved gates.
