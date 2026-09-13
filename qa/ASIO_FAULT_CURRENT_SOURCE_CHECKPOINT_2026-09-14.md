# ASIO Fault Matrix Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-FAULT-MATRIX-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `c4a414747d7a08c36cbe73df91251826c72707e6`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`

This checkpoint revalidates the deterministic fault and lifecycle source
contracts. It does not claim the required real-driver fault-injection matrix.

## Current-source result

The bridge source maps XRUN, reset, resync, sample-rate change, device loss,
callback gap over 250 ms, buffer-size change, queue underflow/full, and related
terminal events to fail-closed handling. The host lease remains owned until
Stop/Close is safely completed; faulted state does not admit a fallback or a
stale ticket. Invalid or partial callback data publishes complete silence.

## Verification

```text
cargo test --manifest-path tools/asio-bridge/Cargo.toml --locked -- --nocapture
pnpm.cmd --dir app run check:asio-v3-contract
```

Result: exit code 0.

- SDK-independent bridge suite: `31 passed; 0 failed; 0 ignored`.
- The suite covered callback frame/progress and silence behavior, non-finite
  and invalid-pointer rejection, native-format conversion, queue/fault lease
  handling, cross-version ownership exclusion, Stop/Close release, and
  SDK-free no-fallback behavior.
- ASIO v3 FFI contract: `22 assertions`, PASS.
- First-party warning count observed in this run: `0`.

## Takeover rerun — 2026-09-14

The SDK-independent bridge suite was rerun with the pinned MSVC
`14.44.35207` x64 linker: all 31 tests passed with zero failures or ignored
tests. The ASIO v3 contract then passed 22 assertions. No real driver was
occupied, reset, unplugged, or reopened, and no native fault-injection matrix
artifact was produced.

## Unresolved acceptance

`ASIO-FAULT-MATRIX-001` stays Open. Occupied-driver, control-panel
rate/buffer-change, reset/resync, XRUN, unplug/replug, callback-gap, and
no-callback recovery must still be exercised against real drivers/devices with
raw telemetry and explicit restart proof. SDK-independent tests cannot replace
that native external evidence.

Next action is the named-device fault matrix on the approved artifact, with
loss-to-zero timing and post-fault Stop/Close/restart records bound to the
exact driver and executable.
