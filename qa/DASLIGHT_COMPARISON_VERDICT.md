# Daslight 5 実機比較 最終判定（Fable + Codex 二重検証）

- 判定日: 2026-07-18
- 対象: Syndocal HEAD `44f7714`（照明完成度シリーズ T1-T12 / F1-F8 全マージ後）
- 比較相手: 実機 Daslight 5（DASBUILD 25.0905.165.111、実ショー Shinkan2026.dvc ロード状態）
- 検証者: Fable（ネイティブexe実機キャプチャ + Daslight実機キャプチャ + ネット調査）、Codex gpt-5.6-sol（コード監査、job `task-mrpzq2ty-01bycw`、読み取り専用）
- 規律: 過大評価禁止。操作手数計測 + 同一条件並置証拠が揃わない軸で優位を主張しない。

## 証拠

### ネイティブ実機キャプチャ（比較は exe 同士。ブラウザ描画は補助のみ）

Syndocal は `target/debug/syndocal.exe`（2026-07-18 ビルド = シリーズ全部入り）を実際に起動し、
`qa/harnesses/capture-window.ps1` / `click-window-point.ps1` で操作・撮影した。

- `target/qa/ui-comparison/daslight5-current.png` ほか daslight5-*.png — 実機 Daslight + 実ショー（41灯 / 13バンク / 30シーン）
- `target/qa/ui-comparison/syndocal-native-initial.png` — VJ Desk + クリップバンク + ドロワー（T6）
- `target/qa/ui-comparison/syndocal-native-menu*.png` — プロジェクトメニュー全景（Recovery / テンプレート / UIスケール / 言語 / レイアウト）
- `target/qa/ui-comparison/syndocal-native-timeline.png` — Control Timeline（KILLゾーン / cueパッド / RATE・WINDOW / Magnet / Arm Cue / Create Super Scene / 型付き LIGHT・VIDEO レーン）
- `target/qa/ui-comparison/syndocal-native-touch.png` — T11 合成 Touch サーフェス（EDIT/LIVE、Default Desk、SHOW CONTROL / DIMMER / COLOR / POSITION / BOガード / cueパッド）
- `target/qa/ui-comparison/syndocal-native-liveedit.png` — Live Desk（Active/Next cue、2Dステージ、選択、タイムライン同居の一画面）

### ネット調査ソース（Daslight 5 機能基準）

- https://daslight.com/en/daslight5 — 公式機能一覧（7系統FXエンジン: Colour / Chaser / Move / Value / Curve / Mappings / Colour Mappings、Super Scene、Touch、3D）
- https://eu-litterature.n-g.co/Release/daslight_5_manual_en.pdf — 公式マニュアル（シーン segments、バンク、Live modifier 群、SSL ライブラリ）
- Nicolaudie フォーラム / SSL クラウドライブラリ（20,000+ プロファイル規模）

### Codex 監査

全文ログ: `C:\Users\kouty\.claude\plugins\data\codex-inline\state\KDMX-4cf9b1b636c3827e\jobs\task-mrpzq2ty-01bycw.log`
（機能マトリクス13カテゴリ + 劣位21項目列挙。HEAD 44f7714 読み取りのみ、ビルド・編集なし）

## 4軸判定

| 軸 | Fable判定 | Codex判定 | 合成 | 根拠要約 |
|---|---|---|---|---|
| UI見た目のリッチさ | 同等圏（劣位点あり） | 未計測 | **同等圏（暫定・Fable単独目視）** | 2026-07-18 更新: DVC-1 インポータで実ショー Shinkan2026 を両アプリにロードし、初の同一ショー・ネイティブ同士並置が成立（`dvc1-go-levels.png` ほか vs `daslight5-current.png`）。バンク/シーンの identity 色・キュー組織・実発光表示は同等圏。確認された Syndocal 劣位点: (a) 41灯密度で 2D マップのラベルが激しく重なる、(b) 既定 Live Desk の cue パッドが 10/30 ページングで Daslight の常時全マトリクスより一覧性が低い（MATRIX 面には全景あり）、(c) 3D ビューは引き続き欠落。この軸更新は画像を見られない Codex の再判定を経ていない Fable 単独目視のため暫定。 |
| 操作性 | 未計測 | 未計測 | **未計測（比較済み3タスクは同）** | 2026-07-23 T18更新: `check-operation-counts.mjs` を現行UIへ修復し13タスクへ拡張。常設Scene MatrixからTimelineへの配置は1 dragでScene Blockが1件増え、Daslight実測1 dragと同数。layer muteは1 click同士、pane expand+restoreは2操作同士。追加10件はSyndocalのgesture/result回帰だけを固定し、Daslight側は`未計測`。したがって照明操作全体の同等性へは昇格しない。 |
| 見やすさ | 同 | 同（構造面） | **同** | グループ/cue identity色の横断適用、名前・時間・fade付き二段ブロック、固定ガター+固定インスペクタ、KILLゾーンの危険表現、型付き Audio/Light/Video レーン、一画面契約 — Daslight で強みと観察された情報構造は全てネイティブ実機で確認できる。弱点は 8-10px メタデータと小型コントロール密度。暗所・距離・色覚を含む実視認性は未計測。 |
| 機能面 | 劣（部分超過あり） | 劣 | **劣（部分超過あり）** | 下記の優位軸/劣位軸参照。 |

**総合: 現時点で「Daslight 5 を超えた」「同等」とは主張しない。**

## 明確な優位軸（ソフトウェア機能範囲。実運用成熟度の主張ではない）

1. タイムライン統合の幅 — Nレイヤー（Lighting/Audio/Video 型付き）、mute/solo/lock、子タイムライン（Super Scene in Timeline）、automation、rate/window ストレッチ、conform-to-tempo
2. 同期ソースの幅 — Tap / MIDI Clock / MTC / LTC / Ableton Link 相当 + タイムライン音声クリップの実再生・波形
3. 映像/VJ 統合 — File / Camera / Screen Capture / NDI、Preview/Program、プロジェクションマッピング（レンズ/キーストーン/コーナーピン）、照明と映像を貫く共有エフェクトソース、audio-reactive ノードグラフ（Kick/Snare/FFT特徴量）
4. オープン I/O — Art-Net / sACN / Enttec PRO / DMXKing / Open DMX / OSC / WebSocket リモートの構成自由度

## 明確な劣位軸（Codex 21項目の要約 + Fable確認）

1. 3D可視化（Easy View 相当のオペレータ向け 3D viewport）が無い（設計上外部化）
2. SSL 20,000+ 規模の統合灯体ライブラリ/検索UXが無い（GDTF + GDTF Share 経路のみ）
3. 7系統FXの独立body/runtime/editor、同一Effect IDのCue-owned state間fade、GDTF CIE xyY metadataによる3–16 emitter calibrated mixingはT19で実装済み。metadata無し追加emittersは推測せずzero。プリセット量、同一タスク、実灯体/外部Art-Net可視化の受入では引き続き劣るまたは未計測
4. シーン単位 Live modifier 群（speed / phase / size / direction / segment / flash / strobe / solo の即時操作）が不足
5. Syndocal側は13タスクを自動計測するが、Daslight同一タスク比較は3件のみ（シーン配置の旧2 vs 1はT18再計測で1 vs 1へ解消）
6. detachable workspace が Stage / Timeline の2ペイン限定（Daslight は任意ペイン）
7. 対応ハードへのスタンドアロン書き込み、パスワードロック相当が無い
8. ネイティブモバイルリモート、長期運用実績、実機ハードウェア受入で劣る
9. 実 MIDI/OSC コントローラでの手数・レイテンシ・フィードバック比較が未実施

## 残る証拠作業（この判定を更新する条件）

1. ~~`check-operation-counts.mjs` を現行UIへ修復し、シーン配置を再計測~~ → **2026-07-23完了**。13タスクをfail-closedで実行し、比較済み3件は同数。追加10件のDaslight実測と、patch/static programming/FX target適用/native Save-reopenを含む比較拡張は未完。
2. ~~`.dvc` インポータで Shinkan2026 を読み込み同一ショー並置を成立させる~~ → **2026-07-18 完了**（DVC-1）。GO実発光は `dvc_local_golden_project_triggers_cue_and_renders_dmx`（実Engine・DMXプレビュー非ゼロ断言）とネイティブ実機の Levels 100% 表示で確認。見た目軸は上表のとおり暫定更新。残り: Codex側の並置再判定（画像不可のため要別手段）と、41灯密度ラベルの重なり解消
3. 実機リグ（実灯体、実DMXノード、実コントローラ）での運用比較
