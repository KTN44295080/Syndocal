# UI Competitive Redesign Plan (Timeline-first)

作成: 2026-07-15 / 計画: Fable（実機比較・多段Workflow分析・敵対検証込み）/ 実装: Opus委任
状態: **承認済み（2026-07-15）** — 実施順:
T1✅ → T2✅ → T9✅ → T10✅ → T8✅ → F1✅ → F3✅ → F2✅ → F4✅ → T4✅（86ca79e）→ F5✅（243aaea）+ T3✅（f5cc523、Fable直接実装・合議マージf64ef96）→ **次: T5/F6以降** → T6 → T7 → F7/F8 → T11 → T12
実装体制（2026-07-16更新）: 実装=Codex gpt-5.6-sol（ローカルCLI、ユーザー指定）/ 計画・検証・コミット=Fable。
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

### T6: VJ Desk再構成（risk: medium / L）
クリップバンク支配的（サムネイル第一のグリッド、12パッド無スクロール表示）。Preview/Programモニタが中央列高さ≥70%。Audio/AutoVJ/Reactive設定はアコーディオンドロワー（既定折りたたみ、localStorage永続）。レイヤー6行常設表示。Out系ボタンのスコープ語彙統一。
SynapseRack式フローティングパネルは一画面契約維持のため不採用（採用するなら別途契約再交渉）。

### T7:（オプション・要明示承認）永続identity色 — protocol変更
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
