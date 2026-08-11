# Daslight MAPPINGS shared 2D raster parity

Date: 2026-08-11

## Binary and specimen identity

- Binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- ProductVersion: `5.0.6.2`
- FileVersion: `25.0905.165.111`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Real saved specimen: `qa/specimens/MappingCatalog-522-529.dvc`
- Implemented in this tranche: MAPPINGS 522 Spiral, 523 Burst, 524 Butterfly,
  525 Plasma, 526 Media with an empty source path, 527 Knight Rider, 528 Sweep,
  and 529 Sparkle.

The specimen was created and saved by Daslight itself. It pins each `RACK TYPE=6 /
EFFECT TYPE=8` ID, `DURATION=5000`, its factory-default PARAM schema, the selected
beam order, and one Rectangle MAPPING. It is source evidence, not a hand-written XML
fixture.

## Shared class proof

The family-8 factories reuse the same evaluator classes already recovered for VALUE
and COLOR FX:

| MAPPINGS ID | Class / evaluator | Factory ID store | Saved class fields |
|---:|---|---:|---|
| 522 | `CSpiralEffect / 0x140366240` | `0x1403686E9` | Radius 0..200, Arms 1..10, Gradient 0..1 |
| 523 | `CBurstEffect / 0x140362B70` | `0x1403685F7` | Color Width 10..900, Gradient 0..1 |
| 524 | `CButterflyEffect / 0x140362EB0` | `0x140368514` | Color Width 1..100, Gradient 0..1, Clockwise |
| 525 | `CPlasmaEffect / 0x1403654F0` | `0x140368437` | Size/Param X/Y 0..20, Speed/Param S X/Y -5..5 |
| 527 | `CKnightRiderEffect / 0x140363FE0` | `0x140368284` | Size 1..100, One Way, Fading, Go Outside, Gradient 0..100 |
| 528 | `CSweepEffect / 0x1403665A0` | `0x1403681AE` | Direction Change |
| 529 | `CSparklesEffect / 0x1403660F0` | `0x1403680D8` | Number 1..10, LifeSpan 0..0.9, Width/Height 1..90 |

`qa/DVC_BURST_EXACT_PARITY.md`, `qa/DVC_VALUE_CATALOG_PARITY.md`, and the
corrected-evaluator sections of `qa/DVC_FULL_FX_CATALOG_PARITY.md` remain the source
proof for those evaluator equations. In particular, Burst constructs a radial
gradient at `(raster_width/2, raster_height/2)` and Qt samples both axes at pixel
centres. The MAPPINGS route therefore evaluates Euclidean `(x,y)` radius; reducing it
to selection order would be observably wrong.

## MAPPINGS base and corrected placement

Every implemented raster validates the common base exactly:

- `TYPE4/ID1` palette, 1..255 colors;
- `TYPE6/ID3` Transform: 0 none, 1 fold X, 2 fold Y;
- `TYPE0/ID4` raster Rotation: integer 0..360;
- one positive-extent Rectangle MAPPING;
- explicit Patch-canvas coordinates for every retained beam target.

The engine first inverse-transforms the rotated Rectangle inclusion mask into its
local frame. It then applies the authored Transform and an inverse raster Rotation
around `(0.5,0.5)`. Pixels rotated outside the finite Rectangle are clipped to
transparent black. This is the existing product correction for Daslight's
axis-aligned-raster lookup defect: mask and raster now share one continuous local
coordinate frame, without nearest-neighbour holes, edge smearing, or per-tick
allocations.

The shared evaluator projections are:

- Burst: 100x100 pixel-centre Euclidean radius and cyclic palette phase.
- Plasma: the recovered byte formula receives both local `x` and `y` in 0..99.
- Knight Rider: the recovered moving head is sampled across the local X axis of the
  100px raster.
- Sweep: the recovered hard boundary is sampled across that same local X axis.
- Spiral: `CSpiralEffect` helper `0x140360400` advances a conical palette by
  `floor(Radius/10)` degrees for each one-pixel annulus; Arms repeats that palette
  around 360 degrees. Polar distance and angle use the same 100x100 pixel-centre
  coordinates as the shared Burst raster. The analytic sampler preserves the
  recovered geometry and authored period without the source QImage/frame cache.
- Butterfly: helper `0x14035E950` paints two conical sectors separated by 180
  degrees. Color Width bounds each sector, Gradient shapes its palette ramp, and
  Clockwise reverses the period rotation. The analytic sampler replaces the doubled
  image/crop implementation while preserving the recovered sector geometry.
- Sparkle: the corrected retained-particle state is now a 100x100 source raster.
  Stable source seeding selects both X and Y, Width and Height fill a rectangular
  particle footprint, and the saved LifeSpan still maps to
  `round(100/(1-LifeSpan))` milliseconds.

The pre-existing COLOR/VALUE strip paths still call their original integer-index
entry points. A regression compares every 1D Burst sample with the new shared
coordinate entry point, proving this refactor does not change the former route.

## Media empty-source boundary

The real MAPPINGS 526 specimen differs from COLOR MAPPINGS Media. It saves exactly:

```text
TYPE6/ID3 Transform=0
TYPE0/ID4 Rotation=0
TYPE8/ID10 Media Path=""
```

An empty path is a validated source no-op and creates no runtime effect. A non-empty
path remains precise fail-closed until the embedded image/video decode and timing
contract is recovered. The importer does not invent a palette or substitute another
generator.

## Wire compatibility and editor exposure

`ColorEffectSpatialPlacement` gained serde-defaulted, default-omitted
`vertical_symmetry`, `horizontal_symmetry`, and `raster_rotation_degrees`. Existing
`.sdc v1` placement JSON retains its byte shape when the fields are false/zero.
Placed shared recipes expose the common Mapping transform and Mapping rotation fields
in Scene Settings; the existing recipe-specific controls remain unchanged.

## Remaining boundary

All ten MAPPINGS IDs 521..530 now have strict converter routes; 526 is intentionally
a source no-op only for the verified empty path. Spiral and Butterfly are reported as
`SyndocalCorrected`: their recovered geometry is preserved by continuous analytic
sampling rather than Qt gradient quantization and QImage frame caches. Sparkle is
also `SyndocalCorrected` because Daslight's process-global qrand history is not
serialized; its saved population grammar and true 2D footprint are preserved with a
stable source seed.

The 2026-08-11 shared COLOR MAPPINGS tranche now routes counterparts 22 Burst,
23 Butterfly, 30 Knight Rider, 32 Perlin, 34 Plasma, 40 Sparkle, 42 Spiral, and
44 Sweep through these recipes and evaluators. Their source family remains Color
Mappings: they target owned color segments with Override merge, preserve T2/ID2
Grayscale, and reject external SELECTIONS. Spiral and Butterfly apply Grayscale in
the common post-process after their recovered geometry is sampled.

COLOR MAPPINGS 33 Media is separately strict NB=6 with T8/ID10 Media Path and
T1/ID11 Colorize. Only an empty path is a source no-op; non-empty media remains
precise fail-closed. No family-5 COLOR MAPPINGS saved specimen exists in the
repository, so this extension is constructor- and synthetic-schema-proven only;
the real specimen claim at the top of this document remains limited to MAPPINGS
522–529. Family-specific COLOR MAPPINGS classes remain for later tranches.
