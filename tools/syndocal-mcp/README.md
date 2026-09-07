# Syndocal MCP adapter

Dependency-free Node.js stdio adapter for a running Syndocal agent bridge. It exposes fixture listing, fixture reading, exact-project fixture transforms, exact-project Video BO control, request-status lookup, bounded runtime diagnostics, canonical backend capability discovery, read-only recording status, and the 47 reviewed canonical control-plane operations. It does not open devices, start Syndocal, or bypass the backend's output ownership, lease, safety, or typed-command checks.

## Start

Use an absolute Node executable and absolute paths in your MCP client's stdio server configuration. The server command is:

```text
node C:/Users/kouty/Documents/KDMX/tools/syndocal-mcp/server.mjs --expected-executable C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe
```

`--expected-executable` is required. `--descriptor` optionally overrides the default `%LOCALAPPDATA%/jp.seraf.ktn.syndocal/agent-bridge-v1.json`. Both paths must be absolute. Launch the expected Syndocal executable with its agent bridge enabled first. This adapter checks the descriptor protocol, process ID and executable path before every broker connection. It uses only `127.0.0.1`.

The local Codex client can register this server using its installed CLI:

```powershell
codex mcp add syndocal -- 'C:/nvm4w/nodejs/node.exe' 'C:/Users/kouty/Documents/KDMX/tools/syndocal-mcp/server.mjs' --expected-executable 'C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe'
codex mcp get syndocal --json
```

These are this workstation's paths; configure the actual checkout and Node paths
on another machine. A running client session may need to reload its MCP
connections before discovering the new tools. No credential belongs in this
configuration: the adapter reads the current process descriptor when called.

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

Only one broker request may be active. An overlapping call is explicitly rejected before dispatch. MCP input and native request frames are bounded to 64 KiB; native and MCP response frames are bounded to 256 KiB. Connection timeout is one second and native response timeout is three seconds. Nothing except newline-delimited MCP JSON-RPC is written to stdout.

## Validate

```text
node tools/syndocal-mcp/check.mjs
```

The integration check launches the CLI against its own fake loopback broker. It exercises negotiation, all nine tool schemas, canonical operation allowlisting, request correlation, Video BO false-success handling, pending/unknown behavior, no automatic retry, overlap rejection, malformed/oversized frames, executable mismatch and credential redaction. It does not operate Syndocal or physical devices. Real native bridge acceptance is a separate integration check.

The adapter implements the MCP **2025-11-25** [stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [initialization lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), and [tools interface](https://modelcontextprotocol.io/specification/2025-11-25/server/tools). Supported JSON-RPC methods are `initialize`, `notifications/initialized`, `ping`, `tools/list`, and `tools/call`.
