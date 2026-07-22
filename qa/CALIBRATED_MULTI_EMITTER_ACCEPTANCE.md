# Calibrated Multi-Emitter Colour Acceptance

Updated: 2026-07-23

## Contract

T19-E adds profile-driven additive-emitter mixing without guessing from attribute names.

- GDTF `PhysicalDescriptions/Emitters/Emitter` metadata is imported into each linked `ChannelFunction` as an optional emitter name, CIE xyY colour and optional dominant wavelength.
- A calibrated binding is enabled only when a fixture exposes three to sixteen valid visible emitters whose XYZ basis is non-degenerate. The command/rebuild path solves non-negative RGB-cube anchor mixtures and stores fixed emitter values in the compiled output binding.
- The 44 Hz path only interpolates four fixed anchors. It performs no emitter search, string normalization, solver work or allocation.
- Metadata-free Amber, Lime, UV and similar additive channels are held at zero. UV emitters whose GDTF entry intentionally omits `Color` may retain their wavelength for inspection, but are not guessed into visible RGB output.
- Ordinary RGB, RGBW, CMY, HSV and colour-wheel fallbacks remain available when no valid calibrated multi-emitter basis exists.

The metadata contract follows the official GDTF 1.2 definitions for [`Emitter`](https://gdtf-development.com/help/developers/gdtf_1_2/file-format-definition/index.html) and the `ChannelFunction.Emitter` link. GDTF permits `Color` to be omitted for non-visible emitters such as UV; that omission is therefore treated as a deliberate fail-closed input rather than synthesized colour.

## Compatibility and validation

- `ChannelFunctionSummary.emitter` is additive, defaults to absent and is omitted from serialized legacy profiles. Protocol tests lock both the unchanged legacy JSON shape and the populated additive shape.
- GDTF parsing covers valid xyY plus wavelength, dotted emitter references, UV without colour, invalid metadata warnings and unresolved references.
- Tauri project validation rejects blank emitter names, non-finite/out-of-range xyY values and invalid wavelengths for patched fixtures and custom profiles.
- The DVC importer writes no invented emitter calibration.
- Setup Library exposes emitter name, xyY and wavelength details; the full five-viewport application matrix requires three visible calibrated rows.

## Engine evidence

Focused tests prove that a calibrated RGB + Amber + UV fixture selects the calibrated Amber emitter for yellow while UV remains zero, and that the same uncalibrated Amber/UV channels remain zero rather than using name-based heuristics. Both independent Colour and Colour Mapping use the same compiled output policy.

The 2026-07-23 local gates passed:

- protocol: 39 tests passed;
- GDTF: 18 tests passed;
- engine: 392 passed, 1 manual performance test ignored;
- Tauri: 326 passed, 9 hardware-dependent tests ignored;
- full Rust workspace, frontend production build, 2701/2701 localization strings and the complete five-viewport application matrix: passed;
- frontend main JavaScript bundle: 484.65 kB.

## 44 Hz release evidence

The calibrated benchmark uses 200 fixtures, 64 mixed Colour/Chaser/Move/Curve/Mapping/Colour Mapping effects and eight evaluated controls per fixture, including RGB, Amber and Lime.

| Path | Result | Fixed gate | Verdict |
|---|---:|---:|---|
| Calibrated ordinary stack | p95 7.672 ms / p99 8.052 ms / max 9.660 ms | 10 / 14 / 18 ms | PASS |
| Calibrated all-64-transition stress | p95 17.489 ms / p99 18.119 ms / max 18.390 ms | 18 / 21 / 22.5 ms | PASS; maximum remains inside one 22.7 ms tick |

The established uncalibrated 5/8/12 ms gate was not relaxed. On the same host session it was marginally red on both the detached pre-T19-E commit and the T19-E worktree:

| Revision | p95 | p99 | max |
|---|---:|---:|---:|
| `e7b29e1` before T19-E | 5.113 ms | 5.428 ms | 5.984 ms |
| T19-E worktree | 5.110 ms | 5.530 ms | 7.142 ms |

This same-host A/B shows no p95 regression from T19-E, but it is not recorded as a green absolute-gate run. An earlier T19-D run on the same day was green at p95 2.654 ms / p99 2.955 ms / max 3.460 ms, confirming that the absolute result is sensitive to current host conditions. Final T23 acceptance must rerun the unchanged gate on a controlled release host.

## Remaining boundary

T19-E closes the calibrated multi-emitter software-model gap. It does not prove physical colour fidelity, fixture spectral output or an external visualizer workflow. T23 must still compare representative calibrated and fallback fixtures through the formal Art-Net external-visualizer path and, where hardware is available, physical output. Internal 3D visualization remains outside the product boundary.
