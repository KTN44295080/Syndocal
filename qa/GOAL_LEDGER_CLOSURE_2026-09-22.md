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

## HTTP/REST/WebSocket native and security slice — 2026-09-29

Base `fc5db89c`. Continuing after the read-only checkpoint found two reproducible
transport defects: a foreign browser Origin received HTTP 200 and reached the
fake broker; one incomplete WebSocket header starved the adapter event loop,
causing an independent HTTP health request to time out. Baseline reproduction
ran in disposable child processes with no native/device calls.

The sidecar now validates literal loopback Host plus actual port and exact
same-origin browser requests before HTTP/WS dispatch. Non-browser Origin-less
clients remain supported. Malformed REST percent encoding returns 400 instead
of rejecting the request handler promise. Incomplete WebSocket frames wait for
new data rather than rescheduling themselves continuously. No native source,
project schema or output behavior changed in this slice.

`check-transport-security.mjs` verifies foreign/null/wrong-port origins, rebound
Host, cross-site metadata, positive same-origin/Origin-less clients, malformed
path recovery, denied WebSocket upgrade, and split-header/payload liveness and
successful reconstruction. The maintained AI5 gate runs it automatically.

Real native evidence:
[`artifacts/native-mcp-network-2026-09-29.json`](artifacts/native-mcp-network-2026-09-29.json).
The extended runner passed 13 checks: the previous seven stdio checks plus
terminal runtime reads and revoked-principal rejection over HTTP JSON-RPC,
REST and WebSocket. Native executable identity is unchanged from the preceding
slice. The HTTP adapter runs in the harness process; clients use real loopback
sockets to the real native broker. Temporary principal/credential cleanup passed.
No native rebuild is required for sidecar-only changes; no new native-build or
hardware acceptance is claimed. The whole AI8/Remote security markers remain
open for their broader clean-install/output/restart/review requirements.

## Isolated native crash/restart slice — 2026-09-29

Base `0fdd6af9`. The regular application was preserved as PID 66956. A separate
`jp.seraf.ktn.syndocal.qa.mcp-lifecycle` release configuration and profile enabled
real process termination/restart without adopting or changing user projects.
Preparing this exposed a build-wrapper ownership bug: `CARGO_TARGET_DIR` was
ignored when stopping the executable. The wrapper now selects the exact absolute
override's release executable and rejects ambiguous relative overrides. The
default checkout path and pinned MSVC procedure remain unchanged. Its maintained
checker passed 247 assertions and 27 hostile fixtures.

Isolated `pnpm.cmd --dir app tauri build --no-bundle --config
src-tauri/tauri.mcp-lifecycle.conf.json` passed using the absolute temporary
target directory and exact MSVC 14.44.35207 linker first on PATH. Build duration
8m18s; Rust warnings 0, same as the preceding normal build; unchanged Vite chunk
advisory 1. QA executable SHA256:
`bcd5ba65d3e58e6d9ca4c7db6377b09ee3574d4d35b6701e2785b64b10fd510f`.

[`artifacts/native-mcp-lifecycle-2026-09-29.json`](artifacts/native-mcp-lifecycle-2026-09-29.json)
records seven successful checks: initial read, dead-process descriptor rejection,
retired-principal rejection after forced termination, old-credential/receipt
non-adoption, old-launch proof rejection with successful fresh read, graceful
native close, and no restored grant after graceful restart. Each of three launches
verified its exact PID/executable, one responsive maximized QA window, and zero
fixtures/video outputs with stopped Timeline. The normal descriptor was byte-for-
byte unchanged. QA processes, temporary credential file and OS credential were
cleaned up. The empty QA profile remains as machine-local QA state.

The process-verified backend bootstrap was extracted for reuse. The normal-app
13-check stdio/HTTP/REST/WebSocket runner passed again after extraction; evidence
is `%TEMP%/syndocal-native-mcp-helper-refactor-20260929-01.json`. No Computer Use,
subagent, project mutation, physical output or normal-app restart was performed.
This proves the isolated release-build broker lifecycle, not durable mutation
publication, clean installation, hardware, or whole AI8 acceptance. Existing
pre-owned frontend changes remain in the builds and outside this commit's scope.

## HTTP session recovery slice — 2026-09-29

Base `1ddcb02a`. Inspection found that the 64-entry HTTP negotiation map never
removed sessions: `touched` was written but never read. Sequential abandoned
clients could permanently prevent new sessions until the sidecar restarted.
The bounded map now lazily expires sessions after five idle minutes using a
monotonic clock. `DELETE /rpc` releases an idle session explicitly; unknown and
busy sessions return 404 and 409. Every asynchronous dispatch, including tool
discovery, pins its session until completion. Expiry resets negotiation only,
never native grants or request receipts. Origin/Host validation also guards DELETE.

`node tools/syndocal-mcp/check-transport-security.mjs` passed with exit 0:
64 real HTTP initializations, overflow rejection, foreign-origin deletion
rejection, explicit release and recovered capacity, plus deterministic expiry,
renegotiation, completion-time idle interval and in-flight retention checks.
Existing split-frame/Origin/path tests passed. Focused `git diff --check` passed.
No compiler ran, so there is no new compiler-warning measurement.

The broader AI5 gate did NOT pass on this run: its unchanged Windows process
identity query exceeded its two-second deadline twice. A direct measurement of
the WMI query took 3.456 seconds. Trying Get-Process also timed out under the
current host load, so that experiment was reverted rather than changing the
identity boundary or deadline. The native read runner failed at the same
pre-dispatch identity check; no new native receipt evidence was produced.
This slice is focused transport evidence, not full AI5/native acceptance.
Next: resolve/retest the host process-query timing boundary, then finish the
consent-preserving diagnostic capture/export backend route. Diagnostic export
currently binds preview and publication to the same captured bytes, and its
native confirmation must not be bypassed by an arbitrary invoke adapter.
High-level ledger statuses remain unchanged; goal remains active.

## Windows inspection recovery — 2026-09-29

Base `825a5c1f`. The repeated process inspection failure was reproduced before
editing. Windows now queries the live PID with Get-Process, avoiding WMI for the
executable path, with a separate bounded ten-second pre-dispatch budget including
PowerShell startup. No result is cached, no retry is added, and absence, access
failure or path mismatch still rejects before opening the broker connection.
The one-second connect and three-second native response deadlines are unchanged.
HTTP/WS test clients allow 15 seconds for this complete path; content and rejection
assertions are unchanged. The integration check additionally rejects a descriptor
whose claimed and expected paths agree but whose live PID names another executable,
and rejects a terminated fixture PID without dispatching to the broker.

`node app/scripts/check-ai5-sidecar.mjs` passed all adapter, HTTP/REST/WebSocket
and security groups. After adding the two OS-identity rejection cases,
`node tools/syndocal-mcp/check.mjs` passed again (15 integration groups and 128
hostile stdio inputs). The QA debugger ownership bootstrap now obtains one
process-parent snapshot instead of invoking WMI separately for each ancestor;
the same exact app ancestry, single IPv4 loopback listener and 12-level bound
remain enforced, within its unchanged ten-second budget.

The real native runner then passed all 13 stdio/HTTP/REST/WebSocket read and
revocation checks against the existing executable. Evidence:
[`artifacts/native-mcp-inspection-recovery-2026-09-29.json`](artifacts/native-mcp-inspection-recovery-2026-09-29.json).
Temporary principal/credential cleanup passed. This resolves the previous slice's
process-inspection validation failure; it does not establish hardware, diagnostic
export or whole AI8 acceptance. No native executable rebuild was needed for these
sidecar/QA-only edits, and no compiler-warning measurement is claimed.

## Native diagnostic export backend — 2026-09-30

Base `7c3f0cb1`. The local trusted main window now has a two-step backend
diagnostic export: prepare captures the existing sanitized package once and
returns its summary, SHA-256, destination and a 120-second single-use receipt;
finish requires the same digest and explicit approval. Cancellation writes
nothing. Approval creates a new ZIP atomically in the destination directory;
an existing target, including one created after preview, is never replaced.
Receipts are bounded to eight per process, expire, and cannot survive restart
or be replayed after publication starts. The existing GUI preview/save-dialog
workflow still uses the same capture content and its existing publication path.
The route inventory is 541 exact Tauri commands, and the two new file-export
commands fail closed in the external control-plane registry.

With pinned MSVC 14.44.35207, 43 focused Rust diagnostic tests passed with
zero Rust warnings; the exact route, frontend invoke and backend operator
contract checks passed. The isolated QA release build passed with one existing
Vite chunk-size advisory. Its executable SHA-256 is
`d96ae25682fef97dd70fc8539b52c18f489f5f4fd5610bdc3b9c8f10146b4902`.
[`artifacts/native-diagnostic-export-2026-09-30.json`](artifacts/native-diagnostic-export-2026-09-30.json)
records nine passing native checks, including a 4,785-byte ZIP whose captured
and written SHA-256 matched, and confirms the normal app identity stayed
unchanged. No Computer Use, subagent, project mutation or physical output was
used. Pre-existing frontend changes were present in the QA build and remain
outside this checkpoint's owned diff.

This closes the local backend diagnostic export slice. External MCP export
still needs an authenticated R5, human-present consent adapter and acceptance;
the updater failure matrix, deployed support drills, clean installation and
physical/venue gates are not established. `OBSERVABILITY-SUPPORT-001` remains
Open, and the overall goal remains active.

## Operator and support runbook draft — 2026-09-30

Base `47dcfe8f`. The source-grounded
[`SHOW_OPERATOR_SUPPORT_RUNBOOK.md`](SHOW_OPERATOR_SUPPORT_RUNBOOK.md)
now covers the P4 procedure list: pre-show checks, output Arm/Standby/Blackout,
device loss, takeover, project recovery, media/profile relink, recording
recovery, diagnostic export, planned update/rollback, emergency shutdown, and
post-incident evidence. It links the existing update, project-publication and
diagnostic source documents instead of copying release commands. The app's
current Project menu and Control > Both labels were checked in source. This is
a draft procedure, not a deployed operator drill; the observability ledger
remains Open pending actual drills and the other P gates.

## Actual Control > Both native surface proof — 2026-09-30

Base `0956b5be` contains the Control > Both upper-desk repair. The earlier
`check:control-upper-workspaces` runner was found to select the top-level Edit
workspace; the actual Control top-level workspace is the `touch` route. The
new `check:touch` assertions covered the requested top-one/bottom-two geometry
at five viewports, plus a focused `1280x752` run. Localization, frontend build,
and isolated native build passed. The separately maintained native QA runner
now selects only Control > Both, verifies one exact-path responsive maximized
window and the same geometry at `1280x752` CSS pixels, and records a directly
inspected [screenshot](artifacts/native-control-both-2026-09-30.png) with
[machine evidence](artifacts/native-control-both-2026-09-30.json). The normal
app identity remained unchanged and the QA process was closed. No output
control was activated. `UI-H5-CONTROL-001` remains Open for the independent
physical, operational, accessibility, and recovery acceptance boundaries.

The same native probe also recorded [Control > Video](artifacts/native-control-video-2026-09-30.png)
and its [geometry evidence](artifacts/native-control-video-2026-09-30.json).
This current route contains Clip Bank, not `PREVIEW TRANSPORT`; source places
that component under Edit > Video Mixer. The user's cropped image alone does
not identify its original route, so this is a current-source finding, not a
claim that the depicted panel was repaired in Control.

The current Control > Video empty project had displayed 32 unusable slot
menus and disabled transition fields. This is now an import-led empty state;
the authored 32-slot flow remains available when a layer exists. The
[final native image](artifacts/native-control-video-empty-2026-09-30.png) and
[machine proof](artifacts/native-control-video-empty-2026-09-30.json) show the
blank state in one exact-path maximized QA window. Five-viewport authored
Clip Bank browser checks, the Control viewport gate, localization, and the
isolated native build passed. The Control UI ledger remains Open for the
broader live/hardware/accessibility/recovery acceptance.

The native QA probe now also covers the [Lighting view](artifacts/native-control-lighting-2026-09-30.png)
with [geometry evidence](artifacts/native-control-lighting-2026-09-30.json):
the existing editable Touch desk occupies the upper row, retains its internal
scroll, and leaves Stage/Faders side by side below. This adds actual Control
route coverage, not an aesthetic or physical-output acceptance claim.

## Local consent expiry clock hardening — 2026-09-30

The local authority service now computes consent preparation and consumption
time from its backend-owned monotonic clock. Its trusted-window commands no
longer accept a renderer-provided `nowMs`; the 15-second single-use UI flow
and exact-context binding are unchanged. This prevents a renderer-selected
past or future timestamp from influencing expiry. The service test also covers
an elapsed one-millisecond consent alongside successful immediate consumption
and replay rejection. This change does not open the external MCP R4/R5 route:
the current bridge still fails closed pending an explicitly reviewed,
human-present adapter and its acceptance evidence. The roadmap's 2026-08-21
OutputControl decision supersedes its older Raw Input challenge text; a
physical Raw Input challenge is not a remaining requirement.

`check:ai4-authority-service`, `check:ai6-admin-ui`, and the frontend build
passed. The pinned MSVC 14.44.35207 isolated native build passed with zero
Rust warnings and the existing Vite chunk advisory; executable SHA-256 is
`008910b005f88ba604f29af4549c06100da0cc5f9ef28cd7de77532d66046fd2`.
The isolated native lifecycle probe passed seven checks with the normal app
identity unchanged. On 2026-10-02, the generated release test binary ran
`agent_bridge::authority::tests::consent_delegates_exact_binding_to_protocol_authority`:
one passed, zero failed or ignored. The initial Cargo filter selected zero
tests, so that initial result is not counted as test evidence.

The user subsequently authorized removing individual human approval from all
external MCP R4/R5 operations. That policy change is the next checkpoint;
the expiry-clock fix above preserves the current local prepared-consent path.

## External MCP R4/R5 without individual approval — 2026-10-02

Base `d78fedca`. The user-authorized policy is implemented and passed the
optimized native gate, including eleven real stdio MCP/native lifecycle
checks. The native probe also exposed and repaired serialization of an
existing legal orphaned-lease transfer. Exact change, compatibility boundary,
test selection, executable/harness identity and unresolved scope are recorded
once in [the checkpoint](EXTERNAL_MCP_UNATTENDED_2026-10-02.md) and its linked
machine evidence. The overall ledger remains 27 Complete, 23 Open and
8 Deferred; the goal remains active. Further work should address typed MCP
gaps and supported migration/security/observability evidence without adopting
the protected Remote PIN/DJ ingress changes listed above.

## External MCP lease authority read — 2026-10-02

Base `2d0e65e0`. The existing owner-bound lease query is now a reviewed R0 MCP
operation, with correct read classification despite its output-prefixed ID.
Native authority fences and lease observations were obtained through a separate
stdio sidecar. Twelve native checks passed; the stale registry test counts were
reconciled against the exact source inventory and the affected Rust tests passed.
Details and preserved negative/positive evidence are recorded once in
[the checkpoint](MCP_LEASE_AUTHORITY_2026-10-02.md). The ledger remains
27 Complete, 23 Open and 8 Deferred; continue executable native security checks.

## Native execution-time authority revocation — 2026-10-02

Base `730f3ebc`. The separate stdio MCP/native probe now proves that both R4
lease Acquire and R5 diagnostic export are rejected after native claim if their
principal is revoked before execution. Replay and receipt lookup are also denied;
no file or lease is created. Fourteen native checks passed. The QA-only change and
exact evidence are recorded once in
[the checkpoint](MCP_HIGH_RISK_REVOCATION_2026-10-02.md). No ledger completion is
manufactured from this slice. Next is the current normal-checkout native artifact.
