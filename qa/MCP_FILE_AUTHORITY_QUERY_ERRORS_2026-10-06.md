# MCP file authority query errors — 2026-10-06

Branch: `codex/showclock-review-20260912`. Base: `33ebdffa`.
Implementation and initial compilation began October 5; live acceptance completed
October 6. The environment-executable goal remains active; broader File/AI8 is Open.

File and managed-backup authority reads now retain a failed native fence capture's
canonical `QueryError`, rather than flattening every reason to
`project_file_authority_query_failed`. A focused outbound IPC error module preserves
the existing operation string or moves the existing typed query error unchanged.
It is not an inbound error parser, another query implementation or a fallback.
Local IPC rejects with the exact four-field object; immutable authenticated MCP
execution reaches the existing renderer processor, which exposes `request_rejected`
and `error.native_query`. The fixed message, code, retryable and resnapshot flags
contain no exception/debug payload. Success bodies and other operation errors stay
unchanged. An uncertain mutation does not acquire read retry metadata.

There is no automatic retry, additional human approval, blocking replacement for
the query's `try_lock`, new worker/polling lifecycle, query schema, product version,
dependency, persistence or compatibility fallback. Current authentication, exact
grants, principal/owner incarnations, project generations and publication checks
remain. The old generic authority failure cannot be classified retrospectively:
its underlying reason was discarded. The new pressure reproduction proves a real
coordinator contention failure class, not the historical failure's cause.

[file-authority-error-tests-2026-10-06.txt](artifacts/file-authority-error-tests-2026-10-06.txt), [file-authority-capture-tests-2026-10-06.txt](artifacts/file-authority-capture-tests-2026-10-06.txt), [file-authority-query-tests-2026-10-06.txt](artifacts/file-authority-query-tests-2026-10-06.txt), [file-authority-file-tests-2026-10-06.txt](artifacts/file-authority-file-tests-2026-10-06.txt), [file-authority-registry-tests-2026-10-06.txt](artifacts/file-authority-registry-tests-2026-10-06.txt), [file-authority-bridge-tests-2026-10-06.txt](artifacts/file-authority-bridge-tests-2026-10-06.txt), [file-authority-inventory-2026-10-06.txt](artifacts/file-authority-inventory-2026-10-06.txt), [file-authority-e4-2026-10-06.txt](artifacts/file-authority-e4-2026-10-06.txt), [file-authority-ai0-2026-10-06.txt](artifacts/file-authority-ai0-2026-10-06.txt), [file-authority-ai1-2026-10-06.txt](artifacts/file-authority-ai1-2026-10-06.txt), [file-authority-ai5-2026-10-06.txt](artifacts/file-authority-ai5-2026-10-06.txt), [file-authority-bridge-2026-10-06.txt](artifacts/file-authority-bridge-2026-10-06.txt), [file-authority-frontend-invokes-corrected-2026-10-06.txt](artifacts/file-authority-frontend-invokes-corrected-2026-10-06.txt), [file-authority-build-wrapper-2026-10-06.txt](artifacts/file-authority-build-wrapper-2026-10-06.txt), [file-authority-sidecar-corrected-2026-10-06.txt](artifacts/file-authority-sidecar-corrected-2026-10-06.txt), [file-authority-qa-build-2026-10-06.txt](artifacts/file-authority-qa-build-2026-10-06.txt), [file-authority-source-freeze-2026-10-06.json](artifacts/file-authority-source-freeze-2026-10-06.json), [file-authority-native-2026-10-06.json](artifacts/file-authority-native-2026-10-06.json), [file-authority-native-2026-10-06.txt](artifacts/file-authority-native-2026-10-06.txt), [file-authority-native-exit-2026-10-06.json](artifacts/file-authority-native-exit-2026-10-06.json), [file-authority-native-single-2026-10-06.json](artifacts/file-authority-native-single-2026-10-06.json), [file-authority-native-single-2026-10-06.txt](artifacts/file-authority-native-single-2026-10-06.txt), [file-authority-native-single-exit-2026-10-06.json](artifacts/file-authority-native-single-exit-2026-10-06.json), [file-authority-native-corrected-2026-10-06.json](artifacts/file-authority-native-corrected-2026-10-06.json), [file-authority-native-corrected-2026-10-06.txt](artifacts/file-authority-native-corrected-2026-10-06.txt), [file-authority-native-corrected-exit-2026-10-06.json](artifacts/file-authority-native-corrected-exit-2026-10-06.json), [file-authority-native-sustained-2026-10-06.json](artifacts/file-authority-native-sustained-2026-10-06.json), [file-authority-native-sustained-2026-10-06.txt](artifacts/file-authority-native-sustained-2026-10-06.txt), [file-authority-native-sustained-exit-2026-10-06.json](artifacts/file-authority-native-sustained-exit-2026-10-06.json), [file-authority-native-async-2026-10-06.json](artifacts/file-authority-native-async-2026-10-06.json), [file-authority-native-async-2026-10-06.txt](artifacts/file-authority-native-async-2026-10-06.txt), [file-authority-native-async-exit-2026-10-06.json](artifacts/file-authority-native-async-exit-2026-10-06.json), [file-authority-native-touch-2026-10-06.json](artifacts/file-authority-native-touch-2026-10-06.json), [file-authority-native-touch-2026-10-06.txt](artifacts/file-authority-native-touch-2026-10-06.txt), [file-authority-native-touch-exit-2026-10-06.json](artifacts/file-authority-native-touch-exit-2026-10-06.json), [file-authority-native-open-2026-10-06.json](artifacts/file-authority-native-open-2026-10-06.json), [file-authority-native-open-2026-10-06.txt](artifacts/file-authority-native-open-2026-10-06.txt), [file-authority-native-open-exit-2026-10-06.json](artifacts/file-authority-native-open-exit-2026-10-06.json), [file-authority-native-granted-2026-10-06.json](artifacts/file-authority-native-granted-2026-10-06.json), [file-authority-native-granted-2026-10-06.txt](artifacts/file-authority-native-granted-2026-10-06.txt), [file-authority-native-granted-exit-2026-10-06.json](artifacts/file-authority-native-granted-exit-2026-10-06.json), [file-authority-native-multi-2026-10-06.json](artifacts/file-authority-native-multi-2026-10-06.json), [file-authority-native-multi-2026-10-06.txt](artifacts/file-authority-native-multi-2026-10-06.txt), [file-authority-native-multi-exit-2026-10-06.json](artifacts/file-authority-native-multi-exit-2026-10-06.json), [file-authority-native-files-2026-10-06.json](artifacts/file-authority-native-files-2026-10-06.json), [file-authority-native-files-2026-10-06.txt](artifacts/file-authority-native-files-2026-10-06.txt), [file-authority-native-files-exit-2026-10-06.json](artifacts/file-authority-native-files-exit-2026-10-06.json), [file-authority-native-files-stable-2026-10-06.json](artifacts/file-authority-native-files-stable-2026-10-06.json), [file-authority-native-files-stable-2026-10-06.txt](artifacts/file-authority-native-files-stable-2026-10-06.txt), [file-authority-native-files-stable-exit-2026-10-06.json](artifacts/file-authority-native-files-stable-exit-2026-10-06.json), [file-authority-native-files-autosave-2026-10-06.json](artifacts/file-authority-native-files-autosave-2026-10-06.json), [file-authority-native-files-autosave-2026-10-06.txt](artifacts/file-authority-native-files-autosave-2026-10-06.txt), [file-authority-native-files-autosave-exit-2026-10-06.json](artifacts/file-authority-native-files-autosave-exit-2026-10-06.json), [file-authority-native-revocation-2026-10-06.json](artifacts/file-authority-native-revocation-2026-10-06.json), [file-authority-native-revocation-2026-10-06.txt](artifacts/file-authority-native-revocation-2026-10-06.txt), [file-authority-native-revocation-exit-2026-10-06.json](artifacts/file-authority-native-revocation-exit-2026-10-06.json), [file-authority-native-files-final-2026-10-06.json](artifacts/file-authority-native-files-final-2026-10-06.json), [file-authority-native-files-final-2026-10-06.txt](artifacts/file-authority-native-files-final-2026-10-06.txt), [file-authority-native-files-final-exit-2026-10-06.json](artifacts/file-authority-native-files-final-exit-2026-10-06.json), [file-authority-normal-build-2026-10-06.txt](artifacts/file-authority-normal-build-2026-10-06.txt), [file-authority-autosave-tests-2026-10-06.txt](artifacts/file-authority-autosave-tests-2026-10-06.txt), [file-authority-unit-binary-2026-10-06.json](artifacts/file-authority-unit-binary-2026-10-06.json), [file-authority-journal-observation-2026-10-06.json](artifacts/file-authority-journal-observation-2026-10-06.json), [file-authority-lane-guard-2026-10-06.json](artifacts/file-authority-lane-guard-2026-10-06.json), [file-authority-autosave-observation-2026-10-06.json](artifacts/file-authority-autosave-observation-2026-10-06.json), [file-authority-normal-prebuild-2026-10-06.json](artifacts/file-authority-normal-prebuild-2026-10-06.json), [file-authority-normal-build-exit-2026-10-06.json](artifacts/file-authority-normal-build-exit-2026-10-06.json), [file-authority-lane-guard-2026-10-06.txt](artifacts/file-authority-lane-guard-2026-10-06.txt), [file-authority-normal-window-2026-10-06.json](artifacts/file-authority-normal-window-2026-10-06.json). Raw results remain separately retained.

Two outbound error units prove all 15 canonical QueryError codes preserve their
exact validated wire and existing strings remain strings. Two capture regressions
prove a held real coordinator returns Overloaded within 100 ms without issuing a
fence or changing project/runtime/output authority, and an unknown owner returns
Forbidden before capture/issue. Query 17, file 48, registry/authored 35 and broker 14
units pass. Counts overlap. The application test executable uses opt-level 0 /
codegen 256 with release dependencies; it is not optimized native acceptance.
The production frontend processor passes 16 groups, including both immutable
File authority routes' Overloaded/Unavailable/Forbidden metadata, unchanged string
errors, one invoke per request and uncertain-mutation rejection.

Ten real private Windows/stdin-MCP groups pass. An exact-grant, issued-fence R5 Open
loads an independently constructed valid 64-page, 6,144-control Touch image, all
unbound labels, 1,174,140 bytes. The complete persistence image is compared with
that constructed source. It contains no physical fixtures, video outputs, media
files, playback or output activation. Four owned authenticated stdio clients each
keep one request in flight; the per-client guard and native budget are unchanged.
Eight bounded measurement rounds offer real asynchronous checkpoint captures plus
local and external authority reads. There is no test-only lock-hold IPC or patched
invoke/query implementation.

Observed in about 20.2 seconds: 36,416 checkpoint offers, 665 captures and 35,751
explicit busy rejections; 512 local authority calls, 13 successes and 499 typed
Overloaded failures; 32 external calls, ten successes and 22 typed Overloaded
failures. Both File and Backup authority errors survive the actual MCP route.
Fresh separate reads subsequently succeed. The complete project, authority,
source bytes, absent destination and closed output gates survive measurement.
All clients, probe credentials and QA processes are reaped; native panic list is
empty. This measures the offered harness workload, not a general performance or
real-time guarantee. The fixture uses existing snapshot/capture paths and does not
change product frame copying, allocation, lock or output-loop behavior.

Forty real private native File/Backup/restore and lifecycle groups pass on the same optimized artifact. Save/Save As/template/status/ack, managed creation/inspection, full independently specified restore, exact terminal receipt/replay, rejection, revocation after claim, restart credential isolation and owned cleanup remain enforced. A separate nine-group revocation run passes. Three earlier full-lane attempts are retained: a replacement authority read returned canonical Overloaded; scheduled GUI autosave correctly added an unacknowledged backup; output authority setup failed through the generic backend observer. The latter underlying cause is unavailable and is not inferred from the succeeding run. The revocation observer now retains bounded native error data without retry.

The cleanup oracle continues to compare every baseline file byte and summary exactly. It recognizes at most four additional scheduled backups only when filename, time window, exact fixture source, all keys and complete independently expected project/mappings agree and native unique-key inspection validates the exact SHA. It preserves these autosaves and removes only the acknowledged explicitly created backup. Known schema-declared f32 leaves may differ in decimal JSON representation only when Math.fround values are identical; IDs, durations, mappings and all other leaves remain exact. Two valid and 16 unsafe recognition cases pass. The earlier 38,057-byte autosave remains preserved, with 34 verified f32 spelling differences. This corrects the test's ownership expectation, not product retention or deletion policy.

Earlier native attempts are retained as negative/inconclusive evidence. The first
violated a stdio client's single in-flight tools/call guard. A second helper used
the wrong bundle publication field and was corrected to the actual
`publication_generation` source contract. Short and sustained empty-project probes
produced local Overloaded errors but no external one. The substantial label fixture
needed the established exact-grant MCP Open path and promotion-before-File-grant
order; direct legacy setup and the earlier grant order failed before measurement.
One-client heavy pressure still missed the external rejection. Four separate real
clients finally exercised concurrent native execution without weakening either
guard or the explicit requirement to observe both external error routes. These are
distinct probes with unique evidence paths, not automatic operation replays. The
two setup failures passed through the old generic backend-session observer, so
their exact native error messages are not available and are not inferred here.
Runtime product source is identical across these native attempts.

Optimized private artifact: `85500ccd0554989cd62fa8332c01af8e178171b2470e34b7ee94b1089e78309a`. Ordinary artifact: `1435f4a6fdeb050d5e4d82c397235711b9b723f61312d7063d9d8aee7ae90b13`,
PID 71812. Both use exact PATH-first MSVC 14.44.35207 pinning. The private build's
final optimized completion and artifact are retained; its overnight exec handle
expired before the shell status could be retrieved. Real native probes independently
record exit 0 on that exact artifact. The ordinary no-bundle build records exit 0
and one responsive visible maximized exact-checkout Syndocal window; debugger
disabled and unauthenticated reads rejected. The previously running ordinary
process was absent at the October 6 prebuild check, so no termination was issued.
No ordinary-window debugger/authentication bootstrap was used.

First-party Rust/TypeScript optimized diagnostic baseline/current/delta is 0/0/0.
The existing Vite chunk advisory is 1/1/0. Selected test compilation has zero current
warnings; Node-only checks do not measure compiler warnings. Native routes remain
555 with fingerprint `e155fd6fbf897f345ceb87fda2e6c9464806fe0840cb99585afb0727e302be5f`,
canonical operations 66 and frontend commands 481. Source inventory, E4, AI0/AI1,
AI5, bridge, frontend invokes, build wrapper and sidecar checks pass. There is no
raised limit or warning allowance. Five protected dirty fingerprints and 40 source
fingerprints are verified. Stable self-review is not independent review. No
Computer Use or subagent is used.

The old QA profile's 32-origin journal, preserved in the October 5 checkpoint,
is currently absent. Its profile directory still exists. The current observation
records absence and unknown cause, not historical byte equality or a deletion
attribution. No current harness operation addresses that old journal; owned
cleanup paths are restricted to unique Temp fixture/credential directories and
probe-created backups in the separately compiled private profile. Historical
preservation is not re-certified. The current private journal retains its existing
32-origin bound. No old journal reset, reconstruction or limit increase is added.

Q4 adds bounded current-source evidence; statuses remain 27 Complete / 23 Open /
eight Deferred. This does not establish every File/AI8 operation, the original
opaque error cause, query unavailability/owner-race native reproduction, all
contention budgets, durable/exported audit, crash/mid-publication/OS races,
hardware/venue, independent review or release acceptance. Canonical backup
list/delete/retention is the next environment-executable implementation boundary.
