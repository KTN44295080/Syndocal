# Control-plane runtime snapshot reader — 2026-09-10

## Scope

This checkpoint removes one unnecessary full public `EngineSnapshot` clone from
the control-plane query capture path. It does not change the query wire
schemas, project/output capture, runtime generation values, IPC commands, or
the existing two-capture equality/fail-closed behavior.

The real-file thumbnail missing → Retry → recovery trial was not rerun. This
checkpoint does not claim native UI recovery, physical output, hardware,
external-client, Mac, signing, publication, or product-wide completion.

## Implementation

`EngineHandle::control_plane_runtime_snapshot()` reads the nine runtime fields
needed by `control_plane_query::capture_source_once` while holding one existing
snapshot read guard. It derives the active flags for Timeline transport,
Follow, loop, clip slots, and transitions without cloning authored fixtures,
cues, media, or authored video. The existing public `snapshot()` and
`try_snapshot()` APIs, poison defaults, and their ownership semantics are
unchanged.

## Validation

All Cargo commands used the documented Windows procedure: MSVC 14.44.35207
Build Tools x64, absolute linker pin, and `where.exe link.exe` first-match.

| Check | Result |
| --- | --- |
| `rustfmt --check --edition 2021` on changed Rust files | PASS |
| `cargo test -p engine --release --locked snapshot_public_tests -- --test-threads=1` | PASS — 3 passed, 1 existing measurement test ignored |
| `cargo test -p syndocal --release --locked control_plane::tests -- --test-threads=1` | PASS — 30 passed, 0 failed |
| `git diff --check` | PASS |

The required Windows native wrapper checker passed with 243 assertions and 27
hostile mutation fixtures. `pnpm.cmd --dir app tauri build --no-bundle` then
completed successfully with the exact pinned linker. The resulting checkout
EXE was 64,545,280 bytes with SHA-256
`BDCB2EB83189D9BFC24F1BEF3E20CF7E6C8B3A1BF06EA2F5EF471C00203CE975`.
Launching that exact path produced one `Syndocal` main window with
`Responding=True`; it was maximized before the check and the exact process was
terminated and verified absent afterward. This is a startup/window proof only,
not a thumbnail recovery or physical-output proof.

The added engine test compares every projected field with the corresponding
published snapshot field. A representative-show performance benchmark was not
run because no preserved `.sdc` input was available in this checkpoint; the
change therefore claims the structural clone reduction, not a measured FPS,
CPU, or lock-wait percentage.

## Remaining boundary

Tick-time snapshot construction, authored/rendered video duplication, deep
delta comparison, and representative-show writer-wait measurement remain
separate work. The completion ledger remains unchanged.

## Timeline Loop authority follow-up

The Loop authority issuer now consumes the same narrow runtime reader for its
transport, loop, and Follow generations. This removes its unrelated full
snapshot clone while preserving the exact fence fields and validation order.

After that source change, the pinned Windows focused suite remained green:
`cargo test -p syndocal --release --locked control_plane::tests --
--test-threads=1` — 30 passed, 0 failed, with no first-party compiler warning.
The wrapper checker again passed (243 assertions, 27 hostile mutation
fixtures), and a fresh `tauri build --no-bundle` produced a 64,544,768-byte
EXE with SHA-256
`8234529A271A7147A1C1E47FB6595615713CF1041D5B98278F09DCD19CEC8E71`.
The exact executable launched one responsive `Syndocal` window, was maximized,
and was terminated and verified absent by exact path.

## Output-control validation follow-up

The current output-control validation now uses the existing narrow output-list
reader for `AddDisplay` and `SetDisplayWindowOpen`; the composition-assignment
path remains on the full snapshot because it also validates authored
compositions. This does not alter output ownership, confirmation, receipt, or
fail-closed decisions.

After this source change, the pinned Windows `control_plane::tests` suite again
passed 30/30 with no first-party compiler warning. A fresh
`tauri build --no-bundle` produced a 64,545,280-byte EXE with SHA-256
`A955DFA2FD6028B83D52604879AF5CC73C4DE78D4B8A255A2AF3546DDC8CB502`.
The exact executable launched one responsive, maximized `Syndocal` window and
was terminated and verified absent by exact path.

## Authored transition-bus reader follow-up

The video-effect-catalog entry point now reads only the authored transition-bus
definitions it carries forward, instead of cloning the complete public engine
snapshot. The catalog mutation contract and its single published admission are
unchanged; runtime transition state remains owned by the existing runtime
reader. This is a structural clone reduction, not a measured show-performance
claim.

The focused reader regression was strengthened with a non-empty authored bus
fixture and rerun under the same pinned MSVC environment:
`cargo test -p engine --release --locked snapshot_read_tests --
--test-threads=1` — 4 passed, 1 existing synthetic benchmark ignored, 0 failed.
The consumer suite remained green at 30/30 in `control_plane::tests`. The
native build immediately before the test-only fixture extension succeeded via
the maintained wrapper and produced a 64,545,280-byte EXE with SHA-256
`8B29F4499834358868276712A3FE7222A3992173C8B1FDBC1CE108BEAB86BF9E`.
That exact executable launched one responsive `Syndocal` window and was cleaned
up by exact path. The fixture-only extension is test configuration and does
not alter the release executable inputs.

## Output-composition validation reader follow-up

The output-composition assignment validator now reads only the authored output
and composition collections it needs, under one publication guard. Output
existence, composition existence, and the existing rejection messages remain
unchanged; no output is enabled or opened by this change. This is a structural
clone reduction, not a physical-output or measured show-performance claim.

After this reader change, the pinned Windows focused suites passed again:
`cargo test -p engine --release --locked snapshot_read_tests --
--test-threads=1` — 4 passed, 1 ignored, 0 failed; and
`cargo test -p syndocal --release --locked control_plane::tests --
--test-threads=1` — 30 passed, 0 ignored, 0 failed. The maintained Tauri
wrapper then completed `pnpm --dir app tauri build --no-bundle` with the exact
MSVC linker. The current EXE is 64,546,816 bytes with SHA-256
`FBF710AD11D9C9C7CD35FD1F30018CE04B91D9CD697BC3879070A7EA5B5A296D`.
Launching that exact path produced one responsive `Syndocal` window and exact
process cleanup succeeded.

## Show Spout validation reader follow-up

The two production Show Spout pair-verification paths now read the published
video-output list through the existing narrow reader. They retain the same
exact-pair decision, unresolved-retirement barrier, and fail-closed behavior;
no sender is constructed or physical output is enabled by this change. The
serial-DMX route is covered by a separate paired DMX/output reader follow-up
below; no sender is constructed or physical output is enabled by either
change.

The default-feature Show Spout regression suite passed 49/49 with no ignored
tests. A fresh pinned `pnpm --dir app tauri build --no-bundle` then produced a
64,547,328-byte EXE with SHA-256
`3557C58F02616C0D666C47576C54A647F9C24F8118F7ED7EB36D90C0FA641760`.
Launching that exact executable produced one responsive `Syndocal` window and
exact process cleanup succeeded. No sender registration or physical output
acceptance was performed.

## Serial-DMX route validation reader follow-up

The managed serial-DMX admission check now reads the DMX route collection and
canonical output configuration together through a narrow reader, instead of
cloning the complete engine snapshot. The exact one-route check, canonical
route equality, error messages, and fail-closed default on poisoned
publication remain unchanged. No serial worker was started and no physical
DMX bytes were emitted.

The reader regression suite passed:
`cargo test -p engine --release --locked snapshot_read_tests --
--test-threads=1` — 4 passed, 1 existing synthetic benchmark ignored, 0
failed. The pinned Windows MSVC 14.44.35207 app suite passed 5/5:
`cargo test -p syndocal --release --locked show_serial_dmx --
--test-threads=1`.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed after TypeScript/Vite and Rust release compilation. The resulting
EXE is 64,552,960 bytes with SHA-256
`9AF10A2007ACA9FE91C223444A6DD491DB7957DF2FE408C1FE8EB80A2A74C6CE`.
Launching that exact path produced one maximized, responsive `Syndocal`
window; exact-path cleanup left zero matching processes.

This is a structural snapshot-clone reduction and local Windows native gate
only. It does not claim serial-DMX device, Art-Net, physical-output, venue, or
product-wide acceptance.

## Timeline-audio allocator reservation reader follow-up

The enqueue-time `SetTimelineAudio` allocator reservation now reads only the
published audio-presence flag, audio-clip emptiness, and projected timeline
layers through the narrow snapshot reader. It no longer clones the complete
public `EngineSnapshot` for this legacy derived-layer reservation. The public
timeline projection is still the source of the layer list, so implicit and
derived layer semantics, ID boundaries, and fail-closed reservation behavior
are unchanged. This is a structural allocation reduction, not a measured
show-performance claim.

The new reader regression passed 1/1, and the focused release allocator suite
passed 23/23. The reader suite passed 5/5 with 1 existing synthetic benchmark
ignored. The pinned Windows MSVC 14.44.35207 app control-plane suite passed
30/30:
`cargo test -p syndocal --release --locked control_plane::tests --
--test-threads=1`.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed after TypeScript/Vite and Rust release compilation. The resulting
EXE is 64,553,472 bytes with SHA-256
`71B5BFBD6916345F11AF55764E58E963B44EB20A4B6430C49106EBB22565EB3B`.
Launching that exact executable produced one maximized-requested,
responsive `Syndocal` window with title `Syndocal`; exact-path cleanup left
zero matching processes.

This remains a local Windows native gate and allocator/read-model regression.
It does not claim audio-device, physical-output, thumbnail-recovery, venue,
or product-wide acceptance.

## Stage-map preset allocator reservation reader follow-up

The enqueue-time `ApplyStageMapPreset` and authored stage-project mutation
reservation paths now read only the selected preset's stage-object collection
through the narrow snapshot reader. Existing trim-before-match, first-match,
missing-label no-op, and allocator observation behavior remain unchanged. The
reader returns a copied selected collection only; it does not expose or mutate
the published snapshot. This is a structural allocation reduction, not a
measured show-performance claim.

The reader and reservation regressions passed 2/2, the reader suite passed
6/6 with 1 existing synthetic benchmark ignored, and the focused allocator
suite passed 25/25 under the pinned Windows MSVC 14.44.35207 environment.
The reservation test includes a whitespace-padded label and stage-object ID
41, preserving the expected next allocator value 42.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed after TypeScript/Vite and Rust release compilation. The resulting
EXE is 64,555,520 bytes with SHA-256
`8C61BAD547BC76E480DD4ED47983DF7A6881AE33686487FCFF9634D79F52D91D`.
Launching that exact executable produced one maximized-requested,
responsive `Syndocal` window with title `Syndocal`; exact-path cleanup left
zero matching processes.

This remains a local Windows native gate and allocator/read-model regression.
It does not claim physical stage-map output, venue, thumbnail-recovery, or
product-wide acceptance.

## USB RDM serial-port safety reader follow-up

The USB RDM request and discovery admission checks now read only the primary
DMX output and managed DMX route collection through the existing narrow reader.
The conflict predicate, case-insensitive serial-port match, fail-closed
behavior, and user-facing error remain unchanged. No RDM request or device
discovery was opened; this is a snapshot-clone reduction around the safety
check, not physical I/O acceptance.

The focused pinned Windows MSVC 14.44.35207 app suite passed 3/3:
`cargo test -p syndocal --release --locked art_rdm_request_tests --
--test-threads=1`. The added regression covers primary-route conflict,
managed-route conflict, and a non-conflicting port.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed after TypeScript/Vite and Rust release compilation. The resulting
EXE is 64,580,608 bytes with SHA-256
`35A87704A9DEB573BA6E8CA574219DF8426892D535C6A767D9539CF191BEA17E`.
Launching that exact executable produced one maximized-requested,
responsive `Syndocal` window with title `Syndocal`; exact-path cleanup left
zero matching processes.

This remains a local Windows native safety-gate regression. It does not claim
USB RDM hardware, serial-DMX output, venue, or product-wide acceptance.

## Auto VJ handoff reader follow-up

The manual `take_video_clip` handoff now reads only the published
`AutoVjAction` required to suppress an already-observed automatic audio
handoff. It no longer clones the complete public engine snapshot after the
visual Take publication. The existing action identity, recovery-layer choice,
and poison/default behavior are unchanged; no audio device is opened by this
reader change.

The focused engine reader regression passed `6/6` with one existing synthetic
benchmark ignored. The app Audio playback regression passed `86/86` with no
ignored tests. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures; release self-tests passed the metadata (`137`),
ASIO packaging (`169`), Windows candidate extractor (`43`), materialization
(`4`), Windows release artifact (`144`), and strict JSON (`130`) assertion
groups. `git diff --check` passed. Individual rustfmt checks for the changed
engine files passed; the repository-wide `cargo fmt --all -- --check` still
reports two pre-existing formatting differences in
`app/src-tauri/src/control_plane_runtime.rs` and
`app/src-tauri/src/fixture_profile_contract.rs`, outside this change.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,580,608 bytes with SHA-256
`EBECD9A62D88C1EF05B486AE436AB0FBE693A1A9164620C1496F319A82AA48E7`.
The isolated native probe at
`target/qa/native-final-validation-20260910-06/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

## Video-output existence reader follow-up

The eight legacy video-output configuration/mapping command adapters now check
one output ID through `EngineHandle::video_output_exists()` instead of cloning
the complete public `EngineSnapshot`. The existing missing-output error,
legacy-route rejection, output lease path, and native output lifecycle are
unchanged; a poisoned publication remains fail-closed as “not found.”

The focused app boundary regression passed `1/1` for missing output IDs. The
engine snapshot-reader regression passed `6/6` with one existing synthetic
benchmark ignored, including present IDs, replacement, and poison/default
checks. Changed engine files passed the individual rustfmt check and
`git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,565,760 bytes with SHA-256
`63D0E6A2C8D1198732DC3BCD16092E95D85109F43E15D4DF4C73658E0DE4F9FA`.
The isolated native probe at
`target/qa/native-final-validation-20260911-02/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim video-output hardware/display acceptance,
physical output, device, Mac, signing, publication, or product-wide
completion.

## Fixture-group validation reader follow-up

`set_group_fixture_limits` now reads only the published fixture group-ID
lists through `EngineHandle::fixture_group_ids_snapshot()`. The existing
group normalization/matching and missing-group rejection remain in the app
adapter; poison/default behavior is fail-closed as an empty membership set.
Fixture payloads and unrelated public/runtime collections are no longer
cloned for this validation.

The focused engine snapshot-reader regression passed `6/6` with one existing
synthetic benchmark ignored, including published-value, replacement, and
poison/default checks for the group-membership reader. Changed engine files
passed the individual rustfmt check and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,563,200 bytes with SHA-256
`769D058089319337B2742ABF62BE981BA5CE9C0ACEB69854755239C174A0AA3E`.
The isolated native probe at
`target/qa/native-final-validation-20260911-01/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim physical output, device, Mac, signing,
publication, or product-wide completion.

The real-file missing → Retry → recovery trial was not rerun; its existing
isolated evidence remains separately recorded. This checkpoint does not claim
thumbnail recovery, physical output, device, Mac, signing, publication, or
product-wide completion.

## Native layer-thumbnail reader follow-up

The native `get_video_layer_thumbnail` worker now reads the published
`VideoSnapshot` and BPM together through one narrow snapshot-reader call. It
preserves the same publication pairing used by the renderer while avoiding a
clone of unrelated `EngineSnapshot` state. Cancellation, renderer ownership,
layer selection, frame-provider BPM, and fail-closed poison/default behavior
remain unchanged.

The focused engine reader regression passed `6/6` with one existing synthetic
benchmark ignored. The native thumbnail regression passed `5/5`, covering
real PNG pixels and dimensions, real-file video-frame positioning, copy
isolation/catalog replacement, worker admission, and cancellation. Individual
rustfmt checks for the changed engine files and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,579,072 bytes with SHA-256
`169CEB01730A70CBEB7E6510873DCFB334DEAE4A01A95A402F08968FE7032420`.
The isolated native probe at
`target/qa/native-final-validation-20260910-07/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

The real-file missing → Retry → recovery trial was not rerun; its existing
isolated evidence remains separately recorded. No PNG was moved. This
checkpoint does not claim thumbnail recovery, physical output, device, Mac,
signing, publication, or product-wide completion.

## Show Spout content-state reader follow-up

The existing-pair NoOp branch of the Show Spout activation transaction now
reads only the published Timeline `playing` flag through
`EngineHandle::timeline_playing()`. It no longer clones the complete public
engine snapshot for content-state synchronization. The transaction's exact
pair validation, output lease/authority checks, fail-closed behavior, and
physical sender lifecycle are unchanged.

The focused Show Spout regression suite passed `49/49` with no failures under
the pinned Windows MSVC 14.44.35207 environment. A fresh maintained-wrapper
`pnpm.cmd --dir app tauri build --no-bundle` completed. The exact checkout
executable is 64,579,072 bytes with SHA-256
`4194BB2BE7540AE961984169A76A86A00D25B77578538BAC370F2B58FE21984E`.
The isolated native probe at
`target/qa/native-final-validation-20260910-08/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This is a local reader and transaction regression only. No physical Spout
sender was enabled, and this checkpoint does not claim physical output,
device, Mac, signing, publication, or product-wide completion.

## Stage Map transaction reader follow-up

The admitted `save_stage_map_preset` transaction now uses the same narrow
`stage_objects_snapshot()` reader when constructing its canonical request.
The ticket admission, request digest, replay behavior, canonicalization,
publication acknowledgement, and fail-closed transaction boundaries are
unchanged; only the unrelated snapshot clone is removed from this read.

The focused Stage Map app regression passed `3/3` with no failures. The
engine reader regression remained `6/6` with one existing synthetic benchmark
ignored, and the changed engine files passed the individual rustfmt check.
`git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,561,152 bytes with SHA-256
`F44C62CB5892BB19E9C6CF2FAB645F1C6C6253604413D6F6FC92A9A3DD72FF74`.
The isolated native probe at
`target/qa/native-final-validation-20260910-10/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim file-dialog interaction, physical output,
device, Mac, signing, publication, or product-wide completion.

## Cue palette-target admission reader follow-up

The `set_cue_palette_targets` admission path now reads the authored cue,
palette, and fixture IDs through one narrow `EngineHandle` publication-
generation reader. Existing missing-cue, missing-palette, missing-fixture,
duplicate-target, and target-limit rejection behavior is unchanged; the
command no longer clones the complete engine snapshot for this admission
check.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the cue/palette/fixture projection in the shared
read-model comparison. The app release structural regression passed `1/1`.
The two changed engine files passed individual rustfmt checks, `git diff
--check` passed, and the maintained wrapper checker passed `243` assertions
with `27` hostile mutation fixtures. The full app rustfmt check remains
baseline-noisy because of unrelated pre-existing formatting differences in
`main.rs`; no broad formatting rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,626,688 bytes
with SHA-256
`B9AAC84A2EC122D362392F9FAA61D03225A97D9043CED629997F9739BB87F90F`.
The isolated native probe at
`target/qa/native-final-validation-20260911-28/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Video-layer launch-state reader follow-up

The legacy `launch_video_clip` admission path now reads only the selected
published layer state through `EngineHandle::video_layer_state_snapshot()`.
Missing-layer rejection, fade-state construction, command ordering, and
poison/default fail-closed behavior are unchanged. Preview reconciliation and
other multi-collection video paths remain on their existing full snapshot
read.

The focused engine snapshot-reader regression passed `6/6` with two existing
ignored tests, including present/replacement and poisoned-publication checks
for the new state reader. The app release `video` regression passed `139/139`
with six existing ignored tests. Changed engine files passed the individual
rustfmt check and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,569,344 bytes with SHA-256
`CA61C46736BD46F43B9446F20AAFAAAD8E7234C59778656AFE0C3689B205221C`.
The isolated native probe at
`target/qa/native-final-validation-20260911-04/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## Video-layer stop-state reader follow-up

The legacy `stop_video_clip` admission path now reuses the selected published
layer-state reader instead of cloning the complete public snapshot. Missing-
layer rejection, stopped-state construction, fade ordering, and
poison/default fail-closed behavior are unchanged. AB-mix and audio-monitor
paths remain separate because they require additional fields or coordinated
side effects.

The app release `video` regression passed `139/139` with six existing ignored
tests. The maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,569,856 bytes with SHA-256
`28B8253CBD5879AD4A3B8FC18666CD887153E02D26A4D76BD9D18A176C6AFD3D`.
The isolated native probe at
`target/qa/native-final-validation-20260911-05/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## Video-layer A/B state-pair reader follow-up

The legacy `set_video_ab_mix` path now reads the requested A/B layer states
under one publication guard through `EngineHandle::video_layer_states_snapshot()`.
The distinct-layer check, missing-layer rejection, opacity calculation, send
ordering, and poison/default fail-closed behavior are unchanged. The paired
reader prevents two independent reads from mixing publication generations.

The engine snapshot-reader regression passed `6/6` with two existing ignored
tests. The app release `video` regression passed `139/139` with six existing
ignored tests. Changed engine files passed the individual rustfmt check and
`git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,761,856 bytes with SHA-256
`009BE3AFD32C393323558E8F3191054FBA5949FE2F884AE8FEE61D7661DD182C`.
The isolated native probe at
`target/qa/native-final-validation-20260911-06/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## Video-layer audio-monitor reader follow-up

The legacy `play_video_layer_audio_monitor` admission path now reads the
selected layer state, source, and Auto VJ last action through
`EngineHandle::video_layer_audio_monitor_snapshot()` under one publication
guard. Reverse-speed rejection, the local-file requirement, audio-play
arguments, handoff suppression, missing-layer rejection, and poison/default
fail-closed behavior are unchanged.

The engine snapshot-reader regression passed `6/6` with two existing ignored
tests. The app release `video` regression passed `139/139` with six existing
ignored tests. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures, and `git diff --check` passed. The repository-
wide `cargo fmt --all -- --check` still reports pre-existing formatting deltas
outside this focused change; no unrelated formatting was rewritten.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,765,952 bytes with SHA-256
`FDC8EB139735F4414D3598E3332C0D3924408625ABF40BF162E4201639E8A8A9`.
The isolated native probe at
`target/qa/native-final-validation-20260911-07/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## Display output window query reader follow-up

The read-only Display output window observation and status queries now use
the existing `EngineHandle::video_outputs_snapshot()` reader instead of
cloning the complete public engine snapshot. Output filtering, app-owned
window lookup, native-handle validation, ownership status, live-window truth
reconciliation, and fail-closed behavior are unchanged. Compound output
activation, Spout validation, and recording paths remain on their existing
full or multi-collection reads.

The app release `video` regression passed `139/139` with six existing ignored
tests. The maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,766,464 bytes with SHA-256
`9B6B4721FE19EC5F31BBA249A945B0B9BB26F161800832A9F8276795A2D1EADC`.
The isolated native probe at
`target/qa/native-final-validation-20260911-08/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## Show Spout candidate reader follow-up

The Windows x64 Show Spout candidate, activation validation, reset
validation, and final confirmation paths now read the authored video outputs
and compositions through the existing
`EngineHandle::video_outputs_and_compositions_snapshot()` reader. The exact
pair/composition validation, allocation boundary, ownership fences,
fail-closed errors, and physical sender lifecycle remain unchanged; only the
unrelated public snapshot clone was removed.

The app release `video` regression passed `139/139` with six existing ignored
tests. The maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,768,000 bytes with SHA-256
`423C13D1FE10360D8B5FA006EF5302B480DD0D5869034B60A4BE300A45F4FFAC`.
The isolated native probe at
`target/qa/native-final-validation-20260911-09/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Spout sender/display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Video plan query reader follow-up

The read-only composition-plan, video-output-render-plan, and external-video
I/O-plan queries now read only the published `VideoSnapshot` through
`EngineHandle::video_snapshot()`. Their plan builders, runtime-status input,
validation errors, and fail-closed behavior are unchanged. Preview diagnostics
and runtime/persistence paths that require additional cross-collection state
remain on their existing readers.

The focused engine snapshot-reader regression passed `6/6` with two existing
ignored tests. The app release `video` regression passed `139/139` with six
existing ignored tests. The maintained wrapper checker passed `243` assertions
with `27` hostile mutation fixtures, and `git diff --check` passed. The two
changed engine files passed the individual rustfmt check.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,769,536 bytes with SHA-256
`9E0E59092FF193FB8B03AAF391BF2B10689A46E2345613816546B800AEB21707`.
The isolated native probe at
`target/qa/native-final-validation-20260911-10/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## External video transport sync reader follow-up

The external video transport ownership-sync helper now accepts only the
published `VideoSnapshot`; its callers use `EngineHandle::video_snapshot()`
instead of cloning the complete `EngineSnapshot`. Route-plan construction,
generic Spout filtering, NDI/Spout/capture-driver synchronization, output
ownership admission and fences, fail-closed handling, and cleanup are
unchanged.

The app release `video` regression passed `139/139` with six existing
ignored tests. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,769,536 bytes with SHA-256
`5F9198F7AF4C391152EB72E9A1EFB532EA19397FC145EB28CD9A3FEB20F2E342`.
The isolated native probe at
`target/qa/native-final-validation-20260911-11/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Native video output window reader follow-up

The debug test-pattern query, Display-window sync/open output lookup, and
stale native-window retirement label calculation now read the published
`VideoSnapshot` instead of cloning the complete `EngineSnapshot`. Display
monitor identity validation, output ownership admission and transition
guards, native window creation/retirement cleanup, test-pattern rendering,
worker fences, and fail-closed behavior are unchanged.

The app release `video` regression passed `139/139` with six existing
ignored tests. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures, and `git diff --check` passed. A standalone
`rustfmt --check` invocation still reports existing repository-wide
formatting deltas outside this focused change; no unrelated formatting was
rewritten.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,769,536 bytes with SHA-256
`2749EE251AB89BE372B42991F1DF24CCF99954528D5F1F0C5FBA2AA11A51F91F`.
The isolated native probe at
`target/qa/native-final-validation-20260911-12/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Video recording admission reader follow-up

Video recording admission and its monitored-layer audio-input selection now
read only the published `VideoSnapshot` rather than cloning the complete
`EngineSnapshot`. Output dimension/name validation, composition/layer
membership filtering, enabled/playing/volume checks, audio position/speed
normalization, worker startup, external capture inputs, and recording
lifecycle ownership are unchanged.

The app release `video` regression passed `139/139` with six existing
ignored tests, including the recording audio-input selection regression. The
maintained wrapper checker passed `243` assertions with `27` hostile mutation
fixtures, and `git diff --check` passed. A standalone `rustfmt --check`
invocation still reports existing repository-wide formatting deltas outside
this focused change; no unrelated formatting was rewritten.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,769,536 bytes with SHA-256
`FF58BA72B2DAE0AA9688B05550874662FA6061C98E34969C347D37F95FE90753`.
The isolated native probe at
`target/qa/native-final-validation-20260911-13/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Video preview reader follow-up

Video preview diagnostics now read the published `VideoSnapshot` directly.
The debug layer preview reads the published `VideoSnapshot` and BPM together
through `video_layer_thumbnail_snapshot()`. Queue inspection, decode-budget
calculation, provider BPM assignment, preview rendering, output-preview
planning, and existing renderer locking are unchanged. VJ preview and live
monitor paths that require transport/transition consistency remain on their
existing full snapshots.

The app release `video` regression passed `139/139` with six existing
ignored tests. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures, and `git diff --check` passed. A standalone
`rustfmt --check` invocation still reports existing repository-wide
formatting deltas outside this focused change; no unrelated formatting was
rewritten.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,769,536 bytes with SHA-256
`78C4BDE338612EB68B3F873EA9D8D214080A22201229A60A47E405FA2529E7C2`.
The isolated native probe at
`target/qa/native-final-validation-20260911-14/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Debug output preview reader follow-up

The debug output preview now reads the published video image, clip runtime,
transition runtime, and BPM through
`EngineHandle::video_output_preview_snapshot()` under one publication guard.
It still samples the output ownership epoch before the reader and preserves
output mapping/effects/transitions, decode-budget selection, renderer locking,
and fail-closed behavior. The production full snapshot capture used by native
Display/Timeline Follow paths remains unchanged because it carries timeline
generation and Follow validation identity.

The engine snapshot-reader regression passed `6/6` with two existing ignored
tests. The app release `video` regression passed `139/139` with six existing
ignored tests after the route-specific seam contract was updated. The
maintained wrapper checker passed `243` assertions with `27` hostile mutation
fixtures, and `git diff --check` passed. The changed engine files passed
individual rustfmt checks. The release build completed with the pinned MSVC
14.44.35207 Build Tools linker without warnings. The exact checkout
executable is 64,773,120 bytes with SHA-256
`4177D7D406EFD83E907593E009C56F5B8A1F555FFCA36B7DBF6430063B613EEB`.
The isolated native probe at
`target/qa/native-final-validation-20260911-15/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Engine telemetry report reader follow-up

The telemetry report commands now read only the counts, clock, DMX routes, and
telemetry needed by the report through
`EngineHandle::engine_telemetry_snapshot()`. The existing report schema,
budget calculations, save dialog, async worker boundary, and diagnostic-package
full snapshot path are unchanged. The reader retains one publication guard and
does not change output ownership or physical-output behavior.

The engine snapshot-reader regression passed `6/6` with two existing ignored
tests. The app telemetry regression passed `2/2`, covering report content and
the source contract that both Tauri commands use the narrow reader without a
full `engine.snapshot()` clone. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures. Changed engine files passed
individual rustfmt checks and `git diff --check` passed. The release build
completed with the pinned MSVC 14.44.35207 Build Tools linker without warnings.
The exact checkout executable is 64,771,072 bytes with SHA-256
`4280D3B725F559A66594B1045848232464DE7A428074955CC012AC01722261B3`.
The isolated native probe at
`target/qa/native-final-validation-20260911-16/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Fixture profile health reader follow-up

The fixture-profile health query now reads only the published fixture summaries
through `EngineHandle::fixtures_snapshot()`. GDTF file inspection, custom
profile locking, fixture ordering, embedded/snapshot detection, and
healthy/warnings/fallback/missing classification remain unchanged. The
full-snapshot helper is retained for its existing unit coverage and is
test-only; production no longer clones unrelated project and runtime
collections for this read-only query.

The engine snapshot-reader regression passed `6/6` with two existing ignored
tests. The fixture-profile and fixture-patch regression passed `49/49`.
The operator feature-fader regression passed `2/2`.
Together these regressions cover the existing profile conversion, DMX
self-exclusion/conflict and invalid-value rejection behavior, existing
operator batch-command behavior,
and all four narrow-reader command contracts. The maintained wrapper checker
passed `243` assertions with `27`
hostile mutation fixtures. The changed main source remained formatted in the
targeted check; the repository-wide check still reports pre-existing formatting
differences in `control_plane_runtime.rs` and `fixture_profile_contract.rs`.
`git diff --check` passed. The release build completed with the pinned MSVC
14.44.35207 Build Tools linker without warnings. The exact checkout executable
is 64,778,752 bytes with SHA-256
`2CD5E1B98A14AB3F877D85CDB7A57F186EA03F58F7DE9653A7968915DA093316`.
The isolated native probe at
`target/qa/native-final-validation-20260911-20/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

The adjacent `use_fixture_profile` command now uses the same fixture-only
reader when rebuilding a selected patched fixture as a memory profile. Its
profile conversion and missing-fixture rejection behavior are unchanged.
The `set_fixture_patch` validation and operator feature-fader command also use
the fixture-only reader; DMX conflict checks and generated batch commands are
unchanged. The focused app source contract covers all four narrow-reader
command paths.

## Timeline layer allocator reader follow-up

The `add_timeline_layer` order allocator now reads only the authored timeline
layer collection through `EngineHandle::timeline_layers_snapshot()`. Layer
ordering, next-order selection, ID allocation, and the existing publication
path are unchanged; unrelated project and runtime collections are no longer
cloned for this command.

The engine snapshot-reader regression passed `7/7` with two existing ignored
tests, and the focused app source contract passed `1/1`. The maintained wrapper
checker passed `243` assertions with `27` hostile mutation fixtures. The release
build completed with the pinned MSVC 14.44.35207 Build Tools linker without
warnings. The exact checkout executable is 64,779,264 bytes with SHA-256
`7678A395F887310D1FCA7D52A48CBEBE60FE725B6DA4DDE9168BDB0E445EEFDB`.
The isolated native probe at
`target/qa/native-final-validation-20260911-21/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Timeline cue-event admission reader follow-up

The `add_timeline_cue_event` and `set_timeline_cue_event` admission paths now
read only the published cue and timeline-event ID collections through the
narrow engine readers. Missing-cue and missing-event rejection, timing
validation, event ID allocation, and the existing publication paths are
unchanged. The replacement path captures both ID collections from one
publication generation so the two checks cannot mix snapshots.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the cue/event projections in the shared read-model
comparison. The app release structural regression passed `2/2`; changed
engine files passed the individual rustfmt checks, `git diff --check` passed,
and the maintained wrapper checker passed `243` assertions with `27` hostile
mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,783,360 bytes
with SHA-256
`7D7F7238A0320C2D566B0BBDC53C9E1F44F1D783F58B424F3725E3C4D2306106`.
The isolated native probe at
`target/qa/native-final-validation-20260911-22/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Timeline scene-block admission reader follow-up

The `add_timeline_scene_block` and `set_timeline_scene_block` admission paths
now read only cue IDs with authored beat lengths and the authored timeline
event collection through one `EngineHandle` publication-generation reader.
Existing cue/event rejection, timing validation, jump-target validation,
event-ID allocation, and scene-block publication behavior are unchanged.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the scene-block admission projection in the shared
read-model comparison. The app release structural regression passed `2/2`;
changed engine files passed the individual rustfmt checks, `git diff --check`
passed, and the maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,789,504 bytes
with SHA-256
`927897E70674FCE7372EBEB8BAD0C02CA519599E299C86B03F5EAF7A977DC762`.
The isolated native probe at
`target/qa/native-final-validation-20260911-23/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Timeline automation enable reader follow-up

The `set_timeline_automation_enabled` admission path now reads the authored
DMX and video automation IDs through one narrow engine reader. Its missing-ID
rejection, enabled-state command, and fail-closed behavior are unchanged;
the two automation collections are combined only inside the reader while the
published snapshot generation is held.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the automation-ID projection in the shared read-model
comparison. The app release structural regression passed `1/1`; changed
engine files and the app command passed targeted rustfmt checks,
`git diff --check` passed, and the maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,791,552 bytes
with SHA-256
`D818763711CC89F32BE1E1AAF85E3AF063BBC831EF0F7828EDE15ABDB110CBFF`.
The isolated native probe at
`target/qa/native-final-validation-20260911-24/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Timeline DMX automation replacement reader follow-up

The `set_timeline_automation` admission path now reads the published fixture
profiles and authored DMX automation IDs from one narrow engine reader. The
existing missing-automation rejection, fixture/attribute validation, keyframe
validation, and publication command are unchanged; fixture and automation
data cannot be mixed across publication generations.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the DMX automation admission projection in the
shared read-model comparison. The app release command structural regression
passed `1/1`, and the existing automation group selection/rejection tests
passed `2/2`. Changed engine files and the app command passed targeted
rustfmt checks, `git diff --check` passed, and the maintained wrapper checker
passed `243` assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,796,160 bytes
with SHA-256
`7BC1938084CA56E863F6CE33F64B7B28AF0AE9B82B3B0E8E85D5436E1DF837D8`.
The isolated native probe at
`target/qa/native-final-validation-20260911-25/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Timeline video automation replacement reader follow-up

The `set_timeline_video_automation` admission path now reads the published
video-layer IDs and authored video automation IDs from one narrow engine
reader. The existing missing-automation rejection, missing-layer rejection,
keyframe validation, and publication command are unchanged; the two
collections cannot be mixed across publication generations.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the video automation admission projection in the
shared read-model comparison. The app release command structural regression
passed `1/1`; changed engine files and the app command passed targeted
rustfmt checks, `git diff --check` passed, and the maintained wrapper checker
passed `243` assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,798,720 bytes
with SHA-256
`9894B23151C06F426D0A61B7B167D71F12262EED16280B9E5A62E73BDD0DBC64`.
The isolated native probe at
`target/qa/native-final-validation-20260911-26/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Timeline audio clip replacement reader follow-up

The `update_timeline_audio_clip` admission path now reads the authored
timeline layers and the target clip's logical output bus through one narrow
engine reader. The existing missing-clip rejection, audio-lane and locked-
layer rejection, sanitization, and publication behavior are unchanged.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the audio-clip admission projection in the shared
read-model comparison. The app release command structural regression passed
`1/1`; changed engine files and the app command passed targeted rustfmt
checks, `git diff --check` passed, and the maintained wrapper checker passed
`243` assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,799,744 bytes
with SHA-256
`FB1457CDB366B66BB58BD879580BC124CEE91A095B37BC4D4BE3421F57E11589`.
The isolated native probe at
`target/qa/native-final-validation-20260911-27/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Reference-palette application reader follow-up

The `apply_reference_palette` admission path now reads the authored palette
and fixture ID collections through one narrow `EngineHandle` publication-
generation reader. Existing missing-palette and missing-fixture rejection,
empty-selection rejection, and palette application behavior are unchanged;
the command no longer clones unrelated engine state for this admission check.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the palette/fixture projection in the shared
read-model comparison. The app release structural regression passed `1/1`.
The two changed engine files passed individual rustfmt checks, `git diff
--check` passed, and the maintained wrapper checker passed `243` assertions
with `27` hostile mutation fixtures. The full app rustfmt check remains
baseline-noisy because of unrelated pre-existing formatting differences in
`main.rs`; no broad formatting rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,634,368 bytes
with SHA-256
`73E84B3BE926749317DC93AF0A51AD31C461BB96CDA6D2568E4EE7325F9BDFBF`.
The isolated native probe at
`target/qa/native-final-validation-20260911-29/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Playback Executor admission reader follow-up

The `create_playback_executor` and `update_playback_executor` admission paths
now read the authored cue-list IDs and Playback Executor summaries through
one narrow `EngineHandle` publication-generation reader. Existing invalid-ID,
missing-bank, duplicate page/slot, range, level, and update-not-found
rejection behavior is unchanged; unrelated engine collections are no longer
cloned for these admission checks.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the Playback Executor projection in the shared
read-model comparison. The app release structural regression passed `1/1`.
The two changed engine files passed individual rustfmt checks, `git diff
--check` passed, and the maintained wrapper checker passed `243` assertions
with `27` hostile mutation fixtures. The full app rustfmt check remains
baseline-noisy because of unrelated pre-existing formatting differences in
`main.rs`; no broad formatting rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,636,416 bytes
with SHA-256
`DA91E5E44DD840163559A278A7E4C426AB101992CF4D7A6BE2CAD960400F031C`.
The isolated native probe at
`target/qa/native-final-validation-20260911-30/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Show Art-Net staged route validation reader follow-up

The fail-closed validator for the pre-authored show Art-Net loopback route now
uses the existing consistent `EngineHandle::dmx_outputs_and_output_snapshot()`
reader. It still requires exactly one authored route, exact equality with the
primary output projection, disabled state, and the strict staged
Art-Net/`127.0.0.1:6454` route shape. No physical output operation was
enabled or attempted; only the snapshot copy boundary changed.

The app release structural regression passed `1/1`, the maintained wrapper
checker passed `243` assertions with `27` hostile mutation fixtures, and
`git diff --check` passed. The engine reader itself was unchanged and remains
covered by the preceding snapshot-reader regression checkpoint. The full app
rustfmt check remains baseline-noisy because of unrelated pre-existing
formatting differences in `main.rs`; no broad formatting rewrite was
included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,635,904 bytes
with SHA-256
`6EB912B7AC4C2014D07D515DAB1CFE8CF40999D18B81D46F7A61F2CD88829519`.
The isolated native probe at
`target/qa/native-final-validation-20260911-33/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## MIDI feedback projection reader follow-up

The `send_midi_feedback` command now reads the existing `EngineSnapshot`
compatibility seam through `EngineHandle::midi_feedback_snapshot()`. The
projection copies only the published fields consumed by MIDI feedback:
fixtures, cue activity, effects, node graphs, video layer/output state,
timeline transport state, BPM, masters, submasters, fade pause, and blackout
state. The existing `crates/io` feedback API is unchanged; no output device or
MIDI transport behavior was widened.

The engine snapshot-reader regression passed `8/8` with two existing ignored
tests, including field-by-field MIDI projection coverage. The app structural
guard for the `send_midi_feedback` route passed `1/1`, the existing MIDI
feedback regressions passed `18/18` with one existing ignored physical-port
test, the maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no observed
first-party compiler warnings. The exact checkout executable is 64,733,696
bytes with SHA-256
`66F45FB5E5CB7E048CCE48B29966E50A47F74CB9CF36744AB3168CA79B159684`.
The isolated native probe at
`target/qa/native-final-validation-20260911-54/native-final-validation.json`
passed: one responsive maximized `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, expected missing media/layer thumbnail
IPC rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and zero listeners on probe port `51670`.

This checkpoint does not claim real-file missing → Retry → recovery: no PNG
was moved or renamed. It also does not claim actual Art-Net/ASIO/NDI/Spout or
other device output, Mac, signing, publication, or product-wide completion.

## Project runtime reset reader follow-up

The post-publication project runtime reset now reads the existing
`EngineHandle::auto_vj_last_action()` projection instead of cloning a full
`EngineSnapshot` only to suppress the prior Auto VJ audio handoff. Reset
ordering and poison-recovery behavior are unchanged; this removes an
unrelated project/output/diagnostic clone from the publication boundary.

The app structural guard passed `1/1`, the maintained wrapper checker passed
`243` assertions with `27` hostile mutation fixtures, and `git diff --check`
passed. A fresh maintained-wrapper
`pnpm.cmd --dir app tauri build --no-bundle` completed with the pinned MSVC
14.44.35207 Build Tools linker and no observed first-party compiler warnings.
The exact checkout executable is 64,732,672 bytes with SHA-256
`673201CD1B1B6FCCE29A00905EECBB97B043CD9770BB50E02D2A21F4DEC67BC7`.
The isolated native probe at
`target/qa/native-final-validation-20260911-55/native-final-validation.json`
passed: one responsive maximized `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, expected missing media/layer thumbnail
IPC rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and zero listeners on probe port `51671`.

This checkpoint does not claim real-file missing → Retry → recovery: no PNG
was moved or renamed. It also does not claim actual Art-Net/ASIO/NDI/Spout or
other device output, Mac, signing, publication, or product-wide completion.

## Cue metadata admission reader follow-up

The `set_cue_metadata` admission path now reads the current Cue body and
same-bank cue numbering through one
`EngineHandle::cue_metadata_admission_snapshot()` publication read. Cue
existence, same-bank duplicate-number rejection, Cue Part validation, MIB
validation, and the subsequent metadata mutation contract are unchanged;
unrelated fixtures, video, effects, and runtime collections are no longer
cloned for this path.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive same-bank projection, separate-bank
numbering, missing-Cue rejection, and poisoned-publication fail-closed
behavior. The app release structural regression passed `1/1`. The changed
engine files passed individual rustfmt checks and `git diff --check` passed.
The maintained wrapper checker passed `243` assertions with `27` hostile
mutation fixtures. The full app rustfmt check remains baseline-noisy because
of unrelated pre-existing formatting differences in `main.rs`; no broad
formatting rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,645,120 bytes
with SHA-256
`E07C1E859818015409483F1FCAA718CE6576ED65149F0092FF0F1053C4B27666`.
The isolated native probe at
`target/qa/native-final-validation-20260911-34/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## NodeGraph enable reader follow-up

The `set_node_graph_enabled` admission path now checks the requested authored
NodeGraph ID through `EngineHandle::node_graph_exists()` instead of cloning the
complete engine snapshot. Its existing missing-graph rejection and the
subsequent engine-owned enable/disable mutation are unchanged. Compound graph
validation and persistence paths remain on full snapshots because they require
graph bodies and cross-collection checks.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including positive and missing-ID behavior plus poisoned
publication fail-closed defaults. The app release structural regression passed
`1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures. The full app rustfmt check
remains baseline-noisy because of unrelated pre-existing formatting
differences in `main.rs`; no broad formatting rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,635,904 bytes
with SHA-256
`3F88F1C028C391B89AAEF59BFC3B195F66B0AB1749E149639DB1382A2FC4C371`.
The isolated native probe at
`target/qa/native-final-validation-20260911-32/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Effect update kind reader follow-up

The nine effect update commands now validate the existing effect kind through
one narrow `EngineHandle::effect_kind_snapshot()` reader. Missing-effect and
effect-kind mismatch errors are unchanged; the update admission paths no
longer clone the complete engine snapshot merely to validate the kind.

The focused engine snapshot-reader regression passed `7/7` with two existing
ignored tests, including the effect-kind projection and missing-ID rejection
in the shared read-model comparison. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures. The full app rustfmt check
remains baseline-noisy because of unrelated pre-existing formatting
differences in `main.rs`; no broad formatting rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no first-
party compiler warnings. The exact checkout executable is 64,635,392 bytes
with SHA-256
`8C5C290FFE46CFFBB77D8BB8C18B279EB18CF7165373BD22C6FB398883DC281F`.
The isolated native probe at
`target/qa/native-final-validation-20260911-31/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
NDI/Spout sender or display hardware, physical output, device, Mac, signing,
publication, or product-wide completion.

## Video-layer ID reader follow-up

The `set_video_layer_order` and `add_video_composition` admission paths now
read only the published video-layer ID list through
`EngineHandle::video_layer_ids_snapshot()`. Their existing missing-layer
error, command ordering, allocation boundary, and poison/default fail-closed
behavior are unchanged. Compound validators that need other authored
collections remain on one full snapshot read to preserve cross-collection
consistency.

The focused engine snapshot-reader regression passed `6/6` with two existing
ignored tests, including published-ID replacement and poisoned-publication
checks for this reader. The app release `video` regression passed `139/139`
with six existing ignored tests. Changed engine files passed the individual
rustfmt check and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,567,296 bytes with SHA-256
`8488EE10CF0BB9B236C74F04FEE6DD7E5FDEE8CDE4B9CD9B5BD7757C112A2B8F`.
The isolated native probe at
`target/qa/native-final-validation-20260911-03/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, video
display hardware, physical output, device, Mac, signing, publication, or
product-wide completion.

## Stage Map export reader follow-up

Standalone Stage Map preset export and the admitted Stage Map preset-save
transaction now read only the authored `stage_objects` collection through
`EngineHandle::stage_objects_snapshot()`. The exported schema, object
ordering, validation, save-dialog behavior, transaction ticket/digest
boundary, and publication path are unchanged; unrelated published project
and runtime collections are no longer cloned for these Stage Map reads.

The focused engine snapshot-reader regression passed `6/6` with one existing
synthetic benchmark ignored, including published-value, replacement, and
poison/default checks for the new reader. Changed engine files passed the
individual rustfmt check and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,573,440 bytes with SHA-256
`73246D69753EA9D0BFF237C1AF2026F9E98EDE2F73EE3E697EA604923F06FE31`.
The isolated native probe at
`target/qa/native-final-validation-20260910-09/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup.

This checkpoint does not claim file-dialog interaction, physical output,
device, Mac, signing, publication, or product-wide completion.

## Video composition removal reader follow-up

The `remove_video_composition` admission path now checks the requested
authored composition ID through
`EngineHandle::video_composition_exists()`. The protected Main composition
(ID 1) rejection, missing-composition rejection, and subsequent engine-owned
mutation are unchanged. Compound video composition and layer/timeline edits
remain on full snapshots where cross-collection consistency is required.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive and missing-composition behavior and the
poisoned-publication default path. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,650,240
bytes with SHA-256
`B603EC01C612AC4C9BC33CA59FEBFED52E70DAB4ACAA0D2E7C7C7B6B26D0A125`.
The isolated native probe at
`target/qa/native-final-validation-20260911-35/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Touch Surface admission reader follow-up

The `set_touch_surface` admission path now reads only the authored fixture
summaries and Cue IDs required by Touch Surface reference validation through
`EngineHandle::touch_surface_admission_snapshot()`. Grid bounds, page/control
limits, duplicate IDs, fixture attributes, group bindings, color and Pan/Tilt
capabilities, Feature Preset ranges and duplicate targets, Cue existence, and
all existing rejection messages remain unchanged. Project-file validation
continues to use the full snapshot boundary because it validates the complete
persisted project.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including the new fixture/Cue admission projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,658,432
bytes with SHA-256
`3BB038CD89D1E4F393725F42E95988B2018137D05782A7159297C6327DEF9B87`.
The isolated native probe at
`target/qa/native-final-validation-20260911-36/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Video effect target admission reader follow-up

The `set_effect_video_target_position` admission path now reads the requested
Effect's target layer IDs and the published video-layer IDs through one
`EngineHandle::effect_video_target_admission_snapshot()` publication read.
Finite position validation, missing-Effect rejection, missing-layer
rejection, non-target-layer rejection, and the existing engine mutation are
unchanged. Effect creation, duplication, and compound target validation remain
on their existing full-snapshot paths.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive/missing Effect target projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,662,016
bytes with SHA-256
`E491C5F89E976158064E6DAEFDF66DECCEF2F4E4E0A8B5A1E0C48B95CE6354A4`.
The isolated native probe at
`target/qa/native-final-validation-20260911-37/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Effect preset export reader follow-up

The `save_effect_preset` admission/export path now reads one authored
`EffectSummary` through `EngineHandle::effect_summary_snapshot()` instead of
cloning the complete engine snapshot. Existing missing-Effect rejection,
effect-to-preset conversion, dialog behavior, serialization, and file-write
boundary are unchanged. Effect creation, duplication, and compound target
validation remain on their existing paths.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive/missing Effect body projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,667,648
bytes with SHA-256
`EF249BDB2AA680856A81380BFFD1654C9CA0A36658B11EE59498726DF73A8BFC`.
The isolated native probe at
`target/qa/native-final-validation-20260911-38/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Fixture preset export reader follow-up

The `save_fixture_preset` admission/export path now reads one authored
`PatchedFixtureSummary` through `EngineHandle::fixture_summary_snapshot()`
instead of cloning the complete engine snapshot. Existing missing-Fixture
rejection, Fixture-to-preset conversion, dialog behavior, serialization, and
file-write boundary are unchanged. Fixture creation, duplication, and
group/all matching paths remain on their existing full-snapshot paths.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive/missing Fixture projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,671,232
bytes with SHA-256
`90CF962143712778FF000AA24118D3406365FE90F27B4A78037FA77E4871914E`.
The isolated native probe at
`target/qa/native-final-validation-20260911-39/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Fixture preset load reader follow-up

The `load_fixture_preset` validation path now reads one authored Fixture
through `EngineHandle::fixture_summary_snapshot()` instead of cloning the
complete engine snapshot. Preset version, manufacturer/profile/mode and
attribute validation, missing-Fixture rejection, and the subsequent
`ApplyAttributeValues` command are unchanged. Group and all-matching preset
load paths remain on their existing full-snapshot paths.

The focused app release structural regression passed `1/1`, and the
maintained wrapper checker passed `243` assertions with `27` hostile mutation
fixtures. `git diff --check` passed; no engine reader implementation was
changed in this unit.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,675,328
bytes with SHA-256
`857810B7944912E042825E9DC2596C42314B2353B3E5AA8235F124EDF711FB3E`.
The isolated native probe at
`target/qa/native-final-validation-20260911-40/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Node Graph preset export reader follow-up

The `save_node_graph_preset_file` path now reads one authored Node Graph
through `EngineHandle::node_graph_summary_snapshot()` instead of cloning the
complete engine snapshot. Runtime telemetry removal through
`node_graph_for_persistence`, preset serialization, dialog behavior, file
writing, and missing-graph rejection are unchanged. Node Graph loading and
multi-graph validation remain on their existing full-snapshot paths.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive/missing Node Graph projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures. The full engine rustfmt check
remains baseline-noisy because of unrelated pre-existing formatting
differences in untouched engine files; no broad formatting rewrite was
included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,676,352
bytes with SHA-256
`5143FD1A8C9C3FEDF6B190CB6F5CA70419D6B34340DE1FBF1435755A43B83295`.
The isolated native probe at
`target/qa/native-final-validation-20260911-41/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Video composition layer admission reader follow-up

The `set_video_composition_layers` admission path now reads editable
composition presence and published video-layer IDs through one
`EngineHandle::video_composition_layer_admission_snapshot()` publication
read. Main-composition rejection, missing-composition rejection, missing
layer rejection, and the existing `SetVideoCompositionLayers` mutation are
unchanged. Timeline-layer composition editing and other multi-collection
video paths remain on their existing full-snapshot paths.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including the composition/layer admission projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,677,376
bytes with SHA-256
`F82FA57784FE2B8802ADA0248B95F1C9297774A2F4A3597222EAA826C7AF7E0D`.
The isolated native probe at
`target/qa/native-final-validation-20260911-42/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Video layer duplicate admission reader follow-up

The `duplicate_video_layer` admission path now reads the source layer's ISF
addition and the current project ISF source total through one
`EngineHandle::video_layer_duplicate_admission_snapshot()` publication read.
Missing-layer rejection, ISF source-size overflow rejection, the project
budget limit, label normalization, ID allocation, and the existing duplicate
mutation are unchanged. The test-only snapshot-budget helpers are explicitly
scoped to tests after this path moved to the narrow reader; no first-party
compiler warnings remain in the final native build.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including positive/missing duplicate-layer projection and
poisoned-publication default behavior. The app release structural regression
passed `1/1`. The changed engine files passed individual rustfmt checks and
`git diff --check` passed. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,678,912
bytes with SHA-256
`09A9471DFECE0FC2AC63AE8D0792813C87E8B74AFDDA9F728907075304AE3E0A`.
The isolated native probe at
`target/qa/native-final-validation-20260911-43/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Video composition timeline layer admission reader follow-up

The `set_video_composition_timeline_layers` admission path now reads editable
composition presence and all published timeline layer summaries through one
`EngineHandle::video_composition_timeline_layer_admission_snapshot()` publication
read. Main-composition rejection, missing-composition rejection, zero or
duplicate lane rejection, stale or non-Video lane rejection, and the existing
`SetVideoCompositionTimelineLayers` mutation are unchanged. The
`take_video_clip` and runtime/Preview ownership paths remain unchanged.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including the timeline-layer projection and poisoned-publication
default behavior. The app release structural regression passed `1/1`. The
changed engine files passed individual rustfmt checks and `git diff --check`
passed. The maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures. The first native build exposed one newly-unused
test-only helper; it was scoped with `#[cfg(test)]`, and the subsequent focused
app test and final native build completed with no first-party compiler warnings.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,681,472
bytes with SHA-256
`011B03C8A4CCFF0DED0927D3DD86088E311706C41EA4848F9697159CE0351C67`.
The isolated native probe at
`target/qa/native-final-validation-20260911-44/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Node Graph admission reader follow-up

The `save_node_graph` and `load_node_graph_preset_file` admission paths now
read only published fixture summaries and video-layer IDs through one
`EngineHandle::node_graph_admission_snapshot()` publication read. Node graph
shape, node/edge, fixture attribute/group, video-layer, preset version, and
application-name validation remain fail-closed; project-wide validation keeps
its existing full-snapshot boundary because it validates all authored
collections together. The node graph mutation and persistence paths are
unchanged.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests, including the node-graph projection and poisoned-publication
default behavior. The app release structural regression passed `1/1`. The
changed engine files passed individual rustfmt checks and `git diff --check`
passed. The maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures. The first native build exposed one newly-unused
test-only validation wrapper; it was scoped with `#[cfg(test)]`, and the
subsequent focused app test and final native build completed with no
first-party compiler warnings.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,677,376
bytes with SHA-256
`F774630E4A1818827950C5774D17D2BDA150025CC9B3BFA46D6974838D06C88C`.
The isolated native probe at
`target/qa/native-final-validation-20260911-45/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Fixture preset group admission reader follow-up

The `load_fixture_preset_for_group` and
`load_fixture_preset_for_all_matching` admission paths now read the published
fixture summaries through the existing `EngineHandle::fixtures_snapshot()`
reader instead of cloning the full engine snapshot. Group matching, preset
compatibility selection, skipped-count reporting, and the existing
`ApplyAttributeValues` mutations are unchanged; the file format and
fail-closed preset validation remain unchanged.

The focused fixture-preset regression passed `7/7`, including compatibility
selection, legacy-file handling, extension rejection, and the two structural
narrow-reader contracts. The maintained wrapper checker passed `243`
assertions with `27` hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,678,400
bytes with SHA-256
`88CCE77391AAD83EA8F072852D10F636AEAAC5A6F53929EE5D1116B0A02D3240`.
The isolated native probe at
`target/qa/native-final-validation-20260911-46/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Cue effect target admission reader follow-up

The `set_cue_effect_targets` admission path now reads only the published Cue
and Effect IDs through one
`EngineHandle::cue_effect_target_admission_snapshot()` publication read.
Cue existence, duplicate target rejection, legacy missing-effect rejection,
transition bounds, and Cue-owned parameter validation retain their existing
fail-closed behavior and error messages. Cue capture and other project-wide
validation paths remain on their existing full-snapshot boundary where they
validate cross-collection authored state together; the mutation and
persistence behavior is unchanged.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests. The focused app Cue/Effect regression passed `4/4`, including
save-history/legacy compatibility, Cue-owned parameter round-trip, duplicate
and unknown-target rejection, and the structural narrow-reader contract. The
maintained wrapper checker passed `243` assertions with `27` hostile mutation
fixtures; `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,687,616
bytes with SHA-256
`1C02103CD39D552D45F44347AC28B6558125C727D879C8A6749110237F525D3F`.
The isolated native probe at
`target/qa/native-final-validation-20260911-47/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Effect duplication reader follow-up

The `duplicate_effect_in_engine` path now reads the requested Effect through
the existing `EngineHandle::effect_summary_snapshot()` reader instead of
cloning the complete engine snapshot. Unknown-effect rejection, summary to
preset conversion, copy relabeling, target validation through the existing
add path, and the resulting mutation are unchanged.

The focused duplicate-effect regression passed `2/2`, and the broader
duplicate-named app regression passed `41/41`; this included LFO, Color, and
Chaser duplicate behavior plus the structural narrow-reader contract. The
maintained wrapper checker passed `243` assertions with `27` hostile mutation
fixtures, and `git diff --check` passed. The full app rustfmt check remains
baseline-noisy because of unrelated pre-existing formatting differences in
`control_plane_runtime.rs` and `fixture_profile_contract.rs`; no formatting
rewrite was included.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,687,616
bytes with SHA-256
`1977C42F1944CE6415238327EA7366B22E5C392AA2E82BD3C65E10C824C692E5`.
The isolated native probe at
`target/qa/native-final-validation-20260911-48/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Effect preset target admission reader follow-up

The `add_effect_preset_to_engine` and sample Chaser target-override paths now
read only published fixture summaries and video-layer IDs through one
`EngineHandle::effect_target_admission_snapshot()` publication read. Fixture
and video-layer reference rejection, missing-group rejection, Chaser group
expansion and patch-order remapping, target deduplication, and the existing
effect request validation remain fail-closed and behaviorally unchanged. The
unused full-snapshot video-layer validator was removed after this path moved
to the published projection; no permissive fallback was added.

The focused engine snapshot-reader regression passed `8/8` with two existing
ignored tests. The focused effect-preset regression passed `8/8`, and the
Chaser regression passed `18/18`, including the target-override and embedded
sample paths. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures; `git diff --check` passed. The first app
compile exposed one newly-unused full-snapshot helper after the narrow-reader
replacement; it was removed, and the subsequent focused app compile and
final native build completed with no first-party compiler warnings.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,688,640
bytes with SHA-256
`FAB7C3DF214383A16AC67B812D93BE4417BAAC8A4279C331BB929D3B0EAC2D75`.
The isolated native probe at
`target/qa/native-final-validation-20260911-49/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## VJ Preview video snapshot reader follow-up

The VJ Preview transport, clip lookup, layer staging, play/seek/speed
commands, and Preview monitor now read the published video projection through
`EngineHandle::video_snapshot()` or the existing
`video_layer_thumbnail_snapshot()` pair. These paths no longer acquire a full
`EngineSnapshot` merely to inspect video layers or the preview BPM. Program
monitor capture and the output fence remain on their existing full-capture
path. Source identity checks, layer validity, stale-frame rechecks, explicit
staging/authority handling, and renderer behavior remain unchanged.

The focused VJ Preview and transport regression passed `26/26`, including the
source-structure guard for the narrow readers. The maintained wrapper checker
passed `243` assertions with `27` hostile mutation fixtures, and
`git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,693,248
bytes with SHA-256
`B885EF2BBF93455CC8FD992617376D42D56CF3D7CDF50B859FBA1D0139B5FC08`.
The isolated native probe at
`target/qa/native-final-validation-20260911-50/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Project control mapping Timeline ID reader follow-up

The project-control mapping publication path now validates DJ track trigger
Timeline references through one `EngineHandle::timeline_ids_snapshot()` read.
The reader publishes the same authored Timeline ID order as the former
full-snapshot validation (`timeline_bank` followed by the current Timeline),
while the existing `validate_dj_track_triggers_against_snapshot` wrapper is
retained for project-file validation paths. Shape validation and rejection of
unknown Timeline IDs remain fail-closed; no fallback or permissive admission
was added.

The engine snapshot-reader regression passed `8/8` with two existing ignored
tests. App regressions passed: the publication structural guard `1/1`, the
normal/unknown Timeline reference fail-closed test `1/1`, and the existing
project-control mapping roundtrip test `1/1`. The maintained wrapper checker
passed `243` assertions with `27` hostile mutation fixtures, and
`git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no observed
first-party compiler warnings. The exact checkout executable is 64,721,920
bytes with SHA-256
`5A68CBA3836B08FB3DDECDD78A10C3041E0AF09E6E9758F426670FCA55A27A94`.
The isolated native probe at
`target/qa/native-final-validation-20260911-51/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Diagnostic package telemetry reader follow-up

The diagnostic ZIP export now reads the existing
`EngineHandle::engine_telemetry_snapshot()` projection instead of cloning a
full `EngineSnapshot`. The projection now includes Timeline event,
automation, and video-automation counts required by `project-summary.json`;
the existing engine telemetry report values and archive entry contract remain
unchanged. The former full-snapshot report helper is test-only because
production diagnostics now use the published telemetry projection directly.

The engine snapshot-reader regression passed `8/8` with two existing ignored
tests. App diagnostic regressions passed `12/12` with one existing ignored
subprocess test, including the required archive entries and the structural
narrow-reader guard. The maintained wrapper checker passed `243` assertions
with `27` hostile mutation fixtures, and `git diff --check` passed. An
intermediate native compile exposed one newly-unused test helper after the
reader replacement; it was constrained to `cfg(test)`, and the final native
compile emitted no first-party warnings.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,721,920
bytes with SHA-256
`8BDF1971B75FB6295C9FABD3BAB283D8AB71732C40055A323083A4CE2A961BFB`.
The isolated native probe at
`target/qa/native-final-validation-20260911-52/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Visualizer projection reader follow-up

The five read-only Visualizer query paths now use one
`EngineHandle::visualizer_snapshot()` projection containing only fixtures, DMX
preview frames, video output summaries, and stage objects. The existing
EngineSnapshot-based Visualizer API remains as a compatibility seam and is
implemented through the same projection, so scene, beam, video-surface,
stage-object, model-plan, and primitive-mesh behavior remains unchanged. No
output-control, renderer worker, or physical-output path was changed.

The engine snapshot-reader regression passed `8/8` with two existing ignored
tests. The Visualizer regression passed `23/23`, and the app structural guard
for all five query paths passed `1/1`. The maintained wrapper checker passed
`243` assertions with `27` hostile mutation fixtures, and `git diff --check`
passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker and no
first-party compiler warnings. The exact checkout executable is 64,729,088
bytes with SHA-256
`F6DBEC60D5BD2ED17E7AF8E3DF12153E519D998FD57BDEB5EA5B2412F0683886`.
The isolated native probe at
`target/qa/native-final-validation-20260911-53/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. A direct post-probe check found zero
exact-path Syndocal processes and only the expected port `TIME_WAIT` entry
owned by PID 0.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Show Spout reset paired projection reader follow-up

The serialized Show Spout reset path now reads only the paired authored video
outputs and compositions through the existing
`EngineHandle::video_outputs_and_compositions_snapshot()` projection before
and after physical sender retirement. Owner/fence admission, exact pair
classification, physical-before-engine ordering, and the fail-closed
ambiguous-pair behavior are unchanged; no physical output is enabled by this
change.

The app regression set for Show Spout reset passed `5/5`, including absent,
legacy, active, physical-retirement-failure, and the structural narrow-reader
guard. The broader narrow-reader structural set passed `31/31` before this
local change. The maintained wrapper checker passed `243` assertions with
`27` hostile mutation fixtures, and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,733,184 bytes with SHA-256
`14FE8D853E8C00ADD5D57734BA04C71030B3C70145D5FDDDDDEAC6FCA179E2C5`.
The first probe invocation was not accepted because Windows PowerShell 5.1
misread the Japanese workspace path before application launch; the same
fixed script was rerun with PowerShell 7. The successful isolated native probe
is recorded at
`target/qa/native-final-validation-20260911-56/native-final-validation.json`:
one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. The report records zero exact-path
Syndocal processes and zero remaining debug listener after cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.

## Display output admission paired projection reader follow-up

The Display output admission core now uses the existing
`EngineHandle::video_outputs_snapshot()` projection for its four serialized
output-presence checks. The checks still read the same published output list;
project authority, owner/fence and lease validation, native hidden-shell/GPU
phases, first-frame fencing, Engine publication, and cleanup ordering are
unchanged. No physical display was opened or enabled by this change.

Display-related app regressions passed `29/29`, including the structural
narrow-reader guard, exact monitor/dimension validation, stale authority
rejection, pending/in-doubt retry barriers, and native display fake-driver
paths. The canonical native-output QA driver passed `1/1`. The maintained
wrapper checker passed `243` assertions with `27` hostile mutation fixtures,
and `git diff --check` passed.

A fresh maintained-wrapper `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned MSVC 14.44.35207 Build Tools linker. The exact
checkout executable is 64,732,672 bytes with SHA-256
`9B09F4A9E075BCD50189773F74694176BF0DBFA54E08E3EDFAA61476B85356FE`.
The isolated native probe at
`target/qa/native-final-validation-20260911-57/native-final-validation.json`
passed: one maximized responsive `Syndocal` window, Standby ownership with
lighting/video disabled, snapshot IPC, missing media/layer thumbnail IPC
rejection with valid native tickets, zero physical-output operations, and
exact executable/listener cleanup. The report records zero exact-path
Syndocal processes and zero remaining debug listener after cleanup.

This checkpoint does not claim real-file missing → Retry → recovery, actual
Art-Net/ASIO/NDI/Spout/device output, Mac, signing, publication, or
product-wide completion.
