# Cross-platform Actions and ASIO timeline revalidation — 2026-09-10

## Scope

This checkpoint repairs two independently reproduced test/CI boundary defects
without changing product behavior, warning assertions, feature selection, or
native/hardware acceptance claims.

The real-file thumbnail missing -> UI Retry -> restore -> recovery trial was not
rerun and is not claimed here. No old Mac candidate, Channel API, physical
output, signing, publication, or installer acceptance is claimed.

## Findings

- Hosted run [34388600941](https://github.com/KTN44295080/Syndocal/actions/runs/34388600941)
  failed the Windows warning ratchet before product compilation because a first
  push supplied `github.event.before` as an all-zero SHA. The warning resolver
  correctly rejects that invalid comparison ref; the failure was in the
  workflow input, not in a warning assertion or product build.
- Hosted run [34385989058](https://github.com/KTN44295080/Syndocal/actions/runs/34385989058)
  failed one Windows workspace test at
  `tests::asio_timeline_play_linearizes_after_test_and_keeps_domains_separate`.
- Before this checkpoint, the same focused test was reproduced locally under
  the exact MSVC 14.44.35207 Build Tools linker. The test used an empty
  Timeline; the engine's ordinary 44 Hz tick is allowed to finish an empty
  Timeline and clear the live snapshot before the assertion. The sibling Solo
  test already seeded a non-empty 16-second Timeline for this reason.

## Bounded repair

- `app/src-tauri/src/main.rs` now gives the Test/Play fixture the same bounded
  non-empty Timeline setup. This changes only test setup; the production
  transport, ASIO fail-closed gate, and assertions are unchanged.
- `.github/workflows/cross-platform.yml` now removes an all-zero
  `WARNING_RATCHET_BASE_REF` in each Windows warning-ratchet gate. The
  warning checker then uses its existing `HEAD^` fallback. A repository with
  no parent still fails closed through the resolver; no warning comparison is
  skipped or weakened.

## Local evidence

The Cargo commands used `vcvars64.bat -vcvars_ver=14.44`, pinned
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and verified that exact path was first in `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `asio_timeline_play_linearizes_after_test_and_keeps_domains_separate` | 10/10 passed after the fixture repair |
| `asio_timeline_play_linearizes_after_solo_and_clears_both_selections` | passed |
| `pnpm.cmd --dir app run check:warnings:self-test` | passed |
| `pnpm.cmd --dir app run check:release` | passed; admission inventory 516 commands, 18 negative fixtures rejected |
| all-zero comparison-ref workflow smoke | passed; environment value removed and checker fallback preserved |
| `git diff --check` | passed |

## Hosted revalidation

Hosted run [34430832502](https://github.com/KTN44295080/Syndocal/actions/runs/34430832502)
validated checkpoint HEAD `ddb392dbffbaacf25ca573b1c7aebbb9c0951a60` on both
Ubuntu 22.04 and Windows 10+; both jobs and all Windows steps completed
successfully.

The first attempt on this SHA exposed one timing-sensitive Windows failure in
`dj_link_production_remote_stop_is_bounded_and_replacement_rejects_replay`
(`Busy` was observed where the test expected `Accepted`). The second attempt
exposed one different Windows failure in
`recording_artifact::publication::tests::recording_publication_recovers_after_process_exit_at_each_boundary`
at `BeforeInstall` (`Recording recovery is ambiguous`). Neither failure was
caused by this checkpoint's icon, warning-ratchet, or ASIO timeline changes.
The DJ Link test passed in 10 focused local repetitions, the recording
publication test passed in 20 focused local repetitions under the pinned MSVC
linker, and the third attempt of the same hosted run completed successfully.
These two initial failures remain recorded as observed hosted timing/file-boundary
flakes; no assertion was removed or weakened and no test was skipped.

This hosted result does not establish ASIO/NDI/DMX hardware, physical output,
macOS, signing, publication, or product-wide completion.

## Remaining boundary

This checkpoint does not close the completion ledger, AI3 durable crash/restart
acceptance, real-file thumbnail recovery, ASIO/NDI/DMX/hardware acceptance,
macOS acceptance, signing, publication, or product-wide completion.

## Current-tree timing-boundary revalidation — 2026-09-10

The current tree at `383d84894ea9cde54506f6344193cae6399f6998` was checked
against the source used by the hosted-success checkpoint. The intervening
changes contain QA records and release-test fixtures only; no product source,
workflow, Cargo manifest, or frontend dependency changed in the two timing
test paths.

Under the documented Windows procedure, Build Tools `14.44.35207` was
initialized, the absolute x64 linker was pinned, and `where.exe link.exe`
reported that linker first. The focused tests were rerun from the current
feature worktree:

| Check | Result |
| --- | --- |
| `recording_publication_recovers_after_process_exit_at_each_boundary` | PASS — 1 passed, 0 failed |
| `dj_link_production_remote_stop_is_bounded_and_replacement_rejects_replay` | PASS — 1 passed, 0 failed |

Neither hosted timing failure reproduced locally. No assertion, retry, skip,
or product behavior was changed in response to those observations. These
focused tests do not replace the hosted matrix and do not claim physical
output, external-client, hardware, or product-wide acceptance.
