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
