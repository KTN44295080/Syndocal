# DMX input timeout revalidation — 2026-09-12

## Scope

This checkpoint covers the current-source Art-Net input timeout state machine in
`crates/io/src/dmx_input.rs`. It is a Windows loopback software test only. It
does not claim physical DMX, an external Art-Net node, a native Syndocal window,
LAN behavior, two-machine behavior, venue acceptance, signing, publication, or
product completion.

## Finding and bounded fix

Before the fix, malformed packets and valid packets for another universe used
`continue` before the common signal-loss check. A valid Universe 3 frame could
therefore remain present indefinitely when unrelated Universe 4 traffic arrived
faster than the socket timeout.

The receive loop now classifies malformed packets and wrong-universe packets
without bypassing the shared timeout section. Only a valid frame for the
configured universe refreshes `last_packet_at`, marks the signal present,
increments the received count, updates learning, and emits a frame. The change
does not add a polling thread, frame copy, or hot-path allocation beyond the
existing frame construction.

## Reproduction and result

The new loopback regression uses timeout 120 ms and:

1. sends one valid Universe 3 frame with channel 1 = 200;
2. continues sending valid Universe 4 packets and malformed 8-byte packets;
3. requires a single Universe 3 `SignalLost` event while that traffic continues;
4. verifies the signal status is cleared and malformed traffic is counted;
5. verifies no duplicate timeout event is emitted before recovery; and
6. sends a valid Universe 3 frame with channel 1 = 201 and verifies recovery.

The test was first run against the unfixed code and failed at the intended
assertion: `valid universe must time out while unrelated or malformed packets
continue` (0 passed, 1 failed). After the bounded receive-loop change it passed.

## Verification

The exact pinned Windows toolchain procedure was used: MSVC 14.44.35207 x64,
with the checkout's linker variable pointing first to
`VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\link.exe`.

- Focused regression: `cargo test -p io --locked artnet_input_signal_loss_is_not_starved_by_unrelated_or_malformed_traffic -- --nocapture --test-threads=1` — 1 passed, 0 failed.
- DMX input module tests: `cargo test -p io --locked dmx_input::tests -- --nocapture --test-threads=1` — 9 passed, 0 failed.
- Full IO crate: `cargo test -p io --locked -- --test-threads=1` — 181 passed, 0 failed, 3 ignored; doc-tests 0 passed, 0 failed.

The existing engine-side DMX merge/signal-clear regression is included in the
previously passing full engine suite. This checkpoint adds no physical-device
evidence; physical DMX and the remaining frontend MIDI/OSC lifecycle work stay
open.
