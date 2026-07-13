# VJ Competitive Audit

Updated: 2026-07-13

## Verdict

Syndocal can run a prepared Windows VJ show, but it is not yet defensible to say that it matches or exceeds SynapseRack as a general-purpose VJ product.

Syndocal is stronger where lighting and video must share cues, timeline, BPM, MIDI/OSC/DMX control, projection mapping, NDI/Spout routes, recording, diagnostics, and fail-safe output behavior. SynapseRack is currently stronger as a pure VJ creation environment: arbitrary node-based tool construction, more than 100 advertised effects, freely arranged/resized panels, a mature content workflow, and public venue adoption.

Primary comparison sources:

- <https://synapserack.com/en/>
- <https://synapserack.com/en/docs/intro/>
- <https://synapserack.com/en/docs/tutorial/4layer>

## Current comparison

| Area | Syndocal evidence | SynapseRack evidence | Assessment |
|---|---|---|---|
| Ready-to-play desk | Dedicated full-window VJ Desk, 12-pad banks, explicit P staging, an independent local-file/still Preview playhead with pause/seek/reverse/loop, routed Program monitor, Cut/Take/Stop, Deck A/B crossfader, audio monitor, recording, live FFT, output and layer decks; an empty show can import up to 64 local videos and atomically create a safe Off/Blackout Program output without opening a window | Startup four-layer template, content browser, layer window, preview, hierarchy and property editor | Syndocal now has a safe one-action first-run path and independent Preview transport; it still lacks richer reusable templates and a content browser |
| Media and outputs | Video/still multi-import, camera/screen/NDI/Spout input, Display/NDI/Spout output, H.264 recording, projection blend/mask | H.264/HAP/image media, Spout/OBS workflow and projector output | Syndocal has broader explicit routing and recording evidence on Windows |
| Effects | Pixelate, blur, glow, edge, color key, transform/color controls, 14 built-in live ISF effects, and one safe imported/built-in single-pass ISF effect per layer | More than 100 advertised real-time effects and HLSL shader import | The basic live library gap is reduced, but SynapseRack still leads materially in breadth and stacking |
| Node authoring | Saved three-stage Source -> Transform -> Output graphs for LFO/position-wave/audio control of lighting/video parameters | General node editor with layer, texture, shader, math, UI, variable and operation modules | SynapseRack leads materially; the products do not currently expose equivalent node freedom |
| Live control | MIDI, OSC, DMX, cue/timeline/BPM, Preview/Program, deck and blackout controls | MIDI, OSC, DMX mapping, keyboard mapping, global tempo, beat sync and operation nodes | Both are viable; Syndocal leads for unified lighting/video cueing |
| Workspace customization | Persists selected workspace/tab/surface/category and UI scale/locale; VJ Desk layout is fixed | Panels can be freely arranged, resized and shown/hidden | SynapseRack leads |
| Performance evidence | Two one-hour engine soaks with zero synthetic drops/send failures; real-GPU 4K BC7 frame; 30-minute recording mux drift 0.0 ms | Official site states GPU acceleration and smooth complex setups; no same-host benchmark is published | No fair winner until representative multi-layer media is tested side by side |
| Operational maturity | Cross-app NDI/Spout and physical camera/MIDI port evidence; remaining hardware/venue gates are explicit | Official site reports 5,000+ downloads and use at multiple named venues | SynapseRack leads in public field validation |

## Fixed in this audit pass

- Added a Tauri desktop capability so application/window events no longer expose `plugin:event|listen not allowed by ACL` in the status line.
- Renamed `Mixer` to `VJ Desk` and made it a dedicated full-window surface; lighting Live Desk, Stage and Faders no longer consume its viewport.
- Added a clear `Import Media` action to the empty Clip Grid and verified that it opens the production multi-select media dialog.
- Updated viewport acceptance so all three reference sizes require the dedicated VJ surface and reject a visible lighting desk in VJ mode.
- Added 14 GPU ISF presets (Invert, Monochrome, Threshold, Posterize, RGB Split, Mirror, Kaleidoscope, Zoom, Rotate, Strobe, Scanlines, Colorize, Vignette and Glitch Shift) to a categorized live selector. Every embedded shader is parsed and compiled by the production ISF preparation path in automated tests; viewport acceptance requires the selector on all three reference sizes.
- Replaced the manual reference snapshot with side-by-side Preview and Program buses. A bounded raw Tauri packet carries backend JPEG frames and timing data; the frontend runs one non-overlapping 10/5 fps loop, pauses when hidden, discards stale responses and reports contention. `qa/VJ_LIVE_MONITOR_ACCEPTANCE.md` records why this is not yet proof of native-output or performance parity.
- Added a first-run action in the empty Clip Grid. File selection and full path validation happen off the IPC thread before mutation; one engine publication barrier creates all stopped file layers and a windowed Display output that remains disabled and blacked out. Cancellation changes nothing, duplicate setup is rejected, a busy snapshot rolls the runtime mutation back, and the exact published snapshot is acknowledged before the frontend commits its single Undo transaction.
- Replaced per-position Libav decoder construction during ordinary forward playback with an eight-layer LRU of sequential sessions and normalized non-zero stream timestamps. B-frame/offset-start oracle parity, seek/reverse reset, EOF drain, invalid-source release, capacity eviction and diagnostics counters are covered. The CLI fallback also retains only one frame per layer and eight layers globally, while scaler reuse and shared decode/render fan-out remain open.
- Separated Preview from the Program project playhead. Explicit `P` staging starts a local File/Still at its in-point and exposes pause, seek, reverse/forward speed, inherited loop bounds and Clear without mutating project state. Cut/Take require the staged layer and transfer its current position/speed to Program. The runtime-only session is cleared across project/history/standby and source-identity changes; a dedicated prefetch-zero renderer and post-render/post-encode generation checks bound cache impact and suppress raced frames.

## Required before parity can be claimed

1. Expand the 14-effect first tranche to a searchable library of at least 100 production-quality effects, including feedback, luma key, displace and noise, then add a multi-effect stack with live-safe reorder, bypass, reset, preset and hold controls. Fourteen single-slot effects are not parity with SynapseRack's advertised breadth.
2. Replace the fixed three-node authoring form with a real graph editor supporting arbitrary nodes, edges, fan-out, reusable graph tools and parameter exposure to the live desk.
3. Expand the safe first-run action into reusable named VJ templates and folder import. The current path creates one Main Program output and selected file layers, but does not preserve a reusable four-layer or venue layout.
4. Add a content browser with folders, search, favorites, missing-media relink and thumbnail health/status.
5. Allow VJ panels to be resized/reordered/hidden and save named desk layouts.
6. Extend independent Preview beyond local File/Still transport where operator rehearsal proves it necessary, including Preview audio scrub and safe live-source semantics. The current Preview deliberately rejects Camera/Screen/NDI/Spout and does not persist a second editable layer state.
7. Run representative HAP Q Alpha/HAP R/ISF packs at 1080p and 4K with multi-layer seek/reverse/loop, both monitor buses, NDI/Spout output, recording and one-hour dropped-frame capture on the target venue GPU. Measure duplicated decoder/cache cost across Preview, Program and output workers rather than inferring performance from synthetic engine soaks.
8. Complete operator rehearsal and public/venue field evidence. Internal synthetic tests do not replace this gate.

Until these are complete, release language must say that Syndocal is a functional unified lighting/VJ application, not that it is perfect or categorically superior to established VJ software.
