# Local priority Blackout under native broker pressure and revocation

Branch `codex/showclock-review-20260912`, base `b0ad4a0c`.

## Actual test boundary

This tests-only change adds the opt-in `--controller-safety-pressure` native lane.
It uses the existing native renderer-registration boundary to retire automatic
bridge consumption while retaining real authenticated, immutable requests.
It does not patch the DOM, invoke function, admission, engine or output worker.
The native local `safety_blackout_engage_v1` command remains independent of the
external broker ledger. No product/Rust/frontend/IPC/schema/approval policy is
changed; promoted exact-grant ExternalMcp R4/R5 needs no individual approval.

Each of two cases starts from a fresh isolated QA process and the existing
independently authored software-loopback Lighting project. Acquisition alone
emits no packet; separate explicit external Arm starts the expected full image.
Only an owned ephemeral `127.0.0.1` Art-Net sender/receiver is used. There is no
physical fixture, serial port, Video/audio source or running Timeline.

The test submits and claims a real R4 half-master intent and a real R5 diagnostic
export. It then admits 62 runtime reads, filling all 64 native detail slots with
pending work. The 65th read is rejected with `inflight_capacity`; its UUID is
absent (`unknown`). Authored master/fixtures/routes and live output are unchanged.

While 10,000 additional R4 intents are outstanding, the test calls the real
local native S0 operation. In the second case it first activates the actual
native Kill Switch, which revokes all external principals and removes their
credentials/grants. S0 succeeds from the previously live full image and new
received packets contain zero in every one of 512 channels. The operation is
not reconstructed from the UI preview or a simulated completion.

## Results

[The native run](artifacts/native-controller-safety-pressure-2026-10-03.json)
and [raw log](artifacts/native-controller-safety-pressure-2026-10-03.txt) pass
all 25 selected groups against optimized QA executable SHA-256
`760ba5017a450773972d433869e23a5d8dcb203d7ca9c62be8f1fa3b059274b3`.

| Case | Pending native slots | Additional intents | Outstanding at S0 | Sidecar unsent | Native rejection | Local S0 receipt | First new zero packet |
| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| Saturation | 64 | 10,000 | 10,000 | 9,999 | 1 `inflight_capacity` | 19.99ms | 14.56ms |
| Kill Switch active | 64 | 10,000 | 9,842 | 9,999 | 1 `agent_kill_switch_active` | 5.56ms | 3.80ms |

Each local-action sequence begins with 10,000 outstanding client requests. The
test separately measures and requires outstanding work immediately before S0,
including after awaiting Kill Switch. These timings measure backend invocation/
loopback packet receipt on this host. S0 already owns
a two-second engine publication expiry and three-second acknowledgment bound;
the test requires its successful production receipt and uses the existing
five-second image-reception deadline. No new realtime/venue threshold is invented.
The first zero packet can arrive before the native acknowledgment reaches Node.

In the first case, all 64 accepted UUIDs remain readable as `pending` through
S0. They are not silently dropped, automatically replayed or falsely completed
to free capacity. The principal is then explicitly revoked. For both claimed
R4/R5 requests, actual native execution returns the current revocation/Kill Switch
error; the second execution attempt is `request_not_executable`. Receipt lookup
also rejects the invalid principal without disclosing a result. No diagnostic
ZIP is created, and a further R4 master intent is refused.

Authored master, ordinary project Blackout, fixture attributes, routes and output
ownership remain byte-equivalent to their pre-S0 checkpoint observations.
The runtime snapshot reports engaged safety Blackout while these sampled authored
fields remain unchanged; this is not a whole Undo/history comparison. Exact local
S0 replay returns an identical terminal receipt/audit sequence; a new local S0
request returns `no_op`. Adding `enabled: false` is rejected at strict native
deserialization and cannot turn S0 into release/toggle.

The receiver validates 2,970 actual packet headers. Ten stable phases compare
all 512 channels on 1,932 packets, including full output during pending-capacity
fill and zero output through invalid external execution and local replay/no-op
checks. Separate native project replacement explicitly disarms and stops each
owned sender; source project bytes remain unchanged. Each case resets the runtime
latch by stopping
and restarting its exact isolated QA executable, with fresh authentication.

The runner verifies an exact responsive maximized QA window on every launch,
fresh process/nonce authentication and its seven existing lifecycle groups,
empty fresh QA startup, normal-profile descriptor preservation, no native panic,
credential revocation/removal, and process/stdio/socket/private-file cleanup.
The ordinary executable remains SHA-256
`b6e39dc927aa7a5184bdad9f77c14486d20a7a059089f5f74a9165e5f3b8e7a3`,
PID 199664, and is not restarted or debugged by this checkpoint.

[The existing burst regression](artifacts/controller-safety-pressure-burst-regression-2026-10-03.json)
passes all 16 groups with the updated shared test forwarding/runner. Its distinct
native discovery and R4 burst coverage is not added to the 25-group count.
[Twelve CLI exclusions](artifacts/controller-safety-pressure-cli-2026-10-03.txt)
reject mixed live lanes before executable selection/launch. The existing
[safety frontend contract](artifacts/controller-safety-pressure-frontend-2026-10-03.txt),
Node syntax, stable owned diff and protected fingerprints pass.

## Checkpoint and remaining scope

The QA artifact retains the preceding exact pinned MSVC 14.44.35207 build
provenance in [the prior build](artifacts/request-capacity-qa-build-2026-10-03.txt).
No compiler runs in this tests/docs checkpoint; compiler-warning baseline/current
or delta is not newly measured. Positive native probe logs contain zero runtime
warnings and no panic locations. Product source and ordinary binary are unchanged.

Q4 `EV-AI8-NATIVE-SAFETY-PRESSURE-2026-10-03` advances the evidence index from
175 to 176. Both maintained ledger validators preserve 27 Complete, 23 Open and
8 Deferred markers, all earlier Q4 records and Q1/Q2/Q3 statuses. No acceptance
marker is promoted by this bounded proof. Twenty-five denotes selected native
check groups, not individual requests, packets, Rust tests or duplicate regression
groups.

This current-source proof is for native external broker detail-slot saturation
and local backend S0, individual revocation after engage, and fresh engage with
Kill Switch active. It does not saturate the Engine command queue or establish
every adapter/operation, physical UI input, complete tick/audio/video/render/UI
budgets, publisher gap/resnapshot during saturation, cross-process aggregate
limits, full authored/recording/crash durability, clean installation, physical
fixtures/devices, venue, independent security review or release acceptance.
No Computer Use or subagents are used. Protected other-owner pairing PIN,
localization, DJ runtime and AI3 work remains outside this checkpoint.
