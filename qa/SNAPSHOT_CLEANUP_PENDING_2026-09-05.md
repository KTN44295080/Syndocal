# Snapshot軽量化・映像プレビュー・タイムライン操作の検証記録

基準: `codex/syndocal-v1.2` / `801fbf5e47710f5ac3703d4099171832996e6f7b`。
以下は時系列の検証記録。現在の実行状態と未確認事項は末尾の最終checkpointを参照。
当初の操作制限は、後続のユーザー指示でこのcheckoutの終了・ビルド・起動が許可された。
アプリ内の自動操作は行わず、ブラウザ検証とnative起動確認、ユーザー操作の受入を区別する。

## 実装と独立レビュー

- engineの`timeline_bank_snapshot`で、全bankをcloneした後にactive項目を破棄・置換する
  旧処理を、active以外の項目だけcloneする処理へ変更。最初の一致・並び順・未登録時の
  appendを維持。lock、publication、audio fence、mutation ownershipは変更しない。
- main.rsの保存用snapshot変換6関数を`project_snapshot_persistence.rs`へ分離。
  protocol/stdのみ依存し、Tauri/AppState/I/Oは持ち込まない。5関数はpub(super)、
  Timeline単体helperはprivate。検証と正規化の呼出し順はmain側に保持。
- 実装を2担当に分け、別担当が安定差分をread-onlyで独立レビュー。blocking findingなし。
  supervising側も差分を確認。抽出関数全文は可視性/改行以外HEADと一致。

## 検証

再現helper:
`target/qa/recording-atomic-20260905/run-native.mjs`。
全Cargo開始前にexact Community MSVC14.44.35207の絶対linker pin/where-firstを検証する。
テストはrelease exeを終了・置換せず、UIや物理出力の検証には数えない。

- `node app/scripts/check-project-storage-helpers.mjs`: PASS。
- owned diffの独立レビュー: PASS。
- `cargo test -p engine --release --locked timeline_bank_snapshot -- --include-ignored --nocapture --test-threads=1`:
  PASS、2 passed/0 failed/0 ignored。旧実装と空・先頭・中間・末尾・未登録・重複IDを比較し、
  所有権とruntime除去を確認。compile warning 0。
- 固定synthetic benchmark: 4096イベントのactive Timelineを含む3項目bank、10000反復。
  旧678.9861ms→新138.6032ms（1回約67.90→13.86µs）。入力/出力black_box、完全一致を確認。
  単回・旧→新の順で測った局所処理の比較であり、実show全体やlock待ちの改善率ではない。
- 既存engine回帰 `timeline_bank_switches_exact_active_entry_and_persists_runtime_free_order`、
  `timeline_layer_legacy_display_is_synthetic_but_persistence_stays_authored`: 各1 passed。
- `cargo test -p syndocal --locked <filter> -- --test-threads=1`: 以下9filterを個別実行し、
  すべて各1 passed/0 failed。実行scriptと各ログは`target/qa/nightgirl-av/`。
  `project_snapshot_for_save_drops_volatile_runtime_state`、
  `blind_programmer_state_is_removed_from_project_save_and_load_boundaries`、
  `project_save_reload_uses_authored_video_and_strips_audio_runtime`、
  `cue_capture_uses_authored_video_instead_of_rendered_modulation`、
  `node_graph_preset_persistence_never_serializes_audio_runtime`、
  `rendered_modulation_and_runtime_telemetry_do_not_create_history_changes`、
  `idle_engine_persistence_image_is_stable_across_runtime_ticks`、
  `project_timeline_layer_normalization_preserves_active_bank_authored_projection`、
  `project_timeline_layer_load_rejects_active_bank_mismatch_before_canonical_normalization`。
- 今回のengine release test/アプリdebug testのfirst-party compiler warningsは0。
  前checkpointの最終構成記録0→今回0、増加0。ただし同じdebug構成の変更前再buildは未実施。
  Gitの改行変換通知はcompiler warningに数えない。所有差分`git diff --check`: PASS。
- native build/起動/画面確認: 未実行。ユーザーが操作再開を許可した後の残件。

## 追加分: full同期の共有とメディアプレビュー責務

- `SnapshotSyncState`の履歴とfull応答を同一の不変`Arc<EngineSnapshot>`で共有。
  full再同期で発生する追加deep cloneを除去。履歴は4件、revision枯渇時は変更前にエラー。
  mainの応答型もArcへ合わせるが、serdeの既存rc対応によりJSON形式は変わらない。
  delta経路にもArc割当が加わるため、すべての要求が速くなるとは主張しない。
- メディアプレビューの寸法・位置・catalog形状検証とprivate pathの描画snapshot構築を
  `media_asset_preview_contract.rs`へ分離。4関数は可視性/改行以外HEADと一致。
  authority、実ファイルhash検証、private copy寿命、renderer接続はmainに保持。
- 別担当の独立read-onlyレビューで両差分ともblocking findingなし。
- `cargo test -p syndocal --locked snapshot_sync::tests -- --include-ignored --nocapture --test-threads=1`:
  PASS、9 passed/0 failed/0 ignored、compiler warning 0。
  full応答と履歴の同一pointer、eviction/service破棄後のserialization、旧所有型と同じ
  JSON bytes、最終解放を確認。既存の2client再構築、null/削除、revision枯渇回帰も成功。
- 同じcompile済みtest binaryをexact MSVC helper経由で実行:
  `media_asset_preview_contract::tests` 2 passed、`media_asset_thumbnail_` 4 passed、
  `media_asset_preview_private_copy_for_real_still_never_reuses_catalog_path` 1 passed。
  `timeline_runtime_sync_wire_keeps_full_and_delta_runtime_outside_persisted_timeline` 1 passed。
  UI表示や物理出力は使わない。
- 比較benchmarkを4ケース・各1000要求・旧新順序を交互にした3回へ拡張。
  旧4件履歴実装と応答件数・最終応答bytes・履歴bytesが一致することを計測外で検証。
  `cargo test -p syndocal --locked benchmark_full_snapshot_sync_shared_capture -- --ignored --nocapture --test-threads=1`:
  PASS。debug中央値（1000要求合計）はfull-heavy 1195.16→528.61ms、
  mixed-heavy 613.16→584.43ms、steady-heavy 576.91→602.49ms、
  steady-light 4.3842→4.3971ms。serializationは除外。
  未最適化の通常deltaでは明確な改善を確認できないため、release testで判断を補う。
  release testはdeps内のテストexeのみを生成し、実行中のreleaseアプリは終了・置換しない。
- `cargo test -p syndocal --release --locked benchmark_full_snapshot_sync_shared_capture -- --ignored --nocapture --test-threads=1`:
  PASS、1 passed/0 failed、compiler warnings 0。最適化test compileは3m18s。
  1000要求合計の3回中央値:

  | ケース | 旧 | 共有後 | full/delta件数 |
  | --- | ---: | ---: | ---: |
  | full-heavy | 234.9692ms | 133.1252ms | 1000/0 |
  | mixed-heavy | 130.2736ms | 114.3125ms | 33/967 |
  | steady-heavy | 111.0318ms | 111.1972ms | 1/999 |
  | steady-light | 0.9136ms | 0.8438ms | 1/999 |

  steady-heavyの同一roundの差（共有後−旧）は−1.6056/+2.1081/+0.1654msで、
  安定した改善・悪化とは判定しない。full/mixedは3回とも改善したため限定効果として採用。
  入力captureのclone、publish、前回応答破棄を含み、serialization/実show/物理出力は含まない。
  full-heavyのroundごとの変動も大きく、製品全体の速度向上率として転用しない。
  4件履歴、full/delta件数、最終応答・履歴bytesの一致は全ケース成功。
  benchmark拡張も独立レビュー済み、blocking findingなし。
- 前回からの純粋関数分離2件を合わせ、main.rs差分は14追加/196削除（純減182行）。
  ソース分割自体による実行速度向上は主張しない。今回の追加範囲もnative gate待ち。

## 追加分: 保存用映像データの再利用・mapping検証分離

- `build_persistence_snapshot`がbuilder生成済みのauthored videoを再構築していた箇所を除去。
  同じrendered imageから生成される既存Someを可変参照し、従来のlayer/chain/presetの
  一時Event消去をそのまま適用する。runtime/描画用snapshotは変更しない。
  builderが必ずSomeを設定する内部不変条件はexpectで明示し、別映像への代替はしない。
  削減は保存要求内の重複構築1回であり、通常tick全体の性能向上率は未計測。
- mapping値・編集field・preset labelの検証3関数を`video_output_mapping_contract.rs`へ分離。
  protocolのみ依存。エラー文、検証順序、mask slice前のcount検査、canonical fieldを維持。
  Tauri/ファイル形式/参照検証/実出力の操作はmainに残す。既存テストは移動・緩和しない。
- 2担当で実装、別担当の独立read-onlyレビューでblocking findingなし。
  supervising側も差分を照合。main差分合計は19追加/302削除（純減283行）。
- `cargo test -p engine --release --locked persistence_video_reuse -- --nocapture --test-threads=1`:
  PASS、1 passed。live pulseの旧構築結果一致、layer/chain/preset消去、rendered/runtime不変を確認。
- `cargo test -p engine --release --locked <filter> -- --test-threads=1`: 以下各1 passed。
  `persistence_snapshot_keeps_rendered_video_separate_from_authored_state`、
  `persistence_snapshot_zeros_transient_isf_events_during_active_pulse`、
  `video_effect_chain_c1_event_pulse_and_persistence_strip_both_representations`。
- `cargo test -p syndocal --locked <filter> -- --test-threads=1`: 以下各1 passed。
  `video_output_mapping_validation_rejects_invalid_ratio`、
  `video_output_mapping_field_is_canonicalized_and_required`、
  `video_output_mapping_preset_label_is_trimmed_and_required`、
  `video_output_mapping_preset_file_serializes_and_validates`、
  `project_file_validation_rejects_invalid_video_mapping_presets`。
- 計9件成功、engine release/application debug compileともfirst-party warnings 0。
  直前の同構成記録0→今回0、増加0。所有差分のdiff checkも成功。
  再現script/ログは`target/qa/snapshot-cleanup-20260905/`。nativeアプリの操作・終了・置換なし。

## 追加分: 保存Timelineの既存bank再利用・Stage値の分離

- 保存時のroot Timeline全量cloneでactive bankを置換する処理を除去。
  元の保存用正規化を`normalize_persistence_timeline`へまとめ、rootと既存の最初のactive項目へ
  同じ処理を適用する。内部bank builderが必ずactiveを生成することをexpectで明示。
  既存項目の順序・後続の重複、raw layer/event情報、derived audio、runtime消去を維持。
- Stageの形状・色・範囲・preset/collection正規化8関数を`stage_map_contract.rs`へ分離。
  8関数は可視性/改行以外HEADと一致。std/protocolのみ依存し、ID採番、transaction receipt、
  digest、ファイル形式はmainに保持。色helperはprivate。
- 別担当による独立レビューで双方blocking findingなし。supervising側も差分・testを照合。
  main差分累計は25追加/430削除（純減405行）。
- 新規旧clone方式との比較はlegacy/explicit/derived audioの3モード×bank6形状の18ケース。
  root/bank全値、最初のactiveとの一致、runtime不変を確認し、PASS。
  再現script: `target/qa/snapshot-cleanup-20260905/run-timeline-stage-tests.mjs`。
- `cargo test -p engine --release --locked persistence_timeline_reuse -- --nocapture --test-threads=1`:
  1 passed（上記18ケース）。以下3filterも各1 passed:
  `timeline_layer_legacy_display_is_synthetic_but_persistence_stays_authored`、
  `timeline_bank_switches_exact_active_entry_and_persists_runtime_free_order`、
  `timeline_audio_explicit_clips_and_master_fields_snapshot_roundtrip`。
- `cargo test -p syndocal --locked <filter> -- --test-threads=1`: 以下各1 passed:
  `stage_map_config_validation_rejects_invalid_bounds`、
  `project_stage_object_validation_rejects_duplicate_ids_and_invalid_size`、
  `stage_map_file_candidate_is_normalized_before_transactional_import`、
  `d4_stage_add_digests_user_intent_before_one_time_allocation`、
  `d4_stage_unchanged_outcomes_are_receipt_backed_without_synthetic_history`、
  `d4_stage_applied_transactions_undo_redo_round_trip_is_exact_through_production_paths`。
  Stage統合harnessはDMX disabledのテストengineを使用し、実UI操作は行わない。
- 計10テスト成功。engine release/application debugのfirst-party warnings 0、直前0→今回0。
  所有差分のdiff check成功。nativeアプリ終了・置換・画面操作なし、commit/pushはnative受入待ち。

## Native再開時の手順

対象テストの完了後も、実show全体のCPU/FPS改善やnative受入完了とは主張しない。
操作再開後に既定のnative gateを実施し、適用範囲の検証成功後に所有差分をcommit/pushする。
別所有`app/scripts/check-viewport-containment.mjs`はstage/編集しない。
SHA256は`3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`のまま。
HEAD/upstreamは基準commitで一致。今回の実装・新規module/test・本文書は未commit。
一般snapshot構築、深いdelta比較、代表showのwriter待ち計測は別の残件。

NightGirlの別ファイル`C:/Users/kouty/Downloads/DSF2026_NightGirl-AV-test.sdc`は
前景3840x1536・背景1920x768の2出力（双方5:2）、前景MP4音声のみ、開始0msで準備済み。
DATE子タイムライン内の44照明ブロック、灯体、profile、groupの保持をファイル比較で確認。
前景AACはffmpegで全区間decode成功。元のtest.sdcは保持。
ネイティブ読み込み・2画面描画・可聴確認・知覚的同期は未確認。
照明306024msに対し前景307207ms/背景307274msなので末尾は同じ長さではない。
詳細のローカル証拠は`target/qa/nightgirl-av/verification.json`。

## 継続作業: profile契約とlive fixtureの重複走査

- `fixture_profile_contract.rs`へ6関数/256行を抽出。純粋なprofile/geometry/emitter/
  custom参照検証とfootprint計算のみ。全本文はHEADと可視性・改行以外一致。
  main差分累計30追加/680削除（純減650行）。独立レビューはblocking findingなし。
- `engineSnapshotLiveState.ts`は選択済みactive cueをtarget集計へ渡し、同一call内の
  Set生成と全cue走査の重複を除去。順序・後勝ち・属性casefold・空時fixtures identityを保持。
- `node target/qa/snapshot-cleanup-20260905/check-live-fixture-reuse.mjs`: PASS。
  HEADとの16入力全値一致/非破壊、空identity、cue getterアクセス2→1を確認。
- `node app/scripts/check-media-asset-authority.mjs`: PASS。
- `pnpm --dir app exec tsc --noEmit`: exit 0、診断なし。独立レビューでもblocking findingなし。
- profile Rust対象テスト、および追加のauthored Timeline構築最適化は検証進行中。
  Native受入/commit/pushは引き続き未実施。

## 継続作業: authored Timelineの不要なruntime投影を省略

- `timeline_snapshot`/`authored_timeline_snapshot`はprivate const-generic builderを共有。
  liveは旧値を直接初期化、authoredは生成後に消していた子transport/Guide/Click/Follow等を
  最初から生成しない。derived audioのpayload複製を省くが、表示layer合成には全audioを使用。
- 旧live構築＋旧authored消去方式を独立oracleとして、空/implicit/explicit/derived切替、
  子transport、128 Guide、Clickキュー/overflow、Follow faultを全値比較。非破壊も確認。
  独立レビュー最終版承認、blocking findingなし。
- 最初のdefaults後上書き方式はlive側の小さな増加を受けて修正。
  各fieldをconst条件で直接初期化し、duration評価順も旧実装へ揃えた。
- 最終command（exact MSVC wrapper経由）:
  `cargo test -p engine --release --locked timeline_snapshot_projection -- --include-ignored --nocapture --test-threads=1`
  2 passed、warnings 0、compile 1m13s。
  log: `target/qa/snapshot-cleanup-20260905/timeline-projection-final.log`。
- 同じ入力/関数pointer/black_boxで1,000回×順序交互3roundの中央値:

  | workload | old | current |
  | --- | ---: | ---: |
  | light live | 170.8 us | 191.5 us |
  | light authored | 154.3 us | 114.7 us |
  | populated live | 6.2215 ms | 6.2326 ms |
  | populated authored | 6.0333 ms | 0.2685 ms |

  populated authoredはこの合成入力で約95.5%削減。live軽量は各roundで+2.6〜30.3ns/call、
  populated liveはroundにより増減。live改善/全経路無劣化や実show FPS改善とは主張しない。
  runtime collection複製削減の利益を採用し、実show/nativeの評価は残す。
- 追加回帰filter `persistence_timeline_reuse`、`timeline_bank_snapshot_tests`、
  `timeline_layer_legacy_display_is_synthetic_but_persistence_stays_authored`は各1 passed。
  最後の保存audio回帰とprofile契約回帰は実行中（以下で確定）。

## 今回の対象検証確定

- `run-projection-regressions.mjs`: 上記4filter各1 passed（保存audio回帰もPASS）。
- `run-fixture-contract-tests.mjs`: 9filter/11tests PASS。
  missing geometry、parent cycle、custom/profile geometry mismatch、emitter calibration、
  missing custom ref、invalid patch、batch conflict publication、prepared patch2件、patch update2件。
- 最終変更のRust合計17tests PASS（projection2 + regression4 + profile11）。
  first-party warnings: engine release 直前0→今回0、application debug 直前0→今回0。
  TypeScript診断なし、16ケース旧実装比較とmedia authority既存checkもPASS。
- 実行中アプリ本体と別所有viewport scriptのSHA256は上記値から不変。
  branch/HEAD/upstreamも上記基準のまま。未commitの所有ファイルへ追加:
  `app/src/engineSnapshotLiveState.ts`、`app/src-tauri/src/fixture_profile_contract.rs`、
  `crates/engine/src/timeline_snapshot_projection_tests.rs`。
- 次の候補であるTimeline automation validationはgroup正規化helperを親から参照するため、
  単純移動すると逆依存になる。共有group契約の境界整理なしに機械的抽出は進めない。
- 現時点の確定変更は対象ソフトウェア検証済み、native受入待ち。
  アプリ終了/再起動/画面操作/実出力を必要とする検証はユーザーの操作保留を維持。
  Native gate成功前のcommit/pushは行わない。

## 追加候補1〜3と操作窓口の復旧責務（2026-09-05 続行）

- `snapshot_public.rs`の公開コピーで`authored_video`を初めから除外。ほか33fieldを明示し、
  protocol field追加はコンパイルで追従を強制。snapshot/try_snapshotのlock・poison分岐保持。
- Timeline映像投影でvideo laneなし/desired clipなしの場合、availabilityを取得・複製しない。
  active経路、staged/shared availability、used ID更新は維持。
- `createMediaThumbnailController.ts`へpermission/cache/generation/購読を分離。
  source配列参照memoで無関係なsnapshot更新による全件map/JSON化を抑制。
  独立レビューで既存のcached同期authority判定の余計な購読を発見し、判定だけuntrack。
  実際の現在値の確認は省略せず、durable E/R/H scalarで変更を追跡する。
- 候補4は最初の責務として`createProjectTransactionRecoveryController.ts`へ約330行の
  Begin/query/adopt/Commit/Cancel/ACK復旧を分離。pending receiptはinstance private。
  raw IPC（6命令のみ型で許可）/history通知/status通知/waitを注入。Appへの逆importなし。
  コマンド分類・operator lock・registration・mapping barrier・authority保持はAppに残る。
  操作窓口約950行全体の分離完了とはしない。残りの切出しではsource-based routing
  checks（別所有viewport harnessを含む）への影響とmounted owner lifecycleを先に整理する。
- App.tsxはこの追加分で28追加/470削除、純減442行。main.rs純減650行は前回から不変。
- 上記各実装は別担当による独立レビューでblocking findingなし。親も実コード/結果を確認。

### 対象テストと限定計測

全Cargoは上記exact MSVC wrapper経由、UI/実機出力なし。
`cargo test -p engine --release --locked <filter> -- <options> --test-threads=1`:

| filter | options | 結果 |
| --- | --- | --- |
| snapshot_public_tests | --include-ignored --nocapture | 3 passed |
| timeline_video_empty_projection_tests | --include-ignored --nocapture | 2 passed |
| snapshot_read_tests | 通常 | 3 passed, 計測1 ignored |
| timeline_video_media_asset_projection | 通常 | 2 passed |
| timeline_follow_ltl5_target_video_automations_are_isolated_from_outgoing_projection | 通常 | 1 passed |

計11 passed。engine release first-party warnings 直前0→今回0。
public poison testの意図的thread panicは捕捉済みでテスト成功。
再現runner: `target/qa/snapshot-cleanup-20260905/run-public-projection-regressions.mjs`。
publicログ: 同directoryの`public-copy.log`、ほか`*-followup.log`。

同条件1000回×順序交互3round中央値（合成入力、実show FPS/lock-waitの証明ではない）:

| workload | old | new |
| --- | ---: | ---: |
| public copy light | 493 us | 465.4 us |
| public copy 1024 authored video layers | 170.9562 ms | 2.5589 ms |
| availability4096 / no video lanes | 29.9865 ms | 10.3 us |
| availability4096 / no active clips | 7.9398 ms | 48.3 us |
| availability4096 / active | 3.7583 ms | 3.6087 ms |

- `node app/scripts/check-media-thumbnail-controller.mjs`: PASS。
  実Solidで同一配列100更新のscan各1回、別layer配列のみ再scan、cached authority同値100通知で
  追加checkなし、真のauthority変更はcheckあり。opt-in/retired result/reset/disposeもPASS。
- `node app/scripts/check-project-transaction-recovery-controller.mjs`: 6シナリオPASS。
  reply loss、history-before-ACK、ACK retryのhistory1回、instance分離、busy receipt再開等。
- `check-project-transaction.mjs`、`check-patch-transaction-d2.mjs`、
  `check-backend-operator-contract.mjs`、`check-vj-first-run.mjs`、
  `check-media-asset-authority.mjs`: 最終統合後すべてPASS。
  checkerの移設追従はassertionを削除せず、新module/portの契約へ変更。
  patch D2のhistory→ACK regexは今回変更前から旧直書き方式を要求して失敗していたため、
  現行settlementのapply→ACK順序＋factory結線へ修正し、behavior testでも証明。
- `pnpm --dir app build`: PASS、Vite 323 modules、警告なし。
  その後のuntrack修正も実Solid testと最終`tsc --noEmit --project app/tsconfig.json`がPASS。
  TypeScript診断0。最初の今回frontend build警告0（それ以前の比較baselineは未測定）。
- 既存`check-vj-media-import-access.mjs`全体は今回変更と無関係の
  `data-lighting-context-tab="lighting"`/`aria-keyshortcuts="E"`要求で停止する。
  廃止済みタブを要求するassertionを今回のために弱めていない。対象thumbnailは上の実Solid検証。
  ログ`target/qa/snapshot-cleanup-20260905/media-import-access.log`。総合check成功とはしない。

### 所有差分・残件

今回追加の所有: App.tsx、2controller、2behavior checker、snapshot_public module/test、
empty projection test、関連6source checker（viewport以外）。前回までのpendingを保持。
保護viewport scriptとrelease exe SHA256は上記値から不変。アプリ終了/置換/操作なし。
branch/HEAD/upstreamは上記基準のまま。native gate未実施、commit/push未実施。
残件: 操作窓口残りの責務分離、既存media-import総合checkのタブ契約棚卸し、
実show/native受入、NightGirl AV-testのnativeロード・前景/背景表示・音声同期。

## Native build完了・ユーザー手動受入へ（2026-09-05 16:38 JST）

ユーザーは「操作は手順を案内すれば本人が行う」と指定し、Syndocal終了済みと確認。
read-only process inventoryおよびbuild wrapperでこのcheckout exeの停止を確認した。
`pnpm --dir app tauri build --no-bundle` exit0。exact Community14.44 linker pinとwhere-firstを
Cargo前に印字確認。frontend323modules成功、本体release compile2m55s、first-party warning0。
log: `target/qa/snapshot-cleanup-20260905/native-build.log`。

新exe: `C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe`
63,686,656 bytes、SHA256 `1ACDE356DB3D600A54C92237EF9E0BE802DB9BFB94E4D90E0DE429FC074C8534`。
製品versionはalpha.69（ローカル確認用、配布版の発行なし）。build後もSyndocal processなし。
こちらから起動/UI操作はせず、ユーザーにこのexeの起動・最大化・AV-testロードを案内する。
一つの応答するwindow、nativeロード、前景/背景preview、可聴/同期確認はまだ未検証。
これらの確認前のnative受入完了・commit/pushは行わない。
`node target/qa/nightgirl-av/verify.mjs`再実行PASS、元project/照明44blocks保持。
保護viewport scriptのSHA256およびHEAD/upstream equalityは上記基準から不変。

## 手動確認からの修正: Space再生/停止とTimeline映像の低fps

ユーザーが新exeで「Spaceは再生できるが一時停止できない」「映像previewが非常にカクカク」と報告。
native受入は未完了として扱い、以下を修正した。

- Spaceは旧resolverで常に次cueへ進んでいた。Timeline surfaceの修飾なしSpaceだけ既存
  toggleTimelinePlaybackへ割当。他画面Space/ShiftSpace、入力/repeat/modifier除外は維持。
  keyboard controllerの判定もroot固定ではなくAppのactiveTimeline（選択中親/子）へ一致させる。
- Timeline previewの旧global10starts/sは2outputs各5fpsを上限としていた。
  per-output30fps目標/global60starts/s上限へ変更。single-flight、round-robin、backoff、
  generation/config/hidden/unmount fence、URL解放は保持。処理時間を含むstart間隔を使用。
  timer待ちはceilで小数deadline直前の0ms反復を防止。3outputs以上/遅いdecoderは各30fps保証なし。
- backend read-only調査では独立fpscapなし。JPEG binary IPC、実request320系寸法、
  共有renderer lock内decode/render、JPEG encodeはlock外。無闇な並列化やdecodebudget変更は行わない。
  libavは要求位置差250ms超で再open/seekするため、旧2output200ms間隔に遅延が加わる場合の
  負荷増幅も考えられるが、実環境の計測では未証明。
- `node app/scripts/check-timeline-space-keyboard.mjs`: 実controller play/pause/resume、
  root/child矛盾、empty、repeat、modifier、外surface PASS。
- `node app/scripts/check-project-shortcuts.mjs`: 15,552 shortcut matrix PASS（担当実行）。
- `node app/scripts/check-timeline-transport-runtime.mjs`: PASS（担当実行）。
- `node app/scripts/check-timeline-output-monitors.mjs`: PASS。fakeclock1/2/4outputsでfairness、
  1〜2outputs各28〜30starts/s、global<=60、single-flight、遅延/設定/非表示/解放を確認。
- `node app/scripts/check-timeline-output-preview-browser.mjs`: headless1920/1280 PASS。
  2output実アス比とunmount時IPC停止。実アプリの滑らかさの証明ではない。
- 別担当の独立レビューで双方blocking findingなし。
- ユーザーへアプリ保存/終了を依頼済み。終了返答まではnative exeを停止・置換しない。
  frontend build検証中。native再build・ユーザー再確認は未実施。
- 最終frontend `pnpm --dir app build` exit0、Vite8.24s、compiler/bundler警告0。
  log: `target/qa/snapshot-cleanup-20260905/timeline-playback-preview-frontend.log`。
  native更新はユーザー終了確認待ち。

## 起動・終了の担当更新

ユーザーはSyndocal終了済みと報告し、起動・終了はassistantに実行を依頼した。
以降、このcheckoutの正確なexe pathに限るプロセス終了/起動は許可済みとして扱う。
アプリ内操作および映像/音声/Spaceの実見確認はユーザーが担当。
Space/cadence修正版の`pnpm --dir app tauri build --no-bundle`を開始。
exactpath停止およびexactMSVC pin/where-firstをログで確認。
log: `target/qa/snapshot-cleanup-20260905/timeline-playback-preview-native.log`。

### Space/cadence修正版のnative build・起動完了

- `pnpm --dir app tauri build --no-bundle` exit0。frontend6.44s、本体release1m51s。
  exactMSVC pin/where-first確認済み、compiler/bundler warning0（直前0→今回0）。
- 新exe SHA256 `22E625C225AD1AB6035EE855F902F7AEC79287940ABFFF5921B27FF76A6DBA82`。
- exactpath既存processなしを確認後、`Start-Process -FilePath <checkout exe> -WorkingDirectory <checkout> -WindowStyle Maximized`で起動。
  PID51836、exe pathはこのcheckout。初期WaitForInputIdleはGUI初期化前に例外を返したため、
  完了とはせず後続read-only process/window列挙で確認。
- Win32 visible-window列挙にてtitleが`Syndocal`のwindowは1件（handle119021432）、
  responsive=True/maximized=True。別途runtime helper windowは存在。UI操作は行っていない。
- native buildと起動応答確認は成功。Space停止/再開、preview実際の滑らかさ、音声同期は
  ユーザーの手動再確認待ち。これらが未確認のため機能受入完了とはしない。

### 追加: プレビュー処理によるUIスレッド占有の解消

- ユーザーはSpace動作改善を確認、残る重さを報告。5秒のread-only process標本ではnative CPUは1core換算10.9%、RSS182.1MB。一部Responding=False。再生状態未観測のためCPU飽和や実fpsの証明にはしない。
- 同期Tauri commandからWebView callback上でsnapshot/decode/render/JPEGを実行していた。`get_live_video_monitor_frame`をasync + `spawn_blocking`へ移し、AppHandleからworker内でStateを取得。renderer try_lock/single-flightは維持。
- ProgramはJPEG後にもoutput epochを再照合し、退役epochなら画像なしBUSYを返す。IPC配信までの原子的保証ではない。Previewの既存二重generation fenceを維持。
- `reset_vj_preview_renderer`はState wrapperからAppState参照へ変更。初回Rust compileのE0308を修正後、同じ対象テストを再実行。
- `node app/scripts/check-timeline-output-monitors.mjs`、`node target/qa/snapshot-cleanup-20260905/check-monitor-worker-extraction.mjs` PASS。独立レビューblocking findingなし。
- `node target/qa/recording-atomic-20260905/run-native.mjs cargo test -p syndocal --locked live_video_monitor_ -- --test-threads=1`: 5 passed、警告0。log `monitor-worker-tests-final.log`。
- `pnpm --dir app tauri build --no-bundle`: exit0、frontend7.95s/native2m25s、警告0。exact MSVC pin/where-firstとexact checkout PID51836停止を確認。log `target/qa/snapshot-cleanup-20260905/monitor-worker-native.log`。
- 新exe SHA256 `C57C38D34851AA573C0C3EE56E4D0B0D937DEF74381499EC128CAA9621F2024C`、PID76516。`powershell.exe -NoProfile -ExecutionPolicy Bypass -File target/qa/snapshot-cleanup-20260905/launch-checkout.ps1`で起動。初回検査はmaximize適用前で失敗、後続read-only Win32再列挙でSyndocal 1window responsive=True/maximized=Trueを確認。証拠 `monitor-worker-launch.json` / `monitor-worker-window-recheck.json`。アプリ内操作なし。
- GPU質問: Timeline monitorはCPUでH264 decode、RGBA合成/mapping、JPEG encode。実出力側にはGpuSurfacePresenter経路があるが、その存在はこのmonitor経路のGPU化を意味しない。今回の変更はUIスレッド隔離であり、hardware decode実装やCPU削減ではない。
- branch/HEADはcodex/syndocal-v1.2 / 801fbf5e47710f5ac3703d4099171832996e6f7b。前段の所有dirty変更を維持。実際の滑らかさ・音声同期・操作感はユーザー受入待ち、commit/push未実施。次: 同一映像でUI応答と映像カクつきを分けて確認し、残ればdecode時間の実測から対処を選ぶ。

### GPU化: 実装中の境界と予備測定

ユーザー「GPUにした方がよくね？」→「ではおねがい」で実装承認。第一単位はWindows H.264/H.265 D3D11VA decode。CPU readback/scaling/composition/JPEGは残り、GPU decodeのみを全経路GPU化と呼ばない。主担当はlib.rs/Preferred error routingと公開診断、独立担当はlibav decoder/hardware module、別担当が共有surface設計調査、reviewはさらに独立。

実機はRTX5090 + AMD Radeon Graphics。FFmpeg8.1.2 CLIで前景3840x1536動画の先頭10秒/300framesを320x128に縮小してnullへ出力。実アプリのfps測定ではない。
- software: wall0.369s、user3.297s+system1.438s、maxRSS362000KiB。
- D3D11VA default: wall0.797s、user1.344s+system1.547s、maxRSS710268KiB。decoder pix_fmt:d3d11をログ確認。
- NVIDIA指定: wall0.775s、user1.469s+system1.594s、maxRSS703928KiB。adapter0選択、decode errors0。
- 単回予備測定ではCPU時間軽減、壁時計とRSS悪化。転送と初期化を含むためGPU導入だけで高速化したとは言えない。実libav経路での継続/seekと診断を次に測る。
- logs: target/qa/snapshot-cleanup-20260905/{software-decode-preflight,d3d11va-preflight,d3d11va-nvidia-preflight}.log。FFmpeg公式device選択契約: https://www.ffmpeg.org/ffmpeg.html 。
- 共有調査: 現VideoFrameはVec<u8>、GpuSurfacePresenterごとにdeviceを生成。DOM imgへそのままtextureを渡すAPIはない。native child surface案はDPI/clip/overlay/lifecycle制御が別途必要。共有GPU context/textureとproducer統合はまだ未実装。

### GPU実機検証・追加UI要望

- libav focused: 15passed/1ignored、warning0。初回Display未実装E0599修正、次の旧16x16CPUoracleがPreferred production GPUを使った失敗は明示test-only software constructorで修正（製品はGPUのまま）。
- 前景/背景の実動画GPUテスト双方PASS。D3D11VAログでNVIDIA RTX5090、43hardwareframes、hardwareerrors0、seek2回、samepositionreuse、CPUoracleとのPTS/画素差、release/reopen/dropを確認。elapsedはCPUoracle混在で性能比較には使わない。
- MSVC wrapper越しのUnicode環境変数pathでIllegal byte sequenceが出たため、元ファイルを変更せずtarget/qa/snapshot-cleanup-20260905/gpu-{foreground,background}-fixture.mp4へコピーし実行。これはテスト環境のpath経路で、製品Unicodepathの成立は別途確認対象。
- invalidadapter2147483647実テストは現状FAIL: FFmpegが指定不存在でもdefaultdeviceを作る挙動を検出。独立レビューで静的に発見できなかったため実証未完として扱い、DXGI存在確認を実装中。rootがこの失敗を再reviewへ通知。
- ユーザーは停止中にも『出力映像を待っています』ちらつき報告。BUSYで直前成功画像を即破棄していた経路を修正。同一project/outputの成功frameは最大1秒保持、独立expiryでhungIPCでも破棄+error、初回BUSYは待機。projectEpoch変更、出力変更、backenderrorは即破棄。Program encode後epoch不一致はBUSYではなくErrにし保持不可。dispose期限timer回帰修正済み。実controllerchecker PASS、独立review問題なし。browser1920/1280比率/unmount PASS。
- DATEの個別照明はchild timelineに格納、ユーザーはダブルクリックで表示を確認。続けて親lane矢印でinline展開を要望。担当がreadonly child lane projectionを実装中。App/panel cue list配線はroot、Overview/projection/checkerは担当所有。子編集は既存ダブルクリック画面で、表示位置が未証明のloop/conform等は説明導線を出す方針。
- Unicodepathの追試: localGPU test wrapperでvcvars由来env再構築後に元Unicodepath envを復元し、元Downloads前景の直接decode PASS。製品libav Unicodepathが原因ではなくテストwrapperの環境変数経路に限定と切り分け。log gpu-unicode-path-real.log。
- DXGI事前adapter存在確認を追加後、invalidindex2147483647のPreferred実テストPASS、CLI fallback0。windows0.61.3既存versionをvideo Windows optional libav dependencyへ追加、Cargo.lockは依存辺1行のみ。独立再review問題なし。
- 追加HEVC640x360/4s合成fixtureでもRTX5090 GPU decode/CPUparity/seek/reuse/release PASS、74hardwareframes、error0。log gpu-hevc-real.log。
- Tauri native monitor Rust tests5passed、warning0。log gpu-monitor-native-tests.log。GPUなしfeature構成のfocused testを検証中。
- 照明展開の独立reviewで2件修正: viewport絞込み済み/毎playheadtick変化するeventsをprojection入力から除去し、Appに全authored親のscalar等値memoを追加。さらにoverlap rail-count Mapへ等値判定を追加し、同じ高さなのにchildDOMを毎tick作り直す通知を抑制。
- readonly child projectionはpositive source offset/parent trim/overlap/repeated parentsを確認。parent tempo/rate/repeat、childconform、negative offsetの未証明mappingは日本語drill-in説明にし偽の時刻を出さない。純checker PASS、component browser1280/640 PASS（fullApp arrowの実native操作とは別）。
- GPUなし libav disabled構成 focused4passed、warning0。TypeScript PASS。native build開始時このcheckout PID76516のみ停止・exactMSVC確認。
- 最終inline独立review PASS。automationLayerIdsにもsameNumberSetを追加し、overlap件数/automation layer集合が同じならsectionLayoutへtick更新が流れない。既存overlap計算自体のcostは本単位では変更なし。
- 実Nightgirl AV-test.sdcをproductionprojectionへ渡す追加read-only検証: 6rows/44blocks/0issues。local checker `target/qa/snapshot-cleanup-20260905/check-nightgirl-inline.mjs`。
- 最初のnative release build PASS3m48s、warning0。ただしbuild中の最終frontend memo修正が後着のため、この成果物を最終版とせず `pnpm --dir app tauri build --no-bundle` 再実行中。final log `gpu-inline-native-final.log`。
- 現在のbranch/HEAD/upstreamはcodex/syndocal-v1.2 / 801fbf5e47710f5ac3703d4099171832996e6f7bで等しい。保護viewportchecker SHA256 3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063維持。ユーザーnative操作受入前のためcommit/pushは未実施。

### GPU / BUSYちらつき / 親照明展開 最終native起動

- `pnpm --dir app tauri build --no-bundle` 最終exit0。frontend8.06s、release2m23s、first-party compiler/bundler warnings0（baseline0→current0、delta0）。exactMSVC pin/where-first確認済み。
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File target/qa/snapshot-cleanup-20260905/launch-gpu-checkout.ps1` exit0。このcheckoutのみ起動、PID87468、Syndocal main window1件 responsive=True/maximized=True。runtime helper windowsは別。アプリ内UI操作は実施していない。
- exe SHA256 `7A67936A72852A155EA8AA7578454F41D3049F74C0967C3CCF10524021CCE0C9`。証拠 `target/qa/snapshot-cleanup-20260905/gpu-inline-launch.json`。
- ユーザー再確認: 親Lighting DATE行左矢印→6子lane表示、ダブルクリックで子編集継続。停止/再生中のpreviewちらつき改善、映像の滑らかさと音声同期。
- GPU decode実機証拠は前景・背景・生成HEVCのGPU AVFrame＋CPUparity/seek/reuse/teardown、invalidGPU外CLI0。最終アプリの再生を自動操作してのGPU負荷/fps測定は未実施。
- GPU化の残件: 現在はGPU decode→CPU readback/scale/composition/JPEG→WebView。出力とpreviewの共有producer、共有GPU context/texture、JPEGを使わない表示は未実装。現native presenterごとdevice/Vec<u8>frameの所有構成を整理し、DPI/overlay/lifecycleを保てる表示方式を次単位として実装・計測する。今回の成果をzero-copy/全GPU描画完了と扱わない。
- 追加所有: Cargo.lock、video Cargo.toml/lib.rs/hap_decoder.rs/libav_decoder.rs/libav_hardware.rs、App.tsx/types.ts、TimelineOverview/TimelineCueEventsPanel/TimelineOutputPreview、TimelineInlineChildRows.tsx/css、timelineInlineChildProjection.ts、output monitor controller/checker、inline child checker/browser、previewbrowser fixture。前段所有dirtyと合わせ保持、保護viewportcheckerは対象外。native機能受入未確認なのでcommit/push未実施。

### 2026-09-05 右下preview / navigation / Undo / 子scene色の追加修正（native反映前）

- ユーザーの重い箇所は右下映像preview。共有providerがrequested output以外のdecoderをretain対象外として破棄し、前景/背景交互requestでopenし直す原因を修正。さらにshared rendererのprefetch 2→0へ変更、停止時や次requestが先読み位置より戻る再seekを解消。output/preview共有producer・CPUreadback/JPEG廃止は未実装。
- `SYNDOCAL_GPU_TEST_VIDEO`=元のUnicode前景path、`SYNDOCAL_D3D11VA_ADAPTER=0`、`node target/qa/snapshot-cleanup-20260905/run-gpu-native.mjs cargo test -p video --locked --features libav output_preview_sessions -- --include-ignored --nocapture --test-threads=1` PASS3。RTX5090実decode: sessions2/opens2/resets0、hardwareframes14/errors0、reuse90。ログpreview-session-retention-tests.log。削除layer解放・他output invalid transition隔離も検証。GPU testはnativeアプリfps計測ではない。
- 親展開child blocksへ既存cueIdentityCssのcue/group色を適用。check-timeline-inline-child-browser.mjs PASS1280/640、computed fill/stroke/text、live色変更、旧clip/overlap/read-only/drill-in検証。ユーザーは矢印展開自体の動作確認済み。
- workspace/navigation controller新設。History APIでBack/Forward、projectepoch隔離、編集中cancel、partial lock、child既存判定。pendingpopstate/branch/epoch競合回帰PASS。物理マウスXbuttonのWebView2内挙動は未確認。
- CtrlZ/Yは文字編集欄ならnative編集Undoを維持、range/select/checkbox等はprojecthistoryへ。check-project-history-keyboard.mjs、Space keyboard checker、shortcut15552matrix PASS。
- project Undo/Redo前にpending mappings flush＋最新history refresh、epoch fence、singleflightを追加。dirty mappingsの全同期代入をreactive signalへ集約し、最初の未送信編集からUndo enable。check-project-history-preflight.mjs PASS（cancel/busy/flush失敗/stale/currentCAS/redo invalidation/reactivity）。
- 手動set_bpmをruntimeからrenderer-ticketed projectmutationへ移動。admission histogram132ticketed/156runtime。Tap/external syncはruntimeのまま、全操作Undo保証ではない。
- manual_bpm新real Engine/native history回帰で既存複数段Undoバグを発見（初回Undo後、次entryのsource revisionが古い）。main navigationでpop/publication前の隣接epoch/hash整合性検査＋成功後のみ隣接sourceをactual checkpointへ更新。current/top厳密CAS維持。独立review ACCEPT。最初testは1pass1fail（manual-bpm-history-tests.log）、修正後再実行中。反復Undo2/Redo2を2周、壊れた隣接Undo/Redoを無変更で拒否するtestを追加。
- 現時点TypeScript、project-transaction checker、backend-operator contract、historypreflight、navigation、keyboard PASS。既存check-vj-media-import-access全体の退役tab/Eshortcut assertion残件は未解決、全suite成功とはしない。
- branch/base/upstream codex/syndocal-v1.2 / 801fbf5e47710f5ac3703d4099171832996e6f7b。保護check-viewport-containment SHA256維持。追加所有: createWorkspaceNavigationController/checker、history keyboard/preflightchecker、hotkeyHelpers/appShortcutActions/createAppKeyboardController、bpm_history_tests.rs、authored_control_plane tests、control_plane admission、output_preview_session_tests.rs。前段所有を保持。native機能受入待ちで未commit/push。
- 修正後 `manual_bpm` real native tests2PASS（53.16s compile、warning0）。`tauri_route_admission_table_is_exact_and_fail_closed`1PASS。runtime fence checkerは旧固定count157を検出したため、set_bpm移動に合わせ156へ更新（分類自体は正しい）。その再実行＋既存history_navigation回帰を実施中。
- 最終frontend追検証: TypeScript、history preflight/navigation/keyboard、media-asset-authority、project-transaction-recovery6scenarios PASS。git diff --check成功（Gitの改行正規化通知はcompilerwarning数と別）。
- 再実行runtime fence1PASS、既存history_navigation1PASS、first-party native test warnings0（baseline0/current0/delta0）。新nativebuild開始前にexact checkout PID87468のみ停止、MSVC14.44.35207 exact pin/where-first確認。frontendbuild7.99s PASS、nativebuild実行中。すべての現変更はnative機能確認待ち。

### 右下preview / navigation / Undo 最終native成果物

- `pnpm --dir app tauri build --no-bundle` exit0、release3m16s、frontend7.99s。compiler/bundler first-party warnings baseline0/current0/delta0。log `target/qa/snapshot-cleanup-20260905/preview-navigation-native.log`。
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File target/qa/snapshot-cleanup-20260905/launch-preview-navigation.ps1` exit0。exact exe起動PID62540、Syndocal main window1件 responsive/maximized=true。SHA256 `81393C1F4BE2B6A539315C1C58CD8D6C1F19323F0EEB056FB047F7A17A5A9B55`。`preview-navigation-launch.json`。アプリ内の自動操作なし。
- 次の安全な確認: ユーザー操作で同じAV projectの前景/背景preview再生・停止の体感改善、展開childscene色、マウスBack/Forward、複数編集CtrlZ/Redoを確認。実アプリfps、音声同期、物理マウス挙動は未測定／未確認。全操作Undo／最適化完了とはしない。native受入待ちなので所有dirtyを保持、未commit/push。
- 残件: preview/output共有producer、GPUtexture/CPUreadback/JPEG経路削減、同一input複数サイズのsession所有、App transaction gateway等の大きなファイル分割、既存VJ media access checker残り。Tap/externalclockはruntime動作で今回手動BPM Undo契約の範囲外。新作業へ広げず現単位のnative受入を先に確認する。

### 追加依頼: preview最適化完了とTIMELINE内一覧

- ユーザー「完了してくれ」を受け、右下previewの実処理削減を継続。更にユーザーはTIMELINE上段右側の開閉可能な一覧から統合/rootとSuper Scene childを選べるUIを依頼。両方を本単位に含める。
- Timeline monitor IPCはversion2、byte7にencoding（JPEG0/RGBA1）、40byteheaderの後に明示payload。TimelineはRGBA要求→常設canvas、JPEGencode/decode/BlobURL生成廃止。既存VJ monitorはJPEG契約でv2を使用、RGBA応答は拒否。v1packetは拒否、永続schemaには変更なし。
- packet/encodingのpuremoduleをlive_video_monitor_packet.rsへ分割。Programmonitorはvideo_monitor_snapshot.rsでBPM/video/clipruntime/transitionruntimeのみ単一inspect_snapshotから取得、照明・cuebank・timelinebank全体のcloneを除去。epoch-before-snapshot/lateepochrejectを維持、poisonはErr。重複warmupdecodeを削除しactualrender1回で直接取得。
- libav seekは最新candidateAVFrame最大1つを保持、採用frame/EOFcandidateのみHWreadback/scale。原前景ファイルを使用したeager旧実装oracleとの全RGBA/PTS完全一致3round PASS、114→6transfers。例round2 908.85ms→448.60ms（debug、sessioncreation除外）、中間frameの不要変換をなくす境界。logseek-projection-gpu.log、tests2PASS、独立decoderreviewACCEPT。
- `node target/qa/snapshot-cleanup-20260905/run-two-video-native.mjs cargo test -p syndocal --release --locked live_video_monitor_benchmark_raw_vs_jpeg_two_real_outputs -- --include-ignored --nocapture --test-threads=1` PASS1。元Unicode前景/背景、RTX5090adapter0。releasecompile2m39s。120sequentialframes: renderp50=4.270ms/p95=5.005ms、rawpacketp50=0.0067ms/p95=0.0303ms、JPEGencodep50=0.3538ms/p95=0.3905ms。20pausedframes renderp95=0.1779ms、HW追加decode0。steadyopens2/resets0/errors0。seek6framesrenderp95=159.73ms（意図的seekのsessionreset含む）。snapshotcapture/IPC/WebView/nativephysicaloutputは計測外。rawpayloadは320x128x4、60requests/sで約9.8MB/s。
- releasebench前にexactcheckoutのPID62540のみ停止、MSVCexactpin/where-firstを各Cargogateで確認。現在appは停止中、新UI/検証完了後native build/launch予定。
- Rustlive_video_monitor6PASS＋ignoredbenchmark1（benchmark別実行PASS）。frontpacket/controller/actualcanvas browser1920x1080/1280x720・TypeScript PASS。実画素/constantcanvas/BUSYexpiry/staleepoch/VJJPEGonlyを検証。snapshot/route独立reviewACCEPT。nativecompilerwarnings0。
- Nativeoutputとの単純framecache共有はmappingの適用位置とFollowの合成契約が異なるため採用しない。本previewは既存finaloutputrendererで正しいmapping/blackoutを維持。最終selectedframeのCPUreadbackとcanvas転送は設計上残る。zero-copyGPU完了とはしない。
- 現所有追加: packet/snapshot/benchmark各Rustmodule、libav_seek_projection_tests.rs、liveVideoMonitorPacket.ts/createLiveVideoMonitorController.ts、timelinecanvas関連とcheckers、TimelineNavigator/panel/新checker、Apptoolbar/navigationadapters。保護viewportcheckerは引き続き対象外。

### タイムライン一覧の実装・検証

- TIMELINE上段headerに「タイムライン一覧」toggle（aria-expanded/controls）。初期表示はopen。上段右sidebar内で「統合タイムライン」「シーン内タイムライン」を区別し、現在選択aria-current、縦scroll、閉じる操作を追加。下段の照明/ソース/映像previewサイズには作用しない。
- root選択は既存のauthoritative select_timeline(play:false)へ接続し、成功した同project/同childcontextのみ子編集を解除。同rootを開く場合はbackendmutationなし。childは既存限定で開き、古い一覧から新規SuperScene生成しない。draftcancel/commitfailure/staleepoch/contextchange/concurrencyを実App関数抽出checkerでPASS。
- TimelineNavigatorへ表示・選択UIを分離、metadata-onlyMapとprimitiveID列で100回snapshot更新後もDOM/focus維持、labelはlive更新。44pxの新buttonhitheight、既存typography/controls不変。
- `node app/scripts/check-timeline-navigator-browser.mjs` PASS（component＋実Appfixture1280/640、currentroot/child/scroll/pendingdisable/collapse/unmount/focus）。screenshots `target/qa/timeline-navigator-20260905/integrated-1280.png`, integrated-640.png。rootも1280画像確認済み。
- `node app/scripts/check-timeline-navigator-actions.mjs` PASS、最終TypeScript PASS、projecttransaction/backendoperator checks PASS。stalecheck-vj-media-import-accessは退役E/Llocaltabsから現3tab/下分類/actuallibraryscope検証へ追従し全PASS（authority/cache等のassert保持）。
- RAWfront/backend独立review2名ACCEPT、Navigator/Appadapters独立reviewACCEPT。snapshot projectionRust1PASS。全nativecompilerwarning現在0。新nativebuild実行中、lograw-navigator-native.log。所有branch/baseは既述のまま、保護viewportchecker SHA256維持。

## 最終checkpoint: 映像preview負荷対策とTIMELINE一覧

- 実装、独立review、適用範囲の自動検証、Windows native build/launchを完了。`pnpm --dir app tauri build --no-bundle` exit0、frontend7.67s、release3m05s。first-party compiler/bundler warnings baseline0/current0/delta0。ログ `target/qa/snapshot-cleanup-20260905/raw-navigator-native.log`。
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File target/qa/snapshot-cleanup-20260905/launch-raw-navigator.ps1` exit0。このcheckoutのexe PID4388、main Syndocal window1件、responsive/maximized=true。SHA256 `A5A9BD1387CF2F5941748180D6C6C78E4F4C7F01B70E3764683B6BA97E68F83B`。証拠raw-navigator-launch.json。
- 現在のUI入口: EDIT→TIMELINE、上段右「タイムライン一覧」。初期open、同名headerbuttonで再表示。統合/rootとSuper Scene childを一覧から再生なしで開ける。
- これはコード／自動テスト／native起動の完了checkpoint。実際のSyndocal画面を操作しての滑らかさ・音声同期・物理マウスBack/Forward・全編集操作Undoの受入は未確認。Rust releasebenchmark値を実アプリend-to-endFPSとは扱わない。デバイス/ASIO/physicalDMX/本番show acceptanceも本単位外。
- 累積所有変更72filesをcommit/push対象とし、保護app/scripts/check-viewport-containment.mjsはstage対象外、SHA256 `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`維持。branch/base `codex/syndocal-v1.2` / `801fbf5e47710f5ac3703d4099171832996e6f7b`、この文書のcommitがcheckpointを識別する。rawqa evidenceはtarget/qaに保持し削除なし。
- 将来の設計課題は全GPUtexture表示/nativeoutput共通producerとApp transactiongateway等の追加分割。今回の右下previewは採用frameのみCPUreadbackし、boundedRGBAをcanvasへ渡す方式で完了。上記の追加設計を実装済みとはしない。VJmediaimport stalechecker残件は解消済み。
