# Media Asset T1 A8 native evidence — 2026-08-13

Status: IN PROGRESS — 12 of 15 native workflow rows executed successfully at this checkpoint. The remaining three rows are deliberately not accepted yet.

## Release identity and native gate

- Checkout: `C:\Users\kouty\Documents\KDMX`
- Executable: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Executable SHA-256: `41FE507275C21916F36894A4D4E5DF4D9CEF537E5223A778302B83CCE93FAE07`
- Build gate: `pnpm --dir app tauri build --no-bundle` succeeded before this run.
- Runtime gate: exactly one responsive `Syndocal` window from the exact executable path was selected and maximized before every UI action.
- Fixture root: `C:\TEMP\Syndocal-A8-2026-08-13-01`
- Source manifest: `C:\TEMP\Syndocal-A8-2026-08-13-01\manifest-before.json`
- Machine-local availability was not treated as persisted project state.

## Accepted native rows at this checkpoint

### 1. Empty first-run picker cancellation — PASS

- Baseline: `projects\empty-a.sdc`, 0 assets / 0 layers / 0 outputs.
- Action: open the first-run picker and press Escape.
- Native message: `VJセットアップをキャンセルしました。プロジェクトは変更されていません。`
- Result: 0 assets / 0 layers / 0 outputs; first-run CTA remained available.
- No save was performed after cancellation; the baseline file remained the source of truth.

### 2. First-run Bootstrap success — PASS

- Input: `media\mismatch-valid.mp4`, 76,750 bytes, SHA-256 `3107CB1EAD787328ADF4737136E3DE1ABA2FA194BADCBA38EC20C4BF280A6C0F`.
- Artifact: `projects\bootstrap-success.sdc`, 10,119 bytes, SHA-256 `5863BE40C4A6396ACE026B7AEAC507D1B91ECBC765FAC9696121CD63EA0FD8E8`.
- Saved state: 1 asset / 1 layer / 1 composition / 1 output.
- Asset 1 persisted the exact input hash and byte size; layer 1 referenced asset 1; Main referenced layer 1 and output 1.
- Output 1 was `enabled=false`, `blackout=true`.

### 3. One-file catalog-only import — PASS

- Baseline: `projects\bootstrap-success.sdc`.
- Input: `media\v01.mp4`, 6,217 bytes, SHA-256 `36791A9A700A831023047700C49E2D4576188D54FB7E02D6AFB91F58C11B36EE`.
- Artifact: `projects\catalog-one.sdc`, 10,765 bytes, SHA-256 `B8464015039973511051992886857E2B64123F243CEE158CDF461DF42FEB9934`.
- Result: assets 1 -> 2; layers remained 1; compositions remained 1; outputs remained 1.
- The new catalog asset had zero layer references and persisted the exact source hash/byte size.

### 4. Twelve-file catalog-only import — PASS

- Baseline: `projects\bootstrap-success.sdc`.
- Inputs: `v01.mp4` through `v12.mp4`, selected as one native picker batch.
- Native report: `メディアライブラリ: 12件 読み込み。`
- Artifact: `projects\catalog-12.sdc`, 17,887 bytes, SHA-256 `8610274F7F383E1DABA9923B6BED597068D1733010D4401DC3852B8C510A71E2`.
- Result: assets 1 -> 13; layers remained 1; compositions remained 1; outputs remained 1.
- All twelve saved asset hashes and byte sizes matched the files in `manifest-before.json` exactly, including `v12.mp4` at 20,606,544 bytes.
- The earlier long-path picker attempt imported only five files and is preserved separately as `projects\batch-long-path-partial.sdc`; it is not counted as acceptance evidence.

### 5. Single File layer add — PASS

- Baseline: `projects\catalog-one.sdc`, 2 assets / 1 layer / 1 composition / 1 output.
- Input: `media\v02.mp4`, 6,214 bytes, SHA-256 `D9C1041B5319D6D3942BE9C17AE92B4E4455AA6AE52F15389D93F7CE41EAA4AE`.
- Native report: `Added video layer 2 from 1 prepared media source.`
- Artifact: `projects\file-add.sdc`, 13,234 bytes, SHA-256 `4C169F43E430FC71D2AD5E4023EF038CBD5FB8DB4F9EE13E75224BB3E9561CAB`.
- Result: assets 2 -> 3; layers 1 -> 2; composition/output counts unchanged.
- Layer 2 referenced asset 20; asset 20 and the layer source projection both pointed to `v02.mp4`; Main layer IDs were `1,2`.

### 6. Single Still layer add — PASS

- Baseline: the preceding File-add runtime state.
- Input: `media\still-valid.png`, 1,505 bytes, SHA-256 `BC139006F52713156501A9C234AF5599398367D67083136DFD20FB6CCE5A035E`.
- Artifact: `projects\still-add.sdc`, 15,737 bytes, SHA-256 `B3484A8AE3C639BA549E959EEBAD84F0EAFF39655D31CEE8F62C6504F41D9E40`.
- Result: assets 3 -> 4; layers 2 -> 3; composition/output counts unchanged.
- Asset 21 and layer 3 used `StillImage`, referenced each other exactly, and persisted the input hash/byte size; Main layer IDs were `1,2,3`.

### 7. Normal mixed import partial success — PASS

- Baseline: `projects\still-add.sdc`, 4 assets / 3 layers / 1 composition / 1 output.
- Selection contained the corrupt 16-byte `corrupt.mp4`, already-known/reused inputs, and ten new valid clips.
- Native report: `メディアライブラリ: 10件 読み込み、4件 再利用、1件 失敗。`
- Artifact: `projects\mixed-normal.sdc`, SHA-256 `5FCF253A0FF3D3C91108870892A44E0EDCA314539B98FBD29DD4DC368F2603F7`.
- Result: assets 4 -> 14; layers remained 3; compositions remained 1; outputs remained 1.
- The corrupt file was not persisted as an asset; valid `v03.mp4` through `v12.mp4` were persisted.

### 8. First-run Bootstrap mixed invalid fail-fast — PASS

- Baseline: `projects\empty-b.sdc`, 0 assets / 0 layers / 0 outputs.
- Selection: `corrupt.mp4` plus valid `match-copy-v01.mp4`.
- Native report: `Error: First-run VJ setup requires every selected media file to prepare successfully; no project changes were made.`
- Failure detail identified `corrupt.mp4` as invalid media metadata/probe input.
- Result after completion: 0 assets / 0 layers / 0 outputs; first-run CTA remained available.
- Baseline artifact remained 4,706 bytes, SHA-256 `EFEF469E930BD1D8FA87A3B67A07C670FC1A8888F71D555B970EA65C554D18FD`.

### 9. Cancel during the first hash — PASS

- Process was restarted with `SYNDOCAL_QA_MEDIA_HASH_PAUSE_MS=30000`; the exact checkout executable was the only Syndocal process and its window was maximized before interaction.
- Input: `media\v12.mp4`, 20,606,544 bytes (>1 MiB), SHA-256 `1DB23DAAE678294856645DAF6E2E6C01E3C7CE8B7D9D64A3D9BF31564AAE74B7`.
- The native operation rail visibly reached `ハッシュ / メタデータ検査中`; Cancel was pressed during the 30-second first-hash pause.
- Native result: `VJセットアップに失敗しました: Media asset operation was cancelled` and the runtime returned to 0 assets / 0 layers / 0 outputs.
- After waiting beyond the full pause interval, no delayed completion or catalog/layer/output mutation appeared.
- The source retained the same 20,606,544-byte length, UTC last-write timestamp `2026-08-13T12:46:07.4152871Z`, and SHA-256 before/after cancellation.
- A same-directory rename-and-restore round trip succeeded immediately after the delayed check, proving that no retained file lock remained.

### 10. Project replacement during prepare/hash — PASS

- A fresh process was restarted with the same 30-second first-hash pause and `media\v12.mp4`; the native rail again visibly reached `ハッシュ / メタデータ検査中`.
- While hashing was paused, the in-app project menu remained usable and opened `projects\bootstrap-success.sdc` through the native project picker.
- The UI immediately switched to the replacement project's 1 asset / 1 layer / 1 output state.
- After waiting beyond the full 30-second pause interval, the replacement state remained 1 / 1 / 1 with no stale completion, error, or count change from the superseded operation.
- `bootstrap-success.sdc` remained 10,119 bytes with SHA-256 `5863BE40C4A6396ACE026B7AEAC507D1B91ECBC765FAC9696121CD63EA0FD8E8`.
- `v12.mp4` remained 20,606,544 bytes with SHA-256 `1DB23DAAE678294856645DAF6E2E6C01E3C7CE8B7D9D64A3D9BF31564AAE74B7` and unchanged UTC last-write timestamp.

### 11. Missing availability — PASS

- Baseline: `projects\catalog-one.sdc`, with orphan catalog asset `v01` at `media\v01.mp4` and zero layer references.
- Before inspection, the exact source was moved to a same-directory parked path; the source path was absent while the original bytes remained recoverable.
- The native per-asset Verify action reported `欠損` for `v01`.
- Native status: `メディア利用可否: 0件を検証済み、1件は要確認。プロジェクト履歴は変更されていません。`
- The source was restored by rename. Its 6,217-byte length, UTC last-write timestamp `2026-08-13T12:46:06.5879161Z`, and SHA-256 `36791A9A700A831023047700C49E2D4576188D54FB7E02D6AFB91F58C11B36EE` matched the pre-test values exactly.
- `catalog-one.sdc` retained SHA-256 `B8464015039973511051992886857E2B64123F243CEE158CDF461DF42FEB9934` and UTC last-write timestamp `2026-08-13T13:15:25.6802559Z`.

### 12. HashMismatch availability — PASS

- The original `v01.mp4` was parked, and the distinct valid `mismatch-valid.mp4` bytes were copied to the exact catalog path `media\v01.mp4`.
- Replacement bytes were 76,750 bytes with SHA-256 `3107CB1EAD787328ADF4737136E3DE1ABA2FA194BADCBA38EC20C4BF280A6C0F`, distinct from the catalog's expected 6,217-byte / `36791A...B36EE` identity.
- The native per-asset Verify action reported `ハッシュ不一致`, not Missing or Verified.
- Native status again stated that one asset required attention and that project history was unchanged.
- The disposable copied replacement was removed and the parked original was restored. The original source length, timestamp, and SHA-256 matched exactly; no parked file remained.
- `catalog-one.sdc` retained the same SHA-256 and last-write timestamp, proving that machine-local availability inspection did not mutate saved project state.

## Remaining rows — not accepted

1. All-reference Relink, including default mismatch refusal and a successful matching-content relink.
2. Restart/load with no implicit source-file touch.
3. Explicit Save/reload semantic equivalence.

## Evidence boundaries

- Browser/TypeScript/static gates are not substituted for the native observations above.
- The first twelve rows have native UI outcomes plus saved-project/hash evidence. Internal coordinator revision/history-generation counters were not directly exported by the UI; where Undo menu state was visible it was consistent with one operation, but this document does not claim an unobserved counter value.
- Screenshot evidence remains in the supervising Codex task transcript; saved `.sdc` artifacts and fixture hashes are the durable local evidence.
- This checkpoint is not Media Asset T1 completion and is not a release-candidate GO.
