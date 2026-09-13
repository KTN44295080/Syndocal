# DJ Link Setup checkpoint — 2026-09-13

Status: accepted for the current Windows-source Setup/software boundary.

This checkpoint closes `DJ-LINK-SETUP-001` only. The implementation exposes an
explicit exact wired Show-LAN binding tuple, machine-local Credential Manager
arm/rotate/disarm, show-once token handling, listener/session diagnostics,
current admitted deck/track/heartbeat/loop/Timeline authority state, project
Track-to-Timeline mapping validation and CAS dispatch, `Use Current Track`, and
bounded disconnect/replacement fencing. The Setup I/O surface keeps DJ Link as
a separate fifth connection workbench and contains no Pedal/MIDI controls.

## Evidence

- `pnpm.cmd --dir app run check:dj-link` passed both the mapping-policy and
  frontend contract checks.
- `node app/scripts/check-viewport-containment.mjs --setup-io-only` passed at
  `1920x1080`, `1920x1032`, `2048x1152`, `1366x768`, and `1280x720`. Each run
  reported `cards=6/1`, `active=dmx:1`, `disclosures=12/12`, `scroll=1`,
  `remote=1`, and `remoteScroll=1` with `failed=[]`; the six cards include the
  DJ Link selector and its fifth-tab pointer/keyboard path.
- The Windows Release test was run after initializing
  `vcvars64.bat -vcvars_ver=14.44` and pinning
  `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.
  `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1
  dj_link -- --nocapture --test-threads=1` passed `120` tests with `0`
  failures and `1` intentional ignored live-network observer test. This
  includes the machine credential blob/store, settings journal, arm/rotate/
  disarm compensation and restart recovery, strict NIC trust/revalidation,
  listener policy, authenticated dispatch, owner/session/replay fencing,
  mapping selector admission, and Timeline-side DJ runtime tests.
- The current exact native artifact was built with the same pinned toolchain and
  observed as one responsive `Syndocal` window from
  `target/release/syndocal.exe`; SHA-256 was
  `38C4E36E28522CCB3D5E458ED55196EB820560945D17670E8B1D2B71C8B5E43D`.
  Only that exact checkout executable was stopped after the smoke test.

## Boundary kept open

This is not a claim of a real wired DJ-PC/Agent, rekordbox Master Track,
Stream Deck/Pedal, MIDI/TouchOSC client, LAN reconnect, or physical/venue
acceptance. The authoritative DJ/Pedal matrix remains `0/12`; those rows stay
open under `DJ-LINK-HARDWARE-001`, `INPUT-PHYSICAL-001`, and the related
`REMOTE-SECURITY-001` external boundary. H4/H5 and release/distribution rows
are not included in this checkpoint.
