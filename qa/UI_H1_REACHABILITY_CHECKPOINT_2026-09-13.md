# UI H1 reachability checkpoint — 2026-09-13

This checkpoint closes `UI-H1-REACHABILITY-001` for the current supported
software surface. The previously unreachable full Video Control surface is now
mounted from Control → Video → Mixer, so the operator path exposes the existing
Preview/Program monitors, clip grid and Take action, layer/effect surface,
output rail, Import Media entry, and recording status without relying on an
internal command or the Edit-only library route.

## Implemented contract

- The Control → Video → Mixer `VideoControlPanel` no longer receives the
  `libraryOnly` projection. Its full control surface is therefore reachable
  from the normal workspace route while the existing library-only branch stays
  available for its dedicated Edit consumer.
- The full Control surface keeps one visible operator route for Import Media,
  clip-bank/Take, Preview/Program, layer/effect inspection, output selection,
  and recording status. No command, backend route, or legacy protocol was
  removed.
- The ISF Event-pulse busy interlock is passed as a live Solid accessor through
  `VideoLayerListPanel` to `VideoIsfEffectPanel`; spreading the prop no longer
  freezes the busy state at its initial value.
- The operator fixture now supplies the mandatory timeline runtime projection,
  owner-registration response, and read-only startup responses required by the
  full Control surface. Its output rail assertion remains fail-closed in the
  browser-only fixture (`state-unchecked`) because that fixture cannot prove a
  native physical window state.

## Verification

The browser plugin was unavailable. The VJ operator acceptance used the local
Vite/Chrome CDP harness and is browser evidence only. Native verification used
the repository Windows procedure with `vcvars64.bat -vcvars_ver=14.44` and the
Build Tools `14.44.35207` x64 linker first in `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:vj-operator` | PASS — 8 cases: en/ja × 1920×1080, 1366×768, 1280×720; all checks passed |
| `pnpm.cmd --dir app run check:frontend-invokes` | PASS — 475 commands |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 480 facade dispatches |
| `pnpm.cmd --dir app run check:empty-states` | PASS |
| `pnpm.cmd --dir app build` | PASS — 356 modules; existing Vite chunk warning only (`App` 501.42 kB), no first-party compiler warning |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — linker pin and `where.exe` preflight passed |
| Exact release executable process smoke | PASS — one exact-path process, one `Syndocal` window, title `Syndocal`, responsive; process stopped after verification |
| `git diff --check` | PASS before checkpoint commit |

Native artifact identity for this checkpoint:

`target/release/syndocal.exe` SHA-256
`38C4E36E28522CCB3D5E458ED55196EB820560945D17670E8B1D2B71C8B5E43D`

## Boundary

This closes the current-source H1 reachability slice and its browser/native
launch evidence. It does not claim native button-by-button interaction,
screen-reader/High Contrast/DPI/IME/reduced-motion acceptance, physical HDMI or
other output, external devices, venue operation, or completion of the H2
shared-shell, H3 Setup/DJ Link, H4 Edit, or H5 Control markers. Those remain in
`COV-UI-SHELL-001` and the corresponding open Flow markers.
