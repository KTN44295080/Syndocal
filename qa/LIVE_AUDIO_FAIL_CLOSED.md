# Live Audio Fail-Closed Acceptance

Date: 2026-07-14

## Hazard

Live Bass/Mid/High values can drive both lighting attributes and video
parameters through the same Node Graph. A disconnected microphone or USB audio
interface must therefore never leave the last non-zero spectrum latched into a
show. A frozen value is treated as invalid input, not as a held creative state.

## Runtime contract

- `running` means the capture runtime still owns a stream and remains stoppable.
  It is not a health signal.
- `stale` means the input value is invalid. Bass/Mid/High meters are zero and
  the engine receives `SetLiveAudioSpectrum(None)`.
- A CPAL stream error latches the input stale. Already queued audio cannot
  publish a later non-zero spectrum over that safety clear.
- If no callback chunk arrives for 250 ms, the worker clears its sample and
  smoothing state, marks the input stale and requests the safety clear once.
  Fresh chunks may re-arm a timeout-stale stream, but never a fault-latched
  stream.
- Receiver disconnect and failure to publish a live spectrum use the same
  fail-closed path. If the engine command queue is temporarily full, the worker
  retains a pending clear and retries without blocking the audio callback.
  `safety_clear_pending` remains true until the engine accepts `None`; the UI
  treats acceptance as queue ordering, not as output-application
  acknowledgement.
- Stop latches publication off before requesting the clear. If the queue is
  full, the stream runtime and retry worker remain owned and Start stays
  blocked. Only after the clear is accepted does Stop destroy the stream, zero
  the meters and return both `running` and `stale` to false.
- Start rejects an active or stopping runtime. An old generation therefore
  cannot enqueue a late clear after a replacement stream has begun.

## Operator contract

The VJ Desk keeps the Stop action available while stale or while backend status
is temporarily unavailable. It does not present a second Start action over a
stream it still owns. A pending engine clear is red and says
`SAFETY CLEAR PENDING`; queue acceptance changes to the deliberately narrower
amber `SAFETY CLEAR ACCEPTED` state and the Node Graph summary
`Live stale · clear accepted`. The UI does not claim output zero without an
engine-application acknowledgement. The operator then uses Stop followed by
Start to reconnect a faulted device.

## Verification

```powershell
cargo test -p syndocal live_audio_input_tests
cargo test -p engine live_audio_node_graph_uses_ephemeral_spectrum_without_timeline_audio
pnpm --dir app run check:live-audio
pnpm --dir app run build
pnpm --dir app run check:localization
```

Focused Rust tests use injected `Instant` values rather than wall-clock sleeps.
They cover the 249/250 ms watchdog boundary, pending-clear retry, fresh-data
re-arm, fault latching, callback suppression after Stop, bounded shutdown
fallback and meter/status reset. The frontend contract test also fixes
command-versus-poll ordering and stopped-pending presentation. The engine test
proves that replacing a live spectrum with `None` drives the same mapped DMX
output back to zero.

## Remaining boundary

This tranche closes stale-value ownership and retry safety, but does not yet
acknowledge the exact engine tick/output publication that applies the accepted
clear. The current callback still
allocates/downmixes into a bounded queue, uses the Windows default CPAL host
(WASAPI), exposes only Bass/Mid/High, and has no explicit buffer/channel
configuration, device hot-plug recovery, capture latency telemetry, onset/BPM
tracking or deterministic Auto VJ policy. Allocation-free capture, explicit
WASAPI configuration, feature-gated ASIO, richer analysis, Audio Reactive Rack
and Auto VJ remain separate measured tranches.
