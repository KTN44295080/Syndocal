# PATCH/Repair transaction checker repair — 2026-09-09

`app/scripts/check-patch-transaction-d2.mjs` のsource assertionを、現在の責務分離に
合わせた。公開結果の `ProjectTransactionPublicationUnconfirmedError` /
`ProjectTransactionPublicationIndeterminateError`、hold時のCancel/replay抑止、Stageの
legacy reply変換は `app/src/projectTransactionMutationController.ts` が所有するため、
旧App.tsx参照を共有controller参照へ移した。

App側に残るPATCH/Repairのtyped facade、restart-required lane、ticket保持、中央mutation
command set、backend E/R/H gateのassertionは変更していない。製品コード、権限、承認、
古い結果の拒否、Cancel/Commit実装は変更していない。

検証:

- `node --check app/scripts/check-patch-transaction-d2.mjs`: PASS
- `node app/scripts/check-patch-transaction-d2.mjs`: PASS
- `node app/scripts/check-project-transaction.mjs`: PASS
- `node app/scripts/check-project-transaction-mutation-controller.mjs`: PASS
- `git diff --check`: PASS

native/device/Mac/署名・公開/製品全体の完成はこのchecker修復からは主張しない。
