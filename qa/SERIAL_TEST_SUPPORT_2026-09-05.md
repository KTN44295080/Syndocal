# 最適化テストの疑似serial依存を明示化

基準: `codex/syndocal-v1.2` / `94e2f5ed03667ac82fed928241f7456d1ef53110`。

## 原因と変更

engineのunit testはioの疑似SerialPortを使用するが、ioはdebug_assertionsだけで
その実装を公開していた。cargo test --releaseは依存crateのcfg(test)も立たないため、
最適化されたengine testが56件の未解決symbol errorでcompileできなかった。

ioにdefaultでは無効のtest-support featureを追加し、engineのdev-dependencyだけで
選択する。io自身のunit testはcfg(test)で同じ実装を使用する。通常の製品buildの
依存関係には追加しない。テストを除外したりdebug assertionsを強制したりせず、
最適化構成でも同じテストをcompileできるようにする。

疑似portと観測用state/操作列をserial_dmx/test_support.rsへ抽出。
既存の型名とfrom_test_serial_portは維持し、実port開通・worker・S0処理は変更しない。
疑似portはCOM interfaceを開かず、既存のworkerへ注入して動作を検証する。

## 検証記録

Cargo build/testはWINDOWS_NATIVE_BUILD.mdのexact MSVC
14.44.35207 Community・絶対Cargo linker pin・where-firstを各回検証し、
target/qa/recording-atomic-20260905/run-native.mjs経由で実行する。
ログはtarget/qa/serial-test-support-20260905/に保持する。

- `cargo test -p engine --release --locked snapshot_read_tests -- --include-ignored --nocapture --test-threads=1`:
  PASS、4 passed / 0 failed / 0 ignored。前回の56 compile errorsは解消。
  最適化構成で固定snapshot・1000反復×3readを比較し、従来2,738,447µs / 新136µs。
  4096件×1024byte padded group labelを含むsynthetic workloadであり、
  実showのFPS・CPU改善率ではない。poison試験の意図したpanicはjoinして検証済み。
- `cargo tree -p syndocal -e normal,build,features --prefix none`:
  通常製品の依存treeにio test-supportなし。
- `cargo tree -p engine -e dev,features --prefix none`:
  engineのdev依存にはio test-supportあり。workspace resolver2と組み合わせて
  test-only有効化を維持する。--all-featuresは明示的にtest-supportを選ぶため、
  製品の不包含証拠には使用しない。
- 独立review: 抽出前後の処理は説明コメント・旧cfgを除き完全一致。
  module/reexport/constructorのcfg条件一致、製品依存との分離を確認。阻害指摘なし。
- `cargo test -p io --release --locked serial_dmx::tests -- --test-threads=1`:
  PASS、25 passed / 0 failed / 0 ignored。featureを明示せずio自身のcfg(test)で実行。
- `cargo test -p engine --release --locked show_serial_dmx_tests -- --test-threads=1`:
  PASS、21 passed / 0 failed / 0 ignored。疑似port、S0、故障/停止/再開、
  localhost Art-Net mirrorを確認。物理COMや現場のDMX nodeは操作していない。
- `cargo check -p io --release --locked --no-default-features`: PASS。
  テスト用featureなしの通常IOもcompile可能。今回のrelease tests/checkの
  first-party warningは0。前回release testはcompile errors56で不成立、
  今回errors0/warnings0。製品側の前checkpoint warning記録0から増加なし。

serial_dmx.rsは7追加/268削除（純減261行）。移設先はテスト用moduleのみであり、
製品のserial adapterとテストdoubleの責務を分けた。

今回の変更はテストsupportとdev-dependencyの境界。製品releaseの実行経路・UIは
未変更のため、対象Cargo検証と製品feature treeで確認し、native UI QAを繰り返さない。
実機DMX・実show CPU/FPS・writer待機の受入を示すものではない。

## 引継ぎ

snapshotの一般読取・tick構築・深いdelta比較の性能は引き続き別単位。
既存のcheck-viewport-containment.mjs差分は非所有として保全する。
SHA-256: `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`。
