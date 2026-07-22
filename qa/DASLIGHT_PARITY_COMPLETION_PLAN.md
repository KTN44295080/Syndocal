# Syndocal Lighting Product Completion Plan

- 作成日: 2026-07-23
- 基準線: `adcb402` (`codex/syndocal-v1.0`)
- 目標: SyndocalをDaslight 5と同一タスクで比較可能な照明ソフトウェアとして完成させる
- 3D方針: **内蔵3Dビジュアライザーは実装対象外**。Art-Netを外部ビジュアライザーへ送信して確認する
- 規律: 内部実装量を競合同等性へ読み替えない。操作手数、ネイティブ実データ、実出力証拠が無い軸は未計測のまま扱う

## 1. 完走の定義

次をすべて満たした時点をソフトウェア完走とする。

1. 主要な照明作業を同一タスクで計測し、重大な操作手数劣位を残さない。
2. Colour / Chaser / Move / Value / Curve / Mappings / Colour Mappingsを、表示上の別名ではなく独立した保存body、validation、runtime、editorとして提供する。
3. Cue所有FXのパラメータ遷移と、Scene Liveのspeed / size / phase / direction / segment / flash / strobe / soloを演奏中に直接操作できる。
4. GDTF Shareを単なる検索・download経路で終わらせず、favorites、offline cache、profile health、missing-profile repair、common-rig packを備えた灯体導入面にする。
5. Stage / Timeline限定ではない名前付きdetachable workspaceと、full / partial operator lockを備える。
6. `.sdc v1`互換、Undo/Redo、project validation、44 Hz予算、JA/EN、viewport、native WebView2を各変更で維持する。
7. 実ショーを読み込み、Art-Net外部ビジュアライザー経路でCue、FX、Scene Live、timelineを確認する。
8. 最終競合監査を現HEADでやり直し、見た目・操作性・見やすさ・機能面を証拠付きで更新する。

対応ハードへのstandalone scene書込みはハードウェア製品開発を要するため、本リポジトリだけで実装済みと扱わない。PCソフト製品として対象外にするか、対応ハード製品を別途作るかを最終判定に明記する。未保有DMX node、灯体、controller、会場による受入も外部依存ゲートとして残し、software PASSへ混ぜない。

## 2. 現HEADで閉じている差

- Scene Matrix主体のControl、統一workspace shell、内部スクロールと可変splitter
- source-linked Scene Block、layer、loop/jump、Super Scene child timeline、tempo conform
- independent Colour / Chaser / Move / Value / Curve runtimeとeditor
- cue-owned FX params、selective Effect Recall、Effects Only
- editable project-saved Touch surfaceとLAN remote反映
- Scene Live speed / size / phase latch、flash momentary
- GDTF import/cache、GDTF Share検索/download、custom fixture builder
- Stage / Timeline popout
- Art-Net / sACN / serial DMX、RDM、MIDI/OSC/WebSocket、active/standby基盤

## 3. 現HEADで残る差

### P0 操作性と証拠

- T18差分でSyndocal側を13タスクへ拡張したが、Daslight同一タスク実測は3件のみ。
- patch、static programming、FX target適用、native show保存/再開はoperation-count未計測。
- current-HEAD gap matrixは作成済みだが、4軸の最終再判定はT19-T23完了後まで行えない。

### P1 FX

- Mappings / Colour MappingsはPositionWave/Colorのrecipe分類で、fixture-order/image-drivenの独立runtimeではない。
- Cue所有paramsはactivation時のcopyであり、Cue間parameter morph/fadeは無い。
- calibrated Amber/Lime/UV/multi-emitter colour outputは未対応。

### P2 Scene Live

- T17はspeed / size / phase / flashまで。direction / segment / strobe / soloが無い。
- DVC import済みSceneを含む実ショーで、全Live modifierと外部Art-Net可視化を通した証拠が無い。

### P3 灯体導入

- GDTF Shareのfaceted search、favorites、offline状態、health表示、missing-profile repair、検証済みcommon-rig packが無い。
- 大規模商用catalogそのものの件数では競合しないため、open GDTF経路の品質と復旧性で評価する。

### P4 運用面

- popoutはStage / Timelineだけで、任意のprogramming/live workspaceを名前付き構成として保存できない。
- operator password lock、部分lock、project policyが無い。
- 128 universeのsoftware tick/UIは通るが、実node一時間soakは外部未完。

## 4. 実施トランシェ

### T18: Operability Baseline

1. operation-count harnessを現行UIへ修復する。
2. Scene MatrixからTimelineへの配置を1 dragで証明する。
3. 最低12タスクへ計測を拡張し、Daslight実測が無い項目は`未計測`として別欄にする。
4. current-HEAD gap matrixを生成し、旧監査の閉じた項目を更新する。

受入: harness fail-closed、対象結果のDOM/engine反映、操作予算回帰、1920x1080 primary + compact containment、JA/EN。

進捗（2026-07-23）: 現行UIへ修復し、13タスクが実CDP gestureと結果反映を含めてPASS。Daslight実測のある3タスクは同数、残る10タスクは`未計測`のまま分離した。current-HEAD監査は`qa/DASLIGHT_CURRENT_HEAD_GAP_MATRIX.md`へ固定。patch / static programming / FX target適用 / native Save-reopenの操作計測は次のT18追加対象として残る。

### T19: Independent FX Completion

- T19-A: Curve protocol/runtime/editor/preset/Cue params
- T19-B: fixture-order Mapping protocol/runtime/editor
- T19-C: Colour Mapping protocol/runtime/editor
- T19-D: Cue parameter morph/fade
- T19-E: calibrated multi-emitter policy（profile metadataが無い場合は推測せずfail-closed）

受入: additive serde defaults、legacy `.sdc` byte-shape、project/preset round-trip、200 fixtures x 64 effects regression、release 44 Hz mixed benchmark、physical acceptance分離。

進捗（2026-07-23）: **T19-A完了**。`EffectKind::Curve`、独立`CurveEffectRequest`、2〜32点のCubic Hermite channel function（各点のin/out tangent）、command-time compile、独立runtime/editor/preset/Cue-owned paramsを追加した。旧project/presetは`curve`を出力せずbyte-shapeを維持。protocol 36、engine 378、Tauri 322、5解像度FX viewport、2635/2635 localization、frontend buildがgreen。Curveを含む64 FX×200灯体release hot-pathはp95 2.778ms / p99 3.400ms / max 4.056ms。T19-B〜Eは未完。

### T20: Complete Scene Live

direction / segment / strobe / soloをT17と同じcommand-time rebuild方式で追加する。44 Hz tickでmap lookup、allocation、parameter branchingを増やさない。release/retrigger/load resetと非永続latchを維持する。

### T21: Fixture Onboarding

GDTF Share catalogをfaceted search、favorites、offline cache/health、missing repair、verified packへ拡張する。認証情報はprojectへ保存しない。

### T22: Workspace and Operator Policy

名前付きpopout workspace、任意主要pane、monitor placement復元、full/partial operator lock、credential-safe policyを追加する。

### T23: Final Acceptance

全Rust/frontend/viewport/native gates、Shinkan実データ、Art-Net外部ビジュアライザー、操作手数、release performanceを実行する。`qa/DASLIGHT_COMPARISON_VERDICT.md`、`qa/LIGHTING_COMPETITIVE_AUDIT.md`、`qa/GOAL_COMPLETION_AUDIT.md`を現HEAD証拠で更新する。

## 5. 共通停止条件

- legacy `.sdc`が同じ入力で意味変更または不要な差分を生む
- 44 Hz hot pathへ未計測の検索、確保、clone、lockを追加する
- browser fixtureだけでnative/physical完了を主張する
- Daslight実測の無いタスクを優位として数える
- 外部Art-Netビジュアライザー方針に反して内蔵3Dへ工数を使う
