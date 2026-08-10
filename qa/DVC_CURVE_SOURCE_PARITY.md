# Daslight DVC Curve source parity evidence

Date: 2026-08-09

## Source under test

- Installed binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- Product version: `5.0.6.2`
- File version: `25.0905.165.111`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Static evaluator entry points recovered from RTTI/vtables:
  - `CSinusEffect`: `0x140370250`
  - `CInverseRampEffect`: `0x14036fcb0`
  - `CPulseEffect`: `0x14036F8D0`
  - `CSquareEffect`: `0x140370420`
  - `CStrobeEffect`: `0x1403705b0`
- The imported source buffer is evaluated on a 40 ms grid.

The evidence above is local static analysis of the named binary. It is version-specific and must
not be generalized to a different Daslight build without repeating the analysis.

## Recovered sample equations

For `sample_count=floor(DURATION/40)` and `t=sample_index/sample_count`:

- Sinus:
  `clamp(sin(2*pi*(Rate/2*t-Phase))*Size/2 + Offset + Size/2, 0, 1)`
- Inverse Ramp:
  `x=Rate/2*t-Phase`, `centered=x-floor(x+0.5)`, then
  `clamp(Offset-centered*Size+Size-0.5, 0, 1)`
- Pulse:
  `carrier=sin(2*pi*(2*Rate*t-Phase))`; Daslight multiplies the carrier by a triangle
  whose fixed slope is `0.005` per sample. Consequently its peak is
  `floor(sample_count/2)*0.005`, so DURATION changes the effective Size and short buffers never
  reach the authored amplitude.
- Strobe:
  `interval=floor(25/Rate)`, `remainder=sample_index%interval`; High when
  `remainder==0` or `remainder<interval*Phase/2`, otherwise Low. Low is `Offset`; High is
  `Offset+Size/2`; both are clamped to 0..1.
- Square:
  `grid=floor((sample_index%sample_count)*400/sample_count)`,
  `cell=trunc(grid+400-Phase*400)%400`, `band=trunc(cell/floor(400/Rate))`;
  even bands are `Offset+Size`, odd bands are `Offset`, then clamped to 0..1. The integer
  `floor(400/Rate)` preserves Daslight's uneven terminal band for Rates that do not divide 400.

`DURATION` is the complete sampled Curve buffer, not `DURATION/Rate`. Rate controls how many
wave cycles are drawn into that buffer. The former Syndocal conversion divided DURATION by Rate
and approximated Strobe as ten 2%-wide pulses; both assumptions were removed.

## Syndocal representation

Imported LFO requests retain an additive `daslight_curve` source profile containing `rate`,
`size`, `offset`, and recovered provenance `sample_ms=40`. Native Syndocal LFOs omit the profile
and retain their existing continuous evaluator.

All routed DVC Curve sources now follow `DVC_CORRECTED_COMPATIBILITY_POLICY.md`:

- Sinus and Inverse Ramp retain the recovered equations but evaluate continuous `t` instead of
  holding 40 ms work samples.
- Pulse retains the recovered carrier and all authored parameters, but uses
  `window=1-abs(2*t-1)` on continuous time. This removes both the DURATION-dependent amplitude
  defect and the 40 ms output hold.
- Square uses `band=floor(fract(t-Phase)*Rate)`, removing the 400-cell residue while preserving
  authored band count, alternating polarity, Size and Offset.
- Strobe uses the exact authored `1/Rate` interval and a common dimensionless 20% base duty,
  extended by `max(0.2, Phase/2)`. Duty is independent from the recovered 40 ms sample grid and
  Rate, so high Rates do not collapse into an always-high output.

Each import report records the recovered grid and the applicable correction; none of these
routes is described as frame-equivalent Daslight output. Size and Offset remain source values
instead of being reduced to clamped endpoints: every evaluated value is clamped. This is
necessary for
`homecoming2606.dvc` scene `all_rampFlash`, whose raw
Inverse Ramp range is approximately `-0.567..0.995`; its negative portion must remain at DMX 0
for a finite interval.

Scene Settings loads the complete imported LFO request into its draft and writes the same source
profile back when saving. The source profile also round-trips through `.sdc`, project history,
Cue-owned effect storage, effect snapshots, and presets. Legacy and native LFO JSON omits the
field and continues to deserialize with `None`.

## Regression evidence

- Focused engine regressions cover removal of the 40 ms hold, exact Strobe Rate with common
  dimensionless duty, equal Square bands, normalized Pulse amplitude, Sinus source Phase, a
  clipped Sinus plateau, the real `all_rampFlash` descending range/zero plateau, and invalid
  source profiles.
- `cargo test -p syndocal dvc_ -- --nocapture`
  - 45 passed, 0 failed.
  - Covers synthetic conversion plus all locally available `.dvc` inventories and goldens.
- `cargo test --workspace --all-targets`
  - All non-ignored tests passed. Engine: 453 passed/1 manual benchmark. Tauri: 371 passed/9
    environment-dependent tests. Video: 114 passed/1 real-GPU test. Other crates: all passed.
- `pnpm --dir app run check:cue-effect-recall`
  - Passed; source request synthesis and Scene Settings load/save preservation are guarded.
- `pnpm --dir app run check:scene-settings`
  - Passed at 1920x1080, measured 1920x1032, 2048x1152, 1366x768, and 1280x720 with no app
    scroll or failed interaction assertions.
- `pnpm --dir app build`
  - TypeScript and Vite production build passed.
- `pnpm --dir app tauri build --no-bundle`
  - Native release build passed and produced `target/release/syndocal.exe`.

## Remaining evidence boundary

- The recovered generator equations and the locally present Curve scenes are software-verified.
- The available real DVC Curve specimens use zero Phasing. Non-zero fixture Phasing is preserved
  and covered by deterministic engine tests, but has not been compared against a physical or
  captured Daslight multi-fixture output trace for this binary.
- A physical controller/fixture is not required to verify these normalized DMX time series, but
  fixture photometry, PWM response, and device latency remain physical acceptance items.
- Native responsive-window acceptance is run separately; it is not inferred from the successful
release build.
