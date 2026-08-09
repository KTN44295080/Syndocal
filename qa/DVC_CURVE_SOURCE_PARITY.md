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
- Strobe:
  `interval=floor(25/Rate)`, `remainder=sample_index%interval`; High when
  `remainder==0` or `remainder<interval*Phase/2`, otherwise Low. Low is `Offset`; High is
  `Offset+Size/2`; both are clamped to 0..1.

`DURATION` is the complete sampled Curve buffer, not `DURATION/Rate`. Rate controls how many
wave cycles are drawn into that buffer. The former Syndocal conversion divided DURATION by Rate
and approximated Strobe as ten 2%-wide pulses; both assumptions were removed.

## Syndocal representation

Imported LFO requests retain an additive `daslight_curve` source profile containing `rate`,
`size`, `offset`, and `sample_ms=40`. Native Syndocal LFOs omit the profile and retain their
existing continuous evaluator.

The evaluator quantizes the source position before applying the recovered equation. Size and
Offset are preserved instead of being reduced to clamped endpoints: every sample is clamped
after evaluation. This is necessary for `homecoming2606.dvc` scene `all_rampFlash`, whose raw
Inverse Ramp range is approximately `-0.567..0.995`; its negative portion must remain at DMX 0
for a finite interval.

Scene Settings loads the complete imported LFO request into its draft and writes the same source
profile back when saving. The source profile also round-trips through `.sdc`, project history,
Cue-owned effect storage, effect snapshots, and presets. Legacy and native LFO JSON omits the
field and continues to deserialize with `None`.

## Regression evidence

- `cargo test -p engine daslight_ -- --nocapture`
  - 6 passed, 0 failed.
  - Covers 40 ms hold boundaries, Strobe Rate/Phase pulse widths, Sinus source Phase, a clipped
    Sinus plateau, the real `all_rampFlash` descending range/zero plateau, and invalid source
    profiles.
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
