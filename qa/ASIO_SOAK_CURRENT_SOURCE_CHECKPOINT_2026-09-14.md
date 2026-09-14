# ASIO Soak Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-SOAK-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `10b1273a230c5214963256d232868af71a135fa6`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`

This checkpoint records the current software lifecycle proof and the explicit
absence of a long-duration device result. No one-hour hardware claim is made.

## Verification

```text
pnpm.cmd --dir app run check:live-audio
```

Result: exit code 0. The current-source fail-closed lifecycle, availability,
selection persistence, presentation, and request-ordering checks passed.

## Takeover rerun — 2026-09-14

The live-audio lifecycle check was rerun after takeover and passed. It again
covered fail-closed availability, explicit selection persistence, presentation,
and request ordering only; no ASIO/WASAPI device was opened and no
long-duration soak or latency artifact was produced.

The SDK-free bridge fault suite and ABI contract evidence are retained in
`qa/ASIO_FAULT_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`; they are deterministic
contract evidence, not a soak substitute. A parallel Cargo retry was stopped
after it became a duplicate build-lock wait and is not counted as evidence.

## Unresolved acceptance

`ASIO-SOAK-001` stays Open. The required matched one-hour ASIO and WASAPI run
still needs real devices, overrun/XRUN telemetry, callback p99 and maximum
budgets, capture-to-engine p95, and loss-to-zero timing bound to the exact
artifact and operator. Source tests cannot establish thermal, clock-drift,
driver, or long-duration behavior.

Next action is a matched one-hour ASIO/WASAPI run on the approved artifact with
raw time-series logs and the stated thresholds recorded.

## Takeover continuation — current-source soak preflight — 2026-09-14

At HEAD `767f1318`, `pnpm.cmd --dir app run check:live-audio` passed the
fail-closed lifecycle, availability, explicit selection persistence,
presentation, and request-ordering contract. No ASIO or WASAPI device was
opened and no long-duration stream, thermal, clock-drift, callback, or latency
artifact was produced. `ASIO-SOAK-001` remains `Open` pending the matched
one-hour physical ASIO/WASAPI run.

## Takeover continuation — ASIO soak preflight — 2026-09-14

At current source HEAD `7ffa46b3`, the following focused source check passed:

```text
pnpm.cmd --dir app run check:live-audio
live audio fail-closed lifecycle, availability contract, selection persistence,
presentation, and request ordering ok
```

The existing M5 harness preflight was also attempted from the ordinary
PowerShell host:

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\qa\run-soak.ps1 -PreflightOnly
```

It failed closed before Cargo or the soak harness because the host did not
provide `VCToolsInstallDir` and the script's automatic exact `vcvars64.bat
-vcvars_ver=14.44` initialization was unavailable. This is an environment
preflight result, not a soak result; no process, device, stream, or report was
created, and it is not counted as acceptance evidence.

The host still exposes ASIO registry entries and USB audio hardware, but the
current checkout does not contain the pinned SDK archive/extraction or a
current-source local Show-ASIO bridge artifact required by the physical
command. Therefore no ASIO or WASAPI stream was opened and no callback/XRUN,
overrun, capture-to-engine, loss-to-zero, thermal, clock-drift, or one-hour
time-series artifact was produced in this continuation.

`ASIO-SOAK-001` remains `Open`. The required matched one-hour ASIO/WASAPI run
must use the approved artifact and named devices, with raw telemetry proving
overrun `0`, callback p99 below `20%` and max below `50%` of buffer duration,
capture-to-engine p95 `<= 40 ms`, and loss-to-zero `<= 250 ms`.

## Takeover continuation — current-source soak preflight recheck after Video repair — 2026-09-14

At current source HEAD `bd3f7415`, the source lifecycle check passed:

```text
pnpm.cmd run check:live-audio
live audio fail-closed lifecycle, availability contract, selection persistence,
presentation, and request ordering ok
```

The existing M5 harness preflight was also rerun without starting Cargo or a
device stream:

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ..\qa\run-soak.ps1 -PreflightOnly
exit code 1
Refusing the Windows native soak build because automatic vcvars64.bat
-vcvars_ver=14.44 initialization failed; run from an x64 Visual Studio 2022
Community Developer Command Prompt. Ambient reason: VCToolsInstallDir absent.
```

This is a failed-closed environment preflight, not a soak result. The harness
has a narrower Community-only preflight; the available Build Tools linker was
used only for the separate SDK-independent bridge contract test and was not
substituted for this soak gate. No ASIO/WASAPI stream, one-hour run, callback or
XRUN telemetry, loss-to-zero measurement, thermal/clock record, or latency
artifact was produced. `ASIO-SOAK-001` remains `Open` pending the approved
artifact, named devices, and the matched one-hour ASIO/WASAPI evidence.
