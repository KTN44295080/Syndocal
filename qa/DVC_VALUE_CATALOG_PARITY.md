# Daslight VALUE FX catalog source-parity evidence

## Scope and source

- Audited binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- File version: `25.0905.165.111`
- Size: `9,778,688` bytes
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Audit dates: 2026-08-09 through 2026-08-10
- Method: read-only PE factory, RTTI, vtable, constructor, serializer, and evaluator disassembly, followed by a Qt 5.15.2 raster-path audit for the ID624 Transform fold. No Daslight file was written and no runtime/UI observation is claimed by DVC-V6.
- Implemented scope: exact VALUE-family factory and serialized schemas for IDs 622-624 and 626-628; exact ID623 Plasma import; DVC-V6's additive Daslight-exact ID624 Knight Rider evaluator and Transform=1 fold; strict schema/domain validation followed by precise fail-closed dispositions for IDs 622, 626, 627, and 628; binary-address and Qt-raster-path proof of the discrete one-row Transform=1 mapping.
- Existing anchors reused: ID621 Rainbow, ID625 Sweep, the shared Plasma byte evaluator, and the shared Transform post-process previously recorded in `qa/DVC_PLASMA_RAINBOW_SOURCE_PARITY.md` and `qa/DVC_SWEEP_SOURCE_PARITY.md`.

All virtual addresses below apply only to the binary identity above.

## Common VALUE-family contract

The shared constructor at `0x140350BE0` selects the VALUE family when object family is
7. Its branch at `0x140350D03..0x140350D37` initializes a default 100 by 1 raster.
Vtable slot 26 at `0x140361CF0`, family-7 branch `0x140361D2F..0x140361D45`, later
forces height to 1 and replaces the width with the caller-supplied runtime width. It
registers:

| TYPE / ID | Name | Default | Domain | Storage | Registration |
|---|---|---:|---:|---:|---|
| 4 / 1 | Color Palette | palette body | registered `COLORS` body | `+0xF0` | `0x1403511A4..0x1403511FE` |
| 6 / 3 | Transform | 0 | 0=None, 1=Vertical symmetry | `+0x100` | `0x140351365..0x1403513AB` |

The Grayscale TYPE2/ID2 registration is guarded to families 2 and 3 at
`0x14035125F..0x1403512B6`; family 7 does not serialize it. Consequently every VALUE
schema below begins `(TYPE4, ID1), (TYPE6, ID3)` and has no ID2.

The common effect serializer at `0x140347160` creates `PARAMS` at `0x14034724F`, walks
the registered property list at `0x1403472BB..0x140347324`, and calls property serializer
`0x14034E440`. That serializer emits `PARAM@TYPE` from property `+0x08`, `PARAM@ID`
from `+0xC8`, scalar `VAL`, or the type-4 `COLORS` body. Constructor registration plus
this exhaustive walk proves the exact serialized schemas; no saved specimen is inferred.

## Recovered VALUE factory

| ID | Class | Factory registration | Wrapper / constructor | RTTI / vtable / entry 28 |
|---:|---|---|---|---|
| 622 | `CBurstEffect` | `0x140367E41..0x140367E60` (`0x26E`, family 7) | `0x14036CA20`; call `0x1403503C0` at `0x14036CA57` | TD `0x1408EF790`; vtable `0x140695D08`; evaluator `0x140362B70` |
| 623 | `CPlasmaEffect` | `0x140367D64..0x140367D83` (`0x26F`, family 7) | `0x14036D270`; call `0x1403542A0` at `0x14036D2A7` | TD `0x1408EF7E0`; vtable `0x140695F18`; evaluator `0x1403654F0` |
| 624 | `CKnightRiderEffect` | `0x140367C84..0x140367CA3` (`0x270`, family 7) | `0x14036D040`; call `0x140352ED0` at `0x14036D077` | TD `0x1408EF8D0`; vtable `0x140696548`; evaluator `0x140363FE0` |
| 625 | `CSweepEffect` | previously proven (`0x271`, family 7) | constructor `0x140355D30` | vtable `0x140696440`; evaluator `0x1403665A0` |
| 626 | `CSparklesEffect` | `0x140367ABB..0x140367ADA` (`0x272`, family 7) | `0x14036D7B0`; call `0x140354ED0` at `0x14036D7E7` | TD `0x1408EF9C8`; vtable `0x140696B78`; evaluator `0x1403660F0` |
| 627 | `CRandomFillEffect` | `0x1403679D8..0x1403679F7` (`0x273`, family 7) | `0x14036D660`; call `0x140354D60` at `0x14036D697` | TD `0x1408EF900`; vtable `0x140696650`; evaluator `0x140365C70` |
| 628 | `CPerlinEffect` | `0x1403678EF..0x14036790E` (`0x274`, family 7) | `0x14036D200`; call `0x140353F50` at `0x14036D237` | TD `0x1408EF950`; vtable `0x140696860`; evaluator `0x140365090` |

This is the same factory dispatch previously used to prove ID621 -> `CRainbowEffect` and
ID625 -> `CSweepEffect`; the six rows above complete IDs 622 through 624 and 626 through
628 without extrapolating from UI names.

## Recovered 40 ms VALUE frame grid

This contract is recovered from the VALUE source-buffer path itself, not inferred from
the separately documented CURVE family. The DURATION loader at
`0x140346D84..0x140346E51` calls signed `QStringRef::toInt`, divides that integer by 40
with unsigned arithmetic after the signed-domain check, and replaces a zero quotient
with one. The importer therefore accepts only a positive, integral signed-32-bit
DURATION for Daslight-exact ID624 and computes:

```text
R = max(1, floor(DURATION / 40))
period_ms = 40 * R
```

The common source-buffer builder is `0x14035AD60`; its evaluator call is
`0x14035B02E..0x14035B03C`. Knight Rider obtains its generated frame count through
vtable `+0xD8`, `0x14035B9A0 -> 0x14035BAA0`, and caps the descriptor-builder input to
`F=min(R,750)`. Syndocal never materializes an unbounded `R * strip_width` replay table.
It bounds the generated source by `F` and compiles the exact runtime state once at
command/rebuild time; the 44 Hz path performs fixed bounded work for one beam without
allocation or search.

The common runtime sampler at `0x14035B430` receives the raw tick produced from the
single-precision elapsed-millisecond counter divided by 40. It reduces that tick modulo
`R`, so `q` is always `0..R-1`; the evaluator's ability to accept an endpoint index does
not imply that runtime playback visits `q=R`. It then performs the following operation
sequence for a generated table of length `T`:

```text
q     = trunc(f32(elapsed_ms) / 40) mod R
u     = f32(q) / f32(R)
p     = f32(T - 1) * u
lo    = floor(f64(p))
frac  = f32(p - f32(lo))
```

The two raw layer images are first resolved to floating QDasColor values at
`0x140357130`; temporal interpolation at `0x1402E1F50` preserves the recovered mixed
precision order `diff=f32(B-A)`, then
`out=f32(f64(A)+f64(diff)*f64(frac))`. The imported period is consequently quantized:
DURATION 1, 9, 10, or 39 gives `R=1, period_ms=40`; 40, 41, or 79 also gives
`R=1, period_ms=40`; 80 gives `R=2, period_ms=80`. Zero, negative, fractional, or
signed-32-bit-overflow DURATION remains fail-closed rather than being clamped.

## ID622 Burst — evaluator recovered, palette-wrap provenance unavailable

`CBurstEffect` constructor `0x1403503C0..0x140350532` adds:

| TYPE / ID | Name | Default | Domain | Storage | Registration |
|---|---|---:|---:|---:|---|
| 0 / 10 | Color Width | 50 | integer 10..900 | `+0x150` | `0x140350432..0x140350480` |
| 1 / 11 | Gradient | 1.0 | 0.0..1.0 | `+0x158` | `0x14035049C..0x1403504E7` |

Exact serialized schema: `(4,1), (6,3), (0,10), (1,11)`.

Entry 28 `0x140362B70..0x140362EA0` constructs a `QRadialGradient` centered on the
raster. ID10 is passed as the raw gradient radius at `0x140362C01..0x140362C53`.
Time-dependent stops at 0, `t-1e-5`, `t`, and 1 are assembled at
`0x140362C5A..0x140362DF6`, the raster is filled at `0x140362DFC..0x140362E34`, and
the common post-process is called at `0x140362E49..0x140362E53`. Away from the narrow
seam, the palette coordinate is a radial cyclic sawtooth equivalent to
`fract(clamp(distance / ColorWidth, 0, 1) - t)`.

ID11 is not dead: the property-change handler `0x14035E8F5..0x14035E90D` reads its
TYPE1 value into effect `+0x128` and invalidates the palette cache; common rebuild
`0x140361EA0` consumes that value through `0x140358240` to select continuous versus
stepped palette interpolation.

DVC-V6 recovered the radial evaluator, but the palette-cache path also reads a wrap byte
at object `+0x12c`. Neither the common VALUE/base constructor chain nor
`CBurstEffect::CBurstEffect` initializes that byte, and it is absent from the exhaustive
serialized-property walk above. The byte changes the palette cache topology. A `.dvc`
therefore supplies neither a value nor enough provenance to derive one; choosing
`wrap=false` or `wrap=true` would be a Syndocal guess rather than a Daslight-exact replay.

Syndocal's existing `Burst { color_width, gradient }` evaluator instead computes one
finite expanding band around a normalized center distance and outputs black outside it.
It also gives Color Width a 0..100 percent domain, whereas Daslight passes the full
integer 10..900 domain as a raw pixel radius. It must remain on its existing native path.
An additive Burst exact branch cannot be exposed while the constructor-state input above
is unknown. Transform=1 is validated as 0/1 before the same deliberate-boundary error,
but the recovered fold cannot repair the missing palette-wrap state.

**Disposition: fail-closed after exact TYPE/ID, Transform 0/1, and domain validation;
blocked specifically on uninitialized and unserialized object `+0x12c`.**

## ID623 Plasma — exact mapping landed

`CPlasmaEffect` constructor `0x1403542A0..0x1403546BD` adds eight TYPE0 integer
properties:

| TYPE / ID | Constructor name | Semantic field | Default | Domain | Storage | Registration |
|---|---|---|---:|---:|---:|---|
| 0 / 10 | Size X | `size_x` | 1 | 0..20 | `+0x758` | `0x14035430A..0x14035434D` |
| 0 / 11 | Param | `param_x` | 2 | 0..20 | `+0x760` | `0x140354365..0x1403543A8` |
| 0 / 12 | Size Y | `size_y` | 1 | 0..20 | `+0x768` | `0x1403543C0..0x140354403` |
| 0 / 13 | Param | `param_y` | 2 | 0..20 | `+0x770` | `0x14035441B..0x14035445E` |
| 0 / 14 | Speed X | `speed_x` | -1 | -5..5 | `+0x778` | `0x140354476..0x1403544BC` |
| 0 / 15 | Param | `param_sx` | 2 | -5..5 | `+0x780` | `0x1403544D4..0x14035451A` |
| 0 / 16 | Speed Y | `speed_y` | 1 | -5..5 | `+0x788` | `0x140354532..0x140354578` |
| 0 / 17 | Param | `param_sy` | -1 | -5..5 | `+0x790` | `0x140354590..0x1403545D6` |

The constructor literally reuses the string `Param` for IDs 11, 13, 15, and 17; the
X/Y/SX/SY names above are positional semantic names proven by the evaluator, not renamed
Daslight constructor strings. Exact schema:
`(4,1), (6,3), (0,10), (0,11), (0,12), (0,13), (0,14), (0,15), (0,16), (0,17)`.

The constructor builds 256 DWORD sine-table entries, consumed through their low bytes,
at `0x140354620..0x140354667`:

```text
table[i] = low_u8(trunc(sin(i * 0.04908734375) * 30))
phase    = low_u8(trunc(frame_index / frame_count * 128))
```

The evaluator/table builder at `0x140358040..0x14035823D`, called by entry 28
`0x1403654F0`, performs low-byte wrapping for every index and the final sum:

```text
value = table[phase * ParamSY + y * ParamY]
      + table[phase * SpeedX + y * SizeY]
      + table[phase * ParamSX + x * ParamX]
      + table[phase * SpeedY + x * SizeX]
```

Phase setup is `0x140358055..0x1403580FC`, the pixel/x loop is
`0x140358130..0x1403581DD`, and y advancement is `0x1403581DF..0x140358217`.
VALUE is a one-row raster, so `y=0`. The existing engine helper executes this same
byte formula and palette lookup. The existing `Plasma` recipe carries all eight fields,
`grayscale=false` exactly represents the absent VALUE ID2, and its
`vertical_symmetry` field exactly represents common Transform 0/1.

**Disposition: proven. ID623 now imports after exact TYPE/ID and integer-domain checks.**

## ID624 Knight Rider — Daslight-exact evaluator landed

`CKnightRiderEffect` constructor `0x140352ED0..0x1403532FA` adds:

| TYPE / ID | Name | Default | Domain | Storage | Registration |
|---|---|---:|---:|---:|---|
| 0 / 10 | Size | 1 | integer 1..100 | `+0x150` | `0x140352F4F..0x140352FA7` |
| 2 / 11 | One Way Only | false | 0/1 | `+0x158` | `0x140352FAE..0x140352FF2` |
| 2 / 12 | Fading | true | 0/1 | `+0x160` | `0x140352FF9..0x14035303D` |
| 2 / 13 | Go Outside | false | 0/1 | `+0x170` | `0x140353044..0x140353088` |
| 0 / 14 | Gradient | 50 | integer 0..100 | `+0x180` | `0x14035308F..0x1403530E4` |

Exact schema: `(4,1), (6,3), (0,10), (2,11), (2,12), (2,13), (0,14)`.

Entry 28 `0x140363FE0..0x1403641C8` renders a descriptor built by
`0x14035C150..0x14035C975` and calls the common post-process at
`0x140364181..0x140364187`. Generated-frame count is obtained through vtable `+0xD8`,
`0x14035B9A0 -> 0x14035BAA0`, and is capped to `F=min(R,750)` before descriptor
construction.

All fields are read at `0x14035C1E3..0x14035C260`. One-way/bounce and
inside/outside select four distinct integer-period paths at `0x14035C2A0..0x14035C5CE`;
Size, Gradient, and Fading form a discrete profile at `0x14035C684..0x14035C768`, and
one-way versus bounce has different palette-lane phase placement at
`0x14035C8D3..0x14035C946`. Each lane advances with the recovered integer modulo rules.
Transform=1 raises the descriptor's effective Size floor to three before applying the
recovered discrete whole-raster fold.

The implementation keeps the recovered `R`, `F`, and descriptor length `T` separate.
For strip width `W`, effective size `S`, and multiplier `M`, the four builder branches
are:

| Motion | Base and multiplier | `T` | Profile length `A` |
|---|---|---:|---:|
| one-way, outside | `B=W+S`; `M=max(floor(F/(2B)),1)` | `B*M` | `(B-1)*M` |
| one-way, inside | `B=W`; `M=max(floor(F/(2B)),1)` | `B*M` | `W*M` |
| bounce, outside | `B=W+S`; `Q=floor(F/(2B))`; decrement odd `Q`; `M=max(Q,1)` | `2*B*M` | `(B-1)*M` |
| bounce, inside | `B=max(W-1+(S is even),1)`; same even-`Q` rule | `2*B*M` | `(W-1+S)*M` |

Let `L=S*M` and `peak=floor((L-1)*(100-Gradient)/100)`. A non-fading profile sets
all `A` entries to one, including entries beyond `L`. A fading profile is zero at
`j>=L`; otherwise it uses `j/peak` through the first peak and
`1-(j-peak-1)/(L-peak-1)` after it, with the recovered duplicated peak and f32 division.
The result is quantized through the Qt 16-bit green component before evaluation.

For `C=palette_count-1`, lanes are emitted for `i=1..C` and select palette ordinal
`(i mod C)+1`, producing `[2,1]` for three stops and `[2,3,1]` for four. One-way phase
uses the integer quotient `floor((T-1)*i/C)` before conversion to f32; bounce uses the
separately recovered half-cycle f32 placement. Each lane remains independent. The
resolver starts with palette stop zero and source-over composites lanes in descriptor
order; a zero 16-bit gain is a no-op. Source color multiplication is f32, the
destination `(1-gain)` term follows the recovered f64 intermediate at `0x1402E12E0`,
and the result returns to f32. Temporal interpolation also remains f32/mixed-precision;
only the final output truncates `component*65535` to u16.

Focused references lock all four motion branches at `W=5`, `S=3`, `R=F=25`, plus a
four-stop overlap where palette values `[0,16000,32000,64000]` resolve frame zero to
`[32000,40000,64000,64000,24000]`. The cap boundary keeps `R=751` while building from
`F=750`; playback still wraps on `R`, then maps into `T` with the common sampler above.

The existing native Knight Rider branch remains unchanged. Two omitted-when-false fields,
`daslight_exact` and `vertical_symmetry`, form the additive compatibility boundary;
legacy/native recipes deserialize to false and retain their previous serialized byte
shape. The VALUE ID624 importer alone sets `daslight_exact=true`, maps PARAM 3 to
`vertical_symmetry`, and quantizes DURATION through the recovered 40 ms frame grid.
Syndocal compiles bounded exact state at command/rebuild time; the 44 Hz path remains a
fixed bounded evaluation per beam.

**Disposition: proven and imported for Transform 0 and 1.**

## ID626 Sparkles — evaluator recovered, qrand provenance unavailable

`CSparklesEffect` constructor `0x140354ED0..0x140355227` adds:

| TYPE / ID | Name | Default | Domain | Storage | Registration |
|---|---|---:|---:|---:|---|
| 0 / 10 | Sparkle Number | 5 | integer 1..10 | `+0x150` | `0x140354F52..0x140354FAA` |
| 1 / 11 | Sparkle LifeSpan | 0.0 | 0.0..0.9 | `+0x158` | `0x140354FB1..0x140355008` |
| 0 / 12 | Sparkle Width | 1 | integer 1..90 | `+0x160` | `0x14035500F..0x140355067` |

TYPE0/ID13 Sparkle Height, default 1, domain 1..90, is registered only for families 3
or 8 at `0x14035506E..0x1403550D4`; it is absent for VALUE family 7. Exact VALUE
schema: `(4,1), (6,3), (0,10), (1,11), (0,12)`.

Entry 28 `0x1403660F0..0x140366236` invokes generator entry 32
`0x14035D170..0x14035D463` once per generated frame, advances retained particles at
`0x14036611E..0x140366202`, and calls the common post-process at
`0x140366213..0x140366219`. Each call appends Sparkle Number new particles at
`0x14035D22F..0x14035D413`; Number is not a bounded simultaneous count. The placement
table consumed at `0x14035D281..0x14035D2FC` is populated from Qt `qrand`, so its
contents are deterministic only when the exact incoming random state and draw history
are known. VALUE renders width-sized rectangles at `0x140372C65..0x140372C86`.
LifeSpan becomes per-frame decrement `(1-LifeSpan)*0.4` at
`0x14035D1BA..0x14035D1E5` and is subtracted at `0x140374526..0x140374530`.

In this Windows Qt 5.15.2 build, `qrand` delegates to the CRT `rand` stream whose state
is per physical thread. `.dvc` serializes neither the initial thread state nor the number
and ordering of earlier draws on that thread. Selecting seed 1, a wall-clock seed, or any
other seed would reproduce one possible realization, not the authored Daslight session.

The existing recipe uses a bounded epoch-seeded SplitMix population, treats lifespan as
a 0..100 percent cutoff with an age fade, and treats Width as a strip-cell radius. Units,
population history, placement, palette selection, and effective width all differ. The
recovered Transform=1 fold cannot restore the missing random provenance.

**Disposition: fail-closed after exact schema and domain validation; blocked on missing
per-thread qrand initial state and prior draw history.**

## ID627 Random fill — evaluator recovered, qrand provenance unavailable

`CRandomFillEffect` constructor `0x140354D60..0x140354EC1` adds:

| TYPE / ID | Name | Default | Domain | Storage | Registration |
|---|---|---:|---:|---:|---|
| 0 / 10 | Point Width | 1 | integer 1..10 | `+0x158` | `0x140354DC4..0x140354E25` |
| 0 / 11 | Point Height | 1 | integer 1..10 | `+0x150` | `0x140354E2C..0x140354E93` |

ID11 is omitted only for family 2, so VALUE family 7 serializes it. Exact schema:
`(4,1), (6,3), (0,10), (0,11)`.

The cell builder `0x140360180..0x1403602BC` makes
`floor(raster_width / PointWidth)` cells. Point Height is read only for families 3 and 8
at `0x1403601BB..0x1403601D4`; in VALUE it is serialized but evaluator-dead and height
is forced to 1. For frame `k`, palette count `N`, and `F=min(frame_count,750)`, entry 28
`0x140365C70..0x1403660E4` computes:

```text
f = max(1, floor(F / N))
q = floor(k / f)
r = k mod f
current = palette[q mod N]
next = palette[(q + 1) mod N]
```

At a transition boundary it fills all cells with current color
(`0x140365E68..0x140365E82`). Otherwise it selects not-yet-chosen cells without
replacement using Qt `qrand()` (`0x140365EE2..0x140365F7D`) and moves those cells to
the next palette color. As for ID626, the Windows Qt 5.15.2 call delegates to a
per-thread CRT stream. `.dvc` carries neither its incoming state nor prior call history,
so the exact cell permutation for the authored session cannot be reconstructed.

The existing recipe has no Point Height field and instead uses deterministic
effect-ID/target hashing, different cell partitioning, black unfilled cells, and
independently hashed palette choices. Implementing the recovered no-replacement loop with
a Syndocal-selected seed would still be an invented realization. The recovered
Transform=1 fold cannot restore the missing random provenance.

**Disposition: fail-closed after exact schema and domain validation; blocked on missing
per-thread qrand initial state and prior draw history.**

## ID628 Perlin — evaluator recovered, palette-wrap provenance unavailable

`CPerlinEffect` constructor `0x140353F50..0x140354297` adds:

| TYPE / ID | Name | Default | Domain | Storage | Registration |
|---|---|---:|---:|---:|---|
| 0 / 10 | Octaves | 4 | integer 2..10 | `+0x150` | `0x140354011..0x140354069` |
| 0 / 11 | Zoom | 75 | integer 1..100 | `+0x158` | `0x140354070..0x1403540C8` |
| 0 / 12 | Direction | 2 | integer 1..100 | `+0x160` | `0x1403540CF..0x140354127` |
| 0 / 13 | Speed | 1 | integer 1..10 | `+0x168` | `0x14035412E..0x140354186` |
| 0 / 14 | Amplitude | 70 | integer 5..100 | `+0x170` | `0x14035418D..0x1403541E5` |

Exact schema: `(4,1), (6,3), (0,10), (0,11), (0,12), (0,13), (0,14)`.

Update method `0x14035FF60..0x140360057` builds a 360-entry sine table. It caches
Octaves, Zoom, Speed, and Amplitude but skips Direction at
`0x14035FFBF..0x140360012`; serialized ID12 is evaluator-dead. Entry 28
`0x140365090..0x1403654EA` uses, for octave `o=0..Octaves-2`:

```text
p = x / Zoom * 2^o
weight(frac) = (1 - cos(pi * frac)) / 2
```

The fixed 32-bit hash and degree-quantized time-rotated sine signal are
`0x14035A950..0x14035AA5F`; cosine interpolation is
`0x140365280..0x140365348`; octave attenuation is `0.7^o` at
`0x140365320..0x14036536E`; output is
`clamp(trunc(sum*128+128),0,255)` at `0x140365395..0x1403653C9`.

The exact evaluator is recoverable, but its palette-cache path reads the same object
`+0x12c` wrap byte as ID622. Neither the common VALUE/base constructor chain nor
`CPerlinEffect::CPerlinEffect` initializes it, and the exhaustive serializer does not
emit it. Because that byte changes cache topology, choosing either boolean value would
invent state absent from the `.dvc`.

The existing native recipe remains intentionally separate: it uses Direction and Speed
as coordinate translation, scales by Zoom/10, uses effect-ID-seeded SplitMix value noise
with smoothstep, weights octaves by 0.5 and normalizes them, then samples the palette as a
float. The recovered Transform=1 fold cannot repair the missing palette-wrap provenance.

**Disposition: fail-closed after exact schema and domain validation; blocked specifically
on uninitialized and unserialized object `+0x12c`.**

## Transform=1 on a one-row raster

The common post-process is `0x140358DD0`; Transform is read and dispatched at
`0x140358FF9..0x14035902E`. Transform value 1 executes
`0x140359190..0x1403592EA`:

1. Copy the source and clear the destination.
2. Build the two destination rectangles at `0x1403591D8..0x14035923E` and the full
   source rectangle at `0x140359244..0x140359273`, then draw the entire source into
   `(0, 0, floor(N/2), 1)` at `0x140359279..0x14035928F`.
3. Horizontally mirror the entire source at `0x140359295..0x1403592A5`.
4. Draw that mirror into `(floor(N/2), 0, floor(N/2), 1)` at
   `0x1403592AC..0x1403592C2`.

The ID624 audit then followed the Qt 5.15.2 raster path used by those two draws. Daslight
enables Antialiasing but not `SmoothPixmapTransform`, so the one-row reduction uses Qt's
fixed-point nearest-neighbour source selection. Recovered reference maps, written as the
source index selected by each destination pixel, are:

```text
N=4 -> [0, 2, 3, 1]
N=5 -> [1, 3, 3, 1, clear]
N=6 -> [0, 2, 4, 5, 3, 1]
N=1 -> [clear]
```

For odd N both destination rectangles have width `floor(N/2)`, so the last column stays
clear; for N=1 both target widths are zero. The constructor's default is 100 by 1, but
slot 26 replaces that width at runtime. DVC-V6 locks these discrete mappings as the
ID624 exact fold; it does not use a continuous tent-map approximation and does not widen
this new exactness claim to unrelated mapping-family consumers.

ID623 Plasma's earlier Transform disposition remains governed by its separately recorded
proof. ID625 Sweep still has only `direction_change` and remains fail-closed for
Transform=1 under its existing focused test. IDs 622, 626, 627, and 628 validate either
Transform value, but their generator-level provenance failures occur before the fold can
make the whole effect reconstructible.

## Importer contract and deliberate boundary

| ID | Class | Exact TYPE/ID schema | Compatibility mapping or boundary | Disposition |
|---:|---|---|---|---|
| 622 | `CBurstEffect` | `4/1, 6/3, 0/10, 1/11` | object `+0x12c` palette-wrap byte is uninitialized and unserialized | Fail-closed |
| 623 | `CPlasmaEffect` | `4/1, 6/3, 0/10..17` | IDs 10..17 one-for-one; no ID2 -> `grayscale=false`; ID3 -> `vertical_symmetry` | Proven/imported |
| 624 | `CKnightRiderEffect` | `4/1, 6/3, 0/10, 2/11..13, 0/14` | dedicated integer evaluator; ID3 -> exact discrete fold; IDs 10..14 one-for-one | Proven/imported |
| 625 | `CSweepEffect` | `4/1, 6/3, 2/10` | ID10 -> `direction_change`; ID3 only exact at 0 | Proven at Transform=0; Transform=1 fail-closed |
| 626 | `CSparklesEffect` | `4/1, 6/3, 0/10, 1/11, 0/12` | per-thread qrand initial state and prior draw history are absent from `.dvc` | Fail-closed |
| 627 | `CRandomFillEffect` | `4/1, 6/3, 0/10, 0/11` | per-thread qrand initial state and prior draw history are absent from `.dvc` | Fail-closed |
| 628 | `CPerlinEffect` | `4/1, 6/3, 0/10..14` | object `+0x12c` palette-wrap byte is uninitialized and unserialized | Fail-closed |

`dvc_import.rs` recognizes all eight VALUE catalog IDs so reports use the recovered class
names. It first applies `require_exact_dvc_params` and
`require_exact_dvc_param_types`, rejects missing/extra IDs and wrong TYPEs, and validates
constructor domains without clamping. ID623 constructs the previously proven Plasma
recipe. ID624 constructs the additive Knight Rider compatibility recipe with
`daslight_exact=true`, preserves Transform 0/1 in `vertical_symmetry`, and uses the
recovered signed 40 ms duration grid. An empty-BEAMS ID624 remains a validated source
no-op and creates no runtime effect.

IDs 622, 626, 627, and 628 return their provenance-specific deliberate-boundary error
before target/no-op handling. Empty BEAMS does not make an unreconstructible generator
safe to import. Only Knight Rider gains new protocol state; both fields default false and
are omitted when false, so legacy and native `.sdc` JSON retains its prior byte shape.
The VALUE editor keeps the import-only compatibility marker hidden, locks exact generator
parameters against silent mode drift, and displays the imported Transform while leaving
the value palette and timing editable.

All static proof remains binary-version-specific. It does not claim physical fixture,
optics, latency, or visual acceptance, and the four blocked rows must not be relabeled
exact until their missing runtime provenance is available.

## Focused regression evidence

The focused regressions cover:

- ID623 Transform 0 and 1, absent Grayscale -> false, all eight exact field mappings,
  targeted conversion, and empty-BEAMS source no-op;
- ID624 Transform 0 and 1, `daslight_exact=true`, all five class fields, strict
  TYPE/ID/domain drift rejection, and empty-BEAMS source no-op;
- the signed duration boundary and recovered frame grid: 1..79 ms representative inputs
  quantize to 40 ms, 80 ms produces two frames, and zero/negative/fractional/i32-overflow
  values fail closed;
- both allowed Transform values for IDs 622, 626, 627, and 628 followed by the exact
  palette-wrap or qrand provenance error, including the same rejection with empty BEAMS;
- wrong TYPE, extra PARAM, invalid Transform, and at least one constructor-domain failure
  for every residual blocked generator;
- legacy Knight Rider JSON byte-shape plus exact-flag round-trip, native false-path
  preservation, the ID624 discrete fold maps for N=4/5/6/1, and the frontend read-only,
  localization, and visible-Transform contracts;
- unchanged ID625 Transform=1 rejection and strict TYPE/extra-PARAM tests.

## Real saved specimen confirmation (2026-08-10, Fable, elevated permissions)

The static factory/serializer proof above was cross-checked against a real Daslight-saved
project. Working only in a throwaway scratch project (never the user's Shinkan2026 show),
two VALUE FX scenes were authored through the maximized Daslight UI and saved via File ->
Save As to `qa/specimens/ValueCatalog-Sweep-Plasma.dvc` (committed as a golden asset).

Product-to-product confirmations:

1. **Generator catalog completeness.** The VALUE FX generator dropdown exposes exactly
   `{Rainbow, Burst, Plasma, Knight Rider, Sweep, Sparkle, Random fill, Perlin}` — the same
   eight generators mapped to IDs 621-628. None is missing or extra.
2. **VALUE FX family chooser.** Creating a scene shows the nine-family chooser (STEPS /
   COLOR FX / CHASER FX / MOVE FX / VALUE FX / CURVE FX / MAPPINGS / COLOR MAPPINGS /
   SUPER SCENE), matching the documented model.
3. **ID625 Sweep serialized schema — exact byte match.** The saved
   `EFFECT TYPE="7" ID="625"` has `PARAMS NB="3"`: `PARAM TYPE="4" ID="1"` (grayscale
   `COLORS` palette), `PARAM TYPE="6" ID="3" VAL="0"` (Transform None),
   `PARAM TYPE="2" ID="10" VAL="1"` (Direction Change ON). This is exactly the recovered
   `TYPE4/ID1, TYPE6/ID3, TYPE2/ID10` schema, and the `VAL="1"` confirms the direction-
   change serialization. `BEAMS NB="1"` with `FIXTURE=... BEAMID="0" IDSELECTION="1"`
   confirms non-empty targeted beam serialization.
4. **ID623 Plasma serialized schema and defaults — exact byte match.** The saved
   `ID="623"` has `PARAMS NB="10"`: `TYPE4/ID1`, `TYPE6/ID3 VAL=0`, then `TYPE0/ID10..17`
   with values `1, 2, 1, 2, -1, 2, 1, -1` — the exact eight-field schema *and* the exact
   constructor defaults (Size X=1, Param X=2, Size Y=1, Param Y=2, Speed X=-1, Param SX=2,
   Speed Y=1, Param SY=-1) recovered by disassembly.

Scope update (same day, second cycle): the specimen is now feature-complete. Two
additional strongpoint-targeted Sweep/Plasma scenes carry a bound Dimmer feature
(serialized as `PRESET SSLPRESET="4" SSLCHANNEL="-1" MIN="0" MAX="1"`), and the
repo-portable golden `dvc_local_golden_value_sweep_and_plasma_import_from_saved_specimen`
asserts they import as `Sweep { direction_change: true }` and `Plasma` with the exact
constructor defaults. The two original laser-targeted scenes stay in the file as real
coverage of the fail-closed path (their f3200a profile exposes no PRESET type 4; the
golden asserts exactly two such skips). The same file now also carries the first real
COLOR MAPPINGS `5/3` body (generator ID 36 with a `<MAPPING>` placement rectangle) and a
Move `4/4` ID 221 rack with `BEAMID` 1..6 — see `qa/DVC_SPECIMEN_REQUESTS.md` for the
full capture record; both feed the next import tranche.
