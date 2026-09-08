# Desktop window checker repair — 2026-09-09

## Scope

修正対象は `app/scripts/check-desktop-window-mode.mjs` の静的チェッカーだけ。
製品コード、CloseRequestedの実装、ネイティブ設定、受入条件は変更していない。

## 初回所見

Windowsの作業ツリーで `app/src/App.tsx` がCRLFとして読み込まれると、
CloseRequestedの敵対fixtureが使用する複数行アンカーはLFだけを含む文字列と一致せず、
`comment-only disposal fence: hostile fixture anchor must remain discoverable` で停止した。
これは現行CloseRequested実装の欠陥ではなく、チェッカーの入力行末依存である。

## 修正

チェッカーがApp.tsxを解析・fixture化する前にCRLFをLFへ正規化した。
その後も、コメント化、静的死分岐、nested callback、失敗フラグ欠落、公開順序違反、
公開後のdead statement、bare return欠落の既存negative fixtureを同じassertionで検証する。
検証を弱めるためのassertion削除・閾値変更・fixture除外は行っていない。

## 検証

- `node --check app/scripts/check-desktop-window-mode.mjs`: PASS
- `node app/scripts/check-desktop-window-mode.mjs`: PASS
- `git diff --check`: PASS
- 追加の純静的契約: project recovery/publication/open、video output routing/window、
  workspace/operator、media asset authority、output control はPASS

`check:edit-video-fx` はPlaywrightのChromium実行体がこのPCに存在せず起動前に停止した。
これは今回のチェッカー修復の合格結果には含めず、別の環境依存UI検証残件として扱う。

## 境界

この記録はデスクトップCloseRequestedチェッカーのWindows行末互換性だけを扱う。
ブラウザ実行体、Windows native build、物理出力、ASIO/NDI、Mac、署名・公開、
製品全体の完成は証明しない。
