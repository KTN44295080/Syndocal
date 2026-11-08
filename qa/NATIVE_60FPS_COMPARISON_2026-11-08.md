# Native Display comparison — 2026-11-08

Base: `main` at `1f20dd12bdbe24811b34ad93f85e4455c3016e7a`.
This checkpoint measures the current committed product; it does not change the
default Display route or product performance thresholds. The owned changes are
this note and the reproducible measurement-harness correction alongside it.
The unrelated untracked Vite directories remain untouched.

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

## Formal ABBAAB result: INCOMPLETE

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

## Current acceptance boundary

A hands-off, unobstructed formal rerun is pending the operator's screen
preparation. The current formal profile is reusable for another ordinary C5
campaign only because every prior session stopped cleanly; the existing
preflight records that reuse as a warning. Its earlier INCOMPLETE campaign means
this profile does **not** qualify for same-profile soak reuse. A later soak needs
a fresh artifact/profile with only retained full C5 PASS history. No ledger
deletion, receipt copying, weakened validity rule or automatic refocusing.

The computer-use policy prohibits automating the Codex/ChatGPT UI, so the operator
must minimize it and keep the Display unoccluded during the rerun. No active
native output, owned sidecar or diagnostic listener remains at this checkpoint.

Application presentation is distinct from physical scanout. C5/soak do not close
the separate native lifecycle/fault/clock-oracle, venue soak or release gates.
