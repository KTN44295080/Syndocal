# Q2 authority and bounded native evidence reconciliation

Branch `codex/showclock-review-20260912`, base `98ac087b`. Documentation and
machine ledger only; no product/runtime behavior changes in this checkpoint.

The Q2 table and canonical Q1-Q4 mirror still required individual human approval
for external R4/R5, despite the user's explicit all-R4/R5 authorization and the
implemented ExternalMcp policy. `DEC-AI-CONSENT-001` now records that decision:
Safe Mode by default, then promotion and exact grants, with no individual approval
for implemented ExternalMcp operations. Other adapters retain their declared
consent policy. Authentication, revocation, kill switch, immutable request identity,
one-shot execution, typed payloads, generations, fences and leases remain required.

`R-AI-SAFETY-001` retains **Open**. Its mitigation now names the observed native
R5 file/R4 lease, exact-grant and post-claim revocation slices. Its outstanding
proof is actual controller-loss fail-safe, physical output/re-arm and complete
domain acceptance; it does not reintroduce an individual approval condition.
The AI Q1 proof points to the exact isolated QA artifact `73952ab1...` and its
bounded evidence rather than saying that only read-only MCP has been observed.
Full hardware/external acceptance remains unrun; full AI acceptance remains
In progress. No Flow marker, Q1 acceptance status or risk closure was promoted.

Four previously committed checkpoint artifacts are now indexed in Q4:

| Evidence | Declared slice |
| --- | --- |
| `EV-AI8-NATIVE-MCP-HIGH-RISK-2026-10-02` | 11 native checks, R5 export and R4 lease workflows, exact artifact `671fb835...` |
| `EV-AI8-NATIVE-LEASE-QUERY-2026-10-02` | 12 native checks including owner-bound lease query, exact artifact `73952ab1...` |
| `EV-AI8-NATIVE-POSTCLAIM-REVOCATION-2026-10-02` | 14 native checks including R4/R5 revocation after claim, exact artifact `73952ab1...` |
| `EV-UI-SINGLE-ROW-HEADER-2026-10-03` | Six-width browser header proof, nine-viewports/PULSE and separate passive native window/authentication proof, exact normal artifact `29d4bed5...` |

Each record links its saved evidence and nonclaims to the relevant Q1 rows.
Native artifact JSONs were inspected for the exact declared group counts and
all-pass results before indexing. The header record distinguishes browser
geometry from the passive native observation. Previously measured optimized
builds retain their one Vite advisory and zero Rust warnings; this docs-only
checkpoint has no compiler-warning measurement of its own.

The canonical master JSON and standalone ledger match. Completion remains
**27 Complete, 23 Open, 8 Deferred**; Q4 grows from 160 to 164. The next executable
safety slice is a real native controller-loss/revocation/lease-expiry drill on an
isolated profile, retaining output-denied defaults and exact ownership. Physical
and clean-install gates require their actual evidence before closure.

Validation: completion checker and Q1-Q4 checker pass; the Q1-Q4 self-test passes
46 negative cases, one positive fixture and the real-repository baseline.
`git diff --check` passes. No compiler/build/device gate was run for this
documentation-only reconciliation.
