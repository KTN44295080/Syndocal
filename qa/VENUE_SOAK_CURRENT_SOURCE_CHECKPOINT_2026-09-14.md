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

## Takeover rerun — 2026-09-14

The current branch was rechecked after takeover from `430ed02e`. The first
short-run attempt exposed two test-harness continuity defects before any venue
claim could be made:

- `samples/front-dimmer-wave.effect` is now a canonical `Mapping` preset, but
  `crates/engine/examples/syndocal_soak.rs` only admitted the legacy
  `position_wave` body.
- The shared demo sample intentionally fades its video automation to zero at
  the four-second cue boundary, which is outside this M5 placeholder
  workload's nonblank-frame contract.

The harness now admits both preset shapes and clamps only the software-soak
fixture's video-opacity floor to `0.25`; the authored sample and product
runtime are unchanged. The soak wrapper also refreshes the Windows process
handle and, only when the generated report is `passed=true`, tolerates a host
that releases the final exit-code property after report publication. A false
report or missing report still fails closed. The harness retains bounded blank
frame diagnostics for future failures.

Focused current-source evidence:

```text
rustfmt --check crates/engine/examples/syndocal_soak.rs: PASS
MSVC 14.44.35207 cargo build -p engine --example syndocal_soak --release --locked: PASS
MSVC 14.44.35207 cargo test -p engine mixed_color_chaser_move --release --locked -- --nocapture: PASS
  p95=2731us, p99=3236us, max=7100us for mixed 64x200 stack
  cue transition p95=7895us, p99=9358us, max=9374us
run-soak.ps1 -DurationSeconds 60 -MixedLighting -SkipBuild: PASS
```

The 60-second current-source report recorded `1801/1801` nonblank frames,
zero dropped frames, `1801` live-audio updates, tick p99 `510us`, command
queue p99 `92us`, command-to-DMX p99 `94us`, `3603` successful loopback DMX
sends, zero DMX failures, zero render errors, and a `26.4 MB` peak working
set. This is software-only loopback evidence; it is not the required one-hour
venue/GPU/integrated A/V/physical-output acceptance.

The product source outside the soak harness and wrapper remains unchanged by
this rerun. `VENUE-SOAK-001` therefore remains Open pending the named
reference-machine run and its retained time-series evidence.

## Takeover continuity — 2026-09-14

The venue-soak implementation and bounded harness evidence remain current at
HEAD `e93c6a47`: the venue-relevant source after the recorded `7daf00fe`
repair contains no product-runtime change that invalidates the 60-second
software result. This is continuity evidence only. The required named
reference machine, maximum-condition GPU/resource and thermal logs,
integrated A/V/lighting/output/recording path, and one-hour retained report
were not run in this takeover, so `VENUE-SOAK-001` remains `Open`.
