# AI5 sidecar boundary revalidation — 2026-09-10

## Scope

This checkpoint revalidates the existing optional local MCP sidecar and records
the exact boundary that is proven on the current `main` source. It does not add
a transport, expose a new command, or close `AI5-SIDECAR-001`.

- Source HEAD: `067d826eb23f63b0c2b22266fd78574553d12f80`
- Working tree before this checkpoint: clean
- Product source changes: none
- Owner: this checkpoint owns only this QA record
- The real-file thumbnail missing -> Retry -> recovery trial was not rerun

## Existing implementation boundary

The current sidecar is `tools/syndocal-mcp/server.mjs`, a dependency-free
MCP stdio adapter for a running Syndocal agent bridge. The static adapter
currently provides nine typed tools, including canonical capability discovery,
read-only recording status, and execution of the explicitly reviewed
canonical operation set. The adapter:

- requires absolute descriptor and expected-executable paths;
- verifies the descriptor protocol, process ID, and canonical executable before
  each broker connection;
- uses only the loopback broker and redacts the descriptor credential from
  diagnostics and replies;
- enforces bounded input/output frames and exact tool schemas;
- keeps the canonical operation-to-Tauri map static;
- rejects arbitrary command names, scripts, DOM automation, and unreviewed
  `FailClosed` operations before native dispatch.

The existing `crates/io/src/remote_ws.rs` surface is a separate desktop Remote
WebSocket/UI route. Its existence is not treated as proof of the AI5 sidecar's
authenticated JSON-RPC/REST/WebSocket adapter contract.

## Focused verification

All commands ran from the repository root against the source above:

| Command | Result |
| --- | --- |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback broker only, no Syndocal/device calls |
| `node app/scripts/check-agent-bridge.mjs` | PASS — 11 groups; 47 canonical operation IDs agree across Rust, TypeScript, and Node; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `git diff --check` | PASS |

The checks cover negotiation, all nine tool schemas, static canonical
allowlisting, request correlation, false-success handling, pending/unknown
behavior, no automatic retry, overlap rejection, malformed/oversized frames,
executable mismatch, credential redaction, canonical parity, stale authority,
reply-loss handling, and unmounted bridge cleanup. They use a fake loopback
broker or deterministic source seams; they do not establish real external
client acceptance.

## Remaining AI5 boundary

`AI5-SIDECAR-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. The current source does not yet provide
the complete roadmap contract for:

- authenticated external JSON-RPC command/query, OpenAPI-compatible REST, and
  streamable event/WebSocket adapters;
- per-principal bounded query/command queues and event fan-out with
  generation-gap/resnapshot behavior;
- sidecar termination/restart acceptance during an active show;
- release-EXE tests with a real external client, pairing, revocation, and
  stale-session rejection across the complete external API surface.

These are not inferred from the passing stdio checker. AI6 administration,
AI7 adversarial proof, and AI8 external acceptance remain dependent on the
missing or unaccepted parts. No transport or authentication policy was
invented in this checkpoint because the exact adapter/session/queue wire and
selected SDK boundary are not present in the current source contract.

## Non-claims and cleanup

This record does not claim full AI control-plane parity, safe unattended
high-risk automation, native/hardware output, physical acceptance, thumbnail
file-move recovery, Mac acceptance, signing, publication, or product-wide
completion. No application process, device, physical output, or external
network service was started by these checks.

## Current-main software revalidation — 2026-09-10

The three deterministic sidecar/bridge checks were rerun against current
`main` source HEAD `0589258c1ea3145fc914483d3d80a1e6ed2b1cff`. No product
source, transport, or authentication policy changed.

| Command | Result |
| --- | --- |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback broker only |
| `node app/scripts/check-agent-bridge.mjs` | PASS — 11 groups; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |

These results remain limited to deterministic local seams. The authenticated
external API, real external client, pairing/revocation across that API,
sidecar restart, physical output, Mac, signing, publication, and thumbnail
file-move recovery are not claimed.

## Current-main sidecar and bridge revalidation — 2026-09-11

The deterministic AI5 checks were rerun after the current authority,
output, snapshot, and authored-bridge checkpoints at source HEAD
`15096b5b4f36ffd7dba610e11f749d11cabe2e6b`. Evidence is preserved under
`target/qa/ai5-current-main-20260911-01/`.

| Check | Result |
| --- | --- |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback only |
| `node app/scripts/check-agent-bridge.mjs` | PASS — 11 groups; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `git diff --check` | PASS |

The checks retain strict tool schemas, canonical operation allowlisting,
request correlation, false-success/pending/unknown behavior, overlap and
malformed/oversized frame rejection, executable mismatch, credential
redaction, stale authority, reply-loss handling, and unmounted cleanup.

`AI5-SIDECAR-001` remains Open. This is deterministic local adapter and
bridge evidence only; authenticated external JSON-RPC/REST/WebSocket parity,
per-principal queues/event fan-out, active-show sidecar restart, real client
pairing/revocation, physical output, Mac, signing, publication, and
product-wide acceptance remain unclaimed.

## Current-main sidecar and bridge revalidation — 2026-09-12

The deterministic AI5 checks were rerun against current `main` at source HEAD
`100de87f15698bf8a6329c736da45d3d9bc891c1`; `origin/main` matched before the
run. No transport, authentication, or sidecar policy was changed.

| Check | Result |
| --- | --- |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback only, no Syndocal/device calls |
| `node app/scripts/check-agent-bridge.mjs` | PASS — 11 groups; real processor/runtime/confirmation modules, no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `git diff --check` | PASS |

The run retains strict tool schemas, canonical operation allowlisting,
request correlation, false-success/pending/unknown behavior, overlap and
malformed/oversized frame rejection, executable mismatch, credential
redaction, stale authority, reply-loss handling, and unmounted cleanup.
`AI5-SIDECAR-001` remains Open for authenticated external JSON-RPC/REST/
WebSocket parity, per-principal queue/event fan-out, active-show sidecar
restart, real-client pairing/revocation, physical output, Mac, signing,
publication, and product-wide acceptance.
