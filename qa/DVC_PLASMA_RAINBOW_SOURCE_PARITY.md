# DVC Plasma / Rainbow source-parity audit

- Audit date: 2026-08-09
- Reference binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- Product version: `5.0.6.2` (`25.0905.165.111`)
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Method: read-only PE RTTI/vtable tracing and function disassembly. No Daslight project was saved and no GUI operation was used for this audit.

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

## Import and editing contract

- Valid 129/130 DVC records no longer report a pattern/timing approximation.
- Nonzero `Grayscale` and unsupported COLOR FX `Transform` still report `Approximate`; they are not silently discarded.
- Fractional or out-of-range generator values fail validation and are reported as `Skipped`, rather than being clamped into a different show.
- Scene Settings exposes every verified Plasma and COLOR FX Rainbow parameter with the same limits and defaults.
- Existing `.sdc` representation is unchanged; the additive recipe variants remain backward compatible.

## Remaining external boundary

- Formula parity is proven against the named Daslight binary and automated engine samples, not by viewing physical fixtures.
- The available DVC specimens prove COLOR FX as a profile-order strip. Syndocal does not claim arbitrary two-dimensional Plasma layout parity until a DVC specimen supplies authoritative per-beam X/Y layout metadata.
- Nonzero Grayscale/Transform behavior remains explicitly approximate.
- Physical emitter mixing, fixture optics, controller latency, and visual perception remain acceptance tests.
