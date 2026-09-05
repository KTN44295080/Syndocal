# Timelineの全映像出力プレビューとソース分類

基準: `codex/syndocal-v1.2` / `ab4f812aafb2cf47f584453a99d4f7a0467c2c54`。
ユーザー確認: 上段はソース／インスペクタ／映像プレビューの3タブ。
プレビューはMain composition単体ではなく、設定された実出力をすべて表示する。

## 変更

- シーン／メディアライブラリの独立した上段切替を廃止し、ソース内の
  すべて／照明／映像／音声へ統合。すべてではシーンとメディアの両方を表示。
  既存のScene authority、適合レーン、明示配置先、ドラッグ配置を維持。
- 映像プレビューには出力名・解像度と各出力の画面を表示。
  出力width/heightの比率を保ち、横長・縦長を混在可能。高さ180pxを上限に
  全画面を収め、画面数が多いときは折り返しと内部スクロールを使う。
- 実出力IDのProgram描画を再利用。Timeline投影、エフェクト、
  トランジション、output mapping、blackoutを既存の描画経路から取得。
  出力未設定は明示した空状態。プレビュー用の架空出力や送信の有効化は行わない。
- 描画UI・取得スケジュールを専用component/controllerへ分離。
  全出力合計で最大10リクエスト/秒、同時1件、巡回取得。
  2出力の場合は各画面およそ最大5fpsであり、実出力の送信FPSではない。
  タブ解除・page非表示で停止。削除・設定変更・遅延応答を隔離し、Blob URLを破棄。
  空Timelineでも取得するため、最後のクリップの画像を固定しない。
  busy/error時は古い画像を消し、取得待ちまたは具体的エラーを表示。

## 検証

- `node app/scripts/check-timeline-output-monitors.mjs`: PASS。
  縦横比、全出力巡回、合計頻度、single-flight、同値snapshot更新、
  設定変更・削除・非表示・破棄時の遅延応答、busy、エラー回復、無効解像度。
- `node app/scripts/check-timeline-output-preview-browser.mjs`: PASS、1920×1080/1280×720。
  実Appの3タブ・Home/End・分類・非選択パネル解除・44pxタブを確認。
  実component/controllerとmock IPCで2出力の16:9/9:16、JPEG描画、unmount後の取得停止を確認。
  初回検証はfixtureがLightingで始まることを見落とし待機失敗。
  既存Timeline navigation selectorで実操作するよう修正後、両サイズ成功。
- `node app/scripts/check-timeline-source-shelf-contract.mjs`、
  `node app/scripts/check-timeline-external-dnd.mjs`: PASS。
  旧category属性の検証は新しい3タブ／共通分類に合わせて更新。
  既存negative guard mutationはCRLF正規化と変更成立assertを追加して維持。
- `node --check app/scripts/timeline-source-target-reveal-proof.mjs`: PASS。
  exported helperの旧カテゴリ操作を更新。全Timeline performance matrixは今回未実行。
- `pnpm --dir app exec tsc --noEmit`: PASS。
  初回の翻訳キー重複を解消。独立reviewでbusy時の古いフレーム残留を修正し再検証。
- `git diff --check`: PASS（GitのLF/CRLF変換通知はcompiler warningではない）。

`pnpm --dir app tauri build --no-bundle`: PASS、Rust release2m22s。
Windows native helperでexact Community14.44.35207 linker pin/where-firstを確認。
wrapperが旧exact exe PID73784だけを終了。TypeScript/Vite/Rust成功、
first-party warnings前checkpoint0→今回0（増加0）、Vite chunk advisory0。
新exe 63,669,760 bytes、SHA256:
`9C46B723EA7FEF56982646F7286A8B68B2302538F041C8AA1F589B53B0B9E300`。
起動PID74472、exact checkoutのSyndocal window1つ、Responding=true。
1920×1032・native「元のサイズに戻す」で最大化状態を確認してからタブを操作。
日本語3タブ、ソース内の4分類、映像プレビューの出力未設定表示を確認。
このnative確認は空のUntitledであり、複数実素材の再生検証とは区別する。
プレビュータブを開いたまま残す。

ローカル証跡は
`target/qa/timeline-output-preview-20260905/`。
旧exe SHA256: `2C1B7805CE7E67054E3E234F710991E0FD106B37A9FF295F2CE2D1073532CF5F`。

## 残る境界

物理ディスプレイ・NDI/Spoutへの送信、実showの複数映像同期・長時間性能は未検証。
プレビューは低解像度・低頻度のreadbackであり、出力実機のフレーム同期確認ではない。
backend IPC自体が無期限停止した場合の復旧は既存IPCの責務で、今回保証しない。
一般snapshot複製・tick内構築等の継続監査はREMAINING_WORK台帳に残す。
別所有`app/scripts/check-viewport-containment.mjs`は変更・stageしない。
SHA256: `3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`。
