# SynapseRack / Syndocal functional comparison - 2026-10-15

- Owner decision: compare functions, not performance. Replaces the cancelled
  SynapseRack rows in [Tier C](SYNAPSERACK_TIER_C_2026-10-15.md).
- Syndocal: shared `main`, base `e6c76d95`, plus uncommitted Part B/C changes;
  rebuilt `1.2.0-alpha.82` release inspected. No feature implementation in Part D.
- SynapseRack: installed Windows **3.3.0 build 122 Demo**; visible Demo/3.3.0
  and installed `C:/Program Files (x86)/SynapseRack/build-info.json` verified.
  Current official manuals may describe a newer catalog than this installed build.
- `verified` means observed UI or inspected implementation, as qualified in the
  cell. A source-verified feature is not a hardware, network or release pass.
- `documented-only` means the vendor/repo describes it, without execution here.
  `unknown` means evidence is missing or conflicting; it does not mean absent.
- Gap cells compare only those evidence levels. No pixel-equivalence, stability,
  device interoperability or feature-count ranking follows from a menu entry.
- Screenshots: `target/qa/synapserack-features/` (ignored). Entered Demo, inspected
  menus/panels/settings, and opened the mapper without a quad or physical output.
  No account, sign-in, key, payment, external controller test or recording take.
- The Demo reported a previous abnormal exit and offered autosave recovery.
  Recorded it, then used normal startup. Existing four-layer template loaded;
  no project was saved. Settings were viewed without Apply or device changes.
- Syndocal's benchmark outputs were closed and the empty project restored to
  Standby before workflow inspection. Main, Video and Timeline were observed;
  final screenshots were refreshed after verifying the native maximize state.
  The empty disarmed session and Demo are left open for review.

## Clip/deck/layer model

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Layer and clip assignment | verified (UI): four Layer panels, MediaBrowser and hierarchy. documented-only: files, live senders or node results feed a layer. | verified (source/UI): reusable media assets, per-layer Clip Bank, in/out, loop, speed, cues and launch quantization; Preview/Program staging. | verified: Different selection models; no demonstrated fixed two-deck limit on either. | verified: [Layer](https://synapserack.com/en/docs/window/layer/); `crates/protocol/src/lib.rs:1138`; `target/qa/synapserack-features/03-demo-ready-0.png`; `target/qa/synapserack-features/syndocal-02-video-2.png` |
| verified: Group composition | documented-only: Group Layer first composites members, then applies its own blend, opacity and transform. Create menu observed. | verified (source): flat contiguous layer groups within one composition; members are actual layers and belong to one group. Members are composited into a group frame for scoped group FX; transition targeting also exists. | documented-only: SynapseRack's separately blended group image is broader than Syndocal's flat group model. Nested image-group parity is unproven. | verified: [Group Layer](https://synapserack.com/en/docs/window/group-layer/); `crates/protocol/src/lib.rs:1008`; `crates/protocol/src/lib.rs:2517`; `crates/video/src/lib.rs:5548`; `target/qa/synapserack-features/05-create-menu-0.png` |

## Composition and blend

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Layer blend modes | verified (UI): Add, AlphaAdd, AlphaBlend, Inversion, Multiply, Screen, Substract, Cutout. | verified (source): Normal, Add, Multiply, Screen. | verified: Syndocal lacks named AlphaAdd/Inversion/Substract/Cutout modes. AlphaBlend versus Normal pixel equivalence was not tested. | verified: `target/qa/synapserack-features/13-blend-modes-0.png`; [Layer](https://synapserack.com/en/docs/window/layer/); `crates/protocol/src/lib.rs:704`; `crates/video/src/lib.rs:9553` |
| verified: Image graph versus modulation graph | verified (UI): NodeEditor. documented-only: typed image ports, composite nodes and reusable graph groups. | verified (source): Lfo/PositionWave/Audio/Transform/Output graph drives lighting and video parameters; composition uses ordered layers. | documented-only: SynapseRack provides general image-graph wiring; Syndocal's closed modulation graph cannot express that image topology. | verified: [Node Editor](https://synapserack.com/en/docs/window/node-editor/); `crates/protocol/src/lib.rs:6052`; `crates/protocol/src/lib.rs:6037`; `target/qa/synapserack-features/03-demo-ready-0.png` |

## Effects and generators, including ISF/shaders

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Effect chains | documented-only: FxShader/FxIn/FxOut chains and custom effects alongside built-ins. Categories observed. | verified (source): 14 built-in ISF presets; canonical chains up to 32 stages with scoped clip/layer/group/composition/output policy. | documented-only: SynapseRack's creative catalog is broader; no one-to-one effect equivalence or quality ranking tested. | verified: [Fx Editor](https://synapserack.com/en/docs/fxeditor/); `crates/video/src/builtin_isf.rs:12`; `crates/protocol/src/lib.rs:134`; `crates/protocol/src/lib.rs:2597` |
| verified: Shader import boundary | documented-only: GLSL/Shadertoy to .srfx; ISF conversion; Windows D3D11 / macOS Metal. Export uses a conversion service. Legacy .hlsl is separate. | verified (source): native single-image ISF filter preparation; rejects audio textures, imported resources, multipass and persistent/target buffers. | documented-only: Different shader ecosystems. Syndocal has explicit compatibility limits; no universal ISF or .srfx interchange claim. | verified: [Fx Editor](https://synapserack.com/en/docs/fxeditor/); [HLSL](https://synapserack.com/en/docs/load-shader/); `crates/video/src/isf_runtime.rs:771`; `crates/video/src/isf_runtime.rs:831` |
| verified: Pure generators and 3D | documented-only: Gradient produces a texture without media; 3D/Field/Text generator categories. | verified (source): no standalone persisted Generator input kind; existing ISF runtime requires one image. Parameter LFOs are not video generators. | documented-only: A direct Syndocal gap for media-free procedural visuals. | verified: [Gradient](https://synapserack.com/en/docs/nodes/generator/gen.gradient/); [Official product](https://synapserack.com/en/); `crates/protocol/src/lib.rs:551`; `crates/video/src/isf_runtime.rs:778`; `qa/VIDEO_LIVE_SOURCES_CHECKPOINT_2026-09-13.md:60` |

## Transitions

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Switching and crossfade | documented-only: CrossFade mixes two image inputs; custom graph/SDK mixer can make A/B workflows. | verified (source): Clip Take Cut/Crossfade/Dip/Wipe/Luma/Displacement/Blur/Glitch/Custom; durations in ms/beats/bars, frozen at admission. | documented-only: Syndocal has explicit authored Take styles; SynapseRack offers graph-built transitions. Exact style parity unknown. | verified: [CrossFade](https://synapserack.com/en/docs/nodes/composite/CrossFade/); [Apps SDK](https://synapserack.com/en/docs/apps-sdk/); `crates/protocol/src/lib.rs:1192`; `crates/protocol/src/lib.rs:1208`; `crates/engine/src/lib.rs:50063` |
| verified: Loop-seam crossfade | documented-only: per-clip toggle, fixed 10 percent overlap. | unknown: loop modes and Take transitions are verified, but an equivalent automatic end/start overlap was not established. | unknown: Candidate gap; do not confuse ordinary clip Take with loop-seam blending. | verified: [Crossfade loop](https://synapserack.com/en/docs/crossfade-loop/); `crates/protocol/src/lib.rs:1138` |

## Output, mapping and projection, including Spout/NDI/Syphon and multi-output

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Native display / multiple routes | verified (UI): Master/Mapping destination menu and Create Output Window. documented-only: direct fullscreen menu supports one active output at a time; bundled window receives master via Spout/Syphon. | verified (source): independently identified outputs/compositions, stable monitor identity and per-output native window/worker. Native single Display observed in Part C. | documented-only: Syndocal supports distinct authored display routes beyond SynapseRack's documented single direct-output selector. Multiple physical displays not retested here. | verified: [Projector](https://synapserack.com/en/docs/projector/); `crates/protocol/src/lib.rs:1680`; `app/src-tauri/src/main.rs:81318`; `target/qa/synapserack-features/06-output-menu-0.png` |
| verified: Projection mapping | verified (UI): SynapseMapper panel with output size and display Off. documented-only: source rectangles, corner-pin, perspective, warp-grid quads. | verified (source): output geometry, edge blend/gamma/black level, polygon/bitmap masks and lens/keystone calibration. | unknown: Both cover projection workflows; equivalence of arbitrary curved quad meshes and per-output calibration is not established. | verified: [SynapseMapper](https://synapserack.com/en/docs/window/synapse-mapper/); `crates/video/src/lib.rs:8750`; `crates/video/src/lib.rs:8793`; `crates/video/src/lib.rs:13872`; `target/qa/synapserack-features/11-synapse-mapper-0.png` |
| verified: Spout / NDI / Syphon | documented-only: receive/send Spout on Windows, Syphon on Mac, NDI on either; master local send always available. Spout/NDI source rows observed. | verified (source): Windows x64 Spout CPU-pixel transport is default; NDI code is feature-gated and ordinary packaging rejects it without licensed overlay; Syphon remains NotBuilt. | documented-only: SynapseRack offers documented macOS Syphon and ordinary NDI workflows that Syndocal's normal package cannot claim. No live peer tested in either. | verified: [Video exchange](https://synapserack.com/en/docs/load-spout/); `crates/video/src/lib.rs:3220`; `app/src-tauri/Cargo.toml:13`; `app/scripts/check-release-metadata.mjs:192` |

## Audio-reactive and BPM/tempo

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Audio-driven parameters | documented-only: device-selected loudness/Bass/Mid/Treble/Onset/Hit nodes feed arbitrary graph parameters. AudioReactive tile observed. | verified (source): RMS/peak, log bands, spectral flux/centroid/density, kick/snare/onset and confidence feed shared lighting/video modulation. | unknown: Both have audio-reactive paths; detector behavior and device compatibility were not compared. | verified: [AudioReactive](https://synapserack.com/en/docs/audio-reactive/); `crates/audio/src/live_features.rs:102`; `crates/protocol/src/lib.rs:6069`; `target/qa/synapserack-features/14-audio-node-catalog-0.png` |
| verified: Tempo and automatic BPM | verified (UI): Global Tempo/TAP and step sequencers. documented-only: Link and MIDI clock output; new BPMDetect catalog lists BPM/Beat/Confidence. | verified (source/UI): BPM/Tap, musical sync, live-audio BPM estimate/confidence, MIDI clock input. unknown: real network Ableton Link interop; injection seam alone is insufficient. | unknown: SynapseRack's older Beat Sync guide denies automatic audio BPM while current BPMDetect reference lists it. Do not claim automatic BPM exclusive to Syndocal. | verified: [Global Tempo](https://synapserack.com/en/docs/window/global-tempo/); [Link](https://synapserack.com/en/docs/nodes/tempo/AbletonLink/); [BPMDetect](https://synapserack.com/ja/docs/nodes/audio/BPMDetect/); [Beat Sync](https://synapserack.com/en/docs/beat-sync/); `crates/audio/src/live_features.rs:137`; `app/src-tauri/src/main.rs:29785` |

## MIDI/OSC/DMX/timecode control

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: MIDI / OSC assignment | verified (UI): MIDI devices, OSC switches/ports, mapping menu. documented-only: click-to-learn MIDI, OSC assignment and typed I/O nodes. | verified (source): MIDI Learn, mapped OSC input/learn and shared action targets; source-specific controls. | unknown: Controller feedback and full mapping-target parity not exercised. SynapseRack documents channel/number mapping without device identity. | verified: [MIDI](https://synapserack.com/en/docs/midi-mapping/); [OSC](https://synapserack.com/en/docs/osc-input-output/); `crates/io/src/midi.rs:768`; `crates/io/src/osc.rs:336`; `target/qa/synapserack-features/09-osc-settings-0.png`; `target/qa/synapserack-features/10-midi-settings-0.png` |
| verified: Lighting / DMX | documented-only: OFL Fixture, Universe merge, Art-Net sender and simulator. OFL Pan/Tilt fine, Amber and UV currently output zero. | verified (source): fixture/cue lighting engine, Art-Net/sACN input, USB-DMX/Art-Net venue-output policy. | documented-only: Syndocal supplies an integrated lighting show workflow; SynapseRack also has real DMX concepts, so it must not be labelled video-only. Physical DMX not exercised. | verified: [OFL Fixture](https://synapserack.com/en/docs/nodes/io/ofl_fixture/); [ArtNet](https://synapserack.com/ja/docs/nodes/io/artnet_output/); `crates/io/src/dmx_input.rs:152`; `README.md:19`; `crates/protocol/src/lib.rs:3721` |
| verified: MIDI clock / MTC / LTC | documented-only: MIDI clock output; guide explicitly says MIDI clock receive is unavailable. unknown: MTC/LTC. | verified (source): MIDI clock receive plus MTC input/output and LTC capture/decode paths. | documented-only: MIDI clock receive is a concrete Syndocal difference. unknown: MTC/LTC competitor parity, not a proven absence. | verified: [Beat Sync](https://synapserack.com/en/docs/beat-sync/); [MIDI Clock](https://synapserack.com/en/docs/nodes/io/midi_clock_out/); `crates/io/src/midi.rs:126`; `app/src-tauri/src/mtc_output.rs:85`; `app/src-tauri/src/ltc_input.rs:336` |

## Recording

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: File recording | documented-only: FileRecorder accepts a graph image and writes MP4. | verified (source): one selected output per active recording worker; H.264/libx264 and optional AAC; atomic final publication preserves prior artifact on failure. | unknown: SynapseRack audio/codec/atomic-publication parity not established. No recording performed in this comparison. | verified: [File Recorder](https://synapserack.com/en/docs/nodes/record/FileRecorder/); `app/src-tauri/src/video_recording.rs:117`; `app/src-tauri/src/recording_artifact.rs:248`; `qa/RECORDING_SINGLE_OWNER_PLAN_2026-10-14.md:20` |
| verified: Retrospective buffer / live resampling | documented-only: buffer-to-file DumpSource/DumpTrigger and granular buffer playback. | unknown: equivalent retrospective capture or granular video buffer not established; ordinary output recording is not that feature. | unknown: Candidate creative gap, pending targeted Syndocal scope confirmation. | verified: [File Recorder](https://synapserack.com/en/docs/nodes/record/FileRecorder/); [Buffer Granular](https://synapserack.com/en/docs/nodes/record/BufferGranular/); `app/src-tauri/src/video_recording.rs:155` |

## Media formats and codecs

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: File codecs and images | documented-only: HAP/Alpha/Q, H.264, ProRes and other OS-dependent codecs; detailed guide limits images to PNG/JPG and rejects Windows HEVC/VP8/VP9. | verified (source): libav regression decodes H.264/HEVC/ProRes; built-in HAP and PNG/JPEG paths. Actual HAP/H.264 displayed in Part C. | documented-only: Syndocal has evidenced Windows HEVC support where SynapseRack's detailed guide rejects it. Broader codec parity unknown. | verified: [Detailed media guide](https://synapserack.com/en/docs/load-video/); `crates/video/src/libav_decoder.rs:2219`; `crates/video/src/lib.rs:3148`; `crates/video/src/lib.rs:3194` |
| verified: Camera / screen / live sources | documented-only: webcam/window capture nodes and external sender inputs. | verified (source): Camera/ScreenCapture/Ndi/Spout/Syphon source model, with explicit backend availability and failures. Syphon enum does not imply a backend. | unknown: Device/permission/reconnect parity. No camera, capture card or external video sender opened for this task. | verified: [Official product](https://synapserack.com/en/); `crates/protocol/src/lib.rs:551`; `crates/video/src/lib.rs:3220`; `qa/VIDEO_LIVE_SOURCES_CHECKPOINT_2026-09-13.md:60` |

## Automation and timeline

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Sequencing / show timeline | verified (UI): Step Sequencer and tempo panels. documented-only: sequencer bool outputs drive arbitrary graph controls. unknown: unified show timeline / cue stack. | verified (source/UI): shared lighting/video/audio timeline, cue prewait/follow/fades, video/output/graph cue targets and authored automation. | unknown: Syndocal's show timeline is established; no equivalent SynapseRack workflow was verified. Node sequencing is not proof of a cue-stack timeline. | verified: [Step Sequencer](https://synapserack.com/en/docs/nodes/tempo/StepSequencer/); `crates/protocol/src/lib.rs:3721`; `crates/protocol/src/lib.rs:3969`; `target/qa/synapserack-features/syndocal-03-timeline-2.png` |
| verified: Autonomous variation | documented-only: LFO/logic/sequencer patches can automate switches/effects. | verified (source): AutoVJ runtime with deterministic actions, manual Take precedence and blackout retention. UI exposes Auto VJ OFF. | unknown: Similar outcomes may be patched in SynapseRack; no equivalent complete AutoVJ policy was verified. | verified: [Four-layer tutorial](https://synapserack.com/en/docs/tutorial/4layer/); `crates/engine/src/lib.rs:33221`; `crates/engine/src/lib.rs:69334`; `crates/engine/src/lib.rs:69822`; `crates/engine/src/lib.rs:69902`; `target/qa/synapserack-features/syndocal-02-video-2.png` |

## Show safety and redundancy

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Blackout / source-loss policy | documented-only: output toggle; lost external sender freezes last frame without warning and reconnects by name. | verified (source/UI): separate DMX/Video/All blackouts; output admission re-proves freshness, authority, project/output identity; faulted/stale authority fails closed. | documented-only: Distinct policies, not a safety ranking. Last-frame hold and fail-closed admission serve different operator intentions. | verified: [Video exchange](https://synapserack.com/en/docs/load-spout/); [Projector](https://synapserack.com/en/docs/projector/); `crates/video/src/lib.rs:853`; `crates/video/src/lib.rs:904`; `app/src-tauri/src/main.rs:81318`; `target/qa/synapserack-features/syndocal-01-main-2.png` |
| verified: Standby / fencing / recovery | verified (UI): autosave-recovery prompt. unknown: paired show clock, output-ownership fencing, split-brain or standby takeover. | verified (source): authenticated ShowClock, stale latch/manual rearm, output revocation, checksum/generation standby publication and split-brain checks. | unknown: Potential Syndocal distinction; competitor redundancy not proven absent. Two-PC/hardware takeover remains separate acceptance. | verified: `target/qa/synapserack-features/01-demo-recovery-0.png`; `crates/protocol/src/show_clock_runtime.rs:186`; `app/src-tauri/src/show_clock_ipc.rs:1311`; `app/src-tauri/src/main.rs:63693`; `app/src-tauri/src/main.rs:64132`; `qa/RECORDING_SINGLE_OWNER_PLAN_2026-10-14.md:151` |

## UI and workflow

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Performance workspace | verified (UI): movable internal panels, node canvas, MediaBrowser, tempo and per-layer transports. documented-only: WidgetPanel custom layout and F10 detach. | verified (UI/source): Setup/Edit/Control, Lighting/Video/Both/Timeline, persistent lower workspace, Preview/Program, media properties and clip bank. | documented-only: SynapseRack offers user-built performance panels; Syndocal provides a prescribed integrated lighting/video desk. Arbitrary widget-layout parity unknown. | verified: [Widget Panel](https://synapserack.com/en/docs/widget-panel/); [Projector](https://synapserack.com/en/docs/projector/); `app/src/uiModes.ts:10`; `target/qa/synapserack-features/03-demo-ready-0.png`; `target/qa/synapserack-features/syndocal-02-video-2.png` |
| verified: Reusable tools / extension | verified (UI): Apps and MCP Server menu entries. documented-only: HTML/JS Apps SDK, graph groups and local MCP graph editing. | verified (source): reusable FX presets, bounded modulation graph and repository native MCP integration. unknown: equivalent embedded HTML app SDK. | unknown: SynapseRack has documented creative extensions; equal plugin-host capability was not established for Syndocal. No MCP server or new plugin was enabled here. | verified: [Apps SDK](https://synapserack.com/en/docs/apps-sdk/); [MCP](https://synapserack.com/en/docs/mcp/); `tools/syndocal-mcp/server.mjs:105`; `tools/syndocal-mcp/server.mjs:227`; `crates/protocol/src/lib.rs:1000`; `crates/protocol/src/lib.rs:6052`; `target/qa/synapserack-features/04-window-menu-0.png` |

## Licensing limits of the Demo

| Feature | SynapseRack | Syndocal | Gap | Evidence |
| --- | --- | --- | --- | --- |
| verified: Demo layer limits | verified (UI): Demo 3.3.0. documented-only: maximum 4 layers / 2 group layers; automatic excess removal. No expiry/watermark per official site. Limit enforcement not exercised. | verified (source): MIT source grant; no equivalent commercial Demo tier claimed. Capacity is still bounded by product validation/resources. | documented-only: SynapseRack Demo caps layer topology; do not turn Syndocal's source licence into an unlimited-capacity claim. | verified: [FAQ](https://synapserack.com/en/docs/faq/); [Official Demo terms](https://synapserack.com/en/); `LICENSE:1`; `target/qa/synapserack-features/03-demo-ready-0.png` |
| verified: Runtime/package restrictions | documented-only: activation uses vendor authentication; not attempted. Vendor pages describe plans, not a licence audit here. | verified (source): default libav/Spout/ASIO loader; separately licensed ASIO bridge excluded from normal bundle; NDI requires licensed overlay. | verified: Installed/runtime availability must be checked independently of the source licence or a visible route selector. | verified: `app/src-tauri/Cargo.toml:8`; `qa/ASIO_SDK_PIN.json:12`; `app/scripts/windows-runtime-inventory.mjs:502`; `app/scripts/check-release-metadata.mjs:192`; [FAQ](https://synapserack.com/en/docs/faq/) |

## Evidence boundaries and validation

- Inspected repository code alongside QA acceptance notes; older README claims
  about eight-stage FX, non-default ASIO loader and retired workspace names were
  not used as current capability authority. Current source says 32 stages and
  a default ASIO loader with separate bridge payload.
- Official-page conflict: FAQ lists more import extensions than the detailed
  import guide. The table follows the detailed guide and does not claim GIF,
  TIFF or .hap import. Windows codec claims remain documented-only for SynapseRack.
- BPMDetect's current Japanese reference lists audio BPM/beat/confidence while
  the older Beat Sync/AudioReactive guides deny automatic BPM. The installed
  Audio category was inspected without creating a device-owning node; no
  detection claim was tested. Do not prioritize a gap from the old guide alone.
- Some English node URLs returned cache misses; direct fetch returned HTTP 403.
  Japanese official BPMDetect/ArtNet references were readable and used instead.
- No physical DMX, camera, ASIO, external NDI/Spout/Syphon, MIDI feedback,
  network Link, timecode or two-PC redundancy test was performed for Part D.
- Part D validation: all 13 requested areas, 29 five-column feature rows,
  every data-cell status marker, repository line ranges, screenshot existence
  and affected relative Markdown links checked; `git diff --check`.
  Independent source review corrected group-image wording and refined citations.
  Product build/tests are Part B/C evidence, not additional feature acceptance. Document remains uncommitted.

- Retain Syndocal's explicit multiple-output identities, unified show timeline,
  timecode paths and fail-closed ownership policy. Unknown competitor redundancy
  is not a reason to remove those safeguards or claim a universal winner.

## Gaps worth closing for a VJ show (priority; implementation not authorized here)

1. **P1 - Creative basics:** add the missing blend operations and a bounded
   pure-generator source path. The blend menu/source enum prove a small,
   concrete gap; media-free gradients/text/noise cover common live fallback
   and texture needs without requiring a complete image-graph editor first.
2. **P1 - Shader compatibility:** define supported ISF multipass/feedback/import
   use cases before widening them. Current explicit rejections block whole
   creative classes; preserve bounded resources and observable failures.
3. **P2 - Reusable image tools and independent group blend/transform:** consider a
   scoped image graph or independently blended group result after the basics.
   Syndocal already composites group members for group FX; the gap is
   independent group blend/opacity/transform and nested image topology, not
   absence of group-image FX. Its modulation graph is a different tool.
4. **P2 - External VJ handoff:** finish an approved NDI distribution path and,
   for Mac-based shows, Syphon implementation/acceptance. The normal-package
   and NotBuilt boundaries are concrete; actual sender/receiver tests are needed.
5. **P3 - Loop-seam blend and retrospective video buffer:** confirm operator
   demand and current Syndocal scope first. The vendor documents useful VJ
   tools, but equivalent Syndocal support was not established, so these remain
   investigation candidates rather than confirmed missing implementations.
