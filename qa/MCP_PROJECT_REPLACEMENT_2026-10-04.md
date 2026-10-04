# Typed external MCP project New/Open

Branch `codex/showclock-review-20260912`, base `fb5a9e3c`. This implements a
bounded external R5 New/Open vertical after the existing replacement preflight
and generation checkpoints. The broader environment-executable goal remains
active; no completion marker is promoted.

Implementation/build evidence began October 4; final regression and recording
completed October 5. The dated filenames retain the checkpoint's start date;
raw timestamps identify each observation. Final source and protected hashes
were revalidated after the overnight continuation.

Authenticated MCP clients can now execute `syndocal.project.new.v1` and
`syndocal.project.open.v1` with an exact File grant and no per-action human
confirmation. `syndocal.query.project.replacement.authority.v1` issues the
required process/session/E/R/H/publication fence under its own Read grant.
The renderer forwards only its generation and broker UUID to native execution;
the native dispatcher reads the immutable authenticated request. Principal
identity is never registered as the main project-transaction owner.

## Change and compatibility

`control_plane_project.rs` owns schema 1 DTOs, strict unknown-field rejection,
operation/action identity, safe integers, exact successors and bounded Open
path/SHA-256. Open hashes and parses one bounded byte read, then calls the
existing preparation and replacement primitives. It never writes the source.
The private native controller owns bounded single-flight terminal receipts,
changed-shape rejection, rate admission and process-memory admitted/terminal
audit records. Audit is not durable or exposed as a new export/query endpoint.

Each replacement preflight, including final publication validation, now
rechecks external principal incarnation and grant against the immutable native
dispatch. Local typed Tauri adapters retain a parented danger confirmation;
external dispatch supplies no renderer-selectable consent bypass. Existing raw
local New/Open signatures and their preparation are unchanged.

The [native negative](artifacts/typed-replacement-new-state-negative-2026-10-04.json)
exposed a real New default-image mismatch: the receipt captured epoch 2,
revision 0 and publication 3, but the next read saw revision 1/publication 4.
The engine had filled the empty Timeline bank and Main video composition after
the prepared hash was captured. A [later native negative](artifacts/typed-replacement-preview-negative-2026-10-04.json)
also identified runtime DMX preview defaults in the prepared hash. Typed New
now prepares known current defaults and applies the existing canonical save
projection before hashing. The [ACK regression](artifacts/typed-replacement-new-tests-2026-10-04.txt)
compares the complete prepared persistence image with the acknowledged engine
image. Its retained [first failure](artifacts/typed-replacement-new-tests-negative-2026-10-04.txt)
identified the remaining Main composition after Timeline normalization; the
equality remains exact. Its expected side now reads the actual prepared input
directly instead of sanitizing that input and hiding runtime preview fields.

Registry wire 4 adds a separate `LocalWindowProjectReplacement` R5 policy;
older consumers rejecting an unknown policy fail closed. Existing R4 policy
shape is unchanged. Exact inventory is 545 Tauri handlers, 56 reviewed canonical
operations, 1,629 legacy rows and 1,662 canonical source rows. The 481 frontend
invoke names and 1,120 unclassified sources remain unchanged. Product version
remains `1.2.0-alpha.69`.

One [harness negative](artifacts/typed-replacement-invalid-args-negative-2026-10-04.json)
expected `request_rejected` for malformed arguments after entering native
execution. The established renderer envelope is `mutation_not_confirmed` at
that boundary. The harness now asserts that exact envelope, the native parse
diagnostic and complete unchanged state; no product rejection was weakened.
Two initial registry-test expectation omissions are retained in
[the negative log](artifacts/typed-replacement-registry-negative-2026-10-04.txt).
The current allowlist explicitly includes the additional R0 read and tests
New/Open as R5 with their exact policy.

## Evidence

- [Actual stdio MCP/native probe](artifacts/typed-replacement-native-2026-10-04.json):
  27 groups pass. Safe Mode/missing/exact grants, forged caller
  and consent fields, six unissued fence components, changed file hash,
  relative path, invalid JSON, valid Japanese current-schema file, exact broker
  and domain replay, changed-shape rejection and default New are exercised.
  Open sets BPM 93, New restores 120; both advance epoch/publication once,
  reset revision, preserve private source bytes and leave output gates closed.
  Separate post-claim revocation-before-NativeStart covers New/Open plus
  existing diagnostics and output. No individual confirmation dialog appears.
- [Replacement preflight regression](artifacts/typed-replacement-preflight-2026-10-04.json):
  22 groups pass. [Output regression](artifacts/typed-replacement-output-2026-10-04.json):
  16 groups pass. These cover the existing software-loopback managed-retirement
  path, not physical fixtures or typed replacement with active managed output.
- [Tap regression](artifacts/typed-replacement-tap-2026-10-04.json): eight groups
  pass. Five 750ms taps change engine BPM 120 to 79.40381; header reads
  79 and footer reports the applied BPM. Header remains one row at
  42px. The actual Tap target is visible, enabled and unobstructed at its
  center. The opt-in QA receiver invokes the registered App callback; no DOM or
  physical click is performed. First Tap/long-pause reset retains existing BPM.
- Rust: five protocol DTO tests, 14 registry tests, 31 native registry/authored
  tests and 22 complete replacement tests pass: 72 unique tests. The earlier
  eight added controller cases overlap the replacement filter. The first
  Cargo batch completed DTO/controller filters, then its queued second compile
  was explicitly retired after remaining filters passed using that exact
  freshly compiled test artifact. Final 31 and 22 use the corrected artifact.
  Test-only CLI package optimization overrides are recorded and verified;
  they do not replace optimized application builds.
- Agent bridge 15 groups, fake-loopback sidecar 16 groups plus 128 hostile
  stdio inputs, Tap async nine cases, exact Tauri inventory plus 18 hostile
  cases, frontend invokes, harness syntax and owned diff checks pass.
- [Normal optimized no-bundle build](artifacts/typed-replacement-normal-build-2026-10-04.txt)
  and [sequential isolated QA build](artifacts/typed-replacement-qa-build-2026-10-04.txt)
  use the maintained pinned PATH-first MSVC 14.44.35207 wrapper. Normal SHA-256
  `9c93c494b85d6e8ded953ff12ab955b39e32876381ca9d157d3c8a2debb2c5d2`; QA SHA-256 `39306a984b9b3909926a9105abad1d73f3dba0a848f2cbffb3b5c0ef6c1b72db`. All four current native probes use
  the final QA hash, report zero native panics and clean up owned processes and
  credentials while preserving the ordinary instance.
- [Ordinary window](artifacts/typed-replacement-normal-window-2026-10-04.json):
  PID 168020, exactly one visible responsive maximized `Syndocal`
  window; unauthenticated read rejects. No authenticated primary-profile
  mutation or primary debugger was used.
- [Source freeze](artifacts/typed-replacement-source-freeze-2026-10-04.json)
  records the final owned source and harness adjustment. Five protected
  other-owner fingerprints remain unchanged. Both ledgers validate at 184 Q4
  records and 27 Complete / 23 Open / eight Deferred markers.

Rust/TypeScript warning/diagnostic baseline/current/delta is 0/0/0; existing
Vite chunk advisory is 1/1/0. Node-only checks have no compiler-warning
measurement. No warning allowance, size threshold or product version changed.
No Computer Use or subagent was used. Stable self-review is not independent
review. Protected dirty frontend work is included in these builds; they are
not clean frozen release candidates.

This does not establish complete current-source AI3/AI8 acceptance, durable
audit/export, legacy Open migration, all file operations, simultaneous
authority-change output atomicity, revocation during parse/final-publication
races, crash durability, physical/serial/video/venue or release acceptance.
Current-schema private Open and canonical default New are the measured slice.
Next close remaining backend file-operation/compatibility gaps and applicable
native evidence without promoting unobserved acceptance gates.
