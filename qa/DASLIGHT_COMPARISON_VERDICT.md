# Daslight 5 比較・ソフトウェア完走判定

- 判定日: 2026-07-23
- 対象: Syndocal `83970ea` + T23受入差分（T18-T23全トランシェ）
- 比較相手: 実機 Daslight 5（DASBUILD 25.0905.165.111、実ショー Shinkan2026.dvc ロード状態）
- 検証: FableのDaslight/Syndocal並置証拠に、CodexのT18-T23実装監督、current-head gate、native WebView2、別プロセスArt-Net受信を追加
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
- 2026-07-23 T23 native再受入 — 同じShinkan2026を41灯体/13バンク/30キューとしてimportし、Scene Matrix、Timeline面、動的FX、Scene Live x4/retrigger x1を実操作
- `target/qa/artnet-shinkan-acceptance.json` — 別プロセスArt-Net monitorの20秒証拠（U0、512 bytes、880 frame、changed 880、rejected 0、stream discontinuity 0、max gap 25 ms）

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
| UI見た目のリッチさ | 同等圏（劣位点あり） | native Shinkan再確認 | **同等圏（全面優位は未証明）** | 同一ショー並置のidentity色/キュー組織に加え、current nativeでMatrix/Timeline/Scene Liveを確認。41灯密度の2Dラベル重なりと小型メタ情報は弱点。内蔵3Dは製品境界から除外し、外部Art-Netを正式経路とする。 |
| 操作性 | 未計測 | 17-task current実測 | **主要17タスクは同等以上・製品全体は未計測** | 2026-08-09更新: 15-task CDP contractにSave/Open shortcut gateを加えた17タスクのうち、Daslight同一タスク実測は16件。15件同数、Touch control追加はSyndocalが1 click少なく、scene live modifier一括resetはDaslightに1操作相当なし。Patch、Static programming、FX target適用も最大化Daslightとの同一完了条件で閉じた。 |
| 見やすさ | 同 | 同（構造面） | **同等圏** | identity色、名前/時間/fade、固定インスペクタ、危険表現、型付きlane、一画面契約をnative/5 viewportで確認。暗所・距離・色覚を含むoperator実視認性は未計測。 |
| 機能面 | 部分超過・部分劣位 | software監査PASS | **比較可能（全面同等ではない）** | 7独立FX、Cue transition、Scene Live/Live Mixer、GDTF onboarding、workspace/lock、Art-Net process境界を閉じた。一方、商用catalog規模、standalone hardware、native mobile、実運用実績はDaslightが優位。 |

**総合: 内蔵3Dを外部Art-Netへ置き換えたPC照明ソフトとして、定義済みソフトウェア目標は完走。Daslight 5の全面同等/超越は主張しない。**

## 明確な優位軸（ソフトウェア機能範囲。実運用成熟度の主張ではない）

1. タイムライン統合の幅 — Nレイヤー（Lighting/Audio/Video 型付き）、mute/solo/lock、子タイムライン（Super Scene in Timeline）、automation、rate/window ストレッチ、conform-to-tempo
2. 同期ソースの幅 — Tap / MIDI Clock / MTC / LTC / Ableton Link 相当 + タイムライン音声クリップの実再生・波形
3. 映像/VJ 統合 — File / Camera / Screen Capture / NDI、Preview/Program、プロジェクションマッピング（レンズ/キーストーン/コーナーピン）、照明と映像を貫く共有エフェクトソース、audio-reactive ノードグラフ（Kick/Snare/FFT特徴量）
4. オープン I/O — Art-Net / sACN / Enttec PRO / DMXKing / Open DMX / OSC / WebSocket リモートの構成自由度

## 明確な劣位軸（Codex 21項目の要約 + Fable確認）

1. 内蔵3Dはない。これは欠落を隠すのではなく、Art-Net外部visualizerを正式境界にした設計差。別プロセス受信はPASS、商用visualizer/実LAN描画は外部環境依存
2. SSL 20,000+規模の商用統合catalog/復旧UXに対し、SyndocalはGDTF Share + 検証cache/health/repair/common pack。規模優位は主張しない
3. 操作比較17タスクのうち16件はDaslight同一タスク実測済み（15件同数、1件Syndocalが少ない）。残る1件はDaslightに同等の一括操作なし。Patch、Static programming、FX target適用まで実測済み
4. 対応ハードへのstandalone scene書込み、native mobile app、長期運用/販売support ecosystemはDaslightが優位
5. 実灯体、Art-Net node、MIDI/OSC controller、会場での手数・latency・feedback比較は未実施

## 残る証拠作業（この判定を更新する条件）

1. ~~`check-operation-counts.mjs` を現行UIへ修復し、主要照明操作を再計測~~ → **2026-08-09更新**。15 CDPタスクと2 project shortcutタスクをfail-closedで実行。Daslight同一タスク実測16件は15件同数、Touch control追加はSyndocalが1操作少ない。scene live modifier一括resetはDaslightに同等操作なし。patch/static programming/FX target適用も完了。
2. ~~`.dvc` インポータで Shinkan2026 を読み込み同一ショー並置を成立させる~~ → **2026-07-18 完了**（DVC-1）。GO実発光は `dvc_local_golden_project_triggers_cue_and_renders_dmx`（実Engine・DMXプレビュー非ゼロ断言）とネイティブ実機の Levels 100% 表示で確認。見た目軸は上表のとおり暫定更新。残り: Codex側の並置再判定（画像不可のため要別手段）と、41灯密度ラベルの重なり解消
3. ~~Shinkan nativeから外部Art-Net経路を実証~~ → **2026-07-23完了**。別プロセスで20秒/880 ArtDMX frame、880 payload change、破損0、per-stream sequence discontinuity 0、最大gap 25 ms。same-host物理NIC自己宛の不達はloopback即受信とのA/BからローカルNIC return/bind/firewall経路と切り分けた
4. 実機リグ（実灯体、実DMXノード、実コントローラ）と別PC commercial visualizerでの運用比較
