---
name: tranche-cycle
description: >
  Syndocal改善トランシェの運用サイクル: Codex(gpt-5.6-sol)への実装委任、監督検証ゲート、
  競合アプリ(Daslight等)の実機キャプチャ比較、コミット規約。ユーザーが「次のトランシェ」
  「Tn を実装」「Codexに投げて」「Daslightと比較して」「UI改善を進めて」等と言ったとき、
  qa/UI_REDESIGN_PLAN.md のトランシェを進めるとき、または競合ソフトの画面比較・検証を
  求められたときは必ずこのスキルを使うこと。実装をこのセッションで直接書き始める前に読む。
---

# Syndocal トランシェ運用サイクル

役割分担（ユーザー承認済み・変更にはユーザー指示が必要）:
- **このセッション（監督）**: 計画・調査・比較・検証・目視確認・監査文書・コミット
- **Codex gpt-5.6-sol（実装）**: ローカルCLI経由。Anthropicセッション上限の影響を受けない

ロードマップの正: `qa/UI_REDESIGN_PLAN.md`（T系=UI再設計、F系=照明機能モデル）。
不変条件の正: `RELEASE_STATUS.md`「回帰させない条件」。優先方針: **照明完成がVJより先**。

## 1. Codexへの委任

`Agent(subagent_type: "codex:codex-rescue", run_in_background: true)` で委任する。
プロンプトには必ず次を含める（欠けると事故る）:

1. **タスク仕様**: 対象トランシェの計画書参照 + 具体設計 + 対象ファイル + 制約
   （frontend-only か engine可か、`.sdc` v1互換 = serde default追加のみ、44Hzスレッド不可侵）
2. **自律続行方針**: 承認待ちで止まらない・スコープ外に出ない
3. **検証ループ**: 下記ゲート一覧を順に実行し、失敗したら直して再実行してから次へ
4. **完全性契約**: 全ゲートpassまで完了報告禁止、ハーネス断言の変更は全列挙
5. **構造化報告契約**: 変更ファイル一覧 / ゲート結果verbatim / 再交渉断言(旧vs新vs理由) / 実測値
6. **`git commit`禁止**（コミットは監督検証後にこちらで行う）
7. **Codexに`check:viewport`（フルマトリクス）を実行させない** — サンドボックスのCDP Chromium
   は病的に遅く、T10では検証待ちだけで2時間超を浪費した実績がある。Codexは実装+高速ゲート
   （tsc / build --configLoader runner / localization / 焦点スクリプト）まで。重いハーネスは
   監督側ローカルで実行する。vite buildはサンドボックスで偽FAILするので`--configLoader runner`を指定。

起動後、rescueエージェントがCodexジョブID（`task-xxxx`）を返す。完了検知は:

```bash
# 状態確認（1回）
node "C:/Users/kouty/.claude/plugins/cache/openai-codex/codex/1.0.6/scripts/codex-companion.mjs" status <jobId> --json
# 完了監視（Monitorツール、60秒ポーリング、persistent）
# status が running/starting/queued 以外になったら "TERMINAL: status=..." をemitして break
```

最終報告はジョブの `logFile`（status JSONに記載）の末尾にある。

同一ツリーで同時に2トランシェを走らせない（作業ツリー競合）。分析Workflowはread-onlyなら並行可。

## 2. 監督検証（コミット前に必ず全部）

Codexの報告は信用せず自分で再実行する。Codexサンドボックスは偽FAILを出すことがある
（例: vite configローダーが親ディレクトリ参照を拒否 → こちらの標準コマンドでは通る）。

```bash
cd app
node node_modules/typescript/bin/tsc --noEmit
node node_modules/vite/bin/vite.js build        # main index < 500 kB
npm run check:localization --silent              # 100%必須（新規文字列はJA訳必須）
npm run check:identity-color --silent
npm run check:large-show-ui --silent             # 2000灯windowing
npm run check:viewport --silent                  # フルマトリクス exit 0（重い→バックグラウンド）
npm run check:timeline-viewport --silent
# engine/protocol変更時はさらに: cargo fmt --all -- --check / cargo test -p engine -p syndocal --locked（焦点→全体）
git diff --check
```

**ライブ目視 + JS計測**（スクリーンショットだけで済ませない）:
- `preview_start {name: "syndocal-app"}`（.claude/launch.json、ポート5178固定）
- `http://127.0.0.1:5178/?syndocalViewportFixture=timeline` でfixture注入
  （他fixture: scene-block-hour / scene-block-large / cue-recall 等）
- javascript_toolで実測する: 対象DOMのgetBoundingClientRect、`--identity`等のCSS変数値、
  `document.documentElement.scrollWidth - clientWidth == 0`（containment）、
  旧UI要素のDOM残存ゼロ確認など、そのトランシェの受入基準を数値で裏取りする
- ブラウザresize後にレイアウトが崩れて見えたらdprアーティファクトを疑い、JS計測を正とする

## 3. コミットと文書

- コミットメッセージ: 何を・なぜ + `Implemented by <Codex/Opus>` + supervisor検証内容と実測値
- トランシェ完了時に `qa/UI_REDESIGN_PLAN.md` の状態を更新、まとまりごとに
  `RELEASE_STATUS.md` へ証跡段落を追記
- **過大評価禁止**: 「Daslight超え」等は同一タスク操作手数計測とキャプチャ並置証拠が
  揃うまで主張しない。内部ゲート合格は実装品質の証明であって競合優位の証明ではない

## 4. 競合アプリの実機キャプチャ比較

ツール: `qa/harnesses/capture-window.ps1`（読み取り専用キャプチャ）と
`qa/harnesses/click-window-point.ps1`（ウィンドウ相対クリック）。

```powershell
pwsh -NoProfile -File qa/harnesses/capture-window.ps1 -TitlePattern "Daslight" -OutPath target/qa/ui-comparison/<name>.png
pwsh -NoProfile -File qa/harnesses/click-window-point.ps1 -TitlePattern "Daslight" -RelativeX <x> -RelativeY <y>
```

鉄則:
- **競合アプリでは保存・破壊的操作をしない**。許可されるのは選択・タブ/ビュー切替・ホバーのみ。
  シーンセルのクリックは実行トリガになり得るため避ける（ブロック選択は安全）
- クリック座標は直前キャプチャのビットマップから読む。**セカンドモニタはDPIスケーリングで
  座標がズレる** — 対象ウィンドウをプライマリ（100%スケール）へ移動してから操作する
- 取得物は `target/qa/ui-comparison/` に置き、観察は `PRIMARY_OBSERVATIONS.md` に記録、
  計画に使う主張は必ずキャプチャ実物で裏取りする（伝聞・記憶で書かない）
- 機能の意味論が画面から読めないときは推測せずユーザーに聞く（ユーザーが回答すると明言済み）

## 5. 性能・レイアウト異常の診断ツールキット

- `app/scripts/profile-marker-click.mjs` — V8サンプリングプロファイル（self time上位）。
  `(program)`支配ならネイティブ（style/layout）側、JS関数が上位ならJS側。
- `app/scripts/trace-marker-flip.mjs` — Chromeトレースで `Blink.Layout.UpdateTime` vs
  style recalc を切り分け。SVGテキストは `InlineNode::FindSvgTextChunks` 等で見える。
- 既知の地雷: **ResizeObserver駆動のピクセルviewBox SVG（TimelineOverview等）を
  `auto`グリッド行やコンテンツ駆動サイズの中に置くと、行サイズ⇄viewBox書換の
  レイアウト発振で1操作が数十秒〜数分になる**（T10で実測143秒）。対処は
  ホストの高さ固定 + `contain: size layout` **のみで十分**。
  `scrollbar-gutter: stable`を保険で重ねない — 実際にスクロールしない要素や
  入れ子の各層に付けると1層あたり~10px幅を食い、scene-blockの
  「workspace≥パネル幅97%」契約を割る（F3検証で実測: 三重gutterで20px損失）。
  gutterは「その要素自身が実スクロールし、かつ出現/消失の往復が観測に影響する」
  場合だけ。
- `check-viewport-containment.mjs`のscene-block-largeは失敗時に
  `FAILED CONDITIONS: [...]` を出力する（監督が計装済み）。
- 偽陰性に注意: 存在しない環境変数でシナリオがスキップされ「PASS」に見えることがある。
  疑わしいpassはシナリオが本当に実行されたか（実行時間・出力量）で裏取りする。
- grepは大文字小文字に注意（`setXxx`セッターは`xxx`の検索に掛からない）。

## 6. ローカル検証インフラの運用規律（341分事件の教訓、2026-07-16）

- **番犬なしで長時間ジョブを走らせない。** ローカルのviewportマトリクス/スライスも
  必ずラッパ（ライブログファイル + N分無成長で報告するウォッチドッグ）経由で実行する。
  `| grep`でパイプすると完了まで出力が見えず、ハングと正常が区別不能になる。
- **ハーネスは固定ポート（Vite 5173 / CDP 9227）**。前回runの死に残りが占有していると
  次のrunが古いインスタンスへ半接続して無限awaitに落ちる（CPUゼロ・HTTP応答あり・
  WS確立不可）。ハーネス自体に`failIfPortOccupied`（fail-fast）と Windows
  `taskkill /T /F`ツリーkillを実装済み — 消さないこと。
- **長寿命headless Chromeは多数の重量ナビゲーション後にレンダラのメインスレッドが
  CPUゼロで凍結することがある**（孤立新品ブラウザ6/6パス vs 同一セッション3/3凍結で実証、
  Chrome 150）。ハーネスはフィクスチャフェーズ毎に`recycleBrowser()`（新品ブラウザ+
  新品プロファイル+waitForApp復帰）を挟む — 消さないこと。強制kill後のプロファイル
  再利用はロック残骸で挙動不定になるため、リサイクルは必ず新品mkdtempプロファイル。
- **凍結の検死手順**: ①CPU脈拍（数秒デルタ; 発振=高CPU、待ちボケ/凍結=ほぼゼロ）
  ②ブラウザレベルWS（/json/versionのwebSocketDebuggerUrl）+ Target.attachToTarget
  flatセッションで`Runtime.evaluate 1+1`（タイムアウト=メインスレッド死）
  ③`Page.handleJavaScriptDialog`（ブラウザ側処理なので凍結中でも応答; "No dialog"なら
  ダイアログ説棄却）④スレッドサスペンド検査（PowerShell Threads.WaitReason）。
  ページWS直結は既存クライアントと排他なので使わない。
- **bashラッパのpidはMSYS空間**。taskkillに渡すpidはPowerShellの
  `Get-CimInstance Win32_Process`のCommandLineマッチで取ること。
- 帰属判断に迷ったら**git stashでプレ変更ツリーとのN連A/B**が最速の裁定者
  （クリーン環境の作り直しをrun間に挟むこと）。

## 7. 2レーン並行実装とマージ合議（2026-07-17ユーザー指示）

- 実装は2レーン: Codex（メイン作業ツリー）+ Fable直接実装（worktree `../KDMX-fable-lane`、
  node_modulesはジャンクション共有、フロントエンド専用トランシェ向き）。ドメイン非重複で割当。
- **マージ・コンフリクト解決はFable+Codexの合議必須**: Fableが解析と解決案を作る → コンフリクト
  ハンクと解決案をCodexレビュージョブへ → 両者一致でマージコミット。不一致はユーザーへ。
  単独判断でのコンフリクト解決は禁止（ユーザー明示指示）。

## 8. 分析・設計が必要なとき

大きな設計判断（新トランシェ系列、構造変更）は Workflow ツールで
Inventory(並列) → Design(複数レンズ) → Judge → Adversarial Verify を回し、
主張はキャプチャ/コードに対して反証テストしてから計画書に固定し、ユーザー承認を得る。
承認が要るのは: 回帰契約の変更、protocol/.sdc変更、実施順の変更。
