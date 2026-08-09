# Daslight VALUE Sweep source-parity evidence

## Scope and source

- Audited binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- File version: `25.0905.165.111`
- Size: `9,778,688` bytes
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Audit date: 2026-08-09
- Implemented scope: the one-row ordered-beam raster used by VALUE FX
- Deliberately excluded: guessing the ID625 saved-DVC XML schema or claiming the unverified 2D MAPPINGS Sweep rotation convention

## Recovered class and parameter contract

The Microsoft RTTI descriptor `.?AVCSweepEffect@@` resolves to the class vtable at
`0x140696440`. Vtable entry 28 resolves to the raster evaluator at `0x1403665A0`.
The constructor at `0x140355D30` installs that vtable and creates one class-specific
boolean property:

| Evidence | Meaning |
|---|---|
| `0x140355D7F`: string address `0x140697610` | ASCII text `Direction Change` |
| `0x140355D99`: stack value `0x0A` | property ID 10 |
| `0x140355DAC`: `xor edx, edx` | default value false |
| `0x140355DB7`: store at object `+0x150` | runtime property handle |
| `0x140355DCF`: zero object `+0x158` | initial latched direction |

This proves the native editor/runtime parameter name, ID and default. It does not by
itself prove every wrapper/common `PARAM` that Daslight writes into a `.dvc`; the
importer therefore continues to reject ID625 until a real saved specimen is available.

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
- DVC: generator ID625 remains fail-closed because no actual saved ID625 source was found.

## Verification boundary

The focused Rust test fixes start state, hard-boundary truncation, the first transition,
the alternating-direction transition and fixed-direction mode with a three-colour palette.
The Scene Settings harness performs the real Solid selector/checkbox path at 1920x1080,
1920x1032, 2048x1152, 1366x768 and 1280x720 while retaining zero app/document scroll and
existing typography/hit-target floors. Native WebView2 observation is deferred while the
desktop belongs to another task; no current native-window claim is made.
