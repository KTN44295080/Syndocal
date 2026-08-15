DRAFT delegation prompt for the next Codex implementation tranche.
Working title: DVC-CURVE unrouted IDs (5 Ramp, 6 Random, 8 Sinus3, 11 Tangeant, 12 Triangle)
Not yet sent. Fable/supervisor should review and trim before handing to Codex.
================================================================================

## Mandatory preamble

1. `cd C:\Users\kouty\Documents\KDMX` before anything else.
2. Confirm current branch is `codex/syndocal-v1.0` (`git branch --show-current`). Do not create a
   new branch. Confirm `git status --short` before you start so you know what is already dirty
   from parallel work and do not revert it.
3. Read `qa/DVC_CURVE_SOURCE_PARITY.md` in full, especially the new
   "Real saved specimen: unrouted CURVE generators (2026-08-11, Fable native capture)" section,
   and `qa/DVC_FULL_FX_CATALOG_PARITY.md` section `## CURVE FX -- family/type 5` before writing
   any code. Also read `qa/DVC_CORRECTED_COMPATIBILITY_POLICY.md` in full -- it is the binding
   product rule for how corrections vs. authored semantics are classified, and it already states
   the unified-evaluator regime (no `daslight_exact` / `syndocal_corrected` dual route; Syndocal
   authors one corrected analytic evaluator per family in its own normalized domain, with DVC
   pixel/enum conversion only at import time).

## Scope boundary

In scope:
- `app/src-tauri/src/dvc_import.rs`: add importer arms for CURVE rack/effect/generator IDs
  `(RACK TYPE=8, EFFECT TYPE=5, ID=5|6|8|11|12)` next to the existing `(Some(8), Some(5), Some(3|4|7|9|10))`
  match block (around line 2168-2174) and the existing per-generator conversion functions
  (`dvc_curve_...` family near lines 4030-4480: Inverse Ramp/Pulse/Sinus/Square/Strobe are the
  precedent to follow for structure, `approximations` notes, `DaslightCurveSource` provenance,
  and `note` format string).
- `crates/protocol/src/lib.rs`: extend `LfoShape` (currently `Sine, Cosine, Pulse, Triangle, Saw,
  Square, Strobe, Random, Perlin` at line ~2010) only if a captured generator's recovered formula
  is not representable by an existing shape plus the existing `DaslightCurveSource` profile
  (rate/size/offset/sample_ms). Prefer reuse: Ramp (ID5) is very likely the ascending counterpart
  of the already-routed Inverse Ramp (ID3, which reuses `LfoShape::Saw` with swapped low/high --
  see `app/src-tauri/src/dvc_import.rs` lines 4040-4090) -- verify by decompiling `CRampEffect`
  and compare against the existing recovered `CRampInvEffect` equation in
  `DVC_CURVE_SOURCE_PARITY.md`. Random (ID6) is very likely `LfoShape::Random`, already present
  and evaluated via `stepped_noise` in `crates/engine/src/lib.rs` (`evaluate_lfo_shape`, line
  ~33083..33115) -- verify `CRandomEffect`'s actual step count/seed semantics before assuming
  parity. Triangle (ID12) is very likely `LfoShape::Triangle`, already present -- verify
  `CTriangleEffect`. Sinus3 (ID8) and Tangeant (ID11) are the two generators most likely to need
  genuinely new evaluator math (a tripled-frequency/harmonic sine and a tangent-based curve,
  respectively, per their Daslight UI names) -- do not guess the formula from the name; decompile
  `CSinus3Effect` (vtable/vtable/evaluator per `DVC_FULL_FX_CATALOG_PARITY.md` row `8 Sinus3`,
  `evaluator 0x140370070`) and `CTangeantEffect` (row `11 Tangeant`, `evaluator 0x140370760`)
  from the same installed binary already used for the other CURVE recoveries
  (`C:\Daslight 5\Daslight 5\Daslight 5.exe`, product version `5.0.6.2`, file version
  `25.0905.165.111`, SHA-256 recorded at the top of `DVC_CURVE_SOURCE_PARITY.md`), following the
  same static-analysis method already used for Sinus/Inverse Ramp/Pulse/Square/Strobe in that
  doc. If a genuinely new `LfoShape` variant is required, add it, wire it into
  `evaluate_lfo_shape` (native/generic dispatch) and, if the recovered formula needs the
  `DaslightCurveSource` profile's rate/size/offset like the other Curve imports, the
  `daslight_curve`-aware dispatch in `evaluate_lfo_effect_normalized_with_offset`
  (`crates/engine/src/lib.rs`, ~line 32207-32300).
- Frontend Scene Settings exposure: `app/src/types.ts` (`LfoShape` union, line ~282),
  `app/src/sceneFxDefaults.ts`, `app/src/effectVisualization.ts`,
  `app/src/components/EffectGraphicalPreview.tsx`, and `app/src/App.tsx` wherever the existing
  routed Curve shapes (Saw/Square/Strobe/Pulse/Sinus via `daslight_curve`) are exposed as
  selectable/editable in Scene Settings -- grep for `daslight_curve`/`DaslightCurveSource` in
  `app/src` for the exact call sites (5 files matched: App.tsx, EffectGraphicalPreview.tsx,
  effectVisualization.ts, sceneFxDefaults.ts, types.ts). New shapes must load into and save back
  from the Scene Settings draft the same way the existing routed Curve shapes already do
  (this repo has an explicit prior regression, `check:cue-effect-recall` /
  `check:scene-settings`, guarding exactly this round-trip -- keep it green and extend its
  coverage to the new IDs if the check enumerates shapes explicitly).
- `app/src/uiLocalization.ts` (and `app/scripts/check-localization.mjs` if it enumerates shape
  names): add JA labels for any newly user-facing generator/shape name (Ramp/Random/Sinus3/
  Tangeant/Triangle if they were not already exposed under those or equivalent existing labels).
  Zero bare user-data labels is an existing repo-wide bar (see the 2026-08-09 handoff evidence:
  "localization: 3033/3033, zero bare user-data labels").

Out of scope for this tranche:
- CURVE ID13 Custom (separate points-list schema, not captured, explicitly deferred -- keep it
  fail-closed / unrouted).
- Any other unrouted family (CHASER 323/324, MAPPINGS 522-529, COLOR MAPPINGS remaining 20, etc).
- Renaming or restructuring the already-routed CURVE 3/4/7/9/10 arms beyond what is needed to sit
  consistently next to the new arms.

## Ground truth from the new specimen (do not re-derive; cite it)

`qa/specimens/CurveCatalog-Unrouted.dvc` (real Daslight 5.0.6.2 native capture, five scenes,
`RACK TYPE="8"` -> `EFFECT TYPE="5"`, each `DURATION="5000"`), extracted and appended to
`qa/DVC_CURVE_SOURCE_PARITY.md` under "Real saved specimen: unrouted CURVE generators
(2026-08-11, Fable native capture)":

| XML `ID` | UI generator name | Existing catalog class (static, already proven) |
|---|---|---|
| 5  | Ramp     | `CRampEffect` |
| 6  | Random   | `CRandomEffect` |
| 8  | Sinus3   | `CSinus3Effect` |
| 11 | Tangeant | `CTangeantEffect` |
| 12 | Triangle | `CTriangleEffect` |

All five specimens share the byte-identical common CURVE `PARAMS NB="5"` schema at UI defaults
(`Rate=2, Size=1, Phase=0, Offset=0, Phasing=0`) and the byte-identical PRESET/BEAMS binding
(Dimmer concrete channel `SSLCHANNEL="33"`, `SSLPRESET="0"`, 32-beam ordered `IDSELECTION`). This
specimen is real-file evidence for the ID<->name mapping and the common param schema; it is NOT
evidence for the per-ID evaluator formula (Rate/Size/Offset/Phase/Phasing are all at default in
every capture, so this file alone cannot distinguish, say, a Ramp implementation bug from a
correct one). The evaluator formulas for these five IDs must still be recovered by static
decompilation, exactly like Sinus/Inverse Ramp/Pulse/Square/Strobe were.

## Per-ID implementation requirements

For every one of the five IDs:
1. Recover the exact evaluator formula from the installed binary (same method/tooling already
   used and documented for the five routed CURVE IDs in `DVC_CURVE_SOURCE_PARITY.md` -- follow
   its "Recovered sample equations" section as the template for how to write up what you find).
2. Classify any recovered defect (40 ms hold, integer/grid residue, DURATION-dependent amplitude,
   etc.) against `qa/DVC_CORRECTED_COMPATIBILITY_POLICY.md`'s acceptance rule and correct it the
   same way the existing five CURVE routes already do (continuous-time evaluation, authored
   Rate/Size/Offset/Phase/Phasing preserved, beam/selection order preserved, clamp to 0..1).
3. Land the corrected analytic evaluator directly in Syndocal-native normalized domains -- do NOT
   reintroduce a `daslight_exact` / dual-route mode. This matches the abolished-dual-route ruling
   already recorded for Burst/Sweep/Knight Rider/Perlin/Sparkle/Random Fill in
   `DVC_CORRECTED_COMPATIBILITY_POLICY.md`.
4. Reuse an existing `LfoShape` variant plus the existing `DaslightCurveSource` provenance profile
   wherever the recovered formula is genuinely representable by it (this is expected for Ramp,
   Random, Triangle per the naming/class parallels above -- but verify, don't assume). Add a new
   `LfoShape` variant only where the recovered formula is not representable (most likely for
   Sinus3 and Tangeant).
5. Preserve the strict `TYPE`/`ID` PARAM schema validation pattern already used by the five
   existing CURVE import functions (reject wrong param count/types/IDs rather than best-effort
   parsing).
6. Preserve fixture/beam/selection order and the Dimmer-only targeting behavior already used by
   the existing CURVE imports (`retain_dvc_dimmer_targets`, `dvc_effect_beam_targets`).
7. Write the import `note` string in the same `feature=...; implementation=SyndocalCorrected;
   duration_ms=...; ...` key=value style as the existing five, naming the evaluator address and
   the specific correction reason.

## Required tests

- A specimen golden test: load `qa/specimens/CurveCatalog-Unrouted.dvc` (already present in the
  repo at that path) and assert successful conversion for all five new IDs (5/6/8/11/12), with
  correct target shape/attribute, correct beam count (32) and order, correct label, and a
  `daslight_curve` (or new profile, if a new `LfoShape` was added) provenance block with
  `rate=2, size=1, offset=0` matching the captured defaults. Follow the pattern of the existing
  CURVE-3/CURVE-7 tests around `crates/.../dvc_import.rs` lines 7249 and 7329 (search for
  `"CURVE 7 must be stored as cue-owned LFO params"` / `"CURVE 3 must be stored as cue-owned LFO
  params"`) and the synced-Sinus test near line 8077-8111.
- Focused engine regressions per new/changed evaluator, following the existing pattern: name the
  recovered defect, show the corrected result, per the acceptance rule in
  `DVC_CORRECTED_COMPATIBILITY_POLICY.md` item 3 ("Add a deterministic regression showing the
  recovered defect and the corrected result").
- `cargo test -p syndocal dvc_ -- --nocapture` must stay green and grow (currently 45 passed for
  the CURVE work alone; the new specimen file adds real coverage on top).
- `cargo test --workspace --all-targets` must stay green (only the existing hardware/GPU/
  long-duration ignores are acceptable).

## JA localization

Any newly user-facing generator/shape label (in Scene Settings, effect list summaries, or
anywhere else the shape name surfaces) needs a JA string in `app/src/uiLocalization.ts`, and
`node app/scripts/check-localization.mjs` (or the current localization check script name/path --
confirm it before running) must stay at zero bare user-data labels.

## Standard gates (must all pass before calling this tranche complete)

- `cargo fmt --check` (or `cargo fmt --all -- --check`, confirm current invocation) with no diff
  beyond the known `C:\Users\kouty` canonicalize warning.
- `cargo test -p syndocal dvc_ -- --nocapture`
- `cargo test --workspace --all-targets`
- `pnpm --dir app build` (or the equivalent direct `tsc --noEmit` + `vite build` invocation if
  pnpm's node_modules-purge prompt risk is present in this checkout -- check `CLAUDE.md`'s "Known
  Verification Commands" notes before choosing)
- `npm run check:cue-effect-recall --silent` and `npm run check:scene-settings --silent` (extend
  their coverage to the new shapes if they enumerate shapes by name)
- `npm run check:localization --silent` (confirm exact script name first)
- Native release build only if this tranche is treated as a milestone; otherwise a lightweight
  `cargo check -p syndocal --locked` plus the above is sufficient per `CLAUDE.md`'s stated
  preference for lightweight checks on small changes.

## Explicit TO-VERIFY items (do not guess; state findings, not assumptions, in the report)

- TO-VERIFY: Whether `CRampEffect` (ID5 Ramp) is exactly the ascending mirror of the already
  recovered `CRampInvEffect` (ID3 Inverse Ramp), or has independently different Rate/Size/Offset
  handling. Decompile before assuming symmetry.
- TO-VERIFY: Whether `CRandomEffect` (ID6 Random) matches the existing `LfoShape::Random`
  (`stepped_noise(phase, 16)`) step count and seed/determinism semantics, or is a distinct
  stepped/held random generator that needs its own evaluator and possibly its own
  `DaslightCurveSource`-style provenance fields.
- TO-VERIFY: Whether `CTriangleEffect` (ID12 Triangle) matches the existing `LfoShape::Triangle`
  evaluator exactly, or has different Rate/Phase handling than the generic native Triangle.
- TO-VERIFY: The exact evaluator formulas for `CSinus3Effect` (ID8 Sinus3) and `CTangeantEffect`
  (ID11 Tangeant) -- both need static recovery from the binary; do not infer semantics from the
  UI name alone (e.g. do not assume "Sinus3" is literally `sin(3x)` without decompiling; Daslight
  naming has not been a reliable formula predictor for prior recoveries such as Pulse's
  fixed-slope triangle window).
- TO-VERIFY: Whether the frontend Scene Settings shape selector needs new option entries, or
  whether reused shapes (Ramp/Random/Triangle) already surface correctly once the importer routes
  them, given the existing UI likely already lists `Saw`/`Random`/`Triangle` as native shapes.
- TO-VERIFY: Current exact script names/paths for `check:cue-effect-recall`, `check:scene-
  settings`, and the localization check, since `CLAUDE.md`'s own history shows script names and
  invocation methods (pnpm vs. direct node) have changed across the project's life.
- Custom (ID13) stays explicitly out of scope and fail-closed; do not attempt it in this tranche.

## Report contract

When done, report: (1) the recovered formula for each of the five IDs with evaluator address and
classification (reused shape vs. new shape) and the specific defect corrected, if any; (2) which
frontend files were touched for Scene Settings exposure and JA localization; (3) full gate output
summary (pass counts, not full logs); (4) explicit resolution of every TO-VERIFY item above,
including any where the answer is "confirmed to diverge from the naive assumption" rather than
just "confirmed as assumed"; (5) exact commit(s) created and whether pushed to
`origin/codex/syndocal-v1.0`.
