# Native final revalidation — 2026-09-10

## Scope

This checkpoint validates the exact no-bundle executable built from current
`main`. It is limited to Windows native startup, state IPC, missing-resource
thumbnail IPC, and owned-process cleanup. It does not perform the requested
real-file missing PNG → Retry → recovery test: that file-moving
operation was not performed and the recovery path remains unclaimed. It
enables no physical output.

## Source and artifact identity

- Source checkout: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`
- Source HEAD at build time: `990662c64e289f3b5ac1d7e568d8f7346a0fe5a2`
- Build command: `pnpm.cmd --dir app tauri build --no-bundle`
- Build result: exit `0`; maintained wrapper selected the exact MSVC
  `14.44.35207` Build Tools linker and returned it first from `where.exe link.exe`
- Executable: `target/release/syndocal.exe`
- Product version: `1.2.0-alpha.69`
- Bytes: `64,541,696`
- SHA-256: `BC4B082590E2A310F630C576EB3C29F9E71154E5B8E839724283ED150C0AD36F`

## Post-checkpoint CI status

- GitHub Actions run for this HEAD: `34388631465`.
- At the time of this checkpoint it was still `in_progress` in both Windows
  and Ubuntu jobs; no CI result is claimed here.
- The local `pnpm.cmd --dir app run check:release` completed with exit `0`.
- SDK-dependent Windows ASIO/NDI checks remain subject to the workflow
  boundary; this is not physical-device acceptance.

## Native probe result

The existing file-move-free native probe was copied to a fresh evidence
directory, updated only for the current artifact identity, run in a fresh
WebView2 profile on loopback port `51532`, and recorded `status: pass`:

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

Raw evidence: `target/qa/native-final-validation-20260910-02/`.

## Boundary

This proves current-source exact-artifact startup/state/thumbnail-IPC
behavior only. It does not prove the real-file missing → UI Retry → recovery
flow (file movement was not performed and the result is intentionally
unclaimed), GUI thumbnail cancellation hard-stop latency, physical output,
device acceptance, Mac execution, signing, publication, or product-wide
completion.

## Historical recovery-record disposition

The tracked records
`qa/THUMBNAIL_NATIVE_FAILURE_RECOVERY_2026-09-08.md`,
`qa/THUMBNAIL_NATIVE_FAILURE_RECOVERY_2026-09-09.md`, and
`qa/THUMBNAIL_NATIVE_FAILURE_RECOVERY_REVALIDATION_2026-09-09.md` contain
historical pass wording for a file-moving PNG recovery probe. Under the
current handoff authority, that file-moving helper operation was not performed
for this acceptance decision. Those historical records and their raw reports
are therefore not used to claim the missing-file → Retry → recovery gate here;
the gate remains unclaimed without rerunning the operation.
