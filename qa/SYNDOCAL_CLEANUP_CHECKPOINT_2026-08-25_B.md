# Syndocal cleanup checkpoint B — 2026-08-25

## Repository identity and scope

- Branch: `codex/syndocal-v1.2`.
- Cleanup-operation starting `HEAD`: `826f30cd2d4b10f30f917e5c27b4f3088ac198f5` (`826f30c`).
- The shared checkout had advanced concurrently to `1d793d103fab9187b737a865f3405a1346e2f6da` immediately before this documentation-only change. That advance is not owned by, or attributed to, this cleanup checkpoint.
- This checkpoint covers only the ten exact recoverable paths listed below. It does not authorize cleanup of Git internals, Cargo build trees, source files, current release/QA evidence, or active Ox working directories.

## Recoverable cleanup performed

Each target was moved to the Windows Recycle Bin. All ten source paths were then verified absent.

| Exact path | Kind | Files | Logical bytes |
| --- | --- | ---: | ---: |
| `C:\Users\kouty\Documents\KDMX\.tmp\measure-expression.js` | file | 1 | 133,636 |
| `C:\Users\kouty\Documents\KDMX\app\debug.log` | file | 1 | 0 |
| `C:\Users\kouty\Documents\KDMX\app\vite-control.err.log` | file | 1 | 142 |
| `C:\Users\kouty\Documents\KDMX\app\vite-control.out.log` | file | 1 | 242 |
| `C:\Users\kouty\Documents\KDMX\app\vite-dev.err.log` | file | 1 | 393 |
| `C:\Users\kouty\Documents\KDMX\app\vite-dev.out.log` | file | 1 | 17,619 |
| `C:\TEMP\opencode\mustuse-test` | directory | 21 | 293,058 |
| `C:\TEMP\opencode\upstream` | directory | 21 | 355,846 |
| `C:\TEMP\opencode\recon` | directory | 19 | 1,775,691 |
| `C:\TEMP\opencode\ox-p1-bk` | directory | 4 | 42,103 |
| **Total** | **10 targets** | **71** | **2,618,730** |

The KDMX-local temporary/log files account for 6 files and 152,032 bytes. The four completed Ox orphan directories account for 65 files and 2,466,698 bytes.

## Exact safety checks

Before any target was moved:

- `Resolve-Path -LiteralPath` succeeded for every exact path.
- Each resolved KDMX target remained under `C:\Users\kouty\Documents\KDMX`; each Ox target was an exact direct child of `C:\TEMP\opencode`.
- The expected file/directory kind matched every target.
- Reparse-point count was zero for every target and all directory descendants.
- External running-process references were zero for every target.
- Every KDMX-local target was untracked and matched an existing Git ignore rule.
- All targets were sent to the Recycle Bin using literal paths; no permanent-delete fallback was used.

After the move, `Test-Path -LiteralPath` returned false for all ten source paths. Recovery remains possible until the Windows Recycle Bin is emptied.

## Storage inventory and protected dominant trees

The audit snapshot measured the KDMX checkout at **112,335,014,744 bytes**. Its two dominant trees were deliberately protected:

| Protected tree | Logical bytes | Reason for protection |
| --- | ---: | --- |
| `C:\Users\kouty\Documents\KDMX\target\debug` | 101,835,678,600 | The task explicitly protected the complete debug tree and `target\debug\incremental`. It may contain active build state, and deleting a build/cache tree would also trigger dependency-graph and frozen-lockfile rebuild obligations. No approved recurring cleanup target set covered it. |
| `C:\Users\kouty\Documents\KDMX\target\release` | 9,048,414,737 | This is the current release tree and contains the running checkout-local `syndocal.exe`. The executable and current release/QA artifacts were protected; no process was stopped. |

Together these trees account for 110,884,093,337 bytes of the snapshot. Their size is not evidence that they are obsolete, and neither tree was modified or removed.

## Protected Git and Ox state

The following Git-internal candidate remains present and untouched:

- `C:\Users\kouty\Documents\KDMX\.git\objects\7a\tmp_obj_4KtGAn`

It is only a garbage **candidate**, not proven unreachable garbage. Direct deletion inside `.git\objects` is prohibited. Re-evaluate its reachability and repository integrity with Git-native read-only checks only after every parallel Git writer has finished.

The following active or evidence-bearing Ox temporary trees remained present and protected:

- `C:\TEMP\opencode\djlm-review-harness`
- `C:\TEMP\opencode\kdmx-t0-harness`
- `C:\TEMP\opencode\nlm-trust-harness`
- `C:\TEMP\opencode\djlm-harness`
- `C:\TEMP\opencode\oxalpha-al-fixture`

No current-session directory, protected evidence directory, tracked/dirty source file, Git object, or Cargo `target` content was touched.

## Non-claims and unverified boundaries

- The Recycle Bin was not emptied. **2,618,730 bytes left the audited source paths, but no equivalent increase in free disk space is claimed.**
- The 112,335,014,744-byte checkout measurement is the audit snapshot, not a promise that concurrent builds or Git activity have left the live total unchanged.
- No Cargo, Tauri, frontend, native, UI, hardware, warning-ratchet, release, or product-version gate was run for this docs-only checkpoint.
- No native process was stopped or changed. The exact checkout-local release executable remained present and running after cleanup.
- No repository-clean, upstream-equality, Git-object-integrity, or Git-garbage-reachability claim is made.
- No recurring cleanup harness is accepted by this note. Its tracked-checkpoint, focused-safety-test, and independent-adversarial-review gates remain mandatory.

## Next safe actions

1. Let all parallel Git operations reach stable checkpoints before inspecting the Git-internal candidate again.
2. At that stable point, perform Git-native read-only integrity and reachability checks; do not remove the candidate by literal filesystem deletion.
3. Keep each protected Ox tree until its owner confirms the session is complete and a fresh exact-path, reparse-point, process-reference, evidence, and ownership audit proves it orphaned.
4. Treat any future Cargo cleanup as a separately reviewed target set. Preserve `target\debug\incremental` and the running release tree unless a later explicit checkpoint safely supersedes those protections.
5. Have the supervising lane review this isolated document diff and include it in a meaningful checkpoint commit/push only after the owning checkpoint validation passes.
