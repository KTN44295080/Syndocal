# DVC import: authored semantics and implementation-defect policy

Date: 2026-08-10

## Product rule

Syndocal imports the authored show, not Daslight's accidental implementation defects.
Recovered factory schemas, parameter domains, beam/selection identity, palette order, phase,
direction, symmetry, geometry, and deliberately stepped effect shapes remain compatibility
contracts. A defect caused by timer granularity, integer truncation, an uninitialised or
non-serialised process state, low-quality raster scaling, or an unpainted destination pixel is
corrected when that correction does not replace the authored effect.

If a defect has a large visible contribution to an existing show, the corrected evaluator is the
import default and the recovered behavior may remain as an explicit **Legacy quirks** option. It
must not be labelled simply `Daslight exact`, because exact provenance does not make a defect a
desirable product contract. Import reports must name every intentional correction.

## Repository-wide audit

| Existing route or boundary | Recovered behavior already represented by Syndocal | Classification | Disposition |
|---|---|---|---|
| CURVE 3/7/9/10 | Source position held on a 40 ms grid | Timer/storage artifact | **Corrected:** continuous-time evaluation retains authored equations and clamp regions |
| CURVE 9 Square | `floor(400/Rate)` leaves an uneven terminal band | Integer-grid artifact | **Corrected:** equal authored bands replace the integer residue |
| CURVE 10 Strobe | `floor(25/Rate)` changes the requested frequency | Timer-grid artifact | **Corrected:** exact authored rate; recovered 40 ms minimum flash width and Phase duty extension retained |
| CURVE 4 Pulse | Fixed `0.005` triangle slope makes amplitude depend on DURATION | Evaluator defect | **Corrected:** normalized continuous window retains Rate, Size, Offset, Phase and beam order |
| VALUE/COLOR 622/121 Burst | 16-bit integer segment residue, 1024-entry RGBA64 cache, raster seam | Rendering artifacts that can visibly affect a show | Keep recovered core as Legacy quirks; corrected core removes gaps/banding without changing radial motion or palette order |
| VALUE/COLOR 624/127 Knight Rider | Qt nearest-neighbour fold can leave the last odd pixel and width-one raster clear | Unpainted-pixel defect | **Corrected:** full destination coverage now retains the recovered painted samples, head motion, fading, direction and palette semantics |
| VALUE/COLOR 625/134 Sweep | 40 ms frame-count floor shortens DURATION; Qt fold can clear the odd tail | Timer and raster defects | Destination coverage **corrected**; authored-duration correction remains pending. Hard boundary and Direction Change stay intact |
| VALUE/COLOR/MAPPINGS 628/128/530 Perlin | 40 ms period floor and Qt nearest-neighbour rotation | Timer and resampling artifacts | Preserve the recovered noise/hash semantics while using authored duration and corrected spatial resampling |
| VALUE/COLOR 623/129 Plasma | 8-bit wrapping sine table is part of the recovered appearance | Visually material evaluator identity | Retain for imported compatibility unless capture evidence shows a defect that can be removed without changing the effect |
| MAPPINGS/COLOR MAPPINGS placement | Rotated inclusion mask with axis-aligned raster lookup | Rotation-coordinate defect | Correct raster lookup into the same inverse-rotated local frame; retain raw Patch coordinates and inclusion geometry |
| VALUE 626/627, COLOR 131/133 | Import fails closed because Qt qrand history is not serialised | Missing provenance, not authored state | Implement a deterministic Syndocal seed and report the correction; do not keep these FX unavailable merely to mimic process history |
| CHASER 325 | Process/thread random order was replaced by a stable permutation | Beneficial correction already present | Keep deterministic behavior and reclassify it from generic compatibility to corrected import |
| MOVE 221-225 | 40 ms path frames are interpolated between adjacent frames; Points is intentionally held | Mostly source parameterisation/effect semantics | Retain geometry, wings, phasing and Points hold. Remove only period shortening or endpoint loss proven to be an artifact |
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
