# Syndocal DMX setup UI checkpoint — 2026-09-07

Branch: `codex/syndocal-v1.2`

Source parent before this checkpoint: `6a65ee2`

Product candidate: `1.2.0-alpha.69`

## Scope

Setup → I/O → DMX now presents a compact routine surface: USB-DMX device
selection, `Prepare`, and one status line. The default surface does not require
a fixture, a patch, or a USB-DMX device when the fixed local Art-Net route is
available.

The following details remain reachable under `Advanced` without occupying the
routine surface:

- fixed Art-Net loopback enable action;
- USB-DMX confirmation, S0 arm, and stop controls under `Manual controls`;
- observed and confirmed machine-local device identities;
- route facts and the two logical routes.

The fixed Art-Net probe and its no-send reconciliation action now live in the
DMX protocol diagnostics panel. Duplicate setup subtitles and repeated
`Advanced:` prefixes were removed. Backend output ownership, strict machine
identity, S0 safety, and probe fences are unchanged.

## Verification

The following source and focused checks passed:

- `pnpm --dir app build` (TypeScript and Vite production build)
- `node app/scripts/check-dmx-show-setup.mjs`
- `pnpm --dir app run check:output-control-runtime`
- `pnpm --dir app run check:safety-blackout-runtime`
- `pnpm --dir app run check:tauri-build-wrapper` — 231 assertions and 27 hostile mutation fixtures
- `pnpm --dir app exec node scripts/check-viewport-containment.mjs --setup-dmx-only` — 1920×1080, 1920×1032, 2048×1152, 1366×768, and 1280×720

The exact Community MSVC `14.44.35207` native gate produced the NSIS bundle
with `pnpm --dir app tauri build --bundles nsis`. The generated executable was
started from this checkout and produced exactly one responsive `Syndocal`
process. The Windows release artifact self-test passed with 140 assertions.

## Follow-up viewport contract

After explicit authorization, the previously preserved overlap viewport lane
changes in `app/scripts/check-viewport-containment.mjs` were included in the
follow-up checkpoint. The focused
`pnpm --dir app exec node scripts/check-viewport-containment.mjs --scene-block-overlap-only`
check passed at 1920×1080.

## Distributed artifact

Installer: `target/release/bundle/nsis/Syndocal_1.2.0-alpha.69_x64-setup.exe`

Size: `81,288,554` bytes

SHA-256: `CFF97A0C0913151338B522F1C18456662A42B9720CA02A7FC7C31C6698D2E630`

## Boundaries

This checkpoint proves source contracts, focused browser layout, native launch,
and artifact structure. Physical USB-DMX electrical output, fixture
illumination, downstream reception, and Art-Net/Unity reception still require
the separate operator/hardware acceptance. The broader viewport suite is not
promoted by this checkpoint; its unrelated Mixer drawer audio-input flow still
has a separate settling failure.
