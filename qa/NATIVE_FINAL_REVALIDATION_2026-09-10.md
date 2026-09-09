# Native final revalidation — 2026-09-10

## Scope

This checkpoint validates the exact no-bundle executable built from current
`main`. It is limited to Windows native startup, state IPC, missing-resource
thumbnail IPC, and owned-process cleanup. It does not repeat the already
accepted real-file missing PNG → Retry → recovery test, and it enables no
physical output.

## Source and artifact identity

- Source checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`
- Source HEAD at build time: `20ecd6cd68fe93c26134cbabb47da8e92deca968`
- Build command: `pnpm.cmd --dir app tauri build --no-bundle`
- Build result: exit `0`; maintained wrapper selected the exact MSVC
  `14.44.35207` Build Tools linker and returned it first from `where.exe link.exe`
- Executable: `target/release/syndocal.exe`
- Product version: `1.2.0-alpha.69`
- Bytes: `64,541,696`
- SHA-256: `6EDBA1ACB75BCBBF757EC3B8B5A36288D55891B3B47D211B994BDE179C703628`

## Post-checkpoint CI

- QA-record commit: `323f83a983cd0f9482b9d883a27d1e07f92d2657`
- GitHub Actions run: `34380451853`
- Windows 10+: success; Rust workspace, in-process video decode, frontend,
  Tauri/release checks, Windows installer bundle, and installer smoke stages
  passed.
- Ubuntu 22.04: success; Rust workspace, in-process video decode, frontend,
  Tauri/release checks, Linux package/AppImage bundle and smoke stages passed.
- SDK-dependent Windows ASIO/NDI checks were unavailable/skipped according to
  the workflow boundary; this is not physical-device acceptance.
- The workflow emitted upstream Node.js 20 deprecation annotations for pinned
  actions; no job failed because of them.

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
