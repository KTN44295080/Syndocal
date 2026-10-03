# Native MCP registered-owner retirement with live loopback output

Branch `codex/showclock-review-20260912`, base `42323c35`.
This checkpoint extends the closed-gate lease proofs with an actual Windows
native Art-Net sender and an independently owned software UDP receiver.
It does not establish physical fixture, video, venue, full controller-loss,
worker/process-crash, durable mutation restart or release acceptance.

## Failure and bounded correction

The first live probe armed one isolated QA Lighting route, received the complete
expected ArtDMX image, and retired the registered native owner. The output image,
sender and runtime gates stayed unchanged, as the lease contract requires.
The subsequent old-lease Lighting master request returned
`mutation_not_confirmed / agent_output_response_invalid` instead of its domain
rejection. [Native negative evidence](artifacts/native-controller-output-before-2026-10-03.json)
and [failure log](artifacts/native-controller-output-before-2026-10-03.txt)
retain the four preceding successful groups and the failed assertion.

`OutputControlRejectionV2::validate` omitted Lighting master, Group submaster
and Video master, although their requests and successful receipts were already
valid. The correction adds those three existing canonical operation IDs to the
rejection validator. It changes no authority check, lease transition, output
execution, request schema or wire version. Unknown/retired/read operation IDs,
unsafe request IDs, unknown fields and unknown error codes remain invalid.

The focused protocol tests cover all seven domain errors for all three masters,
plus invalid wire identity. The valid rejection round trip fails on the baseline;
the invalid-input test already passes. This preserves a confirmed rejection
instead of misreporting uncertainty and does not permit an automatic mutation
retry.

## Probe contract

`check-native-lifecycle.mjs --controller-output` is mutually exclusive with the
other opt-in lanes. The native backend loads a private blank-derived project
with one embedded fixture and one Art-Net destination: `127.0.0.1` at the Node
receiver's ephemeral bound port, universe zero. There are no video layers or
outputs, audio clips, Effects or Graphs. The source fixture is the independently
frozen authored-control corpus; the probe independently sets Dimmer 65535 and
retains Pan/Tilt 32768. It does not use received output to derive expectations.

Every received packet must have the exact 530-byte ArtDMX wire header, version
14, physical byte zero, universe zero and 512 slots. All 512 payload bytes are
compared throughout each stable phase, and at least ten new frames must arrive
in each phase. A cached unchanged image is insufficient.

The expected first eight slots are `[255,0,0,0,128,0,128,0]`; all remaining slots
are zero. Explicit Lighting master 500 milliunits changes only slot one to 128:
the Engine rounds `65535 * 0.5` to 32768 and the EightBit control writes its high
byte. The probe checks:

1. Project load, Safe Mode denial, missing exact grant and Acquire produce no
   packet before explicit authenticated R4 Arm.
2. Arm without an individual approval dialog produces live received ArtDMX;
   Lighting is allowed and Video remains denied.
3. Registered native owner retirement preserves the entire live image and gate
   state while retiring lease authority.
4. Old-generation master and transfer requests reject with no state/image delta.
5. Explicit ForceTransfer increments the orphaned generation; exact terminal
   replay and same-ID shape conflict preserve physical state.
6. A separate explicit master operation by the new owner changes the live image
   to half. Exact replay and an old-owner mutation cannot change it again.
7. Relinquish changes authority only, retains live half output and rejects a
   subsequent ordinary output operation.
8. A separate native load of a private empty disabled project closes both runtime
   gates and stops UDP transmission; original project source bytes stay unchanged.

Cleanup uses the production project-replacement command with current E/R/H and
the current registered owner. The retired raw role command remains unavailable.
On failure the helper attempts the same disarm and still closes its receiver,
stdio sidecar and private files; the parent runner terminates only its exact QA
process and revokes/removes its own credentials. The normal app identity must
remain unchanged. No Computer Use or subagent is used.

## Evidence

- [Protocol negative run](artifacts/controller-output-protocol-before-2026-10-03.txt):
  one valid master rejection test fails with `UnexpectedOperationId`; the
  invalid-input test passes. Two actual tests ran, zero ignored.
- [Protocol positive run](artifacts/controller-output-protocol-2026-10-03.txt):
  all 21 selected `control_plane_command::tests` pass, zero failed or ignored.
  This includes the two new tests and the existing strict receipt/action/fence,
  runtime, lease, transfer, safety and unknown-field contracts.
- [Final native proof](artifacts/native-controller-output-2026-10-03.json) and
  [log](artifacts/native-controller-output-2026-10-03.txt): all 16 groups pass,
  comprising nine live-loopback groups and seven existing native lifecycle
  groups. Native SHA-256 is
  `f14e39191d0836247dd62154126c9d691f92381dc7ae8523c9c780846021fd81`.
  All 684 packets have a validated wire header; 614 new frames across the six
  stable phases also have all 512 slots compared against the independent oracle.
  No native worker panic was observed. Owned QA process, receiver, sidecar,
  credential and private files were cleaned up; normal app identity stayed
  unchanged during the drill.
- [Focused software checks](artifacts/controller-output-focused-checks-2026-10-03.txt):
  16 MCP integration groups, 128 hostile stdio rejections, HTTP/REST/WebSocket
  transport/security, 13 bridge groups and output-control/Standby UI contracts pass.
- [Optimized QA build](artifacts/controller-output-qa-build-2026-10-03.txt) passes
  in 5m55s using the maintained exact MSVC 14.44.35207 linker and PATH-first gate.
- [Optimized normal build](artifacts/controller-output-normal-build-2026-10-03.txt)
  passes in 3m44s. The wrapper resolved and stopped only the exact checkout
  executable's old PID 193772. A plain launch, without a debugger, produced
  PID 34572. [Normal window proof](artifacts/controller-output-normal-window-2026-10-03.json)
  verifies one visible responsive maximized `Syndocal` window and rejection of
  an unauthenticated broker read. Normal SHA-256 is
  `a285c1eb18b5942b14dfee011c319474346331a6d274ced778eaa39ab9b551be`.

| Stable phase | New frames | Dimmer slot |
| --- | ---: | ---: |
| Armed | 22 | 255 |
| Registered-owner retirement | 57 | 255 |
| Old-generation master and transfer rejection | 147 | 255 |
| ForceTransfer, exact replay and shape conflict | 110 | 255 |
| Separate new-owner master, replay and old-generation rejection | 99 | 128 |
| Relinquish and subsequent mutation rejection | 179 | 128 |

The probe's terminal rejection assertion is `ok: false`, `type: rejected`,
`error: forbidden`, matching final lease authorization. Early harness setup
assumptions about a nonexistent `video.sources` collection and retired raw role
cleanup were corrected to the actual `video.layers` schema and production
project-replacement path. An initially assumed `invalid_request` code was
corrected to the actual final authorization contract after serialization was
repaired; the check still requires the exact domain rejection and rejects any
uncertain mutation result. No product authority or safety assertion was relaxed.

Rust warning baseline/current/delta is 0/0/0 for the configurations run; the
existing Vite chunk advisory is unchanged (baseline/current 1/1, App 506.76kB).
The new Rust test block passes focused formatting. Whole-file rustfmt still
reports the same nine pre-existing unrelated formatting hunks; normalized
baseline/current diagnostics are identical. No unrelated formatting is changed.
Both native builds include the preserved unrelated frontend dirty work and are
not clean-release artifacts. No independent agent review was run because the
user prohibited subagents; the bounded production diff and evidence were
inspected directly.

The Q4 mirror and master record this bounded slice as
`EV-AI8-NATIVE-CONTROLLER-OUTPUT-2026-10-03`; evidence count advances 170 to 171.
`COV-AI-CONTROL-001` native proof and `R-AI-SAFETY-001` mitigation are updated.
Their overall acceptance/status is unchanged. Live TTL, native worker/process
loss and explicit re-Arm, durable mutation crash/restart, physical fixtures/video,
complete external/domain matrix and clean installation remain separate work.
