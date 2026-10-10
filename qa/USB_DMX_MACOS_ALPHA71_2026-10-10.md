# USB-DMX on macOS and alpha.71 distribution

Branch `codex/mac-usb-dmx-alpha71-20261010`, base `a29394399dda3530cc694071ba8f37fa0e34feed` (alpha.70 distribution). The primary OneDrive checkout's existing dirty work is preserved. User requested a complete Mac USB-DMX repair and Windows/Mac packages; subagents and Computer Use are prohibited.

## Behavior and compatibility

Alpha.70 required Windows PnP identity in the selector, confirmation, strict IPC status validator, engine admission and verified transport open. On macOS this rejected every USB-DMX candidate even when QLC+ could output using the same hardware.

Alpha.71 captures a macOS IOKit generation and opens either the real `/dev/cu.*` callout descriptor (verified `fstat` device number and unchanged IOKit/USB ancestry before/after open), or the exact native USB FT232R registry service (`0403:6001`). Native FTDI discovery works without VCP drivers; Apple's USB user-client API provides exclusive interface ownership, finite control/bulk timeouts, UART 250000/8N2, BREAK, and transmitter-empty drain. It never seizes another app or detaches a kernel driver. A present callout path suppresses only the native USB entry with the same observed USB ancestor; serial strings alone never deduplicate devices. Unsupported FTDI families are not guessed into compatibility.

All transports retain the canonical Open DMX worker, S0/zero-write receipt and show-route coupling. No extra frame copy or frame-loop IOKit discovery was added. Native USB flush uses bounded device status reads in the existing worker; no extra polling thread. Electrical timing, actual UART drain on the SH-RS09B, unplug under physical output and fixture output still require hardware acceptance.

The selector, confirmation IPC and strict status validator require exactly one platform instance; missing, zero, malformed, mixed, ambiguous, stale and changed identities fail closed. macOS dial-in aliases are excluded to avoid presenting one physical interface twice. Storage version advances independently to 2 at the existing machine-local file path. Exact Windows version-1 selections are read without changing bytes and migrate one-way only on explicit confirmation; all new writes are v2. Corrupt/unknown/future state is rejected without rewriting. Project `.sdc` data is unchanged. Windows still uses its exact SetupAPI device-interface and opened communications-handle verification.

QLC+'s [official DMX USB documentation](https://docs.qlcplus.org/v5/plugins/dmx-usb) says its macOS path uses native USB and advises against installing VCP solely for QLC+. This prompted the native FTDI route in addition to the BSD serial route. No third-party USB library or driver is redistributed; the implementation uses system IOKit/CoreFoundation and the existing serial library. The C shim builds with warnings as errors.

## Validation and pending package acceptance

- Windows MSVC 14.44.35207 PATH-first/pinned linker preflight passed. IO, machine storage, engine admission and route tests run without physical devices. Final counts and compiler warnings will be recorded after the stable source run completes.
- Focused actual Solid component/Playwright proof passed at 1920 and 1280 widths for Windows, macOS callout, and native FTDI USB. It proves selection, exact confirmation payload, confirmation/arm availability and missing-device rejection, with no Tauri/hardware invocation. Artifacts: `target/qa/usb-dmx-alpha71/`.
- Output-control runtime and quick-setup contracts passed, including strict macOS status success and malformed/mixed/dial-in/native-generation rejection. TypeScript and Japanese localization checks passed after filling pre-existing header copy translations in this isolated checkout.
- Release static aggregate passed; final metadata check initially found the unsynchronized README product line, which was corrected. Final package build/window and complete release aggregate acceptance remain pending.
- The manual macOS installer workflow now runs actual macOS IO, engine, machine-binding and route tests before packaging and validates the app extracted from the final DMG before upload. Its new run is pending.

Distribution is an unsigned development build: Windows x64 and macOS 12+ Apple Silicon arm64. Device waveform/output, another physical PC, Gatekeeper/Developer ID/notarization and Intel Mac are not claimed. Independent review is not claimed because the user prohibits subagents; the stable diff is reviewed locally. Ledger release/hardware acceptance markers are unchanged.
