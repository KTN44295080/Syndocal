# Observability and support software revalidation — 2026-09-12

This checkpoint revalidates current-source status truth, diagnostic archive
redaction/contents, updater identity rejection, and release-artifact negative
boundaries. Product source was not changed.

- Source under test: `main` at `acd4c0a881303f2ba67d152cab9a4b6634001d25`.
- Native tests used the exact Build Tools MSVC `14.44.35207` x64 linker with
  the required absolute Cargo pin and PATH-first `where.exe link.exe` check.
- Evidence log: `target/qa/observability-current-main-20260912/observability-tests.log`.

## Current-source results

The following checks passed:

- `check:status`;
- `check:release:self-test` — release metadata 137 assertion groups,
  ASIO packaging 169 assertions, Windows artifact self-test 144 assertions,
  candidate extractor 43 assertions plus materialization 4 assertions, and
  strict JSON 130 assertions;
- `check:bundled-library` — bundled fixture library failure/retry behavior;
- direct Windows release-artifact self-test — 43 extractor plus 4 materialization
  assertions;
- `check:strict-json` — 130 duplicate-key rejection assertions.

Focused native tests passed with the same environment:

```text
diagnostic_:                    6 passed; 0 failed; 0 ignored
updater_:                       3 passed; 0 failed; 0 ignored
project_replacement_is_redacted: 1 passed; 0 failed; 0 ignored
```

These tests cover required diagnostic archive entries/crash reports, narrow
telemetry-reader ownership, bounded diagnostic evidence, redacted project
replacement events, updater identity shape, exact CLI argument admission,
unconfigured updater fail-closed behavior, and public-key non-serialization.

## Remaining boundary

The software gate remains `In progress`. This does not prove a deployed
native support runbook drill, physical startup/takeover/recovery/shutdown,
signed update installation, wrong-signature/channel/downgrade behavior against
a live endpoint, clean-machine acceptance, Authenticode/notarization, or
public release. No external endpoint, installer, or physical output was used.

`git diff --check`: PASS before commit.
