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

## Takeover continuation — current-source fault-contract recheck — 2026-09-14

With the exact Build Tools MSVC `14.44.35207` x64 linker first in
`where.exe`, the SDK-independent `tools/asio-bridge` suite passed `31` tests
with zero failures and ignored tests. The ASIO v3 contract then passed `22`
assertions. No real driver was occupied, reset, unplugged, or reopened; the
native fault-injection matrix remains external and
`ASIO-FAULT-MATRIX-001` stays `Open`.

## Current HEAD fault-contract recheck — 2026-09-14

At current HEAD `6844ea37`, the exact MSVC `14.44.35207` Build Tools
linker was initialized and printed first in `where.exe link.exe`. The
SDK-free `tools/asio-bridge` suite passed 31 tests with zero failures or
ignored tests, followed by the ASIO v3 contract's 22 assertions.

The deterministic suite covered concurrent v2/v3 lease exclusion, failed-stop
fault retention, explicit drain, stale-ticket rejection, strict request
validation, invalid/nonfinite sample rejection, native-format conversion,
full-silence safety, callback frame integrity, generation fencing, and typed
Stop/Close cleanup. No real ASIO driver was occupied, reset, unplugged, or
reopened, and no native fault-injection artifact was produced.
`ASIO-FAULT-MATRIX-001` remains `Open` for the real occupied-driver,
rate/buffer-change, reset/resync, XRUN, unplug/replug, callback-gap, and
no-callback recovery matrix.

## Continuation — current-source fault-contract recheck — 2026-09-15

At current source HEAD `bf004b6d`, with the exact MSVC `14.44.35207` x64 linker
confirmed first by `where.exe link.exe`, the SDK-independent bridge suite and
ABI contract both exited `0`:

```text
cargo test --manifest-path tools/asio-bridge/Cargo.toml --locked -- --nocapture
test result: ok. 31 passed; 0 failed; 0 ignored

pnpm.cmd --dir app run check:asio-v3-contract
ASIO v3 contract tests passed: 22 assertions
```

The deterministic suite reconfirmed callback integrity, silence on rejected
data, generation fencing, lease ownership, stale-ticket rejection, and typed
Stop/Close cleanup. No real driver was occupied, reset, unplugged, or reopened;
`ASIO-FAULT-MATRIX-001` remains **Open**.

## Takeover continuation — current-source fault-contract recheck after Video repair — 2026-09-14

At current source HEAD `466da2a6`, the SDK-independent bridge suite was rerun
after initializing the exact Build Tools MSVC `14.44.35207` x64 environment.
The pinned linker was printed first by `where.exe link.exe`:

```text
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe

cargo test --manifest-path tools/asio-bridge/Cargo.toml --locked -- --nocapture
test result: ok. 31 passed; 0 failed; 0 ignored

pnpm.cmd run check:asio-v3-contract
ASIO v3 contract tests passed: 22 assertions
```

Both commands exited `0`. The deterministic suite reconfirmed callback frame
integrity, full-silence rejection, generation fencing, native-format handling,
lease ownership, stale-ticket rejection, no-fallback behavior, and typed
Stop/Close cleanup. No real ASIO driver was occupied, reset, unplugged, or
reopened, and no native fault-injection artifact was produced.
`ASIO-FAULT-MATRIX-001` remains `Open` for the real occupied-driver,
rate/buffer-change, reset/resync, XRUN, unplug/replug, callback-gap, and
no-callback recovery matrix.
