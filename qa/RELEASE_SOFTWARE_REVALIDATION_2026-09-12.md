# Release software revalidation — 2026-09-12

This checkpoint records current-main release metadata, artifact, wrapper, and
warning-ratchet software evidence. It does not create a release candidate or
claim signing, publication, or physical acceptance.

## Source and artifact identity

- Source base: `074ec974abbc7f08192d10b3e1269d58454d011d` (`main`)
- `origin/main` matched before the checkpoint; no product source was changed.
- Product version: `1.2.0-alpha.69`
- Latest unbundled development EXE: `target/release/syndocal.exe`
- Size: 64,569,856 bytes
- SHA-256: `B7D5702AA8D4FE2F3F6701BB479130464AD46B1CD9528B591413B838DAEB95BC`

## Checks

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:release` | PASS — completion/Q1 ledgers, release-static gates, and release metadata |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — metadata 137 groups, ASIO packaging 169, Windows artifact 144, strict JSON 130 |
| `pnpm.cmd --dir app run check:warnings -- --configuration windows-native-release` | PASS — baseline/current total `0/0`, first-party `0/0`, third-party `0/0`, identity removals `0` |
| `pnpm.cmd --dir app run check:warnings:self-test` | PASS |
| `node app/scripts/check-windows-release-artifacts.mjs --self-test` | PASS — extractor 43 and materialization 4; Windows artifact self-test 144 |
| `pnpm.cmd --dir app run check:tauri-build-wrapper` | PASS — 243 assertions, 27 hostile mutation fixtures |

The release metadata gate confirmed the synchronized `1.2.0-alpha.69`, `.sdc`,
updater overlay, `Seraf()`, and `KTN` surfaces. The warning gate used the exact
MSVC 14.44.35207 x64 environment and completed a native no-bundle build.

## Remaining release boundary

`COV-RELEASE-001` remains `In progress`. No signed release candidate,
Authenticode/Developer ID/notarized artifact, ASIO license publication,
clean-machine install/upgrade/uninstall, live updater, physical acceptance,
public tag, or external publication was performed. The latest EXE hash above
identifies an unbundled local development artifact only.

## Current-main software revalidation — 2026-09-12

The release software gates were re-run after the prior record. The executable
was built from `8812a7ff0e1d2e679bb66f4eb61de6302919498b`; the current `main`
recording point is `5cbd6bc1a53662511d4bc35139a89e6aeac01861`, whose intervening
changes are QA documents and ledger mirrors only.

- Current release metadata check: **PASS** — `pnpm.cmd --dir app run check:release`
- Current release self-tests: **PASS** — metadata 137 assertion groups, ASIO packaging 169 assertions, Windows artifact 144 assertions, strict JSON 130 assertions
- Current warning gate: **PASS** — Windows native no-bundle build; baseline/current total `0/0`, first-party `0/0`, third-party `0/0`, identity removals `0`
- Current warning self-test: **PASS**
- Current Tauri build wrapper: **PASS** — 243 assertions, 27 hostile mutation fixtures
- Current unbundled EXE: `target/release/syndocal.exe`, `64,569,856` bytes, SHA-256 `144CD12F9B247575DE4E40FC966BEA703D74206B159C736AFA5A6FE66957C9E3`

The current EXE hash is also bound to the native thumbnail cancellation probe
in `qa/THUMBNAIL_NATIVE_CANCELLATION_CURRENT_MAIN_2026-09-12.md`. It remains a
local unbundled development artifact; no signed release candidate, publication,
or clean-machine/updater acceptance is claimed.
