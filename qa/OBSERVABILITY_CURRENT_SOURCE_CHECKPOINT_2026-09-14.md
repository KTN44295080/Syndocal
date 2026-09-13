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
