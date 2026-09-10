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
