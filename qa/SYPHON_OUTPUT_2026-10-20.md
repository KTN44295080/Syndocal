# T30: Syphon output implementation and Mac acceptance handoff

Date: 2026-10-20
Worktree branch: `task30-syphon`
Base commit: `e1d72e2306620d9c64454abf426ded50c1261f4d`

## Scope and current proof

This change implements Syphon **server output** behind the `syphon` feature and
`target_os = "macos"`. It reuses the existing `SyphonServer` project value;
there is no project schema migration or new Tauri, MIDI, OSC, or DMX operation.
Syphon input remains explicitly unsupported, including when output availability
is reported as Available.

This handoff is a code and CI candidate. The Windows host can compile and test
the common transport with an injected sender, but cannot compile Objective-C
against the macOS SDK or demonstrate Syphon reception. Mac acceptance below
remains required.

## Runtime design

- `syphon_transport.rs` follows the generic Spout output worker and reuses
  `physical_output_fence.rs`, `video_output_pacer.rs`, and
  `live_video_output_health.rs`.
- Resource creation requires the current output activation and creation lease
  **before** entering the native constructor. Parent publication transfers that
  lease; the worker then waits once for the exact Arm READY activation, with the
  existing 30-second bound. No startup frame is submitted before that barrier.
- The constructor readiness reply has a five-second wait. If the constructor
  outlives that wait, its owner thread and ownership accounting remain in a
  pending-startup record until cleanup and join. A replacement cannot overlap it.
- Syphon's official initializer announces the server name immediately. Discovery
  may therefore see the name during activation; frame publication still waits
  for READY. This implementation does not claim to make discovery transactional.
- Each frame uses current output/presentation, ownership, safety blackout, mapping,
  and Timeline Follow authority. The shared final SDK fence rechecks authority
  while the frame permit is held. A revoked frame cannot increment delivery
  health or acknowledge Follow Applied.
- Rendering uses the current CPU RGBA output materialization and one reusable
  Metal upload texture. RGBA is top-row-first; the bridge uses `flipped:NO`.
  The Syphon renderer performs RGBA-to-BGRA conversion. There is no direct
  sharing of Syndocal's wgpu texture in this version.
- The common monotonic pacer targets 60 Hz without queued catch-up frames.
  Native send waits for GPU completion **and** Syphon's publication completion
  handler. Only successful sends increment `kind = "syphon"` health.
  These values measure the sender boundary, not reception or display scanout.
- Normal Standby, project retirement, and output-role changes close the native
  owner on its worker thread and join it before releasing teardown accounting.
- Explicit native-close failure retains the owner and its exact lease, reports
  Failed, and requires a process restart before rearm. The unwind guard preserves
  the same accounting if Rust panics: successful cleanup retains the fence until
  join; failed cleanup never becomes a false retirement acknowledgement.
- Rust startup/admission/Follow deadlines do not bound a hung Metal command or
  framework stop call. GPU/SDK hangs may keep retirement pending. This is a
  material native-runtime acceptance risk, not a proven real-time guarantee.

Configuration accepts a nonempty server name of at most 255 UTF-8 bytes, with no
control characters, and dimensions from 1 through 8192 pixels on each side.
The maximum tightly packed RGBA frame is 256 MiB. Actual Metal device limits,
allocation success, and throughput must be checked on the target Mac.

## UI boundary

Native backend availability is loaded once at UI mount without constructing a
sender. Only an Available Syphon backend offers the Syphon output choice. Saved
unsupported configurations remain identifiable and preserved. The health row
uses the Syphon name and the existing sender measurement explanation. New UI
strings have English and Japanese wording and use existing theme classes.

The base checkout intentionally disables **all generic external output creation
and configuration mutations** until canonical OutputControl operations exist.
T30 preserves those fences; it does not reactivate the retired `add_video_output`
or `set_video_output_config` routes. The Syphon implementation runs outputs
already configured in a project. The UI states that external creation is not
available yet. A future canonical creation/configuration task is still needed
for an end-to-end new-output authoring workflow.

## Framework source, build, and packaging

Official upstream:
https://github.com/Syphon/Syphon-Framework

Pinned revision:
`f4761677a45b8034a3c2069ec0f3d2553da81fba`

Source archive:
https://codeload.github.com/Syphon/Syphon-Framework/tar.gz/f4761677a45b8034a3c2069ec0f3d2553da81fba

Archive SHA-256:
`a3766088c46588c42b087f4e5ff7ae7bb0f83292af59a0aeb8bba69eba5311c5`

`app/scripts/prepare-syphon.sh` verifies that archive, extracts fresh source,
and builds the official Syphon Xcode scheme for arm64 and x86_64 with deployment
target macOS 12.0. It checks the framework architectures, install name, and
`Resources/default.metallib`, and writes `target/syphon/source-receipt.txt`.
No prebuilt Syphon binary is downloaded or committed.

The upstream distribution licence is BSD 3-Clause; source headers also contain
BSD 2-Clause notices. `licenses/Syphon-BSD.txt` preserves the distribution
licence and all six distinct source-header copyright/notice variants from the
pinned archive. `THIRD_PARTY_NOTICES.md` identifies the source and pin.

`build.rs` invokes the Objective-C compiler only for macOS with the feature.
The narrow C ABI contains Objective-C exceptions and enables ARC exception
cleanup. Windows and Linux have no Syphon native compiler, linker, framework,
or runtime requirement. The bridge does not add Rust dependencies.

The macOS Tauri overlay bundles the prepared framework and licence. Release
linking uses `@executable_path/../Frameworks`; debug binaries/tests also receive
the prepared framework's absolute development rpath. The build and package
paths point to the same worktree `target/syphon/frameworks` directory.

## Required Mac build run

On a Mac with full Xcode/Metal tools and the repository's usual FFmpeg/Tauri
prerequisites, run from the repository root:

```sh
bash app/scripts/prepare-syphon.sh
cargo check --workspace --all-targets --locked
cargo test -p syndocal --bin syndocal syphon --locked -- --test-threads=1
node app/scripts/check-syphon-output-ui.mjs
```

The `macos-installer.yml` workflow runs preparation before Cargo, denies Rust
warnings in its Syphon compile/test gate, and retains the build, source receipt,
test, and UI logs in its macOS evidence artifact. It then uses the existing app/
DMG packaging flow and checks the embedded framework, Metal library, licence,
dynamic dependency, and code signature. That workflow has been edited here;
it has **not** been dispatched or observed passing.

After the owner reviews and publishes the branch through the normal process,
run the macOS installer workflow and review those logs. Test both Apple Silicon
and Intel hardware when available; a universal framework alone does not prove
a complete universal application package or runtime behavior on both.

## Required real-client acceptance

Use an offline copy of a project already containing an enabled `SyphonServer`
output with a unique server name and an existing composition. External-output
creation remains fenced as described above. Start with a 1920x1080 composition
containing asymmetric corner markers, readable text, red/green/blue patches,
an alpha ramp, and a moving frame/time marker.

1. Start the packaged Mac application in Standby. Confirm there is no active
   server stream. Arm the appropriate video-capable role through normal UI
   authority controls; confirm server discovery and receive it in Resolume or
   Syphon Recorder.
2. Verify upright text/corners, channel order, alpha handling, dimensions, and
   motion in the client. Compare receiver evidence with the Syphon health row;
   do not substitute sender fps for client measurement.
3. Start without an attached client, attach one later, then disconnect/reconnect
   it. Confirm output remains valid and receiving resumes without rearming.
4. Run representative 1080p60 and 4K material for a sustained interval. Record
   Mac/GPU model, OS, client/version, composition complexity, resolution, sender
   fps/percentiles/missed intervals, receiver cadence, and visible artifacts.
5. Exercise opacity, mapping, output blackout, safety blackout, composition
   routing, and Timeline Follow. Confirm changes cannot produce a stale frame
   or a false settlement success at the native send boundary.
6. Enter Standby during playback, while a client is connecting, and during a
   slow startup. Confirm frame publication stops and discovery retires after
   native teardown; rearm repeatedly without duplicate servers.
7. Repeat project replacement, resolution/server-name changes through supported
   project loading, and application exit. Confirm old owners retire before
   successors. Capture errors or persistent pending/Failed state.
8. Retain CI artifact identifiers, application hash, server/client logs, and a
   client recording or screenshot of the moving test composition.

No receiver acceptance, physical fps claim, Mac runtime result, push, merge,
release build, or native application launch was performed on the Windows
worktree during this task.

## Review

An independent read-only review compared the new transport and bridge against
the Spout worker, shared physical fence, ownership APIs, and pinned official
Syphon source. It identified and corrected vertical orientation, silent
IOSurface-allocation success reporting, ARC exception cleanup, and Rust unwind
retirement-accounting risks. Windows injected-sender tests cover the portable
behavior; the native proof boundary remains the Mac run above.

## Windows validation completed

All Rust commands used the isolated worktree's debug target directory and the
pinned `pcargo-here.cmd` wrapper. `CARGO_TARGET_DIR` was unset. The final
workspace check and the Rust test runs below completed with **zero compiler
warnings**. Test binaries exercised unit tests; the native application was not
launched.

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | Pass, zero warnings |
| `cargo test -p io --lib -- --test-threads=1` | 234 passed, 4 existing ignored, zero failures |
| `cargo test -p engine --lib output_ownership -- --test-threads=1` | 14 passed |
| `cargo test -p syndocal --bin syndocal syphon -- --test-threads=1` | 17 passed: portable policy, injected transport, and shared fences |
| `cargo test -p syndocal --bin syndocal video_output_pacer -- --test-threads=1` | 2 passed |
| `cargo test -p syndocal --bin syndocal live_video_output_health -- --test-threads=1` | 4 passed |
| `cargo test -p syndocal --bin syndocal external_video_backend_validation_blocks_unavailable_io -- --test-threads=1` | 1 passed |
| `cargo test -p video --features syphon --lib syphon` | 1 passed |
| `cargo test -p video --features syphon --lib external_video` | 5 passed |
| `cargo test -p video --features syphon --lib video_runtime_status` | 2 passed |
| `pnpm install --frozen-lockfile --prefer-offline` in `app/` | Pass |
| `pnpm exec tsc --noEmit` in `app/` | Pass |
| Syphon UI behavior check | Pass: capability choice, saved configuration, creation fence, health, JA/EN |
| Localization check | 4137/4137 static Japanese strings covered; zero unprotected user-data labels |
| Video output routing R4 / output ownership source contracts | Both pass |
| `node app/scripts/check-ai0-source-coverage.mjs` | Pass; counts unchanged |
| macOS artifact workflow contract tests | 12 passed; these are source-level workflow tests, not a Mac build |
| `bash -n app/scripts/prepare-syphon.sh` | Syntax pass on a local copy; preparation was not executed on Windows |
| `git diff --check` and file-encoding checks | Pass; changed Rust and new files are UTF-8 without BOM with LF |

AI0 inventory remains Tauri 586, Engine 285, Remote 116, MIDI/OSC/DMX 229,
Frontend 506, Keyboard 33, UI literal invokes 347, and six fail-closed authority
gates. No action, route, or event count required repinning.

The final logs are retained in `target/task30-validation/` in this worktree.
Use `workspace-verified`, `native-syphon-verified`, `pacer-verified`,
`health-verified`, and `backend-verified` for the final Rust runs, plus
the `*-final`, `io`, `engine-ownership`, and `video-*` logs
for the other checks. An intermediate formatting pass briefly duplicated a
module declaration; that was corrected before the final successful check and
regression runs. Git's `core.autocrlf` notices are separate from compiler
warnings.

The Objective-C/Metal bridge, embedded framework, signing, packaged application,
actual Syphon client reception, and Apple Silicon/Intel behavior remain
unverified until the Mac build and acceptance sequence above is completed.
