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

Physical MIDI/DMX output and Mac GUI remain unobserved.

## UI stage 2 and the alpha.82 development installer

Stage 2 on main: P3 (`2db5cced`) removed duplicated live actions; P9 split
Active/Standby sync and ShowClock LAN into their own I/O Sync connection
(`cc52b06f`) and moved the live audio input to I/O > Audio (`e3db3b30`).

The development installer was built from `cc52b06f`, so it contains stage 1,
P3 and the Sync split, but not the audio-input move or later work. The local
FFmpeg 8.1.2 SDK is gone (WinGet now carries 9.0.1), so, as in the
2026-09-09 repair, `FFMPEG_DIR` pointed at an ASCII-path temporary SDK: the
n8.1 headers and import libraries plus the seven hash-verified pinned DLLs.

- `tauri build --ci --bundles nsis,msi` passed in 2871 s; the only warning is
  the existing Vite chunk advisory.
- `syndocal.exe` 70,536,704 bytes, SHA-256
  `0c8a93be0c56bc86b8691be053edc6b49f839b6a0be0ae25fe5e50717fcde811`.
  It ran as one responsive `Syndocal` window, maximized (`showCmd=3`), and
  closed gracefully.
- NSIS extraction (7-Zip) and MSI administrative extraction passed the
  two-root validator: matching executable, version `1.2.0-alpha.82`, seven
  pinned FFmpeg DLLs, approved tree. Six NSIS-only plugin files were set
  aside: NSISdl, StartMenu, System, modern-wizard.bmp, nsDialogs,
  nsis_tauri_utils.
- `tools/distribution/alpha82/check-setup-helper.mjs` passed eight mocked
  lifecycle cases; `Install-Windows.ps1 -ValidateOnly` passed on the staged
  folder and on a Windows re-extraction of the final ZIP.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Windows-x64-install.zip | 223511590 | `093e9e9366624c51ade42a902d326c749f019517818c5fac99ea9396086b3abe` |
| x64-setup.exe | 82488341 | `cf7b48ae7d1bfa8634d68dc8c187990487e2049ad50ac89d1059f969cf1170f1` |
| x64_ja-JP.msi | 123523072 | `b3d98e99c27d197a347a16265154453f0dfaf60416e24936c1231b25f6e4e3ea` |

All names start with `Syndocal_1.2.0-alpha.82_`. Delivery folder:
`Downloads/Syndocal-1.2.0-alpha.82` in the user profile. ZIP entry names
are UTF-8 flagged (a first .NET Framework ZIP mis-encoded the Japanese guide
name and was replaced). Evidence: [artifacts/ui-stage2-alpha82](artifacts/ui-stage2-alpha82).
No product installer was executed. Unsigned development distribution only.

Pre-existing viewport failures seen on unchanged parents and tracked
separately: `--live-audio-only` `asioGenerationNeverNameRemaps` (resolved
below), and `--scene-settings-only`
`settingsUseInternalScrollWithoutHorizontalEscape` (bottom sentinel not
visible at End).

## `--live-audio-only` ASIO generation check

`asioGenerationNeverNameRemaps` was a stale, timing-dependent check, not a
product regression. The live-audio refresh and selection code, the rail and
the selection storage are unchanged since `a65e3ccb` (2026-08-27).

- The check dates from `a255a51b`, before saved selections existed. Since
  `aed77a26`, completing driver, rate and buffer saves the ASIO choice by its
  stable backend/name/label identity, never the generation-scoped ID. Refresh
  then takes the saved path: that identity must match exactly once and pass a
  capability probe on the new ID before Start re-arms. This is the restore
  contract that `--live-audio-restore-only` already asserts. FC-09 forbids it
  only after a catalogue failure.
- The old check read the rail 45 ms after the generation bump, while that
  probe was still in flight, so it saw the previous ID. It passed when the
  read was early and failed when it landed after the probe (1366x768 EN).
- The check now waits for Refresh to finish. `asioGenerationNeverNameRemaps`
  covers the unsaved path it was written for: with only a driver chosen,
  Refresh keeps g1 invalid and Start locked, issues no capability probe and
  writes no storage. The new `asioSavedSelectionRevalidatesExactIdentity`
  covers the saved path:
  - Start stays locked during the swap.
  - The stored bytes are unchanged and contain no generation ID.
  - Exactly one probe runs, for g3 at 48 kHz.
  - The saved state ends `ready`/`REVALIDATED` with Start enabled.

Evidence:
- `--live-audio-only` passed 10/10 (five viewports, EN/JA).
- `--live-audio-restore-only` and `check-live-audio-input.mjs` passed.
- A temporary App.tsx mutation that let the unsaved ASIO path remap by name
  failed only `asioGenerationNeverNameRemaps`.
- An independent review approved with no P0–P2 findings.

Browser evidence only; no native or physical ASIO claim.

## Control skin, DAW-mode MIDI and FX usability (later 2026-10-14)

All on main and pushed. Commits are rebased on the separate ASIO-check fix
`eaec2479`.

- Control skin `0dc10769`: one slider/fader/progress language. Range and
  progress sizes are unchanged before and after at 1920x1080.
- MIDI `499c74c9` + review fixes `45a2b36c` + `7e9a49d6`. The SMC-Mixer
  lights its button LEDs only from fixed notes, so DAW mode is the working
  LED mode (the user confirmed the LEDs light in DAW mode). Its faders send
  14-bit pitch bend and its V-Pots send Mackie relative steps; both can now
  be learned. Relative steps are allowed only for continuous targets and
  fail closed when the target position is unreadable.
  - An independent review found 2 blocking and 3 should-fix defects. All
    were fixed, and the re-review found no blocking defect.
  - io `midi::`/`control_plane::` 45 passed. The syndocal control-plane /
    midi / admission / dvc / validate filter 257 passed.
  - `project_replacement_control_plane_legacy_open_matches_acknowledged_engine_persistence`
    is intermittent: it fails on varying fixtures and passed 2 of 3
    isolated reruns plus the final filtered run.
  - Physical fader/encoder movement on the SMC-Mixer is not yet observed.
- FX editor `025c66bb`, `141b1e30`, `68369f3b` (read-only audit first):
  - numbered steps (target fixtures, then channels to move);
  - per-family channel guidance;
  - a pinned Save dock with an Unsaved mark and a discard confirmation;
  - Chaser steps follow an untouched target;
  - family buttons read as add-actions;
  - corrected wording;
  - labelled surface rail.
  The Scene Settings scroller's auto rows were sized by leftover height,
  which was the long-standing `settingsUseInternalScrollWithoutHorizontalEscape`
  failure. With max-content rows, viewport `--scene-settings-only` and
  `--scene-fx-block-only` now pass at all five sizes.
- Windows native gate on the MIDI/FX build (`141b1e30` content):
  - `tauri build --no-bundle` took 433 s with 0 first-party warnings.
  - `syndocal.exe` 1.2.0-alpha.82, 70,540,800 bytes, SHA-256
    `2df8afd5890a8318d7ef89cece1f08c6176b483b11d0c25dd4c04a43d23ed722`.
  - The exe ran as one responsive `Syndocal` window, maximized
    (`showCmd=3`), and closed gracefully.

Checks that fail identically on unchanged parents (stale contracts, not
regressions):

- `--live-edit-types-only` expects a 7-entry category rail; the rail now
  has 8.
- `--workspace-operator-only` cannot find `.workspaceOperationsButton`
  after the one-row header.
- Several flows hit `Page.navigate` timeouts once a project is dirty
  (beforeunload prompt).
