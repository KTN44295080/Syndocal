# Daslight COLOR / VALUE Sweep source-parity evidence

## Scope and source

- Audited binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- File version: `25.0905.165.111`
- Size: `9,778,688` bytes
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Audit date: 2026-08-09; implementation closure: 2026-08-10
- Implemented scope: the shared `CSweepEffect` evaluator used by COLOR FX ID134 and
  VALUE FX ID625, including the exact common Grayscale and Transform post-processes
- Product golden: `qa/specimens/ValueCatalog-Sweep-Plasma.dvc` contains a real saved,
  targeted VALUE ID625 body and is imported by a repo-portable regression
- Deliberately excluded from this tranche: the separate 2D MAPPINGS ID528 and COLOR
  MAPPINGS ID44 placement/rotation contracts. They share the class name and evaluator
  entry, but their 2D raster geometry is not implied by the proven ordered-beam path.

## Recovered class and parameter contract

The Microsoft RTTI descriptor `.?AVCSweepEffect@@` resolves to the class vtable at
`0x140696440`. Vtable entry 28 resolves to the raster evaluator at `0x1403665A0`.
The shared colour/value constructor at `0x140350BE0` creates the common palette and
transform properties before the Sweep constructor at `0x140355D30` installs its vtable
and creates one class-specific boolean property:

| Evidence | Meaning |
|---|---|
| `0x1403511A4..0x1403511FE` | palette property `TYPE=4`, ID 1, stored at object `+0xF0` |
| `0x140351365..0x1403513AB` | `Transform` property `TYPE=6`, ID 3, default 0, stored at object `+0x100` |
| `0x140355D7F`: string address `0x140697610` | ASCII text `Direction Change` |
| `0x140355D99`: stack value `0x0A` | property ID 10 |
| `0x140355DAC` + constructor `0x14034BC50` | `TYPE=2`, default value false |
| `0x140355DB7`: store at object `+0x150` | runtime property handle |
| `0x140355DCF`: zero object `+0x158` | initial latched direction |

The common effect serializer at `0x140347160` creates `PARAMS` at `0x14034724F`, walks
the registered property list at `0x1403472BB..0x140347324`, and calls the property
serializer `0x14034E440` for every entry. That serializer writes `PARAM@TYPE` from
property `+0x08`, `PARAM@ID` from `+0xC8`, scalar `VAL`, or the `COLORS` body for type 4.

The two exact serialized bodies are therefore:

| Family / ID | Exact ordered schema |
|---|---|
| VALUE 625 | `TYPE4/ID1 palette`, `TYPE6/ID3 Transform`, `TYPE2/ID10 Direction Change` |
| COLOR 134 | `TYPE4/ID1 palette`, `TYPE2/ID2 Grayscale`, `TYPE6/ID3 Transform`, `TYPE2/ID10 Direction Change` |

The real VALUE golden fixes the first row byte-for-byte with Transform 0, Direction
Change 1, and a non-empty target. The COLOR row is additionally fixed by the recovered
factory/property list and the same common serializer; a real COLOR134 saved specimen is
still useful as an independent product capture, but is no longer an implementation
blocker.

## Recovered ordered-beam evaluation

For `N` palette colours, normalized cycle phase `t`, and output width `W`:

1. `scaled = fract(t) * N`
2. `transition = floor(scaled) mod N`
3. `progress = fract(scaled)`
4. `boundary = trunc(progress * W)`
5. Fill the raster with palette `(transition + 1) mod N`.
6. Overlay palette `transition` from `x = boundary` through the right edge.
7. If `Direction Change` is enabled, latch `transition mod 2` at each palette boundary
   and rotate alternate transitions by 180 degrees.

The key ranges are `0x140366682..0x1403667CF` for transition/progress and pixel boundary,
`0x1403667D3..0x14036682D` for the two hard fills, and
`0x140366833..0x1403668D4` for Direction Change. The result is a hard palette boundary,
not interpolated colour and not alpha blending.

The common Transform post-process at `0x140358DD0` is selected by
`0x140358FF9..0x14035902E`. Transform 1 scales the source to `floor(W/2)` with Qt nearest
sampling and copies the half-width image forward and reversed. For odd `W`, both
destination rectangles have width `floor(W/2)`, so the final pixel remains transparent
black; `W=1` is entirely black. This is the same discrete fold already locked for the
exact Burst and Knight Rider evaluators. COLOR Grayscale is applied after raster
composition with Qt's integer `qGray=(11R+16G+5B)/32`; VALUE has no serialized
Grayscale property.

## Syndocal mapping

- Protocol: `ColorEffectSpatialRecipe::Sweep` now carries additive, default-false,
  false-omitted `daslight_exact`, `grayscale`, and `vertical_symmetry` fields plus
  `direction_change`.
  Legacy `.sdc` bodies keep their previous byte shape and deserialize identically.
- Runtime: imported exact bodies use the shared signed 40 ms frame grid, `F=min(R,750)`
  generated frames, the recovered mixed-precision temporal interpolation, ordered
  selected beam/segment index, integer boundary, exact current/next palette selection,
  alternate 180-degree direction, Qt-nearest Transform fold, and post-composition qGray.
  Native Enhanced Sweep keeps continuous-time evaluation instead of inheriting the
  compatibility frame grid.
- VALUE importer: ID625 accepts both legal Transform values, maps ID3 to
  `vertical_symmetry`, maps ID10 to `direction_change`, keeps Grayscale false, validates
  the exact TYPE/ID schema and signed DURATION before no-op handling, sets
  `daslight_exact=true`, and preserves fixture/beam/selection order.
- COLOR importer: ID134 validates the exact four-property schema and binary domains,
  then maps Grayscale, Transform, and Direction Change one-for-one, validates the signed
  DURATION grid, and sets `daslight_exact=true`.
- Scene Settings: both VALUE Sweep and COLOR Sweep expose an Enhanced / Daslight exact
  evaluator switch, Transform, and Direction Change; COLOR additionally exposes
  Grayscale. New authored recipes default to Enhanced with all three switches off.
  Imported exact bodies can be explicitly upgraded to Enhanced, and imported beam
  targets remain untouched when the recipe is edited.

## Assertion delta and verification boundary

| Contract | Old assertion | New assertion | Reason |
|---|---|---|---|
| VALUE 625 Transform | `Transform=1` rejected after schema validation | both 0 and 1 import exact | shared Qt-nearest fold is now proven and implemented |
| COLOR 134 route | factory-known but generic `Skipped` | strict exact converter route | the evaluator and all common/class fields are represented |
| Sweep protocol | only `direction_change` | additive false-omitted exact/Grayscale/Transform fields | exact COLOR/time/Transform state without legacy JSON drift |
| Scene Settings | VALUE Direction Change only; no COLOR Sweep | full verified controls in both editors | every serialized parameter must remain user-visible |
| Time path | continuous native period for every Sweep | imported exact 40 ms frame grid; native Enhanced remains continuous | preserve Daslight replay without globalizing its coarse time unit |

Focused Rust regressions fix the hard boundary, wrap, alternate/fixed direction, exact
generated-frame sampling/cap, Transform maps for even/odd/single-pixel widths,
post-composition qGray, protocol legacy
shape, strict TYPE/extra-PARAM/binary-domain rejection, VALUE targeted/no-op behavior,
and COLOR134 conversion. Frontend static and real Solid viewport checks fix the new
selectors and controls without reducing typography or hit targets. The 2D mapping-family
geometry remains outside this claim and stays fail-closed until its placement semantics
are independently implemented.

The release 44 Hz stress gate for 200 fixtures x 64 exact Sweep effects measured
p95 4.023 ms, p99 5.080 ms, and max 6.112 ms, passing the 5/8/12 ms thresholds. The
workspace test suite, production frontend build, 3066/3066 localization check, and all
five Scene Settings viewports passed. The native completion gate then rebuilt this exact
checkout with `tauri build --no-bundle`, launched its release executable, and verified
exactly one responsive `Syndocal` window whose accessibility state reported
`準備完了`.
