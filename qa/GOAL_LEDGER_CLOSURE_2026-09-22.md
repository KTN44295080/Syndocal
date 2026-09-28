# Ledger closure execution checkpoint — 2026-09-22

User objective: autonomously close the ledgers. No subagents. Prefer MCP/backend
operations to Computer Use; implement missing operation paths first.

Start base: `5528c4fa` on `codex/showclock-review-20260912`.

## Verified starting inventory

Both machine validators pass: 58 Flow markers, 27 Complete, 23 Open, 8 Deferred;
32 Q1 requirements, 15 Q2 decisions, 14 Q3 risks, 156 Q4 records. This proves
mirror integrity only. Historical `GOAL_COMPLETION_AUDIT.md` completion statements
do not supersede current Flow sections 6–9 or the exact-artifact acceptance rule.

Deferred platform/distribution scope is not silently promoted. Hardware,
two-machine, licensing and signing decisions require their actual evidence.
No marking rows complete from source-only checks or rewriting acceptance to
substitute software observations for physical output.

## Active order

1. Address user-reported Preview Transport crushing and Control clarity first.
2. Complete real native MCP read-only acceptance, including pending terminal
   lookup and revoked/ungranted denial. Then implement missing bounded typed
   backend operations required by the native ingress and Control acceptance.
3. Audit stale Q2/Q3 wording against the 27 accepted implementation rows; close
   only risks whose required evidence is present and current.
4. Progress remaining native, security, migration, observability and accessibility
   rows, then available hardware/soak evidence. Maintain exact artifact identities.

## Native MCP discovery

An in-progress `tools/syndocal-mcp/check-native-readonly.mjs` runs a separate
stdio sidecar against the real native broker. Local administrative bootstrap uses
the existing backend commands through a process-verified loopback WebView2 CDP
connection; no DOM actions. It installs four exact read-only grants in safe mode,
revokes its own principal and deletes its ACL-restricted temporary credential.
No output, authored state or device command is issued.

Actual release build at this slice: MSVC 14.44.35207 pinned first on PATH;
build exit 0; executable SHA256
`6af2f2a80f72e9df8c54cbe37b9cb64f193e5f1ee336f4085d5311a9872ba347`.
The build includes pre-existing unrelated uncommitted frontend changes and is
not a clean frozen release candidate. The later UI rebuild supersedes it.

Actual MCP finding: a granted fixtures read returns pending. Querying its
original request ID then returns a native response-contract mismatch. Source
inspection finds `request.status` absent from the authority service's operation
mapping, even though the broker parses it and the adapter requires original-ID
correlation. Reproduce and repair authorization/correlation with principal-bound
receipt access; do not simply bypass authorization or expose other principals'
results. Fake-loopback adapter tests had not exercised this native path.

Temporary negative evidence:
`%TEMP%/syndocal-native-mcp-readonly-20260922-04.json`.
Its cleanup confirms principal revocation and credential removal. The runner
is unfinished, uncommitted work, not a passed gate.

## Protected pre-existing work

Do not stage or overwrite: `app/src/App.tsx`, `app/src/uiLocalization.ts`,
`app/src/remotePairingPin.ts`, `app/scripts/check-dj-link-runtime.mjs`,
`qa/AI3_NATIVE_INGRESS_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`, `.vite/`.
User's no-subagent instruction supersedes repository delegation preferences.
Do not claim independent review from a self-review.

## Native MCP repair accepted locally — 2026-09-29

Base `996a58bc`, same branch. The earlier negative probe is superseded for the
read-only native MCP slice by
[`artifacts/native-mcp-readonly-2026-09-29.json`](artifacts/native-mcp-readonly-2026-09-29.json).

Two defects were repaired: status lookup was sent through the new-operation
grant mapper, and renderer claim returned a stored dispatch whose principal
fields were empty. Requests now retain their authenticated principal/incarnation
before storage and dispatch. Receipt lookup authenticates the caller and checks
the exact owner; rejection retains the original queried ID. Mutation replay
hashes include the owner. Old unbound hashes remain conflict/replay fences;
restart receipts stay unknown and never expose prior results or trigger replay.
There is no durable JSON field/schema change or destructive migration.

The maintained native runner uses a process-verified main-window backend for
temporary pairing/grants, then an independent stdio MCP client for all reads.
Seven checks passed: ungranted denial; fixtures, runtime, capabilities and
recording reads through terminal receipt lookup; revoked read denial; revoked
receipt denial with original-ID correlation. It proved safe mode with four exact
read grants, revoked its own principal and deleted its ACL-restricted credential.
No DOM action, Computer Use, subagent, project mutation or device/output command.

Validation:

- `cargo test -p syndocal agent_bridge -- --nocapture`: 21 passed, none failed or
  ignored, including exact owner/incarnation isolation, claimed principal,
  revoked status, old unbound tombstones and restart no-replay. Approved MSVC
  14.44.35207 pinned and verified first on PATH. No Rust warnings.
- Fake adapter: 15 integration groups and 128 hostile frames; HTTP/REST/WS
  transport checks passed. Authority service and frontend bridge (11 groups)
  passed. These are separate from the real native evidence above.
- `pnpm.cmd --dir app tauri build --no-bundle`: exit 0, approved pinned linker,
  release compilation 3m50s. Rust warnings 0, unchanged from the UI checkpoint;
  existing Vite chunk-size advisory 1, unchanged and not suppressed.
- Exact executable SHA256:
  `06d90b3a202d4290b32d0e90391c690a9be447aa095f32f4f3b3e460be39a7ff`.
  PID 66956; one exact-path responsive `Syndocal` window; maximized using the
  backend and verified using Win32 `IsZoomed`.

The build includes the protected pre-existing frontend changes listed above;
it is not a clean installation/release artifact. Diff self-review covered auth,
ownership, bounded storage, replay and credential cleanup; independent security
review is not claimed. AI8 and the overall goal remain open: this slice does not
prove mutation/output, crash/update, physical topology, clean installation,
signing, licensing, venue rehearsal or native accessibility acceptance.

Next: use the now-working authenticated MCP path for bounded native operations
required by the remaining domain gates. Preserve physical/clean-machine/venue
and license requirements rather than closing them from software-only evidence.
