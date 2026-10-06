# Explicit MCP deletion-journal management — 2026-10-06

Branch: codex/showclock-review-20260912. Base: 84698234e626dac8b78b4a3de667f98ce4170622.
The environment-executable goal remains active; full File/AI8 is Open.

The canonical journal/status queries require exact Read grants, expose at most
16 redacted rows per page and preserve typed busy errors. Opaque record hashes,
the native process incarnation, storage generation and exact raw-byte journal
hash support explicit management. Owner/principal identities and paths are not
returned. An absent journal uses an opaque absence fingerprint, not a byte hash
of a manufactured empty journal. Exact management status accepts the complete original request and
cannot adopt another owner, principal incarnation or native process's result.

R5 management requires an exact File grant, current issued project/window fence,
operator policy and the observed journal generation/hash. External MCP has no
individual human approval; the local wrapper retains native confirmation.
Acknowledgement removes selected terminal facts (including resolved unknowns)
and stores bounded origin/process high-water replay fences. Prepared facts cannot
be acknowledged. Old acknowledged delete/management requests reject
receipt_expired before a live cache or artifact access.

Explicit unknown-protection release exclusively streams the current bounded
regular artifact (64-KiB buffer, at most 128 MiB), or observes absence. It changes
no backup bytes and leaves the original deletion indeterminate/resolved_unknown.
Absence never proves a past deletion succeeded. A new deletion requires a new
explicit issued request. Compaction removes only selected older-native-process
replay fences; current-process fences remain protected. Atomic reclaim combines
selected result acknowledgement with older-process compaction (16 IDs total).
It recovers simultaneously full management/retired tables without partial
publication. If every retired fence belongs to the current native process,
restart is required before such compaction can be authorized.

Storage schema 2 keeps the authoritative project-backup-deletions-v1.json name.
Strict schema 1 reads are pure; only an explicit mutation migrates atomically
while preserving original facts. Version 3, duplicates, extra fields, malformed
phases/shapes, future generations and invalid limits fail closed without repair.
Bounds are 256 deletion facts, 128 management results, 256 retired origins and
8 MiB. There is no automatic eviction. A change and its exact management receipt
share one atomic write under publication/admission/coordinator ownership, with
final reauthorization. Failed/uncertain writes never report success. Issued old
process fences remain invalid after explicit replay-fence compaction.

Verification used exact PATH-first MSVC 14.44.35207. Protocol 3, deletion/journal
19, Backup 37, File 48 and registry 35 selected tests passed with zero ignored
(the subsets overlap). Inventory/E4/AI0/AI1/AI5/bridge/frontend checks passed.
The source inventory is 561 native routes (107 ReadOnly, 29 FileExport), 72
canonical operations, 1645 legacy rows, 1678 source rows and 481 frontend invokes.
Its frozen route fingerprint is
d2143bc9a5e1038d7f149bbec20ad14828057d6837ebf1562409a8e81f6309ac.

The optimized private no-bundle artifact 83ea15b748ffcc249edb2868735154e5d81696cd69529a8806d95b7d763ae3e1 passed all
55 real authenticated stdio/native groups. Present and absent prepared fixtures,
strict grant/shape/hash/fence rejection, competing writers, exact replay,
retirement, atomic reclaim, original File/Backup/restore/revocation and the
shared App Tap/header path passed. Fixture records do not establish actual crash
or power-loss interruption. Management elapsed time is recorded in the native
JSON; it measures the complete QA sequence, not an output tick/frame budget.

The ordinary optimized no-bundle artifact 3cd6348a39b53b133fb7b1c5d50fb0bf64edb016084fcc029e6bbacfe6334c6b
was launched as one responsive visible maximized Syndocal window. Closure of the
previous exact ordinary process was graceful; no primary debugger/bootstrap or
forced termination was used. The 49-source freeze includes five protected dirty
files preserved byte-exact and left unstaged. This is evidence for the measured
dirty tree, not a clean checkout or distributed release. The earlier restore
profile journal/backup remain byte-exact; existing deletion facts in the current
profile remain intact. Native startup does not establish operator Control clicks
or external-clock ingress.

Before commit on 2026-10-07, fresh OS enumeration found no ordinary process
with this exact executable path; the earlier verified PID was 70364. The reason
for its termination is unobserved. The same unchanged ordinary artifact was
relaunched and again passed the one-window/responsive/visible/maximized and
unauthenticated-read rejection gate. Both observations are retained separately.

Retained negative evidence: an extra field on the internally tagged empty
artifact observation was accepted, caught by protocol tests, then fixed with a
strict empty struct variant. The interrupted first compiler run observed one
unused import, which was removed. E0533 on the explicit empty-variant comparison
was fixed; the new R5 operation was added to exact policy assertions. A build's
post-source guard rejected an intentional QA-evidence-runner edit even though
the child build exited 0; compiled Rust/frontend sources were unchanged. A
subsequent stable build/source check resolved that guard. Initial 54-group
native evidence predates atomic reclaim and is retained as bounded history.
The new simultaneous-cap regression reproduces both former capacity failures
and proves their atomic recovery without weakening resource bounds.

A subsequent full native probe stopped at the exact File-status R0 query after
Save As, acknowledgement and exact re-acknowledgement had completed. The native
query returned terminal typed Overloaded (retryable true, resnapshot_required
false); the failure JSON, exit status and log remain preserved. A fresh complete
probe uses a separately issued principal and individually owned test artifacts.
It does not repeat an uncertain mutation or adopt the failed session's receipt.
No retry, assertion relaxation or product error suppression was added.
The final README-only edit clarifies the absent-journal fingerprint; both source
freezes are retained and all compiled/native-harness source hashes stayed equal.
The staged-diff whitespace gate found trailing spaces on two registry-test log
lines. Git evidence copies normalize line-end whitespace only; original raw
temporary logs are preserved unchanged.

Final Rust and TypeScript warnings: baseline 0/current 0/delta 0. Final Vite
chunk advisory: baseline 1/current 1/delta 0; no allowance or threshold change.
No subagents or Computer Use were used. Self-review is not independent review.
Exact R0 redacted deletion-journal/status reads and R5 observed unknown-protection release, explicit terminal acknowledgement, older-process replay-fence compaction and atomic reclaim now pass under issued project/owner fences and fresh exact grants. Storage 1 reads preserve bytes; explicit mutations migrate to strict bounded storage 2. Late retired identities reject before cached results or artifact access. See qa/MCP_BACKUP_DELETION_MANAGEMENT_2026-10-06.md. Current-source 55-group native proof does not establish artifact retention scheduling, audit export, actual crash/power-loss/restart reconciliation, hostile OS root/parent races, every File/revocation/storage race, independent review, operator Control clicks/external clock input, physical output or release acceptance.

Next action: implement and verify the remaining bounded audit-export/retention
work against the authoritative File acceptance boundaries, preserving all
existing state. The ledger remains 27 Complete / 23 Open / 8 Deferred.

Documentation follow-up — 2026-10-07, base e1f90c07: removed the README's stale
claim that canonical backup listing/deletion were unimplemented and clarified
the implemented observational resolution/acknowledgement API and the separate
artifact-retention/crash boundaries. Only README and this note changed; compiled
sources, native harnesses and the five protected files stayed byte-exact. The
earlier 49-source freeze retains the README before this documentation correction.
AI5 authentication/transport checks passed: 16 fake adapter groups, 128 hostile
inputs rejected, transport/security and nine bounded-capacity groups. No native
or physical calls ran. No compiler-warning measurement exists for this docs-only
follow-up.

The next audit task is now grounded in current source: diagnostics.export already
publishes the bounded diagnostic ZIP, while diagnostic_package.rs's fixed payload
allowlist contains manifest/project-summary/engine-telemetry/video-runtime and
integrity only. It does not export audit history. AgentAuthorityService retains a
separate bounded audit ring. A unified redacted audit-history/export path remains
to be implemented and verified against roadmap section 5; diagnostic export must
not be treated as proof of that requirement.
