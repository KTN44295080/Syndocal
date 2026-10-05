# Typed MCP managed backup restore — 2026-10-05

Branch: `codex/showclock-review-20260912`. Base: `1c435cbf`.
The environment-executable goal remains active; broader File/AI8 stays Open.

Authenticated external MCP can restore one managed backup through
`syndocal.project.backup.restore.v1` (R5, exact File grant). The shared replacement
request uses a positive JS-safe domain ID, a server-issued complete start fence,
and `restore_backup` with managed backup ID, expected lowercase SHA-256 and an
explicit required nullable expected source-path projection. Arbitrary paths,
owners, principals and confirmation-origin fields reject. The immutable native
broker request supplies caller identity; current authentication/grants are checked
before preparation and by the shared replacement preflights, including final CAS.
External invocation needs no individual human confirmation. Local typed invocation
retains the existing native replacement confirmation.

The focused restoration module reuses inspection's bounded same-handle read and
unique-key envelope validation, then moves the project and every MIDI/OSC/DMX/DJ
mapping family into the canonical loader. There is no parallel restore lifecycle
or cloned full image for query projection. ID/digest/source mismatch, invalid
envelope or absent file rejects before output retirement; digest/source changes
return `file_changed`, invalid/unreadable backups `invalid_project`. Preparation
uses the existing private deny-output Engine persistence normalization. Publication
uses the existing owner/incarnation/generation/fence checks, output retirement,
history/recovery transition and final authority CAS. Playback and both output
gates stay stopped. The successful exact terminal names the acknowledged complete
image, next project epoch/publication generation and projected source path.

Restored data is `unsaved_replacement`, matching existing backup restore behavior:
the original `.sdc` source projection is metadata, not proof of its current bytes,
existence or writability. Neither source nor managed backup is written by restore.
Exact domain replay returns the stored terminal without rereading or replacing
the image again, even after the backup disappears; changed shape with the same
domain ID is `conflict`. Existing New/Open and GUI legacy backup paths remain.
No product, dependency, persisted backup or publication journal schema is changed.

[backup-restore-protocol-tests-2026-10-05.txt](artifacts/backup-restore-protocol-tests-2026-10-05.txt), [backup-restore-backup-tests-2026-10-05.txt](artifacts/backup-restore-backup-tests-2026-10-05.txt), [backup-restore-registry-tests-2026-10-05.txt](artifacts/backup-restore-registry-tests-2026-10-05.txt), [backup-restore-replacement-tests-2026-10-05.txt](artifacts/backup-restore-replacement-tests-2026-10-05.txt), [backup-restore-file-tests-2026-10-05.txt](artifacts/backup-restore-file-tests-2026-10-05.txt), [backup-restore-publication-tests-2026-10-05.txt](artifacts/backup-restore-publication-tests-2026-10-05.txt), [backup-restore-native-2026-10-05.json](artifacts/backup-restore-native-2026-10-05.json), [backup-restore-native-2026-10-05.txt](artifacts/backup-restore-native-2026-10-05.txt), [backup-restore-native-negative-authority-2026-10-05.json](artifacts/backup-restore-native-negative-authority-2026-10-05.json), [backup-restore-native-negative-authority-2026-10-05.txt](artifacts/backup-restore-native-negative-authority-2026-10-05.txt), [backup-restore-native-negative-mapping-2026-10-05.json](artifacts/backup-restore-native-negative-mapping-2026-10-05.json), [backup-restore-native-negative-mapping-2026-10-05.txt](artifacts/backup-restore-native-negative-mapping-2026-10-05.txt), [backup-restore-native-negative-capacity-2026-10-05.json](artifacts/backup-restore-native-negative-capacity-2026-10-05.json), [backup-restore-native-negative-capacity-2026-10-05.txt](artifacts/backup-restore-native-negative-capacity-2026-10-05.txt), [backup-restore-qa-initial-build-2026-10-05.txt](artifacts/backup-restore-qa-initial-build-2026-10-05.txt), [backup-restore-old-journal-preservation-2026-10-05.json](artifacts/backup-restore-old-journal-preservation-2026-10-05.json), [backup-restore-profile-guard-2026-10-05.json](artifacts/backup-restore-profile-guard-2026-10-05.json), [backup-restore-ai5-2026-10-05.txt](artifacts/backup-restore-ai5-2026-10-05.txt), [backup-restore-qa-build-2026-10-05.txt](artifacts/backup-restore-qa-build-2026-10-05.txt), [backup-restore-normal-build-2026-10-05.txt](artifacts/backup-restore-normal-build-2026-10-05.txt), [backup-restore-normal-window-2026-10-05.json](artifacts/backup-restore-normal-window-2026-10-05.json), [backup-restore-source-freeze-2026-10-05.json](artifacts/backup-restore-source-freeze-2026-10-05.json), [backup-restore-inventory-2026-10-05.txt](artifacts/backup-restore-inventory-2026-10-05.txt), [backup-restore-e4-2026-10-05.txt](artifacts/backup-restore-e4-2026-10-05.txt), [backup-restore-ai0-2026-10-05.txt](artifacts/backup-restore-ai0-2026-10-05.txt), [backup-restore-bridge-2026-10-05.txt](artifacts/backup-restore-bridge-2026-10-05.txt), [backup-restore-frontend-invokes-2026-10-05.txt](artifacts/backup-restore-frontend-invokes-2026-10-05.txt), [backup-restore-build-wrapper-2026-10-05.txt](artifacts/backup-restore-build-wrapper-2026-10-05.txt), [backup-restore-sidecar-2026-10-05.txt](artifacts/backup-restore-sidecar-2026-10-05.txt). Raw results remain separately retained.

Six protocol tests pass, including both nullable source cases, required-field
rejection, unsafe IDs/digests/metadata and terminal target binding. Fifteen backup
units pass, including three new restoration tests: full mappings and acknowledged
persistence with `.sdc`/absent/non-project source projections, changed/invalid/missing
rejection before confirmation/publication, and Full/Partial Lock plus exhausted
recovery generation before reading. Existing Windows writer exclusion and bounded
backup inspection tests pass. Registry/authored 35, project replacement
11, file 48 and publication 18 regressions pass.
Selected counts overlap and are not summed. Test-only application opt-level 0 /
codegen 256 with release dependencies is not optimized native acceptance. Existing
intentional caught lock-poison fixture panic is documented in registry results.
The frozen project-replacement route subtotal changed from 10 to 11 for this
additional route. The original failing exact-count result is retained separately;
the exact classification checks remain enforced.

40 actual authenticated stdio MCP/native groups pass. The new lane first
loads an independently frozen authored oracle with video sources/outputs removed
and no physical devices opened; it verifies complete stopped Graph/Effects/Cues/
Timeline bank/Touch data, then adds four explicitly specified nonempty mapping
families. After managed backup creation/ack/inspection and New, Restore returns
that full expected image and all mappings, exact receipt authority, unsaved source
projection and closed outputs. It preserves source/backup bytes. Exact grant denial,
forged identity/consent, all six stale fence fields, digest/source mismatch, missing
and malformed backup reject without changing complete project/authority/output
state. Replay succeeds with the artifact temporarily absent and changed shape
rejects. Revocation after claim rejects before native execution. All created files
are individually owned; baseline managed filenames, bytes and summaries survive
cleanup. Probe corruption/removal affects only the acknowledged backup made by this
run and is restored in `finally`. No backup delete canonical acceptance is claimed.
The first native attempt stopped in the unchanged Save As authority query with
`project_file_authority_query_failed` before reaching restore; the generic wrapper
hides its specific QueryError, so its intermittent cause is not established or
fixed. A fresh independent run reached the new fixture, where passing empty local
mapping drafts to `get_project_checkpoint` was correctly rejected after nonempty
mapping load. The fixture now observes the native authority's actual mappings and
supplies them to the existing strict draft-match check; the production check was
not relaxed. The negative runs and final complete run are recorded separately.
The subsequent shared-profile probe hit the existing 32-origin durable publication
bound in template Save before restore. Its negative result is retained. The old
QA journal is preserved byte-for-byte at its 32 origins; its bound is not raised.
The final proof uses the separately compiled checked-in private identifier
`jp.seraf.ktn.syndocal.qa.mcp-lifecycle.backup-restore-20261005`. The lifecycle checker
accepts only the original and this exact private QA identifier, and backup probes
derive their directory from that validated identifier. No normal-profile override
or journal reset is added. Initial/final QA binaries differ by this isolated
configuration; native product source is identical. No automatic product or harness
retry was added. The mapping observation correction affects Node QA helpers only.

Observed restored artifact: 38059 bytes, 805.7 ms including MCP/renderer/harness
overhead, not an isolated filesystem or engine benchmark. This adds no hot-path
frame copies, polling threads, workers or detached lifecycle. Preparation remains
bounded synchronous filesystem/decode plus the shared private Engine normalization
on the existing blocking executor. Large concurrent restore/read pressure, slow
storage, hostile parent/reparse races and non-Windows runtime remain unmeasured.

Optimized private QA artifact: `d213a351e7a232197bcf5b51f5d7eb5c543a4c5bb9e7fc1360a0f27602b0f905`. Optimized ordinary artifact:
`e6b6894d83d4e90e1d15f2dac5dba2467e59b3ef91e5399af82e0c3e131e2bfe`, PID 58680. Both use exact PATH-first MSVC 14.44.35207 linker
pinning. Ordinary app verification proves one responsive visible maximized exact
checkout `Syndocal` window, unauthenticated reads rejected, debugger disabled.
The previous exact ordinary process was closed gracefully; no force kill or
primary-window debug/authentication bootstrap is retried. Protected dirty frontend
work is preserved and included in both builds.

Rust/TypeScript optimized warning/diagnostic baseline/current/delta is 0/0/0;
existing Vite chunk advisory is 1/1/0. Selected units have zero current compiler
warnings; no baseline is invented for new fixtures. Node-only checks do not measure
compiler warnings. Five protected fingerprints and 35 frozen source fingerprints
are verified after validation and commit. No Computer Use or subagent is used.
Stable self-review is not independent review.

Inventory is 555 native routes, SHA-256
`e155fd6fbf897f345ceb87fda2e6c9464806fe0840cb99585afb0727e302be5f`,
66 canonical operations, 1,639 legacy / 1,672 canonical source entries and 481
frontend sources. The reviewed delta is one R5 restore operation/route; the
counter changes reflect that exact additional source, not relaxed assertions.

Q4 adds bounded evidence; statuses remain 27 Complete / 23 Open / eight Deferred.
Canonical backup list/delete/retention, durable/exported audit, mid-publication
revocation/crash/restart and full filesystem/security/clean-install/release/venue/
physical gates remain unaccepted. This work does not reproduce the operator's
physical Tap/external-clock symptom. The intermittent authority-query error remains
an investigation boundary. Expose its specific bounded QueryError and investigate
the authoritative nonblocking capture path before further canonical File work.
