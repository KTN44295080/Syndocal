# Task 20 performance evidence and measured harness

This directory preserves the curated measurements, provenance and current copies of the scripts used for the Task 20 campaign. The scripts are a record of the measured workflow; no fixture generator or harness test suite is archived here. Recomputing the original results requires the retained private raw inputs; reproducing the workload also requires the copied media and show fixtures described below.

## Builds and comparison

Both phases use source base `c204074d` on `task20-perf`, incorporating `main` at `7fe67b00`, and eight decode workers. The before build is a controlled counterfactual with a libav cache of eight and cloned RGBA ISF input. The after build uses cache sixteen and borrowed RGBA input. This compares the combined changes; it does not measure a historical release or isolate each fix's contribution.

| Phase | Executable SHA-256 |
| --- | --- |
| Before | `e0257cad4a191953554dee4d57defaa9138646c5915b9fe776b7316b622e9e38` |
| After | `a1e0ae6519bb3558d2b473fc9d68b9a2487ab6ac55ec8e5bbacf1bb375ef869d` |

The provenance manifest retains both phase build metadata and executable hashes. Preserve the recorded case/phase harness hashes too: the archived current scripts include later hardening and do not retrospectively establish the exact earlier script bytes. Before runs lacked per-sample monitoring of host `cargo`, `rustc`, `link` and `cl` processes; the after guard monitors those names. Standalone test executables and other GPU workloads are not comprehensively monitored.

Current diagnostic changes are QA-only: missing PresentMon files remain explicit partial evidence and cannot establish a pass; lease-renewal object errors are serialized as JSON text in private diagnostics; and exact owned-process shutdown waits up to ten seconds after force termination. Original failed attempts are retained privately with their manifests.

## Local inputs and state

Run commands from the repository root. Stage the archived harness and helper scripts under `target/qa/task20d/`, preserving filenames. The runner expects `cases.json`, `policy.json`, `<phase>-build.json`, a private `original-storage.json`, and show copies named directly `target/qa/task20d/<case-tag>.sdc`. There is no `fixtures/` subdirectory in that contract. The copied source media must resolve from those shows. Fixture generation is not supplied here; use retained QA copies if available, or prepare and validate equivalent copies explicitly. Stage the matching `reviewed-images.json` too when reproducing the reviewed image export.

Use this checkout's default `target/release/syndocal.exe`, with `CARGO_TARGET_DIR` unset. Follow `qa/WINDOWS_NATIVE_BUILD.md` and the specified `build-nobundle.cmd` procedure when building. Capture the resolved executable path, SHA-256 and exact launch PID; attach only to that native instance through loopback CDP port `9270`. The build metadata must match the executable used for the phase.

Start in Standby with every output disabled. Capture the original state once for a new campaign, into a unique preserved capture; the harness's fixed input name is `original-storage.json`. Never overwrite an earlier campaign's original-state capture. `capture-state.mjs original` can update copied Display fixtures to the current primary-monitor mapping, so perform it **before** recording fixture baseline hashes. All Display clients must remain QA windows on the primary monitor, at 1920 × 1080 or 3840 × 2160. Use copied shows only, with local QA transports and no real lighting or network output.

## Running the measured scripts

With the checkout ready and no Syndocal instance running, capture the original state:

```powershell
node target/qa/task20d/capture-state.mjs original
```

One case and the after batch use these exact argument forms:

```powershell
node target/qa/task20d/run-case.mjs h264-1080p30-3l-display-0fx-normal after 20
node target/qa/task20d/batch.mjs --phase after --seconds 20
```

Append selected case tags to the batch command to restrict it. The batch handles the 69-case plan, skips six unsupported NDI cases, and reuses eligible completed case records only for the same executable hash. It refuses to overwrite another build's evidence; runner logs use exclusive creation. Preserve previous attempts privately before an intentional rerun, rather than replacing their evidence in place.

After the campaign, or after a failed case, use the retained recovery flow and verify it:

```powershell
node target/qa/task20d/restore-state.mjs task20-recovery
node target/qa/task20d/capture-state.mjs verify-restored
```

Restoration means observed Standby, all outputs off, and exact Recent/localStorage recovery. A cleanup timeout remains part of the result even when measurements exist. The current stop helper waits up to ten seconds after terminating only the exact owned executable. Verify recovery before continuing after a failed return to Standby; an absent restore observation is not success.

## Curated evidence and recomputation

Committed evidence contains the curated CSV, `detailed-tables.md`, `bottlenecks.md` / `bottlenecks.json`, `provenance.json`, `policy.json`, `cases.json` and the controlled `before-source.diff`. The visual evidence set uses `images/` for nine reviewed PNGs, `reviewed-images.json` to bind their exact bytes, and `ui-validation.json` for selected UI proof.

Raw per-case JSON is excluded because it can contain application state or `errorUi`; the original-state capture is private and must never be committed. Hashes identify private raw inputs without publishing or supplying them. Manifest hashes refer to retained raw bytes. Git can normalize archived text line endings, including the case/policy JSON and batch scripts; compare those hashes against the retained raw files or restore their recorded newline convention. Reviewed PNG bytes are unchanged, and the report gives the normalized UTF-8/LF CSV hash. Recomputing the original aggregate requires access to those original retained records; aggregate numbers and hashes alone cannot reconstruct the samples. A new workload run can produce different measurements and image bytes.

| Helper | Role |
| --- | --- |
| `task20-metrics.py` | Reads retained case, health, usage, presentation and policy inputs; writes aggregate CSV, summary and tables without running the app. |
| `task20-export-csv.py` | Produces a curated CSV with reviewed path/error normalization; preserves numeric, status and hash field strings. |
| `task20-provenance.py` | Exports selected build/case metadata and hashes, excluding private state and raw error payloads. |
| `task20-report-tables.py` | Prints concise Markdown tables from an explicit CSV path, preserving missing, partial and failed evidence. |
| `task20-detailed-tables.py` | Renders full timing/resource tables from curated CSV and policy through the existing metrics formatter, without reading raw cases or recalculating gates. |
| `task20-bottleneck-cases.py` | Reads the CSV and writes bottleneck analysis as Markdown and JSON. |
| `task20-export-ui.py` | Exports selected UI proof and images whose exact bytes match the reviewed manifest. |

With original raw inputs retained under `target/qa/task20d/`, recompute into an ignored output directory:

```powershell
python target/qa/task20d/task20-metrics.py target/qa/task20d --out-dir target/qa/task20d/recomputed
python target/qa/task20d/task20-export-csv.py target/qa/task20d/recomputed/measurements.csv target/qa/task20d/recomputed/measurements-curated.csv
python target/qa/task20d/task20-provenance.py target/qa/task20d target/qa/task20d/recomputed/provenance.json
python target/qa/task20d/task20-report-tables.py target/qa/task20d/recomputed/measurements-curated.csv
python target/qa/task20d/task20-detailed-tables.py target/qa/task20d/recomputed/measurements-curated.csv target/qa/task20d/policy.json target/qa/task20d/recomputed/detailed-tables.md
python target/qa/task20d/task20-bottleneck-cases.py --csv target/qa/task20d/recomputed/measurements-curated.csv --markdown target/qa/task20d/recomputed/bottlenecks.md --json target/qa/task20d/recomputed/bottlenecks.json
python target/qa/task20d/task20-export-ui.py target/qa/task20d target/qa/task20d/recomputed --reviewed-manifest target/qa/task20d/reviewed-images.json
```

The image export requires the exact reviewed raw files. Newly captured images need their own review and matching manifest entries; the earlier manifest does not validate changed bytes.

Missing or insufficient PresentMon evidence and an unfinished native validation window cannot establish an envelope pass. Partial evidence may establish an observed miss while leaving other gates unavailable. Keep the native work-deadline gate distinct from actual output cadence, and retain `null`/unavailable values rather than replacing them with zero.

This is a short windowed QA campaign, not a soak test. The enumerated primary display is 2560 × 1600; the oversized 4K window does not prove 4K scanout. Sources are encoded at 30 fps, so 60 submissions do not establish 60 unique source images. See the performance report for measured tables, timing boundaries, machine details and remaining bottlenecks.
