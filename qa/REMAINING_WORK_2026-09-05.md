# 残件・既存実装・構造整理 — 2026-09-05

基準: `codex/syndocal-v1.2` / `ff02259e25e1938f623b43494b2299ae9482c051`。
初回の作業は録画保存の原子性修正・責務分離とVideo runtime取得の重複抑制。製品候補は `1.2.0-alpha.69`。
この文書のコミットが今回のチェックポイントを特定する。

続行したsnapshot同期の分離・複数windowでの全量応答削減は
[snapshot同期チェックポイント](SNAPSHOT_SYNC_2026-09-05.md)を参照。
続くEdit VideoのレイヤーFX復旧・非表示consumer停止は
[Video FXチェックポイント](EDIT_VIDEO_FX_2026-09-05.md)を参照。
runtimeの小さい読取に伴う全量snapshot複製の除去は
[snapshot読取チェックポイント](SNAPSHOT_READ_2026-09-05.md)を参照。
そこで検出したrelease test構成の不備は[test-support分離](SERIAL_TEST_SUPPORT_2026-09-05.md)
で解消し、最適化構成の比較値を同文書へ記録した。
snapshot生成時のpercentile重複sortと測定窓分離は
[telemetry改善チェックポイント](TELEMETRY_PERCENTILES_2026-09-05.md)を参照。
追加依頼のTimeline全映像出力プレビューとソース分類統合は
[Timeline出力プレビュー](TIMELINE_OUTPUT_PREVIEW_2026-09-05.md)を参照。
以下のalpha.69録画チェックポイントのartifact/hashはその時点の記録を保持する。

2026-09-06: Spout開始とVideo BO復旧・照明との分離、Unity映像の滑らかさは
[ユーザー受入記録](SPOUT_DISABLED_PAIR_SYNC_2026-09-06.md)で確認済み。
続く軽量化は[Spout送信前の限定snapshot読取](SPOUT_OUTPUT_SNAPSHOT_2026-09-06.md)。
一般snapshot/tick構築/深いdelta比較の残件とは区別する。

## 以前の優先残件（2026-09-06時点）

## 2026-09-07 current continuation

今回の候補では、Full videoの旧3失敗を修正して `174 passed / 0 failed /
3 ignored`、録画rendererを子プロセスへ分離して録画フィルタを
`57 passed / 0 failed / 6 ignored`、MCPを9 tools・47 reviewed canonical
operationsへ拡張した。release build/launch、MCP実EXE round-trip、localization
`3693/3693`、I/O直列回帰 `180/0/2` も通過している。詳細は
[Luna引き継ぎ](LUNA_RESUME_HANDOFF_2026-09-07.md)の final continuation を参照。

この2026-09-07の限定対応で残るのは、現ホストで外部設備・秘密情報を要する受入である。製品全体の実装・統合・検証完了を意味しない。
serial DMX/Enttec COMと物理MIDIは安全な列挙で不在、会場GPU/実show総合受入は
未実施、Authenticodeは実証明書・秘密鍵不在のため未署名である。これらを
ソフトウェアloopbackや自己署名証明書で完了扱いにしない。

微小な性能改善を続ける前に、未解決の動作を優先する。以下は今回の改善依頼に
関係する具体的な残件であり、下段の製品ロードマップ全体を完了扱いにしない。

| 優先 | 残件 | 完了条件・現状 |
| --- | --- | --- |
| 1・実装完了、外部受入待ち | 録画のencoder停止・回収 | rendererを長寿命子プロセスへ分離し、親/子の責務を分離。Stop watchdog、Job Object子孫回収、10秒total-stop deadline、250ms reap確認を実装。録画フィルタ `57 passed / 0 failed / 6 ignored`、renderer 2/2、tree 1/1、実H.264/AAC 1/1、合成30分A/V 1/1。実showの運用受入は別境界。 |
| 2・Windows限定完了 | 録画の既存保存先の競合 | [Windows保存先の保全と復旧](RECORDING_PUBLICATION_2026-09-06.md)：検証済みhandleを保持し、競合相手を上書きしない二段階renameと次回予約時の復旧を実装。48件の録画テスト・実FFmpeg・native build/起動確認通過。電源断耐久性や録画全体の受入とは区別する |
| 3・部分対応 | 肥大化コードの責務分離 | [project transactionの確定・取消処理](PROJECT_MUTATION_LIFECYCLE_2026-09-06.md)を独立controllerへ分離し、実行テスト・型チェック・レビュー・native build/起動確認通過。Appのポリシー・authority管理は維持。media lifecycleの追加分離は残件 |
| 4 | 全量snapshot生成・差分比較の負荷 | 実showでclone、writer待機、delta生成とpayload量を分けて計測してから変更する。送信前限定読取の改善値を全体性能へ流用しない |

Spout開始、Video BOの復旧・照明との分離、Unity映像の滑らかさはユーザー確認済み。
追加報告の複数灯体の回転は[修正・検証記録](MAPPING_GROUP_ROTATION_2026-09-06.md)を参照。
その後の「Ctrl+Zで1灯しか戻らない」は[一括履歴修正](MAPPING_GROUP_UNDO_2026-09-06.md)がテスト・native build/起動確認通過。
これらを再度未確認として実機操作を要求しない。GUIの全操作Undo/Redoなど、
上記と別の未検証項目はそれぞれの受入文書の境界を維持する。

## 台帳の読み替え

元台帳58行は Supported 31 / External acceptance 19 / Deferred 2 /
Out of scope 6。Open 50行を未実装50件と数えない。
下表は実装順のための補助分類であり、元の受入状態を変更しない。
「実装・決定残」は部分実装を含む広い契約で、全機能が欠落しているという意味ではない。
録画保存とAI2の限定実装以外は、個別着手時にコードと受入範囲の追加照合が必要。

| ID | 次の作業区分 | 依存条件（既存台帳） |
| --- | --- | --- |
| COMP-Q1-Q4-001 | 既存実装の照合・統合・検証を先行 | Current flow authority and the master roadmap Q1-Q5 contract |
| WARN-MACOS-001 | 保留・現行対象外 | Windows-only product scope and the versioned warning inventory |
| RELEASE-METADATA-GATE-001 | 既存実装の照合・統合・検証を先行 | Synchronized product metadata and release-train policy |
| AI3-DURABLE-RECOVERY-001 | 既存実装の照合・統合・検証を先行 | Existing process-local receipt and owner-retirement implementation |
| AI3-NATIVE-INGRESS-001 | 外部・native・実機受入 | Canonical ingress-policy implementation and available native clients/devices |
| AI3-PHYSICAL-REARM-001 | 外部・native・実機受入 | New/Load/Recovery/Backup/Take Over paths and acknowledged physical output state |
| AI3-DURABLE-ACCEPTANCE-001 | 外部・native・実機受入 | Durable recovery implementation plus verified native dangerous-action workflows |
| AI0-COVERAGE-001 | 既存実装の照合・統合・検証を先行 | AI3 completion and authoritative operation registry |
| AI1-SCHEMAS-001 | 既存実装の照合・統合・検証を先行 | AI0 source coverage and generation authority model |
| AI2-COMMAND-BRIDGE-001 | 実装・仕様決定の残りを具体化 | AI1 schemas and project authority receipts |
| AI4-CONSENT-001 | 実装・仕様決定の残りを具体化 | AI0-AI2 authority paths and the accepted local-consent decision |
| AI5-SIDECAR-001 | 実装・仕様決定の残りを具体化 | AI0-AI4 coverage, schemas, bridge, and consent policy |
| AI6-ADMIN-UI-001 | 実装・仕様決定の残りを具体化 | AI4 principal/grant model and AI5 sidecar health model |
| AI7-ADVERSARIAL-PROOF-001 | 既存実装の照合・統合・検証を先行 | AI0-AI6 implemented paths and frozen review inputs |
| AI8-EXTERNAL-ACCEPTANCE-001 | 外部・native・実機受入 | AI0-AI7 completion plus clean installation and physical output hardware |
| F1-INPUT-GENERATIONS-001 | 既存実装の照合・統合・検証を先行 | Project replacement authority and input-worker lifecycle |
| F2-OUTPUT-OWNERSHIP-001 | 既存実装の照合・統合・検証を先行 | F1 input generations and AI3 output-lease authority |
| SHOWCLOCK-DECISIONS-001 | 実装・仕様決定の残りを具体化 | F2 local ownership boundary and the master decision log |
| VIDEO-FULL-GATE-001 | 既存実装の照合・統合・検証を先行 | Accepted Windows Clip Slot, Layer Bus, and FX tranches |
| VIDEO-C2-C4-001 | 既存実装の照合・統合・検証を先行 | Video full-gate reintegration and timeline authority |
| TIMELINE-FOLLOW-001 | 既存実装の照合・統合・検証を先行 | C2/C4 transition integration and authoritative timing model |
| TIMELINE-PERSISTENCE-001 | 既存実装の照合・統合・検証を先行 | Timeline Follow runtime and project save/reload authority |
| MEDIA-DERIVED-001 | 実装・仕様決定の残りを具体化 | Media asset authority and cancellable background-work boundary |
| AUDIO-AUTHORED-001 | 実装・仕様決定の残りを具体化 | ShowClock master-clock decision and timeline authority |
| VIDEO-LIVE-SOURCES-001 | 既存実装の照合・統合・検証を先行 | Video source identity model and output ownership |
| RECORDING-001 | 実装・仕様決定の残りを具体化 | Audio master-clock and output ownership decisions |
| SHOWCLOCK-IMPLEMENTATION-001 | 実装・仕様決定の残りを具体化 | Frozen ShowClock decisions plus Audio, Recording, and ownership semantics |
| UI-H1-REACHABILITY-001 | 既存実装の照合・統合・検証を先行 | Supported-feature inventory and source-of-truth routing |
| UI-H2-SHELL-001 | 既存実装の照合・統合・検証を先行 | H1 reachability and shared status/selection model |
| UI-H3-SETUP-001 | 既存実装の照合・統合・検証を先行 | H1/H2 shell and PATCH/GDTF/output-device authority |
| DJ-LINK-SETUP-001 | 既存実装の照合・統合・検証を先行 | Authenticated DJ Link listener and project Track-to-Timeline authority |
| UI-H4-EDIT-001 | 既存実装の照合・統合・検証を先行 | H1/H2 shell plus Timeline, Media, FX, Stage, and project authority |
| UI-H5-CONTROL-001 | 既存実装の照合・統合・検証を先行 | H1/H2 shell plus live ownership, recording, and diagnostics |
| REMOTE-SECURITY-001 | 実装・仕様決定の残りを具体化 | AI consent/sidecar policy and output ownership boundaries |
| MIGRATION-COMPATIBILITY-001 | 実装・仕様決定の残りを具体化 | Version-support decisions and durable recovery/save boundaries |
| OBSERVABILITY-SUPPORT-001 | 実装・仕様決定の残りを具体化 | Generation-stamped runtime state and release/update policy |
| ACCESSIBILITY-NATIVE-001 | 外部・native・実機受入 | H1-H5 native UI implementation and available accessibility environments |
| ASIO-LICENSE-001 | 外部・native・実機受入 | Explicit GPLv3-separated artifact or Steinberg SDK decision |
| ASIO-FORMAT-MATRIX-001 | 外部・native・実機受入 | Licensed ASIO artifact and real drivers/devices |
| ASIO-FAULT-MATRIX-001 | 外部・native・実機受入 | ASIO format matrix and real fault-injection hardware |
| ASIO-SOAK-001 | 外部・native・実機受入 | ASIO fault matrix, WASAPI comparison path, and long-duration device access |
| ASIO-LATENCY-001 | 外部・native・実機受入 | ASIO device stream and matched TouchDesigner test environment |
| ASIO-PERSISTENCE-PACKAGE-001 | 外部・native・実機受入 | ASIO artifact decision, device matrix, and final package |
| DMX-ARTNET-001 | 外部・native・実機受入 | F2 output ownership and real Art-Net/sACN nodes and fixtures |
| DMX-USB-RDM-001 | 外部・native・実機受入 | F2 output ownership, Enttec/DMXKing hardware, and analyzer capture |
| INPUT-PHYSICAL-001 | 外部・native・実機受入 | F1 input generations and physical MIDI/OSC/Remote clients |
| DJ-LINK-HARDWARE-001 | 外部・native・実機受入 | DJ Link setup, physical Pedal/DJ-PC/Agent topology, and exact mapping evidence |
| VIDEO-PHYSICAL-001 | 外部・native・実機受入 | F2 ownership, real display topology, and capture/output devices |
| PLATFORM-MAC-LINUX-001 | 保留・現行対象外 | Windows-only product scope decision |
| VENUE-SOAK-001 | 外部・native・実機受入 | Integrated video, lighting, output, and recording paths plus venue GPU |
| SHOWCLOCK-VENUE-001 | 外部・native・実機受入 | ShowClock implementation, witness/fence decision, and two-machine topology |
| COMPARE-PINNED-001 | 外部・native・実機受入 | Pinned comparison versions, licenses, hardware, content, and exact Syndocal artifact |
| DIST-PLATFORM-PACKAGE-001 | 保留・現行対象外 | A future distribution product goal and supported artifact matrix |
| DIST-SIGNING-001 | 保留・現行対象外 | A future distribution product goal and signing credentials/policy |
| DIST-NOTICES-001 | 保留・現行対象外 | A future distribution product goal and license/SBOM disposition |
| DIST-CLEAN-MACHINE-001 | 保留・現行対象外 | A future distribution product goal and signed installation artifacts |
| DIST-UPDATER-001 | 保留・現行対象外 | A future distribution product goal, signed manifests, and update service |
| DIST-PUBLICATION-001 | 保留・現行対象外 | A future distribution product goal, immutable tag, artifacts, and evidence manifest |

集計: 実装・仕様決定12 / 既存実装の照合19 / 外部・native・実機19 / 保留8。

## 再実装せず確認するもの

- Release metadataには候補タグ・updater・artifact検証が既存実装されている。
  `app/scripts/check-release-metadata.mjs` とそのnegative testsを先に照合する。
- AI3には `OutputLeaseDurableReceiptJournal` が存在する。Open行だけを理由に
  新しいdurable journalを作らない。
- `control_plane_query.rs` はlocal-window query/event/cursor、
  `authored_control_plane.rs` は `set_effect_enabled` の限定verticalを実装済み。
  外部API/全authored operation受入と混同しない。
- Timelineメニューは `TimelineItemContextMenu.tsx` に分離済みで、
  `check-timeline-performance-browser.mjs` に外側クリック/Escape等の検証がある。
  古い引継ぎにある「未実装」をそのまま再実装しない。
- ASIO、DJ、DMXの送信ログは、実機の受信・発光・可聴確認の代わりにはならない。

## コードで確認した優先残件

1. **録画の保存境界（今回）**: 変更前はFFmpegが最終保存先へ直接 `-y` し、
   失敗前に既存ファイルを壊し得た。同一directoryの排他的partialへ出力し、
   encoder成功・完全frame数が正・file非空・sync後だけ公開するよう修正。
   新規保存先は公開直前の別writer作成にも原子的に上書きを拒否する。
   既存保存先はidentity/size/mtime変化を検出するが、最後の比較とreplaceはCASではない。
   通常の別writerもこの間に競合できるため、公開時の保存先独占を前提とする残存境界がある。
   partial所有権が変わった場合は公開/削除せず、encoder reap失敗ではpartialを明示して残す。
2. **Video FXの到達不能（限定修復済）**: 素材未選択でもEdit VideoのAdvancedから
   対象layerの既存ISF編集へ到達する専用inspectorを追加。libraryOnlyで非表示だった
   consumerは生成せず、開いたlayerのFX editorだけ生成する。Composition/Group/Output
   FXと録画UIの全体再構成は別の残件。[証拠・範囲](EDIT_VIDEO_FX_2026-09-05.md)。
3. **録画の停止・同期全体**: stderrの継続排出と64KiB上限は今回対応。stdin書込とStop joinの
   上限、A/V時計、空き容量、crash recovery inventory、playable検証、asset importを
   個別の受入に分ける。今回の保存修正だけで `RECORDING-001` を完了にしない。
4. **既存overlapハーネス差分**: `app/scripts/check-viewport-containment.mjs` の
   未コミット差分は今回の所有外。SHA-256
   `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063` を保全し、
   独立した所有権・レビュー・focused gateで引き継ぐ。

## 性能・アーキテクチャの扱い

ファイルサイズは保守性の指標であり、それだけで実行性能の問題とは断定しない。
既存の計測値と今回の静的懸念を分ける。

### 今回確認・修正した要求の増幅

`App.tsx` のclip 250ms / transition 100msのpollが、遅いIPCに対して重複要求を作っていた。
`createVideoRuntimeController.ts` でqueryごとに1つの進行中readへ制限した。
既存のepoch/generation検証を維持し、project reset以前の応答・エラーを破棄する。
`check-video-runtime-polling.mjs --baseline-ref ff02259e25e1938f623b43494b2299ae9482c051`
のdeferred-response試験は、各queryへ20回連続refreshしたとき、修正前のinvoke数/最大同時数
20/20に対し修正後1/1を確認。成功・失敗後の再開、同一token reset、window退役、
古いruntime generation、2queryの独立性も検証する。
これは要求数の実測であり、実機CPU/FPS/latencyの改善率ではない。
永続的にsettleしないIPCのtimeout/cancelは別契約として残す。

### 静的監査で見つかった優先計測点

| 優先 | 実装と懸念 | 最初の検証・修正境界 |
| --- | --- | --- |
| 対応済 | 監査時の `SnapshotSyncState` は全client共通の単一last snapshot/revisionで、交互pollがfull応答を増やしていた | 続行で履歴4件の独立moduleへ分離。2clientの100要求でfull100→2。[計測・制約](SNAPSHOT_SYNC_2026-09-05.md) |
| 高 | `crates/engine/src/lib.rs` のsnapshot cloneとtick内snapshot生成がlock内にあり、authored collectionも複製 | 代表showでclone時間・lock待機・payload bytesを別々に計測。single mutation ownership、fence、ACKを維持 |
| 限定対応済 | clip/transition/Follow/transportの小さいruntime取得が全snapshotをcloneしていた | 専用snapshot_readへ分離して必要fieldのみ取得。synthetic比較と既存runtime回帰を実施。tick構築と一般snapshot読取は未変更 |
| 高 | `main.rs` の `engine_snapshot_delta` は深い比較/clone。Stage 30Hz consumerに対しUI apply間引きだけではbackend仕事量は減らない | 上記snapshot計測とまとめて検証してからrevision/dirty trackingを選ぶ |
| 対応済 | libraryOnlyで非表示の旧mixer consumerがmountされ続けていた | 条件mountへ変更。FXはclosed/open/closedで0/1/0。CPU/FPS改善率は未計測 |
| 対応済（限定境界） | 録画のblocking stdin write、encoder/renderer処理とDropの停止所有権 | Windows FFmpegはJob Objectで子孫まで回収し、stdin/stderrの管轄を保持。worker/Dropは10秒で所有reaperへ移管し、二重Startを防止する。任意の同期インプロセス処理を安全に強制中断することはできないため、rendererは境界協調キャンセルとして扱う |

### 肥大化ファイルの分割順序

監査時のサイズはengine/lib.rs 5,449,535B、main.rs 5,105,608B、App.tsx 1,233,100B、
styles.css 825,039B、protocol/lib.rs 805,446B、io/remote_ws.rs 657,399B、
video/lib.rs 581,530B。巨大な同居テストも含むため、数値をruntime負荷とみなさない。

1. **今回**: `recording_artifact.rs` はUI/AppStateに依存せず予約・検証・公開・清掃を担当。
   `video_recording.rs` はFFmpeg command/audio filter/workerを担当。
   mainは開始/停止のTauri adapterとengine/renderer/statusの接続を保持する。
   workerはまだ既存renderer/preview snapshot adapterに依存する段階的抽出で、
   完全なdomain層独立を達成したという意味ではない。
2. **次のmain単位**: snapshot同期は分離済。project transaction、media lifecycleを個別責務で切り出す。
   UI/Tauri adapter → application service → domain/port → OS adapterの依存を意識し、
   domainがAppState/Tauriを参照する逆依存や、全状態を集めた巨大serviceを増やさない。
3. **App**: 既存controllerを活用し、poll lifecycle・view・開発fixtureを切り分ける。
   FX到達性とhidden consumer修正は専用inspectorへ分離済。同じstateの二重所有を作らない。
4. **Engine/protocol/IO/video**: snapshot・timeline・video・transportの既存責務ごとに、
   公開APIとmutation所有権を維持して分離。同居テスト分離は可読性改善として扱う。
5. **CSS**: cascade順序とscopeを保つ単位で分割。字体/操作対象の縮小を伴わせない。

大規模な一括移動より、依存境界と回帰検証が揃う単位を順に統合する。
今回のmain差分は21追加/236削除（純減215行）。新しいworkerは約11KB、artifactは
テスト込み約23KB。ファイル分割そのものによる実行速度向上は主張しない。
今回Vite出力はApp chunk 500.17kB（gzip153.93kB）、styles 629.16kB（gzip105.37kB）。
大容量chunk advisoryは過去alpha.68 handoffにも記録された既存1件で、今回も1件。
単なるsource分割でbundleが減るとは限らないため、Appのロード境界と初期parse計測を合わせる。

## このチェックポイントの検証

独立レビューで新規保存先の比較→置換競合(P2)を検出し、no-clobber公開とlate-creator試験で修正。
安定差分の再レビューは追加blocking findingなし。既存保存先の非CAS境界は上記の通り残す。

- `pnpm --dir app run check:video-runtime-polling`: PASS。
- `node app/scripts/check-video-runtime-polling.mjs --baseline-ref ff02259e25e1938f623b43494b2299ae9482c051`: 両queryのbaseline 20/20を確認。
- `node app/scripts/check-video-transport-status-polling.mjs`: PASS。
- `node app/scripts/check-video-clip-slot-bank.mjs`: FAIL、変更外の122行目の旧Edit-domain live除外assertion。
  controller assertionsはそれ以前に通過。App.tsx/当該checkerは今回変更しておらず、assertionを緩めていない。
  現行のEdit routing契約との照合を独立した残件とする。
- 2026-09-06 reconciliation: `check-video-clip-slot-bank.mjs` now asserts the current
  three-domain `edit`/`mixer`/`live` `editDomainModes` contract. `uiModes.ts`,
  `WorkspaceChrome.tsx`, the focused upper-workspace route checks, and shortcut checks
  all confirm the three routes; product routing is unchanged. Its viewport-breakpoint
  check is also scoped to the B4 Clip Slot section, excluding the later Timeline bank
  section's intentional media query. The checker-only fix keeps retired assertions
  from masking the current contract. The focused Clip Slot checker and affected
  `git diff --check` passed; no product code changed in this reconciliation.
- `pnpm --dir app run check:release`: PASS（構成する全gate成功）。
- `node app/scripts/test-check-release-metadata.mjs`: PASS、125 assertion groups。
- `cargo test -p syndocal --locked recording_ -- --test-threads=1`: PASS、21 passed / 2 ignored。
  うちartifact 17件。長時間A/V試験は実行しない。
- `cargo test -p syndocal --locked recording_command_writes_a_real_video_and_audio_mp4 -- --ignored --test-threads=1`:
  PASS、1件を明示実行。実FFmpegでpartialへH.264/AACを生成し、既存final保持→公開→ffprobeのvideo/audio streamを確認。
- `cargo test -p syndocal --locked c1_effect_aware_output_preview_production_seams_are_wired -- --test-threads=1`:
  PASS、1件。source契約は抽出先に追従し既存assertionを維持。

Cargoの全実行は `qa/WINDOWS_NATIVE_BUILD.md` 通りvcvars64 14.44を初期化し、
Community MSVC 14.44.35207 Hostx64/x64の絶対linker pinとwhere-firstを開始前に検証。
再現helperとログは `target/qa/recording-atomic-20260905/` に保存（generated/local evidence）。
初回test compileの新規unused Stdio warning 1件は除去し、最終test compileは0件。
変更前のcompiler baselineを今回再buildしていないためbaseline値は未測定。

`pnpm --dir app tauri build --no-bundle`: PASS、Rust release 3m02s。
beforeBuildの `tsc --noEmit && vite build` も成功。最終Rust/TypeScript warning 0、
既存Vite chunk advisory 1（alpha.68記録1→今回1、増加0）。Gitの改行通知はcompiler warningではない。

Native artifact: `target/release/syndocal.exe`、63,391,744 bytes、SHA-256
`6BF34E0D0876F927DA7EB4F3CA578E1F476363CF7F47FB9443A39F5AABDB054D`。
この絶対pathを起動しPID75296を照合。返されたSyndocal windowは1つ、`Responding=true`、
Control画面の描画を確認。最大化した1920x1032画面とnative「元のサイズに戻す」を確認した。
UI操作中にはuser-input検出と16x16補助screenshot選択による座標拒否があった。
ユーザーの操作続行指示後、対象を前面化・最大化し、1920x1032の本体screenshotを選択。
Control→Edit Lighting→Edit Videoの実クリック遷移と空のmedia libraryの正常描画を確認した。
実GUI録画・大きなshow負荷は未検証。起動したアプリはEdit Videoのまま残す。
旧alpha.68 exeは `target/qa/recording-atomic-20260905/syndocal-alpha.68.exe` に保全、
SHA-256 `0B30AC14FD84F28A628522024AD573ADECE5DD1210FC84A80971F3A3F24FA08F`。

物理出力、実機録画の可聴/A/V長時間、全matrix、製品全体の完了は未検証。

snapshot同期とVideo FXの限定修復は各続行文書へ移管。次はengine snapshot clone/lock待機の
実show計測、またはproject transaction/media lifecycleの独立した責務分離。
録画の残存CAS/停止上限と、別所有overlap差分は独立した境界のまま引き継ぐ。
このチェックポイントは明示した所有ファイルのみcommit/pushし、upstream一致を確認する。

2026-09-07 continuation: synthetic 30-minute FFmpeg A/V test passed 1/1 under
the exact Build Tools MSVC 14.44.35207 linker. The retained report records
1800 seconds, 54,000 frames, 0.0 ms start/end drift, first/middle/last video
luma 255/0/255, and audio peaks 4276/0/4123. This is command/codec evidence;
real in-app capture, hardware, and the renderer/OS shutdown deadline remain
separate open boundaries.

## 2026-09-07 current continuation status

The recording teardown boundary in the table above is now implemented and
verified. The focused current filter is `57 passed / 6 ignored / 0 failed`, the
explicit H.264/AAC MP4 test is `1/1`, the synthetic 30-minute A/V test is `1/1`
with 54,000 frames and 0.0 ms start/end drift, and the descendant process-tree
regression is `1/1`. The three ignored subprocess helpers are invoked only by
their owning tests.

The remaining non-claim is intentional: these tests do not prove a real
operator show, physical output, camera/device capture, or safe interruption of
one already-running synchronous CPU/GPU instruction. Those require their
separate hardware/native acceptance boundaries.

## 2026-09-08 製品全体の継続対象

照合元は `main / 00e8785`。完成台帳チェックは50 Open＋8 Deferred、
Q1-Q4対応表チェックも通過した。ただし台帳の整合性は実装・受入の完了ではない。
Open行を未実装件数へ読み替えず、既存コードと個別の証拠を照合して閉じる。

- AI/MCP: 47 reviewed operationsの境界を維持する。全製品操作のparity、
  管理UI、認可・復旧・外部クライアント受入はそれぞれ別契約。
  今回の具体的なアダプター不具合は
  [canonical admission記録](AGENT_CANONICAL_ADMISSION_2026-09-08.md)を参照。
- ShowClock、Audio、Recording: 既存実装を確認し、時計・所有権・障害復旧の
  仕様と統合証拠を揃える。合成30分の保存成功を実showの完了へ拡張しない。
- Media、UI、構造: derived dataの寿命・取消・cache、機能到達性、media lifecycle
  分離を個別に扱う。snapshot負荷は計測を先行し、大規模な無根拠移動をしない。
- 実機・配布: ASIO/DMX/MIDI/映像機器、会場、Mac実機、証明書は別の受入条件。
  arm64開発DMGの生成成功は全Mac機能・Gatekeeper対応の証明ではない。

Mac最終DMG検証ツールは別の未完了作業として保全している。
単体46件とreleaseチェックの成功はあるが、独立レビューの終了判定・レポート
確定時の指摘と、変更後のmacOS CI検証は未完了。製品完成率は算出しない。

## 2026-09-08 直接継続：Mediaの要求寿命

基準は `d317f13`。Codex/Worksや別モデルへ委譲せず、ChatGPTから直接作業した。
今回の具体的な不足は、サムネイルの旧応答を捨てても要求自体が増殖する点。
50回の更新で各系統51要求が未完了のまま残ることを再現し、各系統を
「実行中1件＋最新の待機バッチ1つ」に制限した。13素材の後続バッチ完走、
旧応答の拒否、reset、既存のdispose回帰を検証した。
詳細・証拠・未確認範囲は [thumbnail要求管理](MEDIA_THUMBNAIL_BOUND_2026-09-08.md) を参照。
この変更だけで `MEDIA-DERIVED-001` 全体を閉じない。

次の境界は、発行済みnative thumbnail要求の取消・期限・所有権である。
現在の上限はcontroller単位で、native処理の強制中断や全window共通の
worker pool、cache容量、波形/proxy、実show性能は未証明。
その後も以下は別々に照合する。名称の検索結果やファイルの存在だけで
未実装・実装完了のいずれも断定しない。

- AI/MCP：47操作の許可判定修正と、全操作parity・管理UI・外部認可の完成を分ける。
- 同期/録画：既存Standby同期・音声解析・録画を確認してからShowClockの
  時計/所有権/障害時方針との不足を確定する。再実装を先行させない。
- UI/性能：到達性とnative受入を対応づけ、snapshotのclone・lock・payloadは計測する。
- 実機/配布：機器・会場・証明書・Mac実機は別受入。50 Open＋8 Deferredの
  旧分類は完了率ではない。Mac arm64開発DMGは後続依頼で着手されており、
  旧Windows-only保留記述を現在のMac開発禁止と読み替えない。

前回のMac検証7ファイルは変更せず保全。今回のソフトウェア修正・検証とは別件。
