# Video full-gate software audit — 2026-09-08

## Scope

This is a bounded audit of the existing Windows video/Clip Slot/transition and
Timeline Follow software contracts. It does not add a video feature, reopen an
old candidate branch, or claim native renderer/GPU/display or physical output
acceptance.

- Source base: `e9db716504eb8d86cae9ef31c5160686903d36a0`
- Working tree before this document: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing implementation checked

The current Engine path already publishes and rolls back the complete video
state for the bounded full-gate scenario. The C3 transition bus is typed,
reversible, conflict-safe, and published. Clip Take queue sampling stays
separate from root interpolation, and Follow admission/jump termination stops
before its boundary. Existing frontend contracts retain the accepted Clip Slot
model and explicit Follow hold modes.

## Focused verification

The Rust commands ran after `vcvars64.bat -vcvars_ver=14.44`, with the exact
Build Tools MSVC `14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` and returned first by
`where.exe link.exe`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — B4 focused model/browser contract |
| `node app/scripts/check-timeline-follow-runtime.mjs` | PASS — stale E/G, E/R/H abort receipt, visibility/focus/target contracts |
| `node app/scripts/check-timeline-follow-hold-ui.mjs` | PASS — immediate/hold and one-measure wait-for-Pedal-1 static model |
| `cargo test -p engine --release --locked video_full_gate_engine_path -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked video_transition_bus_c3_is_typed -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked video_sample_clip_take_queue -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked video_sample_follow_admission -- --test-threads=1` | PASS — 1 passed |

`node app/scripts/check-edit-video-fx.mjs` was attempted but could not start
because this PC has no Playwright Chromium executable at the configured local
cache path. This is an environment prerequisite failure, not a product pass;
the browser gate remains open and no browser dependency was downloaded or
assertion weakened. No app process, physical output, device, or external client
was started by this audit.

## Remaining boundary

`VIDEO-FULL-GATE-001`, `VIDEO-C2-C4-001`, and `TIMELINE-FOLLOW-001` remain
`Open` in `qa/SYNDOCAL_COMPLETION_LEDGER.json`. This checkpoint does not prove
the complete C2/C4 reintegration, the unavailable browser/rendered UI gate,
native renderer/GPU/display behavior, 4K/three-output presentation, real show
media, or physical output. Timeline persistence, live sources, ASIO/NDI/DMX
hardware, Mac real-device, signing, publication, and product-wide acceptance
remain unclaimed.

No assertion was weakened and no runtime or physical-output behavior changed.
