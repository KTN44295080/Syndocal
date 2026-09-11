# Selected main integration candidate - 2026-09-08

Status: PROMOTED to main on 2026-09-08. Final frontend validation and
selected-tree native build/IPC acceptance passed. Main is recorded below.
Candidate branch: `chatgpt/core-integration-candidate-20260908`.

## Selected scope

The user authorized main integration only when the changes appear sound.
Six scoped commits were selected from main: original `d317f13`, `3809edc`,
`269c232`, `ab8e670`, `8a8e530`, and `3627811`. They cover canonical own-property
admission, bounded frontend/native thumbnail work, cooperative render cancellation,
libav seek/catch-up regressions, and an ignored virtual-MIDI test.
The unfinished UI ticket/Channel cancellation (`5fc63f1`) is NOT included.
Neither the unintegrated Mac validator nor standalone report publisher is included.
The original 515 native / 456 frontend commands and 47 canonical operations remain.
Runtime admission policy/count/hash, UI layout, ASIO/NDI and versions are unchanged.
Only stale inventory test totals were reconciled to current exact source sets.

## Results on the selected Rust tree

All Cargo commands used the maintained MSVC 14.44.35207 linker/PATH-first checks,
release mode, locked dependencies and serial tests. No hardware tests were enabled.
- Control-plane: 30 passed / 0 failed / 0 ignored.
- Thumbnail: 17 passed / 0 failed / 0 ignored.
- Video with libav: 193 passed / 0 failed / 8 ignored.
- Ordinary video: 180 passed / 0 failed / 3 ignored.
- I/O: 180 passed / 0 failed / 3 ignored; the new virtual-MIDI case stayed ignored.

The five Rust logs contain zero warning diagnostics. A pre-change native warning
baseline was not rerun; no platform-wide warning delta is claimed. Later changes
were confined to frontend retry/checker code and do not alter this tested Rust tree.
Before the final review fixes, TypeScript, check:release, the 456-command frontend
inventory, and the wrapper (243 assertions / 27 hostile fixtures) passed.

## Review findings and unverified final fixes

One read-only Luna/Codex review received the selected inline source diff; it did
not run tests or modify files. Its concerns included delimiter parsing in the
new offline inventory checker and permanent missing thumbnails after transient
native busy/renderer-wait failures. Missing test files were a review-input omission,
not missing source: the compiled native tests above ran from the selected tree.
The unchanged lack of GUI cancellation remains outside this subset's claims.

The candidate now parses only standalone handler-closing lines, rejecting comments
with embedded delimiters. It passed the exact 515-route hash and 18 negative fixtures.
A focused retry module performs at most one serial retry after 100ms, only for two
exact native transient errors and only while its batch remains current. An explicit
Load Thumbnails action also retries missing entries while retaining valid cache hits.
Existing controller regressions passed after this implementation was added.
Additional bounded-retry, retirement, manual-recovery and cache-reuse cases were
then added to the checker, but the final check:media-thumbnails / tsc / check:release
execution request was service-denied before execution. Those new cases are NOT
recorded as passing. The amended code has not received a second independent review.
Therefore the candidate is saved without promotion, not declared ready for main.

## Baseline debt and native boundaries

The untouched main worktree independently reproduced the existing failures:
`check:frontend-command-routing` expects 450 frontend commands but main has 456;
`check-vj-media-import-access` cannot find its old clip-grid branch marker.
A count-only repair exposed further old facade assumptions, so that incomplete
checker patch was not included. Both baseline failures remain recorded as failures.
A broader source-inspection operation was also service-denied; it is not evidence.

An earlier probe in `target/qa/integration-native-20260908-01/` passed real-WebView
negative paths for the EXCLUDED ticket candidate EXE, hash
`4694427CB3E163000BC6E093384521C77E81076340E76E5700A0941F220675F9`.
Both lanes delivered one ticket; cancel/replay returned false after missing-ID
rejection, and missing-Channel calls were rejected. Its own application and debug
listener exited. This is not running-decoder-stop or selected-tree acceptance.
The separate `integration-native-20260908-final` probe is prepared but NOT run;
its hash placeholder intentionally prevents accidental use before a fresh build.

## Resume boundary

First run the final retry/controller, TypeScript, release and inventory checks;
review any failures without weakening assertions. Re-review the amended diff.
Then build the exact selected tree with the maintained Windows no-bundle wrapper,
record its new executable hash, and run the selected-tree native IPC probe.
Only after those gates and the pending-diff review should main be fast-forwarded.
Evidence is retained under `target/qa/main-integration-20260908/` (Rust summaries,
frontend/baseline logs and independent review). Seven old Mac files retain their
starting hashes and remain outside the commit. No installer, release, device or
venue acceptance, final native build, or main update is claimed by this checkpoint.

## 2026-09-08 final candidate validation

The selected tree remained at `d5f64fd8e79974a4b5fdafcb49165c74a6372c05` after
the fresh source review; no product source was edited during this validation.
The thumbnail retry, latest-batch/controller, native-worker ownership, and strict
handler-inventory changes were re-reviewed against the source and their focused
regressions. The review found no blocker. The JavaScript generation fence still
retires stale results without claiming to force-stop an already-running native
worker; worker ownership remains held until the blocking operation returns.

Fresh checks, all exit 0:

- `pnpm --dir app run check:media-thumbnails`: focused controller checks passed,
  including bounded transient retry, terminal failure, stale retry retirement,
  explicit recovery, and valid-cache reuse.
- `pnpm --dir app exec tsc --noEmit`.
- `pnpm --dir app run check:release`: exact 515-command native inventory,
  SHA-256 `6414db9fd02f7147ecc6b7503607bdb77e07a871e02f5b0c1cdd7742a48e37b5`,
  and 18 negative fixtures rejected.
- `pnpm --dir app run check:frontend-invokes`: exact 456-command inventory.
- `pnpm --dir app run check:tauri-build-wrapper`: 243 assertions and 27 hostile
  mutation fixtures.
- `git diff --check`: no whitespace errors. The displayed LF/CRLF notices were
  for unrelated uncommitted Mac work, which remains outside this change.

The no-bundle executable at `target/release/syndocal.exe` was matched to source
HEAD and the recorded artifact: version `1.2.0-alpha.69`, 64,528,896 bytes,
SHA-256 `E042B928F7B0918A31118D19E0DBA5E9FE0DCE990F28A9F0BF9FD55368D47CED`.
The new evidence is under
`target/qa/integration-native-20260908-final-run-01/`. Its real-WebView probe
read the snapshot, accepted omitted `media_assets`, rejected invalid absent-ID
layer and asset requests with their specific missing-resource errors twice each,
and rejected the excluded cancellation command. It observed one responsive,
maximized `Syndocal` window; it issued zero project mutations and zero physical
output commands. The owned application exited and the loopback debug listener
count returned to zero.

This is selected-scope integration evidence only. GUI cancellation, successful
thumbnail decoding, decoder-stop latency, Mac validation, ASIO/NDI/MIDI hardware,
physical output, signing/notarization, release publication, and venue acceptance
remain outside this checkpoint. The known main-baseline checker failures remain
unchanged and are not covered by this candidate result.

Final status: selected candidate was fast-forwarded to main at
`55ef1d58eb19b36224be1dcc7453ff16b28fd47f`. The seven
uncommitted Mac files and the unrelated `check-frontend-command-routing.mjs`
working-tree state remain unowned and must not be staged.

## Main integration record

Remote `main` was re-fetched immediately before integration at
`00e878500f9c0613871109aa612c435d6393be06`. The candidate was an ancestor-safe
fast-forward source; no merge commit, force-push, unrelated branch, or uncommitted
Mac work was included. The integration commit was pushed normally and verified as
remote `main` at `55ef1d58eb19b36224be1dcc7453ff16b28fd47f`.

## 2026-09-08 checker contract repair

The checker-repair lane started from main `3eef03d71f8c0500dd7717b2c8296aa2ee6fef6c`.
Only the following two checker files were changed; product source, native source,
unfinished UI/Channel cancellation work, and the separate Mac worktree were not
edited or staged:

- `app/scripts/check-frontend-command-routing.mjs`
- `app/scripts/check-vj-media-import-access.mjs`

The routing checker now matches the current `456` frontend routes and `133/31`
renderer/server mutation classification, while keeping the transaction ticket,
owner registration barrier, finite route classes, and fail-closed raw transport
boundaries. The three current transport exceptions are exact and scoped: the
canonical agent bridge command cast, the injected agent bridge mount, and the
transaction controller's `tauriInvoke(command, args)` adapter. The old checker
assumption that App itself owns `projectTransactionId/expectedEpoch/ownerId` was
replaced with an assertion that `projectTransactionMutationController.ts` owns
that envelope, dispatch, commit/cancel, and recovery boundary.

The VJ checker now normalizes LF/CRLF source input, checks the current
`createLatestThumbnailBatch` and `readThumbnailWithRetry` contracts, and invokes
the executable media-thumbnail controller regression instead of maintaining a
duplicated stale authority model. It still requires unauthorized empty
projections and rejects source reads before explicit authorization.

Fresh checker-repair evidence is retained under
`target/qa/checker-contract-repair-20260908-02/`. All requested commands exited
zero: routing, VJ media-import access, project transaction controller, media
thumbnails, frontend invokes, TypeScript, check:release, completion ledger,
Q1-Q4 ledger, and `git diff --check`. The negative paths cover broad/raw route
casts and retired routes, missing authorization before thumbnail reads, stale
authority/batch and retry results, reset/dispose retirement, bounded transient
retry and explicit recovery, transaction owner/epoch ticketing, abort before
raw mutation, cancel-on-definitive-failure, no-replay after not-published
recovery, commit-on-published recovery, and hold-on-indeterminate/unconfirmed
publication. The VJ checker continues to state that it is source/policy
evidence, not browser-mounted or native-rendered reachability evidence.

This checker-only repair does not claim a new native build, EXE launch, device
acceptance, physical output, Mac validation, signing/notarization, release
publication, or venue acceptance. Those boundaries remain as recorded above.

## 2026-09-08 follow-up review and icon update

An independent review found and the checker now rejects two additional
fail-open fixtures: an operator admission guard disabled by an unconditional
`false` branch, and an admission guard nested under an unreachable condition.
The direct guard is checked from the TypeScript AST as a statement of the
central `invoke` facade. The retired raw-route fixture is also connected to the
production rejection branch, so removing that `errors.push` path fails the
checker contract. `pnpm --dir app run check:frontend-command-routing` passed
after this repair with 133 renderer mutations, 31 server-authoritative
mutations, 28 raw dispatches, and 463 facade dispatches.

The application icon was regenerated from the deterministic SVG mark with the
unwanted rounded-square background removed. The tracked SVG and generated PNG,
ICO, and ICNS assets now represent a solid black square with the centered white
geometric mark.

The final no-bundle executable was rebuilt from this working tree and passed a
release-native probe. It is version `1.2.0-alpha.69`, 64,532,992 bytes, with
SHA-256
`6A37854E6B8926CEA63D07C129C4F943C491931082BB2EE44A5C59FE1286C517`.
The probe observed one responsive/maximized `Syndocal` window, verified the
owned loopback listener, read Standby ownership and the initial snapshot,
rejected missing asset/layer thumbnail IPC with the expected errors, issued no
physical output, and returned the exact application/listener counts to zero.

This evidence does not claim the completed missing-file/re-fetch/recovery unit
again, physical hardware or venue acceptance, Mac real-machine validation,
ASIO/NDI/MIDI acceptance, signing/notarization, or release publication.

## 2026-09-10 current-main checker revalidation

The current `main` source at `026ec16a818226fd6c47d57610d1b146d5acd166`
was rechecked without modifying product source. This revalidation covers the
existing permission/approval/owner-authority and stale-or-old-result rejection
contracts; it does not change their scope or claim external-client or native
hardware acceptance.

| Check | Result |
| --- | --- |
| `node app/scripts/check-backend-operator-contract.mjs` | PASS — 516 commands, 334 literal frontend calls, 133 transactional mutations |
| `node app/scripts/check-agent-bridge.mjs` | PASS — 11 groups; canonical bridge parity and fail-closed rejection paths |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer mutations, 31 server-authoritative mutations, 28 raw dispatches, 464 facade dispatches |
| `pnpm.cmd --dir app run check:project-transaction` | PASS — production contract and deterministic authority checks |
| `node app/scripts/check-completion-ledger.mjs` | PASS — 50 Open + 8 Deferred rows preserved |
| `node app/scripts/check-q1-q4-ledger.mjs` | PASS — 32 Q1 rows, 29/29 Q0 domains, 10/10 source contracts, 58/58 markers |

No physical output, external service, device, or release artifact was started
by this checker-only revalidation. The real-file thumbnail missing -> Retry ->
recovery trial was not rerun. The ledger remains `50 Open + 8 Deferred`; the
remaining external, physical, Mac, signing/publication, and product-wide
acceptance boundaries remain unclaimed.

## 2026-09-10 current-main release static gate

Current `main` source HEAD `9d086420268417a8ba69130469cd076c4def561f` was
rechecked without changing product source. The full static release gate
completed with exit `0`:

```text
Native Tauri admission inventory exact: 516 commands; 18 negative fixtures rejected
check:media-thumbnails: PASS
check:native-thumbnail-request: PASS
check:snapshot-live-publication: PASS
check:agent-bridge: PASS
check:output-control-runtime: PASS
check:safety-blackout-runtime: PASS
check:asio-packaging: PASS (169 assertions)
check:asio-v3-contract: PASS (22 assertions)
check:timeline-cue-audio: PASS
check:timeline-audio-output-bus: PASS (11 assertions)
check:timeline-loop-runtime: PASS
check:snapshot-runtime-watermark: PASS
check:project-open-bootstrap: PASS
check:video-output-routing-runtime: PASS
check:video-output-window-observation: PASS
check:camera-input: PASS
release metadata: PASS — Syndocal 1.2.0-alpha.69
```

This is source/static release evidence only. It does not close the 50 Open +
8 Deferred ledger, real-file MP4 missing-file recovery, physical output,
external-client, Mac real-device, signing, publication, or product-wide
acceptance boundaries. No Actions result was awaited or used as a success
substitute.

## 2026-09-10 current-main workspace release test gate

Current `main` source HEAD `3faf8d6086408a91ef629e75d2e39b28ed6e2fe3` was
tested with the exact Windows MSVC `14.44.35207` x64 linker pinned and returned
first by `where.exe link.exe`.

The default local release workspace command was run without changing any source
or assertion:

```text
cargo test --workspace --release --locked -j 1
```

It exited `1` after the audio crate passed; the Engine binary reported
`1046 passed / 15 failed / 14 ignored`. All 15 failures were the existing 44 Hz
release performance percentile gates, and the observed p95 values were above
their existing limits only under the default concurrent Rust test-thread mode.
The failure is retained as observed evidence; no threshold or assertion was
removed or weakened.

Each of the 15 failed filters was then rerun independently with
`--test-threads=1`; all 15 passed. The full deterministic workspace command
also passed:

```text
cargo test --workspace --release --locked -j 1 --quiet -- --test-threads=1
```

The serialized run exited `0` with `3533 passed / 0 failed / 46 ignored` across
non-empty test binaries. This identifies host contention in the default local
benchmark execution mode, not a deterministic product regression. The checked
in GitHub Actions workflow already uses `--test-threads=1` for the workspace
and video tests, so no workflow or product change was made for this observation.

This is local source/build/test evidence only. It does not claim native window,
physical output, external-client, Mac, signing, publication, or product-wide
completion. The Actions runs for this SHA were observed but not awaited or used
as a local test substitute.

## 2026-09-10 release checker signature-fixture repair

The existing `check:release:self-test` exposed a real fixture regression before
any production-code change: the Tauri-signer public key and signature fixture
had been changed while the signed fixture payload remained unchanged. The
cryptographic gate correctly rejected the mismatched `windows-x86_64` payload;
the verifier, signature assertions, and rejection paths were not weakened.

The owned repair restores the public-key/signature pair that matches the
tracked fixture payload. Only these fixture files changed:

- `app/scripts/fixtures/release/tauri-signer-public.key`
- `app/scripts/fixtures/release/Syndocal_1.2.0-rc.1_tauri-signer-fixture.bin.sig`

The exact pinned FFmpeg staging source
`C:\SyndocalQA\ffmpeg-8.1.2-full_build-shared` was checked against all seven
inventory byte sizes and SHA-256 values before staging; the staging directory
was not added to Git.

| Check | Result |
| --- | --- |
| `pnpm --dir app install --frozen-lockfile` | PASS; lockfile unchanged |
| `pnpm --dir app run check:release:self-test` | PASS — release metadata 128 assertion groups, ASIO 169, Windows candidate extractor 43, materialization 4, Windows artifact 144, strict JSON 130 |
| `pnpm --dir app run check:release` | PASS — native inventory 516, 18 negative fixtures, all chained static gates, release metadata `1.2.0-alpha.69` |
| `git diff --check` | PASS |

The initial cryptographic rejection and the missing-staged-runtime prerequisite
remain part of the local execution history; neither was converted into a
success by changing an assertion. This checkpoint is checker-fixture evidence
only. It does not claim a new EXE, native-window, physical-output,
external-client, Mac, signing, publication, or product-wide completion.

## 2026-09-10 Windows checkout line-ending determinism repair

The release self-test was rerun independently after the checkpoint above. On
this Windows checkout, the signed `.bin` fixture was present as CRLF bytes
while its tracked Git blob and signature were LF bytes. The cryptographic
checker correctly rejected those different payload bytes. The public key and
signature verifier were not weakened and no rejection assertion was removed.

The narrow repair makes the three release signer fixtures explicit `text
eol=lf` entries in `.gitattributes`, then verifies that the working-tree bytes
match the signed LF payload. No production source, release metadata, or
acceptance assertion changed.

The self-test now asserts the exact canonical fixture payload bytes before
cryptographic verification, so a CRLF-converted or otherwise stale payload is
rejected as a test failure rather than silently becoming a new signature
fixture.

| Check | Result |
| --- | --- |
| `git ls-files --eol` for signer fixtures | PASS — signed payload and signature are `w/lf`; public-key evidence is parsed after canonical trim |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — release metadata 129 assertion groups, ASIO 169, Windows candidate extractor 43, materialization 4, Windows artifact 144, strict JSON 130 |
| `git diff --check` | PASS |

The real-file thumbnail missing -> Retry -> recovery trial remains not run and
this checkpoint does not claim native window, physical output, Mac, signing,
publication, or product-wide completion.

## 2026-09-11 current-main release checker self-test revalidation

The release checker self-tests were rerun at source HEAD
`0097f292f420c8bfd87d3e0a0ad522bc282729e4`. Evidence is preserved under
`target/qa/release-metadata-current-main-20260911-01/`.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — release metadata 137 assertion groups, ASIO 169, video output routing, Windows artifact self-tests, strict JSON 130 |
| `node app/scripts/check-windows-release-artifacts.mjs --self-test` | PASS — candidate extractor 43, materialization 4, artifact self-test 144 assertions |
| `git diff --check` | PASS |

The negative paths remain strict for mismatched signer payloads, line-ending
drift, malformed artifact metadata, invalid candidate materialization, and
duplicate-key JSON. No signature, rejection assertion, or publication path was
weakened.

This is local checker evidence only. It does not claim a signed artifact,
public release, clean-machine install, native-window acceptance, physical
output, external-client, Mac, or product-wide completion.

## 2026-09-11 current-main serialized workspace release revalidation

The current `main` source HEAD `87b0f8855124272bfac2b95a18cad0ff6955a4f4`
was revalidated with the exact Windows MSVC `14.44.35207` x64 linker pinned
and confirmed first by `where.exe link.exe`. The serialized command was:

```text
cargo test --workspace --release --locked -j 1 --quiet -- --test-threads=1
```

It exited `0`. Across the non-empty test binaries the aggregate result was
`3579 passed / 0 failed / 46 ignored`; no assertion or threshold was changed.
The complete output is preserved under
`target/qa/workspace-release-current-main-20260911-01/cargo-test-output.txt`.

This is current-main Windows source/build/test evidence only. It does not
claim native-window, physical-output, external-client, Mac, signing,
publication, clean-machine installation, or product-wide completion.

## 2026-09-11 current-main hosted workspace-command reproduction

The Windows Rust workspace command used by `.github/workflows/cross-platform.yml`
was run locally against the same current-main source without adding Cargo
parallelism changes:

```text
cargo test --workspace --locked -- --test-threads=1
```

The exact MSVC `14.44.35207` x64 linker was initialized and confirmed first by
`where.exe link.exe`. The command exited `0` with the aggregate result
`3579 passed / 0 failed / 46 ignored`. Raw output is preserved under
`target/qa/ci-workspace-current-main-20260911-01/cargo-test-output.txt`.

This revalidates that the single Windows workspace failure observed at older
source `d22b353fb3858ca941230a64e7c9010a7d47ed5a` is not deterministic on the
current main: that run had one `spout_startup_timeout_and_late_constructor_error_share_one_fence`
failure reporting `Output ownership transition is already in progress`, while
the current source and the same workflow command pass. The old failure is
retained as historical evidence; no test assertion or threshold was weakened
and no workflow serialization change was introduced from this observation.

The GitHub Actions run for the current QA-only HEAD is tracked separately and
is not used as a substitute for this local reproduction. This remains local
source/build/test evidence only and does not claim native-window,
physical-output, external-client, Mac, signing, publication, or
product-wide completion.
