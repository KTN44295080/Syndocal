# Cue Effect Recall checker repair — 2026-09-09

## Finding

`app/scripts/check-cue-effect-recall.mjs` はUndoの実装名と終端を旧形の
`const undoProject = async ... const redoProject = async` として要求していた。
現行Appは、準備ガードとbusy/focus確認を `navigateProjectHistory` に置き、実行本体を
`performUndoProject` / `performRedoProject`、公開入口を薄いラッパーに分けている。
その責務分離後も旧checkerがApp全体の巨大source regexで停止していた。

## Repair

App.tsxの読み込みをCRLFからLFへ正規化し、Undo契約のregex範囲を現行の
`performUndoProject` から `performRedoProject` 直前までに合わせた。
範囲内で以下の既存assertionは維持している。

- exact project epoch / undo entry / checkpoint ticket validation
- incomplete history rejection and authority refresh
- `undo_project_transaction` owner binding
- authoritative rollback bundle application and stale-result rejection
- authoritative history publication and failure-side authority polling

製品コード、Undo/Redoの挙動、履歴schema、権限条件は変更していない。

## Verification

- `node --check app/scripts/check-cue-effect-recall.mjs`: PASS
- `node app/scripts/check-cue-effect-recall.mjs`: PASS
- `node app/scripts/check-cue-live-modifier.mjs`: PASS
- `node app/scripts/check-effect-visualization.mjs`: PASS
- `git diff --check`: PASS

この単位は静的checkerの責務追従だけを扱い、native build、実機、物理出力、ASIO/NDI、
Mac、署名・公開、製品全体の完成を証明しない。
