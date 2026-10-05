# Backup query filesystem effects — 2026-10-05

Branch: `codex/showclock-review-20260912`. Base: `fc070e0b`.
The environment-executable goal remains active. This corrects a claim in
[managed backup](MCP_MANAGED_BACKUP_2026-10-05.md); broader File/AI8 stays Open.

The previous managed authority and target policy called `app_data_subdirectory`,
which creates the directory before returning its path. The pure `observe_in`
unit correctly checked candidate selection without creating a directory, but
did not exercise that real application-path wrapper. The original native probe
used an already-existing QA backup directory and could not detect the missing-
directory effect. Consequently the previous no-directory-creation claim was not
proved for production query wiring.

Managed observation and target validation now use the existing path-only
`app_data_subdirectory_path`. Backup Save v1 also resolves the path without
creating it before calling the existing adjacent authority-checked directory
creation hook. Raw list/load/delete use path-only resolution: a missing list
remains empty, missing load rejects, and unclaimed/unreferenced missing delete
stays a no-op. The normal startup's intentional backup directory provisioning is
unchanged. Directory locations, backup format, canonical IDs, principal grants,
fences, receipts, allocator, retention policy and product version are unchanged.

The path-only resolver and managed functions accept a generic Tauri runtime so
the regression uses the actual application path logic with a configured Tauri
MockRuntime. It creates one exclusively owned empty profile under the platform
local-data base, calls production observation and target policy, and checks that
the managed directory remains absent after both calls and empty listing. Cleanup
removes only these owned empty directories, including on the reproduced panic.
The fixture starts no native window, Syndocal engine or media/output worker and
uses the real platform filesystem. The Tauri `test` feature is a dev-dependency with workspace resolver
2, and the test body is `cfg(test)`; no runtime request selects a test mode.

[Failing regression](artifacts/backup-query-effects-negative-2026-10-05.txt), [pre-fix source freeze](artifacts/backup-query-effects-negative-source-freeze-2026-10-05.json), [48 native file units](artifacts/backup-query-effects-native-tests-2026-10-05.txt), [35 registry/authored units](artifacts/backup-query-effects-registry-tests-2026-10-05.txt), [18 publication regressions](artifacts/backup-query-effects-publication-tests-2026-10-05.txt), [ten backup regressions](artifacts/backup-query-effects-backup-regressions-tests-2026-10-05.txt), [28 real stdio MCP/native groups](artifacts/backup-query-effects-native-2026-10-05.json), [probe log](artifacts/backup-query-effects-native-2026-10-05.txt), [optimized QA](artifacts/backup-query-effects-qa-build-2026-10-05.txt), [ordinary build](artifacts/backup-query-effects-normal-build-2026-10-05.txt), [ordinary window](artifacts/backup-query-effects-normal-window-2026-10-05.json), [source freeze](artifacts/backup-query-effects-source-freeze-2026-10-05.json), [inventory](artifacts/backup-query-effects-inventory-2026-10-05.txt), [E4](artifacts/backup-query-effects-e4-2026-10-05.txt) and [AI0](artifacts/backup-query-effects-ai0-2026-10-05.txt) are retained. The registry includes the existing intentional caught lock-poison fixture panic.

The regression failed before the fix with exactly one selected failed test:
`Real application backup observation created a directory`. The final selected
file suite passes 48, including that same application-path assertion. Existing
registry/authored tests pass 35, publication regressions 18 and backup regressions
10. These counts overlap and are separate from the 28 native groups. Unit builds
use test-only package opt-level 0/codegen 256 and release dependencies; optimized
application gates are separate. The initial fixture compile had one new unused-
parentheses warning; it was removed without a warning allowance before the
accepted run, which has zero current warnings.

The actual authenticated stdio MCP rerun keeps complete backup project/mapping
bytes, metadata/hash and authored/path/disposition/output state, rejection and
exact replay/status/ack/protected-delete/next-sequence checks, revocation after
claim before native execution, and individually owned cleanup preserving all
existing QA backup files. It also rechecks the App Tap callback; no Tap product
code changes in this correction. The absent-directory proof is the actual-path
MockRuntime regression, not a native-window or hardware observation. The native
slice still operates under an already-provisioned backup directory.

Optimized QA artifact: `588856e43d90dcbafc391aacd13c2238212d05c63fb7d429cbd960440755c2db`. Ordinary artifact: `86deee98de48357ac594a536193ea5d6c4cf3f48daec806ec506f43624ba0858`,
PID 196828. The exact ordinary checkout has one responsive visible maximized
`Syndocal` window; unauthenticated requests reject. Ordinary launch has no
debugger or authenticated primary-window mutation. The old exact ordinary window
was closed gracefully before its build; no rejected force termination was
retried. Optimized normal/QA Rust/TypeScript warning/diagnostic baseline/current/
delta is 0/0/0; existing Vite advisory is 1/1/0. Selected unit configuration
warning baseline/current/delta is 1/0/-1 for the retained initial fixture and
final run. Node-only checks have no compiler-warning measurement. No new
dependency version or product version changed.

Native inventory stays 553 with hash
`5290a720424c808fd63dd6dcf1a8c4df313a176e66de659ad43f69a36c9fe22c`,
64 canonical operations, 1,637 legacy / 1,670 canonical source entries and 481
frontend sources. The production change removes redundant directory creation
from the query path; it adds no frame-loop copies, workers, locks or polling.
It does not establish isolated query latency or a large-directory benchmark.
Retention still holds project locks across filesystem work and can delay
competing project mutations on slow storage.

Five protected fingerprints remain unchanged, and their dirty frontend work is
included in both builds. No Computer Use or subagent was used; self-review is
not independent review. The prior managed-backup record becomes explicitly
historical with its no-create evidence gap recorded; all original raw artifacts
remain unchanged. Q4 adds current evidence, while authority statuses remain
27 Complete / 23 Open / eight Deferred.

Canonical backup inspect/list/restore/delete, durable/exported audit, full
retention and external filesystem races, mid-publication revocation, retired-
owner/restart receipt access, security/release/venue and physical gates remain
unaccepted. The prior intermittent File authority-read rejection still has no
proven cause or fix. Continue remaining canonical backend File operations after
this query-effect correction.
