# DVC random generators: recovered source and corrected runtime

## Scope

This tranche routes VALUE 626/627 and COLOR 131/133. The Daslight 5.0.6.2
schemas and evaluator structure are recovered exactly, but the per-thread Qt/CRT
`qrand` state and prior draw history are not serialized in `.dvc`. Syndocal therefore
preserves the authored controls and visible generator grammar while replacing only that
unavailable process history with a stable deterministic stream.

| ID | Family / class | Strict serialized schema | Corrected runtime |
|---:|---|---|---|
| 626 | VALUE / `CSparklesEffect` | `4/1, 6/3, 0/10 Number, 1/11 LifeSpan, 0/12 Width` | retained particles, continuous alpha, stable source seed |
| 627 | VALUE / `CRandomFillEffect` | `4/1, 6/3, 0/10 Point Width, 0/11 Point Height` | palette-to-palette no-replacement fill; Height retained as evaluator-dead provenance |
| 131 | COLOR / `CRandomFillEffect` | `4/1, 2/2 Grayscale, 6/3 Transform, 0/10 Point Width` | same corrected fill plus qGray and Transform fold |
| 133 | COLOR / `CSparklesEffect` | `4/1, 2/2 Grayscale, 6/3 Transform, 0/10 Number, 1/11 LifeSpan, 0/12 Width` | same corrected particles plus qGray and Transform fold |

The recovered evaluator addresses remain `CRandomFillEffect@0x140365C70` and
`CSparklesEffect@0x1403660F0`. Exact process-history replay is intentionally not claimed.

## Stable seed provenance

`dvc_corrected_rng_seed` hashes only stable source identity: scene DASUID/name, rack
ordinal/type, effect ordinal/type, and generator ID. It never uses the allocated Syndocal
`effect_id`, wall-clock time, thread identity, or import order outside that source path.
The report records `implementation=SyndocalCorrected`, the numeric seed, and
`unavailable_qrand=stable_source_seed`; this is a corrected conversion, not an
`Approximate` warning.

Native authoring uses the same deterministic evaluator with editable `rng_seed`. Additive
`serde(default)` fields keep old `.sdc` recipes valid. `syndocal_corrected=false` retains
the prior native RandomFill/Sparkle evaluator, and all new authoring/imported recipes set
it true.

## Random fill

- Point Width remains integer `1..10`; VALUE Point Height remains integer `1..10` in
  `source_point_height` but does not affect the one-row evaluator.
- The strip is partitioned with `div_ceil`, correcting Daslight's unpainted remainder.
- Each palette transition compiles one seeded permutation of all cells. Every rank occurs
  exactly once, so selection is deterministic and without replacement.
- Runtime moves continuously from current palette stop to next palette stop in rank order.
  It never clears unfilled cells to black and does not hash an unrelated color per cell.
- Transform uses the shared discrete fold; COLOR grayscale runs after the completed color.

## Sparkle (corrected state machine, 2026-08-10 redesign)

- Particles spawn in 40 ms generation epochs with exactly `Number` (`1..10`)
  simultaneous particles per epoch, seeded by `(generation_epoch, particle_index,
  rng_seed)`. The earlier reading of `Number` as a shortened spawn interval
  (`40 / Number`) was a defect and is removed.
- SINGLE VISUAL CLOCK (product ruling, 2026-08-10): both spawn epochs and particle
  lifetime run on visual (effect) milliseconds. A particle expires strictly at
  `visual_now - born_visual >= lifetime_ms`, so faster clock sync shortens wall
  lifetime while the steady population stays tempo-invariant at
  `Number * (ceil(lifetime_ms/40) + 1)` — bounded at every legal tempo, with no
  saturation wipes. Wall-clock instants are used only to detect discontinuities.
  An earlier draft used visual spawn with wall expiry; adversarial review proved that
  asymmetry made population BPM-dependent and produced periodic full-field resets at
  fast sync, and it was replaced by this ruling.
- AUTHORED PERIOD = RANDOM-LOOP LENGTH (second product ruling, same day): DVC
  generates `floor(duration/40)` frames that repeat, so the twinkle pattern repeats
  every authored period. Corrected Sparkle seeds by the wrapped epoch
  `k mod max(1, floor(period_ms/40))` while absolute epochs govern identity and
  lifetime (particles survive the wrap unchanged). Free-run cadence itself stays
  period-independent (40 ms visual epochs; rate and clock sync scale the clock) —
  matching DVC's fixed 25 fps frame playback.
- Cold start and every reset/seek deterministically synthesize the full trailing
  `ceil(lifetime_ms/40)` epochs at their exact `epoch * 40 ms` births, so a seek
  lands on a fully populated field (no ramp artifact), exactly as DVC shows.
- The evaluator resets deterministically on activation-time change, visual reversal
  (reverse/seek), a visual gap exceeding the lifetime, or an excessive epoch jump
  (separate documented threshold). Same seed plus the same time sequence reproduces
  identical output; placement/palette modulo is 64-bit for 32-bit-target parity.
- Compatible runtime state (same seed/number/width/lifetime/strip length) migrates
  across live-modifier reapplication, target/topology rebuilds, and activation
  rebuilds, so live dial moves no longer wipe the field; snapshot load remains an
  intentional hard boundary.
- The complete strip is built once per distinct visual time with bounded, reused
  allocation sized to the population bound; per-target sampling is O(1) through a
  dense single-beam join, with a map fallback for sparse/multi-beam sets. This
  removed the per-target particle rebuild behind the earlier release-gate p95
  failure (6.30 ms → ~1.6-2.8 ms measured p95, latterly with maximum-parameter
  Sparkle rows and a 254-lane Knight Rider row added to the 64x200 stack).
- The recovered decrement `(1-LifeSpan)*0.4` per 40 ms is converted once to
  `lifetime_ms = round(100 / (1-LifeSpan))`, yielding the exact source domain
  `100..1000 ms`. Raw LifeSpan is retained in `source_lifespan` for audit.
- Editor Legacy recipes store lifespan as a percentage of the authored effect period,
  which is a different domain from the DVC source LifeSpan above. Switching such a
  recipe to the corrected evaluator resolves the percentage against the same authored
  period: `lifetime_ms = clamp(round(period_ms * lifespan% / 100), 100, 1000)`.
  Missing or non-finite period/lifespan state resolves to the 100 ms floor.
- Width `1..90` is a rectangle width, not a radius. With more than one stop, particles
  cycle through stops 1..N-1 as recovered. A one-stop COLOR palette safely uses stop 0
  instead of indexing an empty secondary range.
- Transform operates on completed-strip source indices and post-composition grayscale
  (qGray) runs after the completed colour, following the shared COLOR/VALUE spatial
  path. Validation ranges and messages are unchanged.

## CHASER 325 classification

CHASER 325 already used a reload-stable permutation. Its order was not changed. The
import report now classifies that deliberate qrand replacement as
`implementation=SyndocalCorrected` instead of emitting a permanent Approximate warning.
Other CHASER compatibility boundaries (including Flash 0% minimum duty and populated
zero-pixel targets) are separate and unchanged.

## Focused acceptance

- protocol JSON: legacy byte shape plus corrected RandomFill/Sparkle round trips;
- engine: unique compiled ranks, tail-cell coverage, continuous palette transition,
  seed repeat/change behavior, continuous retained-particle fade, width/lifetime domains,
  and one-stop palette safety;
- importer: VALUE 626/627 strict schema/range/no-op behavior, stable source seed and raw
  provenance; synthetic COLOR 131/133 conversion (`6 converted / 0 skipped`);
- UI: both editors expose Corrected/Legacy, seed, transform, grayscale where applicable,
  and corrected lifetime milliseconds; localization remains 100%.

This document supersedes earlier status text that described these four IDs as permanently
fail-closed solely because process-global qrand history was unavailable.
