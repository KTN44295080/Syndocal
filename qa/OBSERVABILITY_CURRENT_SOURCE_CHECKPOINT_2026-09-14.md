# Observability and Support Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `OBSERVABILITY-SUPPORT-001` (section 6, Open)
- Q1 row: `COV-OBSERVABILITY-001`
- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `9966073ca2b6800289ee4583f0738129a4e4e040`
- Scope: current Windows source checks for generation-stamped status, redacted
  diagnostics, updater identity/configuration, strict release artifacts, and
  project replacement redaction.
- This checkpoint does not close the Flow marker. It records current-source
  software evidence only.

## Implementation change in this checkpoint

The Windows candidate and release-artifact self-tests create junction/reparse
fixtures. Their cleanup previously passed `recursive: true` to `rmSync`, which
can traverse the directory targeted by an alias and leave the self-test without
a deterministic completion boundary. Fixture aliases are now unlinked with a
non-recursive removal; ordinary temporary directories retain their existing
owned, revalidated recursive cleanup. No product runtime or release policy was
changed.

## Verification

The following command was run from the repository root with the exact approved
MSVC 14.44.35207 x64 linker selected by `vcvars64.bat -vcvars_ver=14.44`:

```text
pnpm.cmd --dir app run check:status
pnpm.cmd --dir app run check:release:self-test
pnpm.cmd --dir app run check:bundled-library
node app/scripts/check-windows-release-artifacts.mjs --self-test
pnpm.cmd --dir app run check:strict-json
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 diagnostic_ -- --nocapture --test-threads=1
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 updater_ -- --nocapture --test-threads=1
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_replacement_is_redacted -- --nocapture --test-threads=1
```

Result: exit code 0.

- `check:status`: PASS.
- `check:release:self-test`: PASS, including release metadata, AI0-AI7,
  F1/F2, ASIO packaging, video output routing, Windows candidate extraction
  (43 assertions), verified materialization (4 assertions), Windows release
  artifact checks (144 assertions), and strict JSON (130 assertions).
- `check:bundled-library`: PASS.
- Explicit `check-windows-release-artifacts.mjs --self-test`: PASS, 144
  assertions.
- Explicit `check:strict-json`: PASS, 130 assertions.
- `diagnostic_`: 37 passed, 0 failed, 0 ignored.
- `updater_`: 3 passed, 0 failed, 0 ignored.
- `project_replacement_is_redacted`: 1 passed, 0 failed, 0 ignored.
- First-party warning count observed in this run: 0.
- Reported numeric assertions/groups summed for this evidence record: 967.

## Takeover rerun — 2026-09-14

The complete current-source command set above was rerun after takeover on the
current `16d9f886` tree. The release self-test again completed with exit code
0, including Windows candidate extraction (43 assertions), verified
materialization (4), Windows release-artifact checks (144), and strict JSON
(130). The release-mode focused Rust run again completed with the pinned MSVC
`14.44.35207` x64 linker: `diagnostic_` 37 passed, `updater_` 3 passed, and
`project_replacement_is_redacted` 1 passed, with no failures or ignored tests.
The recheck observed zero first-party warnings and did not contact an external
update endpoint, publish an artifact, or exercise a device.

The updater source continues to fail closed for a missing endpoint/public key,
non-HTTPS endpoint, credentials/query/fragment, invalid base64/public key, and
unknown channel. Release identity JSON contains only endpoint, channel, and a
public-key fingerprint; the public key itself is not serialized. The diagnostic
tests cover bounded fields, duplicate/unsafe ZIP entries, tampering, secrets,
preview-before-destination, atomic publication, and redacted project
replacement.

## Acceptance boundary and next action

The following remain unaccepted and therefore keep the marker Open:

- live signed N→N+1 update and updater failure matrix: wrong channel,
  downgrade, signature/public-key mismatch, corruption, offline, and rollback;
- deployed startup/failure/takeover/recovery/shutdown support drills;
- clean-machine installation and recovery proof;
- signed publication, Authenticode/notarization, and external release endpoint;
- native button-by-button/accessibility, physical device, venue, and soak
  acceptance.

Next action is to run the signed test update matrix on a release-controlled
machine, retain N and N+1 hashes plus the automatic pre-update project backup,
then record operator/support drill evidence. Until those external gates exist,
current-source PASS must not be reported as support or release acceptance.

## Takeover continuation — current HEAD release/diagnostic recheck — 2026-09-14

The complete source command set was rerun against HEAD `8300943a`. The
release self-test passed with release metadata `137` assertion groups, Windows
candidate extraction `43`, verified materialization `4`, Windows release
artifact checks `144`, and strict JSON `130`. The bundled-library and status
checks also passed.

With the exact Build Tools MSVC `14.44.35207` x64 linker, the focused release
Rust tests passed:

```text
diagnostic_: 37 passed / 0 failed / 0 ignored
updater_: 3 passed / 0 failed / 0 ignored
project_replacement_is_redacted: 1 passed / 0 failed / 0 ignored
```

The run completed with zero first-party warnings. It contacted no update
endpoint, published no artifact, and opened no device. The result confirms
the current-source diagnostics/updater/release negative contracts only;
`OBSERVABILITY-SUPPORT-001` remains `Open` for signed update, clean-machine,
deployment/support-drill, signing/publication, and physical/native acceptance.

## Current HEAD release self-test recheck — 2026-09-14

At current HEAD `70a4e4e5` (product source unchanged since the checker fix at
`dce7dc48`), `pnpm.cmd --dir app run check:release:self-test` completed with
exit `0`. The run passed release metadata (`137` assertion groups), AI0/AI1/
AI2/AI4/AI5/AI6/AI7 self-tests, F1/F2 self-tests, ASIO packaging (`169`), video
output routing, Windows candidate extraction (`43`), verified materialization
(`4`), Windows release artifact checks (`144`), and strict JSON (`130`).

This is a current-source regression result only. It contacted no update
endpoint, created no signed/publication artifact, installed no clean machine,
and opened no device. The signed N to N+1 update/failure matrix,
deployment/support drills, signing/publication, clean-machine recovery, and
native/physical acceptance remain unproven, so `OBSERVABILITY-SUPPORT-001`
stays `Open`.

## Takeover continuation — current-source observability/release recheck — 2026-09-14

At current source HEAD `80a7a001`, the focused Node contracts were rerun:

```text
pnpm.cmd --dir app run check:status
status model helpers ok

pnpm.cmd --dir app run check:bundled-library
bundled fixture library failure/retry checks passed

pnpm.cmd --dir app run check:release:self-test
release metadata self-tests ok: 137 assertion groups
AI0/AI1/AI2/AI4/AI5/AI6/AI7, F1/F2 self-tests: PASS
ASIO packaging: 169 assertions
video output routing: PASS
Windows candidate extractor: 43 assertions
verified materialization: 4 assertions
Windows release artifact: 144 assertions
strict JSON: 130 assertions
```

All three commands exited `0`. The artifact self-test took longer than the
focused checks but reached its successful terminal result; no process was
force-stopped and no release artifact was published. This confirms the
current-source status, bundled-library, diagnostics/release-negative, and
artifact-materialization contracts only.

No signed N-to-N+1 update, external endpoint, clean-machine install, support
drill, signing/publication, or physical/native acceptance was performed.
`OBSERVABILITY-SUPPORT-001` remains `Open` pending those external release and
support conditions.

## Current HEAD observability-contract recheck — 2026-09-14

At current HEAD `0c57c34f`, the status, release, diagnostics, updater, and
redaction checks were rerun. `check:status` passed; the complete
`check:release:self-test` passed release metadata (137 groups), AI0/AI1/
AI2/AI4/AI5/AI6/AI7 self-tests, F1/F2 self-tests, ASIO packaging (169
assertions), video output routing, Windows candidate extraction (43),
verified materialization (4), Windows release-artifact validation (144), and
strict JSON (130). The bundled-library runtime and the standalone Windows
release-artifact self-test also exited `0`.

With the exact MSVC `14.44.35207` Build Tools linker initialized and printed
first in `where.exe link.exe`, focused release Rust tests passed:

- `diagnostic_`: 37 passed, 0 failed, 0 ignored;
- `updater_`: 3 passed, 0 failed, 0 ignored;
- `project_replacement_is_redacted`: 1 passed, 0 failed, 0 ignored.

The checks cover bounded diagnostic capture/publication, secret/path/ZIP
redaction, updater identity and fail-closed configuration, project replacement
redaction, and release artifact invariants. No update endpoint, signed
publication, clean-machine install, external device, or support drill was
performed. `OBSERVABILITY-SUPPORT-001` remains `Open` for the signed
N-to-N+1 matrix, deployment/recovery drills, signing/publication, and
native/physical acceptance.

## Takeover continuation — current-source observability recheck after Video repair — 2026-09-14

At current source HEAD `9da6c1ff`, the current-source observability and release
contracts were rerun from the repository checkout. All five commands exited
`0`:

```text
pnpm.cmd run check:status
status model helpers ok

pnpm.cmd run check:bundled-library
bundled fixture library failure/retry checks passed

pnpm.cmd run check:release:self-test
release metadata: 137 assertion groups
ASIO packaging: 169 assertions
Windows candidate extractor: 43 assertions
verified materialization: 4 assertions
Windows release artifact: 144 assertions
strict JSON: 130 assertions
AI0/AI1/AI2/AI4/AI5/AI6/AI7, F1/F2, and video output routing: PASS

node scripts/check-windows-release-artifacts.mjs --self-test
Windows candidate extractor: 43 assertions
verified materialization: 4 assertions

pnpm.cmd run check:strict-json
strict JSON duplicate-key self-test passed: 130 assertions
```

The complete release self-test and the standalone artifact self-test reached
successful terminal results without force-stopping a process. These checks
reconfirm current-source diagnostics/status, redaction, updater/release-negative,
bundled-library, artifact-materialization, and strict-JSON contracts only. No
update endpoint was contacted, no signed/publication artifact was created, and
no clean-machine installation, deployment/support drill, external device, or
physical/native acceptance was performed. The signed N-to-N+1 update/failure
matrix, deployed startup/takeover/recovery/shutdown drills, clean-machine
recovery, signing/publication, and native/physical acceptance remain unproven;
therefore `OBSERVABILITY-SUPPORT-001` remains `Open`.

## Takeover continuation — bounded hostile diagnostic archive corpus — 2026-09-14

At current source HEAD `d37aad9d`, the diagnostic ZIP validator gained a
deterministic, memory-only hostile corpus test. It exercises 512 bounded cases:
one canonical package, one archive-size overflow, 128 random byte inputs, 128
truncated canonical packages, and 254 single-byte mutations. Every case is
limited to at most `MAX_ARCHIVE_BYTES + 1` (`163841`) bytes, and validation is
wrapped in `catch_unwind` so a panic is an explicit failure rather than an
uncaught test abort.

With the exact MSVC `14.44.35207` Build Tools linker initialized and printed
first by `where.exe link.exe`, the focused release command completed with exit
`0`:

```text
diagnostic_: 27 passed, 0 failed, 0 ignored, 1901 filtered out
diagnostic hostile archive corpus: 512 cases, 511 rejected, 0 panics, max_bytes=163841
```

Targeted Rust formatting and `git diff --check` passed. The post-change
`windows-native-release` warning ratchet also passed with output marker coverage
`2/2`, warning-shaped output `none`, baseline/current totals `0/0`, first-party
warnings `0/0`, and identity removals `0`.

This is a current-source validator robustness result only. No update endpoint,
signed publication, clean-machine installation, deployment/support drill,
external device, or physical/native acceptance was performed. The signed
N-to-N+1 update/failure matrix, deployment/recovery support drills,
clean-machine recovery, signing/publication, and native/physical acceptance
remain unproven; therefore `OBSERVABILITY-SUPPORT-001` remains `Open`.

## Continuation — current HEAD observability/release recheck — 2026-09-15

At current source HEAD `902ac03b`, after the Remote authority and high-DPI UI
changes, the current-source observability and release checks were rerun. The
Node command set exited `0`:

```text
pnpm.cmd --dir app run check:status
status model helpers ok

pnpm.cmd --dir app run check:release:self-test
release metadata: 137 assertion groups
AI0/AI1/AI2/AI4/AI5/AI6/AI7, F1/F2 self-tests: PASS
ASIO packaging: 169 assertions
video output routing: PASS
Windows candidate extractor: 43 assertions
verified materialization: 4 assertions
Windows release artifact: 144 assertions
strict JSON: 130 assertions

pnpm.cmd --dir app run check:bundled-library
bundled fixture library failure/retry checks passed

node app/scripts/check-windows-release-artifacts.mjs --self-test
Windows candidate extractor: 43 assertions
verified materialization: 4 assertions
Windows release artifact self-test passed: 144 assertions

pnpm.cmd --dir app run check:strict-json
strict JSON duplicate-key self-test passed: 130 assertions
```

With `vcvars64.bat -vcvars_ver=14.44`, the pinned Build Tools linker was
printed and was first in `where.exe link.exe`:

```text
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
```

The release-mode Tauri tests then completed with exit `0`:

```text
diagnostic_: 38 passed, 0 failed, 0 ignored
updater_: 3 passed, 0 failed, 0 ignored
project_replacement_is_redacted: 1 passed, 0 failed, 0 ignored
```

The run reconfirms current-source status truth, bounded/redacted diagnostic
capture and publication, updater identity/configuration fail-closed behavior,
release artifact negative checks, bundled-library retry handling, strict JSON,
and project replacement redaction. The exact native test command used the
approved linker; no warning lines were emitted by the focused Rust invocation.

This remains current-source software evidence only. No update endpoint,
signed N-to-N+1 publication, clean-machine installation, deployed support
drill, external client, physical device, or native accessibility/interaction
acceptance was performed. `OBSERVABILITY-SUPPORT-001` therefore remains
`Open` for the signed update/failure matrix, deployment/recovery drills,
clean-machine recovery, signing/publication, and native/physical acceptance.

## Continuation — current-source observability/release recheck — 2026-09-18

At current source HEAD after the migration runner checkpoint, the exact MSVC
`14.44.35207` x64 environment was initialized and `where.exe link.exe`
resolved the pinned linker first. The complete current-source Node gate and
focused release Rust filters exited `0`:

```text
check:status: PASS
check:release:self-test: PASS (release metadata 137; ASIO 169; candidate extractor 43; materialization 4; Windows release artifact 144; strict JSON 130; AI0/AI1/AI2/AI4/AI5/AI6/AI7/F1/F2 and video routing)
check:bundled-library: PASS
check-windows-release-artifacts --self-test: PASS (43 extractor + 4 materialization + 144 release artifact assertions)
check:strict-json: PASS (130 assertions)
diagnostic_: 38 passed; 0 failed; 0 ignored
updater_: 3 passed; 0 failed; 0 ignored
project_replacement_is_redacted: 1 passed; 0 failed; 0 ignored
```

The focused Rust run also recorded the deterministic hostile diagnostic
archive corpus: 512 cases, 511 rejected, 0 panics, maximum 163841 bytes.
This confirms current-source status truth, redacted bounded diagnostics,
updater identity and fail-closed configuration, release-negative artifact
checks, bundled-library retry behavior, strict JSON, and project replacement
redaction.

No update endpoint, signed N-to-N+1 publication, clean-machine installation,
deployed support/startup/takeover/recovery/shutdown drill, external client,
physical device, or native accessibility/interaction acceptance was performed.
`OBSERVABILITY-SUPPORT-001` remains `Open` for signed update/failure behavior,
deployment/support drills, clean-machine recovery, signing/publication, and
native/physical acceptance.
