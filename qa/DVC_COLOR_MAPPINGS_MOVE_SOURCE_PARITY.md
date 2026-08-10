# Daslight COLOR MAPPINGS Rainbow / Move Circle source-parity evidence

## Scope and identity

- Audit date: 2026-08-10
- Audited binary: `C:\\Daslight 5\\Daslight 5\\Daslight 5.exe`
- File version: `25.0905.165.111`
- Product version: `5.0.6.2`
- Size: `9,778,688` bytes
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Saved specimen: `qa/specimens/ValueCatalog-Sweep-Plasma.dvc`
- Specimen size: `51,273` bytes
- Specimen SHA-256: `0A4785BC8F2929AA781AC2CD2D5121D63424371EC3421BBD3C95D7F13E6493B4`
- Method: static PE RTTI/vtable tracing, read-only disassembly, committed XML and
  PATCH-body decoding, followed by protocol, engine, importer, project-persistence,
  and editor-preview regression coverage. Daslight was not launched or modified.

All addresses below are image virtual addresses for the exact audited hash. They are
evidence for that build, not a compatibility promise for another Daslight version.

## Disposition

| Source | Disposition | Exactness boundary |
|---|---|---|
| `RACK TYPE=5 / EFFECT TYPE=3 / ID=36` Rainbow | **Implemented exactly in DVC-V5a** | The approved additive Rainbow/placement protocol represents all seven properties and the Patch-canvas Rectangle sampler. The committed body has no runtime targets, so it is validated and reported as a converted source no-op without fabricating a cue effect target. |
| `RACK TYPE=4 / EFFECT TYPE=4 / ID=221` Circle | **Implemented exactly for the full domain; V5b foundation completed by DVC-MOVE-EXACT** | The additive Move protocol retains all ordered fixture/`BEAMID`/selection identities and selects a dedicated 40 ms Daslight Circle mode. The patched Mega Bar has no Pan/Tilt, so this source body remains dormant rather than fabricating movement; the newer moving-head specimen proves a bindable Pan/Tilt route. Raw ID2 `0..1` is exact. |

DVC-V5a implemented the approved COLOR proposal and related MAPPINGS ID521
Rectangle-preservation fix. DVC-V5b implements the separately approved Move extension:
ordered beam targets, exact Circle path/fan-out semantics, the committed ID221 import,
and minimal Scene Settings exposure. DVC-MOVE-EXACT later completes IDs 221–225 and
the nonzero-Phasing/two-wing frame contract. All tranches keep legacy `.sdc v1` bodies
additive and byte-shaped as before when their optional fields are absent.

## Saved-specimen cross-check and corrected target fact

The committed bytes contain the following two bodies:

| Field | COLOR MAPPINGS Rainbow | Move Circle |
|---|---|---|
| Rack / effect / ID | `5 / 3 / 36` | `4 / 4 / 221` |
| Duration | `5000 ms` | `5000 ms` |
| Parameters | NB=7: `(TYPE,ID,VAL) = (4,1,COLORS×8), (2,2,0), (6,3,0), (0,4,0), (1,10,0), (0,11,0), (1,12,1)` | NB=3: `TYPE5/ID1 POINTS×4`, `TYPE1/ID2=0`, `TYPE2/ID3=0` |
| Placement / targets | Rectangle `(X,Y,SX,SY,ANGLE)=(1994,166,513,52,0)`; `LOCKED=0`; **`BEAMS NB=0` and no `SELECTIONS`** | one fixture UUID; `BEAMID=1..6` and literal `IDSELECTION=1..6`, in that order |
| Scene coordinate mode | `ATTRIBUTEVALUE_MODE=0` | `ATTRIBUTEVALUE_MODE=0` |

The request note says the family-5 rack has two BEAMS, but the byte-exact committed
file has `<BEAMS NB="0"/>`. The importer regression pins the committed fact. Static
analysis also shows that Rectangle overlap does not invent targets: the evaluator
iterates an external `SELECTIONS` container when one exists, otherwise the rack-owned
BEAMS container. With neither target source populated, this saved rack evaluates no
beams. Two decoded Patch fixture origins do lie inside the Rectangle, but they are
geometric measurements only, not saved target membership.

## COLOR MAPPINGS `(5,3,36)`

### Factory and class identity

| Evidence | Finding |
|---|---|
| `0x14036B7AD..0x14036B7CC` | Registers creator `0x14036D4A0`, generator ID 36, family 3. |
| `0x14036A38E..0x14036A3AD` | Registers the same creator for COLOR FX `(2,130)`. |
| `0x1403687D3..0x1403687F2` | Registers the same creator for MAPPINGS `(8,521)`. |
| creator `0x14036D4A0`, call at `0x14036D4D7` | Allocates `0x1A8` bytes and calls constructor `0x140354A80`. |
| vtable `0x140695AF8`, RTTI descriptor `0x1408EF740` | Identifies the constructed class as `CRainbowEffect`. |

ID36 therefore uses the same `CRainbowEffect` implementation as IDs 130 and 521;
the family-specific common constructor controls the property/raster context.

### Property registration and saved NB=7

| DVC property | Registration evidence | Recovered contract | Saved value |
|---|---|---|---:|
| TYPE4/ID1 Palette | `0x1403511A4..0x1403511FE` | `COLORS` palette | 8 colours |
| TYPE2/ID2 Grayscale | family 2/3 guard `0x14035125F..0x1403512B6` | Boolean | 0 |
| TYPE6/ID3 Transform | `0x1403512B6..0x1403513AB`; family 3/8 Horizontal entry `0x140351326..0x14035135F` | 0 None, 1 Vertical symmetry, 2 Horizontal symmetry | 0 |
| TYPE0/ID4 Rotation | family 3/8 guard `0x1403513B9..0x140351419` | integer 0..360 degrees | 0 |
| TYPE1/ID10 Color Width | `0x140354AFA..0x140354B57` | 0..1 | 0 |
| TYPE0/ID11 Angle | `0x140354B5E..0x140354BB2` | integer 0..360 degrees | 0 |
| TYPE1/ID12 Gradient | `0x140354BB9..0x140354C0B` | 0..1 | 1 |

The seven registrations reconcile exactly with the saved order, types, and defaults.
Family 3 selects a `100 × 100` raster at `0x140350D03..0x140350D37`.
The Rainbow evaluator is `0x140365A00`; Grayscale/Transform post-processing is
`0x140358DD0`.

### Headline placement result: Patch-canvas Rectangle window

The family-5 Rectangle is not Syndocal stage X/Z metadata. It is an axis-aligned
sampling window plus a separately rotated inclusion mask in Daslight's integer Patch
layout canvas.

| Stage | Static evidence | Exact behavior |
|---|---|---|
| Beam point | `0x1402D5000`; parent QPoint getter `0x1402D80E0`; fixture-angle transform `0x1402D5078..0x1402D5180` | Start from the fixture Patch position, add the beam/local offset, then apply fixture rotation. Sub-beams need not share the fixture origin. |
| Mapping shape | builder `0x140311B30`; rotation `0x140311BEA..0x140311C22`; AABB re-anchor `0x140311C72..0x140311CBD` | Scale the Rectangle primitive to `SX/SY`, rotate it about its centre by `MAPPING@ANGLE`, then re-anchor the rotated bounding-box top-left to raw `X/Y`. |
| Target source | family-5 loop `0x14001BC92..0x14001BCCC`; constructor `0x1402FB060`; SELECTIONS load `0x14001ED51..0x14001EE35` | Iterate external `SELECTIONS` at rack `+0x20` when present, otherwise owned BEAMS at `+0x8`. Rectangle hit-testing filters that list; it does not scan every fixture. |
| Inclusion | `0x14001BCF1..0x14001BD0E` | Hit-test each selected Patch beam point against the transformed shape. |
| Sampling | `0x14001BD14..0x14001BDD4` | `ix = trunc((beam_x - X) * 100 / SX)` and `iy = trunc((beam_y - Y) * 100 / SY)`. There is no inverse `MAPPING@ANGLE` transform before sampling. |

Thus `MAPPING@ANGLE` rotates the inclusion mask only. Generator PARAM ID4 Rotation
rotates the generated raster and remains a distinct property.

For the saved Rectangle, decoded strongpoint fixture origins at `(2004,176)` and
`(2467,178)` would produce, before any beam-local offset:

| Patch point | Window-normalized coordinate | Raster cell by recovered truncation |
|---|---|---|
| `(2004,176)` | `(10/513, 10/52) = (0.019493, 0.192308)` | `(1,19)` |
| `(2467,178)` | `(473/513, 12/52) = (0.922027, 0.230769)` | `(92,23)` |

Again, the saved empty target containers mean these overlapping points are not
actually evaluated by this rack.

### Relationship to MAPPINGS ID521

Family 6/8 ID521 uses the same Rectangle algorithm, not a different stage-X/Z
placement model. Its sampler at `0x14001C260` builds and hit-tests the shape at
`0x14001C2EC..0x14001C38D`, subtracts raw X/Y at
`0x14001C393..0x14001C3B3`, and scales into the raster at
`0x14001C3B7..0x14001C445`.

Before DVC-V5a, Syndocal normalized fixture stage X/Z across the resolved target set,
gave selected sub-beams the fixture position, and discarded the Rectangle. In the
committed ID521 body, Rectangle `(2630,-140,140,50)` places Patch point
`(2640,-130)` at `(0.071429,0.2)` / raster `(7,20)`, while the old single-fixture
normalization produced `(0.5,0.5)` for all four BEAMIDs.

DVC-V5a corrects ID521 through the same optional placement body used by family-5
Rainbow. The importer retains the raw signed Rectangle and exact Patch-canvas point
for each fixture/beam identity. The engine compiles the rotated inclusion geometry and
raw axis-aligned raster cell before the 44 Hz evaluator runs. Therefore the comparison
above now resolves to cell `(7,20)` instead of the old normalized centre. PARAM ID4
Rotation remains generator-raster rotation; `MAPPING@ANGLE` remains inclusion-mask
rotation only.

### Why ID36 is now exactly representable

DVC-V5a extends `ColorEffectSpatialRecipe::Rainbow` instead of adding a parallel
family-3 recipe. This is the narrower representation because IDs 36, 130, and 521 use
the same `CRainbowEffect`, while the existing Rainbow recipe already owns both
symmetries and generator Rotation. Its additive Grayscale field closes the remaining
recipe gap.

`ColorEffectSpatialPattern` now optionally carries the source Rectangle and a
fixture/beam-keyed Patch coordinate table. Runtime placement no longer depends on
stage X/Z normalization. For the committed ID36 rack, the same exact representation
also proves that an empty BEAMS container with no SELECTIONS is a source no-op: the
importer records a converted result but emits no runtime effect target.

### Approved COLOR protocol-change disposition — implemented in DVC-V5a

The 2026-08-10 approval was implemented with this additive shape:

1. `ColorEffectSpatialRecipe::Rainbow` now contains `grayscale`,
   `vertical_symmetry`, `horizontal_symmetry`, `rotation_degrees`, `color_width`,
   `angle_degrees`, and `gradient`. `grayscale=false` is the serde default and is
   omitted on serialization.
2. `ColorEffectSpatialPattern` now contains optional `placement` in addition to
   `recipe` and `beam_targets`. `placement=None` is the serde default and is omitted.
3. `ColorEffectSpatialPlacement` contains coordinate frame
   `DaslightPatchCanvas`, shape `Rectangle`, raw signed `x/y/sx/sy`,
   `mapping_angle_degrees`, sampling rule
   `RotatedInclusionMaskAxisAlignedRaster`, and `target_coordinates`.
4. Each placement target contains `fixture_id`, `beam_index`, and raw signed
   `patch_x/patch_y`; this joins placement to authored target identity without stage
   coordinate reuse.
5. The engine compiles Rectangle centre/extents, mapping-angle sine/cosine, inclusion,
   signed truncating raster cells, and Rainbow projection while rebuilding runtime
   targets. The tick reads fixed per-target coordinates/projection; it performs no
   placement allocation, trigonometry, identity search, or Patch-coordinate lookup.

Legacy Rainbow JSON with no Grayscale and legacy patterns with no placement deserialize
to their defaults and serialize back without adding either field. New populated bodies
round-trip with the explicit placement contract. `LOCKED` remains editor state;
`DASUID` and NAME remain import provenance. The Scene Settings editor exposes Rainbow
Grayscale using the existing localized Grayscale label; the imported placement body is
not an authoring surface.

External `SELECTIONS` remain deliberately fail-closed for ID36. The saved specimen
proves only empty owned BEAMS/no SELECTIONS, so DVC-V5a does not infer how a populated
external selection source should be persisted or resolved.

## Move `(4,4,221)` Circle

### Factory and property registration

| Evidence | Finding |
|---|---|
| `0x140369165..0x140369184` | Registers creator `0x14036CD30`, ID `0xDD` = 221, family/type 4. |
| creator call `0x14036CD67` | Calls constructor `0x1403476C0`. |
| `0x1403476EA..0x1403476F1` | Installs `CCirclePosEffect` vtable `0x1406954C0`. RTTI COL is `0x1407CAF80`; type descriptor is `0x1408EF628`. |

Circle adds no class-specific properties. Common position-effect constructor
`0x140347890` registers exactly:

| DVC property | Registration evidence | Recovered contract | Saved value |
|---|---|---|---|
| TYPE5/ID1 POINTS | `0x140347950..0x1403479A6` | 2..255 normalized X/Y points | four points |
| TYPE1/ID2 Phasing | `0x1403479AA..0x1403479FE` | 0..1, default 0 | 0 |
| TYPE2/ID3 Symmetry | `0x140347A05..0x140347A46` | Boolean, default false | 0 |

The four constructor defaults at `0x140695950`, `0x140695980`, `0x140695960`, and
`0x140695930` are exactly `(0.25,0.5)`, `(0.5,0.75)`, `(0.75,0.5)`, and
`(0.5,0.25)`, matching the committed serializer body.

`Attribute value: Absolute` is not a fourth effect property. The scene key is at
`0x140523F40`; deserialization into scene `+0xD0` is
`0x140011751..0x1400117F0`, `{0,1}` validation is
`0x1400118F1..0x140011908`, and serialization is
`0x140012999..0x1400129B9`. The UI inserts Absolute before Relative at
`0x1402317DA` / `0x140231848`. The saved owning scene has mode 0, hence Absolute.

### Evaluator and path semantics

Circle's vtable slot `+0x80` resolves to evaluator `0x140349650`.

1. `0x1403496E0..0x140349724` computes
   `q = point_count * (sample mod cycle) / cycle`, then uses `floor(q)` as the
   segment and `fract(q)` as its local phase. Every control-point segment receives
   equal time; this is not arc-length redistribution.
2. The two-point branch at `0x140349754..0x140349937` constructs a full analytical
   circle from the pair.
3. For more than two points, `0x14034993C..0x1403499E0` loads wrapped neighboring
   points. Helper `0x1403480F0` recovers signed circumradii for `(prev,from,to)` and
   `(from,to,next2)`. `0x140349A28..0x140349AB7` substitutes the adjacent nonzero
   radius when only one triple is degenerate, then computes the signed average
   `sign(r0) × (abs(r0)+abs(r1))/2`.
4. Equal-sign curvature uses one signed chord arc. Opposite signs use a half-radius
   arc from `from` to the chord midpoint for the first half, then
   `2×midpoint - firstArc(2×(1-t))` for the second half. The recovered ranges are
   `0x140349AC4..0x140349D69`; reflection occurs before the final output clamp.
5. `0x140349B83..0x140349F63` evaluates analytical arcs with sine/cosine. Only a
   degenerate circle falls back to linear interpolation at
   `0x140349FA9..0x14034A06E`.
6. Outputs are clamped to normalized 0..1. The saved four cardinal points share centre
   `(0.5,0.5)` and radius `0.25`, so they produce four exact quarter-circle arcs in
   listed order.

Fan-out is at `0x14034AB70`. It computes
`step = cycle_samples × scalar` at `0x14034ABE0..0x14034ABF1`.
With Symmetry off, target `j` samples `wrap(time - j×step)`. With Symmetry on,
Circle's vtable `+0xC8` method returns true and the second half is traversed in reverse
from `wrap(ceil(cycle/2) - time - k×step)` at
`0x14034AD28..0x14034AE66`. For the centred circle at zero phasing, this mirrors Pan
about 0.5 and leaves Tilt unchanged. DVC-MOVE-EXACT subsequently recovered the raw
composition as `step = cycle × ID2`, with half-up integer frame selection per first-seen
selection rank. Nonzero ID2 is therefore exact rather than fail-closed.

### Pre-V5b Syndocal mismatch and its closure

Before DVC-V5b, Syndocal Move supported only `Line` and `Smooth`. Smooth is
centripetal Catmull-Rom, sampled 32 times per segment and then approximately
reparameterized by arc length. At
the first saved arc midpoint:

| Evaluator | X/Y |
|---|---|
| Daslight analytical circle | `(0.3232233047, 0.6767766953)` |
| Syndocal closed Smooth | `(0.34375, 0.65625)` |
| Absolute delta | `0.0205266953` per axis; Euclidean `0.0290291309`; about `1,345` DMX16 counts per axis |

Line is the inscribed chord and is also non-equivalent. DVC-V5b adds a distinct
`Circle` mode and locks the recovered midpoint, equal-time boundaries, signed-radius
averaging, inflection reflection-before-clamp, two-point full turn, and degenerate
linear fallback in focused Rust and shared-preview regressions. The normalized coordinate
domain itself is compatible with Absolute mode: Syndocal writes
`round(normalized × 65535)` to Pan/Tilt. Physical-degree calibration is outside the
effect body and was not statically recovered.

The former second blocker was targeting. `dvc_rack_targets` retains XML order and
raw `BEAMID`, while literal `IDSELECTION` values establish first-seen zero-based
selection indices. The saved `1..6` therefore becomes internal selection index
`0..5`, still in order. DVC-V5b carries those six identities in the request and
compiles compatible fixture/beam Pan/Tilt bindings once at command/rebuild time.
Repeated source selection IDs retain first-seen rank and intentionally share phase.

### Approved Move protocol-change disposition — implemented in DVC-V5b

The explicit 2026-08-10 approval was implemented with this additive shape:

1. `MoveInterpolation::Circle` selects the wrapped equal-time analytical evaluator.
   Circle requires `closed=true`, accepts the recovered `2..=255` POINTS domain, and
   does not reuse Smooth's sampled arc-length table.
2. `MoveEffectRequest::beam_targets` is a serde-defaulted, empty-omitted ordered Vec of
   `{fixture_id, beam_index, selection_index}`. Empty requests preserve the legacy
   fixture-only serialized shape; focused old-JSON round-trip coverage pins identical
   bytes in both directions.
3. Runtime rebuild resolves each explicit beam to its beam-local Pan/Tilt pair and
   stores fixed attribute names, selection-ranked phase offset, and symmetry role.
   Non-required project/cue restore treats the authored list atomically: if any explicit
   beam cannot bind, the entire source body compiles dormant with zero bindings and can
   rebind after a compatible profile change. Strict Add/Update still rejects the
   unresolved body. The 44 Hz path performs no fixture/beam identity search or allocation.
4. Circle fan-out reads the compiled selection rank as
   `time - rank×step`. Symmetry marks the second half of selection order for recovered
   reverse traversal from half-cycle; legacy Line/Smooth symmetry retains its Pan-mirror
   behavior. Equal selection indices share one phase rank.
5. The importer preserves scene-level `ATTRIBUTEVALUE_MODE` as Absolute or Relative;
   the committed specimen is proven Absolute. DVC-V5b initially mapped only ID2=0;
   DVC-MOVE-EXACT supersedes that temporary boundary for the complete `0..1` domain.

Scene Settings now exposes Circle beside Line/Smooth, forces its recovered closed-path
contract, applies the 255-point limit locally, shows preserved imported beam-target
count, and describes Circle symmetry as reverse traversal. The shared graphical
preview uses the same analytical rules, including degenerate equal-time segments.
Japanese localization covers the new symmetry description; the existing localized
Circle, Closed, and imported-beam labels are reused.

### DVC-MOVE-EXACT supersession

DVC-V5b established the additive beam-target representation and analytical Circle
geometry. DVC-MOVE-EXACT retains those contracts and adds distinct serialized/runtime
variants for all factory IDs 221–225. It recovers the 40 ms integer-frame wrapper,
Curve's uniform Catmull-Rom 16-slice arc-length traversal, Line's cycle-minus-one rule,
Polygon's equal-time edges, Points' held vertices, nonzero raw Phasing, and the exact
two-wing Symmetry bases. Full evidence is in `qa/DVC_MOVE_EXACT_PARITY.md`.

The five compatibility variants are deliberately separate from Syndocal's Enhanced
`Line / Smooth / Circle`. Imported shows retain Daslight timing and quirks; new
authoring keeps continuous high-resolution evaluators, and switching the interpolation
control explicitly migrates an imported body to Enhanced behavior.

### Runtime compilation and 44 Hz benchmark disposition

Command/rebuild time compiles Circle into either the two-point full circle or a fixed
Vec of signed arc, inflection, and degenerate-line segment records. It also resolves
raw beam slots to fixed Pan/Tilt attribute pairs, normalizes first-seen selection
ranks, and stores phase/reverse/mirror flags plus attribute lookup tables. Initial
restore and later rebuild are atomic: any unresolved beam retains a zero-binding
dormant body; partially broken rebuilds clear every old binding rather than emitting
through stale attributes.

At 44 Hz, the evaluator reads those compiled records and target fields. It performs
no allocation, fixture/beam scan, or geometry/identity matching. Exact Circle sampling
uses one analytical `sin_cos` for the selected arc and caches the resulting pair so
Pan and Tilt at the same tick do not evaluate the path twice.

The owning release gates passed on 2026-08-10:

| Gate | Measured | Limit |
|---|---:|---:|
| Circle beam venue, 200 fixtures × 64 effects × 10 frames | `23.5278 ms` total | `< 5 s` |
| Mixed 64-effect × 200-fixture steady tick | p95 `4.308 ms`; p99 `5.179 ms`; max `5.974 ms` | p95 `<=5 ms`; p99 `<=8 ms`; max `<=12 ms` |
| Same mixed stack, outgoing+incoming cue transition | p95 `10.007 ms`; p99 `10.498 ms`; max `10.905 ms` | p95 `<=12 ms`; p99 `<=16 ms`; max `<=20 ms` |

## Importer regression contract

The repo-portable golden test consumes the committed specimen and pins:

- exact ID36 NB=7 parameter types/defaults, eight palette entries, Rectangle values,
  and empty BEAMS/no SELECTIONS;
- ID36 is reported once as a converted `source no-op preserved`, is absent from the
  skipped report, and creates no cue-owned runtime effect target;
- exact ID221 NB=3 parameter types/defaults and four saved points;
- raw fixture UUID plus XML `BEAMID=1..6` / `IDSELECTION=1..6` order;
- importer target parsing as raw beam IDs `1..6`, first-seen selection indices
  `0..5`, one deduplicated fixture, and six ordered steps;
- exactly one converted Circle request with the four POINTS and all path parameters;
- imported fixture/beam targets equal the raw XML order, with exact `BEAMID=1..6` and
  first-seen selection ranks `0..5`;
- no ID221 skipped or approximate entry for the committed body;
- a synthetic repeated/nonsequential selection order remains byte-semantic through
  target parsing and normalized first-seen ranks; and
- nonzero ID2 imports exactly, IDs 222–225 route to distinct evaluators, and factory-outside
  ID226 retains generic fail-closed coverage.

The saved ID36 outcome is exact because Daslight's recovered enumeration chooses the
external SELECTIONS container when present and otherwise the owned BEAMS container.
This body has neither; Rectangle overlap is only a geometric filter over an existing
target list and cannot create membership. A converted no-op is therefore more faithful
than either a synthetic target or a semantic rejection.

The complete existing-521 assertion disposition is:

| Test contract | Old assertion | DVC-V5a assertion | Reason |
|---|---|---|---|
| synthetic DVC-3b import summary and recipe | six converted / zero skipped; two Mappings; four beam targets in selection order; Rainbow Vertical symmetry and Rotation 171 | all values retained; Rainbow also pins additive `grayscale=false`; an additional equality pins Rectangle `(-10,-10,160,70,17.5)` and Patch rows `(1,0,0,0)`, `(1,1,30,0)`, `(2,0,100,50)`, `(2,1,130,50)` | Existing recipe/count behavior is unchanged; the fixture now supplies the source data that the corrected importer must preserve. |
| focused transform import | Horizontal symmetry imports for 521 | same Horizontal symmetry result, additive `grayscale=false`, plus placement `(0,0,100,100,0)` and target `(fixture 1, beam 0, 10,20)` | The prior XML omitted a Rectangle and could only exercise recipe fields; the approved exact path requires and asserts the placement contract. |
| local homecoming recipe count | one Rainbow recipe | unchanged | Placement preservation does not change generator count. |
| Shinkan golden recipe classification | `Rainbow => 521` | unchanged | The protocol addition does not reclassify the generator. |
| repo-portable saved-specimen golden | no 521 source/import assertion | adds raw Rectangle `(2630,-140,140,50,0)`, raw BEAMID `0..3` / IDSELECTION `1..4`, imported recipe defaults including `grayscale=false`, retained beam/selection/Dimmer order, and Patch points `(2640,-130)`, `(2670,-130)`, `(2700,-130)`, `(2730,-130)` | This is additive real-source coverage for the window-preservation fix, not a replacement of an old expected value. |

No prior 521 expected value is deleted, relaxed, or changed to a different number or
meaning. The old stage-normalized centre was not serialized or asserted; DVC-V5a adds
the missing raw Patch-window/coordinate assertions, while focused engine coverage pins
the resulting raster cells and signed integer truncation boundaries.

The DVC-V5b assertion disposition is:

| Test contract | Old assertion | DVC-V5b assertion | Reason |
|---|---|---|---|
| Existing protocol Move preset round-trip | Smooth, fixture/group targets, no beam field | unchanged; `beam_targets=[]` remains omitted | Preserve legacy assertion shape and serialized bytes. |
| New protocol Circle round-trip | no coverage | Circle plus three deliberately reordered beam targets round-trip exactly; legacy JSON reserializes byte-for-byte without a beam field | Additive protocol coverage without replacing the Smooth contract. |
| Saved-specimen ID221 golden | dedicated skipped error; no Move body | one exact Circle body; four raw points and six beam targets equal XML order; no skipped/approximate ID221 entry | The approved representation closes both prior blockers. |
| Unknown `(4,4)` generator | generic family fail-closed behavior | ID222 was the V5b sentinel; DVC-MOVE-EXACT moves the sentinel to factory-outside ID226 | All factory IDs 221–225 are now proven routes. |
| Move Circle nonzero raw Phasing | no focused assertion | V5b fail-closed assertion is replaced by exact ID2=0.5 import and runtime frame-fanout coverage | The raw-to-runtime integer-frame composition is recovered. |
| Project Move validation | fixture-only bodies require a paired Pan/Tilt fixture | unchanged for legacy bodies; explicit imported beam bodies persist atomically dormant when any axis is unavailable, while outside-fixture, duplicate-beam, and group-combination cases reject | Preserve exact source identity without inventing, partially widening, or retaining stale outputs. |
| Default named Circle authoring path | Circle-shaped points used Smooth interpolation | selects analytical Circle and a closed path | The named editor/default recipe now matches its path semantics. |
| Owning Move release regression | 200 fixtures × 64 fixture-only Move effects × 10 frames, `<5 s` | same scale/limit, now analytical Circle with an explicit beam per fixture | Measure the newly owned path/fan-out under the existing worst-case gate instead of leaving it on the legacy path. |
| Mixed 64×200 release regression | Move member used the legacy request shape; established steady/transition percentile limits | Move member uses Circle plus explicit ordered beam targets; all limits unchanged | Exercise Circle inside the production mixed stack without renegotiating the budget. |

No existing numeric expected value was relaxed. Enhanced Line/Smooth validators retain
their `2..=256` domain and legacy target/symmetry behavior; Enhanced Circle and all five
Daslight variants use `2..=255`. Daslight Line alone requires exactly two points;
Circle modes require a closed path.

## Deliberate boundaries

- Protocol changes are additive only. Default-off Grayscale, absent placement, and an
  empty Move beam-target Vec keep the legacy `.sdc v1` serialized shape.
- No Rectangle overlap is promoted into target membership. The empty committed ID36
  rack stays an exact converted no-op.
- Family-5 generators other than the confirmed `(5,3,36)` contract remain unknown and
  fail-closed. ID36 bodies using external `SELECTIONS` also remain fail-closed until
  that source-target mode is proven.
- DVC-V5a corrects MAPPINGS Rainbow ID521 only. It does not broaden the placement claim
  to MAPPINGS Perlin ID530 or another generator family.
- MOVE IDs 221–225 are exact for the recovered evaluator/full PARAM domain. The older
  committed Mega Bar specimen has RGBA/strobe/dimmer channels but no Pan/Tilt, so its
  ID221 request persists with all six beam identities and compiles dormant instead of
  fabricating a movement binding. The newer generic moving-head specimen proves X=Pan,
  Y=Tilt on a bindable profile.
- The fixture-level Scene default simulator has no beam-local value model, so explicit
  Move beam bodies remain unchanged/dormant there rather than collapsing several beam
  identities onto one fixture value. The editor's path preview is still analytically
  exact and the beam count remains visible.
- The Rectangle placement body is importer provenance/runtime input, not a new Scene
  Settings placement authoring workflow. Only the newly representable Rainbow
  Grayscale parameter is minimally exposed, reusing its existing localized label.
- Factory-outside `(4,4)` generator IDs remain fail-closed; all named 221–225 IDs are routed.
- No claim is made about physical fixture optics, mechanical Pan/Tilt calibration,
  controller latency, or visual output.
