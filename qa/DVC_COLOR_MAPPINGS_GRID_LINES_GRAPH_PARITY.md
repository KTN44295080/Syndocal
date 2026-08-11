# COLOR MAPPINGS Grid / Lines / Graph parity

Date: 2026-08-11

## Evidence and claim boundary

Daslight 5.0.6.2 (`25.0905.165.111`, SHA-256
`325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`)
registers COLOR MAPPINGS as `RACK TYPE=5 / EFFECT TYPE=3`, Lines as ID31
(`CLineEffect`), Graph as ID49 (`CGraphEffect`), and Grid as ID50 (`CGridEffect`).
Graph's factory edge is `0x14036CF60 -> 0x140352530`; its evaluator is
`0x1403638A0`, and its palette-tile rebuild is `0x14035F360` through vtable
`0x140696FA8 + 0xB8`. The recovered class bodies fix each raster at 100x100. There is
no native saved Lines/Grid/Graph specimen in the repository.
Accordingly this tranche claims binary/static grammar parity plus synthetic protocol,
importer, engine, and performance regression coverage; it does not claim native saved-body
golden or live Daslight visual acceptance.

Both routes preserve the COLOR MAPPINGS base exactly: palette `T4/1`, Grayscale `T2/2`,
Transform `T6/3` in `0/1/2`, integer Rotation `T0/4` in `0..360`, one positive Rectangle,
owned COLOR beam targets in IDSELECTION order, Patch-canvas coordinates, Override merge,
and fail-closed external SELECTIONS. Placement sampling applies inverse Transform and then
inverse Rotation before `pixel=floor(clamp01(coordinate)*100).min(99)`. The output palette
color is finally passed through the shared QColor/qGray-compatible post-process when
Grayscale is set.

## Grid ID50

The exact PARAM schema is base plus `T0/10 Size=1..5` and `T0/11 Width=2..20`. The class
palette domain is `2..5`; a one-stop palette and palettes above five fail closed.

For `W=H=100`, Size `S`, and line width `L`:

- `A=floor(floor(W/2)/S)` and `C=floor(floor(W/A)/2)`.
- `(A,C)` is exactly `(50,1),(25,2),(16,3),(12,4),(10,5)` for Size 1..5.
- `D=floor((W-L+1)/2)` and `h=floor(L/2)`.
- For layer `n=0..C-1`, `q=(D*fract(2*phase)+n*A) mod D` and
  `k=1+n%(palette_count-1)`.
- Palette 0 first fills the background. Each layer paints, in order, half-open clipped
  rectangles `(q-h,0,L,H)`, `(W-h-q,0,L,H)`, `(0,q-h,W,L)`, and
  `(0,H-h-q,W,L)`. Later hits overwrite earlier hits.

Syndocal caches `A/C/D/h`, width, palette, and Grayscale once. Sampling walks at most five
layers and allocates nothing in the tick path.

## Lines ID31

The exact PARAM schema is base plus `T0/10 Size=2..20`. The class palette domain is
`2..255`; a one-stop palette fails closed. There is no Direction parameter.

Let `N` be palette count and `S` line width. `B=floor((W-S)/(N-1))`. If `B>=1`, the
integer `B` is retained and `q=B*fract(2*phase)`. For each `i=0..N-2`, palette `i+1`
paints `x=i*B+q` and then `x=(i+1)*B-q`, each as a width-S, full-height, half-open
rectangle. Iterations and each pair are later-wins; at `q=0`, the higher palette index
wins coincident starts.

If integer `B=0`, the corrected route uses float
`b=(W-S)/(N-1)` and the same equations/order instead of returning a blank raster. The
engine derives the greatest covering index analytically in O(1); no per-tick vector or
raster is built.

## Graph ID49

The exact PARAM schema is base plus `T0/10 Height=10[1..100]`,
`T0/11 Width=10[1..100]`, `T0/12 Pitch=10[0..100]`,
`T0/13 Frequency=2[0..10]`, `T1/14 Amplitude=1[0..2]`, and
`T1/15 Offset=0[-1..1]`. The constructor first applies the common palette setters and
then reapplies lower bound 2; the same common constructor sets upper bound 10. Therefore
the exact class palette domain is `2..10`; palettes of 1 or 11 fail closed.

Let `P=max(DURATION,10)`, `t` be the shared continuous runtime phase (including authored
Phase/rate/clock sync), and `d=max(Pitch,1)`. Anchor origins are
`x_j=j*d < 100`; foreground palette index is `1+j%(N-1)`. For every anchor:

- `theta=2*pi*Frequency*(t + 40*x_j/P)` and `wave=0.5*sin(theta)`.
- `y_f=(Amplitude*wave - Offset + 0.5)*100 - trunc(Height/2)` and
  `y=trunc_toward_zero(y_f)`.
- The tile covers integer pixels `[x_j,x_j+Width) x [y,y+Height)`, clipped to the
  100x100 raster.
- With `half=trunc(Height/2)`, source rows use `a=row/half` and
  `weight=a` for `a<=1`, otherwise `weight=2-a`. The source Height=1 division-by-zero
  leaves its only row invisible; the Corrected route makes that legal minimum one opaque
  row instead.

Palette 0 is the opaque background. Each tile semantically applies the selected palette
color with alpha multiplied by `weight` over palette 0; Syndocal expresses that through
the existing shared RGB interpolation/quantization path. Anchors paint left-to-right, so
the greatest covering anchor wins its entire row, including a zero-weight edge which
restores palette 0. The engine caches the validated scalar constants and palette once,
finds the greatest covering anchor analytically, and allocates nothing in the tick path.

## Timing, no-op, and gates

The importer accepts a positive signed integer authored DURATION and stores
`period_ms=max(DURATION,10)`. Runtime phase is continuous and the report records
`implementation=SyndocalCorrected`; the source 40ms work-image quantum is not reintroduced.
An empty BEAMS container becomes a source no-op only after full PARAM, palette, Rectangle,
duration, BEAMS, and external-SELECTIONS validation. For ID31/49/50 only, any owned beam lacking
a verified color segment is an ID-scoped error rather than an approximation or silent omit.

Focused regression coverage includes protocol round-trip/default omission; strict importer
positives and negative mutations; target order, placement and empty-target behavior; Grid
constant/anchor/background/overlap cases; Lines integer-B and float-fallback branches;
close continuous phases; corrected placement; qGray; and a release-coupled 32 Grid + 32
Lines, 200-target, 100x100, 44Hz, 1000-release-sample gate plus a dedicated 64 Graph x
200-target gate at palette 10, Height/Width/Pitch 100, Frequency 10, Amplitude 2, Offset 1,
and 100x100. A sibling 64x200 Graph row keeps every other maximum but sets legal Pitch 0,
which compiles to step 1 and exercises the largest anchor set. All rows use the immutable
5/8/12ms limits. Debug runs intentionally use the existing 20-sample branch; release
measurement is owned by the release gate runner.

Catalog status after this tranche is 60/68 routed IDs and 8 remaining: seven COLOR MAPPINGS
dedicated classes plus CURVE Custom ID13.
