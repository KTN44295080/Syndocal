# Syndocal ASIO Bridge

This directory is an independent Windows `cdylib` crate. It is deliberately excluded from the
Syndocal workspace so the default MIT application and lockfile never acquire or link the ASIO SDK.
The ASIO-enabled bridge is a separate GPL-3.0-only artifact unless Syndocal records a signed
proprietary Steinberg licensing path.

## Supported build

Do not invoke an ASIO-feature Cargo build before the preflight. `asio-sys` has an upstream SDK
download fallback and a crate-local `build.rs` is not guaranteed to run before dependency build
scripts. From the repository root, set the SDK ZIP, exact extracted root, and libclang directory:

```powershell
$env:CPAL_ASIO_DIR = 'C:\path\to\pinned\asiosdk'
$env:SYNDOCAL_ASIO_SDK_ARCHIVE_PATH = 'C:\path\to\ASIO-SDK_2.3.4_2025-10-15.zip'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
& .\qa\harnesses\check-asio-build.ps1
```

The preflight checks the ZIP filename and SHA-256, rejects unsafe or duplicate archive paths, and
hash-compares every extracted SDK file with that exact ZIP before Cargo starts. It then runs
SDK-free tests, ASIO all-target check/tests, a release DLL build, first-party/link warning
inventory, and exact export inspection in `target/asio-qa`. The SDK is never vendored or
downloaded by this crate.

The current compile verification input is Steinberg's official
`ASIO-SDK_2.3.4_2025-10-15.zip` from <https://www.steinberg.net/developers/asiosdk-open/>, SHA-256
`D5EBF0C20DD2C5F43771FD0C1418F4B361BF52434EE670097CFA6B3A335E2ECA`. A release workflow must
verify this exact digest before extraction and must not silently follow a newer redirect.

For SDK-free ABI/unit checks only:

```powershell
cargo test --manifest-path .\tools\asio-bridge\Cargo.toml --no-default-features --locked
```

For the ignored physical-driver Start/Stop stability test, set every input explicitly. The test
does not select a default or first driver and fails if the exact ID or configuration is absent.
The callback only reads the borrowed samples and updates atomic counters. This example is the
accepted TOPPING 100-cycle configuration:

```powershell
$env:CPAL_ASIO_DIR = (Resolve-Path '.\target\asio-sdk-2.3.4\ASIOSDK').Path
$env:SYNDOCAL_ASIO_SDK_ARCHIVE_PATH = (Resolve-Path '.\target\ASIO-SDK_2.3.4_2025-10-15.zip').Path
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
$env:SYNDOCAL_ASIO_TEST_DRIVER_ID = 'asio:TOPPING Pro USB Audio Device'
$env:SYNDOCAL_ASIO_TEST_SAMPLE_RATE_HZ = '48000'
$env:SYNDOCAL_ASIO_TEST_INPUT_CHANNELS = '2'
$env:SYNDOCAL_ASIO_TEST_SAMPLE_FORMAT = 'i32'
$env:SYNDOCAL_ASIO_TEST_BUFFER_FRAMES = '128'
$env:SYNDOCAL_ASIO_TEST_CYCLES = '100'
cargo test --manifest-path .\tools\asio-bridge\Cargo.toml `
  --no-default-features --features asio --locked --offline `
  --target-dir .\target\asio-hardware-qa `
  explicit_asio_start_stop_cycles_are_exact_and_clean -- `
  --ignored --nocapture --test-threads=1
```

Each cycle must apply the exact buffer, deliver at least two correctly sized finite callbacks,
emit no terminal or xrun event, report an xrun count of zero, and return typed success from Stop
and Close. The SDK and Rust dependencies must already be present because the command is offline.

## ABI v2 lifecycle

The canonical declarations are in `include/syndocal_asio_bridge.h`.

- `syndocal_asio_bridge.dll` is the only supported output identity. Hyphenated DLL aliases and all
  ABI v1 symbols are intentionally unsupported.
- `syndocal_asio_v2_build_flags() & 0x1` reports whether this exact DLL was compiled with the ASIO
  backend. This check does not enumerate or load any driver.
- Driver IDs are CPAL's persistent `asio:<driver name>` IDs. Raw names, missing IDs, and default or
  first-driver selection are rejected.
- Capabilities and Start accept only strict UTF-8 JSON with `schemaVersion: 2`; unknown fields,
  malformed UTF-8, trailing legacy fields, and other schema versions fail closed. Start
  re-enumerates the requested driver and revalidates its exact rate, channel count, native sample
  format, fixed buffer, and channel mix before it builds and starts a stream.
- Every channel-mix gain must be finite and within `0.0..=1.0`, and at least one gain must be
  non-zero. Values outside that bounded contract are rejected before any driver is opened.
- Driver/catalog/capability, Start, Stop, Close, telemetry, and error outputs are versioned JSON
  with an explicit `kind`. Output strings must be initialized empty and must never alias.
- The sample callback receives one borrowed mono `f32` slice for the exact negotiated hardware
  buffer. The pointer is valid only during that callback, and `callback_frames` reports that exact
  hardware frame count. A later callback with a different frame count is terminal; ABI v2 never
  splits, pads, truncates, or silently accepts a driver-side buffer-size change.
- The audio callback reuses start-time storage and takes no bridge heap allocation or lock. Client
  callbacks must be bounded, non-blocking, and must never unwind or call lifecycle functions.
  Sample and event callbacks can arrive concurrently, so the client must make its context safe for
  that access.
- Stop ends delivery for a handle. There is no Play/resume export: restart always requires Close
  followed by a new explicit Start. This is a clean break, not a compatibility shim.
- Reset, resync, sample-rate change, device change/loss, malformed callbacks, xrun, and a callback
  gap of 250 ms are terminal. `syndocal_asio_v2_telemetry_json` reports XRUNs plus callback-duration
  and capture-delay p50/p95/p99/max counters. The consumer must zero its reactive source
  immediately upon a terminal event.
- Close consumes and nulls the handle pointer, attempts the final Stop when needed, and returns a
  typed failure if teardown fails. ABI v2 deliberately has no void Free export that can hide that
  failure. Do not call Close concurrently or from a callback; the DLL must remain loaded until all
  handles and owned strings have been closed/released.

JSON payloads are UTF-8 bytes without a trailing NUL. Always release successful output or error
strings with `syndocal_asio_v2_string_free` in the same loaded DLL.
