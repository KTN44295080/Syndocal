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
