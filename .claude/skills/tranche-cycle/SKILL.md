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

## 5. 分析・設計が必要なとき

大きな設計判断（新トランシェ系列、構造変更）は Workflow ツールで
Inventory(並列) → Design(複数レンズ) → Judge → Adversarial Verify を回し、
主張はキャプチャ/コードに対して反証テストしてから計画書に固定し、ユーザー承認を得る。
承認が要るのは: 回帰契約の変更、protocol/.sdc変更、実施順の変更。
