# Video authoring authority after same-process Open

Base: `main` at `7b32ad6827d948f2ea529a7d20742e698fca6d7f`.
Scope: the previous-project Video authoring P2 recorded in
[the runtime checkpoint](CHATGPT_RUNTIME_HANDOFF_2026-10-21.md).
This checkpoint does not extend the recorded FX/4K performance envelope or
physical-device acceptance.

## Behavior and compatibility

Previously, the frontend could reuse or Recover a Video-capable lease after
Open advanced the project epoch. Native authoring correctly rejected its old
project binding. Expiry could hide that claim from the public read-only query
while its resource index still prevented a fresh Acquire.

The new `query_video_authoring_lease_authority_v1` observes the registered exact
window/process owner, includes expired claims at their original generation, and
reports project matching plus managed ownership. It does not mutate expiry or
authority. Corrupt/overlapping indices, stale owners and project fences fail
closed.

For an unmanaged previous-project Video or Both claim, the dedicated canonical
`syndocal.output.video.authoring.lease.replace.v1` operation atomically relinquishes
the old identity and acquires a new identity with the same resources. The
registry candidate remains private until the complete two-change receipt is
durable; allocation, capacity and durable-write failures retain the old claim.
Replay retains the original request and new identity.

Native admission requires the exact current project/output fence, no pending
project replacement, an empty output-owner stop-signal set and Engine Ready
Standby with no permitted output, transition, failure or in-flight ownership.
The Engine state mutex covers the authority-only durable commit, preventing
activation from racing that check. Public output counters are projected as
Engine counters plus one; the native boundary checks subtraction before entering
that gate. The callback performs no Engine or physical I/O. Manager observation
under project locks uses try-lock admission instead of waiting behind teardown.

Current-project active reuse and orphan Recover retain their existing behavior.
Generic Recover never changes project binding. A previous managed claim or a
current-project expired claim requires explicit Standby/Relinquish through Output
Ownership, with actionable JA/EN guidance. The automatic replacement neither
arms output nor expands a Video lease to Both. Frontend preparation and the
subsequent edit stay bound to the same expected project.

Output command schema advances from 13 to 14 for appended discriminant 32.
Existing action discriminants and Acquire/Recover/Relinquish wire identities
remain unchanged. Only the new operation accepts the exact retire-plus-acquire
two-change receipt; other operations retain the one-change contract. Product
version remains `1.2.0-alpha.82`; no distributed artifact is produced.

The two new native commands are admitted locally as ReadOnly and RuntimeMutation,
respectively. They remain R5/Unavailable in external discovery and have no
reviewed external canonical adapter/query schema. This deliberately avoids
expanding external access as part of the local authoring fix. The exact native
inventory is 593 commands; the binary-search admission lists remain sorted.

## Evidence

Campaign: `target/qa/lease-project-2026-10-21/` (ignored local evidence).
Failed attempts remain preserved alongside corrected runs.

- Registry: 13 focused tests, including Video/Both, active/orphan/expired,
  owner/process/generation rejection, read purity/index corruption, disjoint
  Lighting, capacity/id exhaustion, durable rollback and immutable replay.
- Native integration: 2 focused tests; actual Engine authority/fence projection,
  Standby preservation, old-project exchange and active/stale/pending/failed SDK
  refusal. Together with registry tests: **15 passed**, zero failed/ignored in
  `native-tests-03.log`. The earlier positive failure exposed the public-counter
  projection error; rejection fixtures were corrected to use valid public fences.
- Engine: **2 passed** (`engine-tests.log`), exact quiescent Standby and
  serialization against activation.
- Protocol: **2 passed** (`protocol-tests.log`), strict distinct action identity
  and corrupt/two-change receipt rejection for other operations.
- Broader affected regressions (`regressions.log`): native control-plane **52**,
  output-lease **149**, native authoring **2**, protocol **301** unit tests plus
  **8** integration and **4** doc tests passed. Zero failed/ignored. The count
  assertions changed by exactly two native entries and two frontend aliases;
  external unclassified sources increased by two, with no new direct operation.
- TypeScript `tsc --noEmit` passed. Authoring UI contract, output-control runtime
  contract, exact frontend/native invoke inventory (**513 commands**) and Tauri
  wrapper contract (**249 assertions, 27 hostile fixtures**) passed. These are
  software contracts, not rendered or physical output evidence.
- Web ChatGPT via Chat On Steroids implemented the exclusively assigned registry
  files, then independently reviewed the supervisor's integration. It identified
  the public/Engine counter conversion and invalid negative fixtures; both were
  corrected and its bounded re-review confirmed resolution. No other concrete
  P1/P2 was reported in the assigned integration. The supervisor inspected its
  registry diff and ran the checks above. Review evidence:
  `web-review-final.txt` / `web-review-final.png`; conversation
  <https://chatgpt.com/c/6ad82e9c-72dc-83ec-b74f-bb44ce538bde>.
  The bounded metadata follow-up (`web-review-inventory.txt`) found no blocker
  with the local-only classification/count boundary; the supervisor's unchanged
  admission checker recomputed the exact hash and verified sorted lists.

### Windows native same-process Open

The source-stable QA build passed `pnpm --dir app tauri build --no-bundle`
with its isolated identifier configuration (`qa-build-03.log` / result and
before/after manifests). MSVC 14.44.35207 was pinned and first on PATH. The copied
QA executable SHA256 was
`eeb22cbb9d4fe12704fd2b8dd14dfecc8b518ab9771edbd66509de5872053d0f`.

Actual maximized, responsive `Syndocal` main, PID **46804**, exact copied path and
CDP process ancestry were verified. Using the real canonical Open and production
frontend controller against that same native process:

1. Open unchanged Japanese-named fixture A (epoch 1), acquire Video without Arm.
2. Open fixture B (epoch 2). Open correctly orphans the previous claim and
   advances its generation from 1 to 2; the project-qualified read reports the
   same old ID and `projectMatches: false`.
3. Configure B's existing output. One replacement creates
   `lease-0000000000000002`, generation 1, with exactly Video scope, then applies
   the intended label. The old identity is relinquished.
4. Replay that identical native replacement after the edit: structurally identical
   receipt, no extra identity. A direct native edit using the old ID returns
   `invalid_request`; B's intended label remains. Both fixture files retain
   their original SHA256.
5. Final ownership is **Ready / Standby**, both physical permissions false,
   no external active routes and no live output windows. `native-project-b.png`
   shows the updated label in the actual Video setup screen. This is local
   native authoring/controller evidence, not a physical output test.

Evidence is chained explicitly: `native-open-03-result.json` records both Opens;
its original assertion incorrectly expected an unchanged generation. No edit or
replacement had been sent. `native-open-04-result.json` hashes that prior record,
checks the same PID/binary/frontend hashes and exact B authority, then resumes
only the remaining authoring work and passes. It does not replay either Open or
restart between Open and edit. Earlier pre-mutation failures (CDP port 9273 and a
fresh-default authored DMX expectation) remain preserved. Port 9272 worked after
restart. Before Open, default authored DMX configuration may be enabled while
startup physically denies all output; after fixture Open, every authored DMX
route is additionally verified disabled. The assertion change distinguishes
authored defaults from physical permission; it does not permit a physical route.

The QA process was closed through its protected discard dialog after preserving
the evidence.

The final standard `pnpm --dir app tauri build --no-bundle` passed
(`standard-build-01.log` / result / before-after manifests), source-stable with
the same exact MSVC procedure. The original checkout executable at
`target/release/syndocal.exe` had SHA256
`d493076c1fe137e523543cafaf3a7742d5ce52cf518aca0b264231c553adc371`.
PID **47120** exposed exactly one responsive `Syndocal` main. Its actual rendered
screen and native system menu confirmed maximization (`standard-startup.json`,
`standard-startup.png`, `standard-maximized.png`). It was closed gracefully after
the startup gate; both owned QA/standard PIDs were absent afterward
(`native-final-gate.json`). No device/output action was performed in the standard
profile. This completes the scoped Windows native gate and the unmanaged
previous-project authoring P2; it does not establish full release acceptance.

Rust warnings measured in the prior standard build, current QA/standard builds and affected
regressions: baseline 0, current 0, delta 0. The existing Vite >500 kB chunk notice
is baseline 1 / current 1 / delta 0; no warning threshold or assertion was weakened.
Raw counts and original logs remain in `warning-counts.json` and the referenced
build logs. Documentation checks alone do not measure compiler warnings.

## Boundaries

Unknown DMX/audio devices, physical scanout, macOS/Syphon and NDI runtime/network
output were not opened or exercised. Existing Stop cancellation/performance
limits remain as recorded in the runtime checkpoint. Managed-lease turnover and
current-project expiry deliberately retain their explicit lifecycle boundary.
