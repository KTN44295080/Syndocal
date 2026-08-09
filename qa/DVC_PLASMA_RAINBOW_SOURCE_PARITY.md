# DVC Plasma / Rainbow source-parity audit

- Audit date: 2026-08-09
- Reference binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- Product version: `5.0.6.2` (`25.0905.165.111`)
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Method: read-only PE RTTI/vtable tracing and function disassembly, plus a maximized Daslight UI check of the available Transform options and Grayscale preview. No Daslight project was saved.

## Located implementations

| Generator | RTTI / evaluator evidence |
|---|---|
| COLOR FX 129 Plasma | `CPlasmaEffect` type descriptor `0x1408ef7e0`; vtable `0x140695f18`; constructor `0x1403542a0`; table builder `0x140358040`; evaluator `0x1403654f0` |
| COLOR FX 130 Rainbow | `CRainbowEffect` type descriptor `0x1408ef740`; vtable `0x140695af8`; constructor `0x140354a80`; evaluator `0x140365a00` |

Addresses are image virtual addresses for the exact binary hash above, not a compatibility promise for another Daslight build.

## Plasma

The constructor exposes these exact integer domains:

| DVC parameter | Daslight label | Default | Domain |
|---|---|---:|---:|
| 10 | Size X | 1 | 0..20 |
| 11 | Param X | 2 | 0..20 |
| 12 | Size Y | 1 | 0..20 |
| 13 | Param Y | 2 | 0..20 |
| 14 | Speed X | -1 | -5..5 |
| 15 | Param SX | 2 | -5..5 |
| 16 | Speed Y | 1 | -5..5 |
| 17 | Param SY | -1 | -5..5 |

Daslight first builds a byte-addressed sine table:

```text
table[i] = low_u8(trunc(sin(i * 0.04908734375) * 30))
phase    = low_u8(trunc(frame_index / frame_count * 128))
```

The recovered double literal is approximately `2pi / 128`; Syndocal retains the literal instead of replacing it with a recomputed constant because the following integer truncation makes boundary rounding observable.

For pixel coordinate `(x, y)`, all index arithmetic and the final sum use low-byte wrapping:

```text
a = table[phase * ParamSY + y * ParamY]
b = table[phase * SpeedX + y * SizeY]
c = table[phase * ParamSX + x * ParamX]
d = table[phase * SpeedY + x * SizeX]
palette_byte = a + b + c + d
```

Syndocal now executes that byte formula directly. Imported COLOR FX targets are the verified profile/channel-order beam strip, so `x=selection_index` and `y=0`. This is deliberate: inventing a mirrored Y coordinate produced a different generator. The resulting byte samples the authored palette continuously.

## Rainbow

The Rainbow constructor exposes:

| DVC parameter | Daslight label | Default | Domain |
|---|---|---:|---:|
| 10 | Color Width | 0 | 0..1 |
| 11 | Angle | 0 | integer 0..360 |
| 12 | Gradient | 1 | 0..1; displayed as 0..100% |

For `N` palette colors, Daslight derives the gradient length from the palette itself:

```text
gradient_length = 1 + (N - 1) * ColorWidth
palette_position = wrap(projected_coordinate / gradient_length - time_phase)
```

This corrects three previous Syndocal differences:

1. width is not a fixed `1 + width` or `1 + width * 7`; it depends on the actual palette count;
2. the spatial projection is divided by the gradient length rather than multiplied by it;
3. time is subtracted, so the sweep direction matches Daslight.

COLOR FX 130 uses a profile-order one-dimensional strip. Therefore `projected_coordinate = strip_x * cos(Angle)` and Angle 90 degrees is uniform over that strip. MAPPINGS 521 keeps its authored normalized stage X/Z coordinates, vertical-symmetry transform, and Rotation, then feeds the same palette-length rule.

## Grayscale and Transform

The shared COLOR FX base constructor is at `0x140350be0` and its image post-process is at `0x140358dd0`. Maximized UI inspection confirms that Plasma and COLOR FX Rainbow expose `None` / `Vertical symmetry`; MAPPINGS Rainbow additionally exposes `Horizontal symmetry`.

`Grayscale` runs after the RGB palette is rendered into Daslight's 8-bit QImage. Syndocal therefore uses Qt's integer `qGray` conversion rather than a floating-point luma approximation:

```text
gray8 = (red8 * 11 + green8 * 16 + blue8 * 5) / 32
output16 = gray8 * 257
```

Transform `1` draws the full source into the first half and a horizontally mirrored full source into the second half. Transform `2` applies the equivalent operation on the vertical image axis. For normalized sampling this is the tent mapping:

```text
source_axis = 1 - abs(2 * target_axis - 1)
```

Syndocal applies this rule to the profile-order strip for COLOR FX and to X or Z for MAPPINGS. The previous MAPPINGS fold used the opposite tent orientation; the corrected mapping now keeps source start at both outside edges and source end at the center, matching Daslight's draw order.

## Import and editing contract

- Valid 129/130 DVC records, including nonzero `Grayscale` and vertical `Transform`, no longer report a pattern/timing approximation.
- MAPPINGS Rainbow imports `None(0)`, `Vertical symmetry(1)`, and `Horizontal symmetry(2)` exactly. COLOR FX rejects any transform outside its verified `0/1` domain.
- Fractional or out-of-range generator values fail validation and are reported as `Skipped`, rather than being clamped into a different show.
- Scene Settings exposes every verified Plasma, COLOR FX Rainbow, and MAPPINGS Rainbow parameter with the same labels, limits, defaults, and mutually exclusive Transform menu.
- Existing `.sdc` representation is additively extended. Missing Grayscale/Transform fields default to off/None and round-trip tests lock backward compatibility.

## Remaining external boundary

- Formula parity is proven against the named Daslight binary and automated engine samples, not by viewing physical fixtures.
- The available DVC specimens prove COLOR FX as a profile-order strip. Syndocal does not claim arbitrary two-dimensional Plasma layout parity until a DVC specimen supplies authoritative per-beam X/Y layout metadata.
- Physical emitter mixing, fixture optics, controller latency, and visual perception remain acceptance tests.

## Focused regression evidence

- `cargo test -p protocol color_effect_spatial_pattern_roundtrips_and_legacy_defaults_to_none`
- `cargo test -p engine color_spatial_`
- `cargo test -p engine color_fx_vertical_symmetry_uses_daslight_source_image_tent_map`
- `cargo test -p syndocal dvc_color_fx_grayscale_and_all_verified_transforms_import_exactly`
- `pnpm --dir app run check:fx-palettes`
- `pnpm --dir app run check:localization`
