# Syndocal 完成計画書 / Codex 作業指示書

作成日: 2026-07-10
対象: Syndocal — Rust/Tauri/SolidJS 製 DMX 照明 + VJ 統合コントロールアプリ
運用: この文書は「完成(v1.0)」までのロードマップと、Codex/Claude セッションへの標準作業指示を兼ねる。
日々のスライス記録は従来どおり `CLAUDE.md` に追記し、この文書はマイルストーン単位でのみ更新する。

---

## 1. 完成(v1.0)の定義 — Definition of Done

以下がすべて満たされたとき、Syndocal v1.0 とする。

### 機能要件
- [ ] 照明: GDTF/カスタムプロファイルのパッチ → 2D マッピング → キュー/タイムライン/エフェクト → Art-Net / sACN / シリアル DMX 出力が、実機で安定動作する。
- [ ] ビデオ: レイヤー合成 → プロジェクター補正(コーナーピン/レンズ/キーストーン) → ディスプレイ出力が、**ネイティブ wgpu 出力**で 60fps 動作する(CPU プレビューはデバッグ用途に降格)。
- [ ] 動画デコード: FFmpeg CLI 抽出ではなく、**インプロセスのデコードワーカー**(FFmpeg ライブラリまたは HAP)でフレーム供給する。
- [ ] 共有エフェクト: 1 つの LFO/PositionWave ソースが照明と映像の両方を駆動する(実装済み — 回帰させない)。
- [ ] タイムライン: キューイベント + 照明/映像オートメーションが 1 本のタイムラインで同期する(実装済み — 回帰させない)。
- [ ] 外部 I/O: MIDI 入出力、OSC、Web リモートが動作する(実装済み)。NDI 送出は実バインディングで動作する。Spout/Syphon は v1.0 では「実装 or 明示的にロードマップ外」を決定済みであること。
- [ ] プロジェクト: `.sdc` の保存/読込/検証/自動リカバリが完結し、外部ファイル欠損時も自己完結スナップショットで復元できる。

### 品質要件
- [ ] `cargo test --workspace` が green。
- [ ] `pnpm --dir app build`(tsc + vite)が green、メインチャンク < 500 kB。
- [ ] `npm run check:viewport` が green(アプリ本体はスクロールしない一画面デスク)。
- [ ] 44 Hz DMX ティックがテレメトリ予算内(budget report が pass)で、1 時間ソークでドロップ/リークなし。
- [ ] **Tier 1: Windows 10+ / macOS 12+** でネイティブリリースビルド、起動、`.sdc` 保存/読込、Art-Net/sACN、MIDI/OSC、wgpu 映像出力、1画面UIのスモークテストが通る。
- [ ] **Tier 2: Linux (Ubuntu 22.04+)** でビルド・起動し、`.sdc` 保存/読込、Art-Net/sACN、MIDI/OSC、wgpu 映像出力、1画面UIのスモークテストが通る。配布形式とデスクトップ統合の差は既知の制限として記録する。
- [ ] Windows の NSIS/MSI、macOS の `.app`/DMG、Linux の AppImage または `.deb` を生成し、Tier 1 では `.sdc` 関連付けまで確認する。
- [ ] CI の Windows / macOS / Linux マトリクスで Rust ワークスペースとフロントエンドの非実機テストが green。OS固有機能を無効化した共通コアも全OSでコンパイルできる。
- [ ] クラッシュ/強制終了 → 再起動 → リカバリチェックポイントから復元、が手動テストで通る。

### ドキュメント要件
- [ ] README がユーザ向けに完成(機能概要、対応ハード、クイックスタート、スモークフロー)。
- [ ] `samples/README.md` が最新のサンプル群(mini-show / effect presets)を反映。
- [ ] ホットキー一覧が UI 内(またはドキュメント)で参照できる。

---

## 2. 現状評価(2026-07-10 時点)

### 完了済み(回帰させないこと)
- Phase 1 MVP: パッチ → DMX 出力 → キュー → スモークテスト一式。埋め込みサンプル `Load Sample` / `Run Smoke`。
- Phase 2 の大部分: エフェクトプリセット群(pulse/wave/flash/chase/fan/circle/ball/random/perlin/shared)、エフェクト複製/編集/ドラフト再利用、混合照明+映像ターゲット、タイムラインオートメーション(ドラッグ/リサイズ/Playhead/Align)、2D マッピングのホットキー群、sACN/シリアル出力 UI、Node Graph の clock-sync。
- フロントエンド分割: App.tsx を約 19,200 行 → 現在約 14,000 行まで削減。85+ スライスで `app/src/components/` と純粋ヘルパーモジュールへ抽出済み。
- ビューポート回帰ハーネス: `app/scripts/check-viewport-containment.mjs`(`npm run check:viewport`)。

### 進行中(未コミットの作業ツリー)
- `app/src/dmxAddressing.ts` + `app/scripts/check-dmx-addressing-helpers.mjs`: DMX アドレス占有/空き判定の純粋ヘルパー抽出。
- `app/src/projectRecoveryStorage.ts` + `app/scripts/check-project-storage-helpers.mjs`: localStorage ベースのプロジェクト自動リカバリチェックポイント(`syndocal.projectRecovery.v1`)。
- App.tsx / PatchFixtureFormPanel / WorkspaceChrome / styles.css / main.rs への対応配線。

**→ 最初のアクション: この進行中スライスを検証して完結させ、コミットチェックポイントを切ること(§4 M0 参照)。**

### 未着手(v1.0 の主要残作業)
1. ネイティブ wgpu ビデオ出力(現在は CPU リファレンスコンポジタ + プレビューウィンドウ)。
2. インプロセス FFmpeg/HAP デコードワーカー(現在は FFmpeg CLI フレーム抽出 + 静止画デコード)。
3. NDI 実バインディング(現在はプレースホルダルート + 診断表示のみ)。Spout/Syphon の採否決定。
4. Enttec Open DMX のハードウェアタイミング改善(FTDI ブレークタイミング)。
5. リリースエンジニアリング(インストーラ、署名、バージョニング、QA パス)。
6. ドキュメント一括更新(意図的に後回しにしてきた分)。

---

## 3. マイルストーン計画

依存関係順。各マイルストーンの末尾に「検証ゲート」を置き、ゲートを通過するまで次へ進まない。

### M0 — 作業ツリーの収束(即時、〜1 セッション)
ダーティな進行中スライスを完結させ、安全なチェックポイントを作る。
- [x] DMX アドレッシングヘルパーのスライス完了: `node app/scripts/check-dmx-addressing-helpers.mjs` green、App.tsx/PatchFixtureFormPanel の配線確認。
- [x] プロジェクトリカバリのスライス完了: `node app/scripts/check-project-storage-helpers.mjs` green、保存/復元/破棄の UI フロー手動確認。
- [x] `pnpm --dir app build` + `npm run check:viewport` green。
- [x] `cargo test -p syndocal project_` green(main.rs を触っているため)。
- [x] **意味のあるコミットメッセージでコミット**(「s」のような無意味メッセージは今後禁止 — §6 参照)。
- 検証ゲート: 上記すべて + `git status` がクリーン。

### M1 — Phase 2 完了宣言 + 安定化(1〜2 週)
Phase 2(エフェクト/タイムライン/マッピングの操作性)を「完了」と宣言できる状態にする。
- [x] エフェクト UI の残課題洗い出し: 標準の照明+映像ドラフトは完全復元。複数映像バインディングを持つ外部プリセットは先頭を編集し、追加分がエンジン上に残ることを Use Draft 時に明示する v1.0 制限とした。
- [x] 2D マッピングのホットキー一覧を UI 内ヘルプ(`?` キーまたはツールレール)として表示。
- [x] Stage 2D 境界/定数を `stageGeometry.ts` に統一(`StageWorldBounds` と `stageViewBoxSize` の重複解消)。
- [ ] App.tsx 継続削減: 目標 10,000 行未満。残る大物(Setup Mapping ビューポート/設定シェル、Output/Telemetry 診断のオーケストレーション)を各 1 スライスで抽出。新規抽出モジュールは `vite.config.ts` の `manualChunks` に振り分けてメインチャンクを 450 kB 以下に維持。
- [ ] エラー/ステータス表示の統一: 現在パネルごとにばらつくメッセージ表示を、単一のステータス行 or トースト規約に揃える(一画面制約内で)。
- 検証ゲート: `cargo test --workspace`、`pnpm --dir app build`、`npm run check:viewport`、手動スモーク(Run Smoke + Setup/Control/Touch 一巡)。

### M2 — UI/UX 完成パス(1〜2 週)
「DasLight 的な一画面デスク」としての完成度を上げる。機能追加ではなく磨き込み。
- [x] 全ワークスペース(Setup 5 タブ / Control / Touch)を 1366x768 と 2048x1129 で目視レビューし、詰まり/はみ出し/読めない表示を列挙 → 修正。共通クローム刷新後、1280x720 も含む全 viewport マトリクスで document/app scroll 0 を確認済み。
- [x] キーボード操作の一貫性監査: 編集ターゲットガード、フォーカスリング、Tab 順。Setup→Control のTab順、可視アウトライン、Setup/Mapping/Control入力編集中のショートカット抑止を3解像度で自動検証。
- [x] Touch ワークスペースのポインタエミュレーション検証: 可視ヒットターゲット最小 40px、Flash は pointer-down 中のみ active で pointer-up 時に復元。1280x720 / 1366x768 / 2048x1129 で自動検証済み。
- [x] 空状態(fixture 0 台、video layer 0 枚、cue 0 個)は、Setup/Control/Touch の各主要パネルで次に開く画面と操作を表示する。
- [x] 用語統一パス: Fixture/Group/Cue/Video Output/Video Layer/Projection Surface/Cue Point/DMX Output を全 UI で統一し、`check:terminology` で旧表記の再侵入を検出。
- [x] 破壊的操作(Remove Fixture/Cue/Output/Effect)は、v1.0 では共通のネイティブ確認ダイアログ + Undo 不可の明示に統一。Undo は v1.x の拡張候補とする。
- 検証ゲート: `npm run check:viewport` + 上記チェックリストを CLAUDE.md に結果記録。
- **M2 完了 (2026-07-11)**: 全チェック項目と検証ゲートを通過。以後のUI変更は同じ一画面・用語・キーボード・Touch回帰ゲートを維持する。

### M3 — ネイティブビデオ出力(2〜4 週、最大の技術リスク)
CPU プレビューを wgpu 実出力に置き換える。**照明エンジンと分離したまま進める**(DMX 44Hz を巻き込まない)。
- [x] 段階 1: `crates/video` に wgpu コンポジタを追加し、既存 CPU リファレンスコンポジタと**同一入力 → 同一出力のゴールデンテスト**を作る(RGBA/BGRA/DXT1/DXT5)。(2026-07-11)
  - [x] M3.1a: wgpu 25.0.2 のオフスクリーン計算パイプラインを追加。RGBA/BGRA/DXT1/DXT5、Normal/Add/Multiply/Screen、opacity が CPU 参照と完全一致するゴールデンテストを実GPUで通す。変形/色補正/FXは未対応時に明示エラーとする。(2026-07-11)
  - [x] M3.1b-1: transform/crop、異解像度ソース、brightness/contrast/hue/saturation/gamma をGPUパスへ移す。座標変換は完全一致、超越関数を使う色補正はチャンネル誤差1/255以内でCPUゴールデンと一致。(2026-07-11)
  - [x] M3.1b-2a: pixelate/blur/glow/edge/color-keyをGPUパスへ移す。整数サンプリングは完全一致、浮動小数点を含むFXはチャンネル誤差1/255以内でCPUゴールデンと一致。(2026-07-11)
  - [x] M3.1b-2b: 合成後の出力マッピング(アスペクト/変形/レンズ/キーストーン/コーナーピン)を独立GPUパスへ移し、CPU参照と完全一致させる。(2026-07-11)
- 段階 2: 出力ウィンドウ(Tauri/winit)に wgpu サーフェスを張り、静止画 → 単一動画レイヤー → 多レイヤー合成 → 補正(コーナーピン/レンズ/キーストーン)の順に段階的に載せ替える。各段階でスライスを切る。
  - [x] M3.2a: `raw-window-handle`を介したクロスプラットフォーム`GpuSurfacePresenter`を追加し、TauriのWebViewなしWindowへVSync付きwgpu Surfaceを生成。補正済みTest Patternをネイティブ提示し、Windows実画面で非空表示・終了を確認。(2026-07-11)
  - [x] M3.2b: 静止画/単一ライブレイヤーを出力ごとの60HzループからネイティブSurfaceへ継続提示し、現行Canvas/IPC経路をReference用途へ降格する。GO後のフレーム変化、個別Window終了、本体終了後のプロセス解放をWindows実画面で確認。(2026-07-11)
  - [x] M3.2c: 多レイヤーGPU合成バッファからSurfaceへCPU readbackなしで直接提示し、リサイズ・複数出力・fullscreenを検証する。(2026-07-11)
    - [x] M3.2c-1: 出力プランとComposition順の選択済みレイヤーフレームを返す`PreparedVideoOutput`境界を追加。ブラックアウト/無効出力はデコードせず空集合を返す。(2026-07-11)
    - [x] M3.2c-2: Prepared入力を同一deviceの合成/補正Storage BufferからSurfaceへ直結し、CPU合成/readback/Canvasを通さない。8x8の2次元compute dispatchでHD/4K時も各次元のGPU上限を超えないことをテストし、Windows実画面で補正済みフレームとGO後の連続更新を確認。(2026-07-11)
    - [x] M3.2c-3: フレームごとのGPUバッファ生成を再利用プールへ置換し、リサイズ・複数出力・fullscreen・1080p60フレーム時間を検証してM3.2cを完了する。(2026-07-11)
- 段階 3: インプロセスデコードワーカー。D1 は (b) HAP 優先 → (a) libav 後続で確定。一般コーデック移行中のみ CLI を互換フォールバックとして残す。
  - [x] M3.3a: 純RustのMOV/MP4デマルチプレクサ + HAPフレームパーサを既定デコーダへ配線し、HAP BC1/BC3とHAP Q YCoCgをFFmpegプロセスなしで供給する。(2026-07-11)
  - [x] M3.3b: HAPのBC圧縮データをCPU RGBA展開せずwgpuテクスチャへ直接アップロードし、HAP Q変換をGPUパスへ移す。(2026-07-11)
    - v1.0 の対象は HAP (BC1)、HAP Alpha (BC3)、HAP Q (YCoCg BC3)。HAP Q Alpha と HAP R/BC7 は素材・仕様・wgpu機能の追加検証が必要なため v1.x 候補とし、未対応形式はCPU展開へ黙ってフォールバックせず明示エラーにする。
    - Windows実画面で640x360の再現可能なHAP Q QA素材をネイティブ出力し、補正済み非空フレームを確認。圧縮アップロード数は1394から2163へ増加し、GPU割当は`1+2`のまま再利用された。
  - [x] M3.3c: H.264/H.265/ProResをインプロセスlibavワーカーへ移し、FFmpeg CLIフレーム抽出をReference/診断用途へ降格する。(2026-07-11)
    - `syndocal`の既定featureでlibavを有効化。Windows/macOS/Linux CIにFFmpeg/Clang開発環境を追加し、共有ライブラリの配布物同梱は`bundle.active`を有効化するM6で確定する。
- [x] 段階 4: 既存のフレームキュー/デコード診断/テレメトリを新パスに接続。CPU プレビューは「Preview (Reference)」として残す。(2026-07-11)
  - Mixerの固定出力デッキに解像度・平均フレーム時間・120サンプル後の60fps判定を1行表示し、詳細診断は既存ツールチップに維持。一画面デッキの縦スクロールは増やしていない。
- [x] 検証ゲート: `cargo test -p video --features libav --locked` 89/89、ネイティブ出力メトリクス2/2、DMX予算判定2/2、`pnpm --dir app build`、全viewportマトリクス、Windowsリリースビルドがgreen。Windows実GPUでHAP Q 3レイヤー(Normal/Add/Screen)を出力し、高DPI物理解像度2880x1620で平均0.8ms・`60 PASS`、非空補正済みフレームを確認。(2026-07-11)
  - 同時DMX比較は映像あり514サンプルでjitter p99 1.597ms、映像なし基準でも約1.65msで、ネイティブ映像による悪化はなし。Windows上の厳格な1ms目標そのものは未達のため、リアルタイムスレッド/タイマ調整をM5の残課題とする。
- **M3 完了 (2026-07-11)**: ネイティブwgpu出力、HAP優先+libavインプロセスデコード、圧縮テクスチャ直接アップロード、固定バッファ、UI診断、Windows実GPU性能ゲートまで完了。macOS/Linuxのネイティブ実行証跡とFFmpeg共有ライブラリ同梱はM6が所有する。

### M4 — 外部 I/O 仕上げ(1〜2 週、M3 と並行可)
- [x] NDI: `crates/io` に NDI SDK バインディング(feature flag `ndi` でビルド切替)。既存のプレースホルダルート/Blocked 表示をそのまま実配線に昇格。(2026-07-12)
  - `grafton-ndi` 0.11.0をRust 1.86互換で固定。受信は専用探索/キャプチャワーカー + 最新2フレーム、送信はCompositionの60Hz参照合成ワーカー。NDI入力は通常動画/HAPと同じFrameDecoder境界へ接続した。
  - Windows NDI 6 SDKでfeatureビルド、I/O 67件、Tauri 148件、ローカル送信→探索→RGBA受信ループバックを確認。macOS/Linux SDKリンクはM6のホスト別リリースゲートで確認する。
- [x] 決定事項 D2: v1.0は共通のDisplay + NDIを対象とする。D3D11/Metal固有のSpout/Syphonはv1.1候補とし、v1.0 UIでは診断付きUnavailableを維持する。(2026-07-12)
- [x] Enttec Open DMX: FTDI ブレークタイミングの改善(専用送信スレッド + 高精度タイマ)。PRO/DMXKing 推奨の UI ヒントは維持。(2026-07-12)
  - エンジン44Hz経路はbounded latest-frame mailboxへの非ブロッキングpublishだけを行い、break/MAB/513-byte writeは専用ワーカーへ分離。波形の実機測定は下記テストマトリクス項目に残す。
- [ ] 実機テストマトリクス作成: 手持ちのノード/インターフェースで Art-Net、sACN(マルチキャスト)、シリアルを各 1 回以上実測し、結果を記録。
  - `qa/M4_IO_VALIDATION.md` に自動/ローカル/物理を分離したマトリクスと再現コマンドを作成。Art-Net/sACN/Serialの物理機器測定は外部依存として未完了のまま明示する。(2026-07-12)
- [x] プラットフォーム境界: Spout(Windows)、Syphon(macOS)、シリアル/優先度制御を `cfg` + feature の背後に隔離し、利用不能な機能は診断付きで無効化する。未導入SDKや未対応デバイスがアプリ起動を妨げない。(2026-07-12)
  - NDI SDKは`ndi` feature、libavは`libav` feature。Spout/Syphonはv1.0でNotBuilt/UnsupportedPlatform診断を維持し、既定ビルドはSDK非依存。OS別コンパイル/配布証跡はM6 CIゲートが所有する。
- 検証ゲート: `cargo test -p io`、実機 or ループバック計測記録、feature flag なしビルドが従来どおり green。

### M5 — 信頼性・パフォーマンス(1 週)
- [ ] 1 時間ソークテスト: mini-show + エフェクト複数 + 動画 1 レイヤー再生で、メモリ/テレメトリ/フレームドロップを記録するスクリプト or 手順を整備。
- [ ] プロジェクトリカバリ(M0 の機能)をクラッシュ注入で検証: プロセス kill → 再起動 → 復元。
- [x] 大規模プロジェクト負荷: フィクスチャ 200 台 / 8 ユニバース / キュー 100 個で UI 応答と DMX 予算を確認。(2026-07-12)
  - Engine再現テストは各キューが200灯体を持つ合計20,000ターゲットをロードし、100番キュー発火、8 x 512 DMX preview、queue failure 0、drain limit hit 0を確認。Windows debug test全体は0.06秒。UIの同規模viewport計測はM5ソークスクリプト側で扱う。
- [x] `.sdc` 後方互換: v1は未知フィールドを許容し、追加済みの省略可能フィールドは`serde(default)`で補完する。将来versionは暗黙移行せず明示拒否し、実サンプルと最小旧形式の回帰テストで固定。
- 検証ゲート: ソーク記録、`engine_telemetry_budget_report*` green、負荷時 budget pass。

### M6 — リリース準備(1 週)
- [ ] バージョニング確定(v1.0.0)、`tauri.conf.json` / `package.json` / Cargo メタデータ整合。
- [ ] GitHub Actions または同等CIに `windows-latest` / `macos-latest` / `ubuntu-22.04` のマトリクスを用意し、Rustテスト、フロントエンドビルド、Tauriコンパイルを継続検証する。
- [ ] Windows NSIS/MSI、macOS `.app`/DMG、Linux AppImageまたは`.deb`を生成し、製品名・`.sdc`関連付け・アイコン・発行者名(Seraf()のKTN)を確認する。
- [ ] コード署名/公証の要否決定(D3)。未署名ならWindows SmartScreenとmacOS Gatekeeperの手順をREADMEに明記する。
- [ ] ドキュメント一括更新: README(ユーザ向け)、samples/README.md、ホットキー一覧、既知の制限(Spout/Syphon 等)。
- [ ] 最終 QA パス: 本計画書 §1 の Definition of Done を上から全チェック。
- 検証ゲート: クリーンな Windows / macOS 環境でインストーラから起動 → Run Smoke green。Linux は Ubuntu 22.04 のクリーン環境でパッケージ起動 → Run Smoke green。

---

## 4. 見積りサマリー

| マイルストーン | 内容 | 目安 |
|---|---|---|
| M0 | 作業ツリー収束 | 即時(1 セッション) |
| M1 | Phase 2 完了 + 安定化 | 1〜2 週 |
| M2 | UI/UX 磨き込み | 1〜2 週 |
| M3 | wgpu 出力 + インプロセスデコード | 2〜4 週 |
| M4 | NDI / Open DMX / 実機 | 1〜2 週(M3 と並行可) |
| M5 | 信頼性・性能 | 1 週 |
| M6 | リリース | 1 週 |

合計目安: 6〜10 週(1 日 1〜3 スライスのペース想定)。クリティカルパスは M3。

---

## 5. 要決定事項(ユーザ判断待ちリスト)

- **D1 — 動画デコード方式**: (a) libav バインディング / (b) HAP 優先 / (c) CLI 据え置き最適化。推奨: (b) を先行し、H.264 等は (a) を後続スライスで。
- **D2 — Spout/Syphon の v1.0 スコープ**: 決定済み(2026-07-12)。v1.0はDisplay + NDI。Spout/Syphonはv1.1候補。
- **D3 — コード署名**: 証明書取得の有無。
- **D4 — UI 言語**: 現在英語 UI。日本語化(i18n)を v1.0 に含めるか。含めるなら M2 で文字列外出しだけ先行しておくこと。
- **D5 — Undo システム**: 破壊的操作の Undo を v1.0 に入れるか、確認ダイアログ統一で代替するか。推奨: v1.0 は確認統一、Undo は v1.1。

---

## 6. Codex への標準作業指示(全セッション共通)

### スライス規律
1. **1 スライス = 1 境界 + 1 検証**。Engine → `cargo test -p engine`、Tauri/検証/プロジェクトファイル → `cargo test -p syndocal`、フロントエンド → `pnpm --dir app build`(+ UI/CSS 変更時は `npm run check:viewport`)、GDTF → `cargo test -p gdtf`。
2. スライス完了ごとに `CLAUDE.md` に日付付きで記録(従来形式を踏襲)。
3. **コミット規約(新規)**: スライス完了 = コミット。メッセージは `<領域>: <変更内容>` 形式(例: `mapping: extract viewport shell into component`)。「s」のような無意味メッセージは禁止。M0 以降、未コミットの巨大ダーティツリーを再び作らない。

### ガードレール(違反 = 回帰)
- アプリ本体(window/document/.app)をスクロールさせない。長いリストはパネル内スクロールのみ。UI/CSS を触ったら必ず `npm run check:viewport`。
- `App.tsx` を成長させない。追加 UI は最初からコンポーネントへ。抽出前に既存 `app/src/*.ts`(特に `uiModes.ts` / `videoOutputMapping.ts` / `videoLayerDefaults.ts`)を grep して重複モジュールを作らない。
- メインチャンク 500 kB 未満を維持。新しい重いパネルは `manualChunks` か lazy-load へ。
- 製品名 Syndocal / 拡張子 `.sdc` / 開発者名 Seraf()のKTN を維持。旧名エイリアスをユーザ向けファイルに出さない。
- 共通コードからOS固有APIを直接呼ばない。Windows/macOS/Linux差分は専用モジュール、`cfg`、feature flagで隔離し、非対応機能は起動失敗ではなく明示的なUnavailable状態にする。
- 3D ビジュアライザをメイン UI に戻さない(`crates/visualizer` はデータ境界のまま)。
- README/docs の更新は M6 まで意図的にバッチする(従来方針の継続)。ただし CLAUDE.md への記録は毎スライス必須。
- `pnpm --dir app build` が node_modules パージを対話で求めた場合は答えず、`node node_modules/typescript/bin/tsc --noEmit` + `node node_modules/vite/bin/vite.js build` の直接実行で代替。
- 広範囲な変更をリバートしない。ダーティツリーは意図的な状態。

### ビルド/検証コマンド早見
```powershell
# フロント(軽量)
pnpm --dir app build
npm --prefix app run check:viewport

# Rust(対象を絞る)
cargo test -p engine <name>
cargo test -p syndocal <name>
cargo test -p io / -p gdtf / -p video

# マイルストーン時のみ
cargo test --workspace
pnpm --dir app tauri build --debug
pnpm --dir app tauri build   # リリースチェックポイントのみ
```

---

## 7. リスクと対策

| リスク | 影響 | 対策 |
|---|---|---|
| wgpu 出力(M3)が想定より難航 | クリティカルパス遅延 | CPU/wgpu ゴールデンテストで段階移行。最悪 v1.0 は「wgpu 出力 + CLI デコード」で出荷し、インプロセスデコードを v1.1 へ |
| NDI SDK ライセンス/配布制約 | M4 遅延 | feature flag で分離済みの設計を維持。動的ロード方式も検討 |
| App.tsx 抽出中の JSX 破壊(過去に発生) | ビルド赤 | スライス毎の `git diff --check` + build。抽出は 1 パネルずつ |
| 実機不足(DMX ノード/プロジェクター) | M4/M5 検証穴 | ループバックテスト網を維持しつつ、実機テスト項目を明示的に「未検証」記録 |
| PowerShell 上書き事故の再発(App.tsx 消失の前歴) | 作業消失 | M0 以降のコミット規律 + プロジェクトリカバリ機能自体の完成 |

---

## 8. 次の一手(このまま実行してよい順)

1. **M0**: 現在の未コミットスライス(dmxAddressing / projectRecoveryStorage)を検証 → 完結 → コミット。
2. D1〜D5 の決定をユーザに確認(特に D1 デコード方式と D4 UI 言語は M2/M3 の内容に影響)。
3. M1 の App.tsx 残り大物抽出(Setup Mapping ビューポートシェル → Output/Telemetry オーケストレーション)から着手。
