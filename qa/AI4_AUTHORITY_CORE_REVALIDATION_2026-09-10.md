# AI4 authority-core revalidation — 2026-09-10

## Scope

This is a bounded revalidation of the existing transport- and UI-neutral
`protocol::agent_authority` core. It does not add product behavior or claim
that the AI4 service is integrated into the desktop, Agent Bridge, pairing,
or external sidecar. The real-file thumbnail missing → Retry → recovery flow
was not performed and is not claimed.

- Source HEAD: `9483ed46cb83ab5fbebd6cbf89ebd4fdae02ec26`
- Product source changes: none
- Physical output, application process, external client, and device: none
- Owner: AI control-plane software boundary

## Verification

The Windows MSVC procedure initialized `vcvars64.bat -vcvars_ver=14.44`,
pinned the Build Tools `14.44.35207` x64 linker in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and verified that exact linker
as the first `where.exe link.exe` result.

```text
cargo test -p protocol --release --locked agent_authority -- --test-threads=1
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 207 filtered out
```

The executed cases cover safe-mode permission limits and exact grants,
principal/grant deserialization validation, consent binding to principal,
owner, operation, arguments, project, and generations, single-use and expiry,
revoke and kill-switch rejection, old authorization after grant or incarnation
change, re-pair generation invalidation, kill-switch pairing closure, overflow
without partial state change, and wrong-principal/owner/operation/argument
rejection without consuming a valid consent.

## Boundary

This proves only the existing pure authority decisions. `AI4-CONSENT-001`
remains open because the Agent Bridge does not yet provide the complete
principal/pairing/grant/revocation/kill-switch/desktop human-presence service.
This run does not prove DPAPI/credential storage, nonce pairing, Raw Input
presence, desktop consent UI, audit integration, external-client acceptance,
release-bypass absence, physical output, Mac, signing, publication, or
product-wide completion.

## Current-main software revalidation — 2026-09-10

The same bounded authority-core test was rerun against current `main` source
HEAD `0589258c1ea3145fc914483d3d80a1e6ed2b1cff`. No product source or
authority policy changed; this checkpoint records only fresh evidence.

```text
cargo test -p protocol --release --locked agent_authority -- --test-threads=1
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 207 filtered out
```

The exact Build Tools MSVC `14.44.35207` x64 linker was pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` after
`vcvars64.bat -vcvars_ver=14.44`, and it was first in `where.exe link.exe`.
The AI4 service/integration and all boundaries listed above remain open.
