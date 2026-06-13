# Rayard Claude Code Handoff

This workspace is a large, intentionally dirty prototype branch for Rayard, a Rust/Tauri/SolidJS DMX lighting + VJ control app.

## Product Decisions To Preserve

- Product name: Rayard.
- Developer / publisher: Seraf()のKTN.
- Project files use `.ry`. Keep legacy project-name and extension aliases out of user-facing files.
- The user prefers lightweight checks for small changes. Use full Tauri builds only at major milestones.
- 3D visualization is deferred/external for now. Keep the app focused on the Setup 2D mapping visualizer unless the user asks otherwise.
- The Setup UI should keep a DasLight-like operational feel: dense, dark, tabbed, grid-based, and tool-oriented.
- README updates can lag implementation unless the user asks for docs. This file is the current handoff artifact.

## Current Workspace State

The worktree is expected to be dirty. Do not revert broad changes just because they are uncommitted.

Current handoff checkpoint, 2026-06-14:

- Progress estimate: about 98.5%. This is a strong prototype/handoff checkpoint, not final product completion.
- Full workspace tests passed: `cargo test --workspace`.
- Frontend production build passed: `pnpm --dir app build`.
- Tauri debug build passed: `pnpm --dir app tauri build --debug`.
- Debug executable was produced at `target/debug/rayard.exe`.
- Current diff is intentionally broad: roughly 20 modified tracked files plus new `app/src/components/VideoOutputWindow.tsx` and `app/src/videoFrameCanvas.ts`.

`app/src/App.tsx` was previously reverted after a bad PowerShell overwrite. The exact 24,429-line dirty version could not be recovered from editor history, but the current file has been rebuilt to a working recovery point: Rayard branding is restored, Setup/Control/Touch still render, Setup Mapping has a broad 2D map workspace, and `pnpm --dir app build` passes. Treat this as the handoff baseline rather than trying to resurrect the lost dirty file with git checkout/reset.

Important high-level changes already present:

- Tauri/Solid app is branded as Rayard in `app/package.json`, `app/index.html`, and `app/src-tauri/tauri.conf.json`.
- `.ry` project association is configured in `app/src-tauri/tauri.conf.json`.
- Setup has `Setup`, `Control`, and `Touch`-style workspace flows in `app/src/App.tsx`.
- Setup Mapping has a 2D mapping view with group/type strips, stage grid, fixture placement, projector/output surfaces, and selection controls.
- Internal SVG/isometric 3D UI has been removed from `app/src/App.tsx`; keep 3D work outside the main UI unless the user asks to bring it back.
- Engine has GDTF/custom profile patching, 44 Hz DMX tick, Art-Net/sACN/serial route support, cues, timeline, effects, video state, telemetry, and project snapshot support.
- Video is currently a CPU/reference preview and state model path with still-image decode, FFmpeg CLI file-frame extraction, bounded frame queues, decode scheduling diagnostics, and Display output preview windows. Native wgpu video output, in-process FFmpeg/HAP workers, NDI, Spout, and Syphon transports are future-phase work.

## Useful File Map

- `app/src/App.tsx`: main SolidJS UI. It is very large and is the source of the current Vite chunk warning.
- `app/src/styles.css`: main UI styling.
- `app/src/components/`: extracted SolidJS UI panels/editors, including `DmxOutputConfigPanel.tsx`, `DmxRawMonitor.tsx`, `DmxTestFramePanel.tsx`, `DmxRoutesPanel.tsx`, `EngineTelemetryPanel.tsx`, and `VideoOutputWindow.tsx` for Phase 1 DMX/output diagnostics and dedicated display-output windows.
- `app/src/customFixtureProfile.ts`: custom fixture profile helpers.
- `app/src/uiModes.ts`: workspace/setup/control tab mode definitions.
- `app/src/videoLayerDefaults.ts`: default video layer state fragments.
- `app/src/videoFrameCanvas.ts`: shared RGBA preview-frame validation, canvas drawing, and data-URL conversion helpers.
- `app/src/videoOutputMapping.ts`: projector/output mapping helpers.
- `app/src-tauri/src/main.rs`: Tauri command boundary, project files, validation, telemetry report, GDTF Share, video preview commands.
- `crates/engine/src/lib.rs`: realtime engine, DMX render loop, cues, timeline, effects, telemetry.
- `crates/gdtf/src/lib.rs`: GDTF ZIP/XML parser and media/model loading helpers.
- `crates/io/src/`: Art-Net, sACN, serial DMX, MIDI, OSC, remote WebSocket.
- `crates/video/src/lib.rs`: video state/runtime/reference compositor/preview frame boundaries.
- `crates/visualizer/src/lib.rs`: renderer-facing scene conversion; keep it as a data boundary while 3D UI is deferred.

## Known Verification Commands

Use targeted checks while iterating:

```powershell
cargo test --workspace
cargo test -p rayard phase1_smoke_project_sample_loads_into_engine_and_renders_cue
cargo test -p rayard phase1_smoke_project_sample_sends_cue_to_artnet_loopback
cargo test -p rayard phase1_smoke_project_sample_is_valid
cargo test -p rayard project_
cargo test -p rayard sample_effect_presets_are_valid_for_phase1_mini_show
cargo test -p rayard gdtf_profile_patch_drives_engine_dmx_preview
cargo test -p rayard engine_telemetry_budget_report
cargo test -p engine engine_sends_rendered_fixture_state_to_artnet_loopback
cargo test -p engine bpm_clock
cargo test -p gdtf
pnpm --dir app build
pnpm --dir app tauri build --debug
```

Notes:

- `pnpm --dir app build` currently succeeds but warns that the main `index-*.js` chunk is slightly above 500 kB because `App.tsx` is still huge.
- `pnpm --dir app tauri build --debug` currently builds `target/debug/rayard.exe`. Full release packaging should still be saved for major release checkpoints:

```powershell
pnpm --dir app tauri build
```

## Recent Verified Points

- GDTF parse -> patch -> Engine DMX preview is covered by `gdtf_profile_patch_drives_engine_dmx_preview` in `app/src-tauri/src/main.rs`.
- Engine rendered fixture state -> Art-Net UDP loopback is covered by `engine_sends_rendered_fixture_state_to_artnet_loopback` in `crates/engine/src/lib.rs`.
- Telemetry budget report pass/fail/warn behavior is covered by `engine_telemetry_budget_report*` tests in `app/src-tauri/src/main.rs`.
- BPM clock tap/MIDI/external sync is covered by `cargo test -p engine bpm_clock`.
- Project snapshot loading restores self-contained fixture controls from `snapshot://fixture/{id}` and does not depend on the original external profile path.
- The app topbar has `Load Sample`, backed by the embedded `samples/phase1-mini-show.ry`, for quick Phase 1 manual smoke checks without a file dialog.
- The app topbar also has `Run Smoke`: it loads the embedded mini show, triggers the first cue, switches to Control, selects DMX Raw U0, and reports non-zero values in A1-A8.
- The Output panel telemetry section now exposes Reset, Save Report, and live budget status. It is backed by `reset_engine_telemetry`, `save_engine_telemetry_report`, and `get_engine_telemetry_report`.
- Latest major checkpoint in this handoff, 2026-06-14: `cargo test --workspace`, `pnpm --dir app build`, and `pnpm --dir app tauri build --debug` passed on Windows.
- Position Wave effect editing has quick presets for origin from stage center or selected fixture, X/Z/radial direction buttons, and a mini stage map for setting origin/direction against current 2D fixture and projector positions. Radial uses the engine's zero-direction distance fallback.
- Control now includes a minimal Node Graph builder that reuses the current LFO/Position Wave source and selected lighting/video target, with enable/disable/remove and `.nodegraph` preset save/load commands.
- The video CPU reference compositor accepts RGBA, BGRA, DXT1, and DXT5 frames; `cargo test -p video` passes after the DXT/BGRA path was added.
- Display output windows now call the CPU preview path with a one-frame decode warmup budget per live frame, while manual debug previews keep the fuller prefetch budget.
- Project validation allows named NDI/Spout/Syphon placeholder routes to load even when the native backend is not linked; backend availability stays in Video Control I/O plans and Sync Routes diagnostics.
- Video Control I/O plan rows now label unavailable external routes as `Blocked`, while Sync Routes reports `Started`, `Kept`, `Stopped`, `Idle`, `Blocked`, or driver failure rows.
- Latest lightweight checkpoint after the output-window decode/refactor and external-I/O validation changes: `cargo test -p rayard video_preview_decode_budget`, `cargo test -p rayard video_preview`, `cargo test -p rayard video_output_decode_preview_summaries_report_schedulable_requests`, `cargo test -p rayard external_video`, `cargo test -p rayard project_file_keeps_external_video_routes_as_runtime_diagnostics`, `pnpm --dir app build`, and focused `git diff --check` over touched Rust/frontend/doc files passed.
- Browser verification at 1280px confirmed Setup, Control, and Touch no longer create horizontal page overflow after the Output/DMX route panel CSS fix.

## Local Smoke Asset

- `samples/phase1-mini-spot.fixture` is a small custom fixture profile for manual Phase 1 checks.
- `samples/phase1-mini-show.ry` is a self-contained project sample with one patched fixture, one cue, Art-Net U0 output, and one 16:9 mapped display output.
- `samples/README.md` documents the manual smoke flow and expected channel footprint.
- Use the topbar `Load Sample` button to load the embedded mini show directly, use `Run Smoke` to load and trigger the first cue in one step, or load `samples/phase1-mini-spot.fixture` from Setup -> Profiles to build the same smoke patch manually at `U0 A1`.
- Expected footprint:
  - Ch 1: `Dimmer` 8-bit
  - Ch 2-4: `ColorRed`, `ColorGreen`, `ColorBlue` 8-bit
  - Ch 5-6: `Pan` 16-bit
  - Ch 7-8: `Tilt` 16-bit
- Good smoke path: patch one fixture, set Dimmer to full, set RGB, move Pan/Tilt, confirm DMX Raw updates on U0, then send an Art-Net test/loopback frame if a receiver is available.
- The project sample also has an Engine smoke test that loads the `.ry`, triggers its cue, and verifies rendered DMX preview values.

## Current Rough Edges

- `App.tsx` is over 1 MB. Do not grow it casually. Prefer extracting cohesive UI/helper code to `app/src/components/` or focused helper modules.
- Vite chunk warning remains because much logic still lives in `App.tsx`. A good next frontend refactor is to extract Setup Mapping and Output panels into separate components.
- Browser visual verification has not been reliable in the current Codex app environment. Use `pnpm --dir app build` as the lightweight frontend gate unless a browser is available.
- README has been adjusted to match the latest product decision: 3D visualization is deferred/external, and the 2D mapping visualizer is the immediate focus.
- The repo contains many broad changes. Before large refactors, inspect the current code paths and avoid undoing unrelated edits.

## Suggested Next Work

1. Stabilize Phase 1 handoff:
   - Run the targeted tests listed above after touching related code.
   - Run `pnpm --dir app build` after frontend changes.
   - Run a full workspace/Tauri build only at a milestone. Last milestone passed on 2026-06-14.

2. Frontend cleanup:
   - Extract the Setup 2D Mapping view from `App.tsx`.
   - Extract Output/Telemetry diagnostics from `App.tsx`.
   - Keep DasLight-like dense controls and dark grid mapping style.

3. Phase 1 operational smoke:
   - Create or document a small local smoke workflow: custom fixture profile, patch to U0 A1, set Dimmer, verify DMX Raw, send Art-Net loopback/test frame.
   - Keep `samples/phase1-mini-spot.fixture` and `samples/phase1-mini-show.ry` valid as the quick regression assets when project-file fields change.

4. Phase 2 direction:
   - Expand effect UI around LFO and position-wave presets.
   - Keep shared effect sources capable of targeting both lighting and video.
   - Keep 2D mapping as the stage coordinate source for position-wave fixtures and projector surfaces.

5. Later VJ direction:
   - Replace CPU preview output windows with native wgpu output surfaces.
   - Add real in-process FFmpeg/HAP decode scheduling.
   - Add NDI, Spout, and Syphon transport bindings behind platform-specific modules.

## Handoff Rule Of Thumb

Move in small verified slices. For this repo, a useful slice usually touches one boundary and one test:

- Engine behavior -> `crates/engine/src/lib.rs` plus a focused `cargo test -p engine ...`.
- Tauri command/validation/project files -> `app/src-tauri/src/main.rs` plus `cargo test -p rayard ...`.
- Frontend UI -> extracted component/helper plus `pnpm --dir app build`.
- GDTF parsing -> `crates/gdtf/src/lib.rs` plus `cargo test -p gdtf`.
