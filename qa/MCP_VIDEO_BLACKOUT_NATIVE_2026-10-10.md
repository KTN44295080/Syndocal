# Native MCP Video BO checkpoint — 2026-10-10

Branch: `codex/native-output-guards-20261010`.
Base: `58463a8dffdfb17351c4ce1e51bc9ec813619fcd`.
Product metadata remains `1.2.0-alpha.71`; this is an internal checkpoint.

## Result and compatibility

The public `syndocal_set_video_blackout` arguments remain the UUID, boolean,
and exact project E/R/H. Its immutable broker request now executes the canonical
native Video `SetBlackout` command and confirms the persisted Video BO bit against
the receipt's exact successor. The retired renderer domain adapter, refresh-hook
interface and all dependency plumbing have been removed. Pairing, exact grants,
revocation, project/output/S0 fences and the active exact-Both lease remain required.
There is no implicit acquisition, renewal, Arm or per-operation confirmation.

Native execution is single-use. The broker UUID owns durable external replay;
the internal numeric receipt ID is allocated monotonically in the native range,
with explicit exhaustion rejection. Its bounded request scope and private lease
receipt domains are distinct from the GUI/canonical caller counters, including
the managed exact-Both projection. A native BO command cannot advance those GUI
watermarks. There are two fixed native receipt domains, not one domain per UUID.

The existing durable receipt structure is unchanged. Its validator recognizes
the native domain only for the reviewed blackout operation, requires exactly
one matching private receipt, and rejects unsupported or ambiguous records.
Older binaries do not accept the new native-domain public terminals; downgrade
compatibility for such receipt journals is not claimed. Existing GUI-domain
records remain valid. No project-file or USB-DMX storage format changed.

External output admission rechecks the original principal after admission and
coordinator waits; lease lifecycle also rechecks after its lifecycle wait.
Video BO rechecks at the actual lifecycle/project/engine-writer/lease commit
boundary. Reset rechecks after lifecycle/admission/owner/coordinator acquisition,
before physical retirement. This does not establish cancellation atomicity
inside every other already-dispatched output worker.

## Evidence

- [Rust checks](artifacts/native-output-guards-20261010/rust-tests.txt):
  22 bridge/guard, 5 target-blackout, 5 reset tests passed.
- [Interleaving and durable-record check](artifacts/native-output-guards-20261010/rust-interleave-test.txt):
  1 passed, including the next lower GUI identity, managed projection, durable
  validation and unsupported-domain rejection.
- [Existing regressions](artifacts/native-output-guards-20261010/rust-regressions.txt):
  2 common-identity and 13 managed exact-Both tests passed. Total: 48 distinct tests.
- [Actual native MCP acceptance](artifacts/native-output-guards-20261010/native-video-blackout-07.json):
  26 checks passed using the private QA profile and exact isolated executable.
  Includes missing/stale authority rejection, explicit fixture-owned lease
  renewal, BO true/applied, true/no-op and false/applied, UUID replay, subsequent
  low canonical IDs, original-principal revocation after claim, native restart,
  graceful close and no restored grant. Normal app descriptor unchanged;
  temporary credentials revoked/removed; no native panic.
- The same native run received ArtDMX at its owned ephemeral `127.0.0.1` UDP
  receiver through the production sender. It proves loopback behavior only.
  During BO operations fixtures/video outputs are empty and all authored DMX
  routes are disabled. Cleanup New restores authored default Art-Net routes,
  but both actual output gates must be closed and the empty Timeline stopped.
- [Sidecar integration](artifacts/native-output-guards-20261010/mcp-integration.txt):
  16 groups and 128 hostile frames passed against a fake broker.
- TypeScript, bridge 15 groups, deferred lifecycle 4 groups, output/standby lease,
  S0, authority service, source coverage, command routing and invoke inventories
  passed. Inventory remains 561 Tauri / 481 frontend commands; 18 negative
  admission fixtures passed. These checks do not imply native or hardware output.
- [QA native build](artifacts/native-output-guards-20261010/native-qa-build.txt)
  passed with exact MSVC 14.44.35207 pin and PATH-first verification.
- [Normal no-bundle build](artifacts/native-output-guards-20261010/normal-native-build.txt)
  and [native window](artifacts/native-output-guards-20261010/normal-native-window.json)
  passed: exactly one visible, responsive, maximized `Syndocal` window; an
  unauthenticated broker read was rejected. The owned candidate closed normally.

Compiler warning baseline/current/delta: Windows first-party Rust 0/0/0.
Vite large-chunk warning 1/1/0. No macOS compiler-warning measurement for this
internal tranche.

## Corrected findings and preserved boundaries

Failed native attempts 01–04 are retained beside the final evidence. They exposed
the missing native execution allowlist entry, random-ID/high-water conflicts,
and missing durable native-domain validation. Attempt 02 also exposed cleanup
masking its original error; cleanup now always attempts disarm and session close.
Attempts 05–06 had passing BO/interleaving behavior but incorrect cleanup test
assumptions: New's authored route defaults are enabled, and the playing bit is
`snapshot.timeline.playing`. Acceptance now checks the observed canonical
cleanup contract; BO-phase disabled-route assertions remain enforced.

The primary checkout's resumed tracked patch SHA-256 remains
`4edd7d33097499e516e0379b355e1583bc769fad7b96ec8fe38e40cef94d5103`.
Its unrelated App/localization/DJ Link/remote PIN/QA changes and caches were not
edited, staged or committed. All work is owned in the isolated branch.

[Delivered alpha.71 ZIP/DMG hashes](artifacts/native-output-guards-20261010/delivered-alpha71-preserved.json)
are unchanged. These internal changes are not in those already-delivered files.
Physical USB-DMX, real macOS UI/output, clean-machine install, signing/notarization
and full product acceptance remain separate. Await the friend's Apple Silicon
device result; capture the detected device/status or diagnosis ZIP on failure.
