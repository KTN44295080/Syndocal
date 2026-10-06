# Bounded diagnostic audit export — 2026-10-07

Branch: codex/showclock-review-20260912. Base: 6d85275b936eee341a5329e1c0c5e4d31578acf4.
The environment-executable goal remains active; full AI8/observability is Open.

The shared local/native/MCP factory emits format 2 with five payloads and
integrity-manifest.json. The new audit-history.json exports at most 16 rows per
source: agent authority, output lease, project file, replacement, safety and
output control. Independent source locks use try_lock and never expire/reconcile
leases, evict receipts or copy whole result bodies. A 65,536-row projection test
reads only 16 selected rows. Source order/count/range, hashes, safe integers,
exclusive cursors and continuation headers are validated before publication.

Identities, operation/request IDs and event/outcome text are stable fingerprints;
paths, raw project/result/error bodies and credentials are excluded. File status
Succeeded/Acknowledged is true, Cancelled/Abandoned/Failed is false, and
Reserved/Selecting/Selected/Prepared/Indeterminate/Missing remains unknown.
A successful native status read never implies an unknown publication succeeded.

MCP returns source headers and accepts auditBefore plus the exact observed
expectedProcessIncarnation. Each older page needs a new explicit request UUID
and destination. Stale processes, future/expired cursors, busy/poisoned sources
and inconsistent payloads fail before publication. No mutation retries or
fabricated empty history are added. Exact immutable principal/grant authority
is rechecked after capture before the existing write-new publisher. This is
not an atomic revocation/filesystem transaction. Local preview/confirmation
remains; external MCP has no individual human approval.

Compatibility: strict writer/validator supports format 2 only and rejects
retired format 1, missing audits and future formats. Previously saved ZIPs and
user data are untouched; there is no ZIP import or migration path. Archive/
sanitized/entry limits remain 160/128/64 KiB. Hash integrity is not authenticity.

Pinned PATH-first MSVC 14.44.35207: 66 unit tests selected, 65 passed, zero failed,
one inherited-stderr child helper intentionally ignored. The affected broker
subset passed 23/23 with zero ignored; subsets overlap. Application test code
uses opt-level 0/codegen-units 256 with release dependencies; it is not optimized
native acceptance. Final optimized private and normal no-bundle builds exited 0.
The private artifact 9434e6d9ffab3e5ea1c84737e52ad736b476f200a7ae6324217ec95dcd623e47 passed 17
real native/stdio groups: source coverage, two disjoint authority pages, process/
future rejection without files, schema rejection for missing process, digest,
replay/shape/no-overwrite, local preview/cancel/race/retired capture, exact grants,
safe mode and pre-execution revocation. A controlled unit proves revocation
during capture prevents publication and never repeats capture.

The full authenticated export request took 757.33 ms
and produced 21198 bytes; this includes transport/admission/publication
and terminal receipt, not engine tick or source-lock timing. Native proof records
independent source hashes. 63 source hashes and five protected
files remained stable; earlier backup/recovery/deletion files stayed byte-identical.
Normal artifact 25dcd6ab9358db5f8b8a96e6d7a5b903fb0b4f36d9343b571c939a1bcc9eeb39: one visible responsive maximized
Syndocal window, unauthenticated broker request rejected. Normal close was graceful
only; no primary debugger, forced termination, subagents or Computer Use.

Initial unit failures were stale five-entry expectations and a four-payload
production fixture; logs remain. They were updated to the intentional six-file
contract without weakening privacy/integrity/rejection assertions. Self-review
then corrected File status success semantics and the observation harness's
integrity filename/MCP schema-error boundary before native acceptance. Earlier
passing units/private build predate that correction and remain bounded history.
The first real-native attempt detected a Tauri unmanaged-state panic during
diagnostic acquisition and completed zero groups. Runtime state is owned inside
AppState, not independently managed; acquisition now follows that owner and uses
non-panicking try_state for registered services. The failed JSON/log/exit remain.
The original normal process and all prior backup/recovery/deletion bytes stayed
unchanged. Full units and optimized native proof were rerun on the corrected
wiring; the successful native record contains no panic locations.
The next native attempt passed local/export/grant/lease groups but exposed a
destination-only wire DTO rejecting pagination before execution. Wire admission
and execution now share one exact DTO; structural invalid cursors reject at
ingress while current-process/range checks remain source-owned. Omitted optional
fields retain the legacy destination-only serialized/hash shape. The partial
failure and new wire success/rejection/legacy-shape regression remain recorded.
The following probe reached both pages but stopped on the harness's assumed
request_rejected code. The existing production agentBridgeTools uses
mutation_not_confirmed after native mutation dispatch; only that incorrect
expectation changed. Specific process/future reason and no-file assertions remain.
Final Rust/TypeScript compiler warnings: baseline 0/current 0/delta 0. Existing
Vite chunk advisory: baseline 1/current 1/delta 0; no thresholds were changed.
Self-review is not independent review.

Current-source diagnostic format 2 exports bounded redacted pages from six retained audit sources through exact File grants, process-bound cursors, write-new publication and exact replay. Native pagination and stale/future/no-process rejection passed. This does not establish full attempt-field coverage, atomic cross-source capture, durable audit retention/eviction, actual post-capture native revocation timing, crash/power-loss/removable-filesystem acceptance, independent review, operator Control clicks/external clock input, physical output, updater/support drills or release acceptance.

Next action: audit remaining software-executable retention/attempt-field work
against the canonical authority boundaries. Keep unobserved native/physical,
crash, support and release gates open. Ledger: 27 Complete / 23 Open / 8 Deferred.
