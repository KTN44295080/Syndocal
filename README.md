# KDMX

KDMX is a Rust/Tauri prototype for unified DMX lighting and VJ control.

Current Phase 1 focus:

- Load a `.gdtf` fixture archive and parse `description.xml`.
- Download a `.gdtf` file from a direct GDTF Share/download URL, save it locally, validate it, and load it into the patch flow.
- Extract fixture metadata, DMX modes, attributes, 8/16-bit channel offsets, and geometry nodes.
- Create a simple custom 8/16-bit fixture profile from a comma-separated attribute list when no GDTF file is available, preview its DMX footprint before patching, and save/load it as a readable `.fixture` JSON file.
- Reuse a patched fixture's summarized controls as a temporary patch profile, so loaded `.kdmx` projects can still add another fixture of the same type even when the original GDTF/custom profile file is not currently loaded.
- Patch one or many fixture instances with universe/address/label, group, position, and rotation data, then edit fixture transforms live after patching.
- Remove patched fixtures and clean their live values, fixture-scoped effects, cue targets, and timeline automations from the running show state.
- Assign fixtures to one or more named groups and target effects at either a selected fixture or group.
- Select groups in Setup, filter the fixture list, and apply live group-oriented control from the Control view.
- Use the Control view's Live Desk for operator-first GO/Back, cue fade pause/resume, timeline play/pause, DMX/video blackout, masters, BPM tap, banked cue pads with numeric hotkeys and optional active-cue following, and live system counts.
- Place the selected fixture directly from the 2D stage view, and apply quick Line/Grid/Circle layouts to the current group or the whole patch.
- Review patched fixture address occupancy with a per-universe DMX map in Setup.
- Validate fixture footprints before patching so a fixture cannot overflow a 512-channel universe or overlap an existing fixture in the same universe.
- Maintain a shared BPM clock with manual BPM entry, tap tempo, and MIDI Clock input.
- Map MIDI Note/CC/Program Change messages to fixture attributes, direct cue triggers, cue-list next/previous, lighting master/group submasters, timeline play/seek, lighting/video blackout, video parameters, video play state, and video cue-point jumps.
- Receive basic OSC control over UDP for fixture attributes, blackout, BPM, and tap tempo.
- Control a lighting grand master that scales Dimmer/Intensity attributes without affecting movement or color channels.
- Control group submasters that scale Dimmer/Intensity for fixtures assigned to matching patch groups.
- Toggle fixture or group Highlight, Solo, and Park states for live focusing; Highlight forces intensity/color channels open at the DMX output stage, Solo suppresses non-soloed fixtures, and Park freezes rendered output values until cleared.
- Receive OSC control for video layer parameters, video master opacity, and video blackout.
- Host a lightweight browser/iPad remote endpoint that serves an installable PWA-style HTML remote page with a Live Desk, selected-fixture fader bank, manifest/icon/service-worker assets, and accepts WebSocket JSON commands for faders, banked cue pads, BPM, timeline transport, blackout, and video parameters.
- Store current fixture/video states as cues, duplicate cues as look variants, rename or retime existing cues, reorder the cue list, update cue contents from the current look, and recall them with GO/Back cue-list controls.
- Pause and resume an active cue fade without losing its current output level.
- Store current video layer states in the same cues, so one cue can recall lighting and video state together.
- Place and edit lighting or video cue events on a shared timeline and play them back with transport controls.
- Analyze PCM/float WAV files into a timeline waveform preview with beat markers and a simple BPM estimate.
- Snap timeline event and automation time inputs to detected audio beats, bars, or a fixed millisecond grid.
- Sync the shared timeline from incoming MIDI Time Code quarter-frame messages.
- Add and edit lighting fixture attribute automation on the timeline with step, linear, and handle-less Bezier ease-in-out keyframes.
- Create placeholder video layers with enabled/mute, solo, opacity, speed, play/pause, position, and A-B loop state for the VJ engine data path.
- Add local video/still-image layers through a file picker or direct path entry, and feed still-image (`PNG`/`JPEG`) decoded RGBA pixels into the CPU preview path.
- Add NDI, Spout, and Syphon input layers by source/sender/server name so external-video routing can be composed before native transport bindings land.
- Cache decoded/resized still-image frames and invalidate them when the source file metadata changes, avoiding repeat disk decode on every preview render.
- Keep the preview-only video runtime and still-image cache behind a `VideoPreviewRenderer` boundary in the `video` crate, so Tauri only requests a rendered preview frame.
- BPM-sync video layers by deriving playback speed from the shared BPM clock, A-B loop length, ratio, and bar count.
- Store and jump to video cue points in the layer state.
- Rename, duplicate, enable/disable, solo, set video layer blend modes (`Normal`, `Add`, `Multiply`, `Screen`), reorder the layer stack, and expose a default `Main` composition snapshot for the future compositor.
- Create additional video compositions with explicit layer membership and order, and route display/NDI/Spout/Syphon output targets to any composition in the engine snapshot.
- Resolve each video output target into a render plan and render an output-specific CPU preview that applies output enabled state, opacity, blackout, and composition routing.
- Open a Display output in a dedicated Tauri output window that continuously polls the output-specific CPU preview path as a renderer bridge before the native wgpu output path lands.
- Open a mapping-aware test-pattern window for Display outputs, with the same output scale, aspect mode, lens distortion, keystone, and corner warp applied as the live preview path.
- Edit projector ratio, offset, scale, lens distortion, keystone, and corner warp from Setup with both numeric fields and a small draggable TL/TR/BR/BL visual map surface.
- Save projector mapping presets inside the project snapshot, then apply, delete, export, or import those ratio/distortion/warp setups per output.
- Control video master opacity and video blackout independently from DMX blackout.
- Edit video layer transform values from the Video panel, including normalized position, scale, rotation, and crop.
- Edit video layer color adjustment values from the Video panel, including brightness, contrast, hue, saturation, and gamma.
- Edit lightweight video FX values from the Video panel, currently pixelate block size, blur radius, glow, edge detection, and color key, and render them in the CPU reference compositor.
- Build renderer-facing composition plans from the current video snapshot, with ordered layers, effective opacity, transform, source, and blend mode.
- Maintain a bounded decoded-frame queue abstraction for future FFmpeg/HAP decoder output and wgpu upload scheduling.
- Manage per-layer frame queues through a lightweight `VideoRuntime` and select the correct frame for each composition layer by playhead position.
- Feed preview frames through a swappable `VideoFrameProvider` trait, with the current provider handling still images, FFmpeg-backed file frame extraction with small lookahead prefetch, and deterministic placeholders for unsupported network/GPU sources.
- Provide a `VideoFrameDecoder` boundary and FFmpeg CLI decoder-backed frame provider so future in-process FFmpeg/HAP implementations can supply decoded frames without changing Tauri or composition code.
- Probe video file metadata with `ffprobe` when available, storing codec, duration, resolution, and frame-rate hints in the layer source snapshot.
- Provide a CPU RGBA8 reference compositor for `Normal`, `Add`, `Multiply`, and `Screen` blend modes so future wgpu shaders have deterministic parity tests.
- Provide a renderer-facing `visualizer` crate that converts the current engine snapshot into 3D fixture, beam, and stage-bounds primitives for the future wgpu visualizer.
- Apply video layer transform basics in the CPU compositor: normalized position, scale, rotation, and source crop are all reflected in preview output.
- Render a small CPU debug preview through Tauri using the `video` crate's preview renderer. Still images are decoded directly and file layers can extract the requested preview frame through `ffmpeg` when it is available on `PATH` or configured with `KDMX_FFMPEG`.
- Display the CPU preview image in the SolidJS Video panel for quick visual verification of still-image layers and blend modes.
- Add and edit video layer opacity, speed, and playhead position automation on the same timeline transport.
- Add and edit video layer transform automation on the same timeline, including position, scale, rotation, and crop parameters.
- Target video layer parameters from the same LFO/position-wave effect system used by lighting, so effects can drive opacity, speed, and transform values without creating timeline automation.
- Add LFO and position-wave effects to drive patched fixture attributes with sine/cosine/triangle/saw/square/random/perlin shapes and override/add/multiply blend modes.
- Reorder the live effect stack from the Control UI so override/add/multiply layering can be adjusted without recreating effects.
- Enable/disable effects without removing them, so live looks can be parked for later reuse.
- Save and load effect definitions as human-readable `.effect` JSON files for reusable LFO and position-wave looks.
- Show lightweight 2D and SVG isometric 3D stage views with fixture dots, selected-fixture highlighting, height guides, and dimmer/color/Pan-aware beam wedges.
- Save and load fixture attribute snapshots as human-readable `.preset` JSON files.
- Save and load the current show state as a human-readable `.kdmx` project snapshot.
- Adjust RGB fixture attributes together with a color picker when `ColorRed`/`ColorGreen`/`ColorBlue` style controls are present.
- Drive a 44 Hz engine loop that renders DMX frames.
- Send DMX frames over one or more Art-Net, sACN/E1.31 UDP, Enttec DMX USB PRO, or Enttec Open DMX compatible output routes.
- Control fixture attributes from a SolidJS/Tauri UI.
- Inspect the current rendered universe with a 512-channel raw DMX monitor.
- View patched fixtures in a lightweight top-down 2D stage view using their X/Z positions and current dimmer/color state.

## Development

Prerequisites currently verified in this workspace:

- Rust/Cargo 1.86.0
- Node.js 20.19.1
- pnpm 10.10.0
- `curl` available on `PATH` for runtime GDTF URL downloads

Install frontend dependencies:

```powershell
cd app
pnpm install
```

Run checks:

```powershell
cargo test --workspace
cd app
pnpm build
pnpm tauri build --debug
```

Run the desktop app:

```powershell
cd app
pnpm tauri dev
```

## MVP Workflow

1. Start the app with `pnpm tauri dev`.
2. Browse for a local `.gdtf` file, enter the path manually, or paste a direct GDTF Share/download URL and download it into the local library. If no GDTF exists, create or load a custom 8/16-bit `.fixture` profile by entering manufacturer, name, mode, and comma-separated attributes such as `Dimmer@1:8, Pan@2:16, Tilt@4:16`. The `@` value is an optional fixture-local start channel and `:8`/`:16` sets the channel resolution.
3. Load or create the profile and choose a DMX mode.
4. Optionally enter comma-separated groups plus position/rotation values, then patch one fixture or a counted batch to a universe/address after confirming footprint and end address. Batch patching can use an address step plus X/Z step to spread fixtures across the patch. Use the DMX Map to confirm occupied ranges per universe. Select a patched fixture later to edit its transform live, reuse that fixture's profile as the active patch profile, click the 2D View to place it in X/Z, or use Line/Grid/Circle layout buttons to arrange the current group or full patch.
5. Select Art-Net, sACN/E1.31, Enttec USB PRO, or Enttec Open DMX, then set the target IP/port/universe or serial port. Add additional DMX routes when the same show should drive multiple output targets at once.
6. Set, tap, or connect MIDI Clock if tempo-synced behavior is being tested.
7. Start OSC input or Web Remote if network control is needed.
8. Move attribute faders to send ArtDMX frames.
9. Use the color picker for RGB-capable fixtures.
10. Save or load a selected fixture's fader state with `.preset` files when reusable looks are needed.
11. Add video file layers with the Browse Source picker or direct path entry when testing VJ state capture, add still-image layers for PNG/JPEG CPU preview, or add named NDI/Spout/Syphon input layers for external source routing placeholders. When `ffprobe` is available on `PATH` or configured via `KDMX_FFPROBE`, file layers capture codec, duration, resolution, and frame-rate metadata. The current implementation stores video source paths and playback parameters but does not decode moving video or bind live external transport yet.
12. Configure lighting master/blackout, group submasters, video master/blackout, create compositions, route outputs, edit output labels/kinds/resolutions/display targets, rename/duplicate/reorder video layers, reorder a custom composition's layer order, play/pause, blend mode, speed, playhead position, layer transform, color adjustment, pixelate/blur/glow/edge/color-key FX, A-B loop, cue points, and optional BPM sync ratio/bar length. Display outputs can be opened in a dedicated output window for CPU-rendered visual checks, or as a test-pattern window for projector ratio/aspect-mode/lens/keystone/corner adjustment. The Setup projector map surface can drag TL/TR/BR/BL handles for coarse corner-warp adjustment, then use the numeric fields for exact values.
13. Store cues from the current patched fixture and video layer states, duplicate cues for look variants, edit cue labels/fade times, reorder the cue list, update existing cues after refining a look, then use GO/Back or direct cue GO to recall them together.
14. Optionally analyze a `.wav` file in the Timeline panel to attach a waveform, beat markers, and an estimated BPM to the timeline; the estimate can be applied to the shared clock.
15. Add stored cues to the Timeline at millisecond positions, choose Lighting or Video track, edit existing event cue/time/track values, optionally snap edit times to beat/bar/grid positions, and use Play/Pause/seek to fire them in time.
16. Add and edit timeline automation for selected fixture attributes or video layer opacity/speed/playhead/transform parameters when continuous changes are needed.
17. Add an LFO or position-wave effect from the selected fixture, target one or more groups, or target a video layer parameter to modulate lighting and video from the same effect engine. Use the effect list Up/Down controls when stack order matters.
18. Save reusable effects as `.effect` files or load an existing effect preset into the current show.
19. Save or load the current show state as a `.kdmx` project snapshot when you need a readable checkpoint.
20. Check fixture placement and simple beam state in the 2D View or the isometric 3D View.
21. Confirm the outgoing values in the DMX Raw monitor before connecting hardware.

The default Art-Net target is `127.0.0.1:6454`, universe `0`, which is useful for loopback receivers and packet tests before connecting hardware. The Output panel can apply the current target as the primary route, or build a route list so multiple Art-Net/sACN/serial outputs are sent on the same 44 Hz engine tick. sACN/E1.31 uses port `5568` by default when selected in the UI and requires universe `1` or greater. Enttec USB PRO output writes label `6` DMX packets to a selected serial port. Enttec Open DMX output uses standard `250000` baud 8N2 serial with a software-timed break and mark-after-break before each 513-byte DMX payload, so exact break timing still depends on the OS and USB driver.

The Rust test suite includes UDP loopback coverage for ArtDMX and sACN packet sending, serial packet-builder coverage for Enttec USB PRO and Open DMX payloads, plus engine-level checks that a patched fixture's fader state reaches network DMX receivers.

Engine telemetry exposes the latest 44 Hz tick interval plus jitter last/max/stddev values in microseconds, current output route count, per-tick DMX send success/failure counts, cumulative DMX send success/failure counts, and last packet bytes, so runtime builds can be watched against the latency and jitter targets before external packet captures are taken.

MIDI Clock support listens for standard `0xF8` timing clock messages and estimates BPM from the 24 PPQN pulse stream. Start (`0xFA`), Continue (`0xFB`), and Stop (`0xFC`) messages drive the shared timeline transport. The same input connection also decodes MIDI Time Code quarter-frame messages (`0xF1`) and syncs the shared timeline position; due timeline cues are fired as the external timecode crosses their event times. The current clock source is shown in the Output panel. The MIDI Control panel can map Note On, Note Off, Control Change, or Program Change messages by optional channel and controller/note/program number. The Learn button opens a temporary 10-second listener and fills the message/channel/number fields from the next supported MIDI message. Current mapping actions are fixture attribute value, trigger cue, cue next/previous, lighting master, group submaster, cue fade pause, timeline play, timeline seek, lighting blackout, video blackout, video parameter value, video cue-point jump, video play state, and video output enable/opacity/fade/blackout. CC values are normalized from `0..127` into each mapping's configured low/high range. MIDI mappings can be saved and loaded as pretty-printed `.midimap` JSON files. MIDI Feedback can connect to a MIDI output and send current snapshot values back through the same mappings; fixture attributes, lighting master, group submasters, timeline position, video parameters, and video output opacity/fade targets are normalized through each mapping's low/high range, trigger cues report active cue state, cue fade pause reports pause state, blackout mappings report blackout state, and timeline/video play plus video output enable report play or enabled state. Auto feedback sends this snapshot feedback periodically for controller LEDs and motorized surfaces.

OSC input defaults to `0.0.0.0:9000`. The fixed lighting routes are `/kdmx/fixture/{id}/{attribute}` with a numeric value, `/kdmx/fixture/{id}/highlight`, `/kdmx/fixture/{id}/solo`, `/kdmx/fixture/{id}/park`, `/kdmx/group/{group_id}/highlight`, `/kdmx/group/{group_id}/solo`, `/kdmx/group/{group_id}/park`, `/kdmx/submaster/{group_id}`, `/kdmx/cue/{id}`, `/kdmx/cue/{id}/go`, `/kdmx/cue/pause`, `/kdmx/cue/resume`, `/kdmx/cue/go`, `/kdmx/cue/back`, `/kdmx/timeline/play`, `/kdmx/timeline/pause`, `/kdmx/timeline/seek`, `/kdmx/blackout`, `/kdmx/master`, `/kdmx/bpm`, and `/kdmx/tap`. Video layer routes are `/kdmx/video/layer/{id}/{param}`, `/kdmx/video/master`, and `/kdmx/video/blackout`; params include `opacity`, `speed`, `position`, `transform_x`, `transform_y`, `scale_x`, `scale_y`, `rotation`, crop fields such as `crop_left`, color fields `brightness`, `contrast`, `hue`, `saturation`, and `gamma`, plus FX fields `pixelate`, `blur`, `glow`, `edge`, `key_red`, `key_green`, `key_blue`, and `key_threshold`. Playback and hot-jump routes are `/kdmx/video/layer/{id}/play` with a boolean, `/kdmx/video/layer/{id}/seek` with a millisecond value, `/kdmx/video/layer/{id}/cue/add` with an optional millisecond value, `/kdmx/video/layer/{id}/cue/remove` with a millisecond value, and `/kdmx/video/layer/{id}/cue/jump` with a cue-point index. Video output routes are `/kdmx/video/output/{id}/enabled` with a boolean, `/kdmx/video/output/{id}/opacity` with a numeric level, `/kdmx/video/output/{id}/blackout` with a boolean, `/kdmx/video/output/{id}/fade` with target opacity plus optional duration milliseconds, and `/kdmx/video/output/{id}/fade_in` or `/kdmx/video/output/{id}/fade_out` with optional duration milliseconds. In addition to fixed routes, the OSC panel can map arbitrary addresses such as `/touchosc/fader1`, or one-segment wildcard patterns such as `/touchosc/page/*/fader/1`, to fixture attribute value, trigger cue, cue next/previous, lighting master, group submaster, cue fade pause, timeline play, timeline seek, video parameter value, video cue-point jump, video play state, video output enable/opacity/fade/blackout, lighting blackout, or video blackout actions. The Learn button opens a temporary 10-second listener on the current Bind IP/Port and fills the address field from the next OSC message; stop the normal OSC input first because both listeners bind the same UDP port. OSC mappings can be saved and loaded as pretty-printed `.oscmap` JSON files. Float fixture attribute values in the `0.0..1.0` range are scaled to 16-bit DMX values.

Web Remote defaults to `0.0.0.0:9100`. Open `http://<host>:9100/` from a tablet browser to load the built-in remote page; the page connects back to `ws://<host>:9100/ws`, requests the current `EngineSnapshot`, and fills fixture, cue, video layer, video output, and attribute selectors from live state. The endpoint also serves `/manifest.webmanifest`, `/icon.svg`, and `/remote-sw.js` so the remote can be added to a tablet home screen as a PWA-style controller where the browser permits it. It renders all controls for the selected fixture as a fader bank, and refreshes the snapshot once per second and after commands, so the tablet view can show fixture/cue/layer counts, banked cue pads with active/next cue state, current layer state, video output state, timeline position, BPM, and blackout/master status. The WebSocket accepts one JSON command per text message. Initial command types are `getSnapshot`, `setAttribute`, `setFixtureHighlight`, `setFixtureSolo`, `setFixturePark`, `setGroupHighlight`, `setGroupSolo`, `setGroupPark`, `setGroupSubmaster`, `blackout`, `lightingMaster`, `setBpm`, `tapBpm`, `triggerCue`, `triggerNextCue`, `triggerPreviousCue`, `setCueFadePaused`, `setTimelinePlaying`, `seekTimeline`, `setVideoParam`, `setVideoLayerEnabled`, `setVideoLayerSolo`, `setVideoPlaying`, `seekVideoLayer`, `addVideoCuePoint`, `removeVideoCuePoint`, `jumpVideoCuePoint`, `setVideoOutputEnabled`, `setVideoOutputOpacity`, `fadeVideoOutputOpacity`, `setVideoOutputBlackout`, `videoMaster`, and `videoBlackout`. The server replies with a small JSON acknowledgement or a snapshot response so the remote page can show command success or parse errors.

Cues store lighting fixture attribute snapshots and video layer state snapshots. Cue recall is evaluated on the 44 Hz engine tick; a cue with `fade_ms = 0` applies lighting and video states immediately, while nonzero fades interpolate base lighting attribute values before effects are stacked and rendered. Active fades can be paused and resumed; the engine subtracts paused time from fade progress so output holds steady while paused. Video cue fades now interpolate continuous visual layer values such as opacity, speed, transform, color adjustment, and lightweight FX, while discrete playback, loop, cue-point, BPM-sync, and playhead fields are applied at trigger time.

Timeline playback supports cue events on `Lighting` or `Video` tracks, fixture-attribute automation on lighting tracks, and video layer automation for opacity, speed, playhead position, and transform parameters. Timeline edit inputs can snap to detected audio beats, BPM-derived bars, or a fixed millisecond grid before commands are sent to the engine. Automation keyframes support `Step`, `Linear`, and `Bezier`; the current Bezier mode is a handle-less cubic ease-in-out curve until explicit curve handles are added to the data model.

The `audio` crate provides the first timeline-audio analysis path without adding native decoder dependencies: it reads RIFF/WAVE PCM or 32-bit float files, builds a bounded peak/RMS waveform, detects simple energy peaks as beat markers, and estimates BPM from beat intervals. The Timeline panel can attach that analysis to the engine timeline snapshot, draw the waveform and beat markers from live state, clear it, and apply the estimated BPM to the shared clock.

Video layers are currently an engine/UI data model: file path or external source name, source metadata, label, enabled state, solo state, stack order, blend mode, opacity, speed, playing state, playhead position, cue points, A-B loop state, BPM sync state, transform, color adjustment state, and lightweight FX state are advanced on the engine tick and can be captured by cues. Existing layers can be renamed, duplicated, soloed, or temporarily disabled without overwriting their opacity; duplication preserves source, blend mode, state, stack position, and custom composition membership so look variants can be made without rebuilding a layer from scratch. If any enabled layer in a composition is soloed, non-solo layers are omitted from the render plan. Cue points, playback, enabled state, and solo state are first-class engine commands, so the main UI and Web Remote can handle live layer operations without rewriting the whole layer state. BPM sync computes effective playback speed from the shared BPM clock, the selected loop region, ratio, and loop bar count. Video file metadata is probed through `ffprobe` when available and persists in `.kdmx`; source duration contributes to the shared timeline duration, can be copied into A-B loop out points from the UI, and clamps non-looping playback so forward playback stops at the source end while reverse playback stops at 0 ms. The snapshot exposes a default `Main` composition containing the active layer order, plus user-created compositions with explicit layer lists and derived output target IDs. Video outputs currently store routing, display/NDI/Spout/Syphon kind, resolution, enabled state, opacity, and blackout state so the future renderer can bind output surfaces without changing cue/timeline state. Output opacity can also be faded over engine ticks from Setup and the Control tab's output bank, giving projector feeds smooth fade-out/fade-in behavior without changing layer levels. The `video` crate can now build output render plans and render an output-specific CPU preview frame, including output opacity and blackout, which gives the later wgpu/display/NDI sender path a deterministic reference. Setup can render either the routed output frame or a calibration test pattern with major/minor grids, center target, safe-area frame, thirds guides, diagonals, and color-coded corner markers inside the output card while projector aspect, lens distortion, keystone, and corner warp values are being tuned. The Tauri app can open a Display output window that continuously renders through this same CPU reference path, so output routing can be checked on a separate screen before the native wgpu surface renderer replaces it. Still-image layers can be decoded into RGBA for the CPU preview path and are cached until the source file metadata changes. File layers can decode a preview frame at the current playhead through an FFmpeg CLI adapter (`ffmpeg` on `PATH`, or `KDMX_FFMPEG=/path/to/ffmpeg`), and NDI/Spout/Syphon input layers currently render deterministic placeholder frames until native transport receivers are connected. The decoder-backed preview provider now pushes a small lookahead window into the per-layer frame queue to mimic the future jitter-absorbing decode ring. Actual in-process FFmpeg/HAP moving-video decode scheduling, wgpu compositing, low-latency display output, NDI, Spout, and Syphon transport are still future VJ-phase work.

The `video` crate now owns renderer-facing primitives that are intentionally independent of Tauri and the lighting engine: a bounded `FrameQueue` for decoded frames, `VideoRuntime` for per-layer queue management and frame selection, a swappable `VideoFrameProvider` boundary for preview/decode frame supply, a `VideoFrameDecoder` boundary with a command-backed FFmpeg adapter, `VideoPreviewRenderer` for preview-only frame submission/composition, `build_composition_plans` for turning `VideoSnapshot` data into compositor-ready layer instructions, and a CPU `composite_rgba8` reference path for blend-mode, transform, color-adjustment, pixelate, blur, glow, edge, and color-key correctness tests. Decoder-backed providers can return multiple frames per layer so the runtime queue receives current and lookahead frames before composition. Tauri commands expose composition plans and request a small CPU debug preview frame; the SolidJS Video panel converts that RGBA preview frame into a browser-rendered image for visual checks until in-process FFmpeg/HAP decoders and wgpu output are connected.

The `visualizer` crate is the renderer-facing boundary for the later native wgpu 3D view. It turns `EngineSnapshot` fixture state into fixture nodes, beam cones with normalized color/intensity/direction, and stage bounds without depending on Tauri or GPU APIs, so the future renderer can be tested against deterministic scene data first. Tauri exposes this through `get_visualizer_scene`, keeping the native visualizer window decoupled from engine internals.

Position-wave effects use each patched fixture's `x/y/z` position. A direction vector projects fixtures along an axis; if the direction is zero, radial distance from the origin is used.

Effect blend modes are evaluated in stack order, and the Control UI can move live effects up or down in that stack. For lighting attributes, `Override` replaces the current value, `Add` saturates at full scale, and `Multiply` scales by the effect value. For video parameters, the same source evaluation is applied to the snapshot state as float values, then sanitized by the video layer state rules.

Effect presets are stored as pretty-printed `.effect` JSON files with a version, effect type, enabled state, and the original LFO or position-wave request body. Loading an effect preset validates the request shape and checks referenced fixture and video layer IDs before creating a new live effect, so stale presets fail visibly instead of silently targeting nothing.

Fixture presets are stored as pretty-printed JSON with the fixture manufacturer, profile name, mode name, and attribute values. They are intentionally readable and suitable for Git tracking. Loading a preset checks the selected fixture's manufacturer, profile name, mode name, and available attributes before applying values, so presets made for another mode are rejected instead of silently writing mismatched channels.

Project snapshots are stored as pretty-printed `.kdmx` JSON files containing a version marker, app name, and the current `EngineSnapshot`. Save Project writes a readable checkpoint for patch/cue/timeline/audio/video/effect/output state, and Load Project rebuilds that snapshot into the live engine while advancing ID allocators so new fixtures, cues, timeline events, video layers, outputs, and effects do not collide with loaded IDs. Loaded fixture profiles are currently rebuilt from the snapshot's summarized controls; the original GDTF XML, meshes, geometry tree, physical data, and source audio bytes are not embedded in `.kdmx` yet.

Custom fixture profiles are kept in memory as `memory://custom/...` profiles after creation or load, and can be saved as pretty-printed `.fixture` JSON files containing the manufacturer, profile name, mode name, and ordered attribute list. Each listed attribute can declare an optional fixture-local start channel with `@` and an 8/16-bit resolution with `:8` or `:16`; Pan/Tilt-style attributes default to center, while other attributes default to zero. Patched fixtures can also be rebuilt into temporary `memory://patched-fixture/...` profiles from their summarized controls. This is a patching convenience, not a replacement for full GDTF physical data.

GDTF URL downloads currently use the system `curl` binary to keep the prototype free of a bundled HTTP client dependency. The command accepts `http://` or `https://` URLs, proposes a safe `.gdtf` filename, follows redirects, validates the downloaded archive through the existing GDTF parser, and removes failed/invalid downloads. A richer GDTF Share browser/search UI is still future work.

The 2D View is intentionally frontend-only for now: it maps patched fixture `x/z` positions into a fixed top-down stage grid, uses current `Dimmer`/RGB attributes when present, highlights the selected fixture, and can place the selected fixture by clicking in the stage. The mapping editor also provides Line/Grid/Circle layout helpers that operate on the selected group filter or the full patch. The SVG 3D View consumes the same `VisualizerScene` data exposed for the future native renderer, projecting fixture nodes and beam primitives into a lightweight isometric stage with height guides.

Video output mapping is currently a CPU-reference path for the future native renderer. Each output can edit its label, output kind, resolution, display fullscreen/monitor target, or network/GPU endpoint name without losing route, opacity, blackout, or mapping state. Each output can adjust projector ratio and distortion correction controls: offset, scale, rotation, aspect ratio, stretch/fit/fill aspect mode, lens distortion, horizontal/vertical keystone, and four-corner warp. Those mapping values can be saved as project-level projector presets, then applied to any output, deleted from Setup, or exported/imported as pretty-printed `.projmap` JSON files for venue/projector reuse. Display outputs can open either a live CPU preview window or a mapping-aware test-pattern window with a grid, center lines, border, and colored corners for projector alignment.

The color picker appears only when the selected fixture exposes red, green, and blue controls using common GDTF-style names such as `ColorRed`, `ColorGreen`, and `ColorBlue`.

## Notes

- `Cargo.lock` is intentionally tracked because the local Rust toolchain is 1.86.0 and newer Tauri transitive dependencies currently require newer Rust.
- The current build does not yet include hardware-timed Enttec Open DMX break scheduling, native low-latency wgpu video output rendering, or native volumetric wgpu 3D visualization. The Display output window is a CPU preview bridge, not the final VJ presentation pipeline.
