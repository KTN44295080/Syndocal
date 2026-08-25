# Syndocal cleanup checkpoint — 2026-08-25

## Recoverable cleanup

At approximately 2026-08-25 13:54 JST, the following verified-regenerable paths were moved to the Windows Recycle Bin as a recoverable cleanup:

| Workspace-relative path | Files | Logical bytes |
| --- | ---: | ---: |
| `target\debug\bundle` | 2 | 137,997,097 |
| `target\doc` | 552 | 23,259,825 |
| `tmp\pdfs\daslight-pages` | 7 | 1,263,670 |
| `tmp\pdfs\super-scene-pages` | 8 | 1,390,661 |
| `tmp\tauri.native-qa.json` | 1 | 407 |
| **Total** | **570** | **163,911,660** |

## Preconditions and verification

- Each exact workspace path was resolved before cleanup.
- Tracked-file references: `0`.
- Reparse-point references: `0`.
- Running-process references: none.
- The listed paths were moved to the Windows Recycle Bin and then verified absent from the workspace.
- `target\debug` remains present and protected.
- `target\debug\incremental` remains present and protected.
- Source files and current builds were not deleted.
- Recovery remains possible until the Windows Recycle Bin is emptied.

## Repository handoff

- Branch: `codex/syndocal-v1.2`.
- Baseline `HEAD`: `30ca524e08f7f8dbab7407f928574813a9c7a634`.
- Baseline upstream (`origin/codex/syndocal-v1.2`): `30ca524e08f7f8dbab7407f928574813a9c7a634`.
- The worktree had existing dirty implementation/evidence files at this checkpoint. This note is isolated from them so its Git commit contains no dirty implementation and claims none as committed.

No build, test, release, or product-version change was required for this docs-only cleanup checkpoint.
