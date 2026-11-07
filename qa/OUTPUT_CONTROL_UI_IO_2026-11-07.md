# Output controls, Sync density and Windows I/O checkpoint

Base: `314ff556`, shared `main`, 2026-11-06–07. Continues Claude's Display
walkthrough and the user's UI/confirmation/I/O requests. No version bump or
distributed artifact. Existing untracked `app/.vite-task74-table/` and
`app/.vite-task74-usb/` are outside this change.

## Changed behavior

- Display Add/Open/Close accepts one exact Video or lighting+video lease. The
  native registry revalidates its actual scope; same-owner/project recovery
  preserves it. Lighting-only, ambiguous, wrong-owner and scope-widening requests
  remain rejected. Frontend receipts bind to the queried scope. No operation ID
  or wire schema change.
- Removed routine native output/editor-display Yes/No gates and redundant
  confirmations for reversible actions, ordinary RDM SET/discovery, diagnostic
  export preview and updates. Lease, grant, fence and audit checks remain.
  Permanent deletion, unrecoverable draft loss, RDM factory defaults and dangerous
  Standby transfer isolation retain their safeguards.
- Updates reject Timeline/invalid FX drafts, await valid FX saves and block
  pane-opening races. Open/Restore/import captures fresh authority after draft
  saving. Close still confirms real unsaved data loss.
- Sync initially shows output state, Enable/Standby, and closed sections for
  standby sync, takeover recording, ShowClock and LTC. Advanced roles/leases
  and clock metrics are disclosed. Active state, errors, split-brain and pending
  recording remain visible. Typography and hit targets were preserved.
- Shortened repeated MIDI, DMX, video-source and mapping text; normal video
  diagnostics and the long same-PC Spout explanation moved into details.
  Display action labels now react to actual open/busy/kind changes.
- Real Spout input testing exposed an uninitialized zero frame from the SDK's
  double-staging warmup. Prime once per connection/update before publishing.
  Valid all-zero black remains valid; no pixel filtering or extra copy/allocation/
  worker. Material runtime changes received independent review before validation.

## Evidence

Ignored local evidence roots: `target/qa/output-ux-20261106/`,
`target/qa/io-continuation-20261106/`, `target/qa/sync-disclosure-20261107/`.

| Boundary | Observed result |
| --- | --- |
| Native Display, isolated QA profile | UI acquired Video-only lease, armed Video with lighting blocked, added DISPLAY2 and opened/closed/reopened its window without native confirmation or an internal Both lease. 1920×1200 captures show changing transformed color bars. `01`–`06` screenshots/trees. |
| Final ordinary Windows executable | `pnpm --dir app tauri build --no-bundle` exit 0; exact Build Tools MSVC 14.44.35207 linker first in PATH. SHA-256 `1ab1456396c77ede76f0a3d2005aa42917544940a21c2095a3dca97eea339cc2`, 76,698,624 bytes. Exact `target/release/syndocal.exe` launched: one responsive Syndocal main window, maximized (Restore enabled / Maximize disabled). Latest Sync rendered and Standby operated. `12`/`13`/`18`/`19` captures. |
| Virtual MIDI → native UI | Private teVirtualMIDI loopback CC7 0/64/127. Native Learn mapped Group 1 opacity; eight observed row samples include 0%, 50%, 100%. Disconnected; helper emitted 600 messages, removed only its own port, unrelated ports preserved. SMC-Mixer/CustomMIDI1 were not opened. `midi-native-values.json`, `07`–`10` captures and peer log. |
| COM5, real serial worker | Exact FTDI `0403:6001`, PnP `FTDIBUS\VID_0403+PID_6001+6&1824F623&0&2\0000`. Canonical S0 engage through local native IPC; UI USB Universe 0 Enabled → Apply → Active, then disabled → Apply → Stopped. Start: physical zero write completed, no live frame queued. Stop: zero BREAK/MAB/write_all/flush and worker shutdown completed, no fault. S0 held throughout. `14`–`18` JSON receipts, `16`/`17` captures. No nonzero frame or fixture illumination test. |
| NDI product / separate SDK peer | Loopback and separate SDK sender → product input passed. Product → exact named SDK receiver passed, changing pixels and 640×360/1280×720 observed, timestamp/timecode progression checked. Product output 1 passed, receiver exit 0. `product-ndi-output-peer2-receipt.json` and logs. Same host only. |
| Spout real DirectX transport | Product input/output each 1 passed. Added first-frame regression: valid transparent black, resized first frame exact `0x7f`, same-name/same-size reconnect first frame exact `0x55`; 1 passed. Separate SDK receiver observed 24 samples / 23 pixel values. SDK counter stayed zero: no FPS claim. |
| Sync browser | English/Japanese at 1920×1080, 1366×768, 1280×800: default fits, keyboard disclosure, no accidental sends, active/failure visibility and Display action prop updates pass. Default fixture text 218 English / 106 Japanese characters with 150px shell headroom. |

The isolated Display QA executable (`68eef36e…`) predates the final Sync/copy,
draft-guard and Spout changes; its picture evidence is separate from the final
ordinary executable. Final native checks used an empty unsaved project and the
existing COM5 identity; original show files were not saved over. The final
ordinary instance was left in Standby, USB worker stopped. The diagnostic CDP
instance exited; the remaining normal instance has no diagnostic launch flag.

## Validation

- Rust: Display/lease boundary 70 passed; protocol recovery scope 1; project
  replacement 26; backup 37; diagnostic export 9; production reset authorization
  1. Reset owner-race expected code corrected from Forbidden to PublicationFailed
  after independent review: grant admission succeeds, domain revalidation rejects
  before physical work. Early grant-denial assertions remain Forbidden.
- Frontend: TypeScript/native prebuild; output runtime/ownership, safety blackout,
  Display/window observation/routing, media authority, destructive draft/update
  admission, project shortcuts/import and dialogs passed. Real browser dialog
  checks passed for main plus detached panes. FX browser assertions completed but
  server cleanup hung; no exit-zero claim for that run.
- Final focused checks: camera profile, localization, output runtime, destructive
  actions, shortcuts (15,552 matrix cases), dialogs and Tauri wrapper (249
  assertions / 27 mutation fixtures) passed. Japanese static coverage 4197/4197;
  unprotected bare user-data labels 0.
- Setup Video and Setup I/O containment passed at 1920×1080, 1920×1032,
  2048×1152, 1366×768 and 1280×720. I/O's old scroll assertion now opens Standby
  sync through its summary before checking the role control; default-closed,
  hit-testing and scroll containment are explicitly asserted.
- First-party compiler warnings: first normal build 0, final normal build 0,
  delta 0, default Windows configuration actually built. No measurement for unrun
  ASIO/Mac configurations. `git diff --check` passed.

## Remaining boundaries and retained failures

- The three-layer/group-FX Display QA sample reported roughly 82–92ms average
  native frame time and 600/600 late frames: **60fps failed**. Visible moving pixels
  are not performance acceptance. No matched baseline attributes this to the UI
  changes; native 60fps/release acceptance stays open.
- No fixture/DMX wire-delivery, physical MIDI/feedback, other-host NDI,
  third-party application, Mac/Syphon, competitor timing or release/signing claim.
- Existing desktop-window T25-E drag-marker and VJ-media-import Timeline JSX-order
  assertions also fail on the base tree; neither was weakened. Failed/aborted logs
  remain. Wrong localization script name and parallel browser startup attempts
  were invocation errors; correct focused/localization and sequential viewport
  reruns passed.
- An hours-old isolated QA session rejected Enable Output with `invalid_request`
  after Video lease expiry. Standby applied; COM5 was never opened there. Fresh
  final native Enable/automatic ownership and COM5 passed. Stale-session Enable
  remains a follow-up, not a passing recovery test.
- NDI helper initially over-required changing user timecodes. Final criteria
  require advancing SDK timestamp or timecode, changing pixels and both sizes.
  PowerShell 5 failed module preflight before product execution; final pinned
  PowerShell 7 run and both process exit statuses passed.

Next acceptance: reproduce stale-session Enable, run the matched native 60fps
campaign, then operator verification of approved physical fixture/MIDI and
external-client paths. This checkpoint does not close those gates.
