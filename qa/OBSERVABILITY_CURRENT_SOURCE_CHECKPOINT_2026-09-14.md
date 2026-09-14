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
