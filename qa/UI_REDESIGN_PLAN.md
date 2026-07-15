# UI Competitive Redesign Plan (Timeline-first)

作成: 2026-07-15 / 計画: Fable（実機比較・多段Workflow分析・敵対検証込み）/ 実装: Opus委任
状態: **承認済み（2026-07-15）** — 実施順: T1 → T2 → **T8（前倒し承認）** → T3 → T4 → T5 → T6 → T7
承認内容: (1) 計画全体 (2) T8をT2直後へ前倒し（Control構造契約の変更を承認） (3) T7のprotocol変更
（frontend-only制約の当該範囲での緩和を承認） (4) T3のページ式フォーム行編集置換 (5) Opus委任即時開始

## 目標

見やすさ・操作手数を軸にUIを再設計する。照明はDaslight 5（v5.0.6.2）を、VJはSynapseRackを上回る。
QLC+はユーザー指示によりUIベンチマークから除外（機能チェックリスト用途のみ）。

## 証拠ベース

- `target/qa/ui-comparison/daslight5-current.png` — 実機Daslight 5 + ユーザー実ショー（Shinkan2026.dvc）、EDIT+TIMELINE、1936x1048ライブキャプチャ
- `target/qa/ui-comparison/syndocal/*.png` — Syndocal全ワークスペース正規スクリーンショット43枚（1920x1080/1032）
- `target/qa/ui-comparison/PRIMARY_OBSERVATIONS.md` — 一次観察（Workflowエージェントが裏取り済み）
- 分析Workflow: Inventory×5 / Design×3独立案 / Judge×3（勝者: INFORMATION-ARCHITECTURE-FIRST 2/3票）
- 要所主張はメインエージェントが一次検証済み: `TimelineOverview.tsx:178`（44単位viewBox）、
  `numericHelpers.ts:69`（hsvToRgb）、`VideoPlaybackTimeline.tsx:212`（--cue-color前例）、
  `protocol/lib.rs:1171`（CueSummary）+ `Option<String>` color前例（:526/:2615）、
  `clockDisplay.ts` + `check:clock-display`ゲート、コミット`7ab0795`（fullscreen VJ focus前例）

## Daslight 5から移すべき可読性装置（キャプチャで実証済み）

1. グループごとの**色相アイデンティティ**がシーンマトリクス・タイムラインブロック・タイトル文字を貫通する（最重要の相互参照装置）
2. タイムラインブロックは**名前+デュレーション刻印つきの色ブロック**（例: "Par-OrangeStrobe (2.78x)" / "02m00s20"）
3. トラックガター: 番号 + eye（表示）+ lock + 展開矢印
4. ブロック選択→固定**BLOCK PROPERTIES**インスペクタ（Start/Duration/Fade In/Fade Out/Conform to tempo/Loop）
5. タイムラインが画面の約半分を占有し、フォームではなく直接操作で編集する

## Syndocal現状の主要欠陥（Inventory実証）

- 色相アイデンティティゼロ: Cue Pad一律グラファイト、500ブロックが全て同一レーン色
- 描画タイムラインは画面の約5%、そのフォームエディタが約15%（逆転している）
- ブロックは幅7%未満でラベル非表示、デュレーションはホバーのみ、msの生数値入力
- 固定インスペクタなし: 編集は12件/ページのフォームリスト（500ブロックで42ページ）
- ブロックはリサイズ不可・ドラッグ中スナップなし・ホイールズームなし・キャンバス上配置不可
- GOボタンがモードで位置移動、ブラックアウト3種がBackと同一外観、フェード表示が2色2様式

## トランシェ（優先順）

### T1: 色相アイデンティティ基盤（risk: low / M）
`app/src/identityColor.ts` 新設: cue_id / groupパスの安定ハッシュ→色相（`hsvToRgb`利用）。
`--identity` CSS変数のみで配布（`--cue-color`前例に従う）。パレットガード: GOアクセント橙±20°を除外、隣接ID間≥30°分離、決定論チェックスクリプト付き。
消費者: Live/Touch Cue Pad（4px左色バー+色相タイトル）、TimelineOverviewブロック/マーカー、Scene Blocks行、CueManagement行、グループチップ。
受入: 同一IDが全面で同一色相 / 500ブロックfixtureで1レーン≥8色相 / ライブDMX由来の灯体色パスはバイト同一（identityは出力色に関与しない）/ 全ゲートpass。

### T2: ブロック解剖学 + ピクセル空間キャンバス（risk: medium / L）
TimelineOverviewを**ピクセル空間SVG**へ変換（ResizeObserver実寸viewBox — Value editor `12bd25a` と同型）。
ブロック=2バンド構造: 上=identity色相+名前、下=暗色相+デュレーション刻印。幅による段階的縮退（名前+刻印→名前→省略→色スリバー）。ブロック高8→約16/44単位相当。
固定左ガター（HTML列、キャンバス外）: レーン名+件数+**レーンeye切替**。キャンバス内ラベルプレート削除（名前衝突解消）。
時間表記は`clockDisplay.ts`経由に統一（新モジュール不要）: M:SS.ff。キャンバス内の生"ms"文字列ゼロ。
フェードウェッジ、プレイヘッド/ポイントマーカーの形状差別化。
契約変更: check-timeline-viewport / check-scene-block系の期待値更新（テスト契約のみ、デスク構造不変）。
（注: 現ルーラーは既にm:ss表記。生msなのはフォーム入力と刻印側 — 誇張しない）

### T3: キャンバス優位分割 + 固定Block Propertiesインスペクタ（risk: medium / L）
`.timelineShowSurface`の縦配分を反転（キャンバス minmax(220px,1.45fr) / エディタ 0.55fr）。
エディタ領域=左右2ペイン固定: 左=Block Finder（仮想化1行リスト: 色相チップ+名前+開始刻印+レーン、テキストフィルタ、クリック=選択+キャンバスreveal。旧ページャリストは'List'トグルで残置）/ 右=Block Propertiesインスペクタ常設（Start/Duration/Loops/Lane/After、Fade In/OutはCue所有のためread-only+'Edit Source'ジャンプ）。
**Enter/blurコミット、Saveボタン廃止**、draft-dirtyチップ表示。時間入力はM:SS.ff⇔生ms両受理。
契約変更: ページ式フォーム行編集の置換（ユーザー承認必要）。

### T4: 直接操作（risk: medium / L）
ブロック端リサイズハンドル（automation-range前例 `TimelineOverview.tsx:336-476` 再利用）、ドラッグ中ゴースト+ライブ刻印、Escキャンセル。
Magnetトグル: ドラッグ中スナップ。カーソル中心ホイールズーム（preventDefault + document scroll 0のハーネス断言）+背景ドラッグパン（7ボタンツールバーはフォールバック維持）。
配置: Cue PadまたはFinder行で「アーム」→レーンダブルクリックで自然デュレーション配置、**スイープドラッグで開始+デュレーション同時指定**。
受入: デュレーション変更=ドラッグ1回 / 配置=ダブルクリック1回 / ズーム中カーソル下時刻静止±2px / 全ゲートpass。

### T5: Control一貫性（risk: medium / M）
TransportCluster共通化: GOが全モードで同一ジオメトリ（±4px）。ブラックアウト3種を**KILLゾーン**（危険トークン: 暗赤枠+ハッチ塗り+destructiveActions連携、橙アクセント不使用）へ分離。
FadeProgress単一化（1色1様式）。ステータス階層化: Active/Next Cueを大セル+identity色相チップ。Live Edit死領域解消（Cue Padグリッドが高さを充填）。
**メタデータ最小フォント11px床 + CSS監査**。
契約変更: モード別トランスポート配置のハーネス断言更新。

### T6: VJ Desk再構成（risk: medium / L）
クリップバンク支配的（サムネイル第一のグリッド、12パッド無スクロール表示）。Preview/Programモニタが中央列高さ≥70%。Audio/AutoVJ/Reactive設定はアコーディオンドロワー（既定折りたたみ、localStorage永続）。レイヤー6行常設表示。Out系ボタンのスコープ語彙統一。
SynapseRack式フローティングパネルは一画面契約維持のため不採用（採用するなら別途契約再交渉）。

### T7:（オプション・要明示承認）永続identity色 — protocol変更
`CueSummary.color: Option<String>` + `EngineSnapshot.group_colors`（serde default、`.sdc` v1互換維持、前例: StageObjectSummary.color / VideoCuePointSummary.color）。set-cue-color / set-group-colorコマンド、カラーピッカーUI。永続色優先・ハッシュfallback。
**「frontend-only」制約の明示的緩和が必要。**

### T8:（契約変更・要明示承認）Timeline Focusレイアウト
VJ fullscreen focus（コミット`7ab0795`）と同型の、全幅Showサーフェス+2Dステージ折りたたみ+Esc復帰。
Control構造契約（上段Cue/Transport+左下Stage+右下タブ）の再交渉。

## 実装体制

- 実装: **Opus** サブエージェント（`Agent(model:"opus")`）、1トランシェ=1検証スライス=1コミット
- 検証: Fable側で各スライスのゲート再実行（build / viewport / localization / 対象ハーネス）+ ライブブラウザ目視 + Daslightキャプチャとの並置比較
- 全スライス共通ゲート: `tsc --noEmit` / `vite build`（index<500kB）/ `check:viewport`全マトリクス / `check:localization` 100% / `git diff --check`
- 各トランシェ完了時に`RELEASE_STATUS.md`と本書へ実測値で追記（過大評価なし: 「Daslight超え」は同一タスク操作手数計測とキャプチャ並置の証拠が揃うまで主張しない）

## 承認履歴

2026-07-15 ユーザー承認済み: T3契約変更 / T7 protocol変更 / T8契約変更+T2直後への前倒し / Opus委任即時開始。
未解決の承認事項なし。
