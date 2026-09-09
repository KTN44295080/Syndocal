# Native final revalidation — 2026-09-10

## Scope

This checkpoint validates the exact no-bundle executable built from current
`main`. It is limited to Windows native startup, state IPC, missing-resource
thumbnail IPC, and owned-process cleanup. It does not repeat the already
accepted real-file missing PNG → Retry → recovery test, and it enables no
physical output.

## Source and artifact identity

- Source checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`
- Source HEAD: `20ecd6cd68fe93c26134cbabb47da8e92deca968`
- Build command: `pnpm.cmd --dir app tauri build --no-bundle`
- Build result: exit `0`; maintained wrapper selected the exact MSVC
  `14.44.35207` Build Tools linker and returned it first from `where.exe link.exe`
- Executable: `target/release/syndocal.exe`
- Product version: `1.2.0-alpha.69`
- Bytes: `64,541,696`
- SHA-256: `6EDBA1ACB75BCBBF757EC3B8B5A36288D55891B3B47D211B994BDE179C703628`

## Native probe result

The existing file-move-free native probe was adapted in memory only for the
new artifact hash, run in a fresh WebView2 profile on loopback port `51530`,
and recorded `status: pass`:

- exactly one visible `Syndocal` window, title `Syndocal`, responsive and
  maximized;
- output ownership `Standby`, with `lighting_allowed=false` and
  `video_allowed=false`;
- `get_snapshot` returned fixture, cue, and video-output arrays, with zero
  initial video outputs;
- missing media asset `999999` was rejected with
  `Media asset 999999 was not found` and a valid asset ticket;
- missing video layer `999999` was rejected with
  `MissingLayer { layer_id: 999999 }` and a valid layer ticket;
- physical output operations: `0`;
- the owned application exited, exact executable process count was `0`,
  loopback listener count was `0`, and the probe's held-file count was `0`.

Raw evidence: `target/qa/native-final-validation-20260910-01/`.

## Boundary

This proves current-source exact-artifact startup/state/thumbnail-IPC
behavior only. It does not prove the real-file missing → UI Retry → recovery
flow (already recorded separately and intentionally not repeated here), GUI
thumbnail cancellation hard-stop latency, physical output, device acceptance,
Mac execution, signing, publication, or product-wide completion.
