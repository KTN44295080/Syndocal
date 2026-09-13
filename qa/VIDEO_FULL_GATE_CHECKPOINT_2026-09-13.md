# Video full-gate checkpoint — 2026-09-13

## Scope

This checkpoint closes `VIDEO-FULL-GATE-001` for the current-source Windows
software and rendered-browser boundary. It revalidates the accepted Clip Slot,
Layer/FX, and transition-bus tranches together without claiming the separate
C2/C4 integration row.

The browser contract was updated to follow the current composition: Edit Video
uses `EditVideoInspector` for layer FX, while Control Video uses the shared
Clip Slot bank and compact transition bus. The prior check stopped on a retired
`.videoMixerLayerPane` selector even though the current inspector was mounted;
the assertion now checks the live `data-edit-video-inspector` and
`videoIsfStackRow` surface.

## Verification

All Rust commands used the pinned MSVC 14.44.35207 x64 linker, confirmed first
by `where link.exe` after `vcvars64.bat -vcvars_ver=14.44`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — fixed 32-pad model, stable IDs, runtime generation, authority, and layout contracts |
| `pnpm.cmd --dir app run check:timeline-follow-runtime` | PASS |
| `pnpm.cmd --dir app run check:timeline-follow-hold-ui` | PASS |
| `pnpm.cmd --dir app run check:video-runtime-polling` | PASS |
| `pnpm.cmd --dir app run check:video-output-routing-runtime` | PASS |
| `node app/scripts/check-authored-effect-enable.mjs` | PASS |
| `node app/scripts/check-effect-draft-helpers.mjs` | PASS |
| `node app/scripts/check-value-effect-generator.mjs` | PASS |
| `node app/scripts/check-cue-effect-recall.mjs` | PASS |
| `pnpm.cmd --dir app run check:fx-visual` | PASS |
| `pnpm.cmd --dir app run check:fx-palettes` | PASS |
| `cargo test -p engine --release --locked -j 1 video_full_gate_engine_path -- --test-threads=1` | PASS — 1/1 |
| `cargo test -p engine --release --locked -j 1 video_sample_clip_take_queue -- --test-threads=1` | PASS — 1/1 |
| `cargo test -p engine --release --locked -j 1 video_sample_follow_admission -- --test-threads=1` | PASS — 1/1 |
| `cargo test -p engine --release --locked -j 1 video_transition_bus_c3_is_typed -- --test-threads=1` | PASS — 1/1 |
| `node app/scripts/check-edit-video-fx.mjs` | PASS — 1920×1080 and 1280×720; no page errors, 64 controls, outer overflow 0 |
| `node app/scripts/check-video-clip-slot-bank-browser.mjs` | PASS — 5 viewports; Edit/Control 32/32 pads, 6–8 columns, outer overflow 0, targets below 44px 0/0 |
| `git diff --check` | PASS |

`check-edit-video-fx.mjs` retained current rendered screenshots and result JSON
under `target/qa/edit-video-fx-20260905/`. The browser path used the installed
Chrome executable and the configured desktop Playwright runtime because the
Browser plugin was not available in this session. No browser dependency was
downloaded.

## Result

The accepted Windows Clip Slot/Layer/FX software surfaces now pass their
current-source static, Engine, and rendered-browser gates together. The
current UI composition is covered by the updated browser checker; no assertion
was weakened to accommodate the retired selector.

## Boundary

This checkpoint does not claim C2 Clip Take and C4 mapping/Timeline transition
integration, native renderer/GPU/display behavior, 4K or three-output
presentation, real show media, physical output, external devices, venue
operation, Mac execution, signing, publication, or product completion. Those
remain separate completion markers or external acceptance gates.
