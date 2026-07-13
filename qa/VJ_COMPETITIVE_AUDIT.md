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
| Ready-to-play desk | Dedicated full-window VJ Desk, 12-pad banks, Preview/Cut/Take/Stop, Deck A/B crossfader, audio monitor, recording, live FFT, output and layer decks | Startup four-layer template, content browser, layer window, preview, hierarchy and property editor | Syndocal is operational after setup; it still lacks a first-run VJ template and rich content browser |
| Media and outputs | Video/still multi-import, camera/screen/NDI/Spout input, Display/NDI/Spout output, H.264 recording, projection blend/mask | H.264/HAP/image media, Spout/OBS workflow and projector output | Syndocal has broader explicit routing and recording evidence on Windows |
| Effects | Pixelate, blur, glow, edge, color key, transform/color controls, one imported safe single-pass ISF effect per layer | More than 100 advertised real-time effects and HLSL shader import | SynapseRack leads materially |
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

## Required before parity can be claimed

1. Ship a searchable built-in effect library and multi-effect stack with live-safe bypass/reset/preset/hold controls. The first target is the common VJ set: RGB split, invert, mirror, kaleidoscope, zoom, rotate, strobe, feedback, glitch, posterize, threshold, luma/chroma key, displace, scanline, noise and colorize.
2. Replace the fixed three-node authoring form with a real graph editor supporting arbitrary nodes, edges, fan-out, reusable graph tools and parameter exposure to the live desk.
3. Add a first-run VJ template/wizard that creates Program composition/output and imports a media folder without visiting Setup.
4. Add a content browser with folders, search, favorites, missing-media relink and thumbnail health/status.
5. Allow VJ panels to be resized/reordered/hidden and save named desk layouts.
6. Run representative HAP Q Alpha/HAP R/ISF packs at 1080p and 4K with multi-layer seek/reverse/loop, NDI/Spout output, recording and one-hour dropped-frame capture on the target venue GPU.
7. Complete operator rehearsal and public/venue field evidence. Internal synthetic tests do not replace this gate.

Until these are complete, release language must say that Syndocal is a functional unified lighting/VJ application, not that it is perfect or categorically superior to established VJ software.
