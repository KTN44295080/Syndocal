# Authored audio software revalidation — 2026-09-12

This checkpoint advances only the deterministic software slice of
`COV-AUDIO-AUTHORED-001` on current `main` at
`2dd51afc291e173ee97773989fe93dbb130dcaca`. No product source was changed.

## Evidence

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:timeline-cue-audio` | PASS — fences, single-flight/latest mutation, duplicate endpoint rejection, canonical invokes |
| `pnpm --dir app run check:timeline-audio-output-bus` | PASS — 11 assertions for explicit bus UI and fail-closed routing |
| `pnpm --dir app run check:timeline-loop-runtime` | PASS — strict authority, exact receipt, reply-loss/stale policy, convergence and legacy clean-break |
| `pnpm --dir app run check:timeline-transport-runtime` | PASS — strict route, snapshot convergence, retry/latest intent and fail-closed pair |
| `cargo test ... timeline_audio` | PASS — 34 passed, 0 failed, 0 ignored, 1832 filtered; MSVC 14.44.35207 linker pinned and first in `where.exe link.exe` |
| `cargo test -p engine ... timeline_fractional_ticks` | PASS — 6 passed, 0 failed, 0 ignored, 1077 filtered; same linker pin |

The attempted `pnpm --dir app run check:timeline-cue-audio:browser` did not
start: this PC has no Chrome or Edge executable at the checker's supported
paths. It is recorded as an environment prerequisite failure, not a product
pass or product failure. No browser result is claimed.

These checks cover the current authored audio/runtime software boundary,
including schema round-trip, seek/loop, fractional timing, resync, stale
source, worker timeout/cancellation, device-generation rejection, bus
replacement, and silent/fail-closed fault behavior. They do not freeze the
open `DEC-CLOCK-MASTER-001`, prove migration corpus compatibility, launch a
native UI, exercise a real audio device, or establish long-duration/venue
acceptance. `COV-AUDIO-AUTHORED-001` therefore remains `In progress`.

## Current-main rerun — 2026-09-12

The authored-audio software checks were rerun against current `main` at
`19e04420100cae89e536be92554efe061a65307e`. All six focused checks passed:

- `pnpm.cmd --dir app run check:timeline-cue-audio` — PASS;
- `pnpm.cmd --dir app run check:timeline-audio-output-bus` — PASS, 11 assertions;
- `pnpm.cmd --dir app run check:timeline-loop-runtime` — PASS;
- `pnpm.cmd --dir app run check:timeline-transport-runtime` — PASS;
- Tauri `timeline_audio` — 34 passed, 0 failed, 0 ignored;
- engine `timeline_fractional_ticks` — 6 passed, 0 failed, 0 ignored.

Both Rust invocations used the documented MSVC 14.44.35207 x64 absolute
linker pin, confirmed first by `where.exe link.exe`. No browser, native UI,
audio device, physical output, or venue/long-duration acceptance is claimed.
