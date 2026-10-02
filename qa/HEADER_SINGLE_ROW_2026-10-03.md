# Single-row header and exact checkout artifact

Branch `codex/showclock-review-20260912`, base `64b08ab3`.

The user rejected a two-row header and asked to reduce its contents. The final
header remains one **42px** row. Back/GO, DMX/VID/ALL blackout, BPM/Tap and Clock
remain directly available. Tools groups fade/timeline transport, clear flags,
MIDI/OSC/DMX Learn, audio input and output details. Workspaces is reached through
the project menu's Workspace Layout section. Existing control, icon and font
sizes are preserved. At the compact boundary the project identity and redundant
Clock prefix are omitted; Clock's complete state remains in its accessible name.
Tools retains active Learn feedback, keyboard activation, outside-click dismissal
and Escape dismissal with focus restored to its trigger.

The saved base reproduced Workspaces overlapping Back and GO at 1280px.
The actual Solid Control route (`touch`) now passes at widths 960, 1100, 1280,
1920, 2560 and 3840: single row, no overlap/overflow, retained hit targets,
safe-blackout/dirty/locked/long-name containment, and hittable Tools controls.
The hit test caught and repaired an ancestor clipping the popup. Transport,
blackout, Learn, tempo, project and Workspaces interactions pass; console errors
are empty. Browser plugin is unavailable; the existing Playwright runtime was
used. This is browser evidence, not native geometry or physical output proof.

Evidence:

- [Before](artifacts/header-before-1280-2026-10-03.png),
  [after](artifacts/header-after-1280-2026-10-03.png),
  [Tools](artifacts/header-tools-1280-2026-10-03.png).
- [Six-width geometry and interactions](artifacts/header-layout-2026-10-03.json).
- `pnpm --dir app run check:workspace-header` passes. Set
  `SYNDOCAL_HEADER_BASELINE_REF=64b08ab3` to also reproduce the saved baseline.
- The existing upper-workspace gate passes all nine configured viewports.
  Its Workspaces assertion now verifies disclosure through the project menu.
- The focused PULSE gate passes, retaining telemetry, stale fail-closed and
  Setup Outputs routing checks. It verifies PULSE in Tools and a 42px title row.
- DMX shortcut checks: 41 assertions; MIDI shortcut checks: 39 assertions.

## Native build identity correction

The first build returned success without updating the intended executable.
`cmd set` used its OEM code page while the wrapper decoded UTF-8, corrupting the
Japanese `CARGO_TARGET_DIR`. A real environment-capture probe reproduced this.
The wrapper now selects code page 65001 before vcvars and `set`. The exact MSVC
14.44.35207 linker pin and PATH-first checks remain enforced. The wrapper checker
passes **249 assertions and 27 hostile mutation fixtures**, including a real
Japanese-path round trip on this Windows host.

The September 29 executable's passive probe is retained as
[stale-artifact evidence](artifacts/stale-checkout-native-window-2026-10-02.json);
it does not establish current Control or MCP behavior. The earlier claim that
the ordinary executable had been refreshed is withdrawn.

The final `pnpm --dir app tauri build --no-bundle` passed and wrote the exact
checkout `target/release/syndocal.exe`. SHA-256:
`29d4bed5fbb4f2967c9e71012040d84ad61e657f3c61ac8c2cd26dafc8da7833`.
Its build completed October 2; the final launch/probe occurred October 3.
[The native observation](artifacts/header-normal-native-2026-10-03.json) pins the
executable/process, verifies exactly one visible responsive maximized Syndocal
window, and proves the native broker rejects an unauthenticated read. The app
remains running. No device/output command or native UI click was issued.

The optimized build reports zero Rust warnings and zero TypeScript diagnostics.
Vite retains the existing one large-chunk advisory (baseline 1, current 1,
delta 0); no warning limit was changed. The build includes protected pre-existing
frontend work, so it is not a clean frozen release candidate.

The earlier normal-profile debug launch was rejected by automatic approval
review with only `blocked by policy` as its reason. The successful final launch
has no debugger. Authenticated MCP acceptance remains evidenced separately on
the isolated QA profile; this passive primary-profile probe does not replace it.

Protected other-owner files remain unstaged, including App, localization,
pairing PIN, DJ runtime checker, AI3 checkpoint and `.vite/`. Self-review is not
claimed as independent review. No completion markers were promoted; the goal
continues with the remaining ledger acceptance requirements.
