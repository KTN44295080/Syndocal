# ASIO Input Acceptance

Updated: 2026-07-14

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
- Select `HostId::Asio` explicitly. Never treat CPAL's default host as ASIO.
- Report build support, driver enumeration, driver open, and active stream as separate states.
- ASIO has no safe system-default fallback. Start without an explicit current-generation driver ID must fail.
- Refresh ASIO drivers only while stopped. Replace the catalog only after complete successful enumeration.
- Re-resolve rate, channels, format, and fixed buffer at Start and reject mismatches instead of substituting another driver or WASAPI.
- Report the actual opened buffer frames when the backend exposes them.
- Reset, resync, sample-rate change, xrun, driver loss, or a 250 ms callback gap must zero the reactive source and enter a terminal fault until the operator explicitly starts again.

CPAL 0.16's ASIO input path does not deliver the stream error callback needed by Syndocal's fail-closed contract. It is therefore not an accepted implementation target.

## Build gate

- Pin the official SDK version and SHA-256 in a separate manual Windows workflow.
- Require explicit `CPAL_ASIO_DIR` and `LIBCLANG_PATH`; do not rely on an implicit build-time download.
- Use an isolated target directory such as `target/asio-qa`.
- Prove the regular cross-platform workflow never enables the ASIO feature.
- Package ASIO notices and the selected license path with the ASIO artifact.

## Hardware acceptance

Run at least two vendor drivers on this host (for example MOTU plus DDJ-FLX10 or TOPPING):

- 44.1, 48, and 96 kHz where the driver advertises them.
- 64, 128, and 256 frame requests where the driver actually opens them.
- Single-channel and stereo-pair selection.
- Start/Stop 100 cycles without a leaked or wedged driver.
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
