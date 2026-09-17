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

## Takeover continuation — current-source AI8 boundary recheck — 2026-09-14

At current source HEAD `fda2ed44`, the five local bridge/admission checks were
rerun:

```text
check:agent-bridge: PASS (11 groups)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
tools/syndocal-mcp/check.mjs: PASS (15 adapter integration groups; fake loopback only)
check:strict-json: PASS (130 assertions)
```

All five commands exited `0`. No real external client, clean-machine install,
release endpoint, signing service, update/restart drill, or physical output
was contacted. The fake loopback adapter remains local-only and does not call
Syndocal or a device.

`AI8-EXTERNAL-ACCEPTANCE-001` remains `Open` pending clean installation,
native external clients, restart/update/security review, signing/publication,
and physical-output acceptance.

## Continuation — current-source bridge and native-sidecar authentication boundary — 2026-09-18

At current source HEAD `fc792925`, the bridge, bootstrap, admission, MCP
adapter, MCP transport, and strict-JSON checks were rerun:

```text
check:agent-bridge: PASS (11 groups; 4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected; SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab)
tools/syndocal-mcp/check.mjs: PASS (15 adapter groups; hostile stdio corpus 128 rejected; fake loopback only)
tools/syndocal-mcp/check-transports.mjs: PASS (HTTP health/JSON-RPC/REST/WebSocket; fake loopback only)
check:strict-json: PASS (130 assertions)
```

The exact release executable was already running with a current descriptor at
`%LOCALAPPDATA%\jp.seraf.ktn.syndocal\agent-bridge-v1.json`; its descriptor
process ID and executable path matched the exact checkout
`target/release/syndocal.exe` (SHA-256 recorded by the native process
preflight). A real local stdio sidecar was then started against that exact
descriptor and executable with principal `show-operator` and incarnation `1`.
The MCP `initialize` response succeeded, while `tools/list` returned:

```text
{"code":-32001,"message":"Authenticated principal is required before tool discovery."}
```

The configured credential file was absent, so no credential was created or
printed and no native tool, mutation, device, or output call was attempted.
This is an observed fail-closed authentication boundary, not a real external
client acceptance result.

`AI8-EXTERNAL-ACCEPTANCE-001` remains `Open`. Clean installation, a paired
real external client with an approved grant, authenticated native tool
discovery/calls, crash/restart and update drills, public-network/security
review, artifact/signing/publication inspection, and physical-output
acceptance remain required. Resume by pairing a declared local principal
through the native administration flow, retaining only redacted descriptor
and process identity evidence, then rerunning read-only discovery before any
bounded mutation.

## Continuation — current-source AI8 boundary recheck — 2026-09-15

At current source HEAD `5d07b7fb`, the five local bridge/admission/parser
checks were rerun and all exited `0`:

```text
check:agent-bridge: PASS (11 groups; real processor/runtime/confirmation modules; no native/device calls)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab; 18 negative fixtures rejected)
tools/syndocal-mcp/check.mjs: PASS (15 adapter integration groups; hostile stdio corpus 128 rejected; fake loopback only)
check:strict-json: PASS (130 duplicate-key assertions)
```

These results reconfirm only the current local bridge, lifecycle, admission,
adapter, and strict-parser boundaries. No real external client, clean-machine
install, release endpoint, update/restart drill, public-network security
review, signing service, or physical output was contacted. The fake MCP
loopback does not call Syndocal or a device, so
`AI8-EXTERNAL-ACCEPTANCE-001` remains `Open`.

The Q4 ledger records this source-only recheck as
`EV-AI8-EXTERNAL-CURRENT-SW-2026-09-15`. The next action remains the exact
release artifact in a clean installation with real external clients,
restart/update/security review, signing/publication inspection, and physical
output acceptance.

## Continuation — current artifact external-acceptance boundary recheck — 2026-09-14

After the current release rebuild and Video desk repair at HEAD `5a1d666f`,
the local bridge and parser contracts were rerun:

```text
check:agent-bridge: PASS (11 groups)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
tools/syndocal-mcp/check.mjs: PASS (15 adapter integration groups; fake loopback only)
check:strict-json: PASS (130 assertions)
```

All five commands exited `0`. They strengthen only the current local
processor, lifecycle, admission, adapter, and strict-parser boundaries. No
real external client, clean-machine install, release endpoint, update/restart
drill, signing service, public-network review, or physical output was
contacted. The fake MCP loopback does not call Syndocal or a device.

`AI8-EXTERNAL-ACCEPTANCE-001` remains `Open` pending clean installation,
native external clients, restart/update/security review, signing/publication,
and physical-output acceptance.
