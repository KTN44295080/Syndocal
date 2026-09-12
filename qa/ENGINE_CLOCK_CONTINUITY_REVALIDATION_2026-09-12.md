# Engine clock continuity revalidation — 2026-09-12

## Scope

This checkpoint closes the reviewed BpmClock continuity defect in the
current Windows engine source. The change is intentionally limited to the
engine clock state and its deterministic Clip Take boundary regression.

The previous implementation re-anchored anchor at every MIDI Clock tempo
estimate and derived the beat counter only from elapsed time after that
anchor. It therefore reset the absolute beat position during a 24 PPQN
stream. External phase-only synchronization had the same risk: it rebuilt the
anchor from a fractional phase and discarded the integer beat position.

## Implemented behavior

- BpmClock now stores an absolute floating-point beat position at its
  anchor. MIDI Clock tempo estimation updates the BPM while retaining that
  position.
- External phase synchronization applies the shortest signed phase delta to
  the current absolute position, so repeated phase wraps do not reset the
  beat counter. Invalid phase still normalizes to zero.
- Explicit SetBpm and tap operations retain their existing manual-reset
  semantics. set_bpm_preserving_beat_position now preserves the same
  absolute position through tempo changes.
- The hot path remains fixed-capacity and allocation-free; no polling thread,
  detached worker, or frame copy was added.
- Existing RuntimeClipClockTracker generation/hold behavior remains the
  boundary for queued global-beat Clip Takes. A source/generation
  discontinuity holds an already-admitted target instead of retiming or
  auto-launching it.

## Reproduction and evidence

The new regression was first run against the pre-fix implementation and
failed at the two-beat assertion after 96 deterministic 24 PPQN pulses. After
the fix:

| Check | Result |
| --- | --- |
| cargo test -p engine --locked bpm_clock -- --nocapture --test-threads=1 | 5 passed, 0 failed |
| cargo test -p engine --locked video_clip_slot_quantization_uses_global_beats_and_holds_discontinuity -- --nocapture --test-threads=1 | 1 passed, 0 failed |
| cargo test -p engine --locked -- --test-threads=1 | 1070 passed, 14 ignored, 0 failed; doc-tests 0 passed |

The runs used the repository's Windows procedure with
vcvars64.bat -vcvars_ver=14.44 and the pinned
MSVC 14.44.35207 x64 linker:

C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe

The focused regression covers repeated MIDI pulse phase progression,
external phase re-sync without integer counter loss, and a wrapped phase
update. The existing Clip Take regression covers immutable global-beat
targets, generation mismatch, discontinuity Hold, requeue, NextBeat, and
NextBar launch behavior.

## Boundary and next work

This is current-source Windows engine software evidence only. It is not a
Tauri/native-window build, physical MIDI/DMX/OSC test, wired-LAN test,
two-process/two-machine test, output-device acceptance, venue rehearsal,
signing, or release acceptance.

The remaining ShowClock implementation gates are the estimator with bounded
slew, Hold/STALE state semantics, deterministic delay/duplicate/reorder
simulation, then the LAN adapter, action scheduling, generation coupling,
frontend/UI integration, output ownership, and two-process/two-machine
fault/soak evidence.
