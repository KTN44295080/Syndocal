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
