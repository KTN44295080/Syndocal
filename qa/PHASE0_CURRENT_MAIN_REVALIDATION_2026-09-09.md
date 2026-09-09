# Phase 0 current-main revalidation — 2026-09-09

## Scope

This checkpoint revalidates the current `main` release/static contracts and
completion ledgers after the F1/F2/video/Timeline QA records were pushed. It
records evidence only; no product source, ledger status, acceptance assertion,
or release policy was changed.

- Source under test: `43c4326851143a6a4842aeabfa9dd17446bd06c0`
- Worktree: `main`, clean before the QA record was added
- Native/hardware scope: none; no physical output, device, or external client
  was started
- Windows native linker procedure: not applicable to these Node-only checks

## Verification

| Command | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — 50 Open + 8 Deferred authority rows; 58 total |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — 32 Q1 rows, 29/29 Q0 domains, 10/10 source contracts, 58/58 flow markers, mirror parity |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — release metadata 128 groups, ASIO packaging 169, video routing, candidate extractor 43, verified materialization 4, Windows release artifact 144, strict JSON 130 |
| `pnpm.cmd --dir app run check:release` | PASS — native admission 516 commands / 18 negative fixtures; media thumbnail, native thumbnail request, snapshot, agent bridge, output/safety, ASIO, timeline/audio, project bootstrap, video, camera, and normal alpha metadata gates |
| `git diff --check` | PASS |

`FFMPEG_DIR` was not set. The ASIO packaging gate therefore remained the
SDK-independent boundary check and did not create or treat an ASIO artifact as
distributable.

## Current-main follow-up — `8a9185eae41933c1c71edfcd9688627fcaf83cde`

The Phase 0 software checks were rerun after the durable-recovery QA-only
checkpoint. Product source, ledger status, acceptance assertions, and release
policy were unchanged.

| Command | Result |
| --- | --- |
| `node app/scripts/check-completion-ledger.mjs` | PASS — 50 Open + 8 Deferred authority rows; 58 total |
| `node app/scripts/check-q1-q4-ledger.mjs` | PASS — 32 Q1 rows, 29/29 Q0 domains, 10/10 source contracts, 58/58 flow markers, mirror parity |
| `pnpm.cmd --dir app run check:release` | PASS — all static admission, thumbnail, snapshot, bridge, output/safety, ASIO boundary, timeline/audio, project, video/camera, and alpha metadata gates |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — release metadata 128, ASIO packaging 169, video routing, candidate extractor 43, verified materialization 4, Windows release artifact 144, strict JSON 130 |
| `git diff --check` | PASS for this follow-up |

`FFMPEG_DIR` remained unset. No distributable ASIO runtime was created or
treated as available.

## Boundary

This closes only the current-main Phase 0 software revalidation checkpoint. It
does not close the completion ledger, the real-file-missing → UI Retry →
restore → recovery acceptance, browser/native-window evidence, physical
ASIO/NDI/DMX/video/audio acceptance, Mac/Linux real-device acceptance,
signing, publication, or venue acceptance.
