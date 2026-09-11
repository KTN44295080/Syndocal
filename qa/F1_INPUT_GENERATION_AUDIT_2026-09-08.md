# F1 input-generation audit — 2026-09-08

## Scope

This is a bounded audit of the existing project and mapping input-generation
boundary. It does not add a MIDI/OSC/DMX API, enable physical output, or close
the full F1 ledger row.

- Source base: `828bbe178f639bb3843ff9461a862c13619fa2e4`
- Working tree before this document: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing implementation checked

`app/src-tauri/src/main.rs` keeps separate callback generations for all
project-owned inputs and mapping-driven inputs. The callback seam checks the
captured generation immediately before send, refuses constructor callbacks until
the worker is installed, and uses non-blocking external admission so a worker
being retired cannot deadlock the replacement waiting to join it. Project
replacement reserves both generations before taking callback-capable input
slots; mapping replacement advances only the mapping generation and preserves
the independent MIDI Clock input. `retire_after_project_control_slots_taken`
joins/releases taken workers before publication and releases partial takes on
failure.

The same source also keeps external admission ahead of coordinator locking for
the direct MIDI/OSC/DMX/Remote mutators and the renderer-ticketed project
mutation path. Stale project epochs and generation-counter exhaustion fail
closed without reserving or publishing a mutation.

## Focused verification

The Rust commands ran from the repository root after
`vcvars64.bat -vcvars_ver=14.44`, with the exact Build Tools MSVC
`14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` and returned first by
`where.exe link.exe`.

| Command/filter | Result |
| --- | --- |
| `cargo test -p syndocal --release --locked callback_epoch -- --test-threads=1` | PASS — 3 passed |
| `cargo test -p syndocal --release --locked installed_callback_gate -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p syndocal --release --locked project_transaction_fence -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p syndocal --release --locked external_admission_ -- --test-threads=1` | PASS — 2 passed |
| `cargo test -p syndocal --release --locked project_control_retirement -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p syndocal --release --locked project_retirement -- --test-threads=1` | PASS — 6 passed |
| `pnpm --dir app run check:project-transaction` | PASS — project transaction and project authority deterministic checks |

The 14 Rust cases cover stale constructor callbacks and installation, overflow,
non-waiting retirement admission, transaction-boundary blocking, callback /
Remote / transaction baseline linearization, lock ordering, join-before-publish
and partial-take release, and project-replacement callback failure without
publication. No app process, physical output, device, or external client was
started.

## Related checker repair boundary

During this audit the existing `check-output-ownership.mjs` was also run. Its
first attempt exposed a retired source marker, `fn
stop_video_output_recording_runtime`, after that helper had moved to
`video_recording_runtime.rs`. The checker-only boundary repair is recorded
separately in `qa/OUTPUT_OWNERSHIP_CHECKER_REPAIR_2026-09-08.md` and is already
integrated as `828bbe1`; no F1 assertion was weakened.

## Remaining boundary

`F1-INPUT-GENERATIONS-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. This checkpoint proves the audited local
generation/admission seams and deterministic stale-callback behavior only. It
does not prove physical MIDI/OSC/DMX clients, device reconnect/latency, all
input worker implementations, venue operation, or the dependent F2 full output
ownership and ShowClock decisions. ASIO/NDI/DMX physical acceptance, Mac
real-device, signing, publication, and product-wide acceptance remain
unclaimed.

No assertion was weakened and no runtime or physical-output behavior changed.

## Current-main software revalidation

The bounded F1 checks were repeated on current `main` at source HEAD
`9389ec5e74a6457b2c93beb197933953b37ced22`. No input API, generation model,
or physical-output path was changed.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:project-transaction` | PASS — project transaction production contract and project authority checks |
| `cargo test -p syndocal --release --locked callback_epoch -- --test-threads=1` | PASS — 3 passed, 0 failed |
| `cargo test -p syndocal --release --locked installed_callback_gate -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked project_transaction_fence -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked external_admission_ -- --test-threads=1` | PASS — 2 passed, 0 failed |
| `cargo test -p syndocal --release --locked project_control_retirement -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked project_retirement -- --test-threads=1` | PASS — 6 passed, 0 failed |

All native commands used the exact MSVC 14.44.35207 x64 linker pin and
`where.exe link.exe` first-match check. This is current-main software evidence
for the audited generation/admission seams only; physical input clients,
reconnect/latency, device behavior, and venue operation remain unclaimed.

## Current-main software revalidation — 2026-09-10

The bounded F1 checks were rerun against current `main` source HEAD
`0589258c1ea3145fc914483d3d80a1e6ed2b1cff`; no input API or generation model
was changed.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:project-transaction` | PASS — project transaction production contract and project authority checks |
| `cargo test -p syndocal --release --locked callback_epoch -- --test-threads=1` | PASS — 3 passed, 0 failed |
| `cargo test -p syndocal --release --locked installed_callback_gate -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked project_transaction_fence -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked external_admission_ -- --test-threads=1` | PASS — 2 passed, 0 failed |
| `cargo test -p syndocal --release --locked project_control_retirement -- --test-threads=1` | PASS — 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked project_retirement -- --test-threads=1` | PASS — 6 passed, 0 failed |

All Cargo commands used the exact Build Tools MSVC `14.44.35207` x64 linker
pin after `vcvars64.bat -vcvars_ver=14.44`, with that linker first in
`where.exe link.exe`. This remains software-only evidence; physical MIDI,
OSC, DMX, reconnect/latency, device, venue, Mac, signing, and publication
acceptance remain open.

## Current-main UI catalogue generation fence — 2026-09-11

The frontend control-input controller now assigns independent monotonic
request generations to overlapping MIDI input and MIDI output catalogue
refreshes. A late success cannot replace a newer catalogue or selection, and a
late failure cannot replace the current operator message. The existing
backend callback/project generations, mapping authority, connection routes,
and physical-I/O behavior are unchanged. This is a stale-UI-result repair, not
a new MIDI/OSC/DMX API or a physical-device acceptance.

The focused checker was extended with source-shape assertions and deferred
success/failure/retirement regressions for both catalogues:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches; MIDI input/output latest-generation fixtures passed |
| `pnpm.cmd --dir app exec tsc --noEmit` | PASS |
| `pnpm.cmd --dir app run check:dvc-midi-shortcuts` | PASS — 39 assertions |
| `pnpm.cmd --dir app run check:release` | PASS — full static release gate; 50 Open + 8 Deferred preserved |
| `git diff --check` | PASS |

The exact Windows native procedure was used for the no-bundle build: MSVC
14.44.35207 was first in `where.exe link.exe`. The resulting executable was
`target/release/syndocal.exe`, version `1.2.0-alpha.69`, 64,744,448 bytes,
SHA-256
`38938F3771354F6E46B9F7F11478E56BA0975671212BDD497ABF281BFB7A02E8`.
The fresh native probe is
`target/qa/native-final-validation-20260911-60/native-final-validation.json`:
one responsive maximized `Syndocal` window, Standby with lighting/video
disabled, snapshot IPC, expected missing-asset/layer thumbnail rejection,
zero physical-output operations, exact application exit, and zero remaining
debug listener.

This checkpoint does not close the full `F1-INPUT-GENERATIONS-001` row. Real
MIDI/OSC/DMX clients, reconnect/latency, device and venue behavior, dependent
F2 ownership, ASIO/NDI/DMX physical acceptance, Mac, signing, publication,
and product-wide completion remain unclaimed. The real-file thumbnail
missing → Retry → recovery trial was not rerun.

## Current-main authority-fenced failure responses — 2026-09-11

The control-input controller now fences failure responses as well as success
responses for the existing project-authority-bearing MIDI and OSC operations.
When a project replacement changes the captured authority while Connect,
Disconnect, Learn, Start, or Stop is pending, the older error cannot reset the
new project's connection state or overwrite its operator message. The backend
commands, expected-epoch fields, physical I/O routes, and fixed three-screen
UI remain unchanged. This is a stale-result repair, not a new API or a
physical-output enablement.

The frontend checker now requires each affected catch path to test the captured
authority before its state/message write, including the recovery catch inside
targeted MIDI/OSC Learn. The normal path and stale/authority-rejection shape
are therefore covered without weakening the existing assertions:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches; stale success/failure fences passed |
| `pnpm.cmd --dir app exec tsc --noEmit` | PASS |
| `pnpm.cmd --dir app run check:dvc-midi-shortcuts` | PASS — 39 assertions |
| `pnpm.cmd --dir app run check:release` | PASS — full static release gate; 50 Open + 8 Deferred preserved |
| `git diff --check` | PASS |

The exact Windows native no-bundle procedure used MSVC `14.44.35207` as the
first `where.exe link.exe` result. The fresh executable was
`target/release/syndocal.exe`, version `1.2.0-alpha.69`, 64,744,448 bytes,
SHA-256
`7AB2261D5BB601ACCA09E5D16AC30F70CC2F6F064F1500F6758DD534529A2E7B`.
The fresh probe report is
`target/qa/native-final-validation-20260911-61/native-final-validation.json`:
one responsive maximized `Syndocal` window, Standby with lighting/video
disabled, snapshot IPC, expected missing-asset/layer thumbnail rejection,
zero physical-output operations, exact application exit, and zero remaining
debug listener. The real-file thumbnail missing → Retry → recovery trial was
not rerun.

This checkpoint does not close the full `F1-INPUT-GENERATIONS-001` row. Real
MIDI/OSC/DMX clients, reconnect/latency, device and venue behavior, dependent
F2 ownership, ASIO/NDI/DMX physical acceptance, Mac, signing, publication,
and product-wide completion remain unclaimed.

## Current-main DMX status generation fence — 2026-09-11

The existing DMX input status poll now rejects an older overlapping status
response before publication, suppresses an older polling error message, and
retires an in-flight response during App cleanup. The DMX Start and Stop
failure paths also check the captured project authority before changing status
or reporting an error. No DMX command, mapping format, worker, or physical
output behavior changed.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app exec tsc --noEmit` | PASS |
| `pnpm.cmd --dir app run check:dvc-dmx-shortcuts` | PASS — 41 assertions, including DMX status cleanup, Learn stale-result ordering, and Start/Stop stale-failure fences |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches |
| `pnpm.cmd --dir app run check:release` | PASS — full static release gate; 50 Open + 8 Deferred preserved |
| `git diff --check` | PASS |

The exact Windows native no-bundle procedure used MSVC `14.44.35207` as the
first `where.exe link.exe` result. The fresh executable was
`target/release/syndocal.exe`, version `1.2.0-alpha.69`, 64,744,448 bytes,
SHA-256
`AA770E13226E2DE2DB2BB86AD6C25C34356E14D74C4C635F381D8212F91CF648`.
The fresh probe report is
`target/qa/native-final-validation-20260911-65/native-final-validation.json`:
one responsive maximized `Syndocal` window, Standby with lighting/video
disabled, snapshot IPC, expected missing-asset/layer thumbnail rejection,
zero physical-output operations, exact application exit, and zero remaining
debug listener. The real-file thumbnail missing → Retry → recovery trial was
not rerun.

This checkpoint does not close the physical `INPUT-PHYSICAL-001` or full
`F1-INPUT-GENERATIONS-001` rows. Physical DMX/MIDI/OSC clients, reconnect and
latency, device and venue behavior, dependent F2 ownership, ASIO/NDI physical
acceptance, Mac, signing, publication, and product-wide completion remain
unclaimed.
