# Live Audio Fail-Closed Acceptance

Date: 2026-07-14

## Hazard

Live Bass/Mid/High values can drive both lighting attributes and video
parameters through the same Node Graph. A disconnected microphone or USB audio
interface must therefore never leave the last non-zero spectrum latched into a
show. A frozen value is treated as invalid input, not as a held creative state.

## Runtime contract

- On Windows the CPAL default host is used as WASAPI shared mode. Start accepts
  either the current system default or an explicitly catalogued input device,
  an optional sample rate and requested fixed buffer, and an Average All,
  Single Channel or Stereo Pair-to-mono channel mix.
- Device IDs are opaque and scoped to one catalogue generation. Refresh
  invalidates the previous generation; a stale or unknown ID fails Start and
  never silently changes to the system default or another same-named device.
  Duplicate device names receive distinct operator labels.
- Per-device capabilities expose the default format and supported channel,
  sample-rate, sample-format and buffer ranges. For the currently selected rate
  they also return one authoritative pre-open `resolved_config`; the frontend
  sends its channel count and sample format back to Start, where the backend
  resolves and validates the same exact stream configuration again. An
  unsupported requested rate, channel mix or fixed buffer fails before capture;
  fixed buffers above the 8,192-frame pool or the 200 ms watchdog-safe duration
  are rejected. A fixed buffer request remains labelled as requested, while
  callback current/min/max reports the frame count the callback actually
  delivered.
- The stream builder normalizes every CPAL PCM sample format currently exposed
  by the API: I8/I16/I24/I32/I64, U8/U16/U32/U64 and F32/F64.
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
- Each slot carries a CPAL timestamp-derived capture-to-worker age estimate.
  Split slots subtract their frame offset, and the worker publishes
  current/maximum estimate, callback count, callback current/min/max, queue
  current/high-water/capacity and dropped chunks/frames without claiming a raw
  timestamp or engine-application latency measurement.
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

The full-window VJ Desk exposes this state in one compact 64 px live-audio rail;
the Clip Grid does not render a duplicate rail. Stopped state shows device,
rate, requested buffer and channel mix. Live state replaces those controls with
overrun, capture-to-worker, selected I/O plus actual callback size, and queue
telemetry. Bass/Mid/High use a dedicated safety-filtered lightweight command at
approximately 30 Hz, while the full status remains approximately 1 Hz. Both
polls are single-flight, and device Refresh also coalesces concurrent requests
while invalidating obsolete capability responses. An explicit selection is
remapped to the refreshed opaque ID only when its backend/name identity is
unique in both catalogues. A missing or ambiguous identity remains visibly
stale, clears capabilities and locks Start until the operator reselects it;
Refresh never presents the system default as if it were the prior microphone.

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

The dedicated viewport harness drives the real Solid component through a
post-mount Tauri invoke mock instead of rewriting stopped DOM into a fake live
rail. Across 1920x1080, 1920x1032, 2048x1152, 1366x768 and 1280x720 in English
and Japanese it exercises Refresh, unique generation-ID remapping, ambiguous
same-name fail-closed/reselection, capabilities, exact Start request, non-zero
30 Hz meters, live telemetry, Stop and safety-clear pending. It requires one
64 px rail, three visible meters, four live telemetry badges and a fully visible
Clip Grid pad inside the actual Clip Grid scrollport, with no critical overflow.

Native Windows acceptance used a maximized current-source Tauri executable and
the system default WASAPI shared input. Start at 48 kHz mono reported callback
current/min/max 480/480/480 frames, capture-to-worker current 10.0 ms and maximum
10.2 ms at the sampled status, overrun 0 chunks/0 frames, and queue
current/high-water/capacity 0/1/4.
Stop completed and returned to the stopped rail. This proves the system-default
path on this host; it is not a one-hour soak, an ASIO result or broad hardware
compatibility evidence.

## Remaining boundary

This tranche closes explicit WASAPI shared selection, stale-value ownership,
retry safety and normal callback allocation. It does not yet acknowledge the
exact engine tick/output publication that applies an accepted clear, nor does
it expose a separate post-open OS/driver applied-configuration acknowledgement;
the rail therefore keeps requested buffer and delivered callback size distinct.
Same-device hot-plug recovery, capture-to-analysis/engine/pixel percentiles,
ASIO, post-open applied-configuration reporting, one-hour hardware soak and
matched TouchDesigner trials remain open. The later rich-audio tranche adds 16
bands, onset/live BPM, centroid/density, kick/snare, a compiled Audio Reactive
Rack and deterministic Auto VJ while retaining this fail-closed contract.
TouchDesigner parity is not claimed. The terminal CPAL error callback may
allocate while formatting its one-shot fault detail; the real-time normal data
callback does not.

## Checker repair revalidation — 2026-09-09

The current `main` source before this bounded repair was
`92bbf3680abaed0b824a4f414b5e62be315ffa01`. The product source was not
changed. `check-live-audio-input.mjs` rejected the existing App wiring because
its count pattern matched only the untyped form
`localize: (source) => ...`, while one of the three App-owned locale adapters
was explicitly typed as `source: string`. The checker now accepts the optional
parameter type annotation while retaining the `>= 3` assertion and its
original failure message.

Verification after the repair:

```text
node --check app/scripts/check-live-audio-input.mjs                         PASS
node app/scripts/check-live-audio-input.mjs                                  PASS
pnpm.cmd --dir app run check:live-audio                                      PASS
node --no-warnings --experimental-strip-types \
  app/scripts/check-live-audio-input-ipc-v1.mjs                              PASS
```

No product assertion was weakened, no device or physical input was started,
and the existing ASIO, hardware, latency, soak, and TouchDesigner boundaries
remain open.

The focused browser restore gate was also rerun after the repair against
`524afe27e466d64fbef0e85ff925891a6e0fc47e` with the configured desktop
Playwright runtime and installed Chrome:

```text
pnpm.cmd --dir app run check:live-audio-restore                              PASS
pass live-audio-restore-1920x1080 {"failedChecks":[], ...}
```

The fixture retained the exact forced-start IPC barriers and passed the
unsaved/saved ASIO and WASAPI disappearance, malformed catalogue, stale
device, explicit reselection, and stable-identity checks. This is browser
fixture evidence only; it did not start the native app, a physical input, or
any ASIO/WASAPI device.
