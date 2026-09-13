# AI3 native ingress current-source checkpoint — 2026-09-14

- Marker: `AI3-NATIVE-INGRESS-001`
- Branch: `codex/showclock-review-20260912`
- Base: `6f8c5a7dd63eb5a70cd42ac6b9e1ec15982f6452`
- Product code change: none in this checkpoint

## Current-source verification

The canonical renderer/native admission and output safety seams were checked
against the current source:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 479 facade dispatches |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected; source fingerprint `a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab` |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — output-control and Standby Sync contracts |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |
| `pnpm.cmd --dir app run check:safety-blackout-runtime` | PASS |

These checks show that current source routes mutations through the inventoried
admission/ownership/safety seams and rejects the checked negative fixtures.

## Acceptance boundary

`AI3-NATIVE-INGRESS-001` remains `Open`. No native MIDI/OSC/DMX/Remote client,
real Art-Net node, fixture, or other physical ingress was connected or driven
in this checkpoint. Static inventory is not physical ingress acceptance, and
the checks do not prove controller movement, feedback, reconnect, latency, or
venue behavior.

## Resume procedure

Use the actual native clients and devices against the exact current artifact.
Record device identity/topology, accepted and rejected ingress, reconnect and
replacement behavior, output state, and any first failure before reconsidering
the Flow marker.
