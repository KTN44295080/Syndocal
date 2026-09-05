# Fixture MCP vertical

Branch: `codex/syndocal-v1.2`. Base: `ec504d1767dc3f81ecc4d3358bde7d209f73049b`.
Product remains `1.2.0-alpha.69`; this is an internal integration checkpoint.

## Boundary

The Node stdio adapter exposes four fixed tools: list fixtures, read one fixture,
set its complete position/rotation with an expected project token, and query an
operation UUID. No computer-use surface, arbitrary invoke, script execution,
output activation, project replacement, or disk-save tool is exposed.

Native owns a loopback broker and a private descriptor in
`%LOCALAPPDATA%/jp.seraf.ktn.syndocal/agent-bridge-v1.json`.
The descriptor and ledger use a protected current-user SID DACL and atomic file
replacement. The adapter checks the requested executable and running process.
Its fixed Windows CIM helper runs hidden. Never copy the descriptor token into
logs, screenshots, or QA evidence.

Events only wake the main renderer. A native single-use claim returns the
canonical request; an event cannot supply its own editing payload. Three
`AgentTransportMaintenance` commands own registration, claim and completion.
Actual edits still use the existing GUI `set_fixture_transform` transaction,
operator-lock checks and epoch/revision/hash compare-and-swap. No new engine
editing path or renderer-owner impersonation was added.

Successful mutation results verify the committed project state through a fresh
atomic authority bundle, within the existing 0.01 transform precision. They do
not claim that the `.sdc` file was saved. Listing returns at most 256 fixtures,
with explicit total/truncated fields; pagination is not implemented.

## Replay, resource and performance boundaries

Mutation identities are recorded before dispatch. Same UUID with different
content is rejected. Duplicate claims cannot execute. Reload invalidates pending
requests; restart retains mutation identities as unknown and never replays them.
An unknown outcome requires inspection, not an automatically generated retry.

Limits: 64 KiB request, 255 KiB result, 256 KiB response frame, 8 concurrent
connections, 2-second native socket deadlines, 64 detailed requests (unfinished
ones never evicted), 4096 durable mutation identities and a 20 MiB ledger.
Capacity exhaustion is explicit; do not delete the ledger to retry an uncertain
operation. There is no per-frame project/fixture polling. Project snapshots and
ledger writes happen on tool requests, outside the video/DMX render path.

## Evidence

- `node tools/syndocal-mcp/check.mjs`: 11 integration groups PASS, fake broker.
- `node app/scripts/check-agent-bridge.mjs`: 6 groups PASS, production frontend
  modules; stale project, mismatch, fake/duplicate event and disposal coverage.
- `pnpm --dir app exec tsc --noEmit`: PASS.
- `pnpm --dir app run check:frontend-invokes`: PASS, 454 commands.
- Exact MSVC wrapper `node target/qa/recording-atomic-20260905/run-native.mjs`
  with `cargo test -p syndocal --locked agent_bridge -- --nocapture --test-threads=1`:
  6 PASS, 1721 filtered. Native ACL file creation is exercised by restart tests.
- Same wrapper with `cargo test -p syndocal --locked control_plane::tests -- --nocapture --test-threads=1`:
  30 PASS, 1697 filtered. Inventory: 514 native routes, 454 frontend aliases,
  1569 legacy / 1602 canonical sources, 1096 unclassified native/other sources.
  The initial count expectation incorrectly counted three aliases as unclassified;
  exact-set inspection corrected that expectation before the passing rerun.
- Independent source review: ACCEPT after native/Node instanceId mismatch was
  repaired; Node regression now uses native 32-lowercase-hex descriptor shape.
- `pnpm --dir app tauri build --no-bundle`: PASS, release compile 4m 02s.
  Exact linker pin/PATH-first and exact-checkout process preflight are recorded in
  `target/qa/snapshot-cleanup-20260905/agent-bridge-native-build.log`.
- Launched exact checkout executable: PID 101284, one responsive maximized
  `Syndocal` main window. SHA256
  `B34F3C032A485473DA698F397B7A85750D09FB6AC2BBBB6327BBAF20CF062B0B`.
  Evidence: `agent-bridge-launch.json` in the same QA directory.
- `node target/qa/snapshot-cleanup-20260905/probe-agent-mcp.mjs`: real stdio
  initialize/tools-list/fixture-list/request-status round-trip PASS against the
  running native process. Result: completed/ok, empty startup project, zero
  fixtures, zero mutations. Evidence: `agent-bridge-native-probe.json`.
- Real descriptor ACL: protected, one non-inherited current-user FullControl
  entry. `agent-bridge-acl.json` records ACL only, no credential.
- `codex mcp add syndocal -- 'C:/nvm4w/nodejs/node.exe' 'C:/Users/kouty/Documents/KDMX/tools/syndocal-mcp/server.mjs' --expected-executable 'C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe'`:
  PASS. `codex mcp get syndocal --json` confirms the enabled stdio configuration.
  Other MCP server entries were not changed. The current task's loaded tools are
  not hot-reloaded; actual Codex tool discovery after reconnect remains separate
  from the successful equivalent stdio probe.

Compiler warning baseline was zero. Current native tests, TypeScript and native
release build: zero first-party warnings, delta zero.

## Remaining acceptance

Real MCP read, client registration and native release launch passed. Hardware/Unity
output is not exercised by these tests. The reported drag-release rotation regression is still a separate open
item; this bridge does not claim to repair it. GUI Undo/Redo and an actual changed
fixture round-trip remain distinct from mock and transaction-unit evidence.
First next action: after the user opens their project, use fixture list/get,
capture its exact project token, then perform a bounded transform/verification
with the user's chosen fixture. Do not invent a fixture or open a project through
an undocumented UI/IPC route to manufacture that acceptance.

Preserve the unrelated dirty `app/scripts/check-viewport-containment.mjs` and
the user's Unity and test `.sdc` files. No cache cleanup is part of this checkpoint.
