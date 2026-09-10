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

## Current-main follow-up — `f45dfe5418dde792b4b2b3f4fe162297aedefbd5`

The Phase 0 software checks were rerun against the current `main` after the
media-operation lifecycle responsibility extraction. The extraction does not
change ledger status, release policy, product version, native admission, or
the fixed three-workspace boundary.

| Command | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:completion-ledger` | PASS — 50 Open + 8 Deferred authority rows; 58 total |
| `pnpm.cmd --dir app run check:q1-q4-ledger` | PASS — 32 Q1 rows, 29/29 Q0 domains, 10/10 source contracts, 58/58 flow markers, mirror parity |
| `pnpm.cmd --dir app run check:release:self-test` | PASS — release metadata 129 groups, ASIO packaging 169, video routing, candidate extractor 43, verified materialization 4, Windows release artifact 144, strict JSON 130 |
| `pnpm.cmd --dir app run check:warnings:self-test` | PASS — warning-ratchet negative and positive fixtures |
| `git diff --check` | PASS |

`FFMPEG_DIR` was not set. The ASIO check remained the SDK-independent
self-test and did not create or treat an ASIO artifact as distributable. This
revalidation is software evidence only; it does not close the real-file
thumbnail recovery boundary, GUI/native IPC acceptance, external clients,
physical output, Mac/Linux real-device acceptance, signing, publication, or
venue acceptance.

## Release static gate coverage checkpoint — based on `d18c2bb17992ba6d3a26f72904dd9c7a60eb7d7e`

The existing completion-ledger and Q1–Q4 validators are now invoked by
`check:release:static`, before native admission and product contract checks.
This is validation wiring only: it does not change ledger statuses, add
product behavior, create release artifacts, sign or publish anything, or
reintroduce a non-library workspace.

The metadata self-test also asserts that both validator commands remain in the
static gate. The normal and candidate release modes continue to use the same
static gate, while RC evidence remains explicit and absent in this checkout.

## Frontend source-inventory gate coverage — based on `0e6e726b756315d6c0dd064085948cc1b582efd5`

The existing frontend routing and Tauri invoke inventory checks are now also
invoked by `check:release:static`, alongside the completion-ledger and Q1–Q4
validators. The release metadata self-test asserts this routing so the gate
cannot silently lose the source-inventory checks. This remains validation
wiring only; `AI0-COVERAGE-001` stays Open for the broader non-frontend
mutation-source inventory and fail-closed classification.

## Backend authority checker gate coverage — based on `a7c63a2770ea84dfd807e94e9df3cc9e9fe471a5`

The existing backend operator and project transaction/authority checkers are now
also invoked by `check:release:static`. Their existing rejection coverage for
ownership, approval, stale authority, terminal state, permission, and recovery
remains authoritative; this change only prevents the checks from being omitted
from the normal static release gate. No product runtime code or acceptance
assertion was changed.
