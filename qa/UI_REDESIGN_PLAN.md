# UI Competitive Redesign Plan (Timeline-first)

作成: 2026-07-15 / 計画: Fable（実機比較・多段Workflow分析・敵対検証込み）/ 実装: Opus委任
状態: **承認済み（2026-07-15）** — 実施順:
T1✅ → T2✅ → T9✅ → T10✅ → T8✅ → F1✅ → F3✅ → F2✅ → F4✅ → T4✅（86ca79e）→ F5✅（243aaea）+ T3✅（f5cc523、Fable直接実装・合議マージf64ef96）→ F7✅（95b33b0、音声クリップブロック+可聴再生）+ T5✅（90dfa1a、Fable直接実装・合議マージ8c364a0）→ F6✅（59b6600、super scene）+ T6✅（4182ad6、Fable直接実装・合議マージf344713）→ F8✅（6a863a1、マルチステップStaticシーン）+ T7✅（60f1ad1、Fable直接実装 protocol込み・3ラウンド合議マージ7ee9214）→ T11✅（5c3de86、編集可能Touchサーフェス）+ T12✅（0010443、Fable直接実装・合議マージd95de64）— **シリーズ全完了 2026-07-18**
実装体制（2026-07-16更新）: 実装=Codex gpt-5.6-sol（ローカルCLI、ユーザー指定）/ 計画・検証・コミット=Fable。

**2026-07-19承認済み継続計画:** T15=タイムライン減量、T16=FXエディタ可視化、T17=Scene Liveモディファイア、および先行するPATCH 512連続表示・Control可変ペイン基盤は [`qa/UI_REDESIGN_T15_T17_PLAN.md`](UI_REDESIGN_T15_T17_PLAN.md) を権威パケットとする。
T9はOpus（Mapping再設計）+Codex（StageGlyphs.tsx共有レンダラー統一）の合作で`1db7af5`として着地。
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

### T4: 直接操作（risk: medium / L）— 2026-07-16ユーザー提供の意味論を統合
Daslight実機の確定挙動（ユーザー確認済み、PRIMARY_OBSERVATIONS.md参照）:
- **ストレッチ2モード**（ツールバートグルで選択）: RATE=短く⇒速く/長く⇒遅く再生（倍率バッジ表示）、
  WINDOW=本来速度のまま、短く⇒途中打ち切り/長く⇒ループ充填。F系のシーン再生レート基盤に依存。
- **フェードのDAW式操作**: ブロック下半分のドラッグでFade In/Out調整。ゾーン設計:
  上帯=移動 / 端=ストレッチ（モード依存） / 下帯端=フェードハンドル。
ブロック端リサイズハンドル（automation-range前例 `TimelineOverview.tsx:336-476` 再利用）、ドラッグ中ゴースト+ライブ刻印、Escキャンセル。
Magnetトグル: ドラッグ中スナップ。カーソル中心ホイールズーム（preventDefault + document scroll 0のハーネス断言）+背景ドラッグパン（7ボタンツールバーはフォールバック維持）。
配置: Cue PadまたはFinder行で「アーム」→レーンダブルクリックで自然デュレーション配置、**スイープドラッグで開始+デュレーション同時指定**。
受入: デュレーション変更=ドラッグ1回 / 配置=ダブルクリック1回 / ズーム中カーソル下時刻静止±2px / 全ゲートpass。

### T5: Control一貫性（risk: medium / M）
TransportCluster共通化: GOが全モードで同一ジオメトリ（±4px）。ブラックアウト3種を**KILLゾーン**（危険トークン: 暗赤枠+ハッチ塗り+destructiveActions連携、橙アクセント不使用）へ分離。
FadeProgress単一化（1色1様式）。ステータス階層化: Active/Next Cueを大セル+identity色相チップ。Live Edit死領域解消（Cue Padグリッドが高さを充填）。
**メタデータ最小フォント11px床 + CSS監査**。
契約変更: モード別トランスポート配置のハーネス断言更新。

**✅完了 2026-07-17 ブランチ90dfa1a → 合議マージ8c364a0（Fable直接実装 worktreeレーン）** —
実装コア: BO3種（DMX/Video/All）を.killButton（#6b2424枠+斜線ハッチ、engaged=#6e2222塗り、
橙不使用）へ、All Clearを.killClearへ。TouchSafetyDeckも同クラス継承。Active/Nextセルは
.liveCueStatusCell + identity色相チップ（8x14、cueIdentityHue）。フェード進捗は
.fadeMeter角形+中立#cfd6db、liveFadeMeter progressのaccent-colorも同色へ単一化。
ハーネス契約変更（列挙）: killZone計測5項目（killButtonCount/killClearCount/
killButtonBorderIsRed/liveStatusMinFontPx/liveCueIdentityChipCount）+
hasExpectedKillZone断言を ^control-live-\d+x\d+$ ゲートで追加、killZoneFailuresを
exit判定・counts行・両JSONダンプへ配線。既存断言の期待値変更なし。
検証: worktreeフルマトリクス217 exit 0 → F7込みmainへの合議マージ（コンフリクト=styles.css
1ハンク、Fable解決→Codexレビュー ADJUDICATION: AGREE、byte照合/ブレース収支/両側断言残存
確認済）→ マージ済みツリーで tsc+viteビルド緑・フルマトリクス217 exit 0・ライブJS計測
（ハッチKILL3個/チップ2色/デスクタブ単一行/スクロール0/0、F7 audioクリップ共存）。
教訓（スキルへ反映済み）: ハーネスmeasureテンプレート内regexは\\dエスケープ必須、
焦点実走の合否はexit codeで確認。

### T6: VJ Desk再構成（risk: medium / L）
クリップバンク支配的（サムネイル第一のグリッド、12パッド無スクロール表示）。Preview/Programモニタが中央列高さ≥70%。Audio/AutoVJ/Reactive設定はアコーディオンドロワー（既定折りたたみ、localStorage永続）。レイヤー6行常設表示。Out系ボタンのスコープ語彙統一。
SynapseRack式フローティングパネルは一画面契約維持のため不採用（採用するなら別途契約再交渉）。

**✅完了 2026-07-18 ブランチ4182ad6 → 合議マージf344713（Fable直接実装 worktreeレーン）** —
実装: Audio In/Auto VJ/Reactiveを折りたたみドロワーバー化（MixerDrawerBar.tsx新規、
localStorage毎ドロワー永続、閉状態でもstatus読み出し表示、ストリップはDOM直下維持+
隣接兄弟セレクタ隠しで既存`>`セレクタ全温存）。パッドはサムネイル第一（絶対カバー+
下端オーバーレイ、56px床）、2列チロームを全ミキサー高さへ昇格し**全5ビューポートで
12/12パッド無スクロール実測**。モニタ7fr/3fr（実測0.70）。レイヤー行94→78pxで6行可視。
ブラックアウト5様式をKILLファミリー（Video BO/BO/Clear BO + killButton/killClear）へ統一。
vj-bankフィクスチャ（14クリップ）+常設5シナリオ×7断言。
ハーネス契約変更（全列挙はコミットf344713/4182ad6参照）: 非live mixerのliveAudioRail検査を
ドロワー既定閉検査へ再交渉、clipGrid床90→56px（理由明記）、monitorDominance≥0.65と
killVocabulary新設、runLiveAudioAcceptanceが音声ドロワーを開閉（開状態のlocalStorage
リークを閉で遮断）、auto-vj/reactive/fullscreen-vjフェーズへopenMixerDrawer追加。
検証: worktreeマトリクス222 exit 0 → F6込みmainへ合議マージ（コンフリクト=
viewportFixtureData.ts 1ハンク、Fable解決→Codexレビュー ADJUDICATION: AGREE、
双方向numstat照合+シンボル単位残存確認）→ マージ済みツリーでビルド緑・マトリクス
222 pass exit 0・vj-bank 12/12×5。既存欠陥2件（live-audio JAタブ探索/reactiveラック
横overflow）はA/Bでベース起因と裁定→タスクチップ経由の別セッション修正
（fa0b987/5742959）がマージ前に着地済み。

### T7:（オプション・要明示承認）永続identity色 — protocol変更

**✅完了 2026-07-18 ブランチ60f1ad1 → 3ラウンド合議マージ7ee9214（Fable直接実装、
frontend-only緩和はユーザー承認済み 2026-07-18）** —
Protocol: CueSummary.color + EngineSnapshot.group_colors（BTreeMap、両方
skip_serializing_ifでレガシーバイト同一）。Engine: SetCueColor/SetGroupColor
（ack+expiry、RestoreGroupColorsロールバック新設）。Tauri: set_cue_color/
set_group_color（cue-point hex検証器を再利用）+ .sdcラウンドトリップ/レガシー
既定テスト。UI: cueIdentityCss/groupIdentityCssが永続色優先（role別明度クランプ
fill 40-65/band 24-36/text 68-80・彩度20-85でgraphite可読性契約維持）・ハッシュ
fallback。17呼び出しサイト全書換（音声クリップのパスハッシュ2箇所は対象外）。
ピッカー: cue編集行 + Scene Matrixグループ列ヘッダ。scene-matrixフィクスチャに
#ff3366(cue301)/#22aa88(backグループ)を永続化。
ハーネス契約変更: scene-matrixへ3条件（永続cue色hsl(345)勝利/永続グループ色
hsl(165)勝利+ハッシュ属性210温存/グループピッカー2個）、cue-recallへ
cueColorPickerPresentInCueEditRow（F8の32条件構造へ33番目として統合）。
既存期待値変更なし。
検証: worktreeマトリクス222 exit 0 → F8込みmainへ合議マージ（コンフリクト3ファイル
5ハンク全て同一アンカー隣接追加、両側保持で解決）→ マージ済みツリーで全ゲート+
マトリクス222 exit 0。合議は3ラウンド: Codex初回レビューがマージ実質全項目を正当
確認しつつFableパケットの条件数誤記を検出しDISAGREE → 訂正 → 訂正漏れ1箇所を
再検出 → 最終AGREE（相互検証が機能した証跡としてコミット7ee9214に全記録）。
`CueSummary.color: Option<String>` + `EngineSnapshot.group_colors`（serde default、`.sdc` v1互換維持、前例: StageObjectSummary.color / VideoCuePointSummary.color）。set-cue-color / set-group-colorコマンド、カラーピッカーUI。永続色優先・ハッシュfallback。
**「frontend-only」制約の明示的緩和が必要。**

### T9: 2D Mapping 照明専用化 + 灯体グリフ再設計（risk: medium / L）— 2026-07-15追加承認
ユーザー指摘（実キャプチャ比較）: 投影面のワープ矩形（XY面の概念）がXZ床面図に矩形描画されて
おり幾何学的に誤り。同一スクリーンがStage Object("Screen")とProjection Surfaceで二重描画。
灯体グリフが過大（大円+太リング）でDaslightの12-16px角形に比べ視覚ノイズ大。
- (a) 平面分離: 投影面の編集ハンドル（コーナーピン/回転/スケール）をMappingから削除し
  Setup > Video Projection Mapへ一本化。Mapping上は**線分+向きマーク+ラベルの参照表示のみ、
  レイヤー既定OFF**（承認済み）。Screen種Stage Objectは薄い帯+ラベル表現へ。
- (b) 灯体グリフ: タイプ別コンパクトグリフ（par=角形 / moving=円+ヨーティック / bar=横長矩形、
  基準12-16px、ズームスケール上限付き）、細枠選択・薄枠ホバー（Daslight流）。ライブDMX色表示は維持。
- (c) 操作性: マーキー視認性、グリッド階調（メジャー/マイナー）、スナップ表示、ズーム操作の改善。
- 受入: 灯体ヒットターゲット≥12pxゲート維持 / 2,000灯windowingゲート維持 / 投影面編集が
  Setup > Videoで完結すること / Mappingに投影面ハンドルDOMが存在しないこと / 全ゲートpass。
- 契約変更: mapping系ハーネス（投影面レイヤー期待値）の再交渉 — 変更断言を全列挙。

### T10: 統一ワークスペースシェル（✅完了 2026-07-16 コミットe0e2e3e）（risk: high / XL）
完了実績: 下半分4領域（GROUPS/2Dステージ/SELECTIONS/コンテキストペイン）のrectがSetup→Control→Setup
切替で全5解像度delta 0。フルviewportマトリクス213件green。付随修正: Timelineキャンバスの
ResizeObserverレイアウト振動（143s→0.1ms、contain: size layout）、コンパクト高でのキャンバス
クリップ外ドラッグ死（max-height:800pxで148px化、1366x768/1280x720で+60pxドラッグ正確コミット）、
scene-block reveal（有界8フレームリトライ）、行コンパクト化（334→117px）。ハーネスは122条件の
名前付き配列 + FAILED CONDITIONS/DRAG STATSログ化。CDP診断ツール2本を追加。
Daslight 3画面比較で実証された不変条項をSyndocalへ移植する最大の構造変更。
**下半分を全ワークスペース常設**にする: GROUPSリボン + 2D Stage Map（T9後の照明床面図、
ツールレール込み）+ SELECTIONS列 + 右下コンテキストペイン。**上半分だけをワークスペースで交換**:
Setup=Library/Patch系タブ、Control=Cueマトリクス+プロパティ、Touch=レイアウトデザイナー（T11）。
右下コンテキストの内容はワークスペース従属: Setup=リミット/パッチ詳細、Control=Timeline/Mixer
（Daslight同配置）、Touch=属性エディタ。ヘッダー（GO/BPM）は現行どおり常設。
- T5のTransportCluster統一はここへ吸収（シェルが1つになるため自然に達成）。
- Touch上半分はT11の「配置と実操作を同一面で行うタッチサーフェス」（下記、ユーザー訂正済み）。
- 契約変更: 現行回帰契約のワークスペース別レイアウト定義（Control 3面構造、Touch構造）を
  「統一シェル+上半分交換」契約へ全面置換。viewport/touch系ハーネスの大規模再交渉を伴う。
- 受入: 3ワークスペース間の切替で下半分のDOMジオメトリが不変（ハーネスでrect一致断言）/
  全ゲートpass / 1280x720〜2048x1152 containment維持。

### T11: 編集可能Touchサーフェス（risk: medium / XL、T10後）— 2026-07-15ユーザー訂正反映

**✅完了 2026-07-18 コミット5c3de86（Codex実装）** — EDIT/LIVE同一面切替、8種パレット、
12×8グリッド、ページ管理、既定プリセットページ（後方互換）、Safety Deck常設、Webリモート
配信（レイアウト+基本操作）、レイアウトは`.sdc`保存（EngineSnapshot.touch_surface、
skip_serializing_ifでレガシーバイト同一 5040/17517実証）。48px契約を配置コントロールへ拡張。
ハーネスは史上最大の再交渉（旧固定デスク断言群→composed契約、全列挙はコミット参照）。
検証: 全Rustゲート独立再現、フルマトリクス227 exit 0。
監査ギャップ#4の解消。Daslight TOUCH実機確認（ユーザー提供スクリーンショット）による正しい構造:
**同一面がEDIT/LIVEトグルで「配置モード」と「実操作モード」を切り替える** — 配置した
カラーホイール/フェーダー/ボタン等はその場でそのまま操作できる（デザイナーと演奏面は分離しない）。
- 統一シェル（T10）のTouch上半分 = グリッド配置のタッチサーフェス。ADD CONTROLS相当の
  パレット（Label/Image/Button/Fader/Dial/Incremental Wheel/Color Wheel/XY Grid）+ ページ管理。
- EDITモード: 配置/リサイズ/割当。LIVEモード: 直接操作、ターゲット≥48px（既存Touch安全契約を継承）。
- 現行の固定Touchデスクは「既定プリセットページ」へ移行し、Safety Deck（GO/BO群）は
  composedレイアウトでも常設ストリップとして残す（安全契約維持）。
- 同じcomposed surfaceをWebリモートへ配信。
- 詳細設計はT10完了後に固定。`.sdc`へのレイアウト保存 or 端末ローカルかは設計時に決定
  （Daslightはショーファイル保存 — 会場持ち回りを考えるとproject保存が有力、protocol変更を伴う）。

### T8: Timeline Focusレイアウト（✅完了 2026-07-16 コミット41aaaaf）
VJ fullscreen focus（コミット`7ab0795`）と同型の、全幅Showサーフェス+2Dステージ折りたたみ+Esc復帰。
T10統一シェル下で「右下Timelineペインの一時全幅化トグル」として実装（Daslightのペイン展開
ボタンと同型）。
完了実績: コンテキストタブバー右端のトグルでTimelineペインがバンド全幅化（1920: 715→1910px /
1366: 540→1356px）、stage/selectionsは非表示・GROUPS帯は維持、Esc/トグル/ワークスペース離脱で
自動復元（4領域rectが0.01px精度で完全復元）。振動対策契約維持（展開時240px・コンパクト148px固定 +
contain: size layout）。状態はコンポーネントローカルの一時信号でApp.tsx非接触・非永続。
ハーネスはpersistent-band-invarianceに展開チェック（14断言）を統合、全5解像度で合否ゲート化。
フルマトリクス212件green。

### T12: マルチウィンドウワークスペース（risk: medium / L、現行シリーズ完了後）— 2026-07-17ユーザー承認

**✅完了 2026-07-18 ブランチ0010443 → 合議マージd95de64（Fable直接実装 worktreeレーン）** —
ペイン窓（?syndocalPaneWindow=stage|timeline、フルAppを単一ペインへ集約 — 全コマンド/
スナップショット機構が無改造で第2窓動作）、open/close_pane_window Tauriコマンド+破棄イベント、
ポップアウトトグル+stageポップ時のメイン帯スロット詰め、ウィンドウ配置はlocalStorage保存
（マシン固有 — T11の.sdc保存と対の設計判断）+ネイティブ起動時再オープン。T10固定高契約が
全階層!important強制のため4層の同格上書き+grid-row固定で窓充填を実現（実測: stage窓1272×659、
timeline窓692/659/628、popped-main 376+896、全状態スクロール0/0）。
ハーネス: pane-windowフェーズ8断言×5ビューポート新設（既存期待値変更なし）。
検証: worktreeマトリクス227 exit 0 → 合議マージ（styles.css EOF 1ハンク、Codex AGREE）→
マージ済みツリー232 exit 0。
基本の「上1+下2」統一シェルを維持しつつ、ペインを別ウィンドウへ分離してマルチディスプレイ対応。
想定構成例: メイン=Live Desk / 2枚目=Timeline全画面 / 3枚目=2Dステージ大画面。
- 下地は既にある: M3のネイティブ出力ウィンドウ実績（Tauriマルチウィンドウ・モニタ指定・独立クローズ）、
  エンジンスナップショット購読=状態の唯一の真実（別ウィンドウも同購読で自然同期）、
  T系で完了したペインのコンポーネント化。
- 実装項目: ペインのポップアウトトグル（T8と同系UI、メイン側はスロットを詰める）/
  ウィンドウ毎のUIローカル状態（選択・ドラフトはウィンドウ固有を既定、必要ならTauriイベント橋渡し）/
  レイアウト永続化（どのペインをどのモニタへ — .sdc保存かローカル設定かはT11と同論点で設計時決定）/
  一画面契約（document/appスクロールゼロ）のウィンドウ単位拡張とハーネス断言。
- 位置づけ: 照明完成度シリーズ（T4→F5→T3/T5→F6-F8→T11）完了後に着手。

## T24系: オペレーター体験トランシェ（2026-07-29開始）

2026-07-29のユーザー指示「Daslightを超える照明ソフトとしての完成を優先（UI含む）、人が操作する・見るに重きを置く」に基づく実機オペレーター監査（`qa/OPERATOR_AUDIT_2026-07-29.md`が正）から導出。実装=Codex gpt-5.6-sol、計画・監督検証・コミット=Fable（体制不変）。各トランシェの受入に**ネイティブ目視を必須**とする（ハーネス合格だけで閉じない）。

- **T24-A ステージ可読性 ✅完了 2026-07-29（23b50a5）** — 41灯密度でのラベル判読不能を解消（12文字省略＋密度25デクラッタ＋衝突回避、check:stage-labels新設）。
- **T24-B Setup>Mapping死空間解消 ✅完了 2026-07-29（4f7bb22）** — Mapping中だけ帯ステージをその場拡張（373→941px/2.52倍、旧上面充填≥99%、離脱0.5px復元、mappingExpansion 19断言新設、invariance計測はPatchへ再交渉）。F6（タイムライン展開レーン高）は未着手のまま残る。
- **T24-C ダイアログ親設定＋インポート再入ガード ✅完了 2026-07-29（6b00c9f）** — 全37ダイアログ呼び出しへ親付与、.dvcインポートのビジー表示＋メニュー無効化。F4の「数十秒無反応」の真因はF3の隠れダイアログと実測で訂正。
- **T24-D インポート内容発見導線 ✅完了 2026-07-29（aba8d89）** — MatrixセルSSバッジ（FX共存・非クリップ断言強化）、タイムライン空状態ヒント、FXワークスペースのCue所有件数＋到達経路表示（F5+F11+F13）。
- **T24-E 小型摩擦バッチ ✅完了 2026-07-29（e334c15）** — F7「ミキサー」→「VJデスク」改名（ハーネス4参照再交渉）、F9フェーダーペイン読取ストレッチ（重なり欠陥をネイティブ目視で検出→修正ラウンド）、F10タップボタン40px行内解決、F12復旧メッセージ動的パターン日本語化＋断言。
- **T24-F 展開タイムラインのレーン優先 ✅完了 2026-07-29（9cd7b90）** — 展開時レーン146→472px（3.2倍）、ミキサー行34pxコンパクト、復元完全一致（F6解消）。
- 未着手で残る監査項目: F8（Matrix列密度: Daslight約12列 vs Syndocal 7列の実測差を確認済み、セル情報量トレードオフ込みでT25候補）
- **T24系完走 2026-07-29**: 全5トランシェ・6コミット（修正ラウンド2件込み）。各トランシェは監督フルゲート再実行＋フルマトリクス232pass＋ネイティブ実機目視（Shinkan2026復元状態）で受入。ネイティブ目視はハーネス盲点の視覚欠陥2件（SSバッジ切り詰め・F9重なり）を捕捉し、うち1件は断言自体を実描画幅検証へ強化して再発を封じた。
- F8（Matrix列密度）はPhase C操作手数計測後に判断

## T25系: 縦密度・脱ごちゃごちゃ（2026-07-29ユーザー実機比較フィードバック起点）

ユーザーがDaslight（Shinkan実ショー）とSyndocalを同時に開いた実機比較での直接指摘を正とする: 「UIが煩雑・量が多い・ごちゃごちゃ」「真ん中の照明/映像マスターは何？BPMはトップにもあるのに重複。全部上にまとめるべき」「左下マッピングビューは見づらく操作しにくい」「右下ライブ編集のフェーダー表示がDaslightと違って分かりにくい」「ライブ編集/タイムライン/VJデスク切替だけで1行消費は無駄」「ラベルと同じ行にまとめられるものが縦に行を取り、情報表示スペースが減っている」。

- **T25-A 行整理・縦密度 ✅完了 2026-07-29（ecbfba3）** — トップバー1行化（GO＋照明/映像マスター＋BPM/タップ、1280収容・42px）、パンくず行と中段マスター行を廃止、文脈タブを34pxペインヘッダへ相乗り。マトリクス370→494px、通常時レーン146→249px。断言削除0・新規6断言。監督フルマトリクスがT12ペイン窓へのヘッダ漏れを検出→窓側抑止で修正。元仕様: (a)パンくず行（ライブデスク/ライブ編集等）を廃止しトップバーへ吸収 (b)照明マスター/映像マスターをトップバーのGO/BPMクラスタ脇へコンパクトスライダーとして移設し、中段マスター行とBPM/タップ重複を削除 (c)文脈タブ（ライブ編集/タイムライン/VJデスク）を文脈ペインヘッダ行内のセグメントコントロールへ相乗りさせ専用行を廃止 (d)解放した縦（目安3行・~130px）をマトリクスと下段帯へ配分。Daslightの「1行相乗り」原則を規範とする。契約影響大: 帯ジオメトリ・T10不変・T24-B/F展開断言の全面再交渉（全列挙必須）。
- **T25-B ライブ編集フェーダーのDaslight型再構成 ✅完了 2026-07-29（305a44f）** — 複数選択でタイプ別カラム（タイプ名＋台数、アイコン＋%readout＋縦フェーダー＋OFF/トリム、非対応は未対応表示、書込は選択中該当タイプへファンアウト）、単灯は縦フェーダー化した詳細エディタ維持、FADERカテゴリはチャンネル別バンク。live-edit-types焦点契約5解像度＋Positionコンソール焦点8断言新設。監督フルマトリクスが初版のT13 Positionコンソール破壊（#position-tool-tab-position消失で全断言未実行）を検出→修正ラウンドで復旧。フルマトリクス237pass。実機41灯選択でユーザー提供Daslightスクショの文法一致を確認。元仕様: Daslightは選択に対しカテゴリレール＋灯体タイプ別カラム（GENERIC/MEGA BAR RGBA…各タイプにダイヤル/OFF）で「選択→タイプ別に一括操作」が一目。Syndocalの単灯属性エディタ形式を、タイプ別カラム主体へ再設計する。
  **受入基準の確定（スクショ2枚）— ✅完遂 2026-07-29（de3fefe、round 3）**: FADERバンク縦化・位置パッド化・向き断言（縦横比実描画検証）新設で完了。 (1)主要操作子は**縦フェーダー**（横スライダー基調から転換）。(2)タイプ別カラム=タイプ名ヘッダ＋状態アイコン＋数値%readout（OFF状態表示）＋縦フェーダー＋微調整コントロール。(3)FADERカテゴリ=選択灯体のチャンネル別縦フェーダーバンク（CH番号＋機能アイコン＋OFF＋縦フェーダー、横スクロール）。実装中のCodex成果物がこの文法と乖離した場合はスクショを添えて修正ラウンドを回す。
- **T25-C 文脈対応ステージクローム ✅完了 2026-07-29（ae5586c）** — Control文脈クローム35超→7要素（ツール1行4アイコン＋マッピング編集リンク＋選択名前リスト、上限12断言付き）、文字レール/スナップ行/readout/ドロワーはSetup>Mappingのみ。監督フルマトリクスが110%スケールフェーズの帯述語旧契約を検出→「選択UI1系統」へ一般化して決着。フルマトリクス237pass、実機7要素確認。元仕様: ユーザー指摘「左のアルファベット一文字ボタン類がスクロールできるせいで領域が圧迫」——S/P/R/H/L/B/G/V/O/%/?のツールレール＋ツール行＋表示/フィット/ズーム行＋スナップ/グリッド行というSetup>Mapping用フル装備がControlの小ペインへそのまま持ち込まれているのが根本原因。対応: (a)Control文脈ではステージを「見る・選ぶ」面へ純化し、必須ツール（選択/パン/全体フィット/ズーム）を**アイコン1行のみ**に集約、文字レール・スナップ/グリッド行・レイヤートグル行は非表示（編集はT24-Bで拡張済みのSetup>Mappingへ誘導） (b)レールのスクロールを廃止（収まらないUIを許さない） (c)選択ドロワーの既定折りたたみを検討 (d)Setup>Mappingのフル編集クロームは不変。Daslightの下段ステージ（ツール1行のみ）を規範とする。あわせて小ペイン時のビームノイズ・コントラストも再考。
  **追加指摘（同日）「そもそも分かりにくい・色々多すぎる」→ 要素数削減を仕様へ昇格**: 実測でDaslight下段ステージのクローム操作要素は約10個（GROUPS行＋ツール1行＋SELECTIONS名前リスト）に対しSyndocalは35個超（readout行「41投影面/0オブジェクト」、ズーム/フィット系だけで7要素、スナップ5プリセット＋グリッド入力、ラベル/ビームトグル、文字レール11個、選択ドロワーの複製/削除/検索/グループ編集群）。T25-C受入条件: Control文脈のステージクローム操作要素を**10個前後**（グループ行＋選択/パン/フィット/ズームスライダーのツール1行＋選択名前リスト）へ削減し、削減した全要素の行き先（Setup>Mapping）を列挙する。ハーネスへ要素数上限断言を追加。
- **T25-D ライブデスクヘッダの1行化（2026-07-29ユーザー追加指摘）**: 「ここも二行である意味あるか？」——1行目（ライブデスクラベル/マトリクス・キューパッド切替/照明・30シーン/ステータス/準備完了）と2行目（トランスポート＋KILLゾーン）を統合する。ラベルは削除（文脈自明）、「準備完了」は最下部ステータス行と完全重複のため削除、「照明・Nシーン」はステータスポップアップへ畳む。統合後: [戻る|GO|Pause Fade|Play Timeline|DMX BO|映像BO|全BO|全消去|フラグをクリア]＋右端[マトリクス/キューパッド|ステータス]の1行。killZone断言（KILL3種・ハッチング・寸法）は維持し、行構成変更分のみ再交渉（全列挙）。解放高はマトリクスへ。
- **T25-E フレームレス一体化ウィンドウ ✅完了 2026-07-30（本体+循環チャンク修正1ラウンド）** — 本体窓のみ`decorations:false`、42pxトップバー=タイトルバー（背景+中央プロジェクト表示がdrag region、インタラクティブ子は除外）、右端に40x40の最小化/最大化/閉じる（close()=CloseRequested経由で復旧フロー維持）、8方向`startResizeDragging`エッジゾーン、capability5権限の最小追加。manualChunks再配分でmain index 498.84→489.93kB、修正ラウンドで`shared-helpers`チャンク新設により循環チャンク警告2件をゼロ化（依存一方向化）。実機受入: 最大化client 1920x1009→**1920x1032**（+23px、契約更新済み）、ドラッグ移動（正確なdelta3回、window.screenXはWebView2でstaleのためGetWindowRectを正とする）、ダブルクリック最大化トグル両方向、最大化/最小化/閉じるボタン、コーナーリサイズ1280x800→1380x850、F11=1920x1080/Esc復帰。NATIVE_WINDOW_ACCEPTANCE/RELEASE_STATUS更新済み。
- **T25-F トップバーPULSEメーター ✅完了 2026-07-30（76e80b0、本体+修正2ラウンド）** — BPM/タップ隣に58x40のPULSE常設（42px行内、1280w共存）。既存`live_audio_input_levels` 33ms単一in-flightテレメトリを状態ゲート化して再利用（停止中ポーリングゼロ実測0件）、250ms鮮度でfail-closed消灯、engine stale/safety-clearも即消灯。クリックでSetup>Video>Outputs音声入力へジャンプ＋フォーカス。PULSE英語不変語。断言11件追加。修正2ラウンド: 音声入力設定セクション追加がsetup-video containmentを小解像度→（修正反転で）大解像度と順に破壊、ラウンド2で全5解像度green（このカテゴリはfail行を出さないため監督フルマトリクスのexit code検証が検出源）。実機受入: 常設・消灯既定・クリックジャンプ・スピーカートーン実収音で点灯（lit=true）・停止で即消灯。
- **T25-G パラメーター語彙の英語回帰 ✅完了 2026-07-29（317b16a）** — カテゴリレール/Speed・Size・Phase/波形7種/FXファミリー8種をja localeでも英語表記へ（COLOUR MAPPINGS綴り採用）。動作語（再生/リセット等）は日本語維持を断言化。identity不変契約として正式登録。実機でSpeed/Size/Phaseストリップ目視。元仕様: 照明業界標準に合わせ、パラメーター語彙クラスを日本語ロケールでも英語表記へ戻す——カテゴリレール（Dimmer/Color/Position/Beam）、Scene Liveモディファイア（Speed/Size/Phase、Daslightダイヤル表記一致）、波形名（Perlin等）、FXファミリー（COLOR FX/CHASER FX/MOVE FX/VALUE FX/CURVE FX/MAPPINGS/COLOUR MAPPINGS）、属性名。文章・説明・メニュー動作・空状態・エラー文は日本語維持。実装はローカライズゲート既存の「意図的共通表記」クラスへの移動で100%契約を維持し、対象語の全列挙＋ハーネスのJAラベル参照の再交渉全列挙を受入条件とする。境界が曖昧な語はDaslight実機表記（ユーザー提供スクショ群）を裁定基準にする。
- **T25-H シーンセル相互作用 ✅完了 2026-07-29（d7f5409）** — アクティブセル再クリック=対象キュー単位リリース＋ストリップ閉じ（1クリック断言）、⣿ドラッグのドロップ先分岐（同列=既存move_cue再利用の並び替え・ゴースト/挿入線付き、タイムライン=配置維持、別列=エラー）。protocol変更不要で完遂。実機で両挙動目視。元仕様: (1)「速度等のコントローラーが再度押しても閉じられない」→ セルクリックは常にトリガーのみでリリース未実装（SceneMatrixPanelのonTriggerCue固定）。**アクティブセルの再クリック=リリース（トグル）**へ変更しストリップも閉じる。再トリガーはストリップ内/GO経由へ整理。T17/T18/T20系断言の再交渉全列挙。(2)「右の並び替えみたいなところをドラッグしても移動できない」→ ⣿は`cueTimelineDragHandle`（タイムライン配置専用）でマトリクス並び替えは不存在のアフォーダンス不一致。**ドロップ先分岐の二役化**（同列内=並び替え/タイムライン=配置）。並び替えのエンジン/protocolコマンド有無を実装前調査し、無ければユーザー承認の上で追加（.sdc互換はserde default原則）。優先度: フェーダー縦化ラウンド直後、T25-Gより先。
- **T25-I フェーダー外装＋FADERビュー純化 ✅完了 2026-07-29（8126422）** — 共有VerticalFaderInput新設（appearance:none＋insetトラック＋グリップ線ピルキャップ32×16px、キーボード/フォーカス/aria維持を断言）、FADERビューからカテゴリ操作カード/GDTF機能パネル撤去・バンク全幅化（幅比1.0）。新規断言11件。実機で新外装＋T25-G英語レール同時確認。元仕様: (1)「デフォルトのスライダーすぎる」→ 全縦フェーダー（タイプ別カラム/チャンネルバンク/単灯）へ専用外装: inset彫り込みトラック＋グリップ線入りピル型キャップ（Daslightキャップ様式、ユーザー提供スクショ裁定）。ネイティブrange要素の見た目依存を廃止。(2)「カテゴリ操作とその右の項目はDaslightにはない」→ FADERビューからカテゴリ操作カード（ゼロ/中域/Full/Default）とGDTF機能パネルを撤去し、チャンネルバンクを全幅化。カテゴリ操作は各カテゴリビュー既存ボタン（出力/Half/Full/バンプ等）で代替済みのため機能損失なし、GDTF readoutはDMXタブへ残置。断言: fader外装のスタイル断言（トラックinset/キャップ寸法）、FADERビューのカテゴリ操作/GDTF不在断言、バンク全幅断言。
- F8（Matrix列密度~1.7倍差）はT25-Aの行整理後に再計測して列幅コンパクト化を判断。

## T26系: シーン中心オーサリング（2026-07-29ユーザーのDaslightモデル解説起点・最重要ワークフロー変更）

ユーザー解説によるDaslightの確定モデル（スクショ2枚で裏付け）: (1)シーン新規作成→右ペインにFX種類chooser（STEPS/COLOR FX/CHASER FX/MOVE FX/VALUE FX/CURVE FX/MAPPINGS/COLOR MAPPINGS/SUPER SCENE）が常駐、未選択ならStatic。(2)FX選択→右ペインが当該FXエディタ（例: CURVE FX=波形プレビュー+Rate/Size/Phase/Offset/Phasing+Attribute value+Beams+Features）。(3)シーンの中身はシーン選択状態で下段EDITフェーダー（Mappingで灯体選択→操作）で作り、**その値がそのままシーンへ上書き保存**。(4)EDIT/LIVEトグルの本義: EDIT=フェーダー値がシーンへ上書き保存／LIVE=シーンへ保存されないライブオーバーライド。現Syndocalは部品（cue-owned FX/Store Scope/Update Look/Programmer/T16 chooser/T17モディファイア）を持つが直結動線に組まれておらず、ユーザー評価は「シーン作成・編集は大きく劣る・操作しにくい」。

- **T26-A シーン設定右ペイン ✅完了 2026-07-29（7f4f321、4ラウンド）** — 右レールへSceneSettingsPane常駐（Static: プロパティ+9ファミリーchooser／FX持ち: T16エディタ切替、既存コマンドのみでprotocol変更ゼロ）。セル右端identity色ストリップ=編集選択、本体=再生のみ。監督が4回の欠陥を検出: ストアフォーム到達不能（全断言未実行の致命）→controlMode 35件→ブラウザ合格/WebView2不発のストリップクリック乖離（実button化+elementFromPoint断言で解決）。実機でDaslightモデル完全動作確認。（**意味論訂正 2026-07-29ユーザー「Daslightでは選択(編集)と再生が違う。押したら再生、右にある色つきの部分を押すと編集モード」**: セル本体クリック=再生のみ（編集選択の副作用なし、T25-Hトグルリリース維持）。**セル右端のidentity色ストリップ=編集選択専用クリックゾーン**（dl-16のB-WineRed右端ピンク帯が実例）。✎は右端ストリップへ統合または廃止。初版の「トリガー=選択を兼ねる」は誤りとして訂正ラウンドで置換）→右ペイン（現・実行中/次のキューのレール領域を拡張）にシーン設定を常駐: 新規/FXなし=FX chooser（T16の9ファミリーを流用）＋Staticプロパティ（fade/duration系）、FX持ち=当該FXエディタ（T16エディタ群のシーン文脈再配置）。FXワークスペースへの遷移なしでシーンのFXを作成・編集できること。
- **T26-B EDIT/LIVE直結書込 ✅完了 2026-07-29（411505e、本体+修正1ラウンド）** — ControlフェーダーデッキへLIVE/EDITトグル（既定LIVE）。EDIT=編集選択シーンへ、確定フェーダー値をscoped `update_cue_from_current`（selectedFixture/selectedGroup、ジェスチャ単位1トランザクション、coalesceキー=キュー+スコープ）で即時上書き。未選択EDITは「EDIT: シーン未選択」ガードで書込なし。選択中はidentity色「EDIT: <シーン名>」バッジ。LIVE非破壊・Undo/Redo実機確認済み。LIVE/EDITは英語不変語へ昇格。修正ラウンド: 実機でトグル文字不可視（3列grid第4子の暗黙行折返し+`.fader span`汎用色+grid stretch54px化）→専用単一行サマリ契約（26px/24px固定・高具体度色継承）＋描画テキスト可視性断言9件（selection/empty/workspace-operator 3幾何）。ブラウザ断言はDOMテキスト存在では不十分で「span rectの最近接クリップ祖先内包」が必要という教訓を確立。
- 実装順: T25-G/I完了後に着手。protocol変更は原則不要見込み（既存コマンド編成）だが、自動Update粒度で必要が生じたら承認を取る。受入はDaslightモデル4点との1:1突合＋操作手数（シーン新規→FX選択→中身作成→保存の一連）のDaslight同数以下。

## T27系: ビジュアルデザイン刷新（2026-07-30ユーザー批評「UIがcssそのまますぎる・DasLightと比べてデザインそのものがダサい」起点）

Daslight/Syndocal全画面並置スクリーンショット（ユーザー提供）による差分診断: ①ボタンが素のボックス（1px枠グレー矩形＋プレーンテキスト。Daslightは列内トランスポートやツールをアイコン化し、テキストボタン依存が低い）②マイクロタイポグラフィ不在（DaslightはSTATIC/FX/COLOR FX等をsmall-caps+letter-spacingの小型型ラベルにし、シーン名との明確なサイズ階層を持つ）③色の階層（Daslightは背景3層+1px境界で面を分け、高彩度はシーン識別色とFXプレビューだけ。Syndocalはオレンジアクセントが大面積で騒がしい）④アイコン不足（固定チップ・マトリクス/キューパッド切替等がテキスト）⑤角丸・余白の一貫性不足。

- **T27-A デザイントークン確立 ✅完了 2026-07-30（8d85583）**: タイポ階層（小型ラベルのsmall-caps/letter-spacing、名前>型ラベルのサイズ関係）、背景3層+境界1pxの面体系、アクセント使用規則（identity色・GO・危険系のみ、下線/スライダー等の常時オレンジを抑制）をstyles.cssのトークン（CSS変数）として定義し全面適用。既存の一画面・focus-visible・identity契約は不変。
- **T27-B シーンセル再設計 ✅完了 2026-07-30（48730cc セル解剖＋05fa507 156px列）**: Daslight型コンパクトカードへ（型ラベル小型化、固定チップ/ドラッグハンドルの控えめ化、セル情報階層=名前>番号・時間>チップ）。T25-H/T26-Aの操作意味論（本体=再生・右端ストリップ=編集選択・展開モディファイア）は不変。
- **T27-C アイコン化**: トランスポート行（戻る/Pause Fade/Play Timeline等）と表示切替（キューエディター/マトリクス/キューパッド/ステータス）をアイコン+ホバーラベルへ（フェーダー列と同じ「アイコン+title」原則）。BO系は誤操作防止の観点でテキスト維持を検討（実装時判断・報告必須）。
- 各トランシェの受入にDaslight並置キャプチャ比較を含める。実施はControlデスク密度パリティ（ツール列拡充+フェーダー列コンパクト化）着地後。

## T28: マッピングのライブカラー表示（2026-07-30ユーザー指摘「マッピング上で今の色の確認ができない。DaslightはMega barの8セグメントを理解して8ドット出し、光らせれば色と明るさで出る」起点）

- **T28-A ライブ出力色の灯体グリフ反映 ✅完了 2026-07-30（444a810、修正3ラウンド）**: エンジン既存のDMXプレビュー/レンダ済み状態の読み取り専用経路をUI~30Hzで購読し、2Dマッピングの灯体グリフを現在の出力色×輝度で描画（消灯=暗色、Dimmerで輝度変調）。engine/44Hz不変。選択外形・identity・T24-Aデクラッタ・2000灯windowing/大規模ショー性能ゲート維持。
- **T28-B マルチセグメント描画 ✅完了 2026-07-30（444a810＋0669ad1グリッドマス化＋759516cセル別ビーム）**: 多セル灯体（Mega Bar等）をプロファイルのチャンネル構造（番号付き属性 Red1/Green1/…/RedN またはGDTFサブジオメトリ）からセグメント数を導出し、yaw追従のNドットバーとして描画、セグメント個別の色・輝度を表示（Daslightパリティ）。
- 受入: Shinkan実機でBar系を発光させ、色・輝度・セグメント分解が2Dマッピングに実表示されること（並置キャプチャ）。カテゴリレール左統一（Fader時の水平化廃止・Daslight準拠、1280w可視CH断言を幅由来値へ再交渉）はT27-A直後の小トランシェとして先行。

## 実装体制

- 実装は2レーン並行（2026-07-17ユーザー承認）: **Codex gpt-5.6-sol**（メイン作業ツリー、cue/エンジン系）+
  **Fable直接実装**（専用worktree `../KDMX-fable-lane`、タイムラインエディタ/UI系）。ドメイン非重複で
  トランシェを割り当てる。
- **マージ規則（ユーザー指示 2026-07-17）**: worktreeブランチのマージ、特にコンフリクト解決は
  **FableとCodexの両方の判断**で行う。運用: Fableがコンフリクト解析と解決案を作成 → 該当ハンクを
  Codexへレビュージョブとして提示 → 両者一致で初めてマージコミット。不一致はユーザーへエスカレーション。
  片方の単独判断でのコンフリクト解決コミットは禁止。
- 旧記述（Opus委任）は2026-07-16のユーザー指示によりCodex gpt-5.6-solへ交代済み。1トランシェ=1検証スライス=1コミットは不変
- 検証: Fable側で各スライスのゲート再実行（build / viewport / localization / 対象ハーネス）+ ライブブラウザ目視 + Daslightキャプチャとの並置比較
- 全スライス共通ゲート: `tsc --noEmit` / `vite build`（index<500kB）/ `check:viewport`全マトリクス / `check:localization` 100% / `git diff --check`
- 各トランシェ完了時に`RELEASE_STATUS.md`と本書へ実測値で追記（過大評価なし: 「Daslight超え」は同一タスク操作手数計測とキャプチャ並置の証拠が揃うまで主張しない）

## 承認履歴

2026-07-15 ユーザー承認済み: T3契約変更 / T7 protocol変更 / T8契約変更+T2直後への前倒し / Opus委任即時開始。
未解決の承認事項なし。
