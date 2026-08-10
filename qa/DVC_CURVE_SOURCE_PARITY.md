# Daslight DVC Curve source parity evidence

Date: 2026-08-09

## Source under test

- Installed binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- Product version: `5.0.6.2`
- File version: `25.0905.165.111`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Static evaluator entry points recovered from RTTI/vtables:
  - `CSinusEffect`: `0x140370250`
  - `CInverseRampEffect`: `0x14036fcb0`
  - `CPulseEffect`: `0x14036F8D0`
  - `CRampEffect`: `0x14036FAE0`
  - `CRandomEffect`: `0x14036FE90` (constructor `0x14036E8C0`)
  - `CSinus3Effect`: `0x140370070`
  - `CSquareEffect`: `0x140370420`
  - `CStrobeEffect`: `0x1403705b0`
  - `CTangeantEffect`: `0x140370760`
  - `CTriangleEffect`: `0x140370930`
- The imported source buffer is evaluated on a 40 ms grid.

The evidence above is local static analysis of the named binary. It is version-specific and must
not be generalized to a different Daslight build without repeating the analysis.

## Recovered sample equations

For `sample_count=floor(DURATION/40)` and `t=sample_index/sample_count`:

- Sinus:
  `clamp(sin(2*pi*(Rate/2*t-Phase))*Size/2 + Offset + Size/2, 0, 1)`
- Inverse Ramp:
  `x=Rate/2*t-Phase`, `centered=x-floor(x+0.5)`, then
  `clamp(Offset-centered*Size+Size-0.5, 0, 1)`
- Ramp:
  `x=Rate/2*t-Phase`, `centered=x-floor(x+0.5)`, then
  `clamp(Offset+centered*Size+Size-0.5, 0, 1)`. The sign in front of `centered`
  is the recovered distinction from Inverse Ramp; swapping request endpoints is not equivalent.
- Random:
  `step=floor(abs(Rate*pi*t-Phase*2*pi))`, then
  `clamp(Size*(random_table[step]/100+Offset), 0, 1)`. Constructor `0x14036E8C0`
  appends exactly 400 `qrand()%100` bytes to the instance table. The process-global qrand
  history is not serialized in `.dvc`, so the table cannot be reconstructed from the file.
- Sinus3:
  `clamp(sin(2*pi*(Rate/2*t-Phase))^3*Size/2 + Offset + Size/2, 0, 1)`
- Tangeant:
  `clamp(tan(2*pi*(Rate/2*t-Phase))*Size/2 + Offset + Size/2, 0, 1)`
- Triangle: letting `x=2*pi*(Rate/2*t-Phase)`, the binary computes
  `(-1)^floor(x/pi-0.5) * (x-floor(x/pi+0.5)*pi) * 2/pi`; equivalently the
  normalized source is
  `clamp(Triangle(Rate/2*t-Phase+0.75)*Size+Offset, 0, 1)`.
- Pulse:
  `carrier=sin(2*pi*(2*Rate*t-Phase))`; Daslight multiplies the carrier by a triangle
  whose fixed slope is `0.005` per sample. Consequently its peak is
  `floor(sample_count/2)*0.005`, so DURATION changes the effective Size and short buffers never
  reach the authored amplitude.
- Strobe:
  `interval=floor(25/Rate)`, `remainder=sample_index%interval`; High when
  `remainder==0` or `remainder<interval*Phase/2`, otherwise Low. Low is `Offset`; High is
  `Offset+Size/2`; both are clamped to 0..1.
- Square:
  `grid=floor((sample_index%sample_count)*400/sample_count)`,
  `cell=trunc(grid+400-Phase*400)%400`, `band=trunc(cell/floor(400/Rate))`;
  even bands are `Offset+Size`, odd bands are `Offset`, then clamped to 0..1. The integer
  `floor(400/Rate)` preserves Daslight's uneven terminal band for Rates that do not divide 400.

`DURATION` is the complete sampled Curve buffer, not `DURATION/Rate`. Rate controls how many
wave cycles are drawn into that buffer. The former Syndocal conversion divided DURATION by Rate
and approximated Strobe as ten 2%-wide pulses; both assumptions were removed.

## Syndocal representation

Imported LFO requests retain an additive `daslight_curve` source profile containing `rate`,
`size`, `offset`, and recovered provenance `sample_ms=40`. Native Syndocal LFOs omit the profile
and retain their existing continuous evaluator.

All routed DVC Curve sources now follow `DVC_CORRECTED_COMPATIBILITY_POLICY.md`:

- Sinus and Inverse Ramp retain the recovered equations but evaluate continuous `t` instead of
  holding 40 ms work samples.
- Pulse retains the recovered carrier and all authored parameters, but uses
  `window=1-abs(2*t-1)` on continuous time. This removes both the DURATION-dependent amplitude
  defect and the 40 ms output hold.
- Square uses `band=floor(fract(t-Phase)*Rate)`, removing the 400-cell residue while preserving
  authored band count, alternating polarity, Size and Offset.
- Strobe uses the exact authored `1/Rate` interval and a common dimensionless 20% base duty,
  extended by `max(0.2, Phase/2)`. Duty is independent from the recovered 40 ms sample grid and
  Rate, so high Rates do not collapse into an always-high output.
- Ramp, Sinus3, Tangeant and Triangle retain the recovered analytic equations and native clamp,
  but evaluate continuous source progress instead of holding 40 ms work samples. Ramp and the
  two distinct harmonic shapes have dedicated `LfoShape` identities; Triangle reuses the native
  triangle only after applying the recovered three-quarter-cycle alignment.
- Random retains the recovered angle-derived integer step cadence and 100 discrete value buckets.
  Its unavailable process-global qrand history is replaced with the same stable source-identity
  seed policy used by the corrected Random Fill and Sparkle importers. The seed is stored in the
  additive `daslight_curve` provenance block, so project reloads are deterministic.

Each import report records the recovered grid and the applicable correction; none of these
routes is described as frame-equivalent Daslight output. Size and Offset remain source values
instead of being reduced to clamped endpoints: every evaluated value is clamped. This is
necessary for
`homecoming2606.dvc` scene `all_rampFlash`, whose raw
Inverse Ramp range is approximately `-0.567..0.995`; its negative portion must remain at DMX 0
for a finite interval.

Scene Settings loads the complete imported LFO request into its draft and writes the same source
profile back when saving. The source profile also round-trips through `.sdc`, project history,
Cue-owned effect storage, effect snapshots, and presets. Legacy and native LFO JSON omits the
field and continues to deserialize with `None`.

## Regression evidence

- Focused engine regressions cover removal of the 40 ms hold, exact Strobe Rate with common
  dimensionless duty, equal Square bands, normalized Pulse amplitude, Sinus source Phase, a
  clipped Sinus plateau, the real `all_rampFlash` descending range/zero plateau, Ramp direction,
  cubed Sinus3, clamped Tangeant, Triangle alignment, deterministic source-seeded Random, and
  invalid source profiles.
- `cargo test -p syndocal dvc_ --locked -- --nocapture`
  - 81 passed, 0 failed.
  - Covers synthetic conversion plus all locally available `.dvc` inventories and goldens.
- `cargo test --workspace --all-targets --locked`
  - All non-ignored tests passed. Engine: 552 passed/2 manual benchmarks. Tauri: 410 passed/9
    environment-dependent tests. Video: 114 passed/1 real-GPU test. Other crates: all passed.
  - The first workspace run ended after the video suite with Windows
    `STATUS_ACCESS_VIOLATION`; `cargo test -p video --lib --locked -- --test-threads=1
    --nocapture` passed 110/0/1 and an unchanged full-workspace rerun passed, classifying the
    first termination as non-reproducing native decoder/GPU test-process interference.
- `node scripts/check-cue-effect-recall.mjs`
  - Passed; source request synthesis and Scene Settings load/save preservation are guarded.
- `node scripts/check-viewport-containment.mjs --scene-settings-only`
  - Passed at 1920x1080, measured 1920x1032, 2048x1152, 1366x768, and 1280x720 with no app
    scroll or failed interaction assertions.
- `node scripts/check-effect-visualization.mjs`
  - Passed; the preview mirrors all five recovered evaluators and deterministic Random steps.
- `node scripts/check-localization.mjs`
  - Passed at 3094/3094 static Japanese strings (100.0%) and zero bare user-data labels.
- `node node_modules/typescript/bin/tsc --noEmit` and
  `node node_modules/vite/bin/vite.js build --configLoader runner`
  - TypeScript and Vite production builds passed.
- `pnpm --dir app tauri build --no-bundle`
  - Native release build passed and produced `target/release/syndocal.exe`.
  - The exact checkout executable was launched as PID 59836; exactly one `Syndocal` window was
    present (`MainWindowHandle=10030242`) and Win32 reported `Responding=True`.

## Remaining evidence boundary

- The recovered ID3-12 generator equations and the locally present Curve scenes are
  software-verified. ID13 Custom uses its separate points-list schema and remains a distinct
  pending tranche.
- The available real DVC Curve specimens use zero Phasing. Non-zero fixture Phasing is preserved
  and covered by deterministic engine tests, but has not been compared against a physical or
  captured Daslight multi-fixture output trace for this binary.
- A physical controller/fixture is not required to verify these normalized DMX time series, but
  fixture photometry, PWM response, and device latency remain physical acceptance items.
- Native responsive-window acceptance is run separately; it is not inferred from the successful
  release build.

## Real saved specimen: unrouted CURVE generators (2026-08-11, Fable native capture)

### Capture context

- File: `qa/specimens/CurveCatalog-Unrouted.dvc` (134,113 bytes, `DASBUILD="25.0905.165.111"`,
same installed binary/SHA-256 recorded above).
- Five scenes were created in sequence in the running Daslight UI, one CURVE rack per scene
(`RACK TYPE="8"` -> `EFFECT TYPE="5"`), selecting the generator from the dropdown in creation
order: Sinus3, Tangeant, Triangle, Ramp, Random.
- All five generators were left at UI default parameters: Rate=2, Size=1.0, Phase=0, Offset=0,
Phasing=0, Attribute value=Absolute.
- Each scene was bound through a scratch save-as to the same mega-bar fixture's concrete Dimmer
channel PRESET (`SSLFIXTURE=69bdd010-d626-11ea-b9df-7da99bfefe5c-3afb4fbb`, `SSLCHANNEL="33"`,
`SSLPRESET="0"`, `MIN="0"`, `MAX="1"`), with an identical ordered 32-beam `IDSELECTION` list
(4 fixtures x 8 beams each) reproduced in every one of the five scenes.
- `DURATION="5000"` (ms) on every captured `EFFECT`.
- Screenshot evidence: `target/qa/ui-comparison/bks-specimens/43-*.png` through `50-*.png`.

### Confirmed name <-> ID table

Extracted via XML parse of the five `EFFECT TYPE="5"` blocks (document/creation order, top to
bottom in the file):

| Document order | XML `ID` | UI generator name (creation order) | Matches `DVC_FULL_FX_CATALOG_PARITY.md` static row |
|---|---|---|---|
| 1 | `8`  | Sinus3   | `8 Sinus3` (`CSinus3Effect`) |
| 2 | `11` | Tangeant | `11 Tangeant` (`CTangeantEffect`) |
| 3 | `12` | Triangle | `12 Triangle` (`CTriangleEffect`) |
| 4 | `5`  | Ramp     | `5 Ramp` (`CRampEffect`) |
| 5 | `6`  | Random   | `6 Random` (`CRandomEffect`) |

The extracted ID set is exactly `{5, 6, 8, 11, 12}` -- the full unrouted CURVE set minus Custom
(`13`). XML document order, live UI authoring order, and the pre-existing static RTTI/vtable
class-name recovery in `DVC_FULL_FX_CATALOG_PARITY.md` agree on all five names with no
discrepancy; this specimen upgrades that table's rows 5/6/8/11/12 from static-only to
static+live-capture confirmed.

### Per-ID PARAMS schema (all five specimens)

Every captured `EFFECT TYPE="5"` carries the same common CURVE `PARAMS NB="5"` schema already
documented in `DVC_FULL_FX_CATALOG_PARITY.md` (constructor `0x14036E500`), all at authored UI
defaults:

```
PARAMS NB="5"
  PARAM TYPE="0" ID="1" VAL="2"   -- Rate      [1..10], authored default 2
  PARAM TYPE="1" ID="2" VAL="1"   -- Size      [0..2],  authored default 1
  PARAM TYPE="1" ID="3" VAL="0"   -- Phase     [0..1],  authored default 0
  PARAM TYPE="1" ID="4" VAL="0"   -- Offset    [-1..1], authored default 0
  PARAM TYPE="1" ID="5" VAL="0"   -- Phasing   [0..1],  authored default 0
```

This is byte-identical across IDs `5`, `6`, `8`, `11`, `12` -- only the parent `EFFECT ID`
attribute and the enclosing `SCENE DASUID` differ. `EFFECT DURATION="5000"` on all five. The
`PRESET`/`BEAMS` binding block (Dimmer concrete-channel PRESET + 32-beam ordered `IDSELECTION`)
is also byte-identical across all five scenes, confirming the capture methodology held the
target/binding fixed while only the generator selection changed.

### Conflict check against the catalog doc

None. `DVC_FULL_FX_CATALOG_PARITY.md`'s CURVE FX table (`## CURVE FX -- family/type 5`) already
listed `5 Ramp`, `6 Random`, `8 Sinus3`, `11 Tangeant`, `12 Triangle` from static factory/ctor/
vtable recovery, and states its remaining 8 rows (of which these 5 are a subset) have exact
factory/schema recovery but no decompiled formula yet. This specimen corroborated the ID/name
assignment for exactly the 5 generators captured here via an independent live-authoring path.
The evaluator formulas are now independently recovered from the pinned executable at the
addresses recorded above; they are not inferred from this default-parameter specimen.
