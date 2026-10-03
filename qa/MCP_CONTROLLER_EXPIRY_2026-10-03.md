# Live native lease TTL expiry and explicit recovery

Branch `codex/showclock-review-20260912`, base `f3600246`.
This checkpoint fills the live-TTL software gap left by the closed-gate
[lease expiry proof](MCP_LEASE_EXPIRY_2026-10-03.md),
[registered-owner retirement proof](MCP_CONTROLLER_OUTPUT_2026-10-03.md) and
[native process restart proof](MCP_CONTROLLER_RESTART_2026-10-03.md).

## Scope and behavior

`check-native-lifecycle.mjs --controller-expiry` owns a separate opt-in lane.
The common output helper owns the private project, independent fixture oracle,
ephemeral `127.0.0.1` UDP receiver, actual native sender, authenticated stdio MCP
and final explicit project-replacement disarm. `native-controller-expiry.mjs`
owns the TTL/recovery assertions. Nine conflicting opt-in flags reject before
executable selection or process launch. No product runtime, IPC, schema,
authority policy or user interface changes are made.

The backend's production-selected TTL remains 60 seconds. The test records
monotonic elapsed time from immediately before acquisition, sends no renewal
while waiting, and cannot substitute a clock. Every received ArtDMX packet must
have the complete native wire header and independently expected 512-channel
image. Each interval between lease observations (a five-second wait plus native
read) must contain at least ten new frames. A cached final image cannot pass;
this does not establish packet-loss, jitter or real-time latency bounds.

The expected full image is `[255,0,0,0,128,0,128,0]` followed by zero channels.
The test verifies:

1. R0 lease reads hide expired authority without changing the output image,
   runtime ownership gates, route, master, Blackout or fixture attributes.
2. An explicit expired renewal rejects `forbidden` and observes one transition
   to orphan generation 2. Exact terminal replay does not execute it again.
3. Fresh stale-generation Renew, Arm and Lighting master operations reject
   `forbidden` and preserve the full image and orphan generation.
4. Explicit recovery advances the same owner's authority to generation 3.
   Exact replay is identical; changing its shape under the same request UUID
   conflicts. Recovery itself leaves the full image and native gates unchanged.
5. A separate explicit recovered-owner master operation changes Dimmer to 128
   (500 milliunits). Exact replay and an expired-owner operation leave that
   complete half image unchanged.
6. Relinquishment preserves the live half image, hides authority and blocks
   subsequent output operations. Separate native project replacement closes
   both runtime gates and stops the owned sender.

The preserved-output contract comes from the AI3 output-lease acceptance
section of `SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`: authority-only transitions
must not send DMX, change Blackout/role, Arm, tear down or stop a worker. No
automatic physical shutdown or recovery was introduced.

## Validation and boundary

The raw native, affected regression and CLI results accompany this checkpoint
under `qa/artifacts`. They pin the actual executable and maintained harness
hashes, selected check groups, observed frame counts, elapsed TTL, unchanged
normal-app identity, credential revocation/removal and absence of native panics.

[The final native result](artifacts/native-controller-expiry-2026-10-03.json)
passes 17 actual selected check groups (six expiry/recovery groups, four common
output/setup/cleanup groups and seven lifecycle groups). The expired-authority
read completes at `62102.4299` ms after acquisition begins; this is an observation
time, not a changed TTL. Ten observation intervals contain at least 250 new
frames each. The receiver validates 3552 complete wire packets; 3450 new frames
occur inside stable-image phases:

| Phase | New verified full-image frames |
| --- | ---: |
| Initial explicit Arm | 21 |
| Real TTL wait and expiry | 2571 |
| Expired renewal and terminal replay | 162 |
| Stale-generation Renew/Arm/master rejection | 246 |
| Explicit recovery, replay and conflict | 174 |
| Explicit recovered-owner half master and replay | 99 |
| Recovered-lease relinquishment | 177 |

No retryable read overload occurs. Exact terminal replay is deliberate
idempotence verification, not resubmission of an uncertain mutation. The runner
reports zero native panic locations, preserved normal identity and cleanup of
the owned QA process, stdio child, receiver, project and credentials.

The cached optimized isolated Windows QA executable is unchanged from the
preceding pinned MSVC 14.44.35207 build. No compiler ran in this tests/docs-only
checkpoint; probe-log warnings are distinct from compiler measurements. Normal
`target/release/syndocal.exe` remains running with its preceding artifact identity.
Protected pairing PIN, localization, DJ runtime and AI3 files remain unchanged.

QA executable SHA-256:
`f14e39191d0836247dd62154126c9d691f92381dc7ae8523c9c780846021fd81`.
Normal executable SHA-256:
`a285c1eb18b5942b14dfee011c319474346331a6d274ced778eaa39ab9b551be`;
PID 34572 remains responsive. These are the preceding checkpoint's builds, not
new clean-install or release artifacts.

[The affected output regression](artifacts/controller-expiry-output-regression-2026-10-03.json)
passes all 16 selected groups after the shared forwarding change.
[Nine CLI exclusion cases](artifacts/controller-expiry-cli-2026-10-03.txt)
reject before any executable selection or launch. Node syntax checks and
source/evidence hash checks pass. Both native logs contain zero probe warnings
and no native panics. This adds no compiler-warning measurement. The existing
nine async Tap regression cases also pass; this checkpoint changes no Tap code.

Q4 evidence `EV-AI8-NATIVE-CONTROLLER-EXPIRY-2026-10-03` records this bounded
slice and advances the indexed evidence count from 172 to 173. Product/domain
completion statuses remain unchanged. The 16-group regression is not added to
distinct expiry coverage. Final maintained ledger checks and owned diff review
pass: 27 Complete, 23 Open and 8 Deferred markers, 32 Q1 rows, 29 domains,
10 source contracts, 15 decisions, 14 risks and 173 evidence records. The master
and mirror match, all earlier Q4 records and Q1/Q2/Q3 statuses are unchanged,
and all five protected file fingerprints remain identical.

This is real native software-loopback TTL/recovery evidence. It does not
establish physical fixtures, serial DMX, video, worker-specific failure,
in-flight mutation crash durability, complete controller-loss/re-arm, venue,
clean installation or release acceptance. No Computer Use or subagents are
used. No product/domain completion status is promoted by this bounded proof.
