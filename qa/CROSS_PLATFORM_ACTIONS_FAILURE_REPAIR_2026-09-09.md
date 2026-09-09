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

The latest rerun [34291912613](https://github.com/KTN44295080/Syndocal/actions/runs/34291912613)
reached the output-marker rebaseline self-test on Ubuntu, where the Linux
runner treated the fixture's `app/node_modules` symlink as an untracked path
despite the directory-only ignore pattern used on Windows. The fixture now
ignores that path name without weakening the audit's allowed-file policy. The
same self-test, release checks, Tauri wrapper checker, and `git diff --check`
pass locally after this bounded change.

After that repair, run 34292560030 passed Linux formatting and warning-ratchet
checks but exposed a real Linux-only test compilation error at
`app/src-tauri/src/tests/dj_link_machine_tests.rs:549`: an `assert_eq!` compared
the whole `Result<PlatformDjLinkCredentialStore, ...>`, requiring `PartialEq`
for the platform store even though the non-Windows constructor always returns
the fail-closed `PlatformUnsupported` error. The assertion now uses
`matches!`, preserving the exact rejection contract without adding equality to
the credential-store type. The same focused test group passes on Windows
locally; a hosted Linux rerun is required to close this gate.

The next rerun [34293175543](https://github.com/KTN44295080/Syndocal/actions/runs/34293175543)
passed the Windows prerequisites, formatting/warning self-test on Ubuntu, and
reached the engine test suite on Ubuntu. It then exposed two stale test
contracts, both unrelated to a product-code regression:

- the generated engine command inventory is now 280 entries, but the test
  still expected the earlier 278-entry baseline; the test now expects 280 and
  explicitly keeps tripwires for the two later publication variants;
- the Art-Net route linearization test still required a byte-for-byte
  unchanged snapshot even though the current safety-authority contract
  publishes `safety_blackout_engaged` after the priority blackout. The test
  now asserts that only this externally visible safety latch changes, while
  route generation, sender ownership, ordinary blackout, and all other
  snapshot fields remain unchanged.

The two focused engine tests pass locally after this test-only repair. A
hosted rerun is required before claiming the Linux test/build gate passes.
The same run also reported a hosted Linux allocation failure while compiling
and a hosted Windows allocation failure during `cargo fmt --check`; these are
runner/resource and current-workspace formatting baseline conditions. The
Windows formatting baseline remains intentionally unfixed and is not being
masked by changing the gate.

The hosted rerun [34294742712](https://github.com/KTN44295080/Syndocal/actions/runs/34294742712)
confirmed that the two repaired engine contracts are no longer blockers:
the engine test binary reported `1061 passed; 0 failed; 14 ignored`. The same
workspace command then reached the Tauri application binary and reported
`1417 passed; 12 failed; 12 ignored`. These 12 failures are in existing
application acceptance/fixture areas (UNC-path admission, C1 preview seam,
NDI startup-fence cleanup, publication-owner retirement, durable lease parent
paths, first-run VJ bootstrap, and Unix media-file versioning). They are not
caused by the two test-only changes in commit `09f415c` and are not being
silently skipped or relaxed as part of this CI prerequisite repair. They need
a separately owned Linux/application-compatibility tranche before the Linux
hosted gate can be called green.

The same rerun reconfirmed the Windows boundary: SDK/MSVC/runtime staging and
the ASIO/NDI availability report passed, while `cargo fmt --all -- --check`
failed on the current workspace's broad pre-existing formatting diff and also
reported a `76544627040`-byte allocation failure. No broad auto-format or
format-gate suppression was applied.

## Boundary

This checkpoint does not claim Windows native-window, hardware, physical
output, release publication, signing, or notarization acceptance.
