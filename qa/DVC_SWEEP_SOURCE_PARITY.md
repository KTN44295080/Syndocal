# Daslight VALUE Sweep source-parity evidence

## Scope and source

- Audited binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- File version: `25.0905.165.111`
- Size: `9,778,688` bytes
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Audit date: 2026-08-09
- Implemented scope: the one-row ordered-beam raster used by VALUE FX and the statically proven ID625 serializer schema
- Deliberately excluded: claiming an unavailable real saved ID625 specimen, Transform=1 raster reflection, or the unverified 2D MAPPINGS Sweep rotation convention

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
The exact statically recovered ID625 body is therefore:

1. `PARAM TYPE=4 ID=1` with the grayscale `COLORS` palette
2. `PARAM TYPE=6 ID=3 VAL=0|1` for `Transform`
3. `PARAM TYPE=2 ID=10 VAL=0|1` for `Direction Change`

This is also cross-checked by the real ID621 VALUE Rainbow specimen, which contains the
same common TYPE4/ID1 and TYPE6/ID3 entries before its class-specific parameters. A real
ID625 file remains desirable as a product-to-product golden, but its default serializer
shape is no longer inferred or guessed.

## Recovered one-row evaluation

For `N` palette colours and normalized cycle phase `t`:

1. `scaled = fract(t) * N`
2. `transition = floor(scaled) mod N`
3. `progress = fract(scaled)`
4. `boundary = trunc(progress * raster_width)`
5. Fill the raster with palette `(transition + 1) mod N`.
6. Overlay palette `transition` from `x = boundary` through the right edge.
7. If `Direction Change` is enabled, the evaluator latches `transition mod 2` at
   each boundary on a one-row raster and rotates alternate transitions by 180 degrees.

The key instruction ranges are `0x140366682..0x1403667CF` for transition/progress and
pixel boundary, `0x1403667D3..0x14036682D` for the two hard fills, and
`0x140366833..0x1403668D4` for Direction Change. The result is a hard palette boundary,
not interpolated colour and not alpha blending.

## Syndocal mapping

- Protocol recipe: `ColorEffectSpatialRecipe::Sweep { direction_change }`
- Runtime: ordered selected beam/segment index, integer pixel boundary, exact current/next
  palette selection, alternating 180-degree direction for one-row VALUE strips
- Scene Settings: `Sweep` is in the VALUE generator selector and exposes the full-size
  `Direction change` checkbox. Imported beam targets remain untouched when recipes change.
- Persistence: the additive protocol body round-trips through `.sdc`.
- DVC: generator ID625 imports the exact TYPE4/ID1, TYPE6/ID3, TYPE2/ID10 schema;
  empty `BEAMS` stays a source no-op and targeted sources preserve fixture/beam/selection
  order. Direction Change 0/1 maps exactly to the Sweep recipe. Transform=1 remains
  fail-closed because the verified one-row Syndocal body does not claim Daslight's generic
  2D raster reflection.

## Verification boundary

The focused Rust tests fix start state, hard-boundary truncation, the first transition,
the alternating-direction transition and fixed-direction mode with a three-colour palette.
The importer regression also fixes the exact three-PARAM serializer schema, no-op and
targeted paths, Direction Change 0/1, wrong TYPE rejection, unexpected PARAM rejection,
and Transform=1 fail-closed behavior.
The Scene Settings harness performs the real Solid selector/checkbox path at 1920x1080,
1920x1032, 2048x1152, 1366x768 and 1280x720 while retaining zero app/document scroll and
existing typography/hit-target floors.
