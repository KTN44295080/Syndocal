# Syndocal alpha.11 main integration checkpoint

Status: **ACCEPTED — D4 merged, pushed, and rebuilt from the active checkout**

Date: 2026-08-25

## Git identity

- Branch: `codex/syndocal-v1.2`
- Merge commit: `b4a5b62ad48c7e3e58f78c66cf2914e0f53a46f5`
- Merge parents:
  - `3067720c253af06e9fb2c59a5831f4c38a2916dc`
  - `0bfc1c03451373f4cb34eebbfab4c5c257878fe0`
- Push: `origin/codex/syndocal-v1.2`
- Divergence after push: `0/0`
- Final staged-merge review: P0/P1/P2 `0/0/0`

Only `RELEASE_STATUS.md` and
`qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md` conflicted. No source file
conflicted. The resolution preserved the exact-linker alpha.10 history, the
complete alpha.11 D4 payload, and the unbound historical v1.1.1 DJ artifact
non-claim.

## Post-merge gates

Before the merge commit, the resolved index passed:

- release metadata and the 102-group release self-test;
- backend inventories `480/310/133`;
- D2 patch transaction, project transaction, and project authority checks;
- frontend routing inventories `133/29/30/407`;
- exact-linker wrapper `156` assertions;
- production frontend build, `272` modules;
- engine D4 focused Cargo tests `6/6`;
- backend D4 focused Cargo tests `7/7`;
- pane lifecycle focused Cargo tests `4/4`; and
- `git diff --cached --check`.

The focused Cargo runs used the exact VS2022 Community 14.44 linker and emitted
zero first-party warning lines. Independent final review found no unmerged
entries, conflict markers, source loss, or weakened exact-linker gate.

## Main-checkout native rebuild and launch

Immediately before the native build, the exact main-checkout executable
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` was confirmed not
running. The still-running D4 QA executable belonged to the distinct path
`C:\Users\kouty\Documents\kdmx-d4-integration\target\release\syndocal.exe`
and was preserved throughout the build. Daslight was not stopped or modified.

The required command completed successfully:

```powershell
pnpm --dir app tauri build --no-bundle
```

The guarded wrapper initialized VS2022 Community with
`vcvars64.bat -vcvars_ver=14.44`, pinned
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and printed `where.exe link.exe` with that exact linker first and Git's
`C:\Program Files\Git\usr\bin\link.exe` second before Cargo began. The
frontend transformed 272 modules and the Rust release profile finished in
3 minutes 16 seconds. The build emitted zero first-party compiler warning
lines.

Artifact:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- size: `57,888,768` bytes
- ProductVersion/FileVersion: `1.2.0-alpha.11`
- SHA-256:
  `522074E96A235310C9D39D3200E2A9D9B5C68429F0160E92318B568A0FE5AEE2`

After the build, the exact D4 QA process was closed by its independently
verified executable path. The new main-checkout executable launched as PID
`114780`. Exactly one targetable top-level window titled `Syndocal` existed;
the process reported responsive, and a fresh native state capture showed the
window maximized over the 1920x1032 Windows work area. The prior D4 checkpoint
remains the authoritative real-4K 3840x2160-at-150% Stage/Timeline detachment
evidence; this post-merge launch proves the merged main artifact and does not
replace or inflate that 4K claim.

Evidence-retention boundary: the wrapper stdout and native state capture were
observed directly during this checkpoint but were not persisted as a raw log
or image in the workspace. Therefore the 3-minute-16-second duration, zero
warning-line observation, PID, and maximized-window geometry above are
operator-recorded evidence. The Git identity and artifact path, size, version,
and hash remain independently re-verifiable; do not cite this paragraph as a
retained raw capture.

## Verified cleanup checkpoint

After the build finished and the merge was pushed, two independent read-only
reviews identified a high-confidence, rebuildable cleanup set. Immediately
before execution, all exact paths were re-resolved, all expected group byte
totals matched, no reparse points were present, and no
`cargo`/`rustc`/`link`/`cl`/`mspdbsrv`/`msiexec` or matching browser writer was
running.

The following exact set was submitted to the Windows Recycle Bin API and was
removed from its source paths:

- corrupt retired PDB:
  `target/debug/deps/syndocal-a9ff2c31091f28c9.pdb.corrupt-20260813-0531`
  — `358,215,680` bytes;
- obsolete unpacked `target/qa/msi-image-1.0.0` — `285,692,928` bytes;
- 33 old UI/browser scratch directories listed by the cleanup audit —
  `14,447,077` bytes; and
- `C:\temp\opencode\mustuse-test\crate\target` — `525,759,281` bytes.

Execution result: 36 exact targets, 2,245 files, `1,184,114,966` logical bytes
removed from their source paths. All 36 source paths were absent afterward.
However, the independent post-operation audit found no matching current-user
`$I` metadata entry or Shell Recycle Bin item. One unrelated SYSTEM SID recycle
store was inaccessible, so permanent deletion cannot be proven either, but
recoverability is **not verified and must not be relied on**. Treat the removed
set as unrecoverable unless it is later found by exact metadata. The measured
C: free-space delta was `-1,687,552` bytes, so no physical disk-space recovery
is claimed. The active release artifact, the full `target/debug` tree, current
QA evidence, and D4/ASIO worktrees were not removed.

### Rebuildable-cache cleanup tranche 2

At `2026-08-25 08:20 +09:00`, the independently audited second cleanup set was
revalidated immediately before deletion. The main checkout was clean at
upstream divergence `0/0`; every exact file count, logical-byte total, and
newest timestamp matched the audit; no build process or process executable
referenced a candidate; and no WSL distribution was running before the
WSL-only cache was removed. These five exact, rebuildable directories were then
permanently deleted:

- `target/warning-capture`: 10,105 files, `4,454,450,485` logical bytes;
- `target/vendor-wry-review`: 1,851 files, `505,783,503` logical bytes;
- `target/root-warning-review`: 394 files, `179,393,546` logical bytes;
- `target/asio-qa`: 662 files, `267,558,263` logical bytes; and
- `target/wsl-node`: 6,189 files, `247,273,102` logical bytes.

Total: 19,201 files and `5,654,458,899` logical bytes (5.266 GiB). The direct
`Remove-Item` command was operator-reported as blocked, and no deletion is
attributed to it. The same already-reviewed absolute paths were then deleted
one by one through PowerShell's `.NET Directory.Delete` API. All five paths
were absent afterward. This was permanent deletion, not a Recycle Bin
operation, and the removed caches must be treated as unrecoverable.

Before deleting `warning-capture`, the four retained-in-document raw-log
fingerprints were recorded as follows:

- `windows-asio.jsonl`: `3B01D68CE7D6C038665FC9C9A7B1645B65D8EDD39E2B08BAE17EB1DBDF8B79C4`;
- `windows-asio.stderr`: `FB7DCAF8E29B544B12B73D6780B91400199A091D5FDE9AA5CF4CBB0BF9780355`;
- `windows-ndi.jsonl`: `EC687F60FF35568D2396BFC9B82BAD82F87CA18B42A8873144088F6BC8EC9DCC`;
- `windows-ndi.stderr`: `04EF181BC137645D732145B74A7456BE213DD94D6D2BF3F764BA60C1D54613C7`.

Tracked warning evidence remains in `qa/warnings/warning-inventory.json` and
the acceptance/checkpoint documents. C: free space rose from
`399,815,106,560` to `405,225,672,704` bytes, an observed physical gain of
`5,410,566,144` bytes (5.039 GiB). The active `target/release/syndocal.exe`,
`target/asio-sdk-2.3.4`, `C:\Program Files\LLVM`, source trees, and all worktrees
were positively rechecked and retained.

The deletion timestamp, pre-delete counts/bytes, process and WSL checks, API,
hash capture, and free-space before/after values in this tranche are
operator-recorded execution evidence; their raw terminal transcript and the
deleted cache inputs are not retained in the workspace. The independently
re-verifiable post-state is limited to the five source paths being absent and
the protected tracked evidence, SDK, LLVM, release executable, and worktrees
remaining present.

### Rebuildable-cache cleanup tranche 3

At `2026-08-25 08:22 +09:00`, the separately audited
`C:\Users\kouty\Documents\KDMX-d4-stage-transaction\target` was revalidated as
15,257 files and `15,289,402,815` logical bytes, with no root or descendant
reparse points and no process referencing the target. Its owning worktree was
clean on `codex/d4-stage-transaction` at
`854b518175f193b4b75d1f9b1f17e364f144de66`, upstream divergence `0/0`; the
source branch and its six main-external commits remain on origin. No tracked QA
document referred to an executable inside this target, and no release
`syndocal.exe` existed there.

The exact target directory alone was permanently deleted through
`.NET Directory.Delete`; its source path was absent afterward. C: free space
rose from `404,245,049,344` to `417,714,548,736` bytes, an observed physical
gain of `13,469,499,392` bytes (12.544 GiB). The owning source/worktree, branch,
commits, main target, ASIO target/SDK, and running main-checkout Syndocal were
not touched. This deletion is irreversible and makes no product-acceptance
claim.

The deletion timestamp, pre-delete inventory/process/reparse checks, API, and
free-space before/after values in this tranche are operator-recorded execution
evidence; their raw terminal transcript and deleted target contents are not
retained in the workspace. The independently re-verifiable post-state is the
exact target path being absent while the source worktree/branch and protected
targets remain present.

The separately audited
`C:\Users\kouty\Documents\kdmx-d4-integration\target` remained intentionally
untouched at the preceding checkpoint: its release executable was the raw
artifact referenced by the
authoritative D4 real-4K acceptance record. The exact executable now also has a
same-File-ID, same-hash hardlink in the protected
`target/qa/retained-native/d4-alpha11-real4k-63cf795d` store, governed by the
tracked `qa/artifacts/native-executables/d4-alpha11-real4k-63cf795d.sha256`
retention contract. The integration target could be deleted only after this
manifest is committed and pushed, followed by a fresh retained-path hash check.

### Rebuildable-cache cleanup tranche 4

At `2026-08-25`, after the retention manifest above was confirmed tracked on
main HEAD `1402a93069d8d630df66b1f682d0813b2d595faa` and the main checkout was
confirmed equal to its upstream, the exact
`C:\Users\kouty\Documents\KDMX-d4-integration\target` directory was revalidated
for permanent deletion. Its owning worktree was clean and equal to upstream at
`0bfc1c03451373f4cb34eebbfab4c5c257878fe0`. The target contained 16,415 files,
2,053 directories, and `18,381,103,174` logical bytes; it contained no reparse
point and no running process executable referenced it.

Both source-tree `syndocal.exe` links and the retained evidence executable were
confirmed to share File ID `0x0000000000000000003c00000034208f`, size
`57,888,768`, and SHA-256
`1B010C40242A5C7DD7A2797EAC1ECA2D31BCACE4455BA57C7F935611075B582B`.
The exact target directory alone was then permanently deleted through
PowerShell's `.NET Directory.Delete` API. It was absent afterward, while the
retained executable remained present with the same hash.

C: free space rose from `415,833,874,432` to `431,955,369,984` bytes, an
observed physical gain of `16,121,495,552` bytes (15.014 GiB). The deleted
target was rebuildable generated content; deletion is irreversible. The owning
source worktree, branch, commits, main checkout target, protected real-4K
evidence executable, running main-checkout Syndocal, and user-authored files
were not removed. This cleanup makes no additional product-acceptance claim.

## Boundary and next action

This closes the alpha.11 D4 merge/build/launch checkpoint only. It is not a
beta, RC, tag, release, or whole-product completion claim. DJ-Link v1.1.2
provenance/package work, live wired-LAN acceptance, ASIO persistence
integration, real-device ASIO completion, and the remaining physical/soak/
distribution rows stay open.

Next dependency-ordered action: finish and independently review the DJ peer,
create and push its exact `beta-v1.1.2` branch, then run the confirmed wired-LAN
acceptance. Continue the independently reviewed ASIO alpha.12 lane in parallel.
