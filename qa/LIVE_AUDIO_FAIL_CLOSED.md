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
- The normal data callback owns no heap allocation and takes no status mutex.
  It downmixes directly into four preallocated 2,048-frame slots and unparks
  the FFT worker. If the pool is exhausted, the callback replaces the oldest
  ready slot instead of waiting, preserving the newest control input while
  incrementing chunk/frame drop telemetry.
- Each slot carries a CPAL timestamp-derived capture-age estimate. Split slots
  subtract their frame offset, and the worker publishes current/maximum
  estimate, queue depth, callback count and maximum callback size without
  claiming a raw timestamp or engine-application latency measurement.
- If no callback chunk arrives for 250 ms, the worker clears its sample and
  smoothing state, marks the input stale and requests the safety clear once.
  Fresh chunks may re-arm a timeout-stale stream, but never a fault-latched
  stream.
- The Engine independently expires a live spectrum after 250 ms without a new
  `Some` command. A stopped, starved or panicked capture worker therefore
  cannot leave the last modulation value latched indefinitely. Slots already
  250 ms old are discarded before FFT and cannot be treated as recovery data.
- A worker panic is caught at the thread boundary and immediately marks the
  backend status stale. Status polling also detects a finished worker or a
  heartbeat older than 250 ms and owns pending-clear retries, so the UI cannot
  remain healthy merely because the stream object is still owned.
- Failure to publish a live spectrum uses the same fail-closed path. If the
  engine command queue is temporarily full, the worker
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
re-arm, fault latching, callback suppression after Stop, preallocated-slot
downmix/reuse, oldest-slot overflow replacement, oversized-callback conversion
limits, split-slot age correction, non-finite PCM recovery, bounded shutdown
fallback and meter/status reset. Engine tests cover explicit clear and the
independent 250 ms TTL. The reusable analyzer test proves its FFT scratch is
cleared between windows. The frontend contract test also fixes
command-versus-poll ordering and stopped-pending presentation. The engine test
proves that replacing a live spectrum with `None` drives the same mapped DMX
output back to zero.

## Remaining boundary

This tranche closes stale-value ownership, retry safety and normal callback
allocation. It does not yet acknowledge the exact engine tick/output
publication that applies an accepted clear. Capture still uses the Windows
default CPAL host (WASAPI), exposes only Bass/Mid/High, and has no explicit
buffer/sample-rate/channel-mix configuration, same-device hot-plug recovery,
capture-to-analysis/engine/pixel percentiles, onset/BPM tracking or
deterministic Auto VJ policy. Explicit WASAPI configuration, feature-gated
ASIO, richer analysis, Audio Reactive Rack and Auto VJ remain separate measured
tranches. The terminal CPAL error callback may allocate while formatting its
one-shot fault detail; the real-time normal data callback does not.
