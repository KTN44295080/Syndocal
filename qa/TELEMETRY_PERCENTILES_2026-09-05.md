# Snapshotのpercentile重複sortと測定窓の責務分離

基準: `codex/syndocal-v1.2` / `502a884e1ee1d404e63ae563a65b4f4104029aec`。
内部改善。製品alpha.69/保存schema/IPCは変更しない。

## 確認した費用と変更境界

従来のbuild_snapshot_with_touch_surfaceはtick jitter・command queue latency・
command-to-DMX latencyの各窓についてp95とp99を別々に取得し、同じ最大256件の
配列をそれぞれcopy/sortしていた。窓3つでsnapshotごとに最大6回のsort。

測定窓の記録・保持数・読取を専用moduleへ分離し、各窓のsort1回からp95/p99を
取得する。直近256件、nearest-rankの順位、空の窓の0、更新・resetの境界は維持。
総sample数・平均/分散・最大値は既存engine側の別統計として保持する。

snapshot生成の他のcollection cloneやpublication lockは今回の対象外。
この重複sortの除去だけでengine全体のCPUや実showのFPS改善率を主張しない。

## 検証記録

同じ入力の従来アルゴリズムとの比較と、既存telemetry回帰を実行した。
Cargoはqa/WINDOWS_NATIVE_BUILD.mdのexact MSVC Community14.44.35207を使用し、
絶対Cargo linker pin・where-firstを開始前に検証する。
再現helper: target/qa/recording-atomic-20260905/run-native.mjs。
ログ/旧exe: target/qa/telemetry-percentiles-20260905/。

旧exe SHA-256: `03536F139712E10F4848682F18B86E916D6B535E9D91A7C34B10BFB1C1747F83`。

- `cargo test -p engine --release --locked telemetry_percentiles -- --include-ignored --nocapture --test-threads=1`:
  PASS、7 passed / 0 failed / 0 ignored。通常6件と比較benchmark1件を明示実行。
  空/単一/20・100・256件境界、wrap後の直近値、読取の非破壊性、reset/reuse、
  同値/u64最大値、全lengthと複数wrapで旧アルゴリズムとの一致を確認。
- release benchmark: 20,000反復、3窓×256件、入力と出力をblack_box。
  従来sort6回: 88.1644ms、新sort3回: 43.9793ms。
  1反復あたり約4.41→2.20µsの小さい集計処理に対する固定データの単回計測。
  snapshot構築全体・lock待ち・実showのCPU/FPSは測っていない。
- `cargo test -p engine --release --locked <filter> -- --test-threads=1`:
  次の3filterを個別実行し、それぞれ1 passed / 0 failed:
  command_latency_telemetry_tracks_percentiles、reset_telemetry_starts_a_new_measurement_window、
  telemetry_accumulates_tick_jitter_statistics。対象はDMX出力disabled。
- 独立review: 阻害指摘なし。記録3箇所、初期化、読込/resetの両経路、
  既存sample数/max/stddevを照合。新しいヒープ確保・lock・読取時の変更なし。
  rootも安定差分と旧計算を照合した。

engine/lib.rsは30追加/112削除（純減82行）。49行のPercentileWindowへ内部状態を
集約し、テストは別ファイル。UI/IO/AppStateへの逆依存は追加していない。

`pnpm --dir app tauri build --no-bundle`: PASS、Rust release2m50s。
TypeScript/Viteも成功。最終release tests/nativeのfirst-party warnings0、
前checkpoint記録0→今回0（増加0）。Vite chunk advisory0→0。
wrapperが正確なcheckout exeの旧PID69888を停止してbuildした。

新exeは63,665,664 bytes、SHA-256
`2C1B7805CE7E67054E3E234F710991E0FD106B37A9FF295F2CE2D1073532CF5F`。
exact pathで起動したPID73784、Syndocal window1つ、Responding=true。
対象を照合後に最大化し、1920×1032画面とnative「元のサイズに戻す」を確認。
空のUntitled/Timelineの正常描画を確認し、起動したまま残す。
物理出力・長時間の実show・UI上での統計resetの実操作は今回未検証。

## 引継ぎ

一般snapshotのcollection複製・tick内構築・深いdelta比較と実showのwriter待機計測は残件。
別所有check-viewport-containment.mjsの差分を保全する。SHA-256:
`3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`。
