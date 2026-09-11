# Video Slot/FX software revalidation — 2026-09-12

This checkpoint advances only the deterministic software slices of
`COV-VIDEO-SLOT-001` and `COV-VIDEO-FX-001` on current `main` at
`e58c7426aca5d3519adb23a99741a7ede061cb59`. No product source was changed.

## Clip Slot / Follow evidence

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — focused B4 model/browser-contract source gate |
| `pnpm --dir app run check:timeline-follow-runtime` | PASS — stale E/G rejection, abort receipts, focus/target and cue visibility contracts |
| `pnpm --dir app run check:timeline-follow-hold-ui` | PASS — Immediate/Hold and one-measure Pedal-1 modes are explicit, normalized and persisted |
| `pnpm --dir app run check:video-runtime-polling` | PASS — deferred response, malformed/native-array omission, generation retention and zero-copy valid arrays |
| `pnpm --dir app run check:video-output-routing-runtime` | PASS — R4 routing contract |
| Engine `video_full_gate_engine_path` | PASS — 1 passed, 0 failed |
| Engine `video_sample_clip_take_queue` | PASS — 1 passed, 0 failed |
| Engine `video_sample_follow_admission` | PASS — 1 passed, 0 failed |

## FX / transition evidence

| Check | Result |
| --- | --- |
| `node app/scripts/check-authored-effect-enable.mjs` | PASS — strict authored enable controller and rendered route |
| `node app/scripts/check-effect-draft-helpers.mjs` | PASS |
| `node app/scripts/check-value-effect-generator.mjs` | PASS |
| `node app/scripts/check-cue-effect-recall.mjs` | PASS |
| `pnpm --dir app run check:fx-visual` | PASS — effect family, LFO, palette, Move/Value/Curve and mapping contracts |
| `pnpm --dir app run check:fx-palettes` | PASS — 32 built-in palettes, persistence and scene FX coverage |
| Engine `video_transition_bus_c3_is_typed` | PASS — 1 passed, 0 failed; typed/reversible/conflict-safe publication |

All Rust checks used the documented MSVC 14.44.35207 linker, confirmed first
by `where.exe link.exe`. The separate real-browser slot gate and browser cue
audio gate were not run because this PC has no Chrome/Edge executable at the
supported paths. These results are source/engine evidence only: native UI,
GPU/4K/multi-display, physical outputs, venue behavior, and full C2/C4
integration remain open. Both ledger rows remain `In progress`.
