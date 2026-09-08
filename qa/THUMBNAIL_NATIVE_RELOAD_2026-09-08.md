# Native thumbnail cache and frontend reload — 2026-09-08

Base main: `66a73772bf38827d7a3957e43b3d30a600531381`.
Scope: successful-cache reuse across view remount and fresh opt-in after WebView
reload. This is a completed native acceptance subset, not missing-file recovery.
No product source, tests, runtime policy, schemas, version or build settings changed.

## Setup and provenance

Used the accepted QA-identifier EXE from [native UI acceptance](THUMBNAIL_NATIVE_UI_2026-09-08.md):
64,532,992 bytes, SHA-256
`A7A8CCAECB8721EEFC00CD4107080023CDD8C78765E64F2DFF84A5005A8768BF`.
`git diff --exit-code b0132b7d9f2350d26a7ef0a1a676825c167635eb HEAD --
app/src app/src-tauri crates Cargo.toml Cargo.lock` passed. The executable hash
matched before launch and after completion. It still has a QA-only identifier;
this is NOT a new default-profile or distribution build. No compiler was rerun,
so this checkpoint has no new compiler-warning baseline/current measurement.

The isolated app retained its preseeded Standby preference; native status denied
both lighting and video with StartupDenied. No physical-output enabling command
was issued, and native video outputs remained empty. The ordinary operator UI
imported newly copied test-owned four-color PNG and silent two-frame MP4 files.
No native reader or IPC response was replaced. Read-only native status/snapshot
calls and passive thumbnail-request observation supplied acceptance evidence.

## Executed result

The second run passed through the real native UI with no page errors:

| Boundary | Observation |
| --- | --- |
| Initial imports | Both assets immediately appeared; actual 160x90 PNG/MP4 thumbnails passed quadrant/alpha assertions. |
| Warm view remount | Leaving Video and selecting it again kept identical image data URLs; thumbnail IPC count remained 4, adding 0 requests. The completed-cache recovery button stayed absent. |
| WebView reload | Native catalog identity was unchanged. Fresh frontend hydration displayed no old thumbnails and emitted no new thumbnail requests before explicit user selection. |
| Fresh explicit selection | Exactly 4 new thumbnail calls: 2 asset and 2 layer requests. Both displayed images matched the initial data URLs. |
| Ownership and exit | Standby/denied outputs preserved; one responsive maximized window; owned process exited; debug listener count 0. |

This is a WebView/frontend reload within one native process, not a saved-project
load, native restart, disk-cold cache benchmark or proof of zero general I/O.
The observed thumbnail totals were 4 asset and 4 layer calls across the whole run.
The final screenshot was inspected. No FPS, latency, complete cache eviction or
full viewport-matrix acceptance is claimed.

Final successful launch: PID 45300, completed `2026-09-08T08:10:40.1751701Z`.
The probe and launcher both exited 0. All six ordinary top-level application
JSON files matched the pre-run hashes; no normal profile was restored or edited.
The first run stopped after warm-cache success because a text locator matched
two Edit labels. Its failure is retained; only the stable test selector changed.

## Evidence and uncompleted outage test

Local evidence root: `target/qa/thumbnail-native-retry-20260908-01/` (the original
name was retained when the accepted scope narrowed). Key files: `probe-02.mjs`,
`launch-02.ps1`, `probe-result-02.json`, `launch-result-02.json`, `run-02.log`,
`reload-success-02.png`, `audit-result.json` and the original failed-run records.
`node --check probe-02.mjs`, native launch and the preservation audit exited 0.
The inherited launch guard checks the exact executable and loopback listener
ancestry; it stops only its owned process. No listener was left behind.

The original planned unit was missing-file -> Retry -> recovery. A write of its
test-only file-move helper was service-blocked before execution. It was not
rerouted or delegated. No fixture was moved, hidden, corrupted or removed. The
subsequent accepted run above exercises ordinary view/reload behavior only.
Therefore native failed-source recovery, successful-cache preservation DURING
that recovery, and GUI cancellation/stop latency remain unverified.

The next owner should implement that separate test on fresh exclusive fixtures,
keep the profile isolated and outputs denied, restore every test-owned file on
failure, verify exact restored content, and use the actual UI retry path. Do not
move operator media, bypass denied device/output policy, fake a native result, or
combine today's successful reload with old mock-reader tests into a native
failure/recovery pass. No broad completion-ledger row is closed by this record.
