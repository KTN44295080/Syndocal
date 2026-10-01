# External MCP output lease observation

Branch `codex/showclock-review-20260912`, base `2d0e65e0`.

## Behavior and compatibility

External MCP clients can now call the existing
`syndocal.output.lease.authority.query.v1` through
`syndocal_execute_control_plane` with `request: {}`. The reviewed projection is
53 operations, including 19 R0 reads. The query requires an exact ExternalMcp
Read grant and works in Safe Mode. The native handler binds the observation to
the current trusted window owner and incarnation, returns current lease identity
and generation, and hides retired/expired/foreign-owner leases. No matching lease
returns `statuses: [{status: "unavailable"}]`.

The existing operation ID and response format are unchanged. Because its ID
starts with `syndocal.output.` rather than `syndocal.query.`, both native replay
classification and the frontend execution router explicitly treat it as a read.
It neither enters the output mutation executor nor persists a mutation tombstone.
No new native invoke, storage version, product version, or output behavior is added.

The native MCP probe obtains output fences and leases through its separate stdio
sidecar instead of its administrative backend bootstrap. Bootstrap remains limited
to pairing/grants and the existing owner-retirement lifecycle. The probe verifies
missing-grant rejection, granted Safe Mode reads, active lease identity/generation,
owner retirement hiding old leases, and empty observation after relinquishment.

## Evidence

- Production bridge processor/runtime: 12 groups passed, including routing to the
  read handler and truthful read failure classification.
- Adapter integration: 16 groups and 128 hostile stdio inputs passed.
- AI0 exact source coverage and AI1 query/event contracts passed. Inventories remain
  Tauri 542, Frontend 481, Engine 280, Remote 116, MIDI/OSC/DMX 206, Keyboard 33.
- Native `control_plane::tests` filter: 31 passed, zero failed or ignored. This
  substring also selects 16 authored-control-plane regressions; the remaining 15
  cover the registry/admission contracts. The first run exposed a stale internal
  route-count assertion, 14 versus the actual 15. It now explicitly checks the
  existing native MCP executor's maintenance classification and the exact count.
  Legacy/source totals are also synchronized to 1625/1658, with 53 reviewed direct
  operations and 1119 unclassified sources. Assertions remain exact.
- Native `agent_bridge` filter: 22 passed, zero failed or ignored, including lease
  read redispatch after restart and persistent mutation replay rejection. Both
  Rust test runs used release dependencies with the application test package
  `opt-level=0`, `codegen-units=256`; they are not the optimized application build.
- Final optimized isolated Windows build passed. Frontend build passed with the
  existing Vite chunk advisory. Native executable SHA-256:
  `73952ab1c734bc8a24597fc5613910e2f6b1790142592ee1a723a3a3d93ef94b`.
- [Real native evidence](artifacts/native-mcp-lease-authority-2026-10-02.json):
  12 checks passed, including the new reads, existing unattended R4/R5 tests, and
  seven crash/restart/credential lifecycle checks. Each launch verified one
  responsive maximized exact-path QA window. Normal app identity was unchanged;
  the QA process, sidecar, principal and credential were cleaned up. No native
  panic was observed. Lighting and Video output remained denied.

The first native probe returned the existing explicitly retryable `Overloaded`
query error during a fence read. It failed and remains preserved at
`%TEMP%/syndocal-mcp-lease-authority-2026-10-02-v1.json`; its cleanup passed.
The QA helper now permits at most three new read intents only after a terminal
`request_rejected` containing that exact retryable lock-contention error, and
records each such rejection. Pending/unknown responses, other errors and all
mutations are never retried. Product/sidecar retry policy is unchanged. The linked
positive evidence is v2; service rejection and success assertions remain intact.

All Windows Cargo runs initialized and verified MSVC 14.44.35207 with its absolute
Build Tools linker pinned first on PATH. Rust warnings were zero, unchanged from
the preceding checkpoint; the existing Vite advisory remains one. The optimized
build preceded only the final test-count corrections, which are excluded from the
application's production code. The built production behavior was tested directly.

## Boundary and next action

This completes the typed lease-query gap. It does not close AI8, physical output,
release/clean-machine, accessibility, venue, or broader migration/support acceptance.
The ledger remains 27 Complete, 23 Open, 8 Deferred. Protected other-owner changes
remain unstaged; no subagents or Computer Use were used. Review was by the
implementing agent. Continue with native high-risk authority revocation at the
execution boundary and other executable security/domain evidence.
