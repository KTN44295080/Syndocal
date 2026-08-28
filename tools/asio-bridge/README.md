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

## ABI v2 compatibility and ABI v3 contract

The bridge crate is versioned independently from the normal Syndocal product:
this crate is `3.0.0`, preserves the exact ABI v2 nine-symbol input surface,
and declares supported ABI range 2 through 3 in its package metadata.  This is
not a normal product version bump and does not put an ASIO artifact in the MIT/
WASAPI build or installer.

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

### ABI v3 output/full-duplex boundary

ABI v3 is a second exact nine-symbol surface under `syndocal_asio_v3_`.  It
does not change, alias, or add fields to an ABI v2 request.  Its strict Start
JSON fixes output-only versus full-duplex mode, separately validated input and
output native tuples, fixed buffer width, first output frame, and session/render
generations.  PROGRAM/CUE physical channel selection is machine-local
application state and is absent from both v3 JSON and portable project data.
Full-duplex input and output share the single ASIO device clock and callback:
their `sampleRateHz` and `fixedBufferFrames` values must be exactly equal or
Start is rejected before driver open. A rejected impossible tuple never yields
an `actualInput` response.
The Start request's session and render generations are immutable ASIO-session
root fences echoed on every callback; only `firstOutputFrame` advances by the
exact fixed block width. Transport or bus generation changes do not restart the
ASIO device. The application callback context must instead hold its own
lock-free current-generation fence and reject stale queued blocks before
returning `ACCEPTED`.

The public header freezes callback return values, terminal event codes,
bridge-owned native-format conversion, output slot non-aliasing, exactly-once
bridge-string release, and context lifetime. The ASIO-feature build now uses a
purpose-built C++ SDK shim for v3 rather than CPAL/asio-sys' process callback
registry. It opens the explicitly named driver, creates one device-width
output-only or full-duplex buffer set and clock, and publishes one allocation-
free callback dispatch. Stop disables client rendering, atomically unpublishes
the session, drains all in-flight callback readers, then disposes buffers and
exits the driver before caller context may be freed.

ABI v2 and v3 share one process-wide lease. Enumeration and capability probes
run only while the lease is Stopped. Cross-version or concurrent Start returns
a visible busy error and never stops or substitutes the current owner. A
terminal owner remains faulted until its explicit Stop and Close complete the
drain. There is no automatic restart or fallback to CPAL, WASAPI, a default
device, another driver, or another stream.

The C++ shim and SDK link are compiled only with the non-default `asio` feature.
`CPAL_ASIO_DIR` and `LIBCLANG_PATH` must already name the reviewed local SDK and
toolchain. This bridge build script and shim never download or vendor the SDK;
the supported preflight must set the explicit paths before Cargo can schedule
the upstream v2 dependency build script, which still contains its own fallback.
Direct ASIO-feature Cargo invocation without that preflight is unsupported. The
resulting bridge remains GPL-3.0-only/local-show-only under the repository's
separate licensing and distribution gate. Software tests cover the exact tuple,
lease, callback, conversion, silence, and teardown contracts, but they do not
claim a physical ASIO device, latency, disconnect, XRUN, or endurance result.
