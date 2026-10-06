# Typed MCP managed backup listing — 2026-10-06

Branch: `codex/showclock-review-20260912`. Base: `b8266e8f`.
The environment-executable goal remains active; broader File/AI8 stays Open.

Authenticated MCP can now list managed backups through
`syndocal.query.project.backup.list.v1` (R0, exact Read grant). Its strict request
contains schema 1, limit 1..16, and an optional/null positive JS-safe `before_id`.
Arbitrary path, destination, owner, principal and confirmation fields reject.
The immutable native broker dispatch checks the current exact grant before
filesystem work and again before returning. Local typed reads use the same policy
without a dialog. Existing legacy GUI listing and persisted backup format remain.

A page descends by managed ID. Its items use the shared canonical inspection
reader to return validated original metadata, SHA of those exact bytes and the
existing restore-source projection. A next cursor is the last returned ID only
when additional eligible names were observed. This is a fresh page observation,
not an atomic directory snapshot, reservation or authorization to restore. A
later restore must obtain current authority and compare its own expected SHA.
Unselected file contents are not inspected. The existing native result-size guard also applies; large escaped metadata pages may require a smaller requested limit.

The focused listing module bounds enumeration at 128 directory entries, response
at 16 items and cumulative consumption at 128 MiB per page. The canonical reader
accepts the remaining byte budget, while its unchanged single-file entry point
retains the existing 128 MiB cap. Files are decoded one at a time; the response
copies only bounded metadata, not project images. Invalid managed filenames (including case aliases),
selected corrupt/future/duplicate-key/unsafe/oversize artifacts, nonregular leaves
and Windows competing writers fail explicitly instead of being silently omitted.
Missing directories yield an empty page without creation. No allocator, write,
publication, project/output mutex, engine/frame copy, worker, polling or retry is
added. Existing Windows opened-leaf sharing/reparse checks remain; root/parent
reparse and hostile OS races are not accepted by these checks.

[backup-list-protocol-tests-2026-10-06.txt](artifacts/backup-list-protocol-tests-2026-10-06.txt), [backup-list-protocol-tests-exit-2026-10-06.json](artifacts/backup-list-protocol-tests-exit-2026-10-06.json), [backup-list-native-tests-2026-10-06.txt](artifacts/backup-list-native-tests-2026-10-06.txt), [backup-list-native-tests-exit-2026-10-06.json](artifacts/backup-list-native-tests-exit-2026-10-06.json), [backup-list-native-tests-negative-2026-10-06.txt](artifacts/backup-list-native-tests-negative-2026-10-06.txt), [backup-list-native-tests-negative-exit-2026-10-06.json](artifacts/backup-list-native-tests-negative-exit-2026-10-06.json), [backup-list-backup-tests-2026-10-06.txt](artifacts/backup-list-backup-tests-2026-10-06.txt), [backup-list-registry-tests-2026-10-06.txt](artifacts/backup-list-registry-tests-2026-10-06.txt), [backup-list-unit-binary-2026-10-06.json](artifacts/backup-list-unit-binary-2026-10-06.json), [backup-list-native-2026-10-06.json](artifacts/backup-list-native-2026-10-06.json), [backup-list-native-2026-10-06.txt](artifacts/backup-list-native-2026-10-06.txt), [backup-list-native-exit-2026-10-06.json](artifacts/backup-list-native-exit-2026-10-06.json), [backup-list-tap-native-2026-10-06.json](artifacts/backup-list-tap-native-2026-10-06.json), [backup-list-tap-native-2026-10-06.txt](artifacts/backup-list-tap-native-2026-10-06.txt), [backup-list-tap-native-exit-2026-10-06.json](artifacts/backup-list-tap-native-exit-2026-10-06.json), [backup-list-tap-visuals-2026-10-06.json](artifacts/backup-list-tap-visuals-2026-10-06.json), [backup-list-registry-tests-negative-2026-10-06.txt](artifacts/backup-list-registry-tests-negative-2026-10-06.txt), [backup-list-registry-tests-exit-2026-10-06.json](artifacts/backup-list-registry-tests-exit-2026-10-06.json), [backup-list-qa-build-2026-10-06.txt](artifacts/backup-list-qa-build-2026-10-06.txt), [backup-list-qa-build-exit-2026-10-06.json](artifacts/backup-list-qa-build-exit-2026-10-06.json), [backup-list-normal-build-2026-10-06.txt](artifacts/backup-list-normal-build-2026-10-06.txt), [backup-list-normal-build-exit-2026-10-06.json](artifacts/backup-list-normal-build-exit-2026-10-06.json), [backup-list-normal-window-2026-10-06.json](artifacts/backup-list-normal-window-2026-10-06.json), [backup-list-normal-close-2026-10-06.json](artifacts/backup-list-normal-close-2026-10-06.json), [backup-list-source-freeze-2026-10-06.json](artifacts/backup-list-source-freeze-2026-10-06.json), [backup-list-inventory-2026-10-06.txt](artifacts/backup-list-inventory-2026-10-06.txt), [backup-list-e4-2026-10-06.txt](artifacts/backup-list-e4-2026-10-06.txt), [backup-list-ai0-2026-10-06.txt](artifacts/backup-list-ai0-2026-10-06.txt), [backup-list-ai1-2026-10-06.txt](artifacts/backup-list-ai1-2026-10-06.txt), [backup-list-ai5-2026-10-06.txt](artifacts/backup-list-ai5-2026-10-06.txt), [backup-list-bridge-corrected-2026-10-06.txt](artifacts/backup-list-bridge-corrected-2026-10-06.txt), [backup-list-frontend-invokes-2026-10-06.txt](artifacts/backup-list-frontend-invokes-2026-10-06.txt), [backup-list-sidecar-corrected-2026-10-06.txt](artifacts/backup-list-sidecar-corrected-2026-10-06.txt). Raw results remain separately retained.

Protocol units pass 6, including request bounds and forged-field
rejection, response duplicate/order/identity/cursor/size rejection. Native listing
units pass three: missing-directory purity, independently checked original hashes
and ID pagination, filesystem preservation on invalid entries, directory-cap
rejection, remaining-read budget and Windows writer rejection/recovery. Backup
regressions pass 18 and registry/authored units 35. Counts overlap.
The selected application unit executable uses opt-level 0/codegen 256 and release
dependencies; it is not optimized native acceptance.

43 real private Windows/stdio MCP groups pass. Three added groups exercise
exact R0 grant and strict request admission, original-byte hashes plus complete
page equality with independently observed summaries, one-item pages completing
without duplicates, full project/authority/output/source preservation, and
case-alias/corrupt/future/duplicate-key/UTF-8/unsafe/oversize/writer rejection followed by a
new successful observation. Existing File/Backup/restore/replay/ack/claim-revoke
and restart cleanup regressions remain enforced. Listing 2 items observed
2316.5 ms including MCP/harness overhead, not an isolated filesystem benchmark or
a realtime/output-loop guarantee. Concurrent large reads and slow storage remain
an unmeasured boundary. Probe-created explicit backups are acknowledged before
owned deletion; baseline bytes/summaries remain exact. Recognized scheduled GUI
fixture autosaves are preserved under the existing strict oracle.

Eight separate native lifecycle/Tap groups also pass on this same artifact. The
real opt-in App callback changes 120 to 80.410576 BPM after five scheduled 750ms
taps; the actual header reads 80 and footer Tapped BPM 80.4. First and post-pause
samples report measurement pending. Header height is 42 CSS pixels and the enabled
unobstructed Tap target is 40 by 40. Passive backend screenshots were inspected;
header/footer are readable without an error overlay. PNGs remain outside Git with
paths/hashes in the visual evidence. This validates the shared header/App callback
in Setup, not the operators physical Control click or external ingress/source.
The two native lanes total 51 groups, including overlapping lifecycle checks.
Native inventory gains exactly one R0 route: 556 commands, fingerprint
`1bc8dbe58be058c6a997a82ca5e8d9f15a57966650c323f8393ddee3d5fdf292`.
Rust/TypeScript/MCP canonical IDs increase from 66 to 67; exact-set comparisons
remain. The first focused bridge check rejected the old expected count of 66;
only the reviewed count changed to 67. The first native test compilation failed before executing tests (exit 101, E0308): the explicit R0 array contained the new 25th operation but still declared length 24. Its type length was corrected to 25, preserving every expected ID; the negative compile log remains retained. The first registry run preserved 31 successes but failed four stale exact-count assertions. The reviewed R0 addition makes the read-only route class 103 to 104 and the legacy source total 1,639 to 1,640 (canonical total 1,672 to 1,673). Only those exact expected counts changed; complete ID sets, risk, capability and all other class counts remain enforced. Its negative log is retained. Frontend literal command inventory stays
481, because the new command uses the existing native canonical executor.
Inventory, AI0/AI1/AI5, real processor bridge, sidecar, E4 and frontend invoke checks
pass. Five protected dirty hashes and 41 source hashes are verified. Stable
self-review is not independent review. No Computer Use or subagent is used.

Private optimized artifact: `2ddfe0bc9d7923930f065ed819f13f792012e9680936a18a4949ad47bf4e0912`. Ordinary artifact: `216d0e0ebe924adceb1b750c2433e3d3f1703776c1e6ae317c82fc044ce17df0`,
PID 57252. Both optimized no-bundle builds record exit 0 with exact PATH-first
MSVC 14.44.35207. The exact previously verified ordinary process was closed only
gracefully before building. The resulting ordinary artifact has exactly one
responsive visible maximized Syndocal window; unauthenticated reads reject and
no primary debugger/authentication bootstrap is used. Rust/TypeScript optimized
warning/diagnostic baseline/current/delta remains 0/0/0; existing Vite chunk
advisory remains 1/1/0. Selected native unit compilation has zero current warnings.
Node-only checks do not measure compiler warnings. No product bump, dependency,
warning allowance, limit increase, persisted migration or compatibility fallback.

Q4 adds only bounded current-source evidence. Statuses remain 27 Complete /
23 Open / eight Deferred. The older QA profile's absent journal is not recreated,
reset or re-certified; its unknown cause remains as recorded in the prior note.
The current private journal's 32-origin bound remains. This does not establish
canonical backup delete/retention, durable/exported audit, concurrent directory
snapshot/retention or mid-read revocation/OS races, crash publication, complete
File/AI8, independent review, clean install, physical/venue or release acceptance.
Canonical delete with exact artifact identity and protected-receipt checks is the
next environment-executable implementation boundary.
