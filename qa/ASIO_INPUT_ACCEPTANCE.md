# ASIO Input Acceptance

Updated: 2026-08-26

## Release boundary

Syndocal's default MIT build remains on the validated WASAPI shared input path. ASIO must be a separately built Windows feature and must not enter the normal installer until one of these distribution paths is selected and recorded:

1. GPLv3 distribution, including the corresponding-source and notice obligations for the ASIO-linked artifact.
2. A signed Steinberg proprietary ASIO SDK agreement covering the distributed artifact.

This is a release gate, not a runtime preference. `asio-sys` statically compiles and links SDK code, so omitting the SDK archive from the installer does not remove the licensing decision.

Primary references:

- Steinberg ASIO SDK open-source terms and FAQ: <https://www.steinberg.net/developers/asiosdk-open/>
- Steinberg developer and proprietary SDK routes: <https://www.steinberg.net/developers/>
- CPAL ASIO setup: <https://github.com/RustAudio/cpal/blob/v0.18.1/README.md>

This file records an engineering gate and is not legal advice.

## 2026-08-30 local-only show-ASIO boundary

The performance is **2026-08-30**; development, acceptance, and show preparation
must be complete by the separate **2026-08-29 completion deadline**. The
proposed show path is a separately licensed, non-default local build selected only by the explicit
`show-asio` feature and its dedicated Tauri overlay. It is not a variant of the
normal MIT/WASAPI artifact and it must not be reached by default features,
normal packaging, or an automatic fallback.

Checkpoint `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` built and
manifest-verified the exact same-host directory
`target/show-asio-local/Syndocal_Show_ASIO_1.2.0-alpha.12_ff61a6dec6eb_x64`.
Its application is 58,637,824 bytes, SHA-256
`1D313900AB94A2429BF784B7D4CCA8E8EC39FBF17E11CB257D76A19656AA2F8D`;
its ABI-v2 bridge is 813,568 bytes, SHA-256
`40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`;
and its manifest SHA-256 is
`DCDFA0D381C851483D9E206637E803920ADDD6CC5B0C313780604FFC1D0EAAC4`.
The manifest fixes `distributionApproved: false`, `sameHostOnly: true`, and
`unbundled: true`. The normal signed updater never governs this artifact: it
must not discover, install, update, repair, replace, select, or attest the
local-only show-ASIO build. Conversely, this build result cannot be promoted to
normal installer/updater acceptance. Its dedicated checker must pass
immediately before use, and physical native/operator acceptance remains open.

Current evidence closes only the exact HOTONE/Ampero ABI-v2 **bridge-only**
44.1 kHz / 2-channel `i32` / 128-frame continuous run of `3,600,031 ms`.
Formal matched 48 kHz ASIO/WASAPI, current native application load and operator
UI, persisted-selection revalidation, occupied/reset/resync/XRUN/unplug/
no-callback recovery, restart, and measured capture/engine/pixel latency remain
open. The complete warning matrix executed for runtime/artifact checkpoint
`ff61a6d` recorded zero first-party warnings; this does not close a physical
ASIO gate. These boundaries fail closed and do not permit WASAPI, another ASIO
driver, an older DLL, or a legacy ABI as a substitute.

Cleanup safety pushed through `c40cfd8` (including predecessor `ef7b647`) does
not authorize deletion of an ASIO build or evidence tree. Its sole current
candidate is `target/debug/incremental`, currently blocked by
`HardlinkDetected`; Apply has not run, no path was deleted, and reclaimed bytes
are 0. Every ASIO-named path remains protected until its exact owner, artifact
identity, and evidence-retention boundary are revalidated.

## Architecture gate

- Keep the default Rodio 0.21.1 / CPAL 0.16 WASAPI path unchanged.
- Build ASIO only through a non-default Windows feature using a separately named CPAL 0.18.1 dependency.
- Keep that dependency in `tools/asio-bridge` as an independent workspace and artifact. Directly adding CPAL 0.18.1 to the Rodio/CPAL 0.16 application graph makes Cargo reject their different Linux `alsa-sys` `links = "alsa"` versions even when the new dependency is optional and Windows-only.
- Select `HostId::Asio` explicitly. Never treat CPAL's default host as ASIO.
- Report build support, driver enumeration, driver open, and active stream as separate states.
- ASIO has no safe system-default fallback. Start without an explicit current-generation driver ID must fail.
- Refresh ASIO drivers only while stopped. Replace the catalog only after complete successful enumeration.
- Re-resolve rate, channels, format, and fixed buffer at Start and reject mismatches instead of substituting another driver or WASAPI.
- Report the actual opened buffer frames when the backend exposes them.
- Reset, resync, sample-rate change, xrun, driver loss, or a 250 ms callback gap must zero the reactive source and enter a terminal fault until the operator explicitly starts again.

CPAL 0.16's ASIO input path does not deliver the stream error callback needed by Syndocal's fail-closed contract. It is therefore not an accepted implementation target.

## Build gate

- Pin the official SDK version, archive filename and SHA-256 in `qa/ASIO_SDK_PIN.json`; archive acquisition remains manual.
- Require explicit `CPAL_ASIO_DIR` and `LIBCLANG_PATH`; do not rely on an implicit build-time download.
- Use an isolated target directory such as `target/asio-qa`.
- Run the supported local build check as `& .\qa\harnesses\check-asio-build.ps1` from the pinned developer shell (`vcvars64.bat -vcvars_ver=14.44`); it validates the explicit local SDK and `libclang.dll` before Cargo can execute `asio-sys`'s download fallback.
- Direct-Cargo P0 preflight (fail-closed, included in `-PreflightOnly`, which never invokes Cargo): `VCToolsInstallDir` must canonicalize (trailing separator/case only) to `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207`; that directory and its `bin\Hostx64\x64\link.exe` must exist; `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` must already be pinned to exactly that linker (the canonical spelling is repinned); and `where.exe link.exe` must succeed non-empty with the pinned linker first, even when Git's `link.exe` appears later in the resolution order. The harness fails closed instead of mutating the parent shell or launching vcvars; if PATH surgery is needed after vcvars, use delayed expansion (`cmd /v:on` with `!PATH!`), never `%PATH%`.
- Deterministic no-Cargo proof: `pwsh qa/harnesses/check-asio-build.ps1 -SelfTest` (parser gate, source contract, linker-pin matrix). Preflight or cargo-check success proves toolchain wiring only, not driver enumeration, stream, soak, latency, or license acceptance.
- Prove the regular cross-platform workflow never enables the ASIO feature.
- Package ASIO notices and the selected license path with the ASIO artifact.

Current pin:

- SDK version: `2.3.4`
- Archive: `ASIO-SDK_2.3.4_2025-10-15.zip`
- SHA-256: `D5EBF0C20DD2C5F43771FD0C1418F4B361BF52434EE670097CFA6B3A335E2ECA`

## Current implementation status

- Implemented: `tools/asio-bridge` is an independently locked and built Windows DLL with ABI/schema v2 and the one canonical filename `syndocal_asio_bridge.dll`. The normal app graph remains on Rodio 0.21.1 / CPAL 0.16; ASIO stays dynamically loaded and separately licensed rather than linking CPAL 0.18.1 into that graph.
- Implemented in the isolated v2 bridge: driver enumeration, capability query, explicit stream open, applied buffer reporting, Stop/Close, backend XRUN count, telemetry and terminal event delivery are separate typed operations. The current application source now loads the canonical ABI/schema v2 bridge and routes catalog, capability, persistent-selection, Start/Stop/Close, callback, terminal-fault, and safety-zero handling through that v2 surface. Source compilation and deterministic contract coverage do not by themselves constitute native or physical ASIO acceptance.
- Implemented: ASIO offers no system-default selection. Driver IDs are generation-scoped, and Start revalidates the explicit driver, rate, channels, native sample format, fixed buffer, and channel mix. Mismatch or disappearance fails rather than substituting another driver, the first enumerated driver, or WASAPI.
- Implemented in the isolated bridge: the realtime callback uses Start-time storage, converts the exact negotiated buffer to mono `f32`, and performs no heap allocation or lock acquisition in the normal callback. Reset, resync, rate/device loss, xrun, nonfinite samples, callback frame change, or a 250 ms callback gap becomes a terminal event that requires Stop/Close and an explicit restart.
- The current application-side v2 path includes the callback adapter, generation-checked one-shot fault latch, safety-zero publication, FFT-worker handoff, persistent-selection status, and the operator rail. The remaining acceptance boundary is execution on the current native artifact with a real selected driver: callback continuity, negotiated configuration, Start/Stop/Close, occupied/reset/resync/XRUN/unplug/no-callback recovery, restart, soak, and latency remain fail-closed and unchecked until directly demonstrated.
- Validated on 2026-08-25 after the current gain/parser and callback-fault repair: the isolated bridge's SDK-free tests pass 12/12; its ASIO-feature deterministic suite passes 14/14 with the one explicitly physical test ignored; Clippy `-D warnings`, ASIO all-target check, and the canonical release build report zero first-party/linker warnings with the exact VS 14.44 linker first. SDK provenance validates the pinned 48-file extraction and archive hash. The release DLL exposes exactly nine v2 symbols with v1/Play/Free absent and has SHA-256 `F6D6C92FB6E1EDA938E3ADBB741DEC596A28DE0EE6D5712F5CBC2880817932C9`. That isolated bridge suite covers empty/all-zero, negative, non-finite, subtly-over-one, and extreme gain rejection through both the parser and exported Start boundary. Those results are bridge-only evidence; the current application compile/no-run checkpoint is recorded below, while physical-driver and native application execution remain unverified. Any earlier ABI-v1 hardware/native evidence is historical and does not close the present v2 hardware gate. Default normal builds remain ASIO-free.
- Not approved for public distribution: `qa/ASIO_SDK_PIN.json` keeps `distribution_approved: false` as the authority. The exact local-only Show-ASIO artifact above exists for same-host performance acceptance, but the normal MIT installer/updater remains fail-closed against `syndocal_asio_bridge.dll`, the retired `syndocal-asio-bridge.dll`, every other `*asio*.dll`, and every DLL wildcard/glob. Windows libav packaging is deliberately limited to the seven exact DLLs recorded in `app/src-tauri/tauri.windows.conf.json`; FFmpeg and Spout notices remain explicit normal-package resources. No public ASIO artifact may be staged, published, or accepted until a separately reviewed GPLv3 distribution path or a signed Steinberg agreement, notices, and release workflow exist.

## Historical application integration compile checkpoint (superseded 2026-08-26)

This section records the earlier source-only checkpoint. Its dirty-tree and
release-build-required statements are historical and were superseded by the
committed `ff61a6d` standard and local-only artifact evidence above. It does not
override the still-open physical native/operator gates.

This checkpoint covers the current KDMX application source only. It is separate
from the isolated `tools/asio-bridge` 12/12 and 14/14 bridge evidence above.
It proves fixed-toolchain compilation and test-binary compilation; it does not
prove that a native executable loaded the bridge or that a real ASIO device was
opened.

Source identity at the checkpoint:

- Branch: `codex/syndocal-v1.2`
- HEAD: `1200aac44e2cd0a9c2f4b7138e76750d5d68e125`
- `origin/codex/syndocal-v1.2`: equal to HEAD
- Working tree: dirty, including the untracked `app/src-tauri/src/asio_bridge_v2.rs`
- The then-existing `target/release/syndocal.exe` was not an artifact claim for
  this historical dirty source checkpoint. The required release build and
  exact-source native launch later completed at `ff61a6d`; physical ASIO
  acceptance did not.

Both commands were run from a fresh VS2022 Community 14.44 environment with
the absolute Cargo linker pin below. `where.exe` was intentionally allowed to
show Git's incompatible `link.exe` second; Cargo was pinned to the first exact
MSVC linker and did not fall through to Git.

```text
cmd.exe /d /v:on /s /c 'call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" -vcvars_ver=14.44 >nul && set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe" && echo PIN=!CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER! && where.exe link.exe && cargo check -p syndocal --features asio'
```

Observed evidence: exit `0`; pinned linker was
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`; `where.exe link.exe` returned that path first and
`C:\Program Files\Git\usr\bin\link.exe` second; Cargo finished the `dev`
profile with first-party warning count `0`.

```text
cmd.exe /d /v:on /s /c 'call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" -vcvars_ver=14.44 >nul && set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe" && echo PIN=!CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER! && where.exe link.exe && cargo test -p syndocal --features asio --no-run'
```

Observed evidence: exit `0`; the test profile compiled the current
`syndocal` test binary with first-party warning count `0`. `--no-run` did not
execute tests, enumerate a driver, open a stream, invoke a physical callback,
or validate XRUN/unplug/restart/latency behavior.

### Application-side deterministic execution (2026-08-26; no hardware driver)

The current dirty application source then executed the deterministic
`asio_bridge_v2` module filter under the same fixed local toolchain. Cargo was
pinned to exactly
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe` through
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`; `where.exe link.exe` returned
that MSVC path first and `C:\Program Files\Git\usr\bin\link.exe` second.
Cargo therefore did not select Git's incompatible linker.

```text
cargo test -p syndocal --features asio asio_bridge_v2 -- --nocapture
```

The initial execution ran 35 deterministic tests: 31 passed, 4 failed, and 0
were ignored. The four failures were traced and corrected without weakening the
safety contracts:

- The stale-generation test sent both a stale sample and a stale event but
  expected one stale callback; it now asserts two fences while still requiring
  zero application-hook delivery.
- The XRUN test's severity matrix already latched every XRUN. Its unrelated
  second terminal assertion incorrectly expected a one-shot terminal latch to
  latch again; it now requires the second result to be false while retaining
  the terminal-event accounting assertion.
- A null sample payload latched local safety zero but also invoked the
  application terminal hook. The callback now latches local safety zero and
  returns before every application hook, so malformed FFI sample data cannot
  reach the application sink.
- The malformed Start-result cleanup path correctly called Stop then Close,
  but its fake transport had no scripted successful Stop response. The fixture
  now supplies that response and continues to assert exactly one Stop and one
  Close on the opened raw handle.

Final execution of the same filter: 35 passed, 0 failed, 0 ignored, and 1020
tests filtered out. No warning originated from `app/src-tauri/src/asio_bridge_v2.rs`.
An external, mid-edit first-party `engine` warning was emitted at
`crates/engine/src/lib.rs:18438` (`DjLinkTimelineObservation.position_ms` is
never read), so the artifact-wide warning gate remains incomplete and this
checkpoint does not claim a warning-free application artifact.

These tests use fake transports, parser/selection payloads, and callback
trampolines only. They did not enumerate or open a hardware ASIO driver,
produce a native release artifact, or verify any physical-driver, native-UI,
latency, soak, XRUN, reset, unplug, or restart row below. In particular, an
observation that TOPPING is present is not acceptance evidence and does not
close any hardware gate.

### Fixed-toolchain preflight shell checkpoint (2026-08-26)

The ASIO preflight is now self-contained under both Windows PowerShell 5.1 and
PowerShell 7. An initial Windows PowerShell run failed before Cargo because the
harness depended on the optional `Get-FileHash` command and the .NET Core-only
`Convert.ToHexString` and `Path.GetRelativePath` APIs. The harness now opens
each file with a read-only `FileStream`, hashes it with SHA-256 while write and
delete sharing remain denied, formats bytes with `BitConverter`, and derives a
relative path only after an explicit canonical child-prefix check. It adds
positive nested-path and negative sibling-escape self-tests and does not add a
compatibility fallback or weaken any archive/extraction comparison.

`qa/harnesses/check-asio-build.ps1 -SelfTest` passes, and the same tracked
harness invoked through `powershell.exe -NoProfile -PreflightOnly` in the
pinned VS 2022 14.44 environment passes with the exact SDK archive hash and 48
extracted files. `where.exe link.exe` again reports the absolute
`14.44.35207\bin\Hostx64\x64\link.exe` first and Git's `link.exe` second.
The preflight explicitly reported that Cargo was not invoked; this evidence is
toolchain/provenance proof only and does not close native or hardware rows.

### Current-source physical bridge execution (2026-08-26; native application still open)

The ignored physical-driver test was then built from the current ABI/schema v2
bridge source with the same absolute VS 2022 MSVC 14.44 linker pin. In both
runs, `where.exe link.exe` returned the required
`14.44.35207\bin\Hostx64\x64\link.exe` first and Git's `link.exe` second. The
test used the pinned local SDK/archive, `--locked --offline`, one test thread,
and only explicitly named driver/configuration inputs; it had no default-driver,
first-driver, other-ASIO-driver, or WASAPI fallback path.

`asio:TOPPING Pro USB Audio Device` at 48 kHz / 2 channels / native `i32` /
128 frames failed on cycle 1 before the stream started. The bridge returned
status 5 and `backend_error`: `failed to build exact ASIO input stream: hardware
is malfunctioning (can be returned by any ASIO function)`. The same result was
reproduced once after verifying and stopping only the three TOPPING vendor
control-panel processes (`ToppingPro.exe`, `ToppingTune.exe`, and
`ToppingUsbAudioCpl.exe`). No Windows audio service, unrelated application, or
other device was stopped, and no fallback occurred. This is a current negative
result, not a successful TOPPING checkpoint; further blind retries are not
accepted as progress.

The connected Ampero Mini then passed the exact current-source v2 bridge test at
`asio:HOTONE AUDIO USB Audio Device`, 44.1 kHz / 2 channels / native `i32` /
128 frames. All 100 Start/Stop/Close cycles completed in 15.5345877 seconds;
the applied buffer was 128 on every cycle, callbacks were 200, Stops were 100,
Closes were 100, and warnings, terminal events, XRUNs, nonfinite samples, frame
mismatches, and fallbacks were all 0. This is physical current-source bridge
evidence. It does not prove that the final release DLL was loaded by the native
application, that the operator UI can run and stop it, or that soak, fault
injection, and latency thresholds pass.

### Current-DLL Ampero continuous bridge soak (2026-08-26 JST; bridge-only)

An isolated current-source ABI/schema-v2 DLL was rebuilt into its evidence
directory under the exact local VS 2022 Community 14.44 linker pin. Its
canonical filename was `syndocal_asio_bridge.dll`, its export inventory had
exactly the nine v2 symbols, and its SHA-256 was
`1E67038D4226F0C1857AEF2A2844500B6CD78158D5D89099B2BBA0446C05C60B`.
The evidence build log records the required absolute MSVC linker first and
Git's `link.exe` second; Cargo remained pinned to the former. This isolated
DLL is not a native Syndocal application artifact.

The bridge catalog and capability request explicitly selected only
`asio:HOTONE AUDIO USB Audio Device`. That driver advertised 44.1 kHz,
two-channel `i32`, and an 8..2048-frame range; exact Start at 44.1 kHz / two
channels / `i32` applied 128 frames. A single continuous bridge-v2 run then
held that one explicit stream for 3,600,031 ms and ended through typed Stop and
Close, both status 0. It recorded 1,240,463 callbacks and 158,779,264 callback
frames; bridge XRUN/API count, terminal events, warnings, nonfinite samples,
and frame mismatches were each 0. The callback's maximum observed interarrival
gap was 16 ms; bridge callback-duration telemetry was p50 2,047 ns, p95/p99
4,095 ns, and max 479,600 ns. The raw report is
`target/qa/asio-ampero-v2-soak-20260826-014416/soak-report-attempt2.jsonl`
with SHA-256
`527F41562A38717E307B4BB89B2F353345A4188C009F96A373AC0A8E873F60E7`.
No default driver, first driver, other ASIO driver, or WASAPI path was selected
or substituted.

After that Stop/Close, one and only one explicit 48 kHz / two-channel / `i32`
/ 128-frame probe ran against the same DLL and the same HOTONE ID. The catalog
and exact capabilities listed 44.1 kHz only. Start returned typed status 4
`config_unsupported` before any stream was accepted:
`driver asio:HOTONE AUDIO USB Audio Device does not support exact 48000 Hz / 2 ch / i32 / 128 frame input`.
It did not open another driver or fall back to WASAPI. The probe report is
`target/qa/asio-ampero-v2-soak-20260826-014416/probe48-report.jsonl` with
SHA-256 `5D39A2B3EEDDB8934D904A1029252B3FA79C90F6CD8AE86D01C213AEE054BEE5`.

This closes only a show-specific, 44.1 kHz Ampero bridge-continuity evidence
row. It does **not** satisfy the formal matched 48 kHz ASIO/WASAPI gate: this
physical driver does not advertise 48 kHz, no 48 kHz ASIO stream was opened,
and no WASAPI one-hour harness or native application/operator-path run was
performed. Fault injection, capture-to-engine/pixel latency, distribution, and
native final-artifact acceptance remain open.

## Hardware evidence (2026-07-14)

| Driver | Requested / applied configuration | Result | Telemetry | Scope |
|---|---|---|---|---|
| `TOPPING Pro USB Audio Device` | 48 kHz, 2 channels, i32, 128 / 128 frames | Start and Stop passed | 7 callbacks, 896 frames, maximum capture delay 4166.7 us, XRUN 0, nonfinite 0, terminal event 0 | Short bridge/app-loader smoke on one working device |
| `asio:TOPPING Pro USB Audio Device` | Explicit persistent ID, 48 kHz, 2 channels, i32, 128 frames; 100 cycles | Bridge ignored hardware test passed in 27.6767468 s with `--locked --offline` | Actual buffer 128 in all 100 cycles; callbacks 200 and at least 2 per cycle; Stop 100; Free 100; warnings 0; terminal 0; XRUN event/API 0; nonfinite 0; frame mismatch 0; fallback 0 | Completed repeated Start/Stop/Free gate on the same working driver |
| Current-source native QA app + `TOPPING Pro USB Audio Device` | UI-selected ASIO, explicit driver, 48.0 kHz, 128 frames, Average All → mono | Start reached `ACTIVE`; Stop returned to `READY`; F11 and Esc round-trip passed | Maximized window capture 1913x1080; F11 capture exactly 1920x1080 at 0,0 with no title bar/taskbar; live rail showed OVR 0/0 and XRUN 0 | Full native operator-path smoke, not an endurance or latency-percentile pass |
| `Realtek ASIO` | Explicit Realtek driver request | Open failed with hardware input/output unavailable | No substitution or silent fallback | Negative fail-closed evidence, not a second accepted driver |
| `HOTONE AUDIO USB Audio Device` (Ampero Mini) | Explicit persistent ID, 44.1 kHz, 2 channels, native i32, 128 frames; 100 cycles | Bridge ignored hardware test passed in 12.7754482 s with `--locked --offline` | Actual buffer 128 in all 100 cycles; callbacks 200; Stop 100; Free 100; warnings 0; terminal 0; XRUN 0; nonfinite 0; fallback 0 | Second-vendor successful stream and repeated-open evidence |

The TOPPING evidence proves that one explicit hardware configuration opens, reports the actual fixed buffer, invokes the capture callback and stops, that the same exact configuration can complete 100 repeated Start/Stop/Free cycles without warning, terminal event, xrun, nonfinite sample, frame mismatch or fallback, and that the production operator path can configure, run and stop it at the primary 1920x1080 full-screen viewport. The 27.6767468-second repeated-open test and short native UI run are still not a one-hour soak or statistical performance result. Callback-duration percentiles, capture-to-engine/pixel latency, physical marker latency, cross-vendor breadth beyond the later exact HOTONE trial, and hot-plug/recovery fault injection were not measured by these runs.

## Warning and repeated-open checkpoint (2026-08-21)

The pinned local `ASIO-SDK_2.3.4_2025-10-15` tree and
`C:\Program Files\LLVM\bin\libclang.dll` passed
`qa/harnesses/check-asio-build.ps1`. The exact isolated all-targets Cargo warning
capture completed with exit 0, one successful `build-finished`, two unique
first-party artifacts (bridge custom build and `cdylib`/`rlib` target), malformed
JSON 0, warning-shaped stderr false, and first-party warnings 0.

The ignored physical test was then rerun against
`asio:TOPPING Pro USB Audio Device` at 48 kHz, two channels, native i32 and an
explicit 128-frame buffer. All 100 Start/Stop/Free cycles completed in
33.4553781 seconds with callbacks 200, actual buffer 128 in every cycle, stops
100, frees 100, warnings 0, terminal events 0, XRUNs 0 and nonfinite samples 0.
This refresh closes the warning-inventory execution blocker for the isolated
bridge; it does not change the remaining distribution-license,
sample-rate/buffer/channel matrix, soak, fault-injection or physical-latency gates
below. The later HOTONE checkpoint separately closes the second-vendor row.

## Second-vendor checkpoint (2026-08-21)

The connected HOTONE Ampero Mini exposed the explicit ASIO driver ID
`asio:HOTONE AUDIO USB Audio Device`. A deliberately incorrect 48 kHz request
failed at capability validation before Start and reported the exact advertised
configuration: 44.1 kHz, one or two input channels, native `i32`, and 8..2048
buffer frames. No default/first-driver/WASAPI fallback occurred.

The exact advertised 44.1 kHz / 2-channel / `i32` / 128-frame configuration then
passed one smoke cycle followed by 100 Start/Stop/Free cycles through the pinned
offline bridge test. The 100-cycle run completed in 12.7754482 seconds with actual
buffer 128 on every cycle, callbacks 200, Stops 100, Frees 100, warnings 0,
terminal events 0, XRUNs 0, nonfinite samples 0, and fallback 0. This closes the
second-vendor successful-stream gate. It is not the remaining sample-rate/buffer/
channel matrix, occupied/reset/unplug fault matrix, one-hour soak, or physical
input-to-pixel latency evidence.

Additional one-cycle HOTONE trials passed at 44.1 kHz / native `i32` for
one-channel 64 and 256 frames and two-channel 64 and 256 frames, each with two
callbacks, exact applied buffer, clean Stop/Free, and zero warning/terminal/XRUN/
nonfinite events. A later one-channel 128-frame attempt, after the separate
TOPPING driver hang described below, did not return and its exact QA processes
were terminated. Because driver-global state may have been contaminated, that
attempt is recorded as unresolved rather than attributed to HOTONE or counted as
a pass. The complete rate/buffer/channel matrix remains open.

A broader TOPPING matrix probe was also attempted. The driver advertised 44.1,
48, and 96 kHz (plus additional rates), one through six input channels, native
`i32`, and 8..2048 buffer frames. Exact 44.1 kHz / one-channel / 64-frame and
128-frame Starts both failed explicitly with ASIO backend `hardware is
malfunctioning`; no fallback occurred. The following 256-frame trial stopped
returning and the bounded test run was cancelled rather than treating the driver
hang as a pass. The remaining TOPPING matrix was not run after that hang. These
results keep the advertised rate/buffer/channel and fault-recovery gates open.

## Hardware acceptance

Run at least two vendor drivers on this host (for example MOTU plus DDJ-FLX10 or TOPPING):

- 44.1, 48, and 96 kHz where the driver advertises them.
- 64, 128, and 256 frame requests where the driver actually opens them.
- Single-channel and stereo-pair selection.
- Start/Stop 100 cycles without a leaked or wedged driver. Completed on the explicit TOPPING configuration above; Free was called successfully in all 100 cycles.
- One-hour 48 kHz / 128 frame ASIO soak and a matched 48 kHz / 256 frame WASAPI soak.
- Driver already occupied by a DAW.
- Control-panel buffer and sample-rate change while active.
- Reset/resync/xrun, unplug/replug, and no-callback failure.
- Confirm no silent fallback to another ASIO driver, the first enumerated driver, or WASAPI.

Record, per trial:

- requested and applied rate/channel/format/buffer;
- callback frame min/current/max and callback duration p50/p95/p99/max;
- dropped chunks/frames and backend xrun/error kind;
- capture-to-worker and capture-to-engine p50/p95/p99/max;
- input-to-visible-pixel latency with a physical loopback marker.

Acceptance thresholds are overrun 0, callback p99 below 20% of the hardware buffer duration, callback max below 50%, capture-to-engine p95 at most 40 ms, and loss-to-zero at most 250 ms. TouchDesigner latency parity is not claimed until the same interface, sample rate, buffer, content, and five-trial protocol are run in both applications.

## Gate state

- [x] Isolated non-default ASIO bridge ABI/schema v2 implemented.
- [x] Current application source integrates the loader, callback/fault publication, persistence, and operator UI against ABI/schema v2; the 2026-08-26 fixed-linker `cargo check --features asio` and `cargo test --features asio --no-run` are warning-free. This is source compile/no-run evidence only.
- [ ] Current ABI-v2 application path is verified natively with a real selected driver and the operator UI; hardware/native execution remains open.
- [x] SDK version/archive/SHA pin recorded; local build requires explicit SDK and libclang paths.
- [x] Direct-Cargo P0 closed: `-PreflightOnly` fail-closed MSVC toolset/linker-pin preflight implemented with parser/static/self-test proof. On 2026-08-25 the live preflight passed inside a fresh `vcvars64.bat -vcvars_ver=14.44` shell with `VCToolsInstallDir` exactly `14.44.35207`, the pinned Community `Hostx64\x64\link.exe` first and Git's `link.exe` second. The current application feature check and test-binary compile/no-run repeated that exact linker pin on 2026-08-26 with first-party warnings 0; this does not close any physical-driver or native-execution row.
- [ ] Current final ABI-v2 DLL completes one explicit working-driver short smoke with applied buffer and XRUN telemetry. Current-source v2 bridge code passed 100 exact Ampero cycles on 2026-08-26, but native release-DLL loading and operator-path telemetry remain unverified.
- [ ] Current final ABI-v2 DLL proves an unavailable explicit driver fails without another-driver or WASAPI fallback. The recorded negative run predates the final v2 checkpoint.
- [ ] Distribution license/artifact path selected and notices/source obligations packaged.
- [ ] Current final ABI-v2 DLL completes the second-vendor (`HOTONE AUDIO USB Audio Device`) 44.1 kHz / 2-channel / i32 / 128-frame stream trial and 100 clean Start/Stop/Close cycles. Current-source v2 bridge code passed this exact 100-cycle test on 2026-08-26; final release-DLL/native-app loading remains open.
- [x] Bridge-only HOTONE/Ampero ABI-v2 44.1 kHz / 2-channel / i32 / 128-frame continuous run completed for 3,600,031 ms. This closes only that bridge-continuity evidence row and does not check any native/operator or matched 48 kHz row.
- [x] Local-only non-default `show-asio` artifact was built and passed its source/runtime-manifest/staging/artifact checks at exact checkpoint `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae`; its manifest fixes `distributionApproved: false`, `sameHostOnly: true`, and `unbundled: true`.
- [ ] That exact local-only artifact passes its dedicated checker immediately before use and completes physical native/operator acceptance for the 2026-08-30 controlled show. The normal signed updater is explicitly out of scope and must never govern it.
- [ ] 44.1/48/96 kHz, 64/128/256 frames and channel-selection matrix completed where advertised.
- [ ] Current final ABI-v2 DLL completes 100 Start/Stop/Close cycles on the explicit TOPPING 48 kHz / 2-channel / i32 / 128-frame configuration with zero warnings, terminal events, XRUNs, nonfinite samples, frame mismatch or fallback. The current-source 2026-08-26 attempt failed explicitly on cycle 1 with the same backend hardware-malfunction result before and after the bounded vendor-control-panel isolation retry; no fallback occurred.
- [ ] Current-source ABI-v2 native VJ Desk configured and ran the explicit TOPPING 48 kHz / 128-frame path in F11 1920x1080, displayed zero overrun/XRUN, stopped to Ready, and returned from full screen with Esc. The recorded run is ABI-v1 historical evidence only.
- [ ] Occupied-driver/control-panel/reset/resync/xrun/unplug failure matrix completed.
- [ ] One-hour matched ASIO/WASAPI soak and callback/capture/engine percentile thresholds passed.
- [ ] Physical input-to-pixel latency and five matched TouchDesigner trials passed.
