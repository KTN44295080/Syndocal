# Syndocal MCP adapter

Dependency-free Node.js sidecar for a running Syndocal agent bridge. It exposes fixture listing, fixture reading, exact-project fixture transforms, exact-project Video BO control, request-status lookup, bounded runtime diagnostics, canonical backend capability discovery, read-only recording status, and the 47 reviewed canonical control-plane operations. It does not open devices, start Syndocal, or bypass the backend's output ownership, lease, safety, or typed-command checks.

## Start

Use an absolute Node executable and absolute paths in your MCP client's stdio server configuration. The server command is:

```text
node C:/Users/kouty/Documents/KDMX/tools/syndocal-mcp/server.mjs --expected-executable C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe --principal-id show-operator --principal-incarnation 1 --credential-file C:/Users/kouty/AppData/Local/Syndocal/show-operator.credential
```

`--expected-executable` is required. `--descriptor` optionally overrides the default `%LOCALAPPDATA%/jp.seraf.ktn.syndocal/agent-bridge-v1.json`. Both paths must be absolute. Real tools additionally require `--principal-id`, `--principal-incarnation`, and `--credential-file`; the file contains the single 64-character credential returned by local pairing and must remain outside project state. Launch the expected Syndocal executable with its agent bridge enabled first. This adapter checks the descriptor protocol, process ID, executable path, and per-launch session nonce before every broker connection. It uses only `127.0.0.1`.

For streamable HTTP/JSON-RPC, start the same sidecar with `--http-port 0` (or a
fixed local port). It binds only to `127.0.0.1`; `GET /healthz` is an
unauthenticated compatibility probe, `POST /rpc` carries the same JSON-RPC
messages and requires an `X-Syndocal-Session` header, `POST
/rest/tools/<tool-name>` is a bounded conventional facade over the same MCP
tool dispatch, and `/ws` carries the same JSON-RPC messages over WebSocket.
REST and WebSocket calls still prove the configured principal on every native
request. The transport has bounded HTTP sessions/connections and eight WebSocket
connections; it has no LAN or public-listener mode.
HTTP negotiation sessions expire after five idle minutes, measured from request
completion. In-flight requests (including discovery) cannot be expired or deleted.
Clients may release an idle session with `DELETE /rpc` and its
`X-Syndocal-Session` header (204 removed, 404 absent, 409 busy). Expired or deleted
RPC sessions must initialize again. This discards transport negotiation only;
native grants, credentials and request receipts are unaffected. At 64 unexpired
sessions new sessions still receive 429; active sessions are never evicted to
admit a newcomer.

HTTP and WebSocket requests require a literal loopback/localhost `Host` with the
actual listening port. Browser requests must have the exact same HTTP origin;
foreign/null origins and cross-site/same-site fetch metadata are rejected before
dispatch. Non-browser clients may omit Origin. Session headers do not replace
these browser-origin checks or the native principal proof.

The local Codex client can register this server using its installed CLI:

```powershell
codex mcp add syndocal -- 'C:/nvm4w/nodejs/node.exe' 'C:/Users/kouty/Documents/KDMX/tools/syndocal-mcp/server.mjs' --expected-executable 'C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe' --principal-id 'show-operator' --principal-incarnation '1' --credential-file 'C:/Users/kouty/AppData/Local/Syndocal/show-operator.credential'
codex mcp get syndocal --json
```

These are this workstation's paths; configure the actual checkout, Node, and
credential-file paths on another machine. A running client session may need to
reload its MCP connections before discovering the new tools. The credential
value is never placed in this configuration: the adapter reads the file only to
create a nonce-bound proof for the current broker request.

On Windows, process identity is read through a fixed, hidden PowerShell/CIM query with a two-second deadline. No tool can supply commands or scripts. On Linux, the test-compatible equivalent reads `/proc/<pid>/exe`. Other platforms without that interface fail closed. The descriptor credential is never included in diagnostics; broker strings and keys are redacted if they echo it. Keep the descriptor private to your local account.

## Tools

- `syndocal_list_fixtures({})`: read fixtures and the current project identity.
- `syndocal_get_fixture({fixtureId})`: read one fixture and project identity.
- `syndocal_set_fixture_transform({requestId, fixtureId, position, rotation, expectedProject})`: send one complete transform with a caller-supplied UUID and the exact project identity returned by a read.
- `syndocal_set_video_blackout({requestId, enabled, expectedProject})`: set Video BO with a caller-supplied UUID and the exact project identity returned by a read. It requires both lighting and video output ownership to already be active; disabling may reveal that existing output and never arms, acquires, or enables output.
- `syndocal_get_request_status({requestId})`: query the original request UUID.
- `syndocal_get_runtime_status({})`: read the project token, lighting/video blackout bits, up to 64 video-output summaries, Timeline transport state, the exact `timeline_runtime` projection, and a separate `observations.output_ownership_status` read. The authority bundle and ownership observation are captured by separate reads and must not be treated as one atomic image.
- `syndocal_get_control_plane_capabilities({})`: read a bounded projection of the backend-owned canonical operation/source inventory and exact local adapter policy. `FailClosed` entries are discovery-only and cannot be invoked through MCP.
- `syndocal_get_recording_status({})`: read bounded active recording, dimensions, frame/drop counters, audio inclusion, path, and last-error state. It never starts, stops, finalizes, or replaces a recording.
- `syndocal_execute_control_plane({requestId,operationId,request})`: execute one of the 47 reviewed canonical operations through a static typed Tauri adapter. `operationId` must be present in the capability registry and `request` must be that operation's exact typed request object. Unreviewed or `FailClosed` inventory entries are rejected.

Position is `{x,y,z}` and rotation is `{pitch,yaw,roll}`. `expectedProject` is `{project_epoch,project_revision,checkpoint_hash}`. All fields are required; transform coordinates must be finite, project epoch/revision values must be nonnegative safe integers, and `checkpoint_hash` must be exactly 64 lowercase hexadecimal characters. Unknown argument fields are rejected.

A new mutation intent needs a new lowercase, hyphenated UUID. Uppercase UUIDs are rejected so one intent has a single request identity. Preserve that UUID until its result is known. `pending` and `unknown` are tool errors with a status-query instruction; the adapter does not retry or automatically poll. A timeout after a mutation may have been sent is `unknown`, never proof that it was not applied. Query the original UUID before deciding any next action. Status requests use a fresh transport envelope UUID but the broker response identifies the queried original UUID. Broker `completed` means terminal processing; only a result with `ok:true` is tool success, so a completed `{ok:false}` response remains an error with its original result. The canonical executor remains a bounded static map; it is not arbitrary `invoke(command, params)`. Query operations are not persisted as mutation tombstones, while canonical mutations retain the same durable replay protection.

Only one broker request may be active per transport session. An overlapping call is explicitly rejected before dispatch. MCP, HTTP, REST, and WebSocket input frames are bounded to 64 KiB; native and sidecar response frames are bounded to 256 KiB. Connection timeout is one second and native response timeout is three seconds. Every broker request carries a fresh client nonce and HMAC-SHA256 proof bound to the descriptor's per-launch session nonce, principal incarnation, request ID, and method. The native broker rejects missing, stale, invalid, or replayed proof before renderer dispatch and then applies the exact ExternalMcp grant; R4/R5 requests remain consent-bound. The stdio mode writes nothing except newline-delimited MCP JSON-RPC to stdout.

## Validate

```text
node tools/syndocal-mcp/check.mjs
node tools/syndocal-mcp/check-transports.mjs
node tools/syndocal-mcp/check-transport-security.mjs
```

For a running Windows native build with its isolated WebView2 loopback debugging
port enabled, run the real read-only acceptance harness:

```text
node tools/syndocal-mcp/check-native-readonly.mjs --expected-executable <absolute-exe> --cdp-port <port> --evidence <new-absolute-json-path>
```

The harness verifies the debugger belongs to the selected executable's process
tree, bootstraps one temporary principal through main-window backend commands,
and runs a separate stdio MCP sidecar. It checks ungranted denial, four granted
reads through terminal receipt lookup, safe-mode read-only grants, and revocation.
It also runs real HTTP JSON-RPC, REST and WebSocket clients against an adapter
in the harness process and the same native broker, before and after revocation.
It revokes its own principal and removes its ACL-restricted credential directory.
It neither sends device/output commands nor establishes physical acceptance.

For native crash/restart proof, use the isolated
`app/src-tauri/tauri.mcp-lifecycle.conf.json` configuration. Build it with
`pnpm.cmd --dir app tauri build --no-bundle --config src-tauri/tauri.mcp-lifecycle.conf.json`
and an absolute `CARGO_TARGET_DIR` of
`%TEMP%/syndocal-native-acceptance-target`; restore the environment afterward.
The configuration has its own application identifier and persistent profile.
Then run:

```text
node tools/syndocal-mcp/check-native-lifecycle.mjs --expected-executable <absolute-TEMP>/syndocal-native-acceptance-target/release/syndocal.exe --cdp-port 9256 --evidence <new-absolute-json-path>
```

This harness refuses the normal executable, requires an empty QA project, and
verifies one responsive maximized QA window on each launch. It tests forced exit,
clean exit, fresh launch identity, retired credentials/grants, old receipts and
launch-proof rejection through real native requests. It checks that the normal
app descriptor is unchanged and removes its own credential and QA processes.
It does not prove durable authored/output mutation, publication recovery or a
clean-machine installation. The shared `native-backend-session.mjs` owns the
process-verified CDP bootstrap; no DOM actions are used. The read-only runner
also accepts an optional absolute `--descriptor` for explicitly isolated builds.

Receipt lookup requires fresh authentication and the exact submitting principal
incarnation. It does not require a new operation grant or dispatch to the renderer.
Other owners cannot read completed results or reuse their IDs. Durable command
hashes include the authenticated owner; pre-owner-binding hashes remain replay
fences and return `request_conflict` on resubmission. After restart, status is
`unknown`: an old receipt cannot prove the new renderer completed an operation.
Never resubmit an uncertain mutation with a new ID without reconciling its state.

The isolated-process security check covers Host/Origin rejection, malformed tool
paths, and event-loop liveness while WebSocket headers/payloads arrive in pieces.
It uses no native broker or devices and is included in `check:ai5-sidecar`.

The integration check launches the CLI against its own fake loopback broker. It exercises negotiation, all nine tool schemas, canonical operation allowlisting, request correlation, Video BO false-success handling, pending/unknown behavior, no automatic retry, overlap rejection, malformed/oversized frames, executable mismatch and credential redaction. The transport check exercises health, JSON-RPC over HTTP, the REST facade, WebSocket JSON-RPC, and the same nonce proof against a fake loopback broker. These checks do not operate Syndocal or physical devices. Real native bridge acceptance is a separate integration check.

The adapter implements the MCP **2025-11-25** [stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [initialization lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), and [tools interface](https://modelcontextprotocol.io/specification/2025-11-25/server/tools). Supported JSON-RPC methods are `initialize`, `notifications/initialized`, `ping`, `tools/list`, and `tools/call`. The HTTP and WebSocket transports reuse the same JSON-RPC dispatcher and native authentication path; they do not add a second command registry.
