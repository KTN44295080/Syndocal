# Video C2/C4 software audit — 2026-09-08

## Scope

This checkpoint audits the existing Clip Slot/Take and mapping-to-Timeline
integration on current `main`. It adds no product behavior and does not merge
the older candidate branches. The previous real-file-missing → UI Retry →
recovery test is not rerun.

- Main base before this checkpoint: `9323ca0826b5de77d1b1650f23646e54e2529501`
- Product source changes: none
- Changed file: this QA record only
- Physical display/capture/NDI/Spout, external clients, and venue acceptance
  were not exercised

## Focused contract

| Command | Result |
| --- | --- |
| `node app/scripts/check-video-clip-slot-bank.mjs` | PASS — fixed 32-pad model, shared Edit/Control surface, stable slot identity, typed Take payload, runtime generation fencing, terminal recovery, and internal layout contracts |
| `node app/scripts/check-video-clip-slot-bank-browser.mjs` | NOT RUN — stopped before the gate because no configured Chrome or Edge executable exists on this PC |

The browser result is an environment boundary, not a product pass. No browser
fixture was started and no browser assertion was counted.

## Rust evidence

All Cargo commands ran after `vcvars64.bat -vcvars_ver=14.44`, with the exact
BuildTools `14.44.35207` x64 linker first in `where.exe link.exe`.

| Focused filter | Result |
| --- | --- |
| `cargo test -p engine --release --locked video_clip_slot -- --test-threads=1` | 13 passed, 0 failed |
| `cargo test -p engine --release --locked video_clip_take -- --test-threads=1` | 4 passed, 0 failed |
| `cargo test -p engine --release --locked video_sample_clip_take -- --test-threads=1` | 1 passed, 0 failed |
| `cargo test -p syndocal --release --locked phase1_smoke_project_sample_timeline -- --test-threads=1` | 2 passed, 0 failed |

The engine coverage includes authored/runtime separation, queue/launch/seek,
quantization, reverse/cut/crossfade, rollback, invalid-asset and project
replacement fail-closed behavior, bank limits, and atomic identity handling.
The Syndocal smoke tests cover Timeline automation position sharing and a
Timeline event reaching light/video effects.

## Boundary

This is software evidence only. It does not close `VIDEO-C2-C4-001` or
`VIDEO-FULL-GATE-001`, because the browser viewport gate is unavailable here
and native/physical output, GPU breadth, external client, and venue tests
remain separate acceptance boundaries.
