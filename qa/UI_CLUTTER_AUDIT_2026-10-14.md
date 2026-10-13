# UI整理の監査と提案 — 2026-10-14

対象: `main` の alpha.81 相当ソースを、リポジトリ既存のブラウザ用フィクスチャ（`?syndocalViewportFixture=` と
`check-viewport-containment.mjs` の疑似バックエンド）で 1920×1080・日本語表示にして、45画面を撮影・棚卸しした。
ネイティブ専用の表示（映像プレビュー、出力ウィンドウ、実機のデバイス一覧）は対象外。

この文書は提案であり、実装の指示ではない。実装時は [AGENTS.md](../AGENTS.md) の「UI sizing」に従い、
文字・ボタン・余白を小さくして詰めるのではなく、並べ替え・折りたたみ・ページ分けで解決する。

## 結論

煩雑さの原因は、大きく次の3つ。

1. **画面の4割を、そのページと関係ない欄が占めている。**
   Setup の全ページ（I/O・映像・Security）に、下段の「グループ／ステージ／選択」欄が常に出ている。
2. **同じことをする場所が複数ある。** 例は次のとおり。
   - DMX出力は1ページに3系統あり、それぞれに確定ボタンがある。
   - ブラックアウトは6か所にある。
   - MIDIの接続は3か所、Learn は3経路ある。
3. **開発者向けの言葉と診断が、通常の操作画面に出ている。**
   例：「1/4 output role Both」「S0 engaged」「Output lease authority query was invalid」「JITTER P99」。

## 提案（効果の大きい順）

| # | 提案 | 効果 | 変更範囲 |
| --- | --- | --- | --- |
| P1 | Setup下段の「グループ／ステージ／選択」欄を、照明・パッチなど必要なページだけに出す | I/O・映像・Securityで画面の約4割が空く。DMXのネットワーク行が1行しか見えない問題も解消 | 画面のみ・小 |
| P2 | DMX出力を「ユニバース一覧」1つにまとめる（下記） | 3系統・2つの確定ボタン・2つの状態表示が1つになる。USB/Art-Net/sACNを同じ表で扱える | 画面＋出力の仕組み・大 |
| P3 | 重複した操作を1か所にし、他はその状態の表示か近道にする | 「どこで押せばいいか」の迷いがなくなる | 画面のみ・中 |
| P4 | 開発者向けの言葉・診断を通常画面から外し、「診断」ページかツールチップへ | 操作者が読むべき文だけが残る | 画面のみ・小〜中 |
| P5 | 英語と日本語の混在をなくす | タブや見出しが統一される | 画面のみ・小 |
| P6 | 常に出ている説明文を「?」ツールチップや初回表示にし、重複した説明を消す | 縦の長さが減る | 画面のみ・小 |
| P7 | 普段押せないボタンは、押せない理由を1行で出すか隠す | 灰色ボタンの列がなくなる | 画面のみ・小 |
| P8 | 1つしかないサブタブの段を消すなど、ナビゲーションの重複を整理 | 画面上部が1段減る | 画面のみ・小 |
| P9 | 置き場所の整理（同期をWebリモートから独立、音声入力をI/Oへ、FXエディタを広い領域へ） | 探す時間が減る | 画面のみ・中 |
| P10 | 使われていない部品（`DmxRoutesPanel.tsx`、`TouchRemotePanel.tsx`）を削除 | 保守が楽になる | コードのみ・小 |

### P2 DMX出力の統合案

現状は3つの仕組みに分かれている。

- 「公演DMX」：USB選択と Prepare（`DmxOutputConfigPanel.tsx:237-277`）
- 「ネットワークユニバース」：下書き・適用・再読込（`NetworkDmxRoutesPanel.tsx:64-96`）
- 「詳細」：Prepare の手順を手で行う操作（`DmxOutputConfigPanel.tsx:279-319`）

内部も「U0 は 127.0.0.1 へのループバック固定、USB はその写し、2つ目以降はネットワークだけ」という作りになっている。
そのため、Art-Net だけの構成でも先に U0 の準備が必要になる。

| 構成 | 現在の操作 |
| --- | --- |
| USB-DMX 1ユニバース | 4クリック。後から挿したデバイスを出すには、RDMツールの中の再スキャンを使うか再起動が必要 |
| Art-Net 1ユニバース | 約7操作＋入力。U0の準備が必須、宛先の初期値は 127.0.0.1、ユニバース番号の初期値は 1 |
| USB U0 ＋ Art-Net 2ユニバース | 約9操作＋スクロール。1080pでもネットワーク行は1行しか見えない |

提案は1つの表にまとめる形。

| ユニバース | 出力先 | 設定 | 状態 |
| --- | --- | --- | --- |
| 1 | USB-DMX | デバイス: FT232R (COM5) | 送信中 32.5fps |
| 2 | Art-Net | 192.168.1.50 / Net 0 Sub 0 Uni 1 | 送信中 44fps |
| 3 | sACN | マルチキャスト | 停止 |

- 行の追加、出力先の変更、「適用」はそれぞれ1つずつにする。準備と適用を分けない。
- USB-DMX を出力先として複数行に選べるようにすれば、複数台のUSB-DMXや2ポートのインターフェースにも、同じ表で対応できる。
- 手動の確認／開始／停止、デバイスの識別情報、ループバックの有効化、テストフレーム、生値モニターは「DMX診断」に移す。
- 内部では、U0 のループバック固定と「USBは U0 の写し」をやめる。各ユニバースに出力先を1つ持たせる形に変える。
  - 対象: `crates/protocol/src/network_dmx_routes.rs`、`crates/engine/src/network_dmx_routes.rs`、`app/src-tauri/src/show_serial_dmx_route.rs`。
  - 出力の持ち主（リース）、ブラックアウト、プロジェクト切り替え時の安全停止の規則は変えない。

### P3 重複の内訳

- **ブラックアウト（6か所）**
  - 上部バー `WorkspaceChrome.tsx:593-617`
  - Edit > 両方 `ControlBothPanel.tsx:103-165`
  - Control > 両方 `TouchBothLivePanel.tsx:41-84`
  - Edit > 映像 `VideoControlOutputsPanel.tsx:62-67`
  - DMX診断 `LightingRuntimeControlsPanel.tsx:66-69`
  - ロック画面 `OperatorLockOverlay.tsx:42`

  上部バーを正とする。Control では本番用に大きいボタンを残し、上部バーと同じ動作にそろえる。DMX診断にある BO／解除・マスター・BPM は削除する。
- **MIDIの接続（3か所）**
  - カード上の「Clockを接続」`IoConnectionDeck.tsx:209-211`
  - MIDIパネル内の「MIDIクロック／MTC」と「MIDIコントロール接続」
  - DMX診断の Scan／Connect `LightingRuntimeControlsPanel.tsx:116-124`

  接続は MIDIパネルの1か所にし、カードは状態表示だけにする。
- **Learn（3経路）**
  - ツールメニューの MIDI／OSC／DMX Learn
  - MIDIパネルの「ControlでMIDIを学習」
  - 3段階の手動マッピングフォーム

  ツールメニュー（ピンクの対象を選ぶ）を主経路にし、手動フォームは「詳細」に置いたままにする。
- **OSC・Webリモート・DJ Link**：開始／有効化ボタンがカードとパネルの両方にある。パネル側だけにする。

### P4 通常画面から外す言葉と診断の例

- Prepare の進捗「1/4 output role Both」「2/4 optional machine-local binding」「4/4 optional Open DMX arm」
  - 場所: `createOutputDiagnosticsController.ts:502-527`
  - 改善: 「出力を準備しています…」の1行にし、失敗したときは次にとる行動を書く。
- 経路情報「wire U0 / logical U0 / S0 engaged / ≈32.5fps」（`DmxOutputConfigPanel.tsx:340-349`）
- DMX診断の Phase 1 スモーク、固定プローブ、JITTER P99／QUEUE DROPS（`OutputDiagnosticsPanel.tsx:61-92`）
- AIアクセスの principal／sidecar／R4／R5、RDMの「development controller UID」

### P5 言語の混在

- 上部の SETUP／EDIT／CONTROL と、Edit の LIGHTING／VIDEO／BOTH／TIMELINE は英語固定（`data-no-localize`）。
  一方、Control の同じタブは「照明／映像／両方」になっている。
- 未翻訳の例: Prepare、No eligible device、DMX diagnostics and test、SECURITY、SELECT、Move over the stage、
  0 fixture(s)、Speed／Size／Phase、FX種別名、Add Colour layer、Plan not checked。

## 進め方の案

| 段階 | 内容 | 理由 |
| --- | --- | --- |
| 1 | P1・P4・P5・P6・P7・P8・P10 | 画面だけの小さな変更で、効果が大きい。Codex の作業とも衝突しにくい |
| 2 | P3・P9 | 画面だけだが、操作の流れが変わるので、本番の操作手順と合わせて確認する |
| 3 | P2（DMX統合）と複数USB-DMX対応 | 出力の仕組みを変えるので、実機テスト（[手順書](PHYSICAL_TEST_DAY_RUNBOOK_2026-10-14.md) のブロックC〜E）とセットで行う |

## 根拠（画面ごとの数）

日本語表示の初期状態で、画面に見えている操作部品の数（下段バンド分を別に記載）。

| 画面 | 見えている部品（上段 / 下段） | 折りたたみ内 | 押せない部品 |
| --- | --- | --- | --- |
| I/O > DMX | 20 / 34 | 57（全部開くと87） | 1 ＋ 下段12 |
| I/O > Webリモート | 16 / 34 | 50 | 0 |
| I/O > MIDI | 17 / 34 | 20 | 0 |
| Edit > 照明 | 16 / 59 | 0 | 6 |
| Control | 24 / 57 | 0 | 6 |
| FXエディタ | 45 / 59 | 29 | 4 |
| Setup > 映像出力 | 約14 / 35 | 124 | 3 |

撮影に使った手順・スクリプト・全画像は作業用フォルダにあり、リポジトリには入れていない。
