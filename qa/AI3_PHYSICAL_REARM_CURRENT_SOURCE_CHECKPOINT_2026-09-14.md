# AI3 physical re-Arm current-source checkpoint — 2026-09-14

- Marker: `AI3-PHYSICAL-REARM-001`
- Branch: `codex/showclock-review-20260912`
- Base: `18e0f99813512cd5bccbcb63f4c5818cc70cbeca`
- Product code change: none in this checkpoint

## Current-source verification

The project replacement, recovery, output-control, ownership, and safety
contracts were checked without opening a physical output device:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:project-transaction` | PASS — transaction and authority production contracts |
| `pnpm.cmd --dir app run check:project-recovery-e3` | PASS — E3 recovery production driver |
| `node app/scripts/check-project-transaction-recovery-controller.mjs` | PASS — 6 scenarios; no native/UI side effects |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — output-control and Standby Sync contracts |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |
| `pnpm.cmd --dir app run check:safety-blackout-runtime` | PASS |

These checks cover source-side candidate/orphan, generation, receipt, output
ownership, and safe-blackout boundaries for replacement/recovery paths.

## Takeover rerun — 2026-09-14

The six focused source checks were rerun against current source after the
takeover. Project transaction/authority, E3 recovery, the six-scenario
transaction-recovery controller, output-control/Standby Sync, output
ownership, and safety-blackout all passed with exit code 0. The run opened no
physical output and produced no hardware ACK or re-Arm observation.

## Acceptance boundary

`AI3-PHYSICAL-REARM-001` remains `Open`. No physical DMX, Art-Net, video,
audio, or other output was enabled. The checks do not prove New/Load/Recovery/
Backup/Take Over retirement and explicit re-Arm against an acknowledged real
output state, nor do they prove hardware ACK or venue behavior.

## Resume procedure

Run the replacement matrix on the exact current artifact with a declared
physical-output topology. Capture pre/post owner, project/lease generations,
Blackout state, operator confirmation, re-Arm result, and hardware observation
for every path; stop and record the first failure.

## Takeover continuation — current Rust replacement boundary — 2026-09-14

The exact current source at `423d9f4c2645543a6b80bb61ca5d5b5174caf289` was
also exercised with the pinned Visual Studio 2022 Build Tools MSVC
`14.44.35207` x64 linker. The release target was compiled with
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` pinned to the exact linker and
verified first by `where.exe`. The focused replacement filter passed 7 tests:

```text
project_replacement: 7 passed / 0 failed / 0 ignored
project_control_retirement_joins_before_publish_and_releases_partial_takes:
1 passed / 0 failed / 0 ignored
output_lease_app_state_retirement_and_project_preflight_are_atomic:
1 passed / 0 failed / 0 ignored
```

The tested source boundary covers Standby-worker join before the fenced swap,
output-retirement failure without publication, no local-state commit before
publication ACK, infallible post-ACK finalization, old-project lease orphaning,
input/control retirement before publication, and atomic output-lease project
preflight. These are current-source Rust fixtures and do not open a physical
DMX/Art-Net/video/audio output device.

The marker remains `Open`. The required New/Load/Recovery/Backup/Take Over
matrix still needs a declared real output topology, acknowledged physical
retirement, explicit re-Arm, hardware observation, and venue behavior. The
additional Rust results therefore strengthen the software evidence only and
do not alter the external acceptance status.

## Takeover continuation — current-source re-Arm/replacement recheck — 2026-09-14

At current source HEAD `e1e76de9`, the replacement/recovery source contracts
were rerun:

```text
check:project-transaction: PASS (transaction and authority)
check:project-recovery-e3: PASS (E3 production driver)
check-project-transaction-recovery-controller: PASS (6 scenarios)
check:output-control-runtime: PASS (including Standby Sync output-lease UI)
check:output-ownership: PASS
check:safety-blackout-runtime: PASS
```

All six commands exited `0`. They confirm source-side candidate/orphan,
generation, receipt, output-retirement, ownership, and blackout boundaries;
they opened no physical output and produced no hardware ACK or re-Arm result.

`AI3-PHYSICAL-REARM-001` remains `Open` pending the exact artifact's physical
New/Load/Recovery/Backup/Take Over matrix, acknowledged output retirement,
explicit re-Arm, hardware observation, and venue evidence.

## Continuation — current artifact re-Arm boundary recheck — 2026-09-14

After the current Video desk release rebuild at HEAD `326e7de1`, the six
source-side checks were rerun against the same checkout:

```text
check:project-transaction: PASS
check:project-recovery-e3: PASS
check-project-transaction-recovery-controller: PASS (6 scenarios)
check:output-control-runtime: PASS
check:output-ownership: PASS
check:safety-blackout-runtime: PASS
```

All six commands exited `0`. The current artifact therefore retains the
candidate/orphan, generation, receipt, output-retirement, ownership, and
blackout safeguards needed before project replacement can publish. No native
dialog, physical DMX/Art-Net/video/audio output, hardware ACK, or re-Arm was
performed in this continuation.

`AI3-PHYSICAL-REARM-001` remains `Open`. The next required evidence is still
the exact release artifact's physical New/Load/Recovery/Backup/Take Over
matrix with acknowledged output retirement, explicit re-Arm, hardware state,
and venue observations.
