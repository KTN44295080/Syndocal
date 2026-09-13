# AI3 durable acceptance current-source checkpoint — 2026-09-14

- Marker: `AI3-DURABLE-ACCEPTANCE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `4f2f259e46427a6a5d9eb56b166d36fe7af198b7`
- Product code change: none in this checkpoint

## Current-source verification

The durable recovery and dangerous-action admission seams were checked:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected |
| `pnpm.cmd --dir app run check:project-recovery-e3` | PASS — project authority and E3 recovery driver |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — output-control and Standby Sync contracts |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |

Together with `qa/AI3_DURABLE_RECOVERY_CHECKPOINT_2026-09-13.md`, this confirms
the bounded current-source journal, authority, admission, recovery, and output
ownership slices. It does not expand the existing software-only claim.

## Acceptance boundary

`AI3-DURABLE-ACCEPTANCE-001` remains `Open`. This checkpoint did not perform a
native dangerous-action Yes/No/Close session, external-client reply-loss or
restart acceptance, physical creation/teardown acknowledgement, or five-
display hardware run. No physical output was enabled.

## Resume procedure

Run the exact current artifact through the native dangerous-action matrix,
including reply loss, crash/restart, restart non-reclamation, physical ACK,
and five-display topology. Retain process identity, journal state, output
state, and first failure for every case before changing the Flow marker.
