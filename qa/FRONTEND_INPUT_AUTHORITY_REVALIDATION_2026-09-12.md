# Frontend MIDI/OSC authority lifecycle revalidation — 2026-09-12

## Scope

This checkpoint covers the current-source frontend authority bridge in
`app/src/createControlInputController.ts`, `app/src/projectAuthority.ts`, and
the `App.tsx` wiring. It covers mapping Learn continuation and the
MIDI/OSC Connect/Start/Feedback flush boundary. It does not claim physical
MIDI/OSC operation, controller movement, LED or clock/MTC observation, latency,
external LAN, two-machine, venue, signing, publication, or product completion.

## Finding and bounded fix

The mapping flush bridge already returned trusted exact own acknowledgements,
but the App adapter discarded that result before it reached the MIDI/OSC
controller. A targeted Learn therefore compared its original authority token
after its own successful mapping commit had advanced the revision/hash and
discarded the learned result before reconnecting the intended worker. The same
flush boundary also did not explicitly block Connect/Start/Feedback when the
flush was untrusted.

The controller now receives a typed trusted-flush result. A pure authority gate
continues Learn only when the captured token is unchanged or the current token
is exactly one of this flow's trusted own acknowledgements. A foreign project,
an untrusted flush, or a later token remains stale and retires the continuation.
MIDI/OSC Connect/Start/Feedback routes also fail closed when mapping persistence
is not trusted. The existing mapping-runtime retirement remains owned by the
backend's acknowledged mapping commit.

## Deferred continuation regression

`app/scripts/check-project-authority.mjs` now covers:

- the flush barrier forwarding its result only after the deferred flush settles;
- Learn continuing across the exact own ACK token;
- rejection of a foreign replacement with no matching own ACK;
- rejection of an untrusted flush, including when the token itself did not
  change; and
- preservation of the existing exact-token stale Learn behavior.

The frontend routing checker also requires both targeted MIDI and OSC Learn
flows to adopt the trusted own ACK before reconnecting, and verifies that App
passes the flush result through without an adapter that drops it.

## Verification

- `pnpm.cmd --dir app exec tsc --noEmit` — passed.
- `pnpm.cmd --dir app run check:project-transaction` — passed, including the
  project-authority deterministic and deferred-ACK checks.
- `pnpm.cmd --dir app run check:frontend-command-routing` — passed: 133
  renderer mutations, 31 server-authoritative mutations, 28 raw dispatches,
  and 464 facade dispatches.
- `pnpm.cmd --dir app build` — passed; TypeScript and Vite transformed 353
  modules and produced the frontend bundle.
- Windows native gate — passed with MSVC 14.44.35207 x64 and the exact pinned
  Build Tools linker; `pnpm.cmd --dir app tauri build --no-bundle` finished with
  `target/release/syndocal.exe`.
- Native process smoke — one exact-path `syndocal.exe` process was observed
  with title `Syndocal`, a nonzero main-window handle, `Responding = True`, and
  a maximize request completed. The Computer Use native RPC was unavailable in
  this environment, so this is process/window-handle evidence rather than a
  screenshot-backed UI interaction matrix.

The physical MIDI slice, physical or external OSC clients, controller movement,
LED observation, clock/MTC, latency, and the broader ShowClock estimator,
bounded-slew, Hold/STALE, LAN, partition/rejoin, two-process, and two-machine
gates remain separate work.
