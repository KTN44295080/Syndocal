# Daslight Burst exact evaluator parity (COLOR 121 / VALUE 622)

## Scope and binary identity

- Audited binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- ProductVersion: `5.0.6.2`
- FileVersion: `25.0905.165.111`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Exact routes: COLOR FX ID121 and VALUE FX ID622, both `CBurstEffect`
- Evaluator: vtable slot 28, `0x140362B70`

All virtual addresses below apply only to that binary. The implementation is an additive
compatibility mode. Syndocal-authored Burst remains the higher-resolution Enhanced recipe.

## Corrected palette-wrap proof

The earlier DVC-V6 disposition was wrong: object byte `+0x12c` is not uninitialized.
The factory wrapper `0x14036CA20` allocates `0x1a0` bytes and calls `CBurstEffect`
constructor `0x1403503C0`, which calls the shared family constructor `0x140350BE0`.
After every family-specific branch rejoins, that shared constructor performs:

```text
0x140351550  movl $0xbf800000, 0x128(%rdi)
0x14035155a  movb $0x1,        0x12c(%rdi)
0x140351561  movl $0x0,        0x114(%rdi)
```

The write is unconditional and precedes the constructor return. Therefore palette wrap is
deterministically `true` for Burst. `+0x12d`, initialized separately at `0x140350CFC`, is
not the same state.

## Cyclic palette cache

The palette rebuild at `0x140361EA0` reads Gradient from `+0x128`, wrap from `+0x12c`,
and calls `0x140358240`. For `N` authored palette colors and wrap=true:

```text
segments = N
P        = floor(65536 / N)
H        = trunc((1 - Gradient) * P)
cache length = N * P

first = palette[i]
next  = palette[(i + 1) mod N]
amount(b) = 0                         when b < H
          = min((b - H) / (P - H), 1) otherwise
cache[i * P + b] = RGB(first, next, amount(b))
```

Integer division intentionally leaves at most `N-1` unused 16-bit codes. The last segment
wraps to the first color. The compatibility path preserves the recovered
`(cache_len - 1)` integer scaling when Burst builds its RGBA64 gradient stops.

## Radial evaluator

`0x140362C01..0x140362C53` constructs a `QRadialGradient` centered at
`(raster_width / 2, raster_height / 2)` and passes raw integer ID10 Color Width as the
radius. `0x140362C5A..0x140362C78` computes generated-frame time as
`f32(frame) / f32(F)`. The time-dependent seam stops are assembled through
`0x140362DF6`; the image is filled at `0x140362DFC..0x140362E34`, then the common
post-process runs at `0x140362E49..0x140362E53`.

For the one-row ordered beam strip, the conceptual coordinate away from the seam is:

```text
pixel_x = source_index + 0.5
distance = abs(pixel_x - raster_width / 2)
radial = clamp(distance / raw_Color_Width, 0, 1)
palette_coordinate = fract(radial - f32(frame) / f32(F))
```

The exact raster does not sample that expression continuously. Source-buffer allocation
at `0x140362A56..0x140362A6D` passes QImage format 26, which is
`Format_RGBA64`. For each generated frame Daslight creates red-channel stops:

```text
initial16 = trunc((1 - t) * 65535)
initial   = floor(initial16 * (palette_cache_length - 1) / 65535)
stops     = [(0, initial), (t - 1e-5, last), (t, 0), (1, initial)]
```

The negative seam stop at frame zero is omitted and `setColorAt(0, 0)` replaces the
initial stop. Qt then generates its 1024-entry RGBA64 QGradientCache table, including the
separate two-stop frame-zero path and 8-bit interpolation weights for multi-stop frames.
The radial fetch chooses `round(radial * 1023)` and the common Daslight post-process uses
the resulting 16-bit red value as the cyclic palette-cache index. Syndocal precompiles all
`min(R,750) * 1024` indices, so the runtime path is exact without invoking Qt.

The `+0.5` sample position is Qt 5.15.2 raster behavior, not an inference from a preview:
`qt_fetch_radial_gradient_template` initializes both coordinates from `x + 0.5` and
`y + 0.5` in the official Qt source. The same source fixes the table size at 1024 and
rounds lookup with `int(pos * 1023 + 0.5)`. `QGradientCache::generateGradientColorTable`
defines the two-stop and multi-stop RGBA64 table builders, while `setColorAt` replaces an
existing equal-position stop. Relevant primary source:

- `https://github.com/qt/qtbase/blob/v5.15.2/src/gui/painting/qdrawhelper_p.h`
- `https://github.com/qt/qtbase/blob/v5.15.2/src/gui/painting/qpaintengine_raster.cpp`
- `https://github.com/qt/qtbase/blob/v5.15.2/src/gui/painting/qbrush.cpp`

COLOR ID2 Grayscale is applied after the completed RGB raster through the existing exact
8-bit `qGray` post-process. VALUE has no ID2 and therefore imports `grayscale=false`.
Transform=1 uses the already proven Qt-nearest two-half fold; an odd final destination
pixel and a width-one raster remain clear.

## Time grid and runtime boundary

Both exact routes use the recovered VALUE-family source-buffer scheduler:

```text
R = max(1, floor(DURATION / 40))
period_ms = 40 * R
F = min(R, 750)
q = trunc(f32(elapsed_ms) / 40) mod R
u = f32(q) / f32(R)
p = f32(F - 1) * u
lo = floor(f64(p)); hi = min(lo + 1, F - 1)
```

The two generated Burst colors are interpolated with the same recovered mixed-precision
order as exact Knight Rider. Command/rebuild time compiles the Qt-quantized result into a
completed `frame x ordered-beam` color table. Identical palette, gradient, frame-grid,
radius, Transform and qGray inputs share immutable palette, gradient and completed-frame
tables. This is a Syndocal improvement over multiplying identical generated buffers; it
does not change a generated sample. The 44 Hz path performs two completed-color reads and
one temporal interpolation. VALUE uses a dedicated grayscale-component fast path instead
of entering the generic RGB binding/output-map evaluator; a regression compares every
fast-path result against that generic reference.

The release-mode 200-fixture x 64 exact-Burst gate evaluates 1,000 complete ticks after
all compilation. On this host it measured p95 `1.967 ms`, p99 `3.829 ms`, maximum
`4.467 ms`, passing the established `5 / 8 / 12 ms` thresholds and the 22.7 ms DMX tick.

## Protocol, importer, and authoring policy

`ColorEffectSpatialRecipe::Burst` gained serde-defaulted, false-omitted fields:
`daslight_exact`, `grayscale`, and `vertical_symmetry`. Legacy `.sdc` JSON retains its
previous byte shape when all are false.

- COLOR 121 and VALUE 622 validate exact TYPE/ID sets and constructor domains.
- Both import with `daslight_exact=true`, exact 40 ms period quantization, ordered beam
  identity, raw pixel Color Width `10..900`, and Gradient `0..1`.
- ImportReport records evaluator `0x140362B70` and the constructor-proven wrap state.
- New authoring defaults to Enhanced (`0..100%` width and gradient).
- The editor exposes an explicit `Daslight exact` / `Enhanced` switch. Switching to
  exact quantizes the period to 40 ms; switching to Enhanced removes transform/grayscale
  compatibility state rather than leaking Daslight quirks into the native evaluator.

## Assertion delta

| Old assertion | New assertion | Reason |
|---|---|---|
| COLOR 121 / VALUE 622 must be Skipped | both create exact runtime targets | unconditional constructor write proves wrap=true and the evaluator is now represented |
| Burst must not expose exact protocol state | exact flags round-trip; false fields stay omitted | additive compatibility mode preserves legacy JSON |
| local Shinkan Burst is skipped | one exact Burst imports; only Random fill and Sparkle remain skipped | class-specific blocker was removed |
| homecoming has no Burst recipes and 4 skipped FX | three exact Burst recipes and only Perlin skipped | all three saved Burst bodies now use the exact route |
| exact Burst VALUE may use the generic RGB spatial path without a dedicated budget | VALUE exact uses a generic-reference-proven component fast path; 64x200 release is p95 1.967 ms / p99 3.829 ms / max 4.467 ms | the generic path measured p95 11.955 ms and failed the existing 5 ms gate |

## Focused evidence

- exact cache topology, wrap segment, hold/ramp threshold, RGBA64 1024-entry table
  quantization, frame-zero replacement, sub-table seam, pixel-center symmetry, 40 ms
  generated-frame interpolation, qGray order, Transform odd-tail clearing, and Enhanced
  false-path validation have dedicated engine regressions;
- duplicate immutable compile tables are interned, the completed-frame fast path is
  byte-for-byte compared with the generic spatial reference, and the 64x200 release gate
  passes `5 / 8 / 12 ms` at `1.967 / 3.829 / 4.467 ms`;
- VALUE 622 and COLOR 121 cover Transform 0/1, strict TYPE/domain rejection, quantized
  duration, exact flags, ImportReport provenance, and saved-project goldens;
- localization and editor contracts cover the explicit evaluator switch and 40 ms label.
