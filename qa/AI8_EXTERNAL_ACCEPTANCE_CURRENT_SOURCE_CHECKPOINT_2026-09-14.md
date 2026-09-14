# AI8 external acceptance current-source checkpoint — 2026-09-14

- Marker: `AI8-EXTERNAL-ACCEPTANCE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `25ca23e495e183c8c72b15309d52b52c4cc4a66d`
- Product code change: none in this checkpoint

## Current-source verification

The current AI/bridge/admission boundaries were checked:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter groups; fake loopback only |
| `pnpm.cmd --dir app run check:strict-json` | PASS — 130 duplicate-key assertions |

These checks cover bounded current-source bridge processing, lifecycle,
admission, adapter validation, and strict parsing. They do not turn the local
software seams into release-native external acceptance.

## Takeover rerun — 2026-09-14

The current-source suite was rerun after takeover. Agent Bridge passed 11
groups, bootstrap passed 4 deferred lifecycle groups, the admission inventory
passed with 539 commands and 18 rejected negative fixtures, the fake loopback
adapter passed 15 groups, and strict JSON passed 130 assertions. No real
external client, release endpoint, clean-machine install, signing service, or
physical output was contacted.

## Acceptance boundary

`AI8-EXTERNAL-ACCEPTANCE-001` remains `Open`. No clean installation, native
external MCP/JSON-RPC/REST/WebSocket client, restart drill for the release
candidate, physical output, public-network security review, signing, or
publication acceptance was performed in this checkpoint. The fake MCP
loopback does not call Syndocal or a device.

## Resume procedure

Use the exact release artifact in a clean machine/install context. Exercise
real external clients, crash/restart and update paths, security review, and
physical output with explicit redaction/artifact inspection. Record first
failure and artifact identity before reconsidering AI8.

## Takeover continuation — current-source bridge recheck — 2026-09-14

The five current-source checks were rerun against HEAD `e4ca306d` after the
takeover:

```text
check:agent-bridge: 11 groups — PASS
check-agent-bridge-bootstrap: 4 deferred lifecycle groups — PASS
check-tauri-admission-inventory: 539 commands, 18 negative fixtures rejected,
SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab — PASS
tools/syndocal-mcp/check.mjs: 15 adapter integration groups, fake loopback only — PASS
check:strict-json: 130 assertions — PASS
```

No real external client, release endpoint, clean-machine install, signing
service, or physical output was contacted. This recheck strengthens only the
local bridge/admission/parser boundary; it is not AI8 release acceptance.

`AI8-EXTERNAL-ACCEPTANCE-001` remains `Open` for clean installation, native
external clients, restart/update drills, public-network security review,
signing, publication, and physical-output acceptance.
