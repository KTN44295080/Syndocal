# Syndocal H3 Setup checkpoint — 2026-09-13

Branch: `codex/showclock-review-20260912`

## Scope

This checkpoint closes the current Windows-source Setup workflow required by
H3: Fixture Library/profile selection (including bundled/cache/verified GDTF and
OFL-backed catalog paths), Patch and Repair, DMX address validation, Stage
placement/mapping, selected-fixture inspection, Video Outputs, and the I/O
connection workbench. Empty/error states, closed advanced I/O disclosures,
pointer drag/drop, keyboard focus/selection, and output-configuration state are
included in the exercised software surface.

The implementation uses the existing authoritative transaction and output
ownership paths. Setup configuration remains authored state; machine-local
output ownership and safety gates are reported separately and do not rewrite
the authored configuration.

## Focused software evidence

The following checks passed on the current source:

- `pnpm.cmd --dir app run check:patch-transaction-d2` — PASS.
- `pnpm.cmd --dir app run check:fixture-catalog` — 69 assertions PASS.
- `pnpm.cmd --dir app run check:patch-profile-counts` — 10 assertions PASS.
- `pnpm.cmd --dir app run check:profile-library` — 2,217 fixtures / 7,359
  modes, OFL/QLC+ merge and conflict policy PASS.
- `pnpm.cmd --dir app run check:gdtf-profile-actions` — 15 actions PASS.
- `pnpm.cmd --dir app run check:fixture-live-color`,
  `check:fixture-limits-degrees`, and `check:dmx-addressing` — PASS.
- `node app/scripts/check-dmx-show-setup.mjs` and
  `node app/scripts/check-mapping-stage-geometry.mjs` — PASS.
- `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1
  d2_ -- --nocapture --test-threads=1` — 9 passed.
- `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1
  patch_ -- --nocapture --test-threads=1` — 30 passed.
- `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1
  gdtf_ -- --nocapture --test-threads=1` — 10 passed.
- `cargo test -p engine --release --locked -j 1 fixture_patch_ --
  --nocapture --test-threads=1` — 10 passed.
- `cargo test -p engine --release --locked -j 1 fixture_profile_repair_ --
  --nocapture --test-threads=1` — 6 passed.
- `cargo test -p gdtf --release --locked -j 1 -- --nocapture
  --test-threads=1` — 18 passed, 0 failed.

## Rendered browser evidence

`node app/scripts/check-viewport-containment.mjs` focused lanes passed with
`failed=[]` at 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720:

- `--patch-only`: continuous 512-cell grid, DND patch/preview/conflict/reject,
  GDTF share/cache/error paths, empty state and responsive scaling.
- `--fixture-catalog-only`: empty and populated catalog states.
- `--mapping-viewport-conformance-only`: grid/layer/axis/drag/handle/readout
  geometry.
- `--setup-video-only`: output selection, preview, mapping and closed/open
  advanced controls.
- `--setup-io-only` and `--setup-dmx-only`: six connection cards, I/O tabs,
  DMX logical routes, twelve/three disclosures, scroll reachability and
  fail-closed preparation boundaries.
- `--setup-stage-band-sequence-only`: Stage selection/clear, return-to-Patch,
  I/O and patch route preservation.
- `--fixture-groups-only`: group create/select/rename/delete, color registration,
  fixture panel and coarse-pointer hit targets.

The focused lanes recorded pointer and keyboard reachability, visible focus,
empty/error recovery actions, internal scrolling, and zero outer document/app
scroll. This is rendered-browser evidence, not a claim of native accessibility
or physical output.

## Windows native evidence

The pinned Windows native procedure (`MSVC 14.44.35207`, linker-first PATH) ran
`pnpm.cmd --dir app tauri build --no-bundle`. The exact checkout executable was
then launched and produced one responsive Syndocal window:

- executable:
  `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe`
- SHA-256:
  `38C4E36E28522CCB3D5E458ED55196EB820560945D17670E8B1D2B71C8B5E43D`
- exact process count: `1`
- window count: `1`
- title: `Syndocal`
- responding: `true`

The exact checkout process was stopped after the smoke check. No other
executable was terminated.

## Boundaries

This closes H3's current Windows-source Setup/browser/native-process slice. It
does not claim NVDA/High Contrast/125–200% native accessibility, real DMX/MIDI/
OSC/RDM/video/audio devices, fixture illumination or downstream reception,
real two-machine/venue operation, signing, publication, or H4/H5 completion.
The broader PATCH/GDTF Q1 row remains in progress where D-lane audit and real
repaired-profile output are still open.
