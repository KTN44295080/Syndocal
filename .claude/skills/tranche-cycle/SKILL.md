---
name: tranche-cycle
description: Syndocalの計画済みトランシェを完了させる、またはDaslight等との実機UI比較を行う。通常の小さな修正だけでは適用しない。
---

# Syndocal tranche cycle

運用・検証・委任・チェックポイントの正本はルートの `AGENTS.md`。
このskillは別のゲート一覧や承認手順を追加しない。

## 計画済みトランシェ

- 作業順は `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` の現行状態と対象項目で確認する。
  対象仕様・受入条件・必要な証跡を参照し、履歴や全計画を毎回読み込まない。
- 対象範囲、編集所有権、完了境界を明確にする。委任する場合はその範囲、必要な検証、
  互換性や停止条件を渡す。モデル名・固定CLI・共有依存worktreeを前提にしない。
- 既承認の範囲内で実装と必要な検証を完了させる。要求や受入契約の変更が必要なら、
  根拠と具体案を示す。作業順の通常の調整を独立した承認ゲートにしない。
- 完了報告には結果と残る未検証範囲を記す。受入条件やハーネス断言を変えた場合は、
  旧条件・新条件・変更理由を示す。

## 条件付き参照

- 競合アプリの実機比較を行うときだけ [comparison.md](references/comparison.md) を読む。
- viewportハーネスを実行・修正する、またはレイアウト発振・検証停止を調べるときだけ
  [viewport-diagnostics.md](references/viewport-diagnostics.md) を読む。
