# Deferred marker current checkpoint — 2026-09-14

## Scope and authority

This document records the eight completion-flow markers intentionally kept
outside the current Windows-local completion target. It is an audit and resume
record, not a status promotion: the authoritative statuses remain in
`qa/SYNDOCAL_COMPLETION_LEDGER.json` and the marker comments in
`qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`.

At current source HEAD `de9d482d`, the ledger remains `27 Complete / 23 Open /
8 Deferred` across 58 markers. No Deferred marker is treated as Complete by
this checkpoint.

## WARN-MACOS-001 — Deferred

Flow authority: macOS development/release warning enforcement is explicitly
outside the Windows target; the current inventory may remain `11 enforced / 2
pending` without blocking the Windows completion target. The ledger dependency
is the Windows-only product scope and the versioned warning inventory.

Current checks:

```text
pnpm.cmd run check:warnings:self-test
warning ratchet self-tests ok

pnpm.cmd run check:warnings
exit code 1: --configuration is required

pnpm.cmd run check:warnings -- --configuration windows-native-release
exit code 1: pnpm emitted warning-shaped output
```

The first command passed. The second was an invocation rejected before a
configuration could run; the third was the corrected Windows configuration and
ran its trusted comparison/native build before failing closed on
warning-shaped build output. This does not evaluate macOS. No macOS compiler,
release build, signing, or acceptance environment is present in this
checkpoint. Resume only when the product scope is expanded to macOS and the
two pending macOS configurations, baselines, and owners are explicitly
approved; then run the macOS warning matrix without weakening the ratchet.

## PLATFORM-MAC-LINUX-001 — Deferred

Flow authority: macOS/Linux real-machine display, media, audio, project
save/reload, and package acceptance are deferred outside the Windows target.
The dependency is an explicit Windows-only product-scope decision.

The current Windows source/build checks and the three-display host inventory
do not provide a macOS or Linux machine, package, driver, or real-machine
acceptance result. No cross-platform artifact or test result was inferred.
Resume only after a scope decision names supported macOS/Linux versions,
hardware, media/audio topology, save/reload matrix, package format, and an
acceptance owner.

## DIST-PLATFORM-PACKAGE-001 — Deferred / Out of scope

Flow authority freezes the platform/package matrix outside the current
Windows-local target. The dependency is a future distribution product goal and
supported artifact matrix.

The current Windows no-bundle executable and source package checks do not
constitute a multi-platform distribution matrix. No macOS/Linux package,
artifact naming decision, or supported-platform release record was created.
Resume only when the future distribution goal specifies platforms, artifact
formats, installer/updater relationship, version policy, and acceptance owner.

## DIST-SIGNING-001 — Deferred / Out of scope

Flow authority freezes Authenticode/Apple signing outside the current target.
The dependency is a future distribution goal plus signing credentials and
policy.

No signing credential was entered or created, and no Authenticode or Apple
signing/notarization acceptance was claimed. Current native build evidence is
unsigned/local and cannot close this marker. Resume only with an approved
signing policy, credential custody, certificate identities, timestamp/
notarization service, and reproducible verification procedure.

## DIST-NOTICES-001 — Deferred / Out of scope

Flow authority freezes BOM/SBOM and notices outside the current target. The
dependency is a future distribution goal and license/SBOM disposition.

Current package-boundary checks (including the ASIO distribution gate) protect
the existing local/non-public boundary, but they are not a final distribution
BOM/SBOM or legal-notice acceptance. No legal disposition was inferred. Resume
only after the distribution artifact matrix and license/SBOM owner approve the
notice inventory, source-obligation records, and artifact-to-notice mapping.

## DIST-CLEAN-MACHINE-001 — Deferred / Out of scope

Flow authority freezes clean-machine install/upgrade/uninstall outside the
current target. The dependency is a future distribution goal and signed
installation artifacts.

The exact local release executable was started for a responsive-window check,
but that is not a clean-machine installation, upgrade, uninstall, repair, or
rollback result. No clean machine was modified and no installer acceptance was
claimed. Resume only with a signed installer and a named clean Windows image,
baseline, install/upgrade/uninstall/repair scripts, retained logs, and recovery
owner.

## DIST-UPDATER-001 — Deferred / Out of scope

Flow authority freezes the signed updater outside the current target. The
dependency is a future distribution goal, signed manifests, and an update
service.

Current updater identity/configuration and fail-closed source checks are not a
live signed update. No update endpoint was contacted, no manifest was signed or
published, and no N-to-N+1, wrong-channel, downgrade, signature mismatch,
corruption, offline, or rollback run was performed. Resume only with the
approved update service, signing/public-key policy, immutable test artifacts,
and the complete failure/recovery matrix.

## DIST-PUBLICATION-001 — Deferred / Out of scope

Flow authority freezes public tag/artifact/evidence publication outside the
current target. The dependency is a future distribution goal, immutable tag,
artifacts, and evidence manifest.

The current branch pushes source checkpoints to its working remote, but no
public release tag, distributable artifact, evidence manifest, or publication
acceptance was claimed. Resume only when the distribution goal authorizes the
immutable tag, artifact set, publication destination, checksums, notices,
release evidence manifest, and rollback/retirement owner.

## Resume order and invariant

1. Resolve the Windows-only scope decision before opening the macOS/Linux or
   distribution markers.
2. Assign owners and record the exact supported platform/artifact matrix.
3. Produce and verify the required signed artifacts and external services only
   after that scope decision.
4. Promote each marker only from its named acceptance evidence; never infer
   cross-platform, signing, clean-machine, updater, or publication acceptance
   from current-source checks, a local executable, or a source push.

The eight markers remain individually addressable and intentionally Deferred.

## Current-source release gate continuity — 2026-09-14

At current source HEAD `461b46e5`, the full Windows-local static release gate
completed with exit code `0`. It reconfirmed all 58 Flow references, Q1/Q4
mirror parity, AI0-AI7 source contracts, project/recovery/publication checks,
media authority, output safety, ASIO/live-audio contracts, Timeline contracts,
video/window contracts, and camera contracts.

The independent Windows warning-ratchet probes are retained separately:
`check:warnings:self-test` passed, while the corrected
`check:warnings -- --configuration windows-native-release` failed closed on
`pnpm emitted warning-shaped output` during the trusted native build. The
failure is not evidence for macOS and no baseline or suppression was changed.
The static gate and local release executable do not alter any Deferred status;
the eight markers remain Deferred until their explicit scope dependencies are
approved and their named external evidence exists.

## Windows warning-ratchet classifier repair — 2026-09-14

The Windows native warning gate initially failed closed because the generic
output probe treated Vite's existing chunk-size advisory as warning-shaped
output. The actual upstream line is:
`(!) Some chunks are larger than 500 kB after minification. Consider:`.
This is a build-size advisory, not a Rust/compiler diagnostic and not a new
first-party warning. The repair is recorded in implementation commit
`dad243b1`: only that exact Vite line (with LF or CRLF termination) is removed
from the generic warning-shaped probe. Unknown `(!)` lines, `warning:` lines,
and `WARN` output remain fail-closed.

Evidence after the repair:

- `pnpm.cmd run check:warnings:self-test` — PASS, including LF/CRLF Vite
  advisory cases and rejection of an incomplete/unknown `(!)` line.
- `pnpm.cmd run check:warnings -- --configuration windows-native-release` —
  PASS. Output marker coverage `2/2`; warning-shaped output `none`; baseline
  warnings `{total:0, first-party:0, third-party:0}`; current warnings
  `{total:0, first-party:0, third-party:0}`; identity removals `0`.
- `node --check scripts/warning-ratchet-lib.mjs` and
  `node --check scripts/test-warning-ratchet.mjs` — PASS from `app/`.

This is Windows-local warning-gate evidence only. It does not provide macOS
or Linux compiler evidence, signed/distributed artifact evidence, clean-machine
installation, updater, publication, hardware, physical-output, or venue
acceptance. `WARN-MACOS-001` and the other seven Deferred markers remain
Deferred; no marker status or warning baseline was promoted.
