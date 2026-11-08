# Native Display comparison — 2026-11-08

Base: `main` at `1f20dd12bdbe24811b34ad93f85e4455c3016e7a`.
This checkpoint measures the current committed product; it does not change the
default Display route or product performance thresholds. The owned changes are
this note and the reproducible measurement-harness correction alongside it.
The unrelated untracked Vite directories remain untouched.

Latest result: the prepared-screen rerun **passed C5** with all three ON windows
and all six lifecycle checks valid. The earlier incomplete attempts below remain
retained. The default OFF route still misses 60fps; this is candidate comparison
acceptance, not default-route adoption or release acceptance.

## Fixed conditions

- Canonical `c5/fixture-3l-groupfx.sdc`, SHA-256
  `bc59772e5b3b6c8ca0422f74df7f98675642f97cd92a910b68dd3b2db9f736cd`:
  three H.264 1080p layers, three ISF stages each, transformed Screen group FX.
- Borderless 2560×1600 Display on DISPLAY1, 180 Hz, scale 2, AC power.
  Monitor identity:
  `b616fa670a69c633b10e9b74616dbfd585d3bfe9e4ce0cd0ef307295e0013f75`.
- A: `SYNDOCAL_QA_NATIVE_GPU` absent; B: set to `1`. Same receipted normal
  executable and runtime DLLs within each campaign. No timing build or trace.
- Authenticated external-MCP control, effective Video only, lighting false,
  no lighting fixtures, DMX outputs disabled, no audio or playing timeline.
- The fixed C5 gates remain binding, including N ≥ floor(60×T) − 1 for every
  ON window, whole-window p95 ≤20 ms, existing work-budget gate and complete
  identity/path/lifecycle proof. A short smoke never establishes C5 acceptance.

## Retained smoke and harness correction

Artifacts below are relative to
`target/qa/steroids-60fps-design-20261025/c5/`.

Build `build-1f20dd12bdbe-normal-2611080047` completed through the maintained
Tauri wrapper and exact Build Tools MSVC 14.44.35207 environment, with zero
compiler warnings. Its receipt verifies the committed archive, pinned FFmpeg/NDI
runtime hashes and unchanged protected normal executable. EXE SHA-256:
`6c5a01c6d42321e5389d05a884bd82090d5685bbcfaab99eed704c8dafcdc969`.

Smoke `runs/20261108-010041-462776-1f20dd12bdbe` stopped after OFF. Its ten-second
window delivered 83 frames (~8.287 fps), with no observed occluder. It is
`SMOKE-NOT-GATING`, exit 1; ON was not launched. Product Stop fulfilled in
780.16 ms, Standby/Ready denied both permissions, health/routes/sources cleared,
output windows closed, MCP lease/principals/credentials/sidecar cleared, and the
app exited gracefully 354 ms after close without a confirmation dialog.

The parent falsely classified a retained, already-exited Windows process object
as alive: creation time remained queryable while its image path was unavailable,
so fallback termination returned an identity mismatch and cleanup correctly
remained failed under the old observation. The failed raw result and ledger
entry are preserved; neither is reclassified as clean.

[C5_HARNESS_2026-11-08.patch](C5_HARNESS_2026-11-08.patch)
retains the exact correction to the ignored C5 harness and its focused test.
`process_identity` requests SYNCHRONIZE and records a zero-time
WaitForSingleObject result on the same handle. Only a signaled object avoids
fallback. Timeout, WAIT_FAILED or missing proof still retain a fallback failure;
actual termination still requires matching path and creation time on one handle.
All existing Stop, child/process, port, MCP and graceful-exit checks remain.
Current `run-c5.py` SHA-256:
`0c20e9876ebdae3ed2ec4ccc6eccefdba768bd8fec98f94a9da6a08ee9000c9d`.

Validation: focused real Windows dead/live child and synthetic unknown/reused-PID
checks passed; host-preflight 15 checks and analyzer 46 checks passed. Analyzer
evidence: `selftest-cleanup-20261108-0107/SELFTEST-RESULT.json`. The analyzer was
first invoked without its required output-directory argument and refused before
testing; the corrected invocation ran all 46 checks. Independent review found
no blocking issue in the stable correction or proceeding directly to formal
ABBAAB with a new artifact/profile. Reverse patch application check passed.

## First formal ABBAAB attempt: INCOMPLETE

Fresh artifact `build-1f20dd12bdbe-normal-2611080105` verified the same committed
source/runtime inventory and protected normal executable. EXE SHA-256:
`7f91c1223018b5a4cdd28ec32d182d7b7920b3a40309ec23dcb97ec3978749db`.
Its native build also had zero compiler warnings (previous build 0, current 0,
delta 0). Dry-run `dry-run/dry-run-20261108-011459-510693.json` passed with no
pending conditions; ordinary windows on the monitor and the historical normal
EXE hash mismatch remained reported warnings, not exemptions from live checks.

Campaign: `runs/20261108-011514-686804-1f20dd12bdbe`, exit 1,
overall **INCOMPLETE**. Fixed ABBAAB, 60-second windows, 15-second warm-up.

| Session | Observation | Result |
| --- | --- | --- |
| s01 OFF | Lease-authority query rejected before output acquisition | No window; exact error code was lost as `[object Object]` |
| s02 ON | 59.9632 fps, output covered by Syndocal main/Codex and other windows | INVALID: V1 occlusion |
| s03 ON | 59.6253 fps, output covered by Codex/Discord throughout | INVALID: V1 occlusion |
| s04 OFF | 8.1967 fps, no occlusion | Valid baseline; rate/cadence/work-budget fail |
| s05 OFF | 8.2461 fps, no occlusion | Valid baseline; rate/cadence/work-budget fail |
| s06 ON | 60.009 fps; 3602 deliveries / required 3600 in 60.0243 s | Valid PASS, G1–G8; whole-window p95 17.4 ms |

Occluded ON readings are diagnostic observations, not a valid performance
comparison or evidence of a current product regression. One valid ON window
does not establish the required three. The valid OFF mean is 8.221 fps; the
valid ON work mean is approximately 5.896 ms, with preparation p95 ≤7.655 ms.

All six sessions now proved clean lifecycle, including the early query failure.
Stop maximum was 475.78 ms; permissions denied, outputs/health/routes/sources
cleared, leases relinquished, principals revoked, credentials removed, sidecars
gone, normal window close without dialogs, no forced termination and CDP free.
The new parent observation proved signaled retained process objects explicitly;
the other case was already gone. Protected normal EXE SHA-256 remained
`1d8e757ea8e7b1ffee8a51496d1c4ef2aa0399148a373c55313f252f1212d228`.

## Structured pre-send query errors

The original MCP adapter used `String(error)`, losing the structured QueryError
code/flags. Its existing bounded pre-send overload retry expected an obsolete
Rust Debug string. The retained patch now forwards only code, retryable and
resnapshot_required; arbitrary message/credential fields are not retained.
Only the existing three read-only pre-send query commands with exact
`code=overloaded`, `retryable=true`, `resnapshot_required=false` qualify for the
unchanged six-attempt/500 ms limit. Mutation receipts, send/transport failures,
denied/unavailable/internal errors and malformed flags remain failures.
The old s01 cause is **unknown**; source try-lock overload paths are not proof
that its lost error had that code. No previous result or ledger is rewritten.

The evaluated-IPC adapter regression passed success, structured errors,
whitelist/flag boundaries, redaction and one execution per adapter invocation;
both JavaScript files passed syntax checks. Independent review confirmed the
typed contract and fail-closed boundary.

## Prepared-screen ABBAAB rerun: PASS

After the operator explicitly reported readiness, Codex/Discord were minimized
by the operator and the PC remained hands off. Dry-run
`dry-run/dry-run-20261108-185859-431663.json` passed without a monitor-clear
warning or pending condition. The same exact receipted executable/runtime was
reused under the existing clean-profile rule; no profile, ledger or receipt was
deleted or relabeled. HEAD had advanced to `3b924896` for QA documentation and
the saved harness patch only; the measured product commit remains `1f20dd12`.

Campaign: `runs/20261108-185915-695037-1f20dd12bdbe`, 18:59–19:12 JST,
runner **exit 0, C5 PASS**. ABBAAB, six completed sessions, three valid windows
per path, fixed 60-second windows and 15-second warm-up, no observed occluder or
invalid condition, no missing gate, all cleanup proved. Harness hashes in
`campaign.json` bind the reviewed process-exit and structured-query corrections.

| ON session | Deliveries / required | Actual duration s | fps | Whole-window p95 ms | Verdict |
| --- | --- | --- | --- | --- | --- |
| s02 | 3601 / 3599 | 60.0164 | 60.0002 | 17.3 | PASS, G1–G8 |
| s03 | 3601 / 3600 | 60.0244 | 59.9923 | 17.3 | PASS, G1–G8 |
| s06 | 3601 / 3600 | 60.0194 | 59.9972 | 17.3 | PASS, G1–G8 |

Each rate satisfies the unchanged N ≥ floor(60×T) − 1 rule; the displayed rate
slightly below 60 in two windows does not represent a relaxed 59.5fps threshold.
ON descriptive mean is 59.997 fps, work mean approximately 6.462 ms and maximum
preparation p95 8.755 ms. Preparation is a proxy, not a complete critical-cycle
measurement. All ON work gates passed, error deltas and cadence-deficit increase
were zero, and G6–G8 proved the intended resident path, receipt identity and
bounded async behavior. One ON Presented interval exceeded 20 ms (maximum
30.6 ms); do not describe this campaign as having zero late intervals.

OFF windows were valid reference observations at 6.3469, 6.7808 and 7.1812 fps
(descriptive mean 6.770); all failed rate/cadence/work-budget. Those values differ
from the morning's OFF readings; this campaign alone does not establish a cause
for that variation. OFF and ON share this campaign's receipted executable,
fixture, monitor, observer and frozen protocol.

All six Stop receipts fulfilled; maximum Stop 325.42 ms. Every session proved
Standby/Ready with video/lighting denied, no live Display/health/routes/sources,
lease relinquishment, principal revocation, credential removal, sidecar exit,
graceful app exit without a dialog, no forced termination, no leftover owned
process and CDP free. Post-campaign OS checks found no Syndocal/owned sidecar or
9285 listener. Protected normal EXE hash is unchanged. No product code, default
route, runtime settings or physical-lighting configuration changed.

## Current acceptance boundary

The current committed product now has a valid receipt-bound C5 A/B comparison
with resident ON passing all required windows. Default OFF remains unchanged
and fails the mandatory performance target. Adoption and release remain open.

The earlier INCOMPLETE campaign is still present in this profile's history,
so it does **not** qualify for same-profile soak reuse despite the later PASS.
A later thirty-minute soak needs a fresh artifact/profile with only retained
full C5 PASS history. No ledger deletion, receipt copying, weakened validity rule
or automatic refocusing. The first safe continuation is prepare a fresh normal
artifact, pass its formal ABBAAB, then reuse that exact artifact for soak.

Application presentation is distinct from physical scanout. C5/soak do not close
the separate native lifecycle/fault/clock-oracle, venue soak or release gates.
