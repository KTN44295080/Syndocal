# F1 input generations checkpoint — 2026-09-13

## Result

`F1-INPUT-GENERATIONS-001` is Complete for the supported current-source input-runtime boundary. The implementation already present on the AI4 base was revalidated at `7decc1de`; this checkpoint adds the dedicated contract gate and synchronizes the completion ledgers.

## Verified boundary

- Project-owned and mapping-owned callbacks use independent monotonic atomic epochs.
- Project replacement advances both epochs; mapping replacement advances only the mapping epoch, preserving independent MIDI Clock transport state.
- Constructor callbacks are rejected until worker installation is complete, and callback sends compare the captured epoch immediately before dispatch.
- External admission and project-transaction fences prevent callback side effects during replacement/commit boundaries.
- Retirement joins/releases workers before publication and releases partial takes on failure; generation overflow fails closed without changing the generation.
- MIDI/OSC Learn retains its own successful mapping continuation but rejects a project or mapping authority change while awaiting the input.
- DMX liveness refreshes only for the configured universe and emits bounded SignalLost retirement after the timeout; malformed or unrelated-universe traffic cannot keep stale input alive.

## Evidence

| Gate | Result |
| --- | --- |
| `callback_epoch` | 3 passed, 0 failed |
| `installed_callback_gate` | 1 passed, 0 failed |
| `project_transaction_fence` | 1 passed, 0 failed |
| `external_admission_` | 2 passed, 0 failed |
| `project_control_retirement` | 1 passed, 0 failed |
| `project_retirement` | 6 passed, 0 failed |
| `pnpm --dir app run check:f1-input-generations:self-test` | PASS, 2 contract-presence cases |
| `pnpm --dir app run check:f1-input-generations` | PASS; focused project, MIDI, OSC, DMX, setup, and routing gates |

All Cargo runs used `vcvars64.bat -vcvars_ver=14.44`, verified the exact MSVC `14.44.35207` x64 linker first in `where.exe link.exe`, and ran the release test binary. No physical device, external client, or output fixture was required for this software marker.

## Nonclaims

This closes only the source/runtime generation and stale-callback marker. It does not close `INPUT-PHYSICAL-001`, physical MIDI/OSC/DMX/Remote clients, device reconnect or latency, venue operation, F2 ownership, ASIO/NDI physical acceptance, signing, publication, or product-wide completion.
