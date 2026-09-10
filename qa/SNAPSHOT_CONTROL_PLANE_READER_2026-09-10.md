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
tests. The fixture-profile and fixture-patch regression passed `49/49`,
covering the existing profile conversion, DMX self-exclusion/conflict and
invalid-value rejection behavior plus all three narrow-reader command
contracts. The maintained wrapper checker passed `243` assertions with `27`
hostile mutation fixtures. The changed main source remained formatted in the
targeted check; the repository-wide check still reports pre-existing formatting
differences in `control_plane_runtime.rs` and `fixture_profile_contract.rs`.
`git diff --check` passed. The release build completed with the pinned MSVC
14.44.35207 Build Tools linker without warnings. The exact checkout executable
is 64,778,752 bytes with SHA-256
`24048A8066CE7816D81EF13CBCCABF1C19BACC6532F2710C911A1E80AE97980E`.
The isolated native probe at
`target/qa/native-final-validation-20260911-19/native-final-validation.json`
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
The focused app source contract covers both narrow-reader command paths.

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
