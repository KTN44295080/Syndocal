# Release-candidate gate routing — 2026-09-08

## Scope

This checkpoint repairs the release-script path so the explicit
`check:release:candidate` gate can reach its RC-only metadata and artifact
checks. It does not create a release candidate, sign an artifact, publish a
manifest, or change product runtime code.

The previous candidate command invoked the normal `check:release` gate first.
That gate intentionally validates development metadata without candidate
evidence, so an RC product version was rejected before the candidate command
could inspect its explicit manifest. The static source/admission checks are
now shared by both modes; normal metadata validation remains in
`check:release`, while candidate metadata validation is invoked only with
`--release-candidate --manifest`.

## Changes reviewed

- `app/package.json`
  - added `check:release:static` for the shared static/admission checks;
  - kept normal `check:release` as static checks followed by development
    metadata validation;
  - routed `check:release:candidate` directly to explicit RC metadata and
    Windows candidate-artifact extraction.
- `app/scripts/test-check-release-metadata.mjs`
  - added exact package-script routing assertions;
  - retained the existing strict RC evidence and invalid-mode assertions.
- `app/scripts/fixtures/release/tauri-signer-public.key` and
  `Syndocal_1.2.0-rc.1_tauri-signer-fixture.bin.sig`
  - replaced only the checked-in test fixture key/signature pair because the
    prior pair did not verify against its unchanged fixture payload;
  - no product key, release artifact, crypto assertion, or verification rule
    was weakened.

## Evidence

Repository base before this checkpoint: `0496f421a7937854faeea3bd89a4bf66d28a52cd`.

| Check | Result |
| --- | --- |
| `node --check app/scripts/test-check-release-metadata.mjs` | PASS |
| `node --check app/scripts/check-release-metadata.mjs` | PASS |
| `app/package.json` JSON parse | PASS |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — 128 release-metadata assertion groups; ASIO 169; extractor 43; materialization 4; Windows artifacts 140; strict JSON 130 |
| `pnpm.cmd --dir app run check:release` | PASS — native admission 516 commands / 18 rejected fixtures; thumbnail, IPC, snapshot, output, ASIO, timeline, video, camera, and normal alpha metadata checks passed |
| `pnpm.cmd --dir app run check:release:candidate` | EXPECTED STOP — all static checks passed, then the explicit RC metadata step rejected the absent `qa/release/release-evidence.json` before artifact extraction |
| `git diff --check` | PASS |

The current package version is `1.2.0-alpha.69`. No RC tag, signed NSIS/MSI
bundle, updater manifest, or external publication was created in this
checkpoint. Therefore the real RC evidence gate remains open and
`RELEASE-METADATA-GATE-001` is not closed by this change.

No native rebuild, native-window launch, physical output, signing, or release
publication was required for this script/fixture-only checkpoint.
