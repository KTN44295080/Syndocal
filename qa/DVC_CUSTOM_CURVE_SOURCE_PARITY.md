# Daslight DVC Custom Curve source parity evidence

Date: 2026-08-11

## Evidence boundary

- Installed binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- Product version: `5.0.6.2`; file version `25.0905.165.111`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Factory/creator/constructor: `0x140369268` / `0x14036CE10` / `0x14036E270`
- Class/vtable/evaluator: `CCustomCurveEffect` / `0x1406C8668` / `0x14036F1F0`
- Scalar outer evaluator: `0x14036F710`; caller sample position: `0x14001BA10`
- Real saved specimen: `qa/specimens/CurveCatalog-Custom.dvc`, SHA-256
  `58F7BF7E7B3472C317DE68721F63FDAE33755C5550A75DFD3555E91E5C0AA4BA`.

The binary and native specimen are version-specific. They do not establish behavior for a
different Daslight build.

## Saved schema and legal envelope

Custom is CURVE family `RACK TYPE=8 / EFFECT TYPE=5 / ID=13`. Its constructor removes the
five-property common Curve schema and registers exactly:

```xml
<PARAMS NB="2">
  <PARAM TYPE="5" ID="1">
    <POINTS NB="2..255">
      <POINT X="..." Y="..."/>
    </POINTS>
  </PARAM>
  <PARAM TYPE="1" ID="2" VAL="0..1"/>
</PARAMS>
```

ID 1 is `Points`; ID 2 is `Phasing`. The native UI-authored X domain is normalized `0..1`.
Y combines value and easing as `10 * easing_code + value`, where value is `0..1` and codes
0..4 are Linear, InCubic, OutCubic, InOutCubic, and OutInCubic. The captured default is
`(0,0.5),(1,0.5)`, Phasing 0, and DURATION 5000 ms. The importer accepts unsigned 32-bit
DURATION `40..4294967295` and keeps the exact authored milliseconds.

The source loader does not sort or clamp points. Syndocal therefore parses in document order
and never sorts, but rejects duplicate/decreasing X, out-of-domain coordinates, invalid Y
codes, malformed counts, unexpected nested elements, external selections, missing explicit
`IDSELECTION`, or non-Dimmer targets as unrepresentable authored input. Legal beam order is
the first-seen XML/`IDSELECTION` order and is preserved. This preserves legal UI-authored data without silently
inventing semantics for malformed files.

## Recovered evaluator

For a source interval `[left.x,right.x)`, the first source-order matching adjacent pair is used.
The right/destination point selects easing. Outside all intervals the decoded last-point value
is returned. The five recovered functions for normalized segment progress `u` are:

- Linear: `u`
- InCubic: `u^3`
- OutCubic: `1-(1-u)^3`
- InOutCubic: `u<0.5 ? 4u^3 : 1-(-2u+2)^3/2`
- OutInCubic: `u<0.5 ? (1-(1-2u)^3)/2 : ((2u-1)^3+1)/2`

Daslight builds `N=floor(DURATION/40)`, evaluates integer samples at
`(sample_index % N)/N`, and its outer wrapper linearly interpolates `n` and `n+1`; the latter
wraps through the discrete evaluator modulo N. Its loader coerces 0..39 ms to one sample and
its serializer writes `N*40`, losing a non-multiple remainder.

Syndocal classifies the route as `SyndocalCorrected`: it evaluates the exact recovered easing
directly at continuous authored progress and keeps the exact authored DURATION. This removes
the 40 ms sampled approximation and duration-remainder loss; it does not change point values,
easing identity, target order, or cycle period.

Custom Phasing is not Syndocal's distributed `fixture_spread`. For stable target rank `i`, the
source uses `fract(progress - i * Phasing)`: adjacent targets lag by the full authored value,
the first target is unshifted, and Phasing 1 wraps all targets together. The dedicated source
profile stores this value; the generic request phase and fixture spread remain zero.

## Syndocal representation and performance

`LfoShape::DaslightCustom` is import-only and requires a dedicated
`DaslightCustomCurveSource`. It stores raw X/Y point order, Phasing, and `sample_ms=40`
provenance. Ordinary Curve, Value, Mapping, Position Wave, Node Graph, and video paths cannot
consume this source. Existing LFO JSON omits the additive field and remains backward-compatible.

At command/rebuild time the engine validates and decodes all points once. The runtime retains
the authored monotonic order and uses `partition_point` for allocation-free O(log 255) segment
lookup. A dedicated release gate constructs 64 enabled effects by 200 fixtures at 255 points,
DURATION `u32::MAX`, Phasing 1, all five easing codes including raw Y 41, 44 Hz, and 1000
release samples. Its immutable limits remain p95 5 ms, p99 8 ms, max 12 ms.

## Automated evidence

- Protocol JSON round-trip preserves raw points, Phasing, sample provenance, and omission for
  legacy/native LFOs.
- Engine tests cover all five right-point easing functions, continuous evaluation, last-point
  fallback, adjacent-target lag, strict source validation, and cross-family/video rejection.
- Importer tests cover malformed count, duplicate X, invalid Y encoding, unexpected children,
  and the real saved specimen's two points plus ordered 32-beam Dimmer target list.
- The release gate asserts every maximum literal and the supported-envelope constants in its
  body before measuring the unchanged 5/8/12 ms limits.

Physical fixture photometry, PWM response, and device latency remain separate hardware
acceptance. This tranche establishes normalized DMX software behavior and native file import,
not real-rig timing parity.
