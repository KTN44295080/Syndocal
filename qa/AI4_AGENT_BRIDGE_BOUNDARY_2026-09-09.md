# AI4 agent-bridge boundary audit — 2026-09-09

## Scope

This checkpoint audits the existing local agent bridge against the AI4
principal/grant/consent boundary. It does not add an API, grant an external
principal, enable unattended output, or claim AI4 completion.

- Source: `main` at `2f647e8`
- Product source changes: none
- Native/hardware scope: no application process, device, external client, or
  physical output was started

## Existing boundary and verification

The current bridge already has a loopback listener, per-process random token,
current-user-only state files, strict request schemas, a renderer-generation
claim barrier, bounded request/result sizes, durable mutation identity, and
replay-safe handling for pending or completed requests.

With the exact Windows procedure (`vcvars64.bat -vcvars_ver=14.44` and the
Build Tools `14.44.35207` x64 linker first in `where.exe link.exe`), this
focused command passed:

```text
cargo test -p syndocal --release --locked agent_bridge -- --nocapture --test-threads=1
```

Result: **9 passed, 0 failed, 0 ignored**. The cases cover strict token and
method/parameter rejection, exact-once claim, mutation tombstones, reload
without automatic redispatch, stale renderer completion rejection, capacity
limits, and oversized result rejection.

## AI4 gap and non-claim

The bridge does not yet provide the AI4 principal/grant service: scoped
capabilities and safe-mode promotion, explicit revoke/revoke-all, a priority
kill switch, or locally issued single-use consent bound to operation and
argument fingerprints. The current token and renderer ledger therefore must
not be described as an AI4 grant or consent implementation.

`AI4-CONSENT-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. External R4/R5 execution, unattended
high-risk automation, physical output, real external-client acceptance, and
the real-file missing → UI Retry → restore → recovery flow remain unclaimed.
