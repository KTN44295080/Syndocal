# H5 Control Both overview checkpoint — 2026-09-14

## Checkpoint identity

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `97c20c94` (`Implement lease-bound video clip launch control`)
- Scope: current-source Control `Both` presentation/interaction slice and the focused browser-gate repair needed after the current full Video desk replaced the retired library-only projection.
- This is a bounded H5 continuation checkpoint. It does not close `UI-H5-CONTROL-001`, claim native button-by-button accessibility, or claim physical output/venue acceptance.
- No product-version bump is required for this internal checkpoint.

## Implementation

- Added `ControlBothPanel` as one combined upper overview. It exposes Lighting current/next cue state, Back/GO/Release, Lighting Master, DMX/All blackout, Video Preview/Program truth, Video Master, clip/selection/recording truth, Video blackout, output counts, timecode, and direct routes to the detailed Lighting and Video domains.
- Added the fourth top-level Control domain: `Lighting / Video / Both / Timeline`. Both intentionally does not duplicate the dense Video desk or move recording into `OutputControl`; detailed clip import/Take/Launch and recording remain owned by the existing Video surface and recording path.
- Routed Both to the existing lease-bound Lighting/Video master and blackout operations. The existing generation/fence and fail-closed authority paths remain canonical.
- Updated workspace mode persistence and keyboard/tab association contracts. Both has no conflicting single-letter shortcut; roving focus reaches all four tabs.
- Updated the focused Control browser checker to observe the current `videoControlPanelMixer`/bounded Media Library disclosure contract rather than the retired `videoControlPanelLibrary` projection, and to verify the Both overview.

## Verification evidence

All commands were run from the repository root and exit status was checked.

| Area | Command/result |
| --- | --- |
| TypeScript | `pnpm.cmd --dir app exec tsc --noEmit` — PASS. |
| Frontend build | `pnpm.cmd --dir app run build` — PASS; 358 modules transformed. Vite emitted only the existing large-chunk advisory. |
| Static release chain | `pnpm.cmd --dir app run check:release:static` — PASS; completion ledger `23 Open + 8 Deferred + 27 Complete = 58`, Q1/Q4 `58/58` Flow references and `57` evidence records before this checkpoint, all listed static contracts passed. |
| Shortcut contract | `pnpm.cmd --dir app run check:project-shortcuts` — PASS; 15,552 matrix cases, source manifest byte-exact. |
| Workspace/operator contract | `pnpm.cmd --dir app run check:workspace-operator` — PASS; 40 assertions. |
| Renderer routing | `pnpm.cmd --dir app run check:frontend-command-routing` — PASS; 133 renderer mutations, 31 server-authoritative mutations, 28 raw dispatches, 479 facade dispatches. |
| Dedicated rendered browser gate | `CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe; SYNDOCAL_CONTROL_VIEWPORT=1920x1080; node scripts/check-control-upper-workspaces-browser.mjs` — PASS. Lighting, Video Media Library/Import Media, Both, and Timeline were traversed in one run; CDP runtime exceptions, console errors/warnings, log errors/warnings, and harness errors were all 0. Both assertions passed for two domain cards, Preview/Program, two masters, two header routes, Back/GO/Release, footer truth, viewport containment, and zero document/app scroll. |
| Visual browser check | Both screenshot visually inspected at 1920×1080: `C:\TEMP\syndocal-control-ui-checkpoints\control-both-1920x1080.png`. The two cards are side-by-side and the existing lower workspace remains visible. |
| Fixed-CDP viewport gate | `check-viewport-containment.mjs --control-mode-surface-only` was attempted with a fresh CDP port and Chrome path but failed closed before the Both phase: `timeline-layered` renderer `Page.navigate` did not reply within 15 seconds. This is not counted as Both evidence. |
| Windows native build | `vcvars64.bat -vcvars_ver=14.44` with the exact Build Tools 14.44.35207 linker, followed by `pnpm.cmd --dir app tauri build --no-bundle` — PASS; `target/release/syndocal.exe` built. The wrapper printed the pinned absolute linker and matching `where.exe link.exe`. |
| Exact native process smoke | Launched only `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe`; one matching PID, title `Syndocal`, nonzero window handle, `Responding=True`; stopped only that exact process; remaining exact-path count `0`. |
| Native artifact identity | SHA-256: `16A853E5F1AB38C97918854B436A2CC8A9B15CF8AA72732BA16DDB2C8A7456AC`. Unsigned current-source process-smoke artifact, not a published release. |

## Remaining boundary and next action

`UI-H5-CONTROL-001` remains **Open**. The browser and native process evidence establish the current-source Both surface and the responsive native executable only. Native button-by-button interaction was not performed in this environment, and no NVDA/High Contrast/DPI/IME/reduced-motion matrix, physical DMX/Art-Net/sACN/USB/RDM/MIDI/OSC/video/display output, external client, two-machine, venue, soak, signing, or publication acceptance is claimed.

The next safe H5 action is to continue with the next individually owned Control acceptance slice, preserving the same lease-bound authority and recording the exact native/hardware boundary. The full 58-marker ledger remains `23 Open + 8 Deferred + 27 Complete`.
