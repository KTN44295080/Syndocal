# Syndocal MCP adapter

Dependency-free Node.js sidecar for a running Syndocal agent bridge. It exposes fixture listing, fixture reading, exact-project fixture transforms, exact-project Video BO control, request-status lookup, bounded runtime diagnostics, canonical backend capability discovery, read-only recording status, and the 53 reviewed canonical control-plane operations. It does not open devices, start Syndocal, or bypass the backend's output ownership, lease, safety, or typed-command checks.

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

On Windows, each request verifies the live PID's executable through `Get-Process`
before opening the native bridge. OS inspection has a separate 10-second bound
including PowerShell startup; the result is never cached. Failure or an absent,
inaccessible or mismatched executable rejects the request before dispatch. The
subsequent connection and native-response deadlines remain one and three seconds.
Clients should allow at least 15 seconds for that complete request path. This
does not add retries or change unknown-mutation recovery rules.

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

On Windows, process identity is read through a fixed, hidden PowerShell `Get-Process` query with a ten-second pre-dispatch deadline, including shell startup. No result is cached and no tool can supply commands or scripts. On Linux, the test-compatible equivalent reads `/proc/<pid>/exe`. Other platforms without that interface fail closed. The descriptor credential is never included in diagnostics; broker strings and keys are redacted if they echo it. Keep the descriptor private to your local account.

## Tools

- `syndocal_list_fixtures({})`: read fixtures and the current project identity.
- `syndocal_get_fixture({fixtureId})`: read one fixture and project identity.
- `syndocal_set_fixture_transform({requestId, fixtureId, position, rotation, expectedProject})`: send one complete transform with a caller-supplied UUID and the exact project identity returned by a read.
- `syndocal_set_video_blackout({requestId, enabled, expectedProject})`: set Video BO with a caller-supplied UUID and the exact project identity returned by a read. It requires both lighting and video output ownership to already be active; disabling may reveal that existing output and never arms, acquires, or enables output.
- `syndocal_get_request_status({requestId})`: query the original request UUID.
- `syndocal_get_runtime_status({})`: read the project token, lighting/video blackout bits, up to 64 video-output summaries, Timeline transport state, the exact `timeline_runtime` projection, and a separate `observations.output_ownership_status` read. The authority bundle and ownership observation are captured by separate reads and must not be treated as one atomic image.
- `syndocal_get_control_plane_capabilities({})`: read a bounded projection of the backend-owned canonical operation/source inventory and exact local adapter policy. `FailClosed` entries are discovery-only and cannot be invoked through MCP.
- `syndocal_get_recording_status({})`: read bounded active recording, dimensions, frame/drop counters, audio inclusion, path, and last-error state. It never starts, stops, finalizes, or replaces a recording.
- `syndocal_execute_control_plane({requestId,operationId,request})`: execute one of the 53 reviewed canonical operations through a static typed Tauri adapter. `operationId` must be present in the capability registry and `request` must be that operation's exact typed request object. Unreviewed or `FailClosed` inventory entries are rejected.

`syndocal.output.lease.authority.query.v1` takes `request: {}` and requires an
exact ExternalMcp Read grant. It returns at most 64 current leases belonging to
the trusted renderer's current native owner, with their identity and generation;
no matching lease returns `statuses: [{status: "unavailable"}]`. Safe Mode permits
the granted read. Despite its `syndocal.output.` prefix, it is an R0 observation:
it never enters the output mutation executor or persists a mutation tombstone.
Owner retirement hides the old owner's leases from subsequent reads. The query
does not acquire, renew, recover, transfer or energize output.

Position is `{x,y,z}` and rotation is `{pitch,yaw,roll}`. `expectedProject` is `{project_epoch,project_revision,checkpoint_hash}`. All fields are required; transform coordinates must be finite, project epoch/revision values must be nonnegative safe integers, and `checkpoint_hash` must be exactly 64 lowercase hexadecimal characters. Unknown argument fields are rejected.

A new mutation intent needs a new lowercase, hyphenated UUID. Uppercase UUIDs are rejected so one intent has a single request identity. Preserve that UUID until its result is known. `pending` and `unknown` are tool errors with a status-query instruction; the adapter does not retry or automatically poll. A timeout after a mutation may have been sent is `unknown`, never proof that it was not applied. Query the original UUID before deciding any next action. Status requests use a fresh transport envelope UUID but the broker response identifies the queried original UUID. Broker `completed` means terminal processing; only a result with `ok:true` is tool success, so a completed `{ok:false}` response remains an error with its original result. The canonical executor remains a bounded static map; it is not arbitrary `invoke(command, params)`. Query operations are not persisted as mutation tombstones, while canonical mutations retain the same durable replay protection.

Only one broker request may be active per transport session, including discovery. Across all transports, one sidecar process admits at most eight native requests before reading the descriptor or credential, verifying the OS process, or opening a native socket. The slot remains owned until process verification and the native socket have finished. There is no admission queue or automatic retry. Overlapping `tools/list` returns JSON-RPC `-32005`; an overlapping tool call now returns structured JSON with the original mutation/status UUID, `status: rejected`, `error: sidecar_overloaded`, and an actionable `nextAction` instead of plain text. These rejections are definitely unsent; authentication failures remain separate. MCP, HTTP, REST, and WebSocket input frames are bounded to 64 KiB; native and sidecar response frames are bounded to 256 KiB. Connection timeout is one second and native response timeout is three seconds. Every broker request carries a fresh client nonce and HMAC-SHA256 proof bound to the descriptor's per-launch session nonce, principal incarnation, request ID, and method. The native broker rejects missing, stale, invalid, or replayed proof before renderer dispatch and then applies the exact ExternalMcp grant. Promoted external MCP principals execute granted R4/R5 operations without individual human approval. Output operations consume the immutable claimed request in the native backend and keep domain fences, leases, rate limits, audits and receipts. The stdio mode writes nothing except newline-delimited MCP JSON-RPC to stdout.

Discovery retains its session and process admission slot while settling a native
`pending` capability receipt through at most four status lookups, bounded by a
three-second settling window. Every lookup re-verifies the process and current
authentication. The capability request is issued exactly once. Native process
inspection retains its separate ten-second per-call deadline; an inspection
already in progress is reaped before releasing the slot. Only a successful
completed receipt exposes tools; revocation still returns `-32001`, while
incomplete or failed authenticated discovery returns `-32003`. No authentication
result is cached.

`node tools/syndocal-mcp/check-request-capacity.mjs` uses an owned protocol fixture
and the real OS process check to verify discovery/call exclusion, 10,000 intents,
eight admitted requests, unsent overflow, and slot release after failures and
uncertain mutation replies. It is included in `check:ai5-sidecar` and is not a
native-product acceptance test.

The opt-in native `--controller-burst` lane sends 10,000 discovery requests and
10,000 R4 master intents through a real stdio sidecar while receiving all 512
ArtDMX channels on an owned loopback socket. It records explicit overloads,
admitted completion/replay, fresh post-burst output and observed packet intervals.
It uses the isolated lifecycle executable and the same argument/cleanup boundary
as `--controller-output`. The lane cannot be combined with other live lanes.
Packet intervals alone do not establish the complete tick/frame/audio/UI,
local priority Blackout, event publisher, physical-device or venue budgets.

The separate opt-in `--controller-safety-pressure` lane retains real native
requests through the existing renderer-registration boundary, fills all 64
broker detail slots, and sends 10,000 additional R4 intents. It verifies local
native S0 engage, received all-zero 512-channel ArtDMX, retained pending requests,
claimed R4/R5 rejection after revocation and during Kill Switch, no diagnostic
artifact, idempotent S0 replay/no-op and rejection of a target-valued payload.
Each of its two cases uses a fresh isolated QA process, exact grants and an
owned software-loopback sender/receiver. It cannot be combined with other live
lanes. The recorded latency is backend-to-loopback evidence, not a physical
button, Engine command-queue, complete realtime budget or venue test.

`syndocal_export_diagnostics` requires the exact File grant for
`syndocal.diagnostics.export.v1`. It writes a sanitized ZIP to a new absolute
path without a preview or approval dialog. Existing targets are rejected;
request identity is retained across restart to prevent automatic replay.

## Validate

For the normal Windows executable without a debugger, run:

```text
node tools/syndocal-mcp/check-native-window-backend.mjs --expected-executable <absolute-exe> --evidence <new-absolute-json-path>
```

This pins the live descriptor/process to the selected executable, checks one
visible responsive `Syndocal` window, records its actual maximized state, and
proves that an unauthenticated broker read returns `agent_authentication_required`
without a result. It performs no UI action, pairing, project read or mutation.
Authenticated MCP and Control layout acceptance remain separate probes.

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

Add `--diagnostics` to verify the local native diagnostic export backend. It
checks capture preview, cancellation, exact ZIP digest, new-file-only atomic
publication, a destination appearing after preview, replay rejection, and
capture expiry across a native restart. Use a new evidence path for every run.
This tests a private temporary destination through a process-verified native
backend session and covers the local preview/acknowledge flow.

Add `--external-high-risk` to test unattended grants through a separate stdio MCP
sidecar, diagnostic ZIP export/replay/no-overwrite, and isolated acquire/renew/
force-transfer/relinquish lease authority. It proves same-owner and stale-generation
rejection, followed by a successful transfer after owner retirement. The empty QA
project has no physical output activation.

Add `--external-revocation` to admit and claim real R4/R5 stdio MCP requests,
revoke their temporary principal, and then attempt native execution. Both must
return `agent_principal_revoked`; a second attempt must return
`request_not_executable`, and the revoked caller must not read the receipt.
No diagnostic file or output lease may be created. The helper registers a new
QA renderer generation immediately before graceful close so the native
claim/revoke/execute order is deterministic without DOM actions. Its credentials
inherit the runner's private ACL and are revoked and removed. It can be combined
with `--external-high-risk` for the full 14-check native slice.

Add `--lease-expiry` as a separate lease lane to verify real stdio disconnect/
reconnect and the backend's 60-second monotonic TTL without renewal or a fake
clock. It checks expired renewal, terminal replay, stale generation rejection,
explicit recovery and relinquishment. The stdio adapter is not the native
window's lease owner; reconnect preserves that live owner, while TTL expiry
revokes authority. Persisted output configuration remains unchanged and both
runtime output gates stay closed. This is not physical signal or complete
controller-loss/re-arm acceptance. Do not combine it with the other lease probes.

Add `--controller-output` as a separate live software-output lane. It loads one
private fixture and an Art-Net route to an owned ephemeral `127.0.0.1` UDP port.
Real authenticated stdio MCP grants and Arm start native output, and the receiver
validates complete 512-channel ArtDMX images. Registered native owner retirement,
stale lease rejection, ForceTransfer/replay/conflict and relinquishment must
preserve the live image and output gates. Only an explicit master operation by
the new owner may change Dimmer from 255 to 128. Cleanup uses the existing native
project-replacement command to disarm the isolated route; retired raw role
commands remain unavailable. The helper closes its receiver, sidecar and private
files, and the runner revokes its credential and terminates only its exact QA
process. Do not combine this lane with the other opt-in probes. This records
actual software loopback reception, not physical fixtures, video, process/worker
loss, full controller-loss/re-arm or venue acceptance.

Add `--controller-restart` as a separate native process-loss lane. It uses the
same independent loopback project and receiver, disconnects the owned stdio
client while live DMX continues, then terminates and relaunches only the exact
isolated QA executable. The receiver must observe silence until a fresh exact
lease and separate explicit R4 Arm. Reusing owner text or the persisted desired
role cannot reclaim authority. Previous-process terminal lookup and exact UUID
replay stay `unknown` without execution; a different shape under that UUID
conflicts, and an old process fence or an unclaimed old lease rejects. Explicit
private project reload and new lease acquisition do not resume output. New Arm
resumes the independently authored full image, and exact replay stays idempotent.
The helper cleans the same owned resources as `--controller-output`; do not
combine these flags or other opt-in lanes. This is software loopback process-loss
and re-Arm evidence, not physical fixtures/video, worker-specific failure,
in-flight mutation crash durability or complete controller-loss acceptance.

Add `--controller-expiry` as a separate live native lease-expiry lane. It uses
the same private loopback project and full 512-channel oracle. Without renewals
or a synthetic clock, it waits for the production 60-second monotonic TTL while
validating every received image and requiring fresh frames at each observation
interval. R0 hides expired authority; a rejected explicit
renewal observes the orphan transition once. Old renew/Arm/master operations,
terminal replay and changed-shape recovery conflict must not change the image
or runtime gates. Explicit recovery advances authority only; a separate explicit
master changes Dimmer from 255 to 128. Relinquishment preserves that live image
and blocks further output. Final private project replacement stops the sender.
Do not combine this flag with other opt-in lanes. This is software loopback
expiry/recovery evidence, not physical fixtures/video, worker-specific failure,
in-flight mutation crash durability or complete controller-loss acceptance.

Add `--controller-inflight` as a separate native R4/R5 interruption lane. It
uses the same owned loopback fixture/project and tests Lighting master plus
diagnostic ZIP export at three broker boundaries: queued, claimed, and native
effect committed before the broker receives completion. The existing native
bridge registration retires its automatic renderer; authenticated stdio
admission, immutable claim and the production native executor remain in use.
No DOM action, invoke patch, fake engine or new product hook is involved.
The runner terminates and restarts only the isolated QA executable after each
case. Fresh pairing/grants cannot reclaim a lease or resume output. Old pending
UUID lookup stays unknown; exact persisted identity is never dispatched again,
and changed shape conflicts. Committed export bytes remain identical; requests
that never executed create no file. Only fresh acquisition plus a separate
explicit Arm resumes the independently authored full output image. All new
output intents share one request-ID allocator so the native high-water fence
remains enforced. Do not combine this flag with other opt-in lanes. This is
bounded R4 master/R5 diagnostic broker-interruption proof, not physical fixtures,
recording/authored crash matrices, worker-internal interruption, complete durable
mutation completion, clean installation or release acceptance.

Canonical native QueryError objects retain their bounded `code`, `message`,
`retryable` and `resnapshot_required` fields in `result.error.native_query` for
rejected reads. The outer error code stays `request_rejected`. Unrecognized
exception objects return a generic bounded error; arbitrary fields are not
forwarded. Uncertain mutations remain `mutation_not_confirmed` and never gain
read retry information. The product and sidecar do not retry requests. QA probes
may issue up to three new read intents only after a terminal explicitly retryable
Overloaded rejection; pending/unknown results and mutations are never retried.

Add `--project-json` as a separate project lane for real native `.sdc` admission.
It loads a private empty project from a Unicode path, then checks duplicate root/
nested/escaped keys, truncation, invalid UTF-8, future versions and an oversized
file. Rejected loads must preserve the authored checkpoint, authority/publication/
history/recovery/path counters, current path, output gates and original file hash.
The helper uses the production `load_project_path` backend command with the
current owner and E/R/H fence; it opens no dialog and issues no Enable/Arm action.
It removes only its prefixed temporary directory. This is native backend
acceptance, not an external MCP project-load tool or complete migration proof.
Do not combine it with the other project/lease/diagnostic/Tap probes.

Add `--backup-json` as a separate backup lane. It creates only exclusively owned
files in the empty isolated QA `project-backups` directory, verifies a real native
restore, and rejects mismatched/zero IDs, versions, duplicate ignored keys,
truncation, UTF-8 failure and an over-128-MiB file. Rejected loads must preserve
the active project/authority/output and source hashes. Only the valid backup is
listed, and an explicit request can restore it after newer corrupt files exist.
It never chooses an implicit fallback, publishes/prunes a backup, or reads the
normal-profile backup contents. Cleanup removes only the files it exclusively
created. Do not combine this lane with any other optional project/lease/output/
diagnostic/Tap probe; it is bounded native backend acceptance, not full O1-O4.

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

The integration check launches the CLI against its own fake loopback broker. It exercises negotiation, all ten tool schemas, canonical operation allowlisting, request correlation, Video BO false-success handling, pending/unknown behavior, no automatic retry, overlap rejection, malformed/oversized frames, executable mismatch and credential redaction. The transport check exercises health, JSON-RPC over HTTP, the REST facade, WebSocket JSON-RPC, and the same nonce proof against a fake loopback broker. These checks do not operate Syndocal or physical devices. Real native bridge acceptance is a separate integration check.

The adapter implements the MCP **2025-11-25** [stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [initialization lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), and [tools interface](https://modelcontextprotocol.io/specification/2025-11-25/server/tools). Supported JSON-RPC methods are `initialize`, `notifications/initialized`, `ping`, `tools/list`, and `tools/call`. The HTTP and WebSocket transports reuse the same JSON-RPC dispatcher and native authentication path; they do not add a second command registry.
