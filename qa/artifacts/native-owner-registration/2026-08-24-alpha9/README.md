# Syndocal 1.2.0-alpha.9 native owner-registration acceptance

Result: **PASS** on 2026-08-24.

This is the privacy-safe native acceptance record for the fixed
`1.2.0-alpha.9` release executable. It proves that a clean no-project launch
registered a distinct transaction owner in each of the seven detachable pane
WebViews before their initial owner-gated selection update, that the main
WebView did the same before its startup and selection work after reload, and
that reloading the main WebView did not steal or rotate any pane owner.

## Fixed native artifact

- Command: `pnpm --dir app tauri build --no-bundle`
- Result: exit 0, release build finished in 2m27s.
- Linker identity: MSVC 14.43.34808 `Hostx64/x64/link.exe`; Cargo was pinned to
  that regular file rather than Git's `usr/bin/link.exe`.
- EXE: 57,489,408 bytes, SHA-256
  `BD4375D09EA09E099E6F24D74DC57B014E60F0C4C124401D1CBA6ACB1EE207FF`.
- PDB: 19,582,976 bytes, SHA-256
  `0DAF81F154F6C3953D5B9DC4A9DC85FF9889CC8CA5A277F4D635E80C2A17FC56`.
- Windows FileVersion and ProductVersion: `1.2.0-alpha.9`.

## Native procedure and result

1. The release executable was launched with no `.sdc` argument and an isolated
   WebView2 profile. The exact executable had one responsive `Syndocal` main
   window, and that verified window was maximized before UI operations.
2. The real **Workspaces** UI opened `Stage`, `Timeline`, `Programmer`, `Setup`,
   `Live`, `Mixer`, and `Touch` in that order. No raw `open_pane_window` IPC was
   used for acceptance.
3. A pre-document, fetch-level IPC observer retained the registration owner
   only inside the page closure. For every pane, registration was fulfilled at
   order 1..2 and the initial selection write was fulfilled at order 3..4.
4. Every pane was maximized and matched its expected Tauri label, pane mode,
   URL kind, and native title. Each pane returned `null` from the no-project
   startup binding probe using its captured owner, and its selection echo was
   exact.
5. After an instrumented main reload, main registration completed at order 2;
   selection and startup began at orders 3 and 4 and both fulfilled. This is a
   barrier followed by safe concurrent bootstrap work, not a claim that startup
   is serialized before selection. The main binding probe returned `null` and
   its selection echo was exact.
6. All seven panes again returned `null` after the main reload, and every pane
   still had exactly one registration. No owner-registration status, legacy
   unregistered-owner text, window error, unhandled rejection, page error,
   console error, or crash was observed in the accepted measurements.
7. All panes were closed through the same main Workspaces UI. The final state
   was one responsive, maximized `Syndocal` window and stored pane state `[]`.

The CDP client reset once while running a deliberately non-destructive group of
binding/selection probes. One subsequent main reload was rejected as evidence
because the disconnected client's init script did not persist. The observer was
reinstalled before the accepted main reload described above. No rejected run is
used to support the PASS claim.

Tauri enumerated five connected displays: two 1920x1080 displays, one
2560x1440 display, one 2560x720 display, and one 3840x2160 4K display. This
records enumeration only; it is not a new fullscreen-output, playback, GPU
reset, or display-cable acceptance claim.

## Evidence files

- `acceptance.json` is the machine-readable, redacted result.
- `main-seven-panes-open.png` shows all seven pane toggles active.
- `pane-stage.png`, `pane-timeline.png`, `pane-programmer.png`,
  `pane-setup.png`, `pane-live.png`, `pane-mixer.png`, and `pane-touch.png`
  capture the maximized WebView contents.
- `main-final-single.png` captures the final main-only state.

Owner identifiers, IPC payloads, project authority values, project data,
process identifiers, network details, display positions/names, isolated-profile
paths, and private absolute paths are intentionally excluded.

This acceptance does not claim physical DJ Link/rekordbox, DMX, MIDI/Pedal,
ASIO, fault-injection-matrix, output-playback, or soak completion.
