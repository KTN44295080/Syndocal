# Timeline/Musical software revalidation — 2026-09-12

This checkpoint advances only the deterministic software slice of
`COV-TIMELINE-MUSICAL-001` on `main` at
`5804258c71b75d8ee0f3fd9a00aef634f81fd73b`. No product source was changed.

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:timeline-advanced` | PASS — snapshot fencing, runtime-free loop protection, control semantics and five-domain lane planning |
| `node app/scripts/check-timeline-navigator-actions.mjs` | PASS — root/child selection, draft cancellation, stale/rejected commit and concurrency exclusion |
| `pnpm --dir app run check:timeline-automation` | PASS |
| `node app/scripts/check-timeline-overlap-clusters.mjs` | PASS — helper performance reported 16.0 ms / 11.0 ms rails |
| `node app/scripts/check-timeline-block-loop.mjs` | PASS — presentation contract |
| `pnpm --dir app run check:timeline-follow-runtime` | PASS — stale/fault/abort, focus/target and cue visibility contracts |
| `pnpm --dir app run check:timeline-follow-hold-ui` | PASS — explicit normalized persisted modes |
| `pnpm --dir app run check:timeline-cue-audio` | PASS — fences, single-flight/latest mutation and canonical invokes |
| `pnpm --dir app run check:timeline-loop-runtime` | PASS — authority, receipt, stale/reply-loss and convergence |
| `pnpm --dir app run check:timeline-transport-runtime` | PASS — route, snapshot convergence, retry/latest intent and fail-closed pair |
| Engine `timeline_fractional_ticks` | PASS — 6 passed, 0 failed, 0 ignored |

The separate real-browser cue-audio gate was not runnable because this PC has
no Chrome/Edge executable at the supported checker paths. This evidence does
not establish audible Guide/click output, native A/V/Lighting synchronization,
real device restart or two-song venue acceptance. `COV-TIMELINE-MUSICAL-001`
remains `In progress`.

## Current-main rerun — 2026-09-12

The Timeline/Musical software gate was rerun against current `main` at
`e7b3d2c8f4c9d9a54f29990d167426ecb3393ed9`, with the documented MSVC
14.44.35207 x64 absolute linker pin confirmed first by `where.exe link.exe`.
All eleven checks passed, including the six `timeline_fractional_ticks` tests.
The overlap helper reported 16.6 ms / 10.4 ms rails in this run. No browser,
audible device, native A/V/lighting synchronization, external clock, or venue
acceptance was performed.
