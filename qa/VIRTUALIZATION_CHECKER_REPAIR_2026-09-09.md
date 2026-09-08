# Virtualization checker repair — 2026-09-09

`app/scripts/check-virtualization.mjs` のCue panel assertionを現行実装へ追従させた。
Cue rowsはauthorityが有効なときだけreconcileし、authority error時は空配列にして
stale rowsを表示しない。checkerはこのfail-closed条件を明示的に要求する。
あわせてWindows CRLF入力をLFへ正規化した。

仮想化の件数、ページ境界、ARIA row count、Chaser/Mappingのbounded rendering契約は
変更していない。製品コード、UI挙動、authority実装は変更していない。

検証:

- `node --check app/scripts/check-virtualization.mjs`: PASS
- `node app/scripts/check-virtualization.mjs`: PASS
- `git diff --check`: PASS

ブラウザ実行体、native build、実機出力、Mac、署名・公開、製品全体の完成は証明しない。
