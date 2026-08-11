# DVC import: authored semantics and implementation-defect policy

Date: 2026-08-10

Update: 2026-08-11 — Burst, Sweep, Knight Rider, Perlin, Sparkle, and Random Fill now have one
production evaluator per family. The surviving implementation is the corrected analytic route,
authored in Syndocal-native normalized domains. DVC pixel/enum values are converted only by the
importer. The former `daslight_exact` and `syndocal_corrected` fields are accepted only as legacy
v1 load input and are removed by deterministic load migration; they are no longer protocol fields,
runtime dispatch inputs, or editor controls. Existing `source_*` fields remain provenance only.

Update: 2026-08-11 — COLOR MAPPINGS ID31 Lines and ID50 Grid use
`implementation=SyndocalCorrected`: the recovered fixed 100x100 paint grammar, integer
stride branch, float fallback, palette order, overlap order, placement, and qGray are
preserved, while the unavailable 40ms QImage work cache is replaced by continuous analytic
sampling at the authored duration. This is an implementation-artifact correction, not a
change to authored parameters or target identity.

## Product rule

Syndocal imports the authored show, not Daslight's accidental implementation defects. The
compatibility target is the same practical look and motion, not instruction-for-instruction,
timer-for-timer, or pixel-artifact parity. Recovered factory schemas, parameter domains,
beam/selection identity, palette order, phase, direction, symmetry, geometry, and deliberately
stepped effect shapes remain compatibility contracts. When those contracts can be represented by
a smaller continuous-time, deterministic, full-coverage common evaluator with a practically
equivalent result, that cleaner evaluator is preferred over recovered frame tables, integer
caches, toolkit raster paths, or process-global state. A defect caused by timer granularity,
integer truncation, an uninitialised or non-serialised process state, low-quality raster scaling,
or an unpainted destination pixel is corrected.

Import reports must name every intentional correction. Tiny differences caused only by a cleaner
implementation do not justify a Legacy mode. For the six unified spatial families above, the
product ruling is final: no alternate evaluator mode is retained.

## Repository-wide audit

| Existing route or boundary | Recovered behavior already represented by Syndocal | Classification | Disposition |
|---|---|---|---|
| CURVE 3/7/9/10 | Source position held on a 40 ms grid | Timer/storage artifact | **Corrected:** continuous-time evaluation retains authored equations and clamp regions |
| CURVE 9 Square | `floor(400/Rate)` leaves an uneven terminal band | Integer-grid artifact | **Corrected:** equal authored bands replace the integer residue |
| CURVE 10 Strobe | `floor(25/Rate)` changes frequency and the 40 ms minimum pulse can become 100% duty at high Rate | Timer-grid artifact | Use exact authored rate and a dimensionless common duty; keep Phase duty intent, not the 40 ms floor |
| CURVE 4 Pulse | Fixed `0.005` triangle slope makes amplitude depend on DURATION | Evaluator defect | **Corrected:** normalized continuous window retains Rate, Size, Offset, Phase and beam order |
| VALUE/COLOR 622/121 Burst | 16-bit integer segment residue, 65,536-entry palette cache, 1,024-entry Qt gradient table, raster seam and 750-frame cap | Rendering/cache artifacts | Replace the current import default with an analytic cyclic evaluator that retains pixel radius, palette order and Gradient meaning. Do not keep a production Legacy mode without a demonstrated project need |
| VALUE/COLOR 624/127 Knight Rider | Qt nearest-neighbour fold can leave the last odd pixel and width-one raster clear | Unpainted-pixel defect | **Corrected:** full destination coverage now retains the recovered painted samples, head motion, fading, direction and palette semantics |
| VALUE/COLOR 625/134 Sweep | 40 ms frame-count floor shortens DURATION; Qt fold can clear the odd tail | Timer and raster defects | Destination coverage **corrected**; move imported effects to the existing continuous common evaluator. Hard boundary and Direction Change stay intact |
| VALUE/COLOR/MAPPINGS 628/128/530 Perlin | 40 ms period floor, 750-frame table, 65,536-entry integer palette cache, Qt nearest-neighbour rotation, evaluator-dead Direction | Timer, cache, resampling and dead-control defects | Preserve the recognizable noise/palette semantics; use authored duration, continuous deterministic evaluation, analytic palette sampling, hole-free inverse rotation, and a documented meaningful Direction mapping |
| VALUE/COLOR 623/129 Plasma | 8-bit wrapping sine table is part of the recovered appearance | Visually material evaluator identity | Retain for imported compatibility unless capture evidence shows a defect that can be removed without changing the effect |
| MAPPINGS/COLOR MAPPINGS placement | Rotated inclusion mask with axis-aligned raster lookup | Rotation-coordinate defect | Correct raster lookup into the same inverse-rotated local frame; retain raw Patch coordinates and inclusion geometry |
| VALUE 626/627, COLOR 131/133 | Import fails closed because Qt qrand history is not serialised | Missing provenance, not authored state | Implement a deterministic Syndocal seed and report the correction; do not keep these FX unavailable merely to mimic process history |
| CHASER 325 | Process/thread random order was replaced by a stable permutation | Beneficial correction already present | Keep deterministic behavior and reclassify it from generic compatibility to corrected import |
| MOVE 221-225 | DURATION floor, frame-rounded phasing, Line endpoint loss; Points intentionally holds authored vertices | Timer/endpoint defects plus one deliberate stepped shape | Retain geometry, wings, selection order and Points hold; use authored duration and continuous phase for the other paths, with equal authored point dwell for Points |
| qGray, hard palette boundaries, Direction Change, Transform intent | Recovered authored/post-process semantics | Artistic contract | Retain |

## Acceptance rule for every correction

1. Preserve the strict TYPE/ID/range schema and the original raw value in provenance.
2. Preserve fixture, beam, selection and palette identity/order.
3. Add a deterministic regression showing the recovered defect and the corrected result.
4. Record `implementation=SyndocalCorrected` and a concise reason in the import report.
5. When Legacy quirks remain, make the mode explicit and keep corrected behavior as the import
   and authoring default.
6. Run focused importer/engine/frontend gates, the full relevant test matrix, the native release
   build, and the exact-checkout responsive-window check before calling the tranche complete.
