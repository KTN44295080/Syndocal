# Output admission and Windows recording repairs — 2026-10-11

Branch: `codex/native-output-guards-20261010`.
Base: `a0d00698252a43e9f7768beb741095b66bafb793`.
Internal checkpoint; product metadata remains `1.2.0-alpha.71`.

## Finding and change

[The base CI run](https://github.com/KTN44295080/Syndocal/actions/runs/38040308609)
failed the same three Rust tests on Windows and Linux. All three failures were
reproduced locally with the exact MSVC 14.44.35207 linker pin and PATH-first check.

The route-count failure concealed a product defect: the registered canonical
Lighting master, group submaster, Video master and Video Take commands had no
reviewed runtime dispatch policy. Ordinary Tauri dispatch rejected them before
their typed native handlers. Add those four names to the sorted inner-authority
preflight list. Clip Launch was already present. The canonical executor retains
its operation/action validation, owner, project/output/S0 fence and lease checks;
no new grant, implicit Arm, retry or output-ownership shortcut was added. The
outer admission guard is released before the existing asynchronous executor,
which acquires its own authority locks. Lookup remains one bounded binary search.

The Bank and both Scene-create test fixtures now include their required UUID
`requestId`. They also reject missing, snake-case and numeric request IDs;
unknown-field and missing/wrong-case owner checks remain. The runtime test retains
exact coverage of all 168 routes and additionally requires the five output
adapters to use their inner-authority policy. The file-export test retains exact
set equality for all 29 routes and classifies the seven newer typed adapters
separately from snapshot exports, checking their DTO and exact native delegate.

The release aggregate subsequently exposed the recording non-completion boundary
outside the metadata checker's first 45 lines. Move the existing recorded
alpha.69 boundary to the beginning of the completion flow, retaining every
required marker and historical qualification. No validator was weakened and
no release or ledger acceptance status was promoted.

## Windows recording repairs

The full Windows workspace run then exposed two recording-publication failures:
`absent_target_publication_succeeds_and_removes_owned_partial` and
`complete_output_replaces_target_only_on_publish` returned `ERROR_INVALID_NAME`.
The variable-size rename buffer relied on alignment padding to terminate its
UTF-16 filename; exact word-sized payloads had no terminator. Reserve one explicit
zeroed UTF-16 terminator while leaving `FileNameLength` unchanged, as required by
[Microsoft's FILE_RENAME_INFO contract](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info).
Eight new cases cover four successive Unicode filename lengths for both new and
existing destinations. The existing recording survives until publication, the
complete replacement is verified, and the owned partial disappears. No-clobber,
handle identity, journal and crash-recovery authority remain unchanged. This adds
at most one allocation word at publication, with no per-frame work.

Actual FFmpeg encoding additionally reproduced a false cancellation after EOF:
the supervisor marked input cancelled even after its owner had closed the pipe.
Move the cancellation latch inside the existing locked live-handle check. This
keeps outstanding-write cancellation and handle reuse protection intact without
retrospectively rejecting a closed input. A controlled child acknowledges EOF,
remains alive while Stop is observed, then exits successfully; the new regression
requires successful completion and preserved diagnostics. Existing blocked-write,
inherited-pipe and forced-termination assertions are retained. No extra frame
copy, worker, polling loop or lock was introduced.

## Validation

- [Initial local reproduction](artifacts/output-admission-ci-20261011/syndocal-ci-regressions-baseline-20261011.txt):
  all three selected tests ran and failed for the CI reasons.
- [Intermediate result](artifacts/output-admission-ci-20261011/syndocal-ci-regressions-fixed-20261011.txt):
  fixture/inventory repairs passed; the new exact policy assertion exposed the
  missing product policies rather than hiding them behind a changed count.
- [Final focused result](artifacts/output-admission-ci-20261011/syndocal-ci-regressions-final-20261011.txt):
  all three selected tests passed, none ignored.
- Release metadata and its 137 assertion groups passed. The unchanged 561-command
  Tauri admission inventory passed with all 18 negative fixtures rejected.
- [Recording publication/recovery tests](artifacts/output-admission-ci-20261011/syndocal-recording-path-fixed-20261011.txt):
  30 passed, zero failed; one subprocess helper is ignored as a standalone test
  and explicitly invoked by its parent at four crash boundaries. This focused
  run preceded the separate closed-input cancellation repair.
- [Actual native MCP/admission proof](artifacts/output-admission-ci-20261011/native-admission-03.json):
  30 checks passed, no native panic, normal profile identity unchanged, QA process
  cleaned up and its credential revoked/removed. Four raw Tauri probes require
  exact typed `invalid_request` responses and unchanged project/runtime/ownership;
  an internally valid lease-acquire request is deliberately routed to each
  different executor. The earlier two failed probes used an internally
  inconsistent operation/action and correctly failed deserialization; fixing
  that fixture retained every no-effect assertion. Existing BO receipt,
  low-ID GUI interleaving, stale/missing authority, revoke-after-claim and restart
  checks remain. All network output was owned loopback, with no physical target.
  The QA executable precedes the unrelated recording repairs; the final normal
  build must include both tranches. Its main/output-source hashes still match.
- Explicit real H.264/AAC encoding passed after the EOF fix: one selected ignored
  test ran, preserving the old destination until publication, checking audio and
  video streams and decoding all 30 submitted frames. It used the installed
  Gyan FFmpeg/FFprobe 9.0.1 solely as a local test tool, not a redistributed runtime
  or acceptance of the delivered bundled encoder. The initial SDK path contained
  runtime DLLs only; the first missing-tool failure did not reach encoding.
- [Final release aggregate](artifacts/output-admission-ci-20261011/syndocal-ci-release-recording-final-20261011.txt):
  `pnpm --dir app run check:release` passed with exit 0 after both recording
  repairs. This validates metadata and source contracts, not release acceptance.
- [Normal no-bundle build](artifacts/output-admission-ci-20261011/syndocal-ci-admission-normal-build-20261011.txt)
  passed with exact MSVC pin/PATH-first verification; release compilation took
  3m02s. [Normal native window](artifacts/output-admission-ci-20261011/normal-native-window.json)
  proves one visible, responsive, maximized `Syndocal` window from the exact owned
  executable and rejected unauthenticated backend reads. This build contains
  both output admission and recording repairs. The owned candidate was closed
  gracefully afterward. No UI actions, Control geometry or physical output were
  claimed from this backend check.

- [Final Windows workspace and encoder summary](artifacts/output-admission-ci-20261011/workspace-final-summary.txt):
  `cargo test --workspace --locked -- --test-threads=1` passed with exit 0:
  3,862 passed, zero failed, 47 ignored. The explicit real-encoder test adds one
  pass (3,863 combined). The summary retains the original raw-log path and SHA256.
  Both new regressions ran; Syndocal's bin alone passed 2,041 with 21 ignored.
  Existing cancellation tests still reject blocked/inherited partial writes and
  retain process ownership. Ignored hardware/desktop/long-soak gates are not
  promoted; subprocess helpers are selected only by their owning parent tests.
- After the final completion-flow edit, the unchanged ledger checker passed
  (23 Open + 8 Deferred + 27 Complete = 58), release metadata passed again, and
  `git diff --check`/Node syntax checks passed.

Windows default Rust compiler warnings: baseline 0, current 0, delta 0. The
normal/QA Vite builds each retain the baseline single large-chunk warning
(1 → 1, delta 0); no warning threshold or compiler allowlist was changed.
No current Linux/macOS compiler-warning measurement or independent review is
claimed. The base hosted CI failure is preserved separately; new hosted results
must be read against the pushed revision.

## Boundaries

The primary checkout's unrelated dirty work is preserved. No subagent or Computer
Use was used. The delivered alpha.71 Windows ZIP and Apple Silicon DMG remain
immutable; this internal repair is not included in those delivered files.
Actual USB-DMX/fixture output, macOS runtime and clean-machine acceptance remain
external gates. No new distributed artifact or product version was created.
