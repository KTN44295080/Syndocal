# 小さいruntime読取の全量snapshot複製を除去

基準: `codex/syndocal-v1.2` / `feec8b5fb57ab440d2e27377700ada06c80e10d4`。
製品は内部検証用alpha.69のまま。保存schema/IPCの変更なし。

## 根拠と責務

transport generation/authority、Follow summary、clip/transition runtimeは、
EngineHandle::snapshotで全projectをcloneしてから小さいfieldだけ取り出していた。
clip/transitionはpollとcommand後のpublication確認で使うため、無関係なcue、fixture、
media、authored video等の複製も繰り返す。前回のsingle-flightは要求の重なりを抑えるが、
各要求の全量複製費用は残っていた。

専用snapshot_read moduleへ読取を分離し、同じ公開snapshotのread lockから必要な
fieldだけ取得する。epoch/generationは1つのguardから取得し、既存の世代比較、
ACK/publication、command ownership順序は維持する。

既存snapshot経由のpoison時defaultは保持する。try_snapshotのpoison回復方式や、
一般snapshotのauthored_video除去、tick側構築・公開・fence順序は変更しない。
これは既存failure policyを新たに推奨する意味ではなく、今回の変更境界である。

## 検証

専用getterの値一致・更新・独立所有・poisonと、既存authoritative runtimeの回帰を確認した。
全量clone対必要field読取の計測は固定入力のsynthetic workloadであり、
実showのFPS/CPU/lock待機時間や物理出力の受入とは区別する。

- `cargo test -p engine --locked snapshot_read_tests -- --test-threads=1 --nocapture`:
  3 passed / 0 failed / 1 ignored（明示計測用）。初回fixtureのnewtype指定漏れ5件を
  修正後に成功。poison試験の意図したthread panicはjoinして検証している。
- `cargo test -p engine --locked benchmark_narrow_snapshot_readers -- --ignored --nocapture --test-threads=1`:
  1 passed。unoptimized + debuginfo構成で1000反復×3read、無関係な4096件の
  1024byte padded group labelを含む固定snapshot。従来3,149,597µs / 新334µs。
  同じデータで全量clone3000回を必要fieldのclone3000回へ置換した比較。
  実show由来のfixtureではなく、release性能や一般的な改善率を主張しない。
- 同benchmarkの`--release`試行は既存engineテストのコンパイルでFAIL（56 errors）。
  例: ioのOpenDmxTestSerialPortがdebug_assertions限定なのにrelease側engineテストが参照。
  今回変更していないテスト構成の残件。debug assertionやwarningを無効化して隠さない。
  続行でこの構成不備を修正し、最適化構成で実行済み:
  [テストsupport分離・release計測](SERIAL_TEST_SUPPORT_2026-09-05.md)。
- `node app/scripts/check-video-runtime-polling.mjs`: PASS、clip/transition各1進行中read。
- `node app/scripts/check-timeline-follow-runtime.mjs`: 初回CRLF sourceにLF固定regexが失敗。
  読込だけCRLF→LFへ正規化してPASS。既存assertionは変更していない。
- 独立安定差分review: 阻害指摘なし。rootも変更したread lock、default、
  全runtime fieldとmainの4接続を確認。fixture型修正はnewtype constructorのみ。
- `cargo test -p syndocal --locked <filter> -- --test-threads=1` は次の3filterを
  個別実行し各1 passed / 0 failed:
  `video_clip_slot_c2_crossfade_polling_advances_generation_and_retry_returns_current_truth`、
  `video_transition_bus_c3_catalog_runtime_retry_release_and_read_are_authoritative`、
  `timeline_follow_runtime_abort_terminal_is_exact_and_history_free`。
  小さいgetterを使う実adapterの世代更新・retry・release・Abort境界を確認した。

Cargoは全て`target/qa/recording-atomic-20260905/run-native.mjs`経由。
MSVC Community 14.44.35207の絶対Cargo linker pinとwhere-firstを各開始前に照合。
ログは`target/qa/snapshot-read-20260905/`。旧exeはsyndocal-before-snapshot-read.exe、
SHA-256 `D2321A10D4B780D381F4B0BFD59C8430B99312BD012D9ED73D6340AB246521E1`。

`pnpm --dir app tauri build --no-bundle`: PASS、Rust release 2m22s。
beforeBuildのTypeScript/Viteも成功。実行した最終test/native構成のfirst-party warning 0、
前checkpoint記録0→今回0（増加0）。Vite chunk advisoryも0→0。
release test構成の失敗は上記に別記し、合格したものに数えない。

exact checkout exeの旧PID956をwrapperで照合・停止後にbuild。
新exeは63,667,712 bytes、SHA-256
`03536F139712E10F4848682F18B86E916D6B535E9D91A7C34B10BFB1C1747F83`。
同pathで起動したPID69888、Syndocal window1、Responding=true。
1920×1032/native「元のサイズに戻す」で最大化を確認し、Video→Timelineの実クリック、
空のTimelineとFollow待機の描画を確認。Untitled/Timelineで起動したまま残す。
実show/物理出力や高負荷状態のnative性能受入は未実施。

Engine libは5追加/34削除（純減29行）、読取は58行の専用moduleへ集約し、テストも別ファイル。
Tauri/AppStateへの新規逆依存やruntime状態の別cacheは追加していない。

## 残件と引継ぎ

tick内のwrite guard下snapshot構築、authored/rendered videoの重複保持、
深いdelta比較、代表showでのwriter待機計測は別単位として残す。
publish順序と音声fenceを壊す一括lock移動は行わない。
別所有のcheck-viewport-containment.mjsは保全する（SHA-256
`3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`）。
