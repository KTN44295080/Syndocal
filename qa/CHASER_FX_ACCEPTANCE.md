# Chaser FX Acceptance

Updated: 2026-07-13

## Reference boundary

The installed Daslight 5 project `Shinkan2026.dvc` was used for local context, but the earlier `Moving Chaser Rnd` screen was a Dimmer effect and is not valid Chaser or Move evidence. The competitive Chaser boundary therefore comes from the official Daslight 5 manual, which documents multiple feature ranges, fixture-index order, beam selection, export/import, conversion to steps and live transport modifiers. Syndocal uses those workflow categories as acceptance input without copying Daslight's layout, colour system, names or undocumented variant algorithms. Exact Daslight variant semantics remain unclaimed until black-box output measurements exist.

Primary references:

- <https://daslight.com/en/daslight5?country=ie>
- <https://eu-litterature.n-g.co/Release/daslight_5_manual_en.pdf>

## Implemented software path

- Independent `EffectKind::Chaser` protocol, engine runtime, Tauri commands, preset/project body and dedicated inspector.
- Two to 256 ordered steps containing explicit fixtures, hierarchical groups or gaps; each step stores an independent level.
- One to 16 feature attributes, each with independent low/high values and per-target compatibility resolution.
- Forward, Reverse, Bounce and deterministic seeded Random traversal.
- One to 16 Wings, one to 64 active pixels, duty cycle, overlap/fading, size, phase and fixture spread.
- Free-time and beat-synced step clocks; Override, Add and Multiply stack blend modes.
- Patch-order group expansion for the built-in fixture-index Chase recipe, explicit step preservation for loaded projects/presets, and single-fixture off-gap behavior.
- Publication-acknowledged Add/Update with targeted rollback, wrong-kind update rejection, Cue Effect Recall, duplicate/relabel/preset round-trip and legacy `.sdc` defaults.
- Precompiled fixture-step matrices and feature bindings plus a per-fixture/effect/tick normalized-level cache; no allocation in the per-attribute evaluation loop.

## Automated acceptance

- Protocol serde and validation cover bounds, defaults and legacy bodies.
- Engine tests cover all traversal modes, active-width semantics at Bounce turnarounds, deterministic Random order shared with the UI, Wings, duty/overlap, gaps, feature scaling, mixed compatibility, stack modes, group changes, dormant targets, project round-trip, publication rollback and wrong-kind protection.
- Tauri tests cover add/update, target overrides, fixture/group hierarchy, mixed feature selections, strict project validation, Cue scopes and preset round-trip.
- `check:chaser-draft` covers draft normalization, traversal/preview order, canonical duplicate attributes and deterministic randomization.
- The release browser gate passed at the primary 1920x1080 operational/visual viewport, the 1280x720 and 1366x768 containment fallbacks and the 2048x1152 extended ceiling. It verifies 6 steps, 2 features, Reverse, active width, Size/Fading changes, step replacement, fixed action regions and zero horizontal overflow.
- Virtualization acceptance keeps the 500-effect Live Rack to a 10-row page and the Chaser step editor to an 8-row page.
- The maximum-control debug regression evaluates a 10-effect stack with 16 features, 64 fixtures, maximum 64-pixel width and 16 Wings over 10 frames under a five-second guard. A separate venue regression evaluates 200 fixtures × 64 Chaser effects × 2 features over 10 frames and verifies the per-tick cache; focused release execution on this host completes in roughly 0.05 seconds. These are regression budgets, not cross-product benchmarks.

## External acceptance still required

Software tests cannot establish physical direction, timing or visual quality. Production sign-off still requires:

1. Representative dimmer, RGB/RGBW, mover and mixed-capability fixture groups over Art-Net, sACN and serial DMX.
2. Forward/Reverse/Bounce/Random order capture against the patched fixture order, including gaps, Wings and more than one active pixel.
3. Free and beat-synced timing capture with p95/p99 step jitter and operator-visible response from editor change to DMX output.
4. One-hour mixed Color/Chaser stack soak with CPU, memory, command latency, packet loss and frame/DMX drop telemetry.
5. Side-by-side operator rehearsal against the installed Daslight project using the same fixture order and controller.

Until those gates pass, this tranche may be called an independent production Chaser implementation, but not proof that Syndocal exceeds Daslight 5 overall.
