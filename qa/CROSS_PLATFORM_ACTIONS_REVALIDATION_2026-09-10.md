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

The existing run [34391689414](https://github.com/KTN44295080/Syndocal/actions/runs/34391689414)
was testing the pre-checkpoint `main` HEAD `86fee3e7400f490ccd9258a9b4ec11e304ec81a3`
while this checkpoint was prepared. It is not post-repair evidence. A hosted
run for this checkpoint is required before claiming the cross-platform hosted
Windows/Linux workflow green.

## Remaining boundary

This checkpoint does not close the completion ledger, AI3 durable crash/restart
acceptance, real-file thumbnail recovery, ASIO/NDI/DMX/hardware acceptance,
macOS acceptance, signing, publication, or product-wide completion.
