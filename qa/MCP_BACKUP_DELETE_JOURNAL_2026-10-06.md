# Bounded MCP deletion journal and status — 2026-10-06

Branch: codex/showclock-review-20260912. Base: d0b6fad0.
The environment-executable goal remains active. Full File/AI8 is Open.

Windows managed-backup deletion now writes its exact request, origin/shape hashes
and verified metadata to an additive project-backup-deletions-v1.json before its
same-handle filesystem effect. A succeeded receipt is flushed before success is
returned. A final authorization rejection is recorded as rejected; failure to
record a terminal result leaves prepared/indeterminate. Preparation failures do
not claim a durable rejection. The existing publication/admission/coordinator
locks, exact File grant, issued fence and owner/project/operator/installer guards
remain. No output, worker or frame hot path changes.

R0 syndocal.query.project.backup.delete.status.v1 requires a current exact Read
grant and the original strict delete request. It reports unknown, indeterminate,
succeeded with the original receipt, or rejected with its bounded error. Status
does not create a journal, issue authority, inspect/delete an artifact or repair
state. Busy reads retain canonical QueryError. Exact durable replay precedes the
in-memory result, fresh fence and artifact access. Cached success cannot hide an
invalid journal or delete a recreated leaf. Origin binds principal incarnation,
window owner and request shape; another origin gets no receipt. Unknown means no
matching retained fact, not proof of no prior effect.

Schema 1 is bounded to 256 records/8 MiB with no implicit eviction. UTF-8, duplicate
keys, future version, unknown fields, duplicate identity, phase, receipt and
request-shape validation fail closed. Publication uses an owned create-new temp,
write/sync and existing atomic replacement/no-replace helpers. Temp cleanup never
removes a create-new collision belonging to another owner. Any prepared record
protects its backup from new callers, raw legacy deletion and retention cleanup.
No automatic retry, reconstruction or adoption of retired origins is added.

Protocol 8, deletion/journal 10, Backup 28, File 48 and registry/authored 35 tests
pass; filters overlap. Application units use opt-level 0/codegen 256 with release
dependencies and are distinct from optimized native acceptance. The first native
compilation rejected the stale R0 test array length (25 versus 26); the exact
classification list length was corrected without relaxing assertions. Negative
raw output and exit 101 remain. Actual Windows sharing/failed atomic replacement
preserves original journal bytes and cleans only the owned temp. Loss of the
process receipt state, prepared facts, wrong caller, typed busy/final revocation,
future/duplicate journal despite cached success and existing deletion guards are
covered. Fixtures are not real crash or power-loss observation.

The first full-native probe stopped after two existing File policy/grant groups,
before deletion, with a connection failure during a Save As rejection/status
lane. Its mutation UUID remains in the native replay journal, while the R0
observation returned rejected; polling-stage failure is inferred from those
facts. The owned process stopped and credential cleanup succeeded. The original
UUID is never resubmitted/adopted, and its outcome after restart is unknown. The
initial sent-flag inference was discarded. The transport cause is unproven;
raw negative output, source freeze and the bounded observation remain. A separate
fresh-principal/fresh-request full probe uses the identical artifact/policies.
No product or mutation retry, relaxed assertion, reset or reconstructed state is
introduced. Transport failure isolation remains a wider unverified boundary.

48 real private Windows/stdio MCP groups pass. Two new groups verify the
exact R0 grant/strict request/read purity, native prepared/future journal guards,
cached replay rejection, new-intent and legacy cleanup protection, byte-exact
fixture restoration, and original succeeded receipt afterward. Prepared/future
states are deliberately injected into the current owned private record and then
restored; the run does not certify a crash between filesystem effect and flush.
Existing private Backup/File/list/restore/revocation/restart cleanup checks remain.
The deletion/replay/negative-fixture group took 11013.2 ms; this is the
whole diagnostic group, not isolated delete latency or hot-path performance.

Tap still uses the common App callback wired to Control's header. The current
private run observes {"bpm":80.56796,"header":"81","footer":"Tapped BPM 80.6"} and a 42-pixel single-row header with an enabled,
unobstructed Tap target. First Tap/after a long pause reports a pending interval.
This does not establish the operator's physical Control click or external-clock
ingress. The ordinary executable includes the protected pre-existing App changes.

Native inventory: 558, fingerprint
1bf242a465f867ad965f81d7e418d7cd21d6208a459b4a8fd968f95fa465806d.
Canonical IDs 68→69; ReadOnly 104→105; R0 exact set 25→26; legacy source
1,641→1,642 and canonical source 1,674→1,675. FileExport 28 and frontend literals
481 remain. Inventory/E4/AI0/AI1/AI5/bridge/frontend checks pass. 45 source
hashes and five protected dirty hashes remain exact. Self-review is not independent
review. No subagent or Computer Use is used.

Private optimized artifact: 05af08b5923e5409ed8e9d632cea39ff903c2e5b164d849b205a3f64a3eb71f7. Ordinary artifact: 49d63be062e6ac5dc7efa4dde95ec51ac999487beccb58b5b7be997372a1fe24,
PID 172028. Both no-bundle builds pass exact PATH-first MSVC 14.44.35207. The prior
ordinary process closes only gracefully. The final artifact owns one responsive,
visible, maximized Syndocal window; unauthenticated primary reads reject, and no
primary debugger or credential bootstrap is used. Rust/TypeScript warnings
baseline/current/delta: 0/0/0; Vite chunk advisory: 1/1/0. Node-only checks have no
compiler measurement. No version bump or warning allowance is added.

The ordinary close helper accepted CloseMainWindow for the exact previously
verified PID and observed HasExited, then rejected its final process enumeration.
A subsequent fresh check and the maintained build preflight both reported no
exact process; the build log has no termination action. The bounded close record
retains this failed guard rather than certifying the initial close gate. Its
enumeration cause is unproven. No forced termination was used.

The old private restore profile's journal/managed artifact stays byte-identical;
the current private profile retains its normal publication/audit history and
gains the owned terminal deletion fact. Its prior journal was absent. All fixture
bytes are restored, then only individually acknowledged owned probe files are
removed. Primary identity remains unchanged and the private credential is revoked
and removed. The older absent QA journal is not reconstructed or re-certified.

The first ledger check rejected master/mirror parity: the recording helper used
a string replacement that interpreted a historical literal replacement token in
its inserted JSON. The authoritative fence now uses a replacement callback and
parses equal to the mirror. Product sources and all previous evidence/status rows
are preserved. Git evidence copies also remove trailing blank EOF lines; raw
logs remain separate. Negative validator output is retained.

Q4 gains bounded evidence; completion statuses remain 27 Complete/23 Open/8 Deferred.
Operator resolution, acknowledgement/retention, audit export, actual crash/power
loss/restart reconciliation, hostile root/parent races, all File/revocation/storage
races, non-Windows deletion, independent review, hardware/venue/clean-install and
release acceptance remain unverified. These are not accepted by this checkpoint.

[backup-delete-journal-protocol-tests-2026-10-06.txt](artifacts/backup-delete-journal-protocol-tests-2026-10-06.txt), [backup-delete-journal-protocol-tests-exit-2026-10-06.json](artifacts/backup-delete-journal-protocol-tests-exit-2026-10-06.json), [backup-delete-journal-native-tests-2026-10-06.txt](artifacts/backup-delete-journal-native-tests-2026-10-06.txt), [backup-delete-journal-native-tests-exit-2026-10-06.json](artifacts/backup-delete-journal-native-tests-exit-2026-10-06.json), [backup-delete-journal-native-tests-before-r0-array-2026-10-06.txt](artifacts/backup-delete-journal-native-tests-before-r0-array-2026-10-06.txt), [backup-delete-journal-native-tests-before-r0-array-exit-2026-10-06.json](artifacts/backup-delete-journal-native-tests-before-r0-array-exit-2026-10-06.json), [backup-delete-journal-backup-tests-2026-10-06.txt](artifacts/backup-delete-journal-backup-tests-2026-10-06.txt), [backup-delete-journal-backup-tests-exit-2026-10-06.json](artifacts/backup-delete-journal-backup-tests-exit-2026-10-06.json), [backup-delete-journal-file-tests-2026-10-06.txt](artifacts/backup-delete-journal-file-tests-2026-10-06.txt), [backup-delete-journal-file-tests-exit-2026-10-06.json](artifacts/backup-delete-journal-file-tests-exit-2026-10-06.json), [backup-delete-journal-registry-tests-2026-10-06.txt](artifacts/backup-delete-journal-registry-tests-2026-10-06.txt), [backup-delete-journal-registry-tests-exit-2026-10-06.json](artifacts/backup-delete-journal-registry-tests-exit-2026-10-06.json), [backup-delete-journal-unit-binary-2026-10-06.json](artifacts/backup-delete-journal-unit-binary-2026-10-06.json), [backup-delete-journal-native-2026-10-06.json](artifacts/backup-delete-journal-native-2026-10-06.json), [backup-delete-journal-native-2026-10-06.txt](artifacts/backup-delete-journal-native-2026-10-06.txt), [backup-delete-journal-native-exit-2026-10-06.json](artifacts/backup-delete-journal-native-exit-2026-10-06.json), [backup-delete-journal-native-before-connection-2026-10-06.json](artifacts/backup-delete-journal-native-before-connection-2026-10-06.json), [backup-delete-journal-native-before-connection-2026-10-06.txt](artifacts/backup-delete-journal-native-before-connection-2026-10-06.txt), [backup-delete-journal-native-exit-before-connection-2026-10-06.json](artifacts/backup-delete-journal-native-exit-before-connection-2026-10-06.json), [backup-delete-journal-source-freeze-before-connection-2026-10-06.json](artifacts/backup-delete-journal-source-freeze-before-connection-2026-10-06.json), [backup-delete-journal-native-connection-observation-2026-10-06.json](artifacts/backup-delete-journal-native-connection-observation-2026-10-06.json), [backup-delete-journal-qa-build-2026-10-06.txt](artifacts/backup-delete-journal-qa-build-2026-10-06.txt), [backup-delete-journal-qa-build-exit-2026-10-06.json](artifacts/backup-delete-journal-qa-build-exit-2026-10-06.json), [backup-delete-journal-normal-build-2026-10-06.txt](artifacts/backup-delete-journal-normal-build-2026-10-06.txt), [backup-delete-journal-normal-build-exit-2026-10-06.json](artifacts/backup-delete-journal-normal-build-exit-2026-10-06.json), [backup-delete-journal-normal-window-2026-10-06.json](artifacts/backup-delete-journal-normal-window-2026-10-06.json), [backup-delete-journal-normal-close-2026-10-06.json](artifacts/backup-delete-journal-normal-close-2026-10-06.json), [backup-delete-journal-source-freeze-2026-10-06.json](artifacts/backup-delete-journal-source-freeze-2026-10-06.json), [backup-delete-journal-profile-preservation-2026-10-06.json](artifacts/backup-delete-journal-profile-preservation-2026-10-06.json), [backup-delete-journal-inventory-2026-10-06.txt](artifacts/backup-delete-journal-inventory-2026-10-06.txt), [backup-delete-journal-e4-2026-10-06.txt](artifacts/backup-delete-journal-e4-2026-10-06.txt), [backup-delete-journal-ai0-2026-10-06.txt](artifacts/backup-delete-journal-ai0-2026-10-06.txt), [backup-delete-journal-ai1-2026-10-06.txt](artifacts/backup-delete-journal-ai1-2026-10-06.txt), [backup-delete-journal-sidecar-2026-10-06.txt](artifacts/backup-delete-journal-sidecar-2026-10-06.txt), [backup-delete-journal-bridge-2026-10-06.txt](artifacts/backup-delete-journal-bridge-2026-10-06.txt), [backup-delete-journal-frontend-2026-10-06.txt](artifacts/backup-delete-journal-frontend-2026-10-06.txt). Raw results remain separately retained outside Git; Git text copies normalize line-end whitespace only.

[backup-delete-journal-ledger-before-parity-repair-2026-10-06.txt](artifacts/backup-delete-journal-ledger-before-parity-repair-2026-10-06.txt), [backup-delete-journal-ledger-before-parity-repair-exit-2026-10-06.json](artifacts/backup-delete-journal-ledger-before-parity-repair-exit-2026-10-06.json).
