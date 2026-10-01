# Native high-risk execution-time revocation proof

Branch `codex/showclock-review-20260912`, base `730f3ebc`.
This checkpoint changes QA tooling and evidence only. Product code, wire formats,
grants and consent policy are unchanged.

## Verified boundary

The optional `--external-revocation` native lifecycle probe uses two temporary
promoted principals with exact File/Output grants. A separate real stdio MCP
sidecar submits each R5 diagnostic export and R4 Lighting lease Acquire. The
native broker admits the request as pending. The trusted-window backend claims
it and confirms the saved method, arguments, principal and incarnation.

The probe then revokes that principal and calls the immutable native executor
with only renderer generation and broker request UUID. Execution returns exactly
`agent_principal_revoked`. A repeated call returns `request_not_executable`,
and the revoked MCP client cannot read its receipt. The diagnostic destination
remains absent, the lease query remains unavailable, and both physical output
domains remain denied. This proves native authorization is rechecked after claim;
admission does not freeze permission for a later execution.

The helper registers a new QA renderer generation at the final backend-only
boundary before graceful close. That retires the automatic frontend dispatcher
and permits deterministic claim/revoke/execute ordering through existing native
commands. It does not patch the renderer, add a product test bypass, navigate the
UI, impersonate a native result, or change the domain executor. The request is a
real authenticated broker request; execution and rejection are real native calls.
Temporary credentials are stored in the runner's existing private ACL directory,
revoked and removed. No subagents or Computer Use are used.

## Evidence

Command: `check-native-lifecycle.mjs --external-high-risk --external-revocation`
against the exact isolated QA executable from the preceding checkpoint.

- [Machine evidence](artifacts/native-mcp-high-risk-revocation-2026-10-02.json):
  14 checks passed, including the two execution-time revocation cases, previous
  positive unattended R4/R5 and lease-query tests, and native lifecycle tests.
- Executable SHA-256:
  `73952ab1c734bc8a24597fc5613910e2f6b1790142592ee1a723a3a3d93ef94b`.
  No native rebuild was needed for these QA-only changes.
- Exact executable/process ownership, one responsive maximized QA window on each
  launch, normal descriptor identity preservation, and zero native panic locations
  were checked. The owned sidecars, principals, credentials and QA process were
  cleaned up. Existing native claim and single-use guards were retained.
- `node --check` passed for the new helper and updated lifecycle runner.
- Ledger validators and `git diff --check` passed. No compiler was run solely for
  this QA checkpoint, so no new compiler-warning measurement is claimed.

This is a bounded native security slice, not independent security review or whole
AI8 acceptance. Physical output, venue, accessibility, migration and broader support
gates remain separate. The ledger remains 27 Complete, 23 Open, 8 Deferred.
Continue by validating the normal checkout executable built from the current tree,
then the remaining executable domain gaps; preserve unrelated dirty work.
