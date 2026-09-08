# Value/FX checker repair — 2026-09-09

## Scope

`app/scripts/check-value-effect-generator.mjs` と
`app/scripts/check-fx-color-palettes.mjs` の静的入力処理だけを修正した。
VALUE generator、COLOR MAPPINGSの値域、選択拒否、保存形、既存UIは変更していない。

## Finding and repair

両チェッカーは `ColorEffectEditorPanel.tsx` の複数行文字列をLFとして検索していた。
WindowsのCRLF作業ツリーでは、実装に存在する以下の契約を見失っていた。

- Grid: 2..5 stops
- Graph: 2..10 stops
- Lines: 2..255 stops

各チェッカーが読み込んだColorEffectEditorPanel.tsxだけをCRLFからLFへ正規化し、
既存の厳密な文字列・正規表現assertionを維持した。値域を広げる変更、失敗fixtureの削除、
assertionの弱体化は行っていない。

## Verification

- `node --check app/scripts/check-value-effect-generator.mjs`: PASS
- `node app/scripts/check-value-effect-generator.mjs`: PASS
- `node --check app/scripts/check-fx-color-palettes.mjs`: PASS
- `node app/scripts/check-fx-color-palettes.mjs`: PASS
- `git diff --check`: PASS

製品コード、Rust、IPC、保存schema、物理出力、ASIO/NDIはこの単位で変更していない。
ブラウザ実行体、native build、実機出力、Mac、署名・公開の受入を証明する記録ではない。
