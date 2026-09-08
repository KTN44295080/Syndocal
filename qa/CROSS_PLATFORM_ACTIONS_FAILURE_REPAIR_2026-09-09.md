# Cross-platform Actions failure repair — 2026-09-09

## Scope

This checkpoint addresses the environment failures observed before the
Cross-platform workflow reached product compilation or native packaging. It
does not change product code, feature selection, warning assertions, or native
acceptance claims.

## Observed failures

The following `main` runs failed at the same pre-build gates:

- [34290258811](https://github.com/KTN44295080/Syndocal/actions/runs/34290258811)
  at `Install Windows video SDK`: GitHub's `windows-2022` runner reported that
  `winget` was not recognized.
- The same Windows failure was reproduced by runs
  [34290158382](https://github.com/KTN44295080/Syndocal/actions/runs/34290158382),
  [34289890211](https://github.com/KTN44295080/Syndocal/actions/runs/34289890211),
  and [34287314800](https://github.com/KTN44295080/Syndocal/actions/runs/34287314800).
- The Ubuntu job in those runs stopped at `Test warning ratchet` with
  `warning-affecting environment is forbidden: CARGO_HOME`; the runner exposed
  `CARGO_HOME=/home/runner/.cargo` to the self-test.

Neither job reached Rust compilation, frontend build, Tauri admission, or
installer packaging. These failures are workflow-environment failures, not
evidence of a product build failure.

## Bounded repair

`.github/workflows/cross-platform.yml` now:

1. downloads the official Gyan FFmpeg 8.1.2 shared ZIP directly because the
   hosted Windows image does not provide the `winget` command;
2. verifies the published SHA-256
   `274923C68904A9B76C73B908F57923DAFBA81155856CD742138515DED570D066` before
   extraction;
3. uses only the expected extracted SDK root and keeps the existing DLL,
   header, import-library, and runtime checks;
4. removes only the runner-injected `CARGO_HOME` from the warning-ratchet
   child environment on each platform. The ratchet's rejection tests and
   warning assertions remain unchanged.

## Local evidence

At the repair checkpoint:

- `pnpm.cmd --dir app run check:warnings:self-test` — passed;
- `pnpm.cmd --dir app run check:tauri-build-wrapper` — passed (243 assertions,
  27 hostile mutation fixtures);
- `pnpm.cmd --dir app run check:release` — passed;
- `git diff --check` — passed.

The first repair rerun reached two additional existing boundaries:

- the warning-ratchet self-test's promotion fixture was hard-coded to a
  Windows host and failed on Ubuntu before the product gate;
- hosted Windows does not provide the locally licensed ASIO/NDI SDKs, so the
  workflow's default cross-platform gates could not reach their own checks.

The follow-up repair makes the promotion fixture use the detected host while
retaining an opposite-platform rejection assertion. It also keeps default
Windows warning gates enforced and runs the ASIO/NDI warning gates only when
both explicit SDK variables are present; otherwise the workflow reports those
gates as unverified/skipped. The GitHub Actions rerun after this follow-up is
required before claiming the Windows or Linux hosted build gates pass.

The same rerun also exposed two independent baseline conditions: the hosted
Windows `cargo fmt --check` gate reports existing formatting differences across
the current Rust workspace (reproduced locally), and later Windows warning
steps must remove the runner's `CARGO_HOME` for each child process. The latter
is included in the next bounded workflow repair; the former remains a product
source formatting baseline and is not being hidden or auto-reformatted here.

## Boundary

This checkpoint does not claim Windows native-window, hardware, physical
output, release publication, signing, or notarization acceptance.
