# 実機テスト日 手順書（alpha.81以降）

作成: 2026-10-14。基準: `main` の alpha.81（FXスクロール／ネットワーク複数ユニバース）。

この手順書は、[完了台帳](SYNDOCAL_COMPLETION_LEDGER.json) で実機確認が残っている項目を、
1日の試験で前に進めるための手順をまとめたものです。各項目の合格条件の原本は、
項目ごとの `*_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md` と
[完了フロー](SYNDOCAL_COMPLETION_FLOW_2026-08-19.md) の第6〜8節です。
この手順書は合格条件を変えず、当日の段取りだけを定めます。

1回の実施だけで台帳の項目を Complete にすることはありません。
結果は各項目のチェックポイント文書に追記し、条件をすべて満たしたかは後で判定します。

## 0. 当日の範囲

| ブロック | 内容 | 進む台帳項目 | 自宅・スタジオで可能か |
| --- | --- | --- | --- |
| A | 準備と起動確認 | 全項目の前提 | 可 |
| B | MIDI入力（SMC-Mixer） | AI3-NATIVE-INGRESS-001, INPUT-PHYSICAL-001 | 可 |
| C | USB-DMX と ブラックアウト | DMX-USB-RDM-001（一部） | 可 |
| D | 出力の退避と再Arm（New／Open） | AI3-PHYSICAL-REARM-001（一部） | 可 |
| E | Art-Net／sACN 複数ユニバース | DMX-ARTNET-001 | Art-Net/sACNノードがあれば可 |
| F | 映像（実ディスプレイ・カメラ・NDI/Spout） | VIDEO-PHYSICAL-001（一部） | 可（NDI/Spoutは無料ツールの導入が必要） |
| G | OSC／TouchOSC／Webリモート | INPUT-PHYSICAL-001 | スマホ・タブレットがあれば可 |
| H | アクセシビリティ | ACCESSIBILITY-NATIVE-001 | 可 |

**当日は扱わない項目と理由**

- **ASIO（6項目）**: 3つの理由で今は試験できません。
  - 配布形態（GPLv3で分離した成果物にするか、SteinbergのSDK契約を結ぶか）が未決定です。
  - このPCには ASIO ビルドに必要な Visual Studio Community 14.44・libclang・固定版SDK zip がありません。
  - インストール版 alpha.81 には ASIO ブリッジが含まれていません。
  詳細と準備事項は第13節を参照してください。
- **DJ Link（DJ-LINK-HARDWARE-001）**: DJ PC、rb-output peer、rekordbox、Stream Deck Pedal の構成を作り直す必要があるため、別の日に [DJ_HW4_OPERATOR_RUNBOOK_2026-08-27.md](DJ_HW4_OPERATOR_RUNBOOK_2026-08-27.md) で行います。
- **SHOWCLOCK-VENUE-001**: PC 2台と有線スイッチが必要です。
- **VENUE-SOAK-001**: 基準機（RTX 5090）とGPUログが必要です。
- **COMPARE-PINNED-001**: Daslight／SynapseRack／TouchDesigner のライセンスと、比較方法の決定（DEC-COMPARE-001）が先です。
- **AI8-EXTERNAL-ACCEPTANCE-001**: クリーン環境と署名の扱いが、完了フロー第9節（配布は対象外として凍結）と食い違っているため、方針決定が先です。

## 1. 共通ルール（必ず守る）

- DMXを実際に出す試験・機器を開く試験は、使い捨てのローカル試験ではありません。操作者が機材の前にいるときだけ行います（[AGENTS.md](../AGENTS.md)）。
- **レーザー（F3200A）は全512チャンネル0のフレーム以外を送らないこと。** 光路の安全確認とチャンネル試験の承認が済むまで、S0（安全ブラックアウト）を解除しません（[DSF2026_SAME_PC_OUTPUT_ACCEPTANCE_2026-08-29.md](DSF2026_SAME_PC_OUTPUT_ACCEPTANCE_2026-08-29.md)）。第5節の `physical_serial_rainbow_demo` は ch1〜121 に0以外の値を送るので、レーザーをつないだまま実行してはいけません。
- ブラックアウトの操作:
  - 上部バーの `DMX` / `VID` / `ALL` は通常のブラックアウトです。
  - S0（安全ブラックアウト）はこれとは別の仕組みです。`Stop USB-DMX` は S0 がかかっている間だけ使えます。
- Daslight や他のチェックアウトの `syndocal.exe` を終了させてはいけません。
- 最初の失敗で止まり、そのときの状態を記録します。やみくもに再試行しません。特にドライバが固まったときは再試行しません。
- ソフト上のループバック（loopMIDI、127.0.0.1 宛て Art-Net など）の結果を、実機の合格として扱いません。
- 同時に2つのことを確認しない。MIDIクロック／MTC と MIDIコントロール、DJ Link と Webリモートは、それぞれ別の手順で試します。

## 2. 前日までの準備

### 2.1 試験対象のアプリ

- Codexが作成中の alpha.81 配布パッケージを使います。パッケージがまだなら、そのチェックアウトの `pnpm --dir app tauri build --no-bundle` で作った `target/release/syndocal.exe` を使います。
- 当日の最初に、PowerShellで実行ファイルの識別情報を記録します（ブロックA）。

### 2.2 機材チェックリスト

| 機材 | 用途 | ブロック | 備考 |
| --- | --- | --- | --- |
| FTDI USB-DMX ケーブル（Open DMX型） | USB-DMX出力 | C, D | デバイスマネージャーでCOM番号を確認（過去は COM3／COM5） |
| 灯体（4ch Dimmer/RGB、121ch リグ など） | 目視確認 | C, D, E | レーザーは0フレームのみ |
| SMC-Mixer（USB） | MIDI入力 | B | ボタンを1つ試験用に決めておく |
| Art-Net／sACN ノードと灯体 | 複数ユニバース | E | ノードの機種・ファームウェアを記録 |
| 2台目のPC（任意） | Art-Net受信の監視 | E | `qa/harnesses/artnet-monitor.mjs` を動かす |
| DMXアナライザ／ロジックアナライザ（任意） | Open DMX のタイミング | C | 無ければタイミング計測は次回 |
| ディスプレイ 2〜3台、HDMIケーブル | 映像出力 | F | 3台目（LKGF803）は接続が不安定だった記録あり |
| カメラ（ASUS 5M／Insta360 Link） | カメラ入力 | F | 4K30・1080p60 は Insta360 Link が必要 |
| NDI 6 Tools、Spout受信アプリ | 映像の送受信 | F | 事前にインストール（無料） |
| スマホ／タブレット（TouchOSC、ブラウザ） | OSC・Webリモート | G | 有線と Wi-Fi の両方を試す |
| NVDA（導入済み） | 読み上げ | H | Speech Viewer を使う |
| 動画を撮れるスマホ等 | 手元と画面の同時記録 | 全体 | MIDI・DMXの「動かした」証拠になる |

### 2.3 cargo の試験を使う場合（ブロックB・C・F の一部）

[WINDOWS_NATIVE_BUILD.md](WINDOWS_NATIVE_BUILD.md) のリンカ固定が必要です。新しく開いた `cmd` で、1行ずつ実行します。

```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" -vcvars_ver=14.44
set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe"
where link.exe
```

`where link.exe` の1行目が上の `14.44.35207` のパスであることを確認します。Git の `usr\bin\link.exe` が先に出たら、その窓は使わず開き直します。
他のエージェントがビルド中のチェックアウトでは cargo を実行しません（`target` を共有するため）。

### 2.4 記録の置き場所

- 動画・写真などの大きな生データ: `target/qa/physical-<日付>/`（リポジトリには入れない）
- ログ・JSON・要約: `qa/artifacts/physical-<日付>/`（後でコミット）
- 各ブロックで、開始・終了の時刻（壁時計）を必ず記録します。

### 2.5 当日までに決めておくこと

合格の数値が文書に無い項目があります。「空欄の閾値は不合格」とする方針があるため（[RELEASE_DECISION_2026-08-12.md](RELEASE_DECISION_2026-08-12.md)）、試験前に第12節の未決事項を決めておきます。

## 3. ブロックA: 準備と起動確認（約15分）

1. 試験に使う `syndocal.exe` の識別情報を記録します。

   ```powershell
   $exe = "<試験に使う syndocal.exe のフルパス>"
   (Get-Item $exe).VersionInfo | Select-Object ProductVersion, FileVersion
   Get-FileHash $exe -Algorithm SHA256
   (Get-Item $exe).Length
   ```

2. アプリを起動し、`Syndocal` ウィンドウがちょうど1つ、応答していることを確認して最大化します。
3. 新規の空プロジェクト（保存しない）を開きます。すべての出力が無効で、ブラックアウトがかかっていることを確認します。
4. MIDIポートの一覧を記録します。過去にポート一覧が実行ごとに変わったことがあるため、毎回最初に取ります。出力を開かずに一覧だけ出す方法は、わざと一致しない名前を入力に指定する方法です（第2.3節の窓で実行）。

   ```bat
   set SYNDOCAL_TEST_MIDI_INPUT=__ENUM_ONLY_NO_MATCH_INPUT__
   set SYNDOCAL_TEST_MIDI_OUTPUT=__ENUM_ONLY_NO_MATCH_OUTPUT__
   cargo test -p io --release --locked -j 1 physical_midi_ports_enumerate_open_and_send_feedback -- --ignored --nocapture --test-threads=1
   ```

   入出力の一覧を表示した後、一致しないため何も開かずに失敗終了（exit 101）すれば正常です。
   **入力・出力とも一致する名前を指定してはいけません。** このテストは出力に `B0 7B 00`（All Notes Off）を送ります。

## 4. ブロックB: MIDI入力（約45分）

対象: AI3-NATIVE-INGRESS-001。合格に必要なのは次の3つです（[AI3_NATIVE_INGRESS_RECHECK_2026-09-22.md](AI3_NATIVE_INGRESS_RECHECK_2026-09-22.md)）。

- 操作者が動かしたことを確認できる、生のMIDIデータ
- Syndocalで意図した入力が選ばれていること
- 出力を伴わない割り当て（タップBPM）が、アプリの経路で反応したこと

### 4.1 何も触らない状態の記録（ベースライン）

過去に、誰も触っていないはずの15秒間に CC が308件記録されたことがあります。先に「触らない30秒」を記録します。

```bat
set SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer
set SYNDOCAL_TEST_MIDI_CAPTURE_SECONDS=30
cargo test -p io --release --locked -j 1 physical_midi_input_captures_operator_ingress -- --ignored --nocapture --test-threads=1
```

- このテストはクロックやアクティブセンシングも含めてすべて記録します。何か届いただけでは「動かした証拠」になりません。
- 名前は部分一致で、最初に一致したポートが選ばれます。別のポートを使うときは `MIDIIN2 (SMC-Mixer)` や `SMC-Mixer-bt` のように完全な名前を指定します。

### 4.2 アプリでの手順

1. 空の未保存プロジェクトで、Setup > I/O を開きます。
2. **MIDI カードの見出し部分をクリック**して開きます。カードの主ボタンは `Connect clock` です。9/22の試験ではこれを誤って押しました。押さないこと。
3. 入力の選択欄で `SMC-Mixer` を明示的に選びます。未選択だと一覧の先頭（`CustomMIDI1`）が使われます。
4. 手動割り当てを開き、Capture → Action に `Tap BPM`（タップBPM）→ Add の順に操作します。
   - Capture は10秒待ちます。その間に試験用の**ボタン**を押します。値が0より大きいと発火するため、エンコーダ（値 `41` を送り続ける）を割り当てると連打扱いになります。
5. 割り当て数が1になったことを確認し、`Connect MIDI Control`（MIDIコントロール接続）を押します。割り当てが無いと押せません。
6. 2人目が見ている状態で、2秒以内に一定の間隔でボタンを3〜4回押します。BPM表示が変わることを確認します。
7. 生データも取る場合は、別の窓で 4.1 のコマンドを `SYNDOCAL_TEST_MIDI_CAPTURE_SECONDS=30` で同時に動かし、開始・終了の時刻を記録します。
   アプリとこのテストが同じ入力を同時に開けるかは未確認です。開けなかった場合はその結果を記録し、アプリ側の観察だけで次に進みます。

### 4.3 してはいけないこと

- シーン用（ピンクの対象）の Learn を使わない。出力 `SMC-Mixer` に自動接続してフィードバックが有効になります。
- `CustomMIDI1` に何も送らない。
- `TimelinePlay` と `EffectEnabled` は、MIDIからは設計上拒否されます。試験用の割り当てに使わない。
- タップBPM はタイムラインの再生権限を無効にする副作用があります。空のプロジェクトでだけ使います。

### 4.4 記録するもの

- ポート一覧（番号つき）
- 生のバイト列と、`midir` のタイムスタンプおよび壁時計の時刻
- UIで選んだ入力名、割り当てのJSON、`MIDI control connected with 1 mapping(s).` のメッセージ、操作前後のBPM
- 製品バージョンと実行ファイルの SHA-256
- 手元と画面を同時に撮った動画

### 4.5 余裕があれば（INPUT-PHYSICAL-001）

- **Clock／MTC**: コントロールを切断した状態で行います。Setup > I/O > MIDI > `Connect MIDI Clock / MTC` で、DAWなどの送信元を `CustomMIDI1` 以外の専用 loopMIDI ポート経由で接続します。生データの記録には 4.1 のコマンドが使えます（`F8` がクロック、`F1` がMTCクォーターフレーム）。
- **フィードバック／All Notes Off**: 「今から送る」と宣言してから、専用の手順で行います。過去のLED確認では、ノート 0／8／16／24 をベロシティ127で送りました（[M4_IO_VALIDATION.md](M4_IO_VALIDATION.md)）。

## 5. ブロックC: USB-DMX とブラックアウト（約45分）

対象: DMX-USB-RDM-001 のうち Open DMX の部分と、ブラックアウトの回帰確認。

### 5.1 送信の動作確認（灯体の目視）

レーザーを外し、4ch／121ch の灯体だけをつないだ状態で行います。

```bat
set SYNDOCAL_PHYSICAL_SERIAL=COM5
set SYNDOCAL_PHYSICAL_SECONDS=30
set SYNDOCAL_PHYSICAL_MASTER=20
cargo test -p engine --release --locked -j 1 physical_serial_rainbow_demo_drives_master_dimmer_and_rgb_cells -- --ignored --nocapture --test-threads=1
```

- `COM5` は当日のデバイスマネージャーの番号に合わせます。
- 出力に `demo running … sends=` の行が出ていることを必ず確認します。このテストは、ポート指定が空だと何もせずに「成功」で終わります（`Skipping physical serial demo`）。
- このテストが示すのは「COMポートが開き、送信回数が増え、失敗が0」ということだけです。波形、アプリの出力権限（リース／S0）の経路、RDM は対象外です。
- 終了時に0フレームを送らないため、終わった後の灯体の状態を記録します。

### 5.2 アプリからの出力

1. Setup > I/O > DMX > `Show DMX`（公演DMX）で USB-DMX のデバイスを選び、`Prepare` を押します。
   予備の経路は Advanced > `Manual controls`（手動操作）の Confirm / Start / Stop USB-DMX です。
2. Enttec USB Pro は FTDI と同じID（0403:6001）なので、ここで選べてしまい Open DMX として駆動されます。**USB Pro は選ばないこと。** 現在のUIには USB Pro／DMXKing で出力する経路がありません（第12節）。
3. 灯体が点くことを確認し、次のブラックアウト回帰を順に確認します。
   - ブラックアウトをかける → 灯体が消える → 解除する → 元の状態に戻る。
   - USB-DMX を有効にした直後でもブラックアウトがかかる（alpha.78 で修正）。
   - Safe BO 解除で `OutputControl rejected (invalid_request)` が出ない（alpha.73 で修正）。期待される動作は、S0 の元の状態が保たれ、最初に0を書き込むことです。
   - 過去に「Open DMX の Arm が 4回中4回 `publication_failed`」になった記録があります。再現するかを確認します。

### 5.3 タイミング計測（アナライザがある場合）

- Open DMX の期待値は、フレーム周期 約30.76ms（約32.5fps）です。44Hz ではありません（[SERIAL_DMX_COMPATIBILITY.md](SERIAL_DMX_COMPATIBILITY.md)）。
- 合格条件は Break 176µs、MAB 16µs と、フレーム周期・失敗の記録です（完了フロー 第8節）。
- RS-485 側へのプローブの当て方は事前に決めておきます（第12節）。

### 5.4 RDM（ゲートウェイがある場合のみ）

Setup > I/O > DMX > `RDM tools`（RDMツール）で行います。

- Gateway IP の初期値は 127.0.0.1 です。実機のIPを入力します。
- USB での探索中は、そのポートのDMX出力を無効にしておく必要があります。
- `Force Full Discovery` は DMX出力を一時的に途切れさせることがあります。
- `SET` は灯体に実際に書き込みます。書き込んでよいPIDを事前に決めます。

### 5.5 記録するもの

- COM番号、FTDIのデバイスID、灯体の種類とアドレス
- テストの出力全文（`sends=` の行を含む）
- 各操作の時刻と灯体の状態（動画）
- アナライザの生データ（ある場合）
- RDM: 経路、ファームウェア、UID／PID、探索時間、ACK／NACK／タイムアウト

## 6. ブロックD: 出力の退避と再Arm（約45分）

対象: AI3-PHYSICAL-REARM-001 のうち New と Open の経路。Take Over は PC 2台が必要なので対象外です。

期待される動作（[PROJECT_REPLACEMENT_PREFLIGHT_2026-10-04.md](PROJECT_REPLACEMENT_PREFLIGHT_2026-10-04.md)）:
有効な New／Open では、両方のゲートを閉じ、全0のフレームを送ってから送信を止めます。
Open は S0 を保ったままにします。明示的にブラックアウトを解除すると、作成済みの出力に戻ります。

1. USB-DMX を出力中の状態にします（ブロックC 5.2）。
2. `New`（新規）を実行します → 灯体が0になり、送信が止まることを確認します。
3. 再Arm（Prepare → Start）して、出力が戻ることを確認します。
4. 保存済みプロジェクトを `Open`（開く）で開きます → 0になり、S0 が保たれることを確認します → ブラックアウトを明示的に解除 → 作成済みの出力に戻ることを確認します。
5. バックアップからの復元と復旧チェックポイントは、UI上の場所を当日確認します。見つからなければ、その旨を記録して次回に回します。

記録するもの（経路ごと）: 操作前後の出力の持ち主、プロジェクト／リースの世代、ブラックアウトの状態、操作者の確認、再Armの結果、灯体の観察。

## 7. ブロックE: Art-Net／sACN 複数ユニバース（約60分）

対象: DMX-ARTNET-001。alpha.81 で追加した複数ユニバースの、初めての実機確認です。

### 7.1 設定

1. Setup > I/O > DMX で、まず U0 を `Prepare` します。U0 が未準備だと「Prepare U0 output before applying network universes.」と表示されます。
2. `Network universes`（ネットワークユニバース）で `+ Art-Net`（Art-Netを追加）または `+ sACN`（sACNを追加）を押し、Universe・`Destination`（出力先）・Port を設定して `Apply network routes`（ネットワーク出力を適用）を押します。食い違いが出たら `Reload routes` を押します。1ページに5経路まで表示されます。
3. **追加した経路は必ず出力先を書き換えます。** Art-Net の初期値は `127.0.0.1` なので、そのままではノードに届きません。sACN の初期値は `multicast` です。
4. Universe の対応:
   - Art-Net の Universe 欄は 15ビットの Port-Address（0〜32767）です。ノード側の Net／Sub-Net／Universe に変換して合わせます。
   - sACN は 1〜63999 で、`multicast` は 239.255.上位.下位 になります。
   - ブロードキャスト宛ては未検証なので、ユニキャストを使います。
5. 各灯体のユニバースを、経路のユニバースに合わせてパッチします。

### 7.2 確認すること（完了フロー 第8節の項目）

アドレス、RGB／ホイール、Pan／Tilt、明るさ、ストロボ、ブラックアウト、再接続（ノードのLANを抜き差しして戻るまでの時間）、複数ユニバース、スイッチ構成、44Hz の連続性。

### 7.3 受信の監視（2台目のPCがある場合）

```bash
node qa/harnesses/artnet-monitor.mjs --self-test
node qa/harnesses/artnet-monitor.mjs --artnet-port 6454 --http-port 6455 --duration-seconds 600 --capture-changes --max-transitions 1024 --evidence qa/artifacts/physical-<日付>/artnet-monitor.json
```

- Art-Net 専用です。sACN の受信監視ツールはありません（必要なら Wireshark）。
- UDP 6454 は1つのアプリしか受信できません。監視PCでは Daslight／Unity を閉じてから動かします。
- Syndocal を動かしているPC自身のIPで受信しても、配送の証拠になりません。
- 以前のソフト上の基準値は「44フレーム/秒、最大の間隔25ms、シーケンスの欠落0（20秒間）」でした（[ARTNET_EXTERNAL_VISUALIZER_ACCEPTANCE.md](ARTNET_EXTERNAL_VISUALIZER_ACCEPTANCE.md)）。実機での時間と閾値は第12節で決めます。

### 7.4 記録するもの

ノードの機種とファームウェア、Port-Address の対応表、監視ツールのJSON、灯体の観察（動画）、再接続にかかった時間、`DMX diagnostics and test` の `Save telemetry report` の出力。

## 8. ブロックF: 映像（約60分）

対象: VIDEO-PHYSICAL-001 の一部。1時間連続の試験と GPU リセットの試験は、そのための仕組みが無いので今回は対象外です。

1. 表示環境を記録します。Windows の 設定 > システム > ディスプレイ で、各ディスプレイの番号・解像度・倍率・リフレッシュレートを撮影します。
   `qa/harnesses/run-syndocal-three-display-show-acceptance.ps1` は、実行ファイルのハッシュやディスプレイの識別名をすべて指定して使う別の試験（3画面の本番配置）用です。この試験では使いません。
2. 出力ウィンドウを、それぞれ特定のディスプレイに割り当てます。解像度、リフレッシュレート、DPI、画面が真っ黒でないこと、を記録します。
3. 抜き差し、並び替え、フルスクリーン切り替えを1つずつ行い、どう反応したかを記録します。
4. カメラ: 抜いたときに、手動で I/O を確認しなくても対処方法の分かるエラーが出ることを確認します。4K30／1080p60 の行は Insta360 Link で行います。
5. NDI／Spout: 事前に入れた受信アプリで、受信名・解像度・fps を記録します。

- 試験の前に、フレーム落ちの上限、GPU温度、映像と音のずれの上限を数値で決めておきます（空欄は不合格扱い）。
- 記録するもの: ディスプレイの識別名・モード・DPI・リフレッシュレート、出力IDとウィンドウの対応、画面の写真、受信側の情報、障害の時系列、診断のエクスポート。

## 9. ブロックG: OSC／TouchOSC／Webリモート（約45分）

対象: INPUT-PHYSICAL-001。DJ Link が有効な間は通常の Webリモートを開始できないので、DJ Link は使いません。

1. OSC: Setup > I/O の OSC カードで、待ち受けIPとポート（初期値 9000）を設定して開始します。TouchOSC から送り、反応を確認します。
2. Webリモート: `Web Remote`（Webリモート）カードで、ポート（初期値 9100）を設定して開始し、表示されたPINでペアリングします。iPad／Android、有線と Wi-Fi の両方で試します。
3. 遅延: 測り方の仕組みがまだありません。今回は「操作から画面の反応まで」を動画で撮り、後でコマ送りで測ります（第12節で正式な方法を決める）。

記録するもの: 端末とアプリのバージョン、接続方式、再接続にかかった時間、動画、操作者の所見。

## 10. ブロックH: アクセシビリティ（約45分）

対象: ACCESSIBILITY-NATIVE-001。UI Automation の構造、キーボードで Prepare → いいえ まで行く経路、ハイコントラスト、200%表示は確認済みです。残りは次のとおりです。

1. NVDA を起動し、Speech Viewer で読み上げ内容を記録しながら、主要な画面を順に移動します。
2. 表示倍率 125% と 150% で、文字の切れやはみ出しが無いかを確認します。
3. 日本語IMEで、プロジェクト名やシーン名を入力します。
4. Windows の「アニメーション効果」をオフにした状態で、動きが抑えられるかを確認します。
5. キーボードだけで、危険な操作（ブラックアウト解除など）と復旧の流れを最後まで行えるか。ダイアログを閉じたとき（Esc 含む）、フォーカスが元の場所に戻るか。

- システム設定（ハイコントラスト、倍率、アニメーション）は操作者が変更し、終わったら必ず元に戻します。出力は無効のままにします。

## 11. 終了手順

1. すべての出力にブラックアウトをかけ、USB-DMX とネットワーク出力を停止します。
2. MIDI／OSC／Webリモートを切断・停止します。
3. アプリを終了し、プロセスが残っていないことを確認します。
4. 生データを `target/qa/physical-<日付>/` に、要約を `qa/artifacts/physical-<日付>/` に置き、要約ファイルの SHA-256 一覧を作ります。
5. 結果を各項目のチェックポイント文書に追記します。合格・不合格・未実施を分けて書き、未観察のことは「未観察」と明記します。

## 12. 当日までに決めること（未決事項）

| # | 内容 | 関係するブロック |
| --- | --- | --- |
| 1 | タップBPM で十分か、それとも無効な試験用灯体へのフェーダー割り当て（アプリ側に記録が残る）にするか | B |
| 2 | 生データの記録を `midir` の直接記録で足りるとするか、アプリ側のログが必要か | B |
| 3 | 遅延の測り方とサンプル数（p50／p95／p99／最大） | B, G |
| 4 | MTC の送信元とポート | B |
| 5 | Art-Net の連続性を見る時間と、最大間隔の閾値 | E |
| 6 | 「スイッチ構成」の意味（管理スイッチ、IGMPスヌーピングの有無など） | E |
| 7 | 再接続の合格時間 | E |
| 8 | USB-DMX の「長時間」の長さ | C |
| 9 | 「出力状態の確認」を、アナライザで取るか目視で足りるとするか | C, D |
| 10 | RDM で書き込んでよいPID と、使うコントローラUID | C |
| 11 | 映像のフレーム落ち・GPU温度・映像と音のずれの上限 | F |
| 12 | Enttec USB Pro／DMXKing の出力経路をUIに用意するか（現状は無い） | C |

## 13. ASIO の準備事項（試験の前提）

- 決めること: ブリッジDLLを GPLv3 の別成果物として配る（ソースと表記を同梱）か、Steinberg の ASIO SDK 契約を結ぶか（[ASIO_INPUT_ACCEPTANCE.md](ASIO_INPUT_ACCEPTANCE.md)）。
- 決定前でも、同じPC限定の `show-asio` ビルド（配布不可）で予行はできます。ただし、各項目の合格は承認済みの成果物で取る必要があります。
- このPCに必要なもの:
  - Visual Studio Community 2022 の MSVC 14.44（ASIO のビルドとソークの準備スクリプトは Community のパスしか受け付けない）、または Build Tools も受け付けるようスクリプトを直す
  - libclang
  - 固定版 `ASIO-SDK_2.3.4_2025-10-15.zip`
- 試験に使う機器の候補:
  - MOTU M Series（48／96kHz の主候補）
  - DDJ-FLX10（2社目のベンダー）
  - HOTONE は 44.1kHz 専用です。TOPPING は「hardware is malfunctioning」で失敗した記録があり、ドライバ名も変わっています。
- ソークの時系列、callback のパーセンタイル、映像までの遅延を記録する仕組みはまだありません。先に作る必要があります。
