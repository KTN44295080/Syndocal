# Syndocal 2026-08-30 show completion handoff

Status date: 2026-08-28 JST

This is the concise authoritative resume note for the final show-critical tranche. It supersedes chat-only status, but it does not supersede the detailed acceptance documents named below.

## Source authority

- KDMX checkout: `C:\Users\kouty\Documents\KDMX`
- Branch: `codex/syndocal-v1.2`
- Alpha.27 checkpoint parent HEAD and upstream: `e583141cc60decff7c062db21a39f69241f894c8`. The enclosing checkpoint commit is created and pushed immediately after the gates recorded here.
- DJ Agent checkout: `C:\Users\kouty\Desktop\rb-output`
- Branch: `beta-v1.1.2`
- DJ Agent committed HEAD and upstream: `a13d7bff59db5e7c00e19655f87c69db7cb52005` on `beta-v1.1.2`; its worktree was clean at the recorded checkpoint.
- The DJ Agent operator-return path received independent source-review `GO` with no P0/P1/P2. The external full regression passed `506 tests / 504 passed / 0 failed / 2 skipped` with first-party warnings 0. DJ-PC pull/restart, strict preflight, active runtime version, real ACK, and physical pedal acceptance remain external gates.
- KDMX product metadata is synchronized at `1.2.0-alpha.27`; all source/native gates recorded below passed before this checkpoint was committed.

## Accepted source boundaries

### Pedals and DJ Link source boundaries (not physical acceptance)

- Pedal 1 / F13 owns the current loop toggle in Timeline-control mode.
- Pedal 2 / F14 owns loop-half in both modes: Rekordbox MIDI loop-half in DJ-control mode and `DJ_TIMELINE_LOOP_HALF` for an active Syndocal Timeline loop in Timeline-control mode.
- Pedal 3 / F15 owns Timeline `+4 bars` only in Timeline-control mode.
- F13 DJ release starts HPF and emits the correlated `DJ_RELEASE` on the same edge. The local Rekordbox action then completes HPF, ChannelFader fade, stop, and reset independently of Syndocal delivery.
- Stage 2 commands require exact Timeline/play-session/release authority and revalidate current playing Timeline identity in the engine worker before mutation.
- Re-enabling a completed loop after position B re-enters at A; disabling does not jump.

Focused source evidence on the current dirty tree:

- protocol DJ Link: 14 passed, 0 failed, first-party warnings 0.
- engine DJ Link: 30 passed, 0 failed, first-party warnings 0.
- Syndocal DJ Link: 119 passed, 0 failed, 1 ignored live-network test, first-party warnings 0.
- Independent Stage 2 adversarial review: GO.
- DJ Agent v1.1.10 independent adversarial review: GO. Supervisor focused rerun: 88 passed, 0 failed; `git diff --check` passed before commit.

### Imported fixture stage layout

- `960 sound waves strongpoint` is authoritative as four physical cells and four logical RGB segments. Daslight's twelve displayed cells are the known three-row duplication bug, not twelve physical emitters.
- The Strongpoint collapse requires exact normalized profile identity, twelve raw DVC cells, four logical segments, and the exact three-row duplication pattern. A fixture label cannot trigger the exception; unrelated twelve-cell profiles remain twelve cells.
- Mega Bar remains eight physical cells and eight logical RGBA segments, with separate global dimmer and strobe controls.
- Physical-only layouts remain physical-only; the frontend does not infer color roles from control names.
- Unknown persisted layout fields and duplicate PATCH beam indices fail closed.

Focused source evidence on the current dirty tree:

- protocol stage-layout validation: 13 passed, 0 failed, first-party warnings 0.
- DVC stage-layout unit tests: 5 passed, 0 failed, first-party warnings 0.
- duplicate PATCH beam rejection: 1 passed.
- synthetic exact topology import: 1 passed.
- operator-owned `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` acceptance: 1 passed with Strongpoint 4/4, Mega Bar 8/8, and wristband layout absent.
- TypeScript build, Strongpoint browser contract, mapping viewport conformance, stage-label, mapping geometry, and fixture live-color checks: passed.
- Independent physical-layout adversarial review: GO.

### Show authoring and safe publication

- The DSF2026 authoring tool uses a Windows fail-closed safe-write path with exclusive create, reparse rejection, identity revalidation, flush, exact-length verification, and cleanup on failure.
- Independent safe-write review: GO; focused tests 3 passed.
- Fresh native import/save evidence uses the exact operator-owned source `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`. The report is exact at `fixtures=46`, `profiles=12`, `fixture_groups=15`, `cues=2`, `Converted=93`, `Approximate=4`, `Skipped=0`, and `Unsupported=1` for one Daslight hardware binding. The saved base has exact stage layouts: six Mega Bar layouts with 8 cells and 8 logical segments each, plus three Strongpoint layouts with 4 cells and 4 logical segments each.
- Native Save As output artifact: `target/qa/dsf2026-native-alpha27/DSF2026-imported-alpha27.sdc`, `1,079,564` bytes, SHA-256 `B21165A70A41A4036153359E579E1433C2739EC1C3EDCC0C46B94F513238DFB1`.
- Alpha.15 is retired as a current authoring base. Its artifacts remain historical evidence only and must not be supplied to the pinned authoring CLI.
- The former alpha3-alpha8 candidates are superseded. The reviewed final candidate is the alpha9 reference-audio artifact documented below; do not deploy an earlier candidate.
- The current supervisor rerun passed the authored-show test, DJ Link frontend/runtime contract, localization `3556/3556` with 0 unprotected labels, stage-label contract, fixture-limit degree contract, Timeline context-menu browser contract, I/O disclosure scroll contract, Strongpoint segment browser contract, Stage Settings viewport contract at five sizes, TypeScript/Vite production build, release metadata, Tauri wrapper self-test, and the `frontend-typescript-vite-windows` warning ratchet. First-party warnings were zero.
- The final alpha.27 native release build completed under the exact MSVC 14.44 linker gate. The built `target/release/syndocal.exe` is `60,314,624` bytes with SHA-256 `CEBB44C713043CCE885D87E3651464F3756A5CEE2D1700728D412AC7B18C48EC`.
- Exactly one checkout-owned alpha.27 process was responsive after that build: PID `87732`, HWND `124064278`, and the verified Syndocal window was maximized.
- This handoff claims no DJ HELLO/ACK exchange, physical pedal acceptance, or physical DMX-output acceptance; those remain unverified.
- `target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha3.sdc` passed the current structural Timeline preflight, including the one-measure transition and indefinite pedal-release holds.
- That alpha3 artifact is not the final show file because its base predates the newly imported fixture `stage_layout`. Do not deploy it as the final stage-layout authority.

### 2026-08-28 reference-audio authoring delta

- The two operator-supplied MP3 files are now accepted only as rehearsal/reference material through the ordinary Media Library -> Timeline Audio Clip path. They are not click/guide assets and are not armed for show playback.
- The authoring helper preserves both originals, copies byte-identical files beside the generated candidate under ASCII names, records SHA-256/byte size/duration, and refuses overwrite or divergent existing bytes.
- Each authored song Timeline receives one ordinary Audio layer with one `media_asset_id` clip. The layer is muted by default and requires an explicit operator unmute before rehearsal playback.
- Final reviewed candidate `target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha9-reference-audio.sdc` is `1,092,555` bytes with SHA-256 `E53AB3B4432E21C8EAEA4F9D727F3AC6ED8793B8282AA10CAEB3A265FDDC3CBF` and passed structural preflight. Measured durations are `214032 ms` for 人生オーバー and `273432 ms` for 惑う星; source and managed-copy SHA-256 values match.
- Windows publication holds verified non-reparse parent/leaf handles across source and sidecar revalidation through candidate flush. Parent/leaf substitution, same-hash reparse substitution, partial write, and flush failure fail closed without candidate or temporary-directory residue.
- Focused authoring regression passed with first-party warnings 0, including explicit no-reference assertions for zero Timeline Audio Clips and zero added Media Library assets. The final independent Terra xHigh review is `GO` with no P0/P1.
- Alpha9 loaded successfully into the exact native process through the single-instance project-forwarding path and reported 46 fixtures plus the expected 12 embedded profiles and mappings. Timeline Audio Clip playback remains unverified.
- A cold command-line launch with alpha9 exposed an open startup race: `Project authority changed before mutation (expected epoch 0 revision 0)`. Starting Syndocal first and forwarding the project path then loaded successfully. This race is not accepted or hidden; fix it in the next narrow tranche before cold-start deployment is claimed.

### 2026-08-28 machine-local USB-DMX delta

- The project retains only the logical serial DMX route. COM/PnP identity is stored in machine-local state and is selected from a current-device dropdown, then persisted only by an explicit Confirm action.
- Missing, stale, ambiguous, renumbered, and A->B->A physical identity changes fail closed. The exact opened Windows handle identity is revalidated rather than trusting a selected port label.
- The S0 blackout epoch is linearized ahead of activation/live ticks. The same physical write gate is held from selected-frame authority through BREAK, MAB, `write_all`, and `flush`; S0-first therefore permits zero live frames, while worker-first permits at most the already-started live frame followed only by zero frames.
- The deterministic worker proof uses the real `EnttecOpenDmxSender` worker and real Open-DMX write sequence with a fake `SerialPort`, including the public `safety_blackout_engage_published` interleave and non-deadlock assertion.
- Focused Rust evidence: `cargo check -p engine`, serial DMX 14/14, engine DMX 29/29, show route 4/4, machine binding 3/3, serial A->B->A 1/1, first-party warnings 0. Frontend output-control, production build, and exact command routing/invoke gates passed.
- Final independent adversarial review is `GO` with no P0/P1/P2. Physical USB-DMX/fixture output remains open.
- Final source gates after the active/bank normalization repair: exact MSVC 14.44 workspace Rust `2683 passed / 0 failed / 17 ignored` across `2700` executed tests, followed by the final exact-gated Syndocal rerun `1199 passed / 0 failed / 12 ignored` across `1211` tests; frontend production build; release/ASIO packaging `169` assertions; Tauri wrapper `231` assertions plus `27` hostile fixtures; exact frontend route/invoke inventories; Stage label, Strongpoint, fixture-limit degree, and remote disclosure contracts. First-party warnings remained 0; `cargo fmt --all -- --check`, `git diff --check`, and `pnpm --dir app run check:release` passed.

### ASIO PROGRAM/CUE output gate added 2026-08-28

- `qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md` is now the authoritative output gate. ASIO output is a show-critical requirement, not optional polish.
- Adopted architecture C: retain the exact input/Reactive Capture ABI/schema v2 surface, and add an exact v3 output/full-duplex surface to the same canonical bridge DLL. ASIO playback uses one v3 session; a parallel v2 session on the same driver is forbidden and must fail busy.
- PROGRAM stereo and CUE mono are project-level logical buses. Physical PROGRAM L/R, CUE, optional Spare, driver identity, sample rate, format, and buffer remain machine-local. Missing bus data migrates one way to PROGRAM; CUE never falls back to PROGRAM.
- Two logical Rodio mixers feed a non-realtime renderer and bounded preallocated interleaved SPSC. The ASIO callback only copies a complete block or outputs a complete silent block and latches terminal Fault. PROGRAM/CUE share one device and clock.
- DSF2026 acceptance mapping is MOTU M4 Output 1/2 = PROGRAM L/R and Output 3 = CUE at exact 48 kHz, with Output 4 optional Spare. This is a selectable profile, not MOTU-specific code.
- Device/rate/mapping conflict, disconnect, XRUN, reset/resync, buffer/rate change, callback gap, or underflow must stop output without WASAPI/default-device/rate fallback. Explicit revalidation and Start are required.
- Independent Terra xHigh review added three implementation-blocking P0 boundaries: quiesce and join the legacy `FollowProgram` CUE/normal Rodio output before v3 Start and stay silent on Start failure/Fault; freeze exact v3 callback/queue/lifetime semantics before code; and prove arbitrary non-contiguous/reordered physical mappings with all unselected channels zero. It also requires exact queue/race injection tests and updates every show-ASIO v2-only export checker to the exact v2-nine plus v3-nine set.
- The acceptance contract was committed and pushed at `e583141cc60decff7c062db21a39f69241f894c8`. Implementation begins as the separate `1.2.0-alpha.28` tranche after the alpha.27 native checkpoint is fixed.
- Implementation, independent review, native build, MOTU M4 output proof, and M32/DL16 routing proof remain open.

## Required remaining acceptance

1. Fix and verify the cold command-line project-load authority race, then verify playback of both ordinary alpha9 Timeline Audio Clips and the machine-local serial selector UI.
2. Pull the committed DJ Agent checkpoint on the DJ PC without exposing the token and confirm strict preflight, active runtime version, real ACK, and reconnect snapshot recovery.
3. Implement the `1.2.0-alpha.28` ASIO PROGRAM/CUE tranche: v3 output/full-duplex bridge, logical project buses, machine-local mapping, one-stream render/SPSC, fail-closed runtime, operator UI/test outputs, deterministic tests, independent adversarial review, exact native build, and packaging proof.
4. Perform physical Pedal 1/2/3 acceptance with Rekordbox MIDI In: any-Deck title admission/fallback, Stage 1 HPF/fade/stop plus same-edge Release, source-loop release and visible lighting event, one-bar Follow with destination first-measure hold, Stage 2 absolute toggle/loop-half/+4, ACK/reject/retry, operator-return, and reconnect snapshot recovery.
5. Verify serial DMX through the operator-selected actual USB interface and physical fixtures. Do not assume a fixed COM number on the show PC, and serialize output ownership against Daslight without terminating Daslight implicitly.
6. Perform MOTU M4 at exact 48 kHz and M32/DL16 physical acceptance. PROGRAM must reach only M4 1/2 -> DL16 5/6 -> M32 Ch18/19 -> Main/Broadcast; CUE must reach only M4 3 -> DL16 7 -> M32 Ch20 -> IEM 4/5/6 and remain absent from Main/Broadcast/Floor.
7. Update this handoff with physical evidence and exact artifact identities, then create meaningful commits and push both checkouts. Hardware, real ACK, serial DMX, ASIO device, M32 routing, reconnect, and post-freeze native acceptance remain explicitly unverified until observed.

## First safe resume actions

- Do not regenerate the final show from alpha3 or deploy superseded alpha4-alpha8 reference candidates; alpha9 is the reviewed reference-audio candidate.
- Keep the current Syndocal/Rekordbox/DJ Link processes alive until immediately before the exact alpha.27 native release build boundary.
- Re-run `git status --short`, verify branch/HEAD/upstream equality, and inspect every owned diff before versioning or committing.
- Preserve the operator-owned DVC, all token material outside the checkout, and existing QA artifacts.
