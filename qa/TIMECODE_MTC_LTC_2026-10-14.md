# MTC output and LTC input — 2026-10-14

Owner request: "MTC送信・LTC音声" (send MIDI Time Code; take SMPTE LTC from
audio). Both are machine-local runtime features: nothing is saved in the
project.

## MTC output (Setup › I/O › MIDI › "MTC output")

- One MIDI output, 24 / 25 / 29.97 drop-frame / 30 fps, following the
  Timeline playhead.
- While the Timeline plays it sends four quarter frames per frame. Each
  eight-piece sequence carries the frame at which its first piece went
  out, and starts on a frame boundary.
- Start, stop and jumps send one full-frame SysEx so receivers locate at
  once. A tempo change, tap or sync update is not a jump.
- A count-in is not playback. A playhead that stops moving for 100 ms
  holds the output, so receivers never run ahead and then jump back.
- Only a PC that owns lighting or video output sends. A Standby or unarmed
  PC is silent, and arming starts with a full frame.
- The MIDI feedback port and the MTC port refuse each other.
- The receive side was fixed at the same time. A quarter-frame sequence
  completes only from piece 0 to piece 7, and the decoded position adds the
  two frames the sequence took to send (MTC specification). Full frames are
  unchanged.

## LTC input (Setup › I/O › Sync › "LTC input")

- One channel of an audio input, in its own shared-mode capture,
  independent of Live Audio Input.
- The biphase-mark decoder:
  - removes DC offsets and follows the signal level;
  - measures the bit cell afresh at the start and after two frames without
    sync, so a tone or a wind-down never locks it;
  - accepts ±20 % speed.
- Frame rate comes from the frame length and the labels. 24 and 25 fps
  are told apart by their labels; 29.97 and 30 fps by the drop-frame flag.
- Each frame chases the Timeline (`SyncTimelineTimecode`, source LTC) at
  the frame's end plus its age in the captured block. It uses the same
  project fence as MIDI timecode.
- A frame that does not follow the previous one is taken only when the
  next frame confirms it.
- LTC start is refused while MIDI Clock / MTC input is connected.
- Reversed timecode is ignored.

## Evidence (this PC, no hardware)

- MTC (`crates/io/src/mtc_output_tests.rs`):
  - drop-frame and every-frame round trips;
  - quarter-frame and full-frame encode/decode;
  - four evenly spaced quarter frames per frame;
  - a receiver tracking within one frame;
  - stop, seek and tempo behaviour;
  - the count-in and stall hold.
- LTC (`crates/audio/src/ltc_tests.rs`, synthesized biphase audio):
  - every rate at 44.1, 48 and 96 kHz;
  - quiet, inverted, noisy, DC-offset and ±speed signals;
  - silence, tones and noise decode nothing;
  - a tone or slow timecode before normal LTC recovers;
  - off-speed 24/25 fps classified by labels;
  - a missed frame keeps the rate.
- LTC chase (`app/src-tauri/src/tests/ltc_input_tests.rs`): the chase
  position, drop-frame labels, invalid labels and the continuity gate.

## Still to check on hardware (owner)

1. **MTC:** send to a DAW or a lighting console that chases MTC. Check it
   locks at each rate, follows play, stop and seek, and drifts less than one
   frame over 10 minutes.
2. **LTC:**
   - Feed LTC from a DAW or a generator into one input channel at each rate,
     and check the Timeline follows within one frame.
   - Unplug the cable mid-show and check the status reads "no LTC signal".
   - Replug and check it relocks.
3. **Loopback:** MTC out of this PC into a second PC's MIDI Clock / MTC
   input; the two Timelines stay within two frames.
