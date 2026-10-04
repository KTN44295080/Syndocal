# Typed MCP project file publication — 2026-10-05

Branch: `codex/showclock-review-20260912`. Base: `dd8edfc6`.
The environment-executable goal remains active; broader File/AI8 acceptance is Open.

Authenticated external MCP can now Save, Save As and export a user template to
an explicit destination, query its publication receipt and acknowledge it,
without an individual human confirmation. These are six typed canonical routes,
with server-derived window owner/incarnation and principal-scoped origin. The
existing GUI Save/Save As/template selection path remains available. Local use
of the new canonical mutation adapters retains native confirmation.

The API IDs are `syndocal.project.save.v1`, `syndocal.project.save_as.v1`,
`syndocal.project.template.save.v1`, `syndocal.query.project.file.authority.v1`,
`syndocal.query.project.file.status.v1` and `syndocal.project.file.acknowledge.v1`.
Authority takes `{schema_version:1, operation_id:<write ID>, destination:<absolute path>}`.
Use its canonical destination, full process/session/E/R/H/publication fence,
path/disposition generations, `next_request_id` and `target_sha256` in the write
request. A null target hash requires absence; an existing target requires its
exact lowercase SHA-256. The original full write request is also the status/ack
body. IDs advance sequentially per derived origin; acknowledge the terminal
before the next write. No request field can claim an owner, origin, principal or
confirmation bypass. Invalid schema, unsafe integer, hash/path/surface and stale
authority reject before publication.

The adapter reuses the existing durable reservation, staging, clean-save
journal, exact terminal, acknowledgement tombstone and capacity limits. The
whole typed request hash is bound into existing request metadata, including
the target/hash, generations and process/session/publication fence. It does
not add another save pipeline or persisted journal schema. Status projects the
stored receipt without attaching a later live authority bundle. Target hashing
is bounded to 128 MiB and streams a 64 KiB buffer. Fresh authority/session
validation occurs before publication locks; the final CAS checks the real
window incarnation, coordinator and immutable external authorization without
inverting the query/coordinator lock order. Destination/hash validation repeats
at the actual filesystem effect. New targets use the existing no-replace
Windows `MoveFileExW(MOVEFILE_WRITE_THROUGH)` primitive. Existing-target hash
revalidation is not an OS-wide conditional replace against other processes.
Template preflight rejection cannot be relabelled as a success merely because
an identical pre-existing target is observed during failure reconciliation. The final no-replace filesystem error also retains the guard classification: a same-byte racing creator cannot manufacture a successful template receipt. The real Windows unit checks both different and identical existing bytes, preserved staging/target, and new-target success. After this final self-review correction, 44 native units, 34 registry/authored units, 18 publication regressions, both optimized builds and the 22-group native slice were rerun.

Native inventory: 551, hash `0d0c58fa38bb8814f3a0e59771ddbe9a1023f1cf2ef44f7c2e3e3d72eebc4c20`.
Canonical operations: 62; legacy sources: 1,635; canonical sources: 1,668;
frontend sources remain 481. Four R5 File routes use the new publication policy;
two queries use R0 read policy. Mutations have four/second burst-eight admission,
256 caller buckets with 15-minute expiry and 65,536 append-only memory audit
entries with reserved terminal slots. This audit is not durable/exported;
publication receipts already use the durable journal's bounded service.

Superseded evidence from before the final no-replace guard is retained under the `before-no-replace-guard` artifact suffix. Current hashes and linked original artifact names below refer to the final rerun.

Evidence: [Protocol](artifacts/file-publication-protocol-tests-2026-10-05.txt): three pass. [Native units](artifacts/file-publication-native-tests-2026-10-05.txt): 44 pass, including full request-to-journal shape binding and real Windows no-clobber publication. [Registry/authored tests](artifacts/file-publication-registry-tests-2026-10-05.txt): 34 pass (including an intentional caught lock-poison fixture panic). [Publication regressions](artifacts/file-publication-publication-tests-2026-10-05.txt): 18 pass. The native unit package uses test-only opt-level 0/codegen 256 with release dependencies; optimized app gates are separate. Retained initial [compile](artifacts/file-publication-compile-negative-2026-10-05.txt) and [registry](artifacts/file-publication-registry-negative-2026-10-05.txt) failures were corrected by wiring semantic schemas and the exact six-source count delta, without weaker acceptance assertions. [Actual authenticated stdio MCP](artifacts/file-publication-native-2026-10-05.json): 22 groups pass, covering owned Japanese Save As/current-path Save/template complete-byte roundtrips, expected absent/existing target rejection, grants/safe mode/forged/stale/unsafe requests, exact replay/status/ack/re-ack/next sequence, real App Tap BPM followed by saving the new complete project, and three Save/Save As/template revocations after claim before native start. [Optimized QA](artifacts/file-publication-qa-build-2026-10-05.txt) and [ordinary builds](artifacts/file-publication-normal-build-2026-10-05.txt), [ordinary window](artifacts/file-publication-normal-window-2026-10-05.json), [source freeze](artifacts/file-publication-source-freeze-2026-10-05.json), [inventory](artifacts/file-publication-inventory-2026-10-05.txt), [bridge](artifacts/file-publication-bridge-2026-10-05.txt), [E4](artifacts/file-publication-e4-contract-2026-10-05.txt) and [AI0](artifacts/file-publication-ai0-2026-10-05.txt) are retained separately. Retained [live-telemetry comparison failure](artifacts/file-publication-native-negative-2026-10-05.json), [f32 template representation failure](artifacts/file-publication-template-negative-2026-10-05.json) and [unclassified authority-read failure](artifacts/file-publication-revocation-negative-2026-10-05.json) document the harness corrections and the unresolved intermittent read separately. Unit counts overlap and are not summed into the native group count.

Two retained harness failures compared live beat/latency telemetry as though
it were authored state, and shortest-f32 template text against f64 JSON text.
The final check compares the complete persistence image and every authority
field separately from the embedded live observational snapshot; only the known
f32 BPM scalar is normalized to its exact native representation for template
comparison. Every other project leaf remains exact. A later revocation fixture
had one unclassified native authority-read rejection before the second File
case; it did not capture the backend cause. The fixture now records a bounded
native error rather than discarding it. The fourth run passed without a product-code change relative to the unclassified read failure, without retries or disabled assertions. The final rerun includes the separate no-replace classification correction described above. This does not establish a fix or
root cause for that intermittent read; its retained evidence remains a follow-up.

Optimized QA artifact: `3c0942bb145c397bfcfd688abf520c71834328b1079e449181b82a33321fa04b`. Ordinary artifact: `b772473d9f2bc62060773f447caead803bc0b5b39e4e44828ac486f4e221d442`.
The ordinary exact checkout executable has one responsive visible maximized
`Syndocal` window, PID 54388, and unauthenticated requests reject. Its launch
uses no debugger or authenticated primary-window mutation. The earlier before-guard launch helper timed out after 30 seconds waiting for its window; that same PID later passed the passive gate without a duplicate launch. The final ordinary build has its own retained process-owned window gate; no startup-cause fix is claimed.
Optimized normal/QA Rust/TypeScript warning/diagnostic baseline/current/delta:
0/0/0; existing Vite advisory: 1/1/0. Selected unit runs have zero current
warnings; no new test-only configuration baseline is inferred. Node checks have
no compiler-warning measurement. No warning allowance or version changed.

Five protected fingerprints remain unchanged; protected dirty frontend work
is included in these builds, so they are not clean frozen distributions.
No Computer Use/subagent was used; stable self-review is not independent review.
In the final native Tap proof, the actual callback changes 120 BPM to 80.66219; the header reads 81 and footer reads `Tapped BPM 80.7`, and Save preserves that complete project. The engine estimates BPM from the second Tap onward and restarts measurement after a gap greater than two seconds.

This proves owned private-file publication and an actual App Tap callback,
not the operator's physical press or external clock state. Durable/public audit,
retired-owner/restart receipt access, simultaneous external filesystem races,
crash/update/security/release/venue, physical devices and remaining File
operations are unaccepted. Q4 adds bounded evidence; status counts stay
27 Complete / 23 Open / eight Deferred. Next implement the remaining backend
File/audit gaps with separate native proof.
