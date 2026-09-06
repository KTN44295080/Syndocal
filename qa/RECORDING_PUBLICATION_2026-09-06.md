# Recording destination concurrency and recovery

Base `8213f4a8072a814cd72d223bb6b8ec3f727fda6e`, branch
`codex/syndocal-v1.2`, internal product `1.2.0-alpha.69`.

## Changed Windows contract

The former final identity/size/mtime comparison followed by a pathname-based
replace left a race in which another ordinary writer could lose its file.
Windows publication now locks the verified staging file and previous destination
with read/write/delete access and read sharing only. Renames act on those
handles and never replace an existing destination. A writer already holding
incompatible access causes a specific rejection before destination mutation.

Existing destinations use two renames: move the checked old file to a unique
backup, then install the checked new recording without overwrite. A late creator
wins its destination slot; neither publication nor rollback overwrites it. In
that case the old backup, completed partial and recovery intent remain, with
their paths in the error. New destinations use one no-clobber handle rename.

This is **recoverable two-phase publication, not atomic replacement**. The target
can be absent between renames. Before that gap, a per-target recovery intent is
created exclusively, fully written and synced. Its handle remains locked, so a
second live reservation cannot treat an in-progress publication as a crash.
Ordinary artifact Drop relinquishes cleanup to recovery as soon as the journal
is created, including panic/error paths. No recovery path deletes the retained
new partial when restoring the previous file.

On the next reservation of that target, recovery examines actual locked file
identities, lengths and modification times, without trusting an advisory phase:

| Target / backup | Action |
| --- | --- |
| Previous / absent | Keep old target and new partial; retire intent |
| Absent / previous | Restore old file without replacing anything; retain new partial |
| Replacement / previous | Remove exactly the old backup, then the intent |
| Replacement / absent | Remove only the intent |
| Changed, missing required file, linked/reparse, invalid record | Retain files and report specific failure |

Restoration reports that the previous file and completed partial were preserved
before allowing a retry. A version-1 intent is bounded to 64KiB, rejects unknown
top-level and target-state fields, requires the expected target and same-parent
reserved staging/backup names, and retains its file lock through recovery.

The platform adapter uses the generated windows 0.61.3 ABI. Primary contracts:
[SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)
and [FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info).

## Scope and architecture

Recording capture and worker ownership are unchanged. Publication/recovery is a
separate module with a small Windows file-handle adapter; no new per-frame work,
pixel copy, worker, IPC or project schema change is introduced. The recovery
record version is independent of product version and `.sdc` format.

This Windows tranche does not claim power-loss durability on all filesystems,
arbitrary directory replacement protection, memory-mapped tamper detection,
or cryptographic content identity. The historical Unix existing-target path
remains unchanged and is outside the current Windows product acceptance.
Full crash inventory/import UI, renderer/pipe cancellation and long recording
clock acceptance remain separate recording requirements.

## Evidence

`target/qa/recording-publication-20260906/recording-tests-acceptance.log`:
48 passed, 4 ignored, 1724 filtered, 1.94s, with the exact pinned Community
MSVC 14.44.35207 linker. Tests include competing destination creation after the
old-file move, locked source/backup mutation rejection, the final checked-handle
comparison, retained modified files, malformed/future/oversized/escaping intents,
all recovery states, and four real subprocess exits without Rust unwinding.
These exits establish process-interruption recovery, not sudden power loss.

`ffmpeg-smoke.log`: explicitly selected real A/V MP4 test passed, 1 passed,
1775 filtered, 0.31s. It preserves the old target until publication, uses the
production encoder/publication path and decodes all 30 video frames.
The ignored subprocess helpers are invoked by parent tests; the long A/V test
is not counted as executed. Final test compiler warnings are zero.

Independent review accepted the final handle ownership/recovery design and
diff. Its nested unknown-field finding was fixed with strict deserialization
and a negative case. The Windows native build passed with zero first-party
warnings (previous accepted build 0, delta 0). The executable launched as one
responsive maximized Syndocal main window. Shared artifact identity and startup
evidence are recorded in [the group rotation checkpoint](MAPPING_GROUP_ROTATION_2026-09-06.md).
This does not add unobserved native recording UI or power-loss acceptance.
