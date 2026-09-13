# Venue Soak Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `VENUE-SOAK-001` (section 8, Open)
- Q1 row: `COV-PERFORMANCE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `72494d5476cd3b61ae5e0468388379c548accf82`
- Authority: `qa/M5_RELIABILITY_VALIDATION.md` and the performance benchmark
  records

This checkpoint records the current frontend build and status-model proof only.
It does not claim a maximum-condition venue run.

## Verification

```text
pnpm.cmd --dir app run check:status
pnpm.cmd --dir app run build
```

Result: exit code 0.

- Status model helpers: PASS.
- Frontend production build: PASS; 358 modules transformed and built.
- The existing Vite large-chunk message remains an advisory; no first-party
  compiler warning was observed in this focused run.

## Unresolved acceptance

`VENUE-SOAK-001` stays Open. The required maximum-condition venue GPU test must
run the integrated video, lighting, output, recording, Preview/Program, ISF,
resource, and one-hour paths with time-series CPU/GPU/memory/frame/tick logs.
The build and status checks cannot establish thermal, resource, clock, physical
output, or venue behavior.

Next action is the named venue/reference machine one-hour run with exact source
artifact, scene, device topology, resource logs, and first-failure retention.
