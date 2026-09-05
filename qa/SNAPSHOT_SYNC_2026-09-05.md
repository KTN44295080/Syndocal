# Snapshot同期の分離と複数windowでの全量応答削減

基準: `codex/syndocal-v1.2` / `f3008782862165eecc93ec9c248d1b30c2f96c48`。
内部の改善・検証用チェックポイント。製品/保存schema/IPCのversionは変更しない。
同じalpha.69表記のローカル実行物はSHA-256で区別し、旧artifactを保全する。

## 問題と修正境界

変更前は全windowで1つのlast snapshot/revisionを共有していた。
Aがr1、Bがr2を取得すると、Aの次要求r1は最新r2と一致せずfullになり、
Bも同じ理由でfullになる。この交互実行が続くとdeltaを使えない。

今回の同期serviceは直近4件のimmutable snapshotをrevisionで引き、
要求元が持つ正確なsnapshotと現在値の差分を返す。
null/未知/evict済みrevisionは完全な現在snapshotで再同期する。
window IDを無制限mapに追加せず、保持数は4件に固定する。
full/差分のJSON構造、frontendのproject/read-generation/runtime watermarkは維持する。

旧handlerはengine snapshot取得後に同期mutexを取っていたため、
古い取得結果が後からlockを取り新しいrevisionを得る可能性もあった。
同期lockの中で取得・応答作成・履歴登録を順序付ける。
調査時、snapshot_sync lockを取得するproduction経路はこのhandlerだけで、
EngineHandle::snapshotはengineのread lockとcloneだけ。逆順取得は見つかっていない。
revision上限で番号を再利用せず明示的なエラーにする。

## 責務・性能上の制約

snapshot同期・delta DTO/計算は専用moduleへ移し、Tauri/AppStateへの依存を持たせない。
mainはTauri adapterと既存timeline runtime wireの組み立てを保持する。
受入テストは別ファイルへ分離し、巨大なmainへ新規テストを積み増さない。

保持数1→4にはメモリ増加のtradeoffがある。各snapshotサイズ自体は既存projectに依存し、
固定byte上限を保証するものではない。4件以上遅れたconsumerはfullに戻る。
engine側のsnapshot clone、深い比較、lock待機時間そのものを最適化したとは主張しない。
EngineHandle::snapshotのpoison時defaultや、project全体のauthority方式も今回変更しない。
既存snapshot wireにはbackend project identityがなく、renderer側read guardとTimeline watermarkを
維持する範囲。今回の履歴をproject authorityの新しい証明として扱わない。

## 検証

実production serviceの2client workload（256 cue、100要求）は、旧single-entry方式の
full100/delta0・16,030,482 bytesに対し、履歴4件でfull2/delta98・325,296 bytes。
JSON payloadはこの条件で約97.97%減。共通timeline runtime envelopeを除いたpayloadを
同じ入力・要求順で比較した値であり、実機CPU/通信時間の削減率ではない。

各clientで毎応答のfull/deltaをJSONへ適用しcurrent snapshot全体との一致を検証する。
eviction・未知/future cursor・nullable消去・collection削除/再追加・revision上限も扱う。
capture順序はadapterのlock→capture→publishの実コードと逆順lock不在を静的確認した。

- `node app/scripts/check-snapshot-runtime-watermark.mjs`: PASS。
- `node app/scripts/check-cue-effect-recall.mjs`: PASS。
- `node app/scripts/check-media-asset-authority.mjs`: 初回は旧direct-normalize呼出しを要求する
  source assertion(902行付近)でFAIL。既存Appのshared ingress経由の正規化に追従修正後PASS。
  消滅済みslice終端も更新。既存のempty catalog/full/video delta動的検証は維持。
- 独立review: delta全field/serde/functionの移設一致、history capとexact base、capture順序、
  frontend guard維持、追加テスト、checker更新を確認し、修正必須所見なし。

最終Cargo: `cargo test -p syndocal --locked snapshot_sync -- --test-threads=1 --nocapture`
7 passed / 0 failed / 0 ignored。
`cargo test -p syndocal --locked engine_snapshot_delta_reports_grouped_cue_activation_and_release -- --test-threads=1`
1 passed / 0 failed。既存cue activation/releaseの意味も維持。
追加roundtrip試験の初回は空mapのfull省略/delta明示{}をJSON字面比較して失敗したが、
全fieldのtyped復元一致とwire上の明示消去の両方を要求する形へ修正し成功。
独立reviewでこの変更が検証を弱めないことも確認。

最終test compileのfirst-party warningは0（直前alpha.69記録0→今回0）。
内部抽出はmainの16追加/159削除、純減143行。新serviceとテストは別moduleに配置した。

`pnpm --dir app tauri build --no-bundle`: PASS（Rust release 2m09s）。
beforeBuildのTypeScript/Viteも成功。Rust/TypeScript警告0、既存Vite chunk advisory1で増加0。
`target/release/syndocal.exe`: 63,700,480 bytes、SHA-256
`8DF0786FA5FE203D1B5C36E7FD512C3F492F1CAFF27EB9FB5F5166F0A2C78678`。
exact pathで起動したPID43784、Syndocal window1つ、Responding=trueを照合。
1920x1032の最大化画面でEdit Video→Lightingへ実クリックし正常描画を確認。
アプリは起動したまま残す。実show/2native windowの負荷受入とは区別する。
旧exeは `target/qa/snapshot-sync-20260905/syndocal-alpha.69-before-snapshot.exe` に保全し、
前回SHA-256 `6BF34E0D0876F927DA7EB4F3CA578E1F476363CF7F47FB9443A39F5AABDB054D` と一致確認。

全Cargo実行は
`qa/WINDOWS_NATIVE_BUILD.md` のCommunity MSVC 14.44.35207絶対pin/where-first検証を適用。
再現コマンドは前チェックポイントの `target/qa/recording-atomic-20260905/run-native.mjs`
経由で実行。stdout/stderrは `target/qa/snapshot-sync-20260905/` に保存する。

別所有の `app/scripts/check-viewport-containment.mjs` は変更・stageしない。
SHA-256: `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`。
物理出力、実showでのCPU/FPS/latency、複数native windowの負荷試験は別受入。

次の候補はVideo FX到達性・hidden consumerの修正、またはengine snapshot clone/lock待機の
代表show計測。今回のsource/QAだけをcommit/pushし、Gitでcheckpointとupstream一致を確定する。
