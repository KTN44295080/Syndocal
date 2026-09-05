# Edit VideoのレイヤーFX到達性と非表示consumer整理

基準: `codex/syndocal-v1.2` / `ebc63997e5aa9a760954cfc0068042cc6816d9ed`。
内部改善のため製品/schema versionは変更しない。ローカルnative artifactはhashで識別する。

## 問題と対象範囲

唯一のVideoControlPanelはlibraryOnlyでmountされ、CSSが旧mixer contextを隠していた。
その中のVideoLayerListPanelが唯一のレイヤーISF編集経路だったため、編集機能自体が
到達不能だった。既存inspectorのAdvanced Video Controlsは素材選択時だけの説明文だった。
また、非表示のLayer/ISF、clip、audio、output等のconsumerはmountされたままだった。

Edit Videoのinspectorを専用componentへ分離し、素材propertiesとレイヤーFXを分ける。
素材選択なしでもFX disclosureを開ける。レイヤーを選んで既存ISFパネルと既存の
authoritative callbackを使い、閉じている間と画面外ではFX editorをmountしない。
libraryOnlyはメディアlibraryに必要なconsumerだけをmountする。

これはレイヤーISF編集経路の修復。録画UI、全VJ/Composition/Group/Output FXの再構成、
物理出力やエンジンの描画性能改善を完了したという意味ではない。
DOM/component mount数と実機CPU/FPSの改善率は区別する。

## 検証・引継ぎ

focused browserで素材未選択の到達性、選択layerのcallback、closed/open/unmount、
通常/短いviewportでの内部scrollと操作対象を検証した。

### 結果

- `node app/scripts/check-edit-video-fx.mjs`: PASS、exit 0。再実行用に
  `pnpm --dir app run check:edit-video-fx` を登録。1920×1080 / 1280×720で
  closed/open/closedのISF editor数0/1/0、旧library consumer数0。
  8段stack、layer切替、layer 2へのbuiltin呼出1回、実Appのrefresh反映、
  同一layer更新時の開いた詳細保持を確認。native IPC境界はmock。
- 各viewportの64操作要素は横clip 0、document overflow 0、既存の高さを維持。
  末尾Clear stackへscroll後のtrial clickで到達性を確認（削除は実行しない）。
  初期44px判定は別Clip Slot契約の誤適用だったため、既存FXの26/28px・checkbox18px
  に合わせた。製品サイズは縮小していない。rootもoverview/stack画像を確認。
- 独立した安定差分review: 阻害指摘なし。初回の横切れ懸念は専用CSSの折り返しで修正。
  UIは既存callbackへ依存し、engine状態の複製や新規IPC経路を作っていない。
- `pnpm --dir app exec tsc --noEmit`: PASS。
- `pnpm --dir app tauri build --no-bundle`: PASS、Rust release 1m48s。
  `node target/qa/recording-atomic-20260905/run-native.mjs pnpm --dir app tauri build --no-bundle`
  で実行。MSVC 14.44.35207 Communityの絶対Cargo linker pin・where-firstを検証。
  build wrapperは旧PID43784の正確なcheckout exeを照合して停止した。
- 警告は前チェックポイント記録→今回: Rust/TypeScript 0→0（増加0）、
  Vite chunk advisory 1→0。App chunk 500.17→499.32 kB、閾値変更なし。
  CPU/FPS改善率や総bundleの削減率を示す計測ではない。
- native exeは63,704,576 bytes、SHA-256
  `D2321A10D4B780D381F4B0BFD59C8430B99312BD012D9ED73D6340AB246521E1`。
  正確なpathで起動したPID956、Syndocal window 1つ、Responding=true。
  1920×1032・native「元のサイズに戻す」で最大化を確認して操作。
  Edit Lighting→Video→詳細の実クリック、素材/layerなしの日本語案内を確認。
  空のUntitledのまま残す。実native layerへのFX適用・GPU描画・物理出力は未検証。
- ローカル証拠: `target/qa/edit-video-fx-20260905/` のnative-build.log、
  result-{viewport}.json、overview/stack/controls/expanded画像。
  旧exeは同directoryのsyndocal-before-video-fx.exeに保持、SHA-256
  `8DF0786FA5FE203D1B5C36E7FD512C3F492F1CAFF27EB9FB5F5166F0A2C78678`。

### 対象外の失敗・次の境界

- `node app/scripts/check-localization.mjs`: FAIL。3656/3680 (99.3%)、
  user-data未保護0。新規4文字列は翻訳済。未翻訳24件はShow DMX/USB-DMX/
  diagnostics/fixture types等、今回変更していない文言。100%合格とは扱わない。
- `node app/scripts/check-vj-media-import-access.mjs`: FAIL。CRLF読込に対するLF固定の
  short-height終端marker検索が失敗。診断で改行を正規化すると別の広すぎるCSS sliceが
  downstreamのvideoClipGridPanelを誤検出した。診断編集は撤回済、checker差分なし。
  この旧source checkerは別単位で改行非依存・対象scope限定へ直す必要がある。
- 今回はFX到達性・不要mountの限定修復。代表showのsnapshot clone/lock待機計測と、
  project transaction/media lifecycleの責務分離は残件台帳へ引き継ぐ。

別所有 `app/scripts/check-viewport-containment.mjs` は変更・stageしない。
SHA-256 `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063` を保全。
