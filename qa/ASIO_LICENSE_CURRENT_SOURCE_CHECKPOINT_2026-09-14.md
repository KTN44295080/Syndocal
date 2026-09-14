# ASIO License Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-LICENSE-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `5f1646ceaf3b935e51148d1b5fb6d0cdfabdc5a6`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md` and `qa/ASIO_SDK_PIN.json`

This checkpoint revalidates the current-source distribution boundary. It does
not select a GPLv3-separated artifact, create a Steinberg agreement, or approve
public ASIO distribution.

## Current boundary

`qa/ASIO_SDK_PIN.json` remains pinned to Steinberg ASIO SDK 2.3.4 with
`distribution_approved: false`. The normal MIT/WASAPI package continues to
reject the canonical ASIO bridge, the retired bridge name, arbitrary ASIO DLL
names, and DLL globs. The SDK-derived bridge remains a separately built,
same-host QA/local-only payload. The distribution decision is still the
explicit choice between a separately reviewed GPLv3 route and a signed
Steinberg proprietary SDK route, with notices/source obligations and installer
and updater separation recorded before publication.

## Verification

The current source checks were run from the repository root:

```text
pnpm.cmd --dir app run check:asio-packaging
pnpm.cmd --dir app run check:asio-v3-contract
```

Result: exit code 0.

- ASIO packaging boundary self-test: `169 assertions`, PASS.
- ASIO v3 contract test: `22 assertions`, PASS.
- The packaging proof rejects ASIO runtime injection, DLL globs, resource
  remapping, missing/extra approved runtime DLLs, SDK pin drift, future pin
  fields, and distribution approval drift.
- The contract proof covers the current ABI-v3 source contract; it is not a
  license or legal review.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun — 2026-09-14

The two license/package-boundary checks were rerun after takeover and passed:
ASIO packaging reported 169 assertions and the ABI-v3 contract reported 22
assertions. The run again kept `distribution_approved: false`, used no public
ASIO artifact, and made no licensing or legal decision.

## Unresolved acceptance

`ASIO-LICENSE-001` stays Open because no licensing owner has recorded either
the GPLv3-separated artifact path or a signed Steinberg agreement. The remaining
ASIO format, fault, soak, latency, persistence, and final-package markers also
remain Open; this source checkpoint cannot replace real-driver, native, or
external legal evidence.

Next action is an explicit product/licensing decision with the selected notices,
source-obligation, artifact, installer, updater, and approval records. Until
then the current fail-closed non-distribution boundary is the accepted state.

## Takeover continuation — current HEAD licensing boundary recheck — 2026-09-14

At HEAD `e5dcd3ea`, `check:asio-packaging` passed `169` assertions and
`check:asio-v3-contract` passed `22` assertions. `ASIO_SDK_PIN.json` remains
on Steinberg ASIO SDK `2.3.4` with `distribution_approved: false`. No public
ASIO artifact was produced and no licensing or legal decision was inferred.
`ASIO-LICENSE-001` remains `Open` pending the explicitly recorded GPLv3 or
Steinberg agreement route and its notice/source/package approvals.
