# Lighting Competitive Audit

Updated: 2026-07-13

## Verdict

Syndocal is already a serious unified show-control application, but it is not yet defensible to claim that it exceeds Daslight 5 as a lighting product. It leads in some technical/operational areas—lighting and video in one cue/timeline, open Art-Net/sACN/serial routing, RDM, active/standby recovery, diagnostics and automated reliability evidence—but Daslight 5 currently leads in lighting-specific effect breadth, fixture-catalog maturity, editable Touch surfaces, hardware standalone playback, detachable workspaces and public product maturity.

The official comparison target for this audit is Daslight 5 version 5.0.6.2, listed for Windows and macOS on 2025-09-05. The product page advertises up to 100 universes, 20,000+ fixture profiles, 1,000 banks, seven live FX engine families with unlimited stacking, Super Scene timelines, Live Mixer, custom Touch surfaces, multiple mobile remotes, 3D visualization, detachable dual-monitor windows, hardware standalone playback and password locking.

Primary sources:

- <https://daslight.com/en/daslight5?country=ie>
- <https://www.daslight.com/en/download.htm>

Local visual comparison on 2026-07-13 used the installed Daslight 5 with `Shinkan2026.dvc` loaded. Its Control desk keeps group scene columns, the selected FX inspector, fixture-group tabs, the 2D stage and per-group Live Mixer visible together. Its Setup desk similarly keeps the fixture library, patch grid, group/stage map and dimmer/movement limits on one plane. Syndocal's new fixed full-window desks now match that task-density direction without copying Daslight's controls or colour system, but its lighting workflow still separates Live Edit, Timeline and several scene modifiers more than the loaded Daslight project does.

## Current comparison

| Area | Syndocal evidence | Daslight 5 official evidence | Assessment |
|---|---|---|---|
| Patch and profiles | GDTF import/cache, GDTF Share search/download, custom fixture builder, patch grid, bulk patch, footprint/conflict validation, fixture/group limits | 20,000+ cloud profiles, instant matrix/strip fixtures, patch/order/group setup and per-fixture limits | Syndocal is technically capable; Daslight leads in catalog size and established profile workflow |
| Universe scale | Art-Net 0-32767 and sACN 1-63999 addressing, up to 256 configured routes; 128 distinct enabled Art-Net universes succeed in one production engine tick and the 128-route setup UI passes all reference viewports | Up to 100 DMX universes | Syndocal now exceeds the declared configuration scale; a sustained 128-universe real-node soak is still required for a performance win |
| Programming | Programmer/Blind, group and fixture attribute editing, palettes, tracking/block behavior, cue parts, IFCB timing, follow, mark/MIB, HTP/LTP and playback executors | Manual fixture control, palettes, blind scene editing, scene priorities/fades/steps and live editing | Broad parity in core programming concepts; direct operator comparison remains required |
| Effects | Ordered LFO/Position Wave/Color stack with reorder/bypass/clone/preset, BPM sync and graph-driven audio; independent Color runtime provides 2-8 stops, Cycle/Bounce/Sequence/seeded Random, RGB/HSV interpolation, fixture spread and RGB/RGBW/CMY/HSV/colored-wheel output; 13-preset library is filterable as Colour, Chaser, Move, Value, Curve, Mappings and Colour Mappings | Seven live engines (Colour, Chaser, Move, Value, Curve, Mappings, Colour Mappings), multiple types per engine and unlimited stack | The former single-attribute colour alias is closed, but Daslight still leads overall: six Syndocal library families still share LFO/Position Wave, normal Effect state is not Cue-recalled, calibrated multi-emitter colour and image/video mapping are absent, and unlimited practical stacking lacks release/venue proof |
| Timeline/show creation | Cue events plus lighting/video automation, step/linear/Bezier keys, imported audio waveform/FFT, shared clock, LTC/MTC and linked node-graph state | Super Scene combines source scenes and updates when sources change; audio import, dimmer/phasing automation, loop/jump | Syndocal leads in unified A/V targets; Daslight leads in source-linked scene composition and scene-oriented workflow |
| Live playback | Cue Lists, Back/GO, 64 executors across 99 pages x 16 slots, group submasters, grand master, blackout, highlight/solo/park, cue and video controls | Live Mixer group dimmer/HUE/saturation/flash/strobe/blackout/solo; per-scene dimmer/speed/phase/size/direction/segments/GO/BPM | Both are viable; Daslight exposes more scene-specific live modifiers while Syndocal exposes stronger console-style cue control |
| Sync and control I/O | MIDI Clock/MTC, Ableton Link command path, LTC, tap/manual BPM, audio FFT, MIDI/OSC/DMX learn and WebSocket remote | Tap, MIDI Clock, Ableton Link, line-in audio; keyboard/MIDI/DMX one-click mapping and OSC | Feature coverage is close; same-controller step-count and response tests are still missing |
| Touch and remote | Fixed responsive Touch desk, LAN web remote, multiple clients, output selection/faders and cue/video controls | Custom Touch surfaces with buttons/faders/dials/wheels/color/XY, mobile apps and multiple instances | Daslight leads because its surface is operator-configurable |
| Visualization | Interactive 2D stage/map and a 3D render-payload/model/beam engine with GLB/GLTF/OBJ/3DS/Collada ingestion; no polished standalone 3D operator viewport is yet evidenced | 3D visualizer software | Daslight leads in delivered operator workflow; Syndocal has deeper internal asset/render groundwork than its current UI exposes |
| Workspace | Dedicated one-screen Edit/Live/VJ desks, UI scale/locale and persisted tabs/layout state | Detachable windows for dual-monitor arrangements | Daslight leads for workstation customization; Syndocal leads only where a fixed low-clutter desk is preferable |
| Installation/lock/standalone | Project backups, crash recovery, full output disarm on load, active/standby checkpointing and diagnostics; no hardware scene write or operator password mode | Hardware standalone scene write and full/partial password lock | Split: Syndocal leads in software failover evidence; Daslight leads for installations without a PC and access control |
| Performance evidence | Two one-hour internal engine soaks, command/DMX latency telemetry, large-show tests at 200 fixtures/8 universes/100 cues, a deterministic 200-fixture/64-Color-effect cache/evaluation regression and virtualized 2,000-fixture UI | No comparable official benchmark values are published | No fair winner; current Syndocal evidence does not cover a release-build Color p95/p99/soak, 100+ live universes or same-host Daslight measurement |
| Product maturity | v1.0 release pipeline and explicit software/cross-app/hardware acceptance separation | Established commercial hardware/software ecosystem and maintained manuals/mobile apps | Daslight leads in field maturity and ecosystem evidence |

## Priority implementation gates

1. Sustain 128 distinct universes through the one-hour telemetry budget on a real Art-Net/sACN node and capture packet loss/jitter. Configuration, engine tick and UI containment are complete at 128; external sustained evidence remains.
2. Continue beyond the completed independent Color engine: add Cue Effect recall, then independent Chaser, Move, Curve/Value and image/video Mapping runtimes with a fast visual rack, virtualized practical stacking, reorder, bypass, clone, preset and live phase/size/speed controls.
3. Add source-linked Scene Blocks to the timeline so edits to a source cue/scene propagate to every placed instance without destructive duplication; add loop count and jump targets.
4. Build an editable Touch layout designer with buttons, faders, encoders, color/XY pads, snap/grid, named layouts and multi-client permissions.
5. Expose the existing 3D scene/model/beam engine as a real-time operator visualizer with camera navigation, focus/selection and performance telemetry.
6. Add named detachable workspaces/windows for dual-monitor programming and live operation.
7. Add full/partial operator lock with credential-safe project policy. Treat hardware standalone playback as a separate hardware-product workstream rather than pretending software alone closes it.
8. Build a curated fixture onboarding/catalog experience on top of GDTF Share: search facets, favorites, cached/offline availability, profile health, missing-profile repair and verified common-rig packs.
9. Run same-host cold start, project-open, control-to-DMX, UI input latency, CPU/GPU/memory and 128-universe soak tests. If Daslight can be legally installed on the comparison host, capture the same measurements there.
10. Broaden serial DMX toward the QLC+ device class: build the maintained VID/PID catalog on top of the completed conservative USB identity recommendation, then add Pro Mk2/Ultra Pro dual output, verified device profiles and serial DMX input. Existing ENTTEC Pro-compatible, DMXKing ultraDMX and Open DMX output remains the baseline; see `qa/SERIAL_DMX_COMPATIBILITY.md`.
11. Complete real controller, DMX node, serial interface, fixture rig, projector and operator rehearsal evidence before any superiority language is published.

## Fixed in this audit pass

- Raised the validated simultaneous DMX route limit from 64 to 256.
- Added a 128-universe validation boundary test and an engine state/tick test covering all 128 route telemetry entries.
- Added an enabled 128-universe Art-Net production tick test; all 128 sends complete with zero reported failures.
- Seeded 128 routes into the browser acceptance fixture. Setup > DMX now must render the complete route set and remain contained at 1280x720, 1366x768 and 2048x1152.
- Replaced the unclassified effect samples with an accessible seven-family rail and 13 preset cards. Added Curve Saw and migrated target-required Colour Spectrum/Colour Chase to the independent Color runtime. Browser acceptance performs stop add/remove plus Random/HSV-longest edits at all reference sizes; `qa/LIGHTING_FX_ACCEPTANCE.md` records the explicit non-parity boundary.

## Claim policy

Passing internal unit/UI tests proves implementation quality, not competitive superiority. “Exceeds Daslight 5” requires all applicable gates above, a documented weighted scorecard with no critical losses, and repeatable real-rig evidence. Hardware standalone parity may be declared not applicable only if Syndocal is deliberately positioned as a software-only product; it cannot silently be counted as a win.
