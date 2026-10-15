# ShowClock sample lifetime - 2026-10-15

- Checkout: shared `main`, base `e6c76d95aeff7a510869fb2cf41d34eaa3b1e0ff`.
- State: Part B remains uncommitted for Claude review. No schema/version change.
- Reported CI failures: runs 38583808952 / 38484067643, standby `Stale`
  instead of `Locked`. CI logs were supplied in task3, not fetched again.

## Cause and change

- Primary sends every 250 ms but previously advertised a 250 ms lifetime.
  The estimator takes `min(sample lifetime, configured stale window)`, so
  the documented 750 ms holdover was reduced to one send interval.
- `app/src-tauri/src/show_clock_ipc.rs`: primary lifetime now aliases
  `SHOW_CLOCK_DEFAULT_STALE_AFTER_US` (750000); the regression asserts it is
  three send intervals so cadence/default drift fails explicitly.
- Before the fix, the new deterministic test failed at the 400 ms assertion:
  actual `Stale`, expected `Locked` (`target/qa/task4/show-clock-red.log`).
- After the fix the test checks `Locked` at 400000 and 750000 us,
  `Stale` at 750001 us, and latched `Stale` after another valid sample.
  It uses the primary's real lifetime via the signed-sample helper; no sockets
  or sleeps. Existing stopped-primary and output-revocation tests are retained.
- A stopped primary becomes stale on the first advance after 750 ms,
  nominally within 750 ms plus the 100 ms receive poll, subject to scheduling.
  Stale still latches until Manual Hold and re-arm; output is revoked before
  queue polling. No physical output was enabled for this validation.

## Every sample-expiry use checked

- `crates/protocol/src/show_clock.rs:313`: serialized field, unchanged schema.
- `show_clock.rs:339`: shape accepts 1..5000000 us; 750000 is legal. The
  maximum remains unchanged.
- `show_clock.rs:370`: authenticated canonical bytes include the lifetime;
  new samples are signed normally. Authentication is not weakened.
- `show_clock.rs:550`: replay/reordering uses sequence and monotonic sender/show
  time; no absolute 250 ms age assumption or expiry-based replay exemption.
- `crates/protocol/src/show_clock_runtime.rs:181,205`: store sample lifetime,
  then cap it by configured stale interval. Estimator semantics unchanged.
- `app/src-tauri/src/show_clock_ipc.rs`: primary emission and signed test
  helper share the real lifetime. No second production expiry setting.
- Remaining explicit lifetimes in `show_clock.rs`, `show_clock_runtime.rs`,
  `crates/io/src/show_clock_lan.rs:258` and
  `crates/io/tests/show_clock_two_process.rs:51` are test fixtures exercising
  valid shorter lifetimes; they do not set the primary's production lifetime.
- Action admission and scheduling use their own sequence, target time,
  horizon and late policy (`show_clock.rs`, `show_clock_runtime.rs:354,398`).
  Unsafe estimator state still blocks execution. No action timing changed.
- Status publishes estimator state and measured `sample_age_us`;
  `app/src/components/ShowClockStatusPanel.tsx:64,340` formats age in ms and
  reads state. No hardcoded sample-expiry assumption in UI/status.

## Review and validation

- Independent source review approved the stable diff; no blocking findings.
- Owned test matches rustfmt; unrelated baseline formatting is untouched.
- Execution evidence: `target/qa/task4/part-b-results.json` and named logs.
- `test -p protocol show_clock`: 21 passed; app `show_clock`: 11 passed.
- Both reported flaky tests: 20/20 each (40 successful Cargo invocations).
- Full app: 2106 passed, 1 failed, 22 ignored; only the known
  `project_backup_listing_missing_directory_pagination_exact_bytes_and_no_allocation`
  failed, then passed alone (1/1). No assertions or waits were weakened.
- Compiler warnings: pre-fix regression build 0; current protocol/app builds 0;
  delta 0. No `allow`/`expect` attributes added. `git diff --check` passed.
- Shared Part B/C release `build-nobundle.cmd`: exit 0. Specified native
  gate: exit 0, exactly one responsive/maximized `Syndocal` main window
  (plus the tiny SIW helper), clean app exit 0, no process left.
- Release Rust warnings 0; unchanged frontend Vite chunk warning remains
  App 517.29 kB / 500 kB (baseline/current equal). Logs: `native-build.log`,
  `native-gate.log` under `target/qa/task4/`. This is not hardware acceptance.
