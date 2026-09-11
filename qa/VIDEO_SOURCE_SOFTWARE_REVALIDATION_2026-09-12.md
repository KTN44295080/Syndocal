# Video source software revalidation — 2026-09-12

This checkpoint advances only the deterministic source/UI software slice of
`COV-VIDEO-SOURCE-001` on current `main` at
`3d6b0febb85804507953d4a107044271b8460e5c`. No product source was changed.

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:camera-input` | PASS — camera profile UI and transport status polling contracts |
| `pnpm --dir app run check:video-output-window-runtime` | PASS — controller, exact-Both recovery, receipt rejection, single-flight, incarnation reducer, no legacy invoke |
| `pnpm --dir app run check:video-output-window-observation` | PASS — output-window observation contract |
| `node app/scripts/check-timeline-source-shelf-contract.mjs` | PASS — authoritative Bank/Scene source-shelf contract |

The automated proof now records these four focused checks as passing. This is
not camera, capture, NDI, Spout, Syphon, generator, permission, reconnect,
GPU, native-window, or physical-device acceptance. No silent fallback or
physical availability claim is made; `COV-VIDEO-SOURCE-001` remains `In
progress` for those boundaries.

## Current-main rerun — 2026-09-12

The four deterministic checks were rerun against current `main` at
`0ce12323a313290cadeb7c2010b5e6d5857b471f`. All passed again:

- `pnpm.cmd --dir app run check:camera-input`;
- `pnpm.cmd --dir app run check:video-output-window-runtime`;
- `pnpm.cmd --dir app run check:video-output-window-observation`;
- `node app/scripts/check-timeline-source-shelf-contract.mjs`.

This rerun confirms the software contracts only. It does not promote the row
to native or hardware acceptance; the live-source, permission, reconnect,
NDI/Spout/physical, and one-hour matrix boundaries remain open.
