# Timeline Follow software audit — 2026-09-08

## Scope

This checkpoint audits the existing Timeline Follow runtime and Hold/Immediate
authoring contracts on current `main`. It adds no product behavior and does
not claim the separate native/physical or full product gate. The previous
real-file-missing → UI Retry → recovery test is not rerun.

- Main base before this checkpoint: `66c8d4c50a9e8aab4cb803639d46f102b2126740`
- Product source changes: none
- Changed file: this QA record only
- Physical audio/video output, external clients, and venue acceptance were not
  exercised

## Static contracts

| Command | Result |
| --- | --- |
| `node app/scripts/check-timeline-follow-runtime.mjs` | PASS — stale E/G rejection, E/R/H abort receipt recovery, operator/performance/cue visibility, focus, target, and localization contracts |
| `node app/scripts/check-timeline-follow-hold-ui.mjs` | PASS — Immediate/Hold and one-measure wait-for-Pedal-1 modes are explicit, normalized, and persisted |

These are source/model checks; they are not DOM or native-operator evidence.

## Rust evidence

All Cargo commands ran after `vcvars64.bat -vcvars_ver=14.44`, with the exact
BuildTools `14.44.35207` x64 linker first in `where.exe link.exe`.

| Focused filter | Result |
| --- | --- |
| `cargo test -p engine --release --locked timeline_follow -- --test-threads=1` | 27 passed, 0 failed |
| `cargo test -p syndocal --release --locked timeline_follow -- --test-threads=1` | 3 passed, 0 failed |

The engine tests cover generation-fenced abort, terminal receipt replay,
crossfade/BPM slew, audio/video/lighting quorum, source-map and target
isolation, Hold/Immediate timing, guide cadence, late failure, rollback,
project/authoring replacement, settlement acknowledgement, and exact owner
chains. The application tests cover history-free terminal abort and captured
audio-consumer acknowledgement.

## Boundary

This is a focused software audit. It does not close `TIMELINE-FOLLOW-001` or
`VIDEO-FULL-GATE-001`; native window operation, physical audio/video, browser
DOM evidence, external clients, and venue/soak gates remain separate.

## Current-main software revalidation

The Timeline Follow software vertical was revalidated on current `main` at
source HEAD `d50cf957f4ec7e85c1db7875724a51254426a9e5`. No follow schema,
transport, or UI behavior was changed.

```text
node app/scripts/check-timeline-follow-runtime.mjs
PASS
node app/scripts/check-timeline-follow-hold-ui.mjs
PASS

cargo test -p engine --release --locked timeline_follow -- --test-threads=1
test result: ok. 27 passed; 0 failed; 0 ignored

cargo test -p syndocal --release --locked timeline_follow -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored
```

The Cargo commands used the exact MSVC 14.44.35207 x64 linker pin and
`where.exe link.exe` first-match check. This is current-main software evidence
only; browser DOM, native window, physical audio/video, external client, and
venue/soak acceptance remain outside this audit and the parent ledger row stays
Open.
