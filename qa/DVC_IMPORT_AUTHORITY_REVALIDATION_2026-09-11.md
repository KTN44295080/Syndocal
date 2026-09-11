# DVC import authority revalidation — 2026-09-11

## Scope

This checkpoint repairs only the existing Daslight Project (`.dvc`) import
controller's failure response. If a project replacement changes the captured
project authority while import is pending, the older import error is now
discarded instead of overwriting the newer project's operator message. The
busy flag is still cleared in `finally`, and the current-authority success and
cancel paths are unchanged.

No DVC file format, import API, native command, physical I/O, output arming,
fixed three-screen layout, or performance hot path was changed.

## Evidence

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:dvc-import-controller` | PASS — 8 cases, including current success, cancel, failure cleanup, stale success, and stale failure suppression |
| `pnpm.cmd --dir app exec tsc --noEmit` | PASS |
| `pnpm.cmd --dir app run check:release` | PASS — full static release gate; 50 Open + 8 Deferred preserved |
| `git diff --check` | PASS |

The Windows no-bundle build used the repository-required MSVC procedure;
`14.44.35207` was first in `where.exe link.exe`.

The resulting executable was
`target/release/syndocal.exe`, version `1.2.0-alpha.69`, 64,744,448 bytes,
SHA-256
`AA770E13226E2DE2DB2BB86AD6C25C34356E14D74C4C635F381D8212F91CF648`.

The fresh native probe report is
`target/qa/native-final-validation-20260911-65/native-final-validation.json`.
It passed with exactly one responsive maximized `Syndocal` window, Standby
ownership with lighting and video disabled, snapshot state retrieval,
expected missing-media and missing-layer thumbnail IPC failures, zero
physical-output operations, exact application exit, and zero remaining debug
listener.

The previously accepted real-file thumbnail missing → Retry → recovery trial
was not rerun. This checkpoint does not claim physical DVC/MIDI/DMX/OSC device
acceptance, ASIO/NDI acceptance, Mac/Linux acceptance, signing, publication,
or product-wide completion.
