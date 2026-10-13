# Parallel branch checkpoint — 2026-10-14

Branch `claude/parallel-20261014` (temporary worktree
`C:/Users/janua/AppData/Local/SyndocalDev/worktrees/claude-parallel-20261014`),
base `562058ea` on `main`. The work ran beside Codex's alpha.82 lane and is
merged into `main`; the worktree and branch are removed afterwards.

## Changes

| Area | Change | Evidence |
| --- | --- | --- |
| Physical test day | [Runbook](PHYSICAL_TEST_DAY_RUNBOOK_2026-10-14.md) for the open external-acceptance rows that one home/studio day can advance; ASIO prerequisites and undecided thresholds listed | Link check, `git diff --check` |
| Completion ledger | 70 post-2026-10-02 evidence paths linked to 16 Open/Deferred rows; statuses, classifications and nonclaims unchanged | `check-completion-ledger` 23/8/27, self-test 19 negatives, `check-q1-q4-ledger` |
| DJ Link gate message | The disabled-extension error points to the installer (the Setup I/O Extensions tab was removed in alpha.80); Japanese text added; NSIS README updated | `extensions::` 4/4, localization |
| Physical serial demo | `physical_serial_rainbow_demo_drives_master_dimmer_and_rgb_cells` fails when `SYNDOCAL_PHYSICAL_SERIAL` is unset instead of passing without hardware | `cargo check -p engine --tests` |
| macOS warnings | Windows-only DJ Link NLM/GAA policy, credential targets/variants, capture/E3 test helpers and Windows x64 Spout helpers compile only where used; RAII-only macOS audio owner renamed `_owner` | See results |
| Network/file parser hardening | ShowClock drops malformed/noise datagrams instead of stopping the standby; Web Remote makes the accepted socket blocking, evaluates only a complete ≤8 KiB HTTP head (431/400/408 otherwise), caps connections waiting for a head at 32 (503 beyond), and routes on the real `Upgrade` header; WebSocket accept bounded by the Remote/DJ Link limits; sACN ignores alternate start codes for levels and rejects slot counts above 513; GDTF entries capped (16 MiB description, 64 MiB media/model) without trusting archive sizes; seeded hostile corpora for Art-Net, sACN, RDM, OSC, `request_from_text` and `DjLinkEnvelope::parse_json`; ShowClock panel shows the dropped count | See results |
| MIDI feedback visibility | MIDI card shows a Feedback pill, the connected output, sent count and last message; warns and opens the feedback disclosure when learned scene feedback has no output; Learn pairs `MIDIIN<n> (X)` with `MIDIOUT<n> (X)` | See results |

## Compatibility notes

- Web Remote: a request whose head does not complete within 8 KiB / 1 s is
  rejected (previously only the first 2 KiB were evaluated and the rest was
  parsed later without policy). More than 32 connections still waiting for
  their head receive 503. Browser LAN requests are far below both bounds.
- Windows/macOS sockets accepted by Web Remote and DJ Link are now blocking
  with the existing 100 ms read timeout, as on Linux. The loops no longer
  busy-spin; DJ Link server-to-agent state is sent at most once per 100 ms
  read pass instead of immediately.
- sACN input: packets with alternate start codes (e.g. 0xDD priority) are
  not applied as DMX levels and are not counted as invalid.
- GDTF: profiles whose `description.xml` exceeds 16 MiB, or whose media/model
  entries exceed 64 MiB, are rejected with `EntryTooLarge`.
- ShowClock: malformed datagrams from the paired address are dropped and
  counted (`dropped_datagrams`; on Windows, datagrams too large to receive
  are counted whatever their sender). A well-formed envelope with a bad MAC,
  a replay or a session mismatch still faults the standby.
- MIDI feedback status JSON gains an `output` object; existing fields are
  unchanged. ShowClock status gains `dropped_datagrams`.

## Results

- Rust (debug, serial where the crate shares socket test hooks), 0 warnings:
  `io` 209 passed / 4 hardware-ignored plus 1 + 2 in other test binaries;
  `protocol` 265 + 1 corpus + 7 + 4; `gdtf` 19; `syndocal show_clock` 9;
  `syndocal extensions::` 4; `io midi::` 39;
  `cargo check -p syndocal --all-targets` clean. `cargo test -p io` with
  default parallel threads still fails DJ Link socket tests that share global
  hooks; the unchanged baseline fails the same way.
- Frontend: tsc, localization, check-midi-learn (4 pairing cases), MIDI learn
  browser, workspace operator, DVC MIDI shortcuts, MIDI fader input, command
  routing, admission inventory; browser render of the three feedback states
  in Japanese and English.
- macOS CI (run 38450821088, attempt 2, branch head `cd6d099b`): first-party
  release warnings 82 → 0 (third-party `tauri-runtime-wry` 5 unchanged);
  test-build warnings 29 → 1, the remaining one being the linker note
  `__eh_frame section too large`. Attempt 1 failed one unrelated
  authored-control-plane BPM test that took 43 s on the runner; it passed on
  rerun and passes on Windows (42/42 `control_plane::tests`).
- Independent review: first pass found no blockers and five should-fix items
  (non-blocking accepted sockets, unbounded pre-head waiting, ShowClock
  negative test, UI exposure of the dropped count, untested success paths);
  all were fixed and a re-review found no blockers or should-fix items.

## Remaining boundaries

- MIDI LED behaviour on the SMC-Mixer is not observed; the new status only
  shows what was handed to the driver.
- ShowClock still faults on a spoofed well-formed envelope with a bad MAC or
  a replay (fail-closed by design). A MAC-failure threshold is an owner
  decision.
- The 32-connection pre-head cap trades thread exhaustion for possible LAN
  lockout under a flood of partial heads; a per-IP sub-cap is not
  implemented. A worker thread is still spawned before the cap check.
- Test-timing nits from the re-review (50 ms pause inside the 100 ms
  first-byte window; a fixed 300 ms sleep in the pending-cap test) are left
  as is; both passed 24/24 under load.

## Integration on main and UI stage 1

Codex stopped at its usage limit after preparing the alpha.82 dispatch-ordering
candidate. Its working tree was committed unchanged (`ebd8a2d8`), the branch was
merged twice (`b722f9c2`, `292011b9`), and the worktree and branch were removed.
UI clutter audit stage 1 then landed directly on main
([audit](UI_CLUTTER_AUDIT_2026-10-14.md)):

- P1: Setup I/O and Security no longer render the fixture/stage/selection band.
- P5: workspace and Edit-domain tabs follow the UI language; common setup and
  status strings gained Japanese text.
- P4/P6/P7: Show DMX preparation uses operator wording with stable stage IDs;
  repeated network-universe, DMX-input and DJ Link hints were trimmed; MIDI
  feedback shows only the buttons usable in its connection state.
- P10: unused `DmxRoutesPanel.tsx` and `TouchRemotePanel.tsx` were removed.

P8 (single setup sub-tab) is deferred because more than ten viewport-checker
flows navigate through `.setupModeTabs`. The Audio notice is unchanged because
`check-audio-output-panel` requires it.

Windows native gate on main `32d09668`:

- `pnpm --dir app tauri build --no-bundle` passed in 447 s with zero
  first-party warnings (the existing Vite chunk advisory remains one).
- `target/release/syndocal.exe` is Product/FileVersion `1.2.0-alpha.82`,
  70,540,800 bytes, SHA-256
  `0B1E631457F2A206A023210CF043B0FCCC7F1E1FB4A9F1309FFDFB2F46663229`.
- The exact executable ran as one process with one responsive `Syndocal`
  window, maximized (`showCmd=3`), and then closed gracefully.

Win/Mac packages, physical MIDI/DMX output and Mac GUI remain unobserved.
