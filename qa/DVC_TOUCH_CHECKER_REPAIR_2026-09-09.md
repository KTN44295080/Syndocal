# DVC Touch checker repair — 2026-09-09

`app/scripts/check-dvc-touch-feature-preset.mjs` のprotocol入力だけをCRLFからLFへ
正規化した。Windows作業ツリーで `FeaturePreset` の複数行契約を見失っていた
チェッカーの行末依存を除去したもので、製品コード・protocol schema・DVC import・
原子性・権限境界は変更していない。

検証:

- `node --check app/scripts/check-dvc-touch-feature-preset.mjs`: PASS
- `node app/scripts/check-dvc-touch-feature-preset.mjs`: PASS（10 assertions）
- `node app/scripts/check-dvc-dmx-shortcuts.mjs`: PASS（35 assertions）
- `git diff --check`: PASS

この単位は静的契約の入力互換性だけを扱い、native build、物理DVC/MIDI、ASIO/NDI、
Mac、署名・公開、製品全体の完成は証明しない。
