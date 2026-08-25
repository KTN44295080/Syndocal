# ASIO Input Acceptance

Updated: 2026-08-25

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
- Implemented in the isolated v2 bridge: driver enumeration, capability query, explicit stream open, applied buffer reporting, Stop/Close, backend XRUN count, telemetry and terminal event delivery are separate typed operations. The current application loader is still ABI v1 and is deliberately incompatible with the v2 DLL until the app-integration tranche replaces it; this is not accepted as a working ASIO application path.
- Implemented: ASIO offers no system-default selection. Driver IDs are generation-scoped, and Start revalidates the explicit driver, rate, channels, native sample format, fixed buffer, and channel mix. Mismatch or disappearance fails rather than substituting another driver, the first enumerated driver, or WASAPI.
- Implemented in the isolated bridge: the realtime callback uses Start-time storage, converts the exact negotiated buffer to mono `f32`, and performs no heap allocation or lock acquisition in the normal callback. Reset, resync, rate/device loss, xrun, nonfinite samples, callback frame change, or a 250 ms callback gap becomes a terminal event that requires Stop/Close and an explicit restart.
- Not yet integrated for ABI v2: the application-side callback adapter, generation-checked one-shot fault latch, safety-zero publication, FFT-worker handoff, and the 64 px operator rail still use the retired ABI-v1 loader contract. Existing UI controls do not make the v2 DLL operable; the loader and UI state publication must move to v2 together before native acceptance.
- Validated on 2026-08-25 after the current gain/parser and callback-fault repair: SDK-free tests pass 12/12; the ASIO-feature deterministic suite passes 14/14 with the one explicitly physical test ignored; Clippy `-D warnings`, ASIO all-target check, and the canonical release build report zero first-party/linker warnings with the exact VS 14.44 linker first. SDK provenance validates the pinned 48-file extraction and archive hash. The release DLL exposes exactly nine v2 symbols with v1/Play/Free absent and has SHA-256 `F6D6C92FB6E1EDA938E3ADBB741DEC596A28DE0EE6D5712F5CBC2880817932C9`. The suite covers empty/all-zero, negative, non-finite, subtly-over-one, and extreme gain rejection through both the parser and exported Start boundary. Physical-driver and application evidence still predates the final v2 checkpoint and must be rerun. Previous ABI-v1 app catalogue/capability evidence is historical only; it does not validate the present v2 boundary. Default normal builds remain ASIO-free.
- Not approved for distribution: `qa/ASIO_SDK_PIN.json` keeps `distribution_approved: false` as the authority. The normal MIT installer/updater is fail-closed against `syndocal_asio_bridge.dll`, the retired `syndocal-asio-bridge.dll`, every other `*asio*.dll`, and every DLL wildcard/glob. Windows libav packaging is deliberately limited to the seven exact DLLs recorded in `app/src-tauri/tauri.windows.conf.json`; FFmpeg and Spout notices remain explicit normal-package resources. No ASIO distribution artifact is generated, staged, published, or accepted until a separately reviewed GPLv3 artifact path or a signed Steinberg agreement, notices, and release workflow exist.

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
- [ ] Application loader, callback/fault publication, persistence, and operator UI integrated against ABI/schema v2 and verified natively.
- [x] SDK version/archive/SHA pin recorded; local build requires explicit SDK and libclang paths.
- [x] Direct-Cargo P0 closed: `-PreflightOnly` fail-closed MSVC toolset/linker-pin preflight implemented with parser/static/self-test proof. On 2026-08-25 the live preflight passed inside a fresh `vcvars64.bat -vcvars_ver=14.44` shell with `VCToolsInstallDir` exactly `14.44.35207`, the pinned Community `Hostx64\x64\link.exe` first and Git's `link.exe` second; the full ASIO Cargo check remains part of the next native checkpoint.
- [ ] Current final ABI-v2 DLL completes one explicit working-driver short smoke with applied buffer and XRUN telemetry. The recorded ABI-v1/pre-checkpoint run is historical evidence only.
- [ ] Current final ABI-v2 DLL proves an unavailable explicit driver fails without another-driver or WASAPI fallback. The recorded negative run predates the final v2 checkpoint.
- [ ] Distribution license/artifact path selected and notices/source obligations packaged.
- [ ] Current final ABI-v2 DLL completes the second-vendor (`HOTONE AUDIO USB Audio Device`) 44.1 kHz / 2-channel / i32 / 128-frame stream trial and 100 clean Start/Stop/Close cycles. The recorded Start/Stop/Free result is historical.
- [ ] 44.1/48/96 kHz, 64/128/256 frames and channel-selection matrix completed where advertised.
- [ ] Current final ABI-v2 DLL completes 100 Start/Stop/Close cycles on the explicit TOPPING 48 kHz / 2-channel / i32 / 128-frame configuration with zero warnings, terminal events, XRUNs, nonfinite samples, frame mismatch or fallback. The recorded Start/Stop/Free result is historical.
- [ ] Current-source ABI-v2 native VJ Desk configured and ran the explicit TOPPING 48 kHz / 128-frame path in F11 1920x1080, displayed zero overrun/XRUN, stopped to Ready, and returned from full screen with Esc. The recorded run is ABI-v1 historical evidence only.
- [ ] Occupied-driver/control-panel/reset/resync/xrun/unplug failure matrix completed.
- [ ] One-hour matched ASIO/WASAPI soak and callback/capture/engine percentile thresholds passed.
- [ ] Physical input-to-pixel latency and five matched TouchDesigner trials passed.
