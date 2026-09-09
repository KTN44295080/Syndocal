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

The next run [34302972902](https://github.com/KTN44295080/Syndocal/actions/runs/34302972902)
reached the remaining gates on both hosts. Its Windows job passed the warning
self-test and all Cargo warning ratchets, then failed only in the frontend
generic-output checker because the literal marker
`vite v6.4.2 building for production...` was not found. The Vite build itself
completed successfully. This was reproduced locally by removing `NO_COLOR`:
Vite inserts ANSI color codes inside that marker (`vite v6.4.2 <color>building`),
so the marker text is still semantically present but not byte-contiguous.
`app/scripts/warning-ratchet-lib.mjs` now removes terminal escape sequences
before applying the same exact marker and warning-shaped-output assertions;
the self-test includes ANSI-colored marker and warning fixtures. No marker was
shortened or removed.

On the same run, Ubuntu reported `1427 passed; 2 failed; 12 ignored`. The two
failures are `ndi_open_failure_hands_one_fence_to_parent_until_cleanup_ack`
and `ndi_startup_timeout_and_late_constructor_error_share_one_fence`; both
panic at the explicit re-arm with `Output ownership transition is already in
progress`. Windows focused executions of both tests pass. An Ubuntu-only
rerun was started without source changes (attempt 2 of run 34302972902) to
separate a hosted scheduling/resource flake from a deterministic Linux gate;
its result remains required before claiming the hosted Linux gate is green.

The follow-up run [34305610272](https://github.com/KTN44295080/Syndocal/actions/runs/34305610272)
used commit `25ba7aea41d72b0d66ed121ba5ec6ab223b468c8`. Its Windows job passed
the warning self-test, generic warning ratchets, and the unavailable-SDK
boundary before reaching the native release warning gate. Its Ubuntu job
passed the warning gates and engine suite, then reported `1428 passed; 1
failed; 12 ignored` in the Tauri application binary. The only reported test
failure was
`ndi_transport::capture_decoder_tests::ndi_startup_timeout_and_late_constructor_error_share_one_fence`
at its explicit re-arm assertion. The sibling
`ndi_open_failure_hands_one_fence_to_parent_until_cleanup_ack` passed in this
run. An earlier runner-side `memory allocation of 32969475296 bytes failed`
message did not terminate the job and is recorded separately from the test
failure.

The failing test was narrowed to a real scheduling boundary: the test engine
continues its 44 Hz tick while the NDI startup failure fence is held, and a
Lighting permit admitted just before the fence remains in-flight until that
tick returns. Dropping the startup cleanup lease must therefore keep
`transition_active` set until the permit drains; the fail-closed production
gate is not weakened. The test now retries only that exact
`Output ownership transition is already in progress` response, with a
one-second deadline, before asserting the re-arm. Local Windows focused runs
of both NDI fence tests passed 40/40 paired repetitions after this change;
`cargo fmt --all -- --check` still reports unrelated pre-existing formatting
differences in other files, so no broad formatting change was made. A new
hosted rerun of the changed commit is required before claiming the Linux gate
green.

The next hosted run [34307030258](https://github.com/KTN44295080/Syndocal/actions/runs/34307030258)
used `5a325a413048ba44663ba1a0858a055a7f3ab0c8`. Ubuntu passed setup,
formatting, and warning gates, and the Rust workspace reached the video crate
before failing
`ffmpeg_cancellable_process::tests::cancellation_terminates_and_reaps_a_running_process`.
The test reported `cancelled child must be reaped promptly` after 35 seconds.
The Linux fixture used `sh -c "sleep 30"`; killing only the shell leaves its
grandchild holding the piped descriptors, so the reader joins wait for the
grandchild. This is a test-fixture process-tree issue, not an FFmpeg
cancellation result. The fixture is now a direct `sleep 30` child on Unix,
preserving the direct-child kill/reap assertion without changing product
process cancellation. The Windows job had already passed setup, MSVC pinning,
runtime staging, and the colored-output warning gate before the run was
superseded by this bounded test repair. A new hosted rerun is required.

The next hosted run [34307971676](https://github.com/KTN44295080/Syndocal/actions/runs/34307971676)
confirmed the earlier repairs on Ubuntu: the full Ubuntu job passed in 25m49s,
including Rust workspace tests, Tauri checks, Linux deb/AppImage packaging,
smoke testing, and artifact upload. Its Node.js 20 deprecation annotation is
an action-runtime warning, not a job failure. Windows passed setup, MSVC and
runtime staging, all default warning ratchets, and the native release warning
ratchet, but then failed three Windows recording publication tests:

- `recording_artifact::tests::absent_target_publication_atomically_rejects_a_late_creator`;
- `recording_artifact::tests::absent_target_publication_succeeds_and_removes_owned_partial`;
- `recording_artifact::tests::complete_output_replaces_target_only_on_publish`.

The result was `1791 passed; 3 failed; 21 ignored`. All three failures came
from `SetFileInformationByHandle(FileRenameInfo)` returning
`ERROR_INVALID_NAME (0x8007007B)` when the short canonicalized target was
passed with Rust's `\\?\\` extended prefix. The product publication protocol
was not weakened: no replace flag was enabled, ownership and recovery journal
checks remain unchanged, and the late-creator test still requires rejection.
The Windows rename adapter now converts only short extended drive/UNC paths
to conventional absolute DOS/UNC form for `FileRenameInfo`; paths exceeding
the conventional limit retain the extended form and therefore remain
fail-closed. Pure path-normalization tests cover drive, UNC, and long-path
cases. A hosted Windows rerun is required to validate the API-level repair.

The hosted rerun [34311340390](https://github.com/KTN44295080/Syndocal/actions/runs/34311340390)
validated the source repair but did not complete the Windows job within the
workflow's explicit `timeout-minutes: 60` limit. Ubuntu completed successfully
in 20m55s, including the Rust workspace, in-process video decode, Tauri checks,
Linux packaging, smoke test, and artifact upload. Windows passed setup, MSVC
pinning, runtime staging, all warning ratchets, the Rust workspace (including
the three previously failing recording publication tests), in-process video
decode, frontend build, Tauri check, and release metadata. It reached
`pnpm --dir app tauri build --ci --bundles nsis,msi`; the release binary finished
in 6m31s, pinned FFmpeg DLL staging and the ASIO packaging self-test passed, and
NSIS `makensis` started. The job was then canceled at 05:32:10Z with
`The job has exceeded the maximum execution time of 1h0m0s`; no compiler,
test, packaging assertion, or installer error was reported. This is a CI
wall-clock budget failure in the combined Windows test-and-bundle job, not a
new product or recording-publication failure. The Node.js 20 deprecation
annotation is an action-runtime warning and is not a job failure.

## CI wall-clock budget repair

The Windows test-and-bundle job exceeded its explicit 60-minute limit after
the source repair had already passed the Rust workspace, video decode,
frontend, Tauri, and release-metadata gates. The measured run spent about
19 minutes in the Windows warning ratchets and about 15 minutes in the native
release warning ratchet before the test/build gates completed; it reached NSIS
only near the end of the one-hour budget. The workflow job timeout is now 90
minutes so the existing installer, smoke-test, and upload assertions can run
to completion. This changes no product code, warning assertion, SDK
availability boundary, or packaging command. A hosted rerun at the new
workflow commit is required before claiming the Windows hosted packaging gate
green.

## Boundary

This checkpoint does not claim Windows native-window, hardware, physical
output, release publication, signing, or notarization acceptance.
