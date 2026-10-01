# External MCP R4/R5 without individual approval

Branch `codex/showclock-review-20260912`, base `d78fedca`.
The user explicitly authorized all external MCP R4/R5 operations without
individual human approval. That decision supersedes the previous requirement
for this adapter in the AI roadmap and completion flow.

## Behavior and compatibility

Promoted external MCP principals with an exact operation grant now pass R4/R5
authority admission directly. Pairing, request proof, Safe Mode, exact adapter
and capability grants, principal incarnation, revocation and kill switch still
reject invalid authority. Other adapters retain their local consent policy.
Existing request shapes and broker protocol version are unchanged; the intentional
behavior change is that a valid external high-risk grant no longer returns
`agent_consent_required`.

All 25 currently implemented canonical R4 output operations are in the 52-operation
MCP projection, including Lighting master, group submaster, Video master, Take
and Clip Launch. Canonical external output execution uses a new trusted-window internal
command that accepts only renderer generation and broker request identity.
The backend retrieves the immutable authenticated socket payload, consumes the
claimed request once, and rechecks its grant before entering the shared output
controller without a confirmation dialog. Exact typed operation/action identity,
domain fences, output leases, rate limits, audits and terminal receipts remain
in that controller. Renderer-supplied replacement arguments cannot select work.
The internal command is classified in the exact 542-route Tauri inventory;
the frontend invoke inventory is 481. Retired and unimplemented operations
remain unavailable rather than being sent through a generic command passthrough.

The native probe exposed a pre-existing receipt mismatch: the lease core
permits transfer from HeldOrphaned to HeldActive, but the protocol validator
accepted only HeldActive as the source. The validator now accepts both legal
transfer sources while rejecting Unclaimed/missing phases, an orphaned Renew,
and an active Recover. The receipt has the same wire version and fields.
This accepts the core's existing committed transition; it does not relax lease
ownership or generation checks. Native response encoding also uses fallible
serialization so an invalid domain response cannot panic through `json!`.

The added `syndocal_export_diagnostics` tool requires the exact File grant for
`syndocal.diagnostics.export.v1`. It captures the existing sanitized package and
publishes a new ZIP atomically to an absolute destination, without a preview
or approval step. Existing destinations are rejected, including filesystem
races. The broker retains mutation identity across restart; reply loss does
not cause automatic resubmission. High-risk authority admission is recorded
in the bounded existing audit viewer; output domain audit is unchanged.

## Evidence

- Protocol authority tests: 10 passed, zero failed or ignored.
- Protocol command tests: 19 passed, zero failed or ignored, including transfer
  serialization round trips and invalid-source rejection.
- Native `agent_bridge` tests: 22 passed, zero failed or ignored. This test-only
  run used release dependencies with `profile.release.package.syndocal.opt-level=0`
  and `codegen-units=256`; it is not the release application build.
- Production bridge processor/runtime checks: 12 groups passed.
- MCP adapter integration: 16 groups and 128 hostile stdio rejections passed.
- HTTP/REST/WebSocket transport and session/origin/host/security checks passed.
- Exact frontend invokes, native inventory, backend operator and output runtime
  contracts passed. AI0 source coverage passed with Tauri 542, Frontend 481,
  Engine 280, Remote 116, MIDI/OSC/DMX 206 and Keyboard 33.
- Frontend build passed with the existing Vite chunk advisory.
- The complete `check:release:static` software gate passed on this working tree.
- Final optimized isolated Windows native build passed. SHA-256:
  `671fb83527bc8cdc961414b71fc830b5e293e0e54afada5212d0ebb8474359d4`.
- [Real native evidence](artifacts/native-external-mcp-high-risk-2026-10-02.json):
  eleven checks passed through a separate stdio MCP sidecar and real native
  broker. The probe verified Safe Mode and missing File/Output grants; new ZIP
  digest/length, UUID replay/conflict, existing-file rejection; lease Acquire,
  Renew, same-owner transfer rejection, owner retirement, stale-generation
  rejection, successful ForceTransfer and terminal replay, Relinquish, and
  stale renderer rejection. The existing seven native crash/restart/credential
  lifecycle checks also passed. No native worker panic was observed.
  Each launch had one responsive maximized exact-path QA window. Lighting and
  Video remained denied; the normal app identity was unchanged. The owned QA
  process, stdio sidecar, principal and temporary credentials were cleaned up.

All Windows Cargo runs initialize MSVC 14.44.35207, pin its exact Build Tools
linker, print the pin and verify it is first in `where.exe link.exe`. The
environment's vcvars startup printed a missing `vswhere.exe` diagnostic; it
still initialized x64 with the verified pinned linker. No Rust compiler warnings
were emitted by the tested configurations.

The first probes rejected a same-owner transfer correctly. Probes v5-v7 then
exposed the orphaned-transfer receipt error; the preserved temporary negative
evidence ends at `%TEMP%/syndocal-external-high-risk-2026-10-02-v7.json`.
An intermediate build also rejected a call to a private protocol validator;
that attempted call was removed before the final successful build. It is not
counted as passed evidence. The final probe is v8 as linked above.

This checkpoint does not establish physical output,
clean installation, distribution, venue acceptance or complete AI8 acceptance.
The normal app and other-owner dirty changes are preserved. No subagents or
Computer Use are used. Diff review was performed by the implementing agent,
not an independent reviewer. The completion ledger remains 27 Complete,
23 Open and 8 Deferred. Next executable work is the remaining typed MCP
operation gaps and the supported migration/security/observability acceptance
items; hardware and release gates retain their separate evidence boundaries.
