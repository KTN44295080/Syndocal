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

- Handoff checkpoint: 100%. This is a strong prototype handoff point, not final product completion.
- Product completion is not 100% yet: native wgpu video output, in-process FFmpeg/HAP workers, hardware-timed Open DMX refinement, and real NDI/Spout/Syphon bindings remain future work.
- Full workspace tests passed: `cargo test --workspace`.
- Frontend production build passed: `pnpm --dir app build`.
- Tauri debug build passed: `pnpm --dir app tauri build --debug`.
- Tauri release build passed: `pnpm --dir app tauri build`.
- Debug executable was produced at `target/debug/rayard.exe`.
- Release executable was produced at `target/release/rayard.exe`.
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
- `app/src/numericHelpers.ts`: pure numeric/DMX-value/color/geometry-matrix helpers extracted from `App.tsx` (clamp/percent, hsv<->rgb, geometry matrix math). No SolidJS/state deps; safe to grow with other pure helpers.
- `app/src/channelFunctionHelpers.ts`: pure GDTF channel-function helpers extracted from `App.tsx` (sorted/picked/ranked function value lookups, label/range/detail formatting, plus color/gobo wheel predicates `isColorWheelFunction`/`isGoboWheelFunction`/`goboPatternForFunction`). Depends on `AttributeControl`, `numericHelpers`, and the `GoboSlotPattern` type from `components/WheelSlotPanel`.
- `app/src/svgExportHelpers.ts`: SVG/stage-map export DOM helpers extracted from `App.tsx` (`svgExportComputedStyleProperties`, `standaloneSvgExportSelectorsToRemove`, `inlineComputedSvgStyles`, `downloadTextFile`, `safeExportFileNamePart`). No app-state deps.
- `app/src/favoritesStorage.ts`: localStorage-backed color and position favorites extracted from `App.tsx` (`loadColorFavorites`/`saveColorFavorites`, `defaultPositionFavorites`, `positionFavoriteFromUnknown`, `loadPositionFavorites`/`savePositionFavorites`). Reuses the `PositionFavorite` type already exported by `components/PositionControlPanel` (App.tsx's duplicate local interface was removed). Storage keys are module-private.
- `app/src/controlCategory.ts`: per-category "quick look" presets + value resolution extracted from `App.tsx` — `CategoryQuickLook` type, `quickLooksForCategory`, `opticsRoleForControl`, `quickLookValueForControl`. The `ControlCategory` enum / `controlCategories` table / `controlCategoryForAttribute` classifier are NOT here — they already live in `uiModes.ts` (shared with components); this module imports `ControlCategory` from there.
- `app/src/stageObjects.ts`: stage decoration object helpers extracted from `App.tsx` (`stageObjectKinds`, `stageObjectDefaultColor`, `stageObjectClass`). Depends only on `StageObjectKind`.
- `app/src/uiModes.ts` (pre-existing): single source of truth for `WorkspaceTab`, `SetupSubTab`, `ControlMode`, `ControlCategory` types and the `setupSubTabs`/`controlModes`/`controlCategories` tables + `controlCategoryForAttribute` and tab/mode shortcut helpers. App.tsx and components both import from here (App.tsx's old local duplicates were removed in slice 19).
- `app/src/videoOutputMapping.ts` (pre-existing): projector/output mapping helpers + `defaultVideoOutputMapping`, `NumericVideoOutputMappingField`, `outputAspectRatio`, and (added slice 18) `MappingVideoOutputCornerKey`/`mappingVideoOutputCorners`/`mappingVideoOutputCornerGain`. This is the single source of truth for video-output mapping; `controlMappingActions.ts` re-exports `NumericVideoOutputMappingField` from here.
- `app/src/projectSnapshot.ts`: `projectComparableSnapshot`/`projectSnapshotSignature` extracted from `App.tsx` — strip volatile runtime fields (active cue/fade, timeline/video playback position, clock phase, dmx preview, telemetry) so two `EngineSnapshot`s can be compared for real project changes.
- `app/src/wheelMedia.ts`: GDTF wheel-slot media helpers extracted from `App.tsx` (`WheelMediaBytes`/`WheelMediaPayload` types, `wheelSlotMediaPath`, `wheelMediaCacheKey`, `canLoadWheelMedia`, `wheelMediaPayloadToObjectUrl`). No app-state deps.
- `app/src/hotkeyHelpers.ts`: keyboard-shortcut helpers extracted from `App.tsx` (`isEditableShortcutTarget`, `controlCueHotkeyIndex`, `mappingStageToolFromHotkey`). Depends only on the `MappingStageTool` type.
- `app/src/fixtureLimits.ts`: fixture pan/tilt/dimmer limit + axis-mapping helpers extracted from `App.tsx` (`defaultFixtureLimits` const plus `normalizeLimitRange`, `applyAxisLimit`, `sourceValueForAxisLimit`, `dimmerValueWithinLimits`, `effectivePanTiltValues`, `sourcePanTiltValues`). Depends on `numericHelpers` + `FixtureLimits`/`PatchedFixtureSummary` types.
- `app/src/controlMappingActions.ts`: MIDI/OSC control-mapping action sets, the video-output mapping field catalog (`videoOutputMappingFieldOptions`), and small domain types/predicates extracted from `App.tsx` (`NumericVideoOutputMappingField`, `MappingFixtureFlag`, `FixtureFlagClearKind`, `fixtureFlagClearKinds`, the `is*MappingAction` predicates, `normalizeFixtureFlagClearKind`, `videoOutputMappingFieldOption`). Depends only on `./types`.
- `app/src/videoHelpers.ts`: pure video-source label + media path/time formatting helpers extracted from `App.tsx` (`videoSourceInputLabel`, `videoSourceInputPlaceholder`, `videoSourceCanBrowseFile`, `videoSourceKindLabel`, `mediaLabelFromPath`, `shouldReplaceVideoLayerDraftLabel`, `formatDuration`, `formatVideoTime`, `videoSourceMetadataLabel`). Depends only on `VideoSourceKind` from `./types`. (`beamPoints`, the fixture beam-triangle geometry that sat next to these, was moved into `stageGeometry.ts` instead.)
- `app/src/fixtureVisuals.ts`: fixture visual-kind classification + mapping-stage label/size helpers extracted from `App.tsx` — `MappingFixtureVisualKind` type, `fixtureVisualKind`, `mappingFixtureStageSize`, `fixtureTypeKey`, `fixtureTypeLabel`, `mappingTypeGlyphClass`. Depends only on `PatchedFixtureSummary` from `./types`.
- `app/src/editorDrafts.ts`: editor draft models and pure "from summary" builders extracted from `App.tsx` — `VideoOutputConfigDraft`, `CueMetadataDraft`, `TimelineEventDraft`, `TimelineAutomationDraft`, `TimelineVideoAutomationDraft` interfaces plus `videoOutputConfigDraftFromSummary`, `cueMetadataDraftFromSummary`, `timelineEventDraftFromSummary`, `timelineAutomationDraftFromSummary`, `timelineVideoAutomationDraftFromSummary`. Depends only on `./types`.
- `app/src/mappingViewPresets.ts`: Setup-mapping stage-tool + saved view-preset model and localStorage helpers extracted from `App.tsx` — `MappingStageTool` type, `MappingViewPreset` interface, `normalizeMappingStageTool`, `defaultMappingViewPresets`, `mappingViewPresetFromUnknown`, `loadMappingViewPresets`, `saveMappingViewPresets`. App.tsx imports the two types plus load/save (the default/normalize/from-unknown helpers are only used internally by load now). `mappingStageTools` array and storage key are module-private.
- `app/src/stageGeometry.ts`: stage 2D coordinate helpers extracted from `App.tsx` — `StageWorldBounds`, `stageViewBoxSize`, `stagePadding`, and `stageWorldToSvgPoint`/`svgPointToStageWorld` conversions. (`CueCapturePreviewPanel.tsx` and `StageMap2D.tsx` still keep their own local `StageWorldBounds`; unifying those onto this module is a possible future cleanup.)
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

- `pnpm --dir app build` currently succeeds with no chunk warning. `vite.config.ts` `manualChunks` already splits `control-panels` (~40 kB) and `mapping-editors` (~17 kB) out of the main bundle, so the main `index-*.js` chunk is ~419 kB, under the 500 kB warning threshold. The earlier "chunk warning" note is stale.
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
- Latest major checkpoint in this handoff, 2026-06-14: `cargo test --workspace`, `pnpm --dir app build`, `pnpm --dir app tauri build --debug`, and `pnpm --dir app tauri build` passed on Windows.
- Position Wave effect editing has quick presets for origin from stage center or selected fixture, X/Z/radial direction buttons, and a mini stage map for setting origin/direction against current 2D fixture and projector positions. Radial uses the engine's zero-direction distance fallback.
- Control now includes a minimal Node Graph builder that reuses the current LFO/Position Wave source and selected lighting/video target, with enable/disable/remove and `.nodegraph` preset save/load commands.
- The video CPU reference compositor accepts RGBA, BGRA, DXT1, and DXT5 frames; `cargo test -p video` passes after the DXT/BGRA path was added.
- Display output windows now call the CPU preview path with a one-frame decode warmup budget per live frame, while manual debug previews keep the fuller prefetch budget.
- Project validation allows named NDI/Spout/Syphon placeholder routes to load even when the native backend is not linked; backend availability stays in Video Control I/O plans and Sync Routes diagnostics.
- Video Control I/O plan rows now label unavailable external routes as `Blocked`, while Sync Routes reports `Started`, `Kept`, `Stopped`, `Idle`, `Blocked`, or driver failure rows.
- Latest lightweight checkpoint after the output-window decode/refactor and external-I/O validation changes: `cargo test -p rayard video_preview_decode_budget`, `cargo test -p rayard video_preview`, `cargo test -p rayard video_output_decode_preview_summaries_report_schedulable_requests`, `cargo test -p rayard external_video`, `cargo test -p rayard project_file_keeps_external_video_routes_as_runtime_diagnostics`, `pnpm --dir app build`, and focused `git diff --check` over touched Rust/frontend/doc files passed.
- Browser verification at 1280px confirmed Setup, Control, and Touch no longer create horizontal page overflow after the Output/DMX route panel CSS fix.
- 2026-06-14 (Claude takeover, frontend slice 1): Extracted pure numeric/DMX/color/geometry-matrix helpers out of `App.tsx` into new `app/src/numericHelpers.ts` (clamp01, finiteOr, clampRange, clampDmxValue, formatDmxPercent, dmxValueToPercent, percentToDmxValue, formatShortDmxPercent, valueToHexByte, outputAspectRatio, hsvToRgb, rgbToHsv, rgbToHex, defaultColorFavorites, normalizeHexColor, geometryIdentityMatrix, normalizedGeometryMatrix, multiplyGeometryMatrix, cumulativeGeometryMatrix, geometryMatrixTranslation, rotateStageOffsetYaw). `App.tsx` imports them back; net ~120 lines removed. Verified with `pnpm --dir app build` (tsc + vite, green, no chunk warning). Working tree was clean before this slice and is intentionally left uncommitted. Next safe slices: continue moving other pure helpers, then tackle the larger Setup Mapping / Output-Telemetry JSX panels (these need signal/prop threading, so do them as their own verified slices).
- 2026-06-14 (Claude takeover, frontend slice 2): Extracted stage-coordinate helpers into new `app/src/stageGeometry.ts` (`StageWorldBounds`, `stageViewBoxSize`, `stagePadding`, `stageWorldToSvgPoint`, `svgPointToStageWorld`) and GDTF channel-function helpers into new `app/src/channelFunctionHelpers.ts` (`normalizedFunctionText`, `channelFunctionValue`, `sortedChannelFunctions`, `pickFunctionValue`, `indexedFunctionValue`, `rankedFunctionValue`, `channelFunctionLabel`, `channelFunctionRangeLabel`, `channelFunctionDetail`). `App.tsx` imports them back. `App.tsx` now ~19,005 lines (down from ~19,082 after slice 1, ~19,200 originally). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted. Remaining pure-helper candidates in `App.tsx` worth extracting next: small pure label/enum helpers (e.g. `outputProtocolLabel`, `controlCategoryForAttribute`, `fixtureVisualKind`/`fixtureTypeKey`/`fixtureTypeLabel`/`mappingFixtureStageSize`, `videoSourceInputLabel`) — most depend only on `./types` enums and could form a `labelHelpers.ts`. After the obvious pure helpers are exhausted, the next big win is extracting whole JSX panels (Setup 2D Mapping, Output/Telemetry), which require signal/prop threading and should each be their own carefully-verified slice.
- 2026-06-14 (Claude takeover, frontend slice 3): Extracted SVG/stage-map export helpers into new `app/src/svgExportHelpers.ts` and the color/gobo wheel predicates (`isColorWheelFunction`, `isGoboWheelFunction`, `goboPatternForFunction`) into `app/src/channelFunctionHelpers.ts`. `App.tsx` now ~18,885 lines (down from ~19,005 after slice 2; ~19,200 originally — ~315 lines moved out across the three slices). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-14 (Claude takeover, frontend slice 4): Extracted localStorage color/position favorites into new `app/src/favoritesStorage.ts` and removed the duplicate local `PositionFavorite` interface from `App.tsx` (now imports the type from `components/PositionControlPanel`). `App.tsx` now ~18,785 lines (~420 lines moved out across four slices into `numericHelpers.ts`, `channelFunctionHelpers.ts`, `stageGeometry.ts`, `svgExportHelpers.ts`, `favoritesStorage.ts`). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-14 (Claude takeover, frontend slice 5): Extracted the Setup-mapping stage-tool/view-preset model + localStorage helpers into new `app/src/mappingViewPresets.ts` (`MappingStageTool`, `MappingViewPreset`, `normalizeMappingStageTool`, `defaultMappingViewPresets`, `mappingViewPresetFromUnknown`, `loadMappingViewPresets`, `saveMappingViewPresets`). `App.tsx` now ~18,655 lines (~550 lines moved out across five slices into six helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-14 (Claude takeover, frontend slice 6): Extracted the editor draft interfaces + "from summary" builders into new `app/src/editorDrafts.ts` (video-output config, cue metadata, timeline event/automation/video-automation). `App.tsx` now ~18,575 lines (~630 lines moved out across six slices into seven helper modules: `numericHelpers`, `channelFunctionHelpers`, `stageGeometry`, `svgExportHelpers`, `favoritesStorage`, `mappingViewPresets`, `editorDrafts`). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-14 (Claude takeover, frontend slice 8): Extracted the video-source label/media-formatting cluster into new `app/src/videoHelpers.ts` and moved `beamPoints` into `stageGeometry.ts`. This cluster sat immediately above `export default function App()`, so the tail end of the module-level pure helpers is now extracted. `App.tsx` now ~18,461 lines (~740 lines moved out across eight slices into nine helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-14 (Claude takeover, frontend slice 9): Extracted the MIDI/OSC control-mapping action sets + field catalog + classifier predicates/types into new `app/src/controlMappingActions.ts` (moved the `NumericVideoOutputMappingField`/`MappingFixtureFlag`/`FixtureFlagClearKind` types and `fixtureFlagClearKinds` const with them; App.tsx re-imports all). `App.tsx` now ~18,400 lines (~800 lines moved out across nine slices into ten helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted. STATUS: the cleanly-separable top-level helpers are now largely exhausted — most remaining module-level helpers are either tiny scattered label fns coupled to the core `ControlCategory` enum, or live inside the `App()` component body. Further meaningful size reduction now means extracting JSX panels (Setup 2D Mapping, Output/Telemetry), which thread SolidJS signals and should be done as deliberate, individually-reviewed slices rather than rapid autonomous micro-extractions.
- 2026-06-14 (Claude takeover, frontend slice 10, dedup): Removed duplicate untyped `defaultTransform`/`defaultColorAdjust`/`defaultFxAdjust` constants from `App.tsx`; these already existed (typed as `Transform2D`/`VideoColorAdjust`/`VideoFxAdjust`) in `app/src/videoLayerDefaults.ts`, which `App.tsx` was not importing. App.tsx now imports them from there, eliminating drift risk. Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-14 (Claude takeover, frontend slice 11): Extracted the type-free static UI preset arrays (`defaultColorPalette`, `colorQuickLooks`, `panTiltNudgeSteps`, `panTiltTargetPoints`, `mappingSnapPresets`) into new `app/src/uiPresets.ts`. `App.tsx` now ~18,338 lines (~865 lines off the original ~19,200 across eleven slices into eleven new helper modules: `numericHelpers`, `channelFunctionHelpers`, `stageGeometry`, `svgExportHelpers`, `favoritesStorage`, `mappingViewPresets`, `editorDrafts`, `fixtureVisuals`, `videoHelpers`, `controlMappingActions`, `uiPresets`). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted. This concludes the clean module-level pure-helper/data extraction phase — the remaining interleaved enum tables (`setupSubTabs`, `controlCategories`, `mappingVideoOutputCorners`) are tightly bound to core App-local types and give little benefit moved, and everything else is inside `App()`. The next high-value step is JSX-panel extraction (Setup 2D Mapping view, remaining Output/Telemetry orchestration), which threads SolidJS signals as props and should be done as deliberate, individually-reviewed slices.
- 2026-06-15 (Claude takeover, frontend slices 12-13): Two more clean pure-helper groups that turned out to still exist above `App()`: slice 12 extracted keyboard-shortcut helpers into new `app/src/hotkeyHelpers.ts` (`isEditableShortcutTarget`, `controlCueHotkeyIndex`, `mappingStageToolFromHotkey`); slice 13 extracted fixture limit/axis helpers + `defaultFixtureLimits` into new `app/src/fixtureLimits.ts`. `App.tsx` now ~18,271 lines (~930 lines off the original ~19,200 across thirteen slices into thirteen new helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted.
- 2026-06-15 (Claude takeover, frontend slices 14-15): slice 14 extracted project-snapshot comparison helpers into new `app/src/projectSnapshot.ts`; slice 15 extracted GDTF wheel-slot media helpers + types into new `app/src/wheelMedia.ts`. `App.tsx` now ~18,214 lines (~990 lines off the original ~19,200 across fifteen slices into fifteen new helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted. Remaining cohesive top-level group worth one more slice: the `ControlCategory` domain (done in slice 16).
- 2026-06-15 (Claude takeover, frontend slice 16): Extracted the control-category domain into new `app/src/controlCategory.ts` (`ControlCategory`/`CategoryQuickLook` types, `controlCategories` table, `controlCategoryForAttribute`, `quickLooksForCategory`, `opticsRoleForControl`, `quickLookValueForControl`). `App.tsx` now ~18,034 lines (~1,170 lines off the original ~19,200 across sixteen slices into sixteen new helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted. This effectively completes the module-level helper extraction: everything above `export default function App()` is now either a tiny core type alias / a few interleaved enum tables tightly bound to App-local types (`SetupSubTab`, `MappingVideoOutputCornerKey`, etc.), or the `invoke`/`listen` Tauri wrappers. All further size reduction now requires extracting code from INSIDE the `App()` component body — i.e. JSX panels (Setup 2D Mapping view, Control views, Output/Telemetry orchestration) that thread SolidJS signals and must be done as deliberate, individually-reviewed slices.
- 2026-06-15 (Claude takeover, frontend slices 17-19 + dedup pass): slice 17 extracted stage-object helpers into new `app/src/stageObjects.ts`. slice 18 moved the projector corner table/type/gain (`MappingVideoOutputCornerKey`, `mappingVideoOutputCorners`, `mappingVideoOutputCornerGain`) into the existing `videoOutputMapping.ts`, and removed App.tsx's DUPLICATE `defaultVideoOutputMapping` const (canonical copy already in `videoOutputMapping.ts`) + de-duplicated `NumericVideoOutputMappingField` (now defined once in `videoOutputMapping.ts`, re-exported by `controlMappingActions.ts`). slice 19 discovered that `uiModes.ts` already owned `WorkspaceTab`/`SetupSubTab`/`ControlCategory`/`controlCategories`/`controlCategoryForAttribute`/`setupSubTabs` — App.tsx had been carrying stale duplicates and the slice-16 `controlCategory.ts` had wrongly re-defined the `ControlCategory` trio. Fixed: App.tsx now imports those from `uiModes.ts`, and `controlCategory.ts` keeps only the quick-look helpers and imports `ControlCategory` from `uiModes`. Also de-duplicated `outputAspectRatio` (removed the slice-1 copy from `numericHelpers.ts`; App.tsx imports it from `videoOutputMapping.ts`) and `videoOutputAspectModes` (App.tsx local copy removed; imported from `videoOutputMapping.ts`). `App.tsx` now ~17,958 lines (~1,245 lines off the original ~19,200; 18 new helper modules + consolidation onto pre-existing `uiModes.ts`/`videoOutputMapping.ts`/`videoLayerDefaults.ts`). Verified with `pnpm --dir app build` (tsc + vite, green; main index chunk ~418 kB). Still uncommitted. LESSON FOR FUTURE EXTRACTION: before creating a new helper module, grep the existing `app/src/*.ts` (especially `uiModes.ts`, `videoOutputMapping.ts`, `videoLayerDefaults.ts`) — App.tsx still contains several constants/types that are stale duplicates of already-extracted ones; prefer importing the existing canonical copy over minting a new module.
- 2026-06-14 (Claude takeover, frontend slice 7): Extracted fixture visual-kind/label helpers into new `app/src/fixtureVisuals.ts` (moved the `MappingFixtureVisualKind` type with them; App.tsx's two interfaces that reference it now import the type). `App.tsx` now ~18,546 lines (~660 lines moved out across seven slices into eight helper modules). Verified with `pnpm --dir app build` (tsc + vite, green). Still uncommitted. NOTE: the remaining easily-separable pure helpers are now mostly small scattered label functions (diminishing returns). The next high-value step is extracting whole JSX panels (Setup 2D Mapping, Output/Telemetry) — a different, higher-risk slice class that threads SolidJS signals as props and should be reviewed deliberately rather than done as rapid autonomous micro-slices.

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

- `App.tsx` is ~18,340 lines (was ~19,200). Still very large; do not grow it casually. Prefer extracting cohesive UI/helper code to `app/src/components/` or focused helper modules (see `numericHelpers.ts` as the pure-helper extraction pattern). NOTE: the clean module-level pure-helper/data extractions are now essentially done (11 slices, 11 new helper modules); remaining module-level items are small enum/label tables coupled to the core `ControlCategory`/`SetupSubTab`/`MappingVideoOutputCornerKey` types, and the rest of the size is the `App()` component body (signals + JSX).
- No Vite chunk warning currently. The main `index` chunk is ~419 kB (under 500 kB) because `control-panels`/`mapping-editors` are already split via `manualChunks`. Further extraction is now a maintainability goal, not a chunk-size fix. To actually shrink the main chunk, route newly extracted modules into a named chunk in `vite.config.ts` or lazy-load a heavy panel; otherwise small helper modules still bundle into `index`. Good next refactor targets remain the Setup 2D Mapping view and Output/Telemetry panels.
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
