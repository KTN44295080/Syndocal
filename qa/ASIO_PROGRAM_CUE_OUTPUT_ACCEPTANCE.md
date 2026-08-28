# ASIO PROGRAM / CUE Output Acceptance

Updated: 2026-08-28

## Show boundary

This is a release-blocking gate for the DSF2026 show path. The software
deadline is 2026-08-29 and the performance is 2026-08-30. The implementation
must remain generic for supported multichannel ASIO devices; MOTU M4 is the
DSF2026 acceptance device, not a hard-coded product dependency.

The normal MIT/WASAPI build and installer remain separate from the locally
licensed, non-default show-ASIO artifact described by
`qa/ASIO_INPUT_ACCEPTANCE.md`. No ASIO SDK-linked DLL or feature may enter the
normal installer, updater, repair path, or default build graph.

## Architecture decision

The existing ASIO bridge ABI/schema v2 is an input and Reactive Capture
contract. Its request contains input-channel selection and its callback sends
captured mono samples to the application. Playback output must not be hidden in
that request or callback.

Adopt a versioned output-capable boundary rather than adding optional output
fields to ABI v2:

- Keep ABI v2 input entry points and schema behavior unchanged.
- Add ABI/schema v3 output/full-duplex entry points to the same canonical DLL,
  while preserving the exact nine v2 input exports and their signatures. The
  v3 block is the exact corresponding nine-operation surface
  (`abi_version`, `build_flags`, `drivers_json`, `capabilities_json`,
  `string_free`, `start`, `stop`, `close`, and `telemetry_json`) under the
  `syndocal_asio_v3_` prefix. Header, binary-export, loader, and packaging tests
  reject any missing, extra, retired, or unknown export.
- Freeze the v3 header/schema before implementation. Its Start contract must
  define output-only versus full-duplex callback types, native-format conversion
  ownership, device channel count, fixed callback frame count, first output
  frame, session/render generation, queue-full/underflow result, context
  lifetime, telemetry/fault codes, and the guarantee that no callback can occur
  after successful Stop/Close. An operation-name-only export set is not an ABI.
- Open one explicit ASIO driver as one multichannel output stream. PROGRAM and
  CUE must share that stream and clock domain; two independent output devices or
  streams are not an accepted implementation.
- Do not build v3 output on the current CPAL/`asio-sys` callback path. Its ASIO
  callback implementation takes synchronization locks and can resize an
  interleaved buffer after a buffer-size change, while the underlying dispatcher
  also takes a global callback mutex. V3 therefore requires a purpose-built or
  reviewed forked SDK callback backend with one `ASIOCreateBuffers` set and a
  lock-free static session dispatch. This is a hard realtime gate, not an
  optimization item.
- If Reactive Capture input and playback output are both active on the same
  ASIO driver, v3 must own one full-duplex driver session. ASIO playback never
  leaves an input-v2 session active in parallel. An already-active v2 session
  makes v3 Start fail with a visible busy fault; there is no implicit stop or
  migration of the live session.
- V2 and v3 load the canonical bridge DLL once and share one process-wide ASIO
  host/session lease with explicit
  `Stopped | Starting(v2|v3) | Active(v2|v3) | Stopping | Fault` states. The
  existing v2 callback-generation fence alone is not a lease. Concurrent
  Starts and every cross-version Start while another version owns the driver
  reject as Busy without stopping or migrating the owner.
- The shared lease also fences driver enumeration and capability discovery,
  because those operations may instantiate or interrogate the real ASIO
  driver. Live enumeration and capability probing are allowed only in
  `Stopped`. While v2 or v3 owns Starting, Active, Stopping, or Fault, the
  bridge either returns the immutable catalog captured for that owner or
  rejects the request as Busy; it never touches the driver concurrently.
  Deterministic tests cover v2/v3 enumerate, capabilities, Start, Stop, and
  Close races against every owning state.
- Fault recovery is an explicit drained transition:
  `Fault --operator Stop/Close; dispatch unpublished and callback readers drained--> Stopped`.
  Only then may a fresh driver enumeration and capability revalidation occur,
  followed by an explicit Start. If Stop or Close cannot complete, ownership
  remains Fault/Locked, live driver probing stays forbidden, and no fallback or
  automatic restart occurs.
- The realtime callback may only consume prevalidated, preallocated audio state
  and publish lock-free/bounded telemetry. Filesystem access, decoding, device
  enumeration, UI work, blocking locks, allocation, and fallback selection are
  forbidden in the callback.

The v3 request describes the exact output stream width/rate/native format/fixed
buffer and whether the stream is output-only or full-duplex. Full-duplex input
fields are mandatory only in duplex mode. Input and output native formats and
capability tuples are validated separately; v3 must not assume that they are
identical. The exact paired full-duplex tuple and actual opened values are
revalidated while the Start lease is held. PROGRAM/CUE physical mapping remains
application-owned machine state and is deliberately absent from the bridge wire
schema.

Rodio mixers are drained by a non-realtime render worker into a preallocated,
bounded interleaved-frame SPSC. Rodio's mixer source is not pulled directly in
the ASIO callback because its internal source management uses synchronization
and dynamic collections. The ASIO callback only copies the next complete block
whose session generation, transport authority, first output frame, and frame
count are exact, or emits one complete silent block and latches a terminal
fault. Partial, stale-generation, non-contiguous, and mixed-authority frames are
forbidden. Start prefills complete blocks before the hardware callback can run;
seek, pause, loop, speed, stop, and project/transport revision changes rotate the
render generation before old queued audio can become audible.

The existing Timeline Cue Audio runtime for generated Click/Guide events is an
input to the same logical CUE mixer. While show-ASIO output is active it must not
use the existing `FollowProgram` route and must not open a second explicit OS
output device. Normal MIT/WASAPI behavior remains unchanged when show-ASIO is
not selected. This is one routing coordinator, not a parallel playback path.

Every Timeline and preview source constructor is admitted through one
application `AudioOutputRouter`. While v3 is Starting, Active, or Fault, direct
Rodio `OutputStream`/default-device/explicit-device construction is unreachable
for both PROGRAM and CUE. A test or preview action cannot bypass this owner.

The implementation is split at explicit boundaries rather than growing the
existing oversized v2 and application files:

- `tools/asio-bridge/src/v3_abi.rs`: strict v3 wire/FFI types and validation;
- `tools/asio-bridge/src/v3_rt_backend.rs`: SDK session, callback, and shared
  v2/v3 lease;
- `app/src-tauri/src/asio_bridge_v3.rs`: exact dynamic loader and lifecycle;
- `app/src-tauri/src/asio_program_cue.rs`: render blocks, mapper, and telemetry;
- `app/src-tauri/src/audio_output_router.rs`: exclusive normal/show-ASIO source
  admission and backend transition fence.

The show-ASIO header, build wrapper, export checker, local artifact manifest,
and local-only documentation move together from an exact v2-nine contract to an
exact v2-nine plus v3-nine contract. The ordinary MIT packaging checker remains
v2/v3-ASIO-artifact-free.

Backend change is a fenced state transition:

```text
Normal -> Quiescing -> ASIO Starting -> ASIO Active
                         |                 |
                         +----> Fault <----+
```

Entering `Quiescing` retires and joins every PROGRAM sink, the legacy
`FollowProgram` CUE attachment, every `ExplicitDevice` CUE stream, and every
in-flight output-prepare worker before v3 may Start. The transition must prove
that no normal/default/explicit Rodio stream or late prepare result remains
publishable. ASIO Start failure or Fault stays silent: it must not recreate
`FollowProgram`, reopen a default/explicit normal device, or resume prior
samples. Returning to the normal backend requires an explicit operator
selection and a fresh validated Start. Tests must cover each former output
route, a late prepare completion, transition, Start failure, Fault, Stop, and
retry without a second stream or one CUE sample becoming audible on PROGRAM.

## Logical buses and persistence

Project data stores logical routing only:

- `PROGRAM`: stereo audience/broadcast material. This is the default for every
  existing Timeline Audio Clip whose saved data has no bus field.
- `CUE`: mono click/guide/performer cue material.

Physical channel indices must never be written to a Timeline Item, Audio Clip,
Media Asset, or portable show project. A one-way default/migration maps absent
logical-bus data to PROGRAM. Invalid and future schema values fail closed.
This applies to root, nested child, directly triggered child, follow transport,
import, save/reload, duplicate, paste, trim, split, and lane-move projections:
only a missing legacy field becomes PROGRAM, while an explicit CUE value is
preserved end to end.

Machine-local audio configuration stores:

- backend and exact ASIO driver identity;
- sample rate and fixed/requested buffer settings;
- PROGRAM L, PROGRAM R, CUE, and optional Spare physical output channels;
- the capability/catalog generation used to validate the selection.

This output profile has its own strict versioned machine-only schema. Unknown
fields, duplicate fields, malformed values, corrupt bytes, oversized input,
and future schema versions lock the output configuration without rewriting the
file. A failed restore never falls back to an OS default device, the legacy CUE
route, or another backend. Recovery requires an explicit operator selection,
successful capability revalidation, and Start.

The UI displays physical channels as one-based Output 1, Output 2, and so on.
The internal zero-based boundary must have explicit off-by-one tests. Stale,
missing, duplicated, ambiguous, or out-of-range mappings lock output until the
operator reselects and explicitly starts it.

## Mixing and channel mapping

For each output frame the logical render is conceptually:

```text
PROGRAM = [program_left, program_right]
CUE     = average(cue_left, cue_right) or the unchanged mono cue sample
```

The validated machine mapping places those values into the selected physical
channels of the device-sized interleaved frame. Every other channel, including
Spare during ordinary playback, is zero. The implementation is not fixed to
four channels and must not assume contiguous or ascending selections. A fixture
such as PROGRAM L=Output 5, PROGRAM R=Output 1, CUE=Output 7 must prove exact
channel placement, zero in every unselected device channel, and no CUE leak.

Stereo CUE uses an average/gain-safe mono sum, never raw addition. PROGRAM and
CUE may play simultaneously and must retain the existing Timeline seek, pause,
resume, stop, loop, speed, fade, and synchronization semantics.
Changing a playing clip's logical bus in either direction is itself a render
generation barrier. Save/edit, import, paste, duplicate, split/trim, lane move,
and undo/restore equivalents may publish the new bus only after old queued
blocks are retired; not one frame may remain audible on the old bus after the
authoritative change.
The current Timeline clip path does not yet publish or apply an authoritative
audio speed. V3 may claim speed preservation only after that source of truth and
its render barrier are implemented and tested; otherwise non-default speed must
reject visibly instead of being silently ignored.

## DSF2026 machine profile

The machine-local starting profile is:

- driver: `MOTU M Series ASIO` after exact live enumeration;
- sample rate: 48,000 Hz;
- PROGRAM L: Output 1;
- PROGRAM R: Output 2;
- CUE: Output 3;
- Spare: Output 4.

These values are an operator-selectable profile, not project data and not a
device-name special case. A driver that cannot provide the exact rate, channel
count, format, or buffer is rejected; no 44.1 kHz, other device, default device,
or WASAPI substitution is permitted.

## Fail-closed runtime contract

The following conditions stop or reject the ASIO output and surface a visible,
actionable fault:

- explicit driver is absent or its identity/capabilities changed;
- exact sample rate, native format, channel count, or buffer cannot be opened;
- PROGRAM L/R or CUE is absent, duplicated, or outside the device channel set;
- device loss, stream reset/resync, sample-rate or buffer-size change, XRUN,
  malformed callback, callback gap, or realtime scheduling failure;
- the application cannot sustain one authoritative stream/clock.

CUE must never be downmixed or rerouted to PROGRAM. PROGRAM must never be
rerouted to CUE. A fault must not switch to WASAPI, another ASIO device, an OS
default, or another sample rate. Reconnection does not resume playback; the
operator must revalidate and explicitly Start.

No panic or foreign exception may cross the v3 callback FFI boundary. A caught
callback failure writes one complete silent device-width block, latches a
terminal Fault through the non-realtime notifier, and returns without unwind.
Stop and Close first unpublish static dispatch, then drain every in-flight
callback reader before freeing callback context. Deterministic panic/fault and
Stop/Close race injection must prove silence, terminal state, and no
use-after-free.

## Operator UI

The machine Audio settings must provide:

- backend, driver, sample rate, buffer, PROGRAM L/R, CUE, and optional Spare
  selectors based on the current enumerated capability set;
- clear Ready, Locked/Invalid, Active, and Fault state with the exact reason;
- safe-level Test PROGRAM L, Test PROGRAM R, Test CUE, Test Spare, and preferably
  Test PROGRAM Stereo actions;
- routing preflight/solo controls for PROGRAM-only and CUE-only playback;
- optional PROGRAM L/R/CUE peak meters whose callback publication remains
  bounded and realtime-safe.

Timeline Audio Clip settings expose `Output Bus: PROGRAM | CUE`; existing clips
show PROGRAM. A compact PROGRAM/CUE marker may be added to the item without
shrinking unrelated UI controls.

Test actions start at a bounded safe level, are mutually exclusive with live
show playback, and use the same validated device/channel mapper as Timeline
audio. A test path must not bypass stale-identity, duplicate-channel, exact-rate,
or terminal-fault gates.

No date or performance-day mode is stored in the product. A day that does not
use CUE is operated by leaving CUE material unarmed or muted; it does not change
the device schema or silently reroute CUE.

## Deterministic software acceptance

- [ ] Missing bus data deserializes/migrates to PROGRAM and reserializes
      canonically.
- [ ] Invalid or future bus/schema values fail closed.
- [ ] Physical mappings exist only in machine-local state.
- [ ] One-based UI channels map exactly once to zero-based callback channels.
- [ ] Non-contiguous/reordered mapping (PROGRAM L=5, PROGRAM R=1, CUE=7) writes
      only those exact callback channels and zeroes every unselected channel.
- [ ] PROGRAM stereo reaches only mapped PROGRAM L/R.
- [ ] Mono CUE reaches only mapped CUE.
- [ ] Stereo CUE is averaged safely and reaches only mapped CUE.
- [ ] Simultaneous PROGRAM+CUE uses one frame clock and one ASIO output stream.
- [ ] The v3 callback backend contains no CPAL/`asio-sys` callback mutex or
      callback-time allocation/resize path.
- [ ] V2/v3 mutual exclusion and concurrent Starts are linearized by one shared
      lease; Busy never stops or migrates the current owner.
- [ ] V2/v3 driver enumeration and capability discovery obey the same lease;
      active-owner races never interrogate or reconfigure the live driver.
- [ ] Device-loss recovery performs no driver query before an operator
      Stop/Close fully drains Fault to Stopped; only Stopped permits fresh
      enumeration/revalidation, and playback remains stopped until explicit
      Start. Stop/Close failure remains Fault/Locked without fallback.
- [ ] Output and optional input native formats/capability tuples are validated
      separately and the exact opened tuple matches the request.
- [ ] Existing generated Click and Guide events enter the same CUE mixer; no
      second OS output stream is opened while show-ASIO is active.
- [ ] PROGRAM silence is written to the CUE channel; CUE silence is written to
      both PROGRAM channels for isolated bus tests.
- [ ] Unmapped, duplicated, stale, and out-of-range physical channels reject
      Start with a specific conflict/error.
- [ ] Exact-rate mismatch rejects Start without fallback.
- [ ] Disconnect/XRUN/reset/resync/rate-change/buffer-change/callback-gap enters
      terminal Fault and requires explicit operator Start.
- [ ] Seek, pause, resume, stop, loop, speed, fades, and nested Timeline playback
      keep PROGRAM and CUE synchronized.
- [ ] Every transport/revision barrier invalidates queued old-generation blocks;
      the callback emits no old samples after the authoritative change.
- [ ] A live PROGRAM-to-CUE or CUE-to-PROGRAM edit rotates the render generation;
      no queued sample reaches the formerly authoritative bus after the change.
- [ ] Queue empty/full, decode/render stall, insufficient prefill, Stop/Close
      races, and callback-after-close are injected deterministically: the only
      callback result is one complete silent block plus terminal Fault, with no
      partial frame, stale generation, lock/allocation, or use-after-free.
- [ ] Show-ASIO Starting/Active/Fault blocks every direct normal/default/explicit
      OS output constructor, including test and preview paths.
- [ ] Quiescing retires every PROGRAM sink, legacy `FollowProgram`, every
      `ExplicitDevice` CUE stream, and all in-flight output-prepare workers
      before v3 Start. Transition, Start failure, Fault, Stop, and retry leave
      no second stream and never reopen normal output without explicit operator
      selection.
- [ ] Callback panic/fault injection cannot unwind across FFI; it produces one
      complete silent block plus terminal Fault, and context is freed only after
      every in-flight callback reader drains.
- [ ] Project reload preserves logical bus; app restart restores then
      revalidates machine mapping without auto-start.
- [ ] Missing, corrupt, oversized, unknown-field, duplicate-field, and future
      machine-output schemas remain Locked, preserve their bytes, and never
      reopen a default device or legacy CUE route.
- [ ] A CUE-unused operating day is represented only by unarmed or muted CUE
      content; no date-specific mode or physical mapping is written to project
      data.
- [ ] ABI/header/schema tests prove v2 input behavior is unchanged and the new
      output ABI rejects unknown/future fields and revisions.
- [ ] Header, loader, bridge build, export checker, local manifest schema, and
      local-only documentation require exactly the nine v2 plus nine v3 exports;
      no gate remains pinned to a v2-only symbol set.
- [ ] Packaging tests prove the default build/installer/updater contains no ASIO
      SDK-linked artifact or enabled ASIO feature.
- [ ] Focused Rust/UI tests pass with zero first-party warnings.
- [ ] Exact Windows native gate and normal MIT
      `pnpm --dir app tauri build --no-bundle` pass, with proof that the normal
      executable/installer/updater contains no ASIO SDK-linked artifact.
- [ ] A separate dedicated show-ASIO native build passes with the exact v2-nine
      plus v3-nine bridge exports, reviewed manifest/source hashes, real app
      loader Start/Stop/Fault smoke, and exactly one responsive maximized
      Syndocal window. The normal native build is not evidence for this gate.

## Physical MOTU M4 acceptance

- [ ] Enumerate and explicitly select MOTU M Series ASIO at 48 kHz.
- [ ] Test PROGRAM L is audible only on M4 Output 1.
- [ ] Test PROGRAM R is audible only on M4 Output 2.
- [ ] Test CUE is audible only on M4 Output 3.
- [ ] Test Spare is audible only on M4 Output 4.
- [ ] PROGRAM stereo is audible only on Outputs 1/2.
- [ ] CUE mono/stereo-source test is audible only on Output 3.
- [ ] PROGRAM+CUE play simultaneously on Outputs 1/2+3 with no audible leak or
      transport divergence.
- [ ] Unplug enters Fault and silences output; reconnect does not auto-resume.

## M32/DL16 system acceptance

- [ ] M4 1/2 -> DL16 IN5/6 -> M32 Ch18/19 reaches Main L/R and Broadcast 9/10.
- [ ] M4 3 -> DL16 IN7 -> M32 Ch20 reaches only IEM buses 4/5/6.
- [ ] CUE is absent from Main, Broadcast, and Floor sends.

M32 routing is external to Syndocal implementation, but these rows are required
physical show evidence. They remain unchecked until observed on the actual
system; software or simulated tests cannot close them.

## Completion record

At each checkpoint record branch/HEAD/upstream, adopted ABI and runtime design,
changed files, schema and migration behavior, exact tests and warning counts,
native artifact identity, unverified physical rows, blockers, commit SHA, and
push result. Do not mark this gate complete while any required software, native,
MOTU M4, or M32/DL16 row remains unverified.
