# Tap BPM publication and Control readout

Branch `codex/showclock-review-20260912`, base `b9d0766f`.

The user reported that Tap did not change BPM. Calling the real App Tap callback
in the isolated Windows QA build reproduced the bug after the frontend authority
refresh was repaired: Tap 2 left the engine at `80.227936` BPM, the native header
at `120`, and the footer at `Tapped BPM 120.0`. The interval calculation worked;
the publication and renderer read boundaries were incomplete. The first failed
run's artifact identity is
preserved in [the negative evidence](artifacts/tap-native-before-barrier-2026-10-03.json);
that file records failure and artifact identity, not the assertion payload above.
Adding the engine publication acknowledgment alone still reproduced the issue.
[The diagnostic negative run](artifacts/tap-native-scope-mismatch-2026-10-03.json)
captures engine `80.14449`, header `120`, adopted project revision `2`, and the
renderer watermark still bound to revision `1` at read generation `0`.

`tap_bpm` now submits exactly one Tap and waits for an internal snapshot
publication barrier. Its acknowledgment succeeds only after the existing engine
publication path makes the clock visible. Expiry or publication failure reports
an error without replaying the mutation. The read barrier does not mutate
persistence or video configuration, clone a persistence image, or add polling.
Later normal commands cannot pass its command-drain barrier.

Mapping-only authority reads can adopt a changed E/R/H token before its canonical
snapshot. The authority poll now detects that the snapshot scope is behind: it
omits one optional cursor to request a complete bundle, and applies its validated
canonical image even when the token was already adopted. This keeps local mapping
arrays and editor drafts, and preserves rejection of old project/runtime images.
Ordinary snapshot reads cannot reset the scope, and malformed canonical data
cannot reset it either. Convergence within the same project lifecycle also
retains the previous runtime high-water mark, so even a canonical but late
bundle cannot rewind transport, Loop or Follow. Rejection leaves the previous
scope untouched. No synthetic epoch or revision is sent.

The frontend also settles a pre-tap authority poll, refreshes the canonical project
identity, then reads the snapshot. It reports the returned clock rather than
the old UI cache. A null read or replaced project cannot report the old tempo.
Control's one-row 42px header and existing control sizes are unchanged.

## Validation

- `check:tap-tempo`: nine async cases covering ordering, stale reads, project
  replacement and failures without mutation retries.
- Focused engine release tests: four selected, four passed, none ignored. Three
  new publication cases prove the shared clock changes before acknowledgment,
  expiry is nonmutating, and publication failure cannot acknowledge or replay Tap.
  Test-only engine optimization overrides were used; this is unit evidence.
  [Raw result](artifacts/tap-engine-tests-2026-10-03.txt).
- Focused native control-plane release tests: 31 selected, 31 passed, none
  ignored. The first run caught the stale unclassified-source count; the exact
  inventory was corrected. Internal engine sources are now 281, while canonical
  external operations remain 53, native routes 542 and frontend aliases 481.
  The new barrier remains unavailable as a canonical external operation.
  [Raw result](artifacts/tap-native-contract-tests-2026-10-03.txt).
- The snapshot runtime watermark regression reproduces token-before-image
  adoption at unchanged read generation, rejects ordinary/malformed B and delayed
  A (including an old runtime labeled with B's guard), and accepts 80 BPM only
  after canonical B starts the correct scope. A stale canonical same-project
  runtime cannot adopt the new scope.
- Project transaction, snapshot runtime watermark, frontend invoke, Tauri
  admission and AI0 exact-source coverage gates pass, including the coverage
  parser self-test. TypeScript and Node syntax checks pass.
- `check:workspace-header`: six widths pass. A passive tempo change updates the
  displayed value without entering BPM edit mode; manual editing still works.
  [Browser evidence](artifacts/tap-header-2026-10-03.json). The Browser plugin is
  unavailable; the existing Playwright runtime was used.

## Real Windows Tap acceptance

The pinned MSVC 14.44.35207 optimized isolated QA build and the optional
`check-native-lifecycle.mjs --tap-bpm` gate pass. [Native evidence](artifacts/tap-native-final-2026-10-03.json)
records eight passing check groups and executable SHA-256
`67438962da781e67185a2cfac3ead8e5a02b351958cf139ad579d36c3111ec7f`.
The QA-only opt-in receiver invokes the real App callback through backend
diagnostics, without a DOM/native click or a second tempo implementation.

Five taps scheduled 750ms apart produce native engine BPM `120`, `81.47317`,
`78.91437`, `79.91345`, `79.174965` (see exact machine values in the linked
evidence). Every callback finishes with the native header's rounded BPM and
footer reflecting its clock; the header remains 42px. A 2200ms pause then one
Tap resets history to one sample and retains `79.174965` BPM. The test also
records the current renderer E/R/H scope and runtime watermark. Neither
matching project tokens nor ordinary reads bypass the canonical scope fence.

The runner verifies one responsive maximized QA window on each launch, empty
project/no playback, authenticated stdio read and restart/replay/revocation
boundaries, zero native panic locations, exact executable ownership, preserved
normal-profile identity, and cleanup of the QA process and temporary credentials.
This is the real production callback in an isolated QA artifact, not a physical
user click or device-output acceptance.

## Ordinary checkout build and launch

`pnpm --dir app tauri build --no-bundle` passes with the exact linker pin and
PATH-first preflight. The intended checkout executable was refreshed at
`2026-10-02T21:00:04.6161809Z` (October 3 JST), after this build started.
SHA-256: `08bd0aa60175e9ad438639bd0065fad680d876ab390f930ddf94b4c55bda151f`.
[The passive native proof](artifacts/tap-normal-native-2026-10-03.json) pins the
exact executable and PID `198712`, verifies one visible responsive maximized
`Syndocal` window, and proves unauthenticated broker reads remain rejected.
The normal app remains running. This primary-profile observation has no debugger
and does not claim an authenticated primary-profile Tap or physical click test;
the real App callback was verified in the isolated native QA artifact above.

Ordinary generated assets contain neither `__syndocalQaTapTempo` nor its state
receiver. Both optimized builds report zero Rust warnings and zero TypeScript
diagnostics. Vite retains one existing chunk advisory: baseline 1, current 1,
delta 0. No warning limit was changed. The builds include protected pre-existing
frontend work and are not clean frozen release candidates.

Completion and Q1-Q4 ledger validators pass with the existing 27 Complete,
23 Open, 8 Deferred markers and 164 indexed evidence records unchanged.

No Computer Use or subagents are used. The protected pairing PIN, localization,
DJ runtime and AI3 work remains outside this checkpoint. Self-review is not
claimed as independent review. Physical output and complete ledger acceptance
remain separate; no completion marker is promoted by this local repair.
