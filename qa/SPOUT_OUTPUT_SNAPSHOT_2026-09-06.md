# Spout送信前のsnapshot読取を限定する

基準: `codex/syndocal-v1.2` / `9766a5d2af83d3d2f0ebca923375399ba057d79a`。

ユーザー確認によりSpout開始、Video BO復旧・照明との分離、Unity映像の滑らかさは
受入済み。次の軽量化単位として、送信前の不要な全量snapshot複製を除去する。

## 対象

Spout workerは60Hzの処理ループでTimelineの再生状態を読み、live/keepaliveの
送信前に出力ペアの一致を確認する。各読取が `engine.snapshot()` 経由で
照明・シーン・Timelineなどを丸ごと複製していた。前景・背景のworker双方で発生する。

既存の `snapshot_read.rs` に再生状態と出力一覧の限定読取を追加する。
同じpublicationのRwLockとpoison時defaultを維持し、キャッシュや別の状態所有者は
追加しない。出力一覧は従来のpublic snapshotと同じ `snapshot.video.outputs` であり、
別のauthored-video投影へ切り替えない。送信前の権限・S0・project identityの検証順序、
pair一致の条件、SDK呼出しや送信頻度は変更しない。

## 検証境界

既存snapshot読取テストで値の一致、返り値の独立性、publication置換、poisonを確認する。
並行publicationで出力ペアが混ざらないことも確認する。

明示実行の `benchmark_show_video_outputs_snapshot` は
`SYNDOCAL_SNAPSHOT_BENCH_PROJECT` で指定した保全済みshowを読取専用で使用する。
エンジンworker・media decoder・出力deviceは起動しない。
同じ保持画像に対し、旧来の全量取得2回と新しい再生bool/出力一覧取得を比較し、
ウォームアップ後5回の交互順序測定を記録する。これはclone/lock取得の単独コストで、
writer競合、実アプリFPS、GPU/Unity cadenceの改善率とは区別する。

一般snapshot読取、tick内snapshot構築、深いdelta比較はこの単位では変更しない。
別所有の `app/scripts/check-viewport-containment.mjs` と元showファイルは保全する。

## 検証結果

独立レビューは最終production差分とテストをACCEPT。同じpublication guardと
public/rendered値、poison時defaultを保つことを確認した。mainの変更は2行の
呼出し置換、Spout workerは1行の置換。読取APIは既存の専用moduleへ追加し、
benchmarkを別ファイルに置いた。一般snapshotや送信ループの別処理は未変更。

MSVC 14.44.35207の絶対linker pinとPATH先頭を確認して実行:

- `cargo test -p engine --release --locked snapshot_read_tests -- --nocapture --test-threads=1`:
  4 passed / 0 failed / 1 ignored。ignoredは既存の無関係なsynthetic benchmark。
- `cargo test -p engine --release --locked benchmark_show_video_outputs_snapshot -- --ignored --nocapture --test-threads=1`:
  1 passed / 0 failed / 0 ignored。

計測入力は保全コピー `DSF2026-before-activation-fix.sdc`、1,239,692bytes、
46灯体、15cue、2映像出力。各roundは再生boolと出力一覧の2読取を2,000回。
旧2回全量clone / 新限定読取の中央値は817,392,400ns / 342,800ns
（約817.39ms / 0.343ms）。全roundはgenerated logに記録した。
読み取り部分の無競合測定であり、実機FPSやGPU性能の改善を示す数字ではない。

ログは `target/qa/spout-output-snapshot-20260906/` の
`narrow-read-tests.log` と `show-read-benchmark.log`。当初の広すぎる
`snapshot_ --include-ignored` 指定はコンパイル中に中止し、テスト実行前に
対象を上記の2コマンドへ限定した。中止した実行を合格に数えない。

- `cargo test -p syndocal --locked show_spout -- --nocapture --test-threads=1`:
  49 passed / 0 failed / 0 ignored。ログ `native-spout-tests.log`。
  実SDKの送信性能測定とは別の、fake transportを含む既存の回帰検証。

上記3コマンドのfirst-party compiler warningは0。

`pnpm --dir app tauri build --no-bundle`: PASS、release 3m36s、Vite 8.81s。
正確なMSVC linker pin/PATH-firstをwrapperで確認。Compiler/Vite warningは
baseline 0 → current 0（delta 0）。App chunkは499.92kBのまま。

このcheckoutの `target/release/syndocal.exe` を起動し、PID76980、
responsive/maximizedのSyndocal main window1件を確認。
SHA256: `BE5CC6D9FD34D1C20F77E5B1C66BB129A694F9EFC925BEA3820A2AAFF83256D6`。
最新backup `backup-1788672323209.json` を別途保全して
`DSF2026-before-output-read.sdc` にexportし、標準CLI project-open経由で読み込んだ。
MCPの読取は46灯体、出力ID1/2・元の名称・3840×2160/1920×1080、停止中のTimeline、
Ready/Standby・ownership errorなしを確認。復元した出力のauthored enabledはtrue、
project loadによりphysical outputはdisarmedであり、今回の受入で再送信はしていない。
ログ: `native-build.log`、`native-launch.json`、`after-runtime.json`。

元Unity showと非所有viewport checkerは既存SHA256と一致。元projectへの上書き、
Unity操作、device出力は行っていない。実アプリでの送信interval/FPS測定は未実施。
次の独立候補は一般snapshot生成時のclone/lock待機とdelta比較の実show計測。
