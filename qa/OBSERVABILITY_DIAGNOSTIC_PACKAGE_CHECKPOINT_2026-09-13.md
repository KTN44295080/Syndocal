# Observability diagnostic package checkpoint — 2026-09-13

## Result

This checkpoint records the bounded diagnostic-export implementation inherited
after `46f46db3` and validated on the current Windows checkout. The implementation
now constructs a four-payload, schema-projected diagnostic package in memory,
adds a fixed integrity manifest, previews the sanitized package before the
destination dialog, and publishes only the exact approved bytes through a
create-new staging file and atomic replacement path.

This is evidence for the software portion of `COV-OBSERVABILITY-001`. It does
not close `OBSERVABILITY-SUPPORT-001` because updater failure behavior, deployed
support drills, clean-machine installation, signed/publication acceptance, and
the complete operator runbook matrix remain open.

## Inherited and current work

- Inherited uncommitted files at takeover: `main.rs`,
  `diagnostic_package.rs`, `diagnostic_package_publication.rs`,
  `diagnostic_export_workflow.rs`, and their focused test modules.
- Current integration repair: the runtime serializer accepts the production
  `EngineTelemetryReport` and `VideoRuntimeStatus` types through the bounded
  `Serialize` path; test-only fixture recursion is bounded without changing
  the product release setting.
- No product version bump, external service call, physical-device output, or
  distribution publication was performed.

## Validation evidence

All commands below ran from the repository root on 2026-09-13 and returned
exit code 0 unless noted otherwise.

### Focused Rust diagnostics

```text
cmd.exe /d /s /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" -vcvars_ver=14.44 && set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe" && where link.exe && cargo test --manifest-path app\src-tauri\Cargo.toml --release --locked -j 1 diagnostic -- --nocapture --test-threads=1'
 where link.exe: C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
 44 selected tests: 43 passed, 0 failed, 1 ignored
 ignored boundary: inherited stderr subprocess helper
 panic text emitted by two intentional panic/recovery fixtures was caught by tests
 finished in 0.43s after the Release compile
 first-party Rust compiler warnings: none observed in this command
 no device, network, updater endpoint, or external process was used
```

The focused suite covers redaction and schema projection, byte/depth/node
bounds, deterministic ZIP/integrity output, malformed/duplicate/unsafe ZIP
rejection, tamper detection, preview validation, publication staging failure
preservation, collision and alias rejection, and the preview/confirm/destination
workflow including no-recapture and no-write cancellation cases.

### Static contracts

```text
pnpm.cmd --dir app run check:status                         PASS
pnpm.cmd --dir app run check:bundled-library                PASS
pnpm.cmd --dir app run check:strict-json                    PASS (130 assertions)
pnpm.cmd --dir app run check:release:self-test              PASS
pnpm.cmd --dir app run check:output-ownership               PASS
pnpm.cmd --dir app run check:release:static                 PASS
```

The aggregate release-static run also passed completion/Q1-Q4 ledger checks,
AI0-AI7 source contracts, project/media/output/timeline/video/audio/ASIO
contracts, and the camera/input checks. Before this checkpoint it reported
`23 Open + 8 Deferred + 27 Complete` completion rows and `52` Q4 evidence
records; this checkpoint adds one evidence record without changing marker
status.

### Windows native build and process smoke

The prescribed Build Tools 14.44 environment was initialized and the exact
absolute linker was first in `where.exe link.exe`. The maintained wrapper
reported that the exact checkout executable was not running before build.

```text
pnpm.cmd --dir app tauri build --no-bundle
exit code: 0
artifact: target/release/syndocal.exe
SHA-256: 2F66AEE1E77ACFE11118E617E5E2E80CE1D6C37D3DB504BF4637537492B4B15A
```

The exact artifact was launched once after build. Process inspection by the
resolved absolute executable path found exactly one process/window:

```text
BeforeExactCount: 0
ExactCount: 1
Title: Syndocal
Responding: true
MainWindowHandle: 22675934
```

The exact process was then terminated by that same absolute path. No other
`syndocal.exe` path and no Daslight/other checkout process was touched. The
build emitted one existing Vite chunk-size warning; this is not a Rust
first-party compiler warning and no new warning suppression was added.

## Acceptance boundary

Accepted by this checkpoint:

- current-source diagnostic redaction and schema projection;
- bounded deterministic package and integrity validation;
- preview-before-destination control flow;
- exact approved-byte publication and atomic staging behavior;
- focused Rust tests, static contracts, fixed-MSVC native build, and exact
  artifact process smoke.

Still open and not claimed here:

- updater wrong-channel, downgrade, signature, corruption, offline, rollback,
  and live signed-endpoint failure matrix;
- deployed support/startup/failure/takeover/recovery/shutdown runbook drills;
- clean-machine installation and support escalation proof;
- signed/publication/distribution acceptance;
- native button-by-button/accessibility matrix, physical devices, external
  clients, venue/soak, and product-wide completion.

Therefore `COV-OBSERVABILITY-001` remains `In progress` and
`OBSERVABILITY-SUPPORT-001` remains `Open`.
