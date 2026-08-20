# Syndocal × rekordbox × Stream Deck Pedal acceptance

Date: 2026-08-20
Status: Required, not yet implemented or hardware-accepted
Source authority: user-supplied integration specification received 2026-08-20

## 1. Product outcome

Syndocal is the sole input and show-control authority for the DJ-to-band transition.
An Elgato Stream Deck Pedal gesture enters Syndocal first. One typed Syndocal trigger
then drives both the existing Timeline/Loop runtime and configured MIDI output to
rekordbox. rekordbox is not the master clock or state authority for this workflow.
The required synchronization boundary is one input causing coordinated state
transitions; exact beat-clock synchronization is not claimed.

Do not create a separate bridge application. On Windows, rekordbox is reached through
a user-selected virtual MIDI port such as loopMIDI. No rekordbox Note, CC, channel,
device name, or toggle assumption may be hard-coded.

```text
Stream Deck Pedal / UI / Keyboard / OSC / MIDI / API / Remote
                              |
                    typed Syndocal trigger
                       /              \
           existing Timeline action   configured MIDI action
                                      |
                               virtual MIDI port
                                      |
                                  rekordbox
```

## 2. Canonical trigger contract

Physical inputs and product actions remain decoupled. The minimum typed trigger set is:

| Trigger | Syndocal action | rekordbox MIDI action |
| --- | --- | --- |
| `DJ_LOOP_HALF` | Halve the active/armed musical A-B loop through the existing canonical Loop `1/2` action | User-mapped rekordbox `LOOP 1/2` message |
| `DJ_FILTER_CLOSE` | No Timeline transport, loop, cue, lighting, or video mutation | Start one configured MIDI CC ramp toward the learned low-pass value |
| `DJ_RELEASE` | Disable/release the active/armed loop without seeking, then continue normal Timeline progression | Send the configured deterministic stop action and optional reset sequence |

The same trigger must be invocable by UI, configurable keyboard shortcut, MIDI, OSC,
API/control plane, Stream Deck, and Remote only through the shared typed
Trigger/Action/receipt path. Adapters contain no independent domain logic. Safety,
authority, generation, rate-limit, reply-loss, and audit rules remain those of the
canonical control plane; adding a pedal path must not bypass them.

## 3. Pedal mapping

The default proposed Stream Deck keyboard gestures are configurable examples, not
hard-coded product behavior:

| Pedal | Proposed gesture | Trigger |
| --- | --- | --- |
| Pedal 1 | `Ctrl+Alt+F1` | `DJ_RELEASE` |
| Pedal 2 | `Ctrl+Alt+F2` | `DJ_LOOP_HALF` |
| Pedal 3 | `Ctrl+Alt+F3` | `DJ_FILTER_CLOSE` |

Syndocal must receive the gestures through a reviewed global-hotkey or equivalent
native input path. Bindings are editable, conflict-checked, persisted, keyboard-layout
aware, do not shadow text input or safety controls, and remain operable when the main
window does not have focus. Device/input loss must be visible and fail closed.

## 4. `DJ_LOOP_HALF`

One trigger invokes the existing authored/runtime musical A-B Loop `1/2` command and
the configured rekordbox Loop Half MIDI action from the same accepted event. Repeated
accepted triggers may produce `4 -> 2 -> 1 -> 1/2 -> 1/4` beat lengths, subject to the
existing minimum engine tick, grid quantization, and project/timeline boundary rules.

The active loop start remains anchored. Halving must not create a second loop system,
seek unexpectedly, double-fire boundary events, or derive Syndocal state by reading
rekordbox. If either action cannot be admitted, the receipt and operator-visible result
must truthfully distinguish full success, rejection before either action, and any
bounded partial external-send failure. Retry with the same request identity may not
halve twice or emit an untracked duplicate MIDI message.

## 5. `DJ_FILTER_CLOSE`

One trigger starts a non-blocking MIDI Control Change ramp. Defaults and editable
parameters are:

- duration: approximately `2000 ms` by default;
- channel: `1..16` in the UI and canonical `0..15` on the MIDI wire;
- CC number, start value, and end value: `0..127`;
- output device: selected from current MIDI output enumeration;
- update cadence: bounded and smooth enough for the configured duration;
- direction and endpoint: learned/configured for the actual rekordbox mapping.

The ramp runs outside the UI and Timeline realtime threads. It must not pause, release,
seek, or otherwise mutate the Syndocal Timeline. A second `DJ_FILTER_CLOSE` while the
same ramp is active is ignored by default and returns a truthful idempotent status;
future restart/replace behavior requires an explicit setting and tests. Cancellation,
device disconnect, send failure, shutdown, project replacement, stale callback, and
reply loss are bounded, generation-fenced, logged, and cannot crash Syndocal.

## 6. `DJ_RELEASE` and reset sequence

One trigger releases/disables the current Syndocal loop without seeking so ordinary
Lighting, Video, Audio, and other Show Control events continue on the Timeline. From
the same accepted event, Syndocal sends a configured rekordbox stop action. The
operator chooses the learned deterministic action only after native rekordbox testing;
`PLAY/PAUSE` must not be assumed safe because an unobserved toggle can restart playback.

An optional persisted reset sequence is supported and enabled/disabled as one mapping
setting. Its actions and delays are editable and bounded. The initial required shape is:

```text
0 ms    rekordbox Stop + Syndocal Loop Release
100 ms  configured rekordbox Loop Off
200 ms  configured rekordbox Filter Center
```

The reset sequencer uses the same selected MIDI output and typed message configuration.
It does not block Timeline progress. Repeated trigger, reply loss, device loss, project
replacement, and shutdown cannot duplicate an already acknowledged step silently.

## 7. Generic MIDI OUT and persistence

Syndocal must enumerate MIDI output devices and support at least Note On, Note Off,
and Control Change. Each action stores output-device identity, channel, message type,
Note/CC number, and value. Device identity must survive ordinary reorder where an exact
stable identity is available; a missing, stale, or ambiguous device locks the mapping
instead of silently selecting another output.

Trigger bindings, MIDI mappings, ramp parameters, stop choice, and reset sequence are
project/profile data with additive backward-compatible defaults. Save, Save As,
template, backup/recovery, Undo/Redo where applicable, project replacement, and future
schema rejection follow the generic project-authority path. Product SemVer does not
substitute for a mapping/project schema migration.

## 8. Operator surface

Setup must provide one discoverable Pedal/DJ transition configuration surface within
the existing I/O and mapping shell. It includes input gesture, conflict/error state,
MIDI output selection and refresh, per-trigger mapping, CC ramp values/duration,
retrigger policy, stop choice, reset enablement/steps, and a non-energizing validation
or test path. Do not shrink shared controls to fit; use existing disclosure and internal
scrolling contracts.

The surface exposes Idle, Looping, Filtering, and Released truth only if those states
are derived from existing runtime/trigger state. Do not create a parallel state machine
when the existing Loop and Action runtimes already carry the authority.

## 9. Required proof

Automated proof must cover:

1. one physical-input event produces one canonical trigger and one terminal receipt;
2. Loop Half changes Syndocal once and emits the configured MIDI action once;
3. repeated Loop Half reaches each permitted musical length without boundary doubles;
4. filter ramp endpoints, monotonic direction, cadence bounds, duration, and 0/127 edges;
5. ramp re-entry is ignored by default and never blocks UI/Timeline processing;
6. Release disables the loop without seeking and later Timeline cues continue;
7. reset steps execute once in order at their configured delays;
8. same-ID retry is idempotent and same-ID/different-shape is rejected;
9. wrong owner/generation, rate limit, stale callback, and project replacement fail closed;
10. missing/disconnected/renumbered MIDI output cannot crash or silently reroute;
11. legacy project load supplies safe defaults and save/reload preserves exact mappings;
12. UI, keyboard, MIDI, OSC, API/control plane, and Remote adapters converge on the same action.

Native/hardware acceptance must record exact app commit/artifact, Windows version,
Stream Deck/Pedal model and firmware, Stream Deck software version, virtual MIDI
driver/version/port, rekordbox version/deck, learned MIDI mapping, audio device,
operator/date, and raw timestamp/video/log evidence. It must demonstrate all three
pedals, repeated Loop Half, a timed filter ramp, deterministic stop, optional reset,
unfocused/main-window focus changes, device disconnect/reconnect, and next-show reuse.

## 10. Completion gate

This requirement is complete only when all of the following are true on one reviewed
native build:

- Stream Deck Pedal input is received by Syndocal through configurable bindings;
- Pedal 2 halves both the Syndocal loop and the learned rekordbox loop on each press;
- Pedal 3 ramps the learned rekordbox filter without changing Timeline state;
- Pedal 1 stops rekordbox deterministically, releases the Syndocal loop, and normal
  Timeline cues continue;
- optional Loop Off and Filter Center reset leaves rekordbox ready for the next use;
- no MIDI device or a disconnected device leaves Syndocal running with truthful error;
- every binding/value/device is editable and persisted, with no rekordbox constants;
- independent review, automated/native/hardware evidence, warning-zero supported
  configurations, and the product's ordinary release gates pass.

Until physical Stream Deck Pedal, virtual MIDI, and rekordbox evidence exists, the
feature remains `Required / Hardware pending` and must not be advertised as complete.
