# ShowClock decision-freeze checkpoint — 2026-09-13

## Scope

This checkpoint closes `SHOWCLOCK-DECISIONS-001` for the supported
current-source software boundary. The decision log now names the remaining
ShowClock dimensions that were implicit in the runtime revalidation:
authentication/key lifecycle, strict version compatibility, and the supported
network envelope.

The decisions are additive to the previously accepted transport, fence, and
clock-master rows. They freeze policy; they do not convert loopback evidence
into external acceptance.

## Frozen decisions

| Decision | Frozen policy | Rejection boundary |
| --- | --- | --- |
| `DEC-SHOW-AUTH-001` | A paired session uses a 32-byte HMAC key, session/node identity, sequence, nonce, and clock/fence generations. Keys and session identity are process-lifetime state; restart or explicit re-pair requires a fresh session/key. There is no silent key reuse or online automatic rotation claim. | Authentication, session, generation, nonce, and sequence failures are rejected before estimator, queue, or output state mutation. |
| `DEC-SHOW-VERSION-001` | ShowClock accepts only schema/protocol version 1. Mixed-version rolling operation is not supported; a future or downgraded version requires an explicitly implemented and separately accepted decision. | Unsupported schema/protocol envelopes are rejected before payload processing. |
| `DEC-SHOW-NETWORK-001` | The supported topology is a manually paired, wired two-PC LAN with an exact peer and unicast UDP port 44,666. Discovery is only an untrusted endpoint hint. Wi-Fi, multicast, public discovery, broker availability, and public-LAN operation are outside this acceptance boundary. | Packets from a non-paired endpoint, malformed paired packets, and oversized packets fail closed without extending the receive deadline. |

These rows are intentionally separate from `DEC-SHOW-TRANSPORT-001`,
`DEC-SHOW-FENCE-001`, and `DEC-CLOCK-MASTER-001`: the earlier rows choose the
transport/fence/master policies, while these rows make the authentication
lifecycle, version refusal, and support envelope independently auditable.

## Evidence and verification

- `qa/SHOWCLOCK_RUNTIME_LAN_REVALIDATION_2026-09-12.md` records the current
  implementation values, fresh-session restart boundary, strict versioned
  envelope, exact-peer loopback tests, bounded estimator/action policy, and
  the external-boundary nonclaims.
- The current-source checks previously recorded there pass for the protocol,
  exact-peer LAN adapter, Tauri IPC, two-process loopback, frontend build, and
  Windows native process smoke. This checkpoint adds no unverified hardware or
  venue assertion.
- The Q1/Q4 validator checks that all three decision rows are referenced by
  `COV-SHOWCLOCK-001` and that both ledger copies remain structurally equal in
  their live counts.

## Boundary

This closes the supported software decision-freeze marker only. It does not
claim native UI interaction acceptance, physical MIDI/OSC/DMX/Art-Net output,
real two-machine partition/rejoin or crash/restart rehearsal, replay-state
restoration, witness/physical interlock, automatic failover, venue behavior,
signing, publication, or product completion. Those remain separate markers or
external acceptance gates.
