# DVC Random Chaser Evidence

Updated: 2026-08-09
Scope: Daslight 5 `CHASER FX / Chaser random` (`RACK TYPE=3`, `EFFECT TYPE=6`, `ID=325`).

## Sources and audit identity

- Installed Daslight: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
  - Product version `5.0.6.2`, file version `25.0905.165.111`
  - SHA-256 `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Bundled Qt runtime: `C:\Daslight 5\common\Resourcesx64\Qt5Core.dll`
  - Version `5.15.2.0`
  - SHA-256 `8D2FF4CE9096DDCCC4F4CD62C2E41FC854CFD1B0D6E8D296645A7F5FD4AE565A`
- Real read-only DVC: `C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc`
- Synthetic boundary specimen: `synthetic_fx_dvc()` in `app/src-tauri/src/dvc_import.rs`.

The public Daslight manual describes the Chaser family and generator workflow but does not
define the serialized `Random sequence` or `Nb cycles` fields. The meanings below therefore
come from the saved DVC plus static inspection of the installed executable; no UI state was
changed during this audit.

## Confirmed Daslight representation

The Random Chaser constructor at image address `0x140374c90` creates these parameters:

| DVC ID | Daslight label | Type and range |
|---|---|---|
| 11 | Fading | boolean |
| 12 | Nb pixels on | integer `1..1000` |
| 13 | Flash | float |
| 14 | Random sequence | integer `0..255` |
| 15 | Nb cycles | integer `1..255` |

This disproves the former importer interpretation of parameter 14 as a boolean.

At `0x1403764d0`, `0x140379235..0x140379249`, and
`0x140379e27..0x140379e35`, Daslight multiplies the selected step/beam count by
`Nb cycles`. That product is the complete Random sequence length. Playback loops after this
whole sequence; `Nb cycles` is not a stop-after-N counter.

At `0x140379f0c..0x140379f60`, Daslight seeds a Qt `QRandomGenerator` with the integer
`Random sequence` and fills a sequence-length array. Target allocation at
`0x140375ca0..0x1403762f3` also uses Qt's global random generator while building the active
target sets. Consequently, a `.dvc` contains the selector and cycle count but not the exact
ephemeral target order generated for one particular Daslight process load. That one-load
random order cannot be reconstructed from the project file alone.

## Syndocal contract

Syndocal now preserves both values exactly:

- `Random sequence` maps to `ChaserEffectRequest.random_seed`, including valid value `0`.
- `Nb cycles` maps to additive `random_cycle_count`; missing legacy data defaults to `1`.
- Step duration divides the DVC effect duration by `step_count × random_cycle_count`.
- Every cycle is a complete deterministic permutation of the selected steps. The saved seed
  generates consecutive permutations, and the entire sequence repeats after all cycles.
- `Pixels on`, `Flash`, and `Fading` continue to drive active width, duty gate, and overlap.
- Unlike Daslight's process-global allocation, Syndocal's result is stable after save/reload.
  This is deliberately stronger for show reproducibility while preserving the observable DVC
  timing, bounds, distribution, and repeat-length semantics.

## Old vs new vs reason

| | Old | New | Reason |
|---|---|---|---|
| Parameter 14 | boolean `0/1` | integer `0..255` | Daslight constructor range and Qt seed path |
| Parameter 15 | reported as unreproduced | persisted `1..255`; full sequence length | installed binary multiplies target count by cycles |
| Random period | `duration / steps` | `duration / (steps × cycles)` | the Daslight free-run path includes all cycles |
| Reload | one fixed permutation | saved multi-cycle deterministic sequence | exact ephemeral Daslight load order is absent from DVC; deterministic output is safer |
| UI | one generic seed field | seed plus Random cycles, only in Random mode | complete authoring and edit parity |

## Automated evidence

- Protocol JSON and preset roundtrip, including legacy default `random_cycle_count=1`.
- Importer synthetic `Random sequence=1`, `Nb cycles=2`: six-slot traversal and `200 ms`
  per step from `DURATION=1200`.
- Importer rejects sequence `256` and cycles `256` without guessing.
- Real Shinkan golden preserves all Random Chasers as sequence `0`, cycles `1`, with no
  obsolete `RandomSeq` / `NbCycles` approximation report.
- Engine verifies three fair four-step permutations, a 12-slot DMX-value time series, and exact
  repeat at `1200 ms`.
- Scene Settings headless contract verifies seed `0`, cycles `3`, nine preview slots, two active
  slots, zero horizontal overflow, and editor containment at `1920×1080`, native-like
  `1920×1032`, `2048×1152`, `1366×768`, and `1280×720`.

Verified gates on 2026-08-09:

- `cargo test --workspace --all-targets` — zero failures; Syndocal `371` passed / `9` ignored,
  Engine `448` passed / `1` ignored.
- `check:chaser-draft`, `check:cue-effect-recall`, `check:project-storage`, `check:effect-draft`,
  `check:timeline-automation`, `check:timeline-viewport`, `check:localization`, and
  `check:release` — all passed.
- `pnpm --dir app tauri build --no-bundle` — production frontend and native release build passed.
