# Syndocal ASIO Bridge

This directory is an independent Windows `cdylib` crate. It is deliberately excluded from the
Syndocal workspace so the default MIT application and lockfile never acquire or link the ASIO SDK.
The ASIO-enabled bridge is a separate GPL-3.0-only artifact unless Syndocal records a signed
proprietary Steinberg licensing path.

## Supported build

Do not invoke an ASIO-feature Cargo build before the preflight. `asio-sys` has an upstream SDK
download fallback and a crate-local `build.rs` is not guaranteed to run before dependency build
scripts. From the repository root, set both variables to explicit local directories and use:

```powershell
$env:CPAL_ASIO_DIR = 'C:\path\to\pinned\asiosdk'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
& .\qa\harnesses\check-asio-build.ps1
```

The preflight validates the required SDK headers and `libclang.dll` before Cargo starts, then runs
the isolated manifest with `--no-default-features --features asio --locked` and
`target/asio-qa`. The SDK version and SHA-256 remain a release-workflow input; the SDK is never
vendored or downloaded by this crate.

The current compile verification input is Steinberg's official
`ASIO-SDK_2.3.4_2025-10-15.zip` from <https://www.steinberg.net/asiosdk>, SHA-256
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
emit no terminal or xrun event, report an xrun count of zero, and return successfully from Stop
and Free. The SDK and Rust dependencies must already be present because the command is offline.

## ABI v1 lifecycle

The canonical declarations are in `include/syndocal_asio_bridge.h`.

- `syndocal_asio_build_flags() & 0x1` reports whether this exact DLL was compiled with the ASIO
  backend. This check does not enumerate or load any driver.
- Driver IDs are CPAL's persistent `asio:<driver name>` IDs. Raw names, missing IDs, and default or
  first-driver selection are rejected.
- `syndocal_asio_start` re-enumerates the requested driver and revalidates the exact rate, channel
  count, native sample format, fixed buffer, and channel mix before it builds and starts a stream.
- The sample callback receives borrowed mono `f32` chunks. The pointer is valid only during that
  callback. A callback may be split into multiple chunks if a driver exceeds the requested buffer;
  `callback_frames` always reports the original hardware callback size.
- The audio callback reuses start-time storage and takes no bridge heap allocation or lock. Client
  callbacks must be bounded, non-blocking, and must never unwind or call lifecycle functions.
  Sample and event callbacks can arrive concurrently, so the client must make its context safe for
  that access.
- `stop` pauses the stream and `play` resumes only a non-terminal stream. A terminal event requires
  `free` followed by a new explicit `start`; no driver or backend fallback occurs.
- Reset, resync, sample-rate change, device change/loss, malformed callbacks, xrun, and a callback
  gap over 250 ms are terminal. Xruns are also counted by `syndocal_asio_xrun_count`. The consumer
  must zero its reactive source immediately upon a terminal event.
- Do not call `free` concurrently with any other handle function or from a bridge callback. The DLL
  must remain loaded until all handles and owned strings have been freed.

JSON payloads are UTF-8 bytes without a trailing NUL. Always release successful output or error
strings with `syndocal_asio_string_free` in the same loaded DLL.
