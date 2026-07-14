# Syndocal

Syndocal は、DMX照明とVJ映像を同じタイムライン、キュー、BPMクロック、エフェクトソースで駆動するデスクトップ制御ソフトウェアです。

- 製品名: **Syndocal 1.0.0**
- 開発: **Seraf()のKTN**
- プロジェクト: **`.sdc`** (可読JSON)
- Tier 1: Windows 10+ / macOS 12+
- Tier 2: Ubuntu 22.04+ / Arch Linux

## 主な機能

### 照明

- GDTF読込、GDTF Share検索/取得、簡易カスタム`.fixture`プロファイル
- 8/16bit属性、複数ユニバース、重複/範囲外を防ぐDMXパッチ
- 階層グループ、複数所属、Highlight / Solo / Park、マスター/サブマスター
- Art-Net、sACN/E1.31、Enttec USB PRO、DMXKing ultraDMX、Enttec Open DMX
- LFOと3D位置ウェーブ、エフェクトスタック、共有ノードグラフ
- 灯体、投影面、ステージオブジェクトを扱う実用的な2D Stage Map

### 映像

- HAP / HAP Alpha / HAP Qの直接GPU圧縮テクスチャ経路
- H.264 / H.265 / ProResのインプロセスlibavデコード
- PNG/JPEG、可変速、リバース、A-Bループ、キューポイント、BPM同期
- Layer / Composition / Display Output、Normal / Add / Multiply / Screen合成
- 変形、クロップ、色補正、pixelate / blur / glow / edge / color key
- レイヤーごとに最大8段のordered ISF 2 stack。14種の内蔵FXまたは単一pass ISFを追加し、段別に選択、並べ替え、bypass、reset、削除、parameter操作して`.sdc`へ埋め込み保存
- wgpuネイティブ出力、複数Display、投影比率/keystone/lens/corner warp補正
- feature-gated NDI送受信。SDK未導入時は起動失敗せず`NotBuilt`を表示

### 統合制御

- 照明と映像が混在する共有タイムライン
- 1キューで灯体値、映像Layer、Output、Node Graphを同時リコール
- 1つのLFO/位置ウェーブから灯体属性と映像パラメータを同時駆動
- マイクLive FFTの16 band／RMS／Peak／onset／BPM／kick／snareから照明・映像を同時駆動。WindowsではWASAPI sharedに加え、通常MIT版と分離したASIO bridge featureでdriver／sample rate／fixed buffer／channel mixを明示選択できる。切断／250ms無入力時は共有0クリアを要求し、未受理を明示する
- Tap / MIDI Clock / MTC / LTC / Ableton Link用共有クロック境界
- MIDI、OSC、WebSocket、iPad/Android向けPWAリモート
- 10秒間隔の自動Recovery、Recent Project、`.sdc` OS関連付け

## 画面構成

- **Setup**: Lighting / Video / Mapping / I/Oを分離し、パッチ、灯体プロファイル、出力、投影補正を設定
- **Control**: 上段のライブ/キューデスクと、左下の常設2D Stage、右下のLive Edit / Timeline / Mixer
- **Touch**: タブレットやタッチ操作向けのキュー、灯体、映像操作

アプリ本体は一画面内に固定されます。長い灯体ライブラリやリストだけが各パネル内部でスクロールします。

## インストール

CI/Release成果物は次の形式です。

- Windows: `Syndocal_1.0.0_x64-setup.exe` (NSIS)、`Syndocal_1.0.0_x64_ja-JP.msi`
- macOS: `.app`、DMG
- Linux: `.deb`、AppImage

### 未署名ビルド

v1.0の個人配布物はコード署名されていません。

- Windows SmartScreen: ファイルのプロパティに「ブロックの解除」があれば有効にし、警告画面では発行元とハッシュを確認してから「詳細情報」→「実行」を選択します。
- macOS Gatekeeper: Finderでアプリを右クリックして「開く」、または「システム設定」→「プライバシーとセキュリティ」から対象アプリを許可します。

正式な公開配布ではWindows Authenticode証明書とApple Developer ID/Notarizationが必要です。秘密鍵や署名資格情報はリポジトリに保存しません。

## 最初の操作

1. Project menu (`...`) から **Sample** を開きます。
2. Setup / PatchでMini Spot 1、DMX U0 Address 1、2D Mappingを確認します。
3. Setup / I/O / DMXでArt-Net loopback `127.0.0.1:6454` または実機出力を設定します。
4. Controlで灯体を選択し、Dimmer/Color/Positionを操作します。
5. TimelineまたはGOで、照明と映像が同じキューから変化することを確認します。
6. Project menuから`.sdc`として保存します。

Project menuの`UI language`からEnglish／Japaneseを切り替えられます。選択は端末設定として保存され、`.sdc`には混在しません。

同梱サンプルの詳細は [samples/README.md](samples/README.md)、キー操作は [HOTKEYS.md](HOTKEYS.md) を参照してください。

## プロジェクトとプリセット

- `.sdc`: ショー全体。灯体、埋め込みカスタムプロファイル、キュー、タイムライン、映像、出力、Stage Mapを保存
- `.fixture`: 簡易灯体プロファイル
- `.preset`: 灯体属性スナップショット
- `.effect`: LFO/位置ウェーブ
- `.nodegraph`: エフェクトノードグラフ
- `.projmap`: プロジェクター補正
- `.midimap` / `.oscmap`: MIDI / OSCコントロールマッピング共有
- `.sdctemplate`: ショー構成、埋め込みプロファイル、MIDI / OSCマッピングをまとめたユーザーテンプレート。読込時は全出力を無効化

`.sdc` v1は未知フィールドを読み飛ばし、後から追加された省略可能項目を既定値で補完します。将来versionは暗黙変換せず、対応外として明示的に拒否します。

## 開発

必要環境:

- Rust stable (`rust-toolchain.toml`で安定版チャネルを指定)
- Node.js 22
- pnpm 10.9
- FFmpeg開発/共有ライブラリ (一般動画のlibav build)
- 各OSのTauri 2ビルド依存

```powershell
pnpm --dir app install --frozen-lockfile
cargo test --workspace --locked
pnpm --dir app build
pnpm --dir app tauri dev
```

軽量チェック:

```powershell
pnpm --dir app run check:release
pnpm --dir app run check:project-storage
pnpm --dir app run check:release-ui:browser
```

実ウィンドウを含むUIリリース受入（Windowsでは最大化→F11フルスクリーン→Esc復帰を自動検証）:

```powershell
pnpm --dir app run check:release-ui
```

リリース:

```powershell
pnpm --dir app tauri build --ci --bundles nsis,msi
```

Windowsの完全libav bundleでは`FFMPEG_DIR`を共有FFmpeg SDKルートへ設定します。bundle直前にDLLがステージされ、MSI/NSISへ同梱されます。NDIを有効にする場合は別途NDI SDKを導入し、`--features ndi`とSDKのライセンス条件に従ってください。

ASIOは既定buildへ含めません。Windowsのローカル技術検証は、手動取得したSDKを`CPAL_ASIO_DIR`、LLVMの`libclang.dll`を`LIBCLANG_PATH`へ明示してから次を実行します。SDK pinは[qa/ASIO_SDK_PIN.json](qa/ASIO_SDK_PIN.json)、受入と配布境界は[qa/ASIO_INPUT_ACCEPTANCE.md](qa/ASIO_INPUT_ACCEPTANCE.md)を正とします。GPLv3版として分離するかSteinberg proprietary agreementを締結するまで、通常installerへASIO bridgeを同梱しません。

```powershell
& .\qa\harnesses\check-asio-build.ps1
```

## 性能・QA

- Engine: 44Hz、UIとは別スレッド、bounded lock-free command queue
- Windows: Pro Audio MMCSS Critical + 1ms timer、macOS: USER_INTERACTIVE QoS
- 1時間release soak: 108,001フレーム、drop 0、tick jitter p99 0.535ms、command-to-DMX p99 0.476ms、最大13.3MB (ハーネス構成)
- 大規模Engine gate: 200灯体、8ユニバース、100キュー、20,000ターゲット
- UI viewport policy: browser 1920x1080とnative F11 1920x1080を運用・見た目の主ゲートにする。Windows最大化は1920x1032の作業領域内で実client 1920x1009（title barを除く）を確認し、Escで同寸法へ復帰させる。1280x720 / 1366x768はcontainment fallback、2048x1152は拡張上限回帰とし、小画面fallbackの通過だけではデザイン合格にしない
- Fullscreen VJ Focus gate: 1920x1080のfullscreen mode fixtureでは重複する外側headerを除き、Clips／Live Monitors + Outputs／Layersを684／811／405px、Preview／Programを317／476px（Program/Preview 1.502）に配分する。live-audio dockは92px、主操作は32px、設定操作は28pxとし、telemetryの切れ・critical overflowを0に固定する。1366x768／1280x720は従来のheader、64px audio rail、Preview／Program 1:1を維持する。専用identifierのcurrent-source native QA buildでも、最大化1913x1080からF11でexact 1920x1080へ移行し、このFocus配置と実ASIO telemetryを確認後、Escで通常配置へ復帰した
- Populated VJ operator gate: 7 layerを6+1 bankで扱い、各visible layerへBuilt-in FX／選択段のEnable・Bypass／Advancedを固定する。重いISF editorは閉じている間DOM 0件、開いた1 layerだけ最大8段のstack rowと選択段editorをmountする。Event／Bool／Long／Float／Point2D／Color、8/8 Add lock、Event 1→0と履歴不変／busy interlock、選択・focusを保つMove、Remove後の安全な隣接選択、GPU診断更新、段index付きBypass transaction、3 outputのLive／Off／BO rail同期を1920x1080／1366x768／1280x720の英日6ケースで検証し、全操作phaseでunsafe overflow 0／owner外rect 0を要求する
- ISF stack runtime: 有効段を最大8段まで一つのcommand encoderへ順番に積み、frameごとのhost upload 1回、GPU内ping-pong、最終readback 1回で処理する。変換shader／GPU pipelineは最大64件を再利用し、実行段数、stack render time、段別compile errorを診断へ出す。現行のheadless rendererは最終RGBAをCPUへreadbackし、別deviceのnative outputへ再uploadするため、outputまでのzero-copyやTouchDesigner級GPU-resident graphを示すものではない
- Live audio UI: VJ Desk内の通常／compact 64px railとfullscreen VJ Focusの92px dockでStart／Stop、device／rate／requested buffer／channel mix、16対数band、RMS／Peak、onset、BPM／confidenceとcallback／queue telemetryを常時確認する。軽量feature statusは約30Hz、詳細telemetryは約1Hzでsingle-flight更新し、5 viewport×英日stateful gateで固定する
- Live-audio viewport gateはDOMを直接改変せず、Tauri invoke mockから実Solid stateを駆動する。5解像度×英日でRefresh、generation ID再対応、同名曖昧時の再選択ロック、capability、192kHz／8192-frame／stereo pair Start request、42/58/76% meter、live telemetry、Stop／clear-pendingまで検証する
- Windows native live-audio acceptance: 最大化したcurrent-source buildでsystem defaultのWASAPI shared入力を48kHz monoでStart／Stopし、実callback 480/480/480 frames、capture-to-workerの採取時点current 0.1ms／max 10.1ms、OVR 0 chunks／0 frames、queue current/high-water/capacity 0/1/4を確認した。この1構成の測定をASIOや長時間・他deviceの性能証明には読み替えない
- Windows ASIO short smoke: 分離bridgeとapp loaderから`TOPPING Pro USB Audio Device`を明示指定し、48kHz／2ch／i32／requested・applied 128 framesでStart／Stopした。7 callbacks／896 frames、max capture delay 4166.7us、XRUN 0、nonfinite 0、terminal event 0。`Realtek ASIO`はhardware unavailableとして失敗し、別driverやWASAPIへfallbackしなかった。これは1台・短時間の構成確認であり、第二の正常driver、1時間soak、物理pixel latency、TouchDesigner同条件A/Bを満たさない
- Windows ASIO 100-cycle: bridgeのignored hardware testを`--locked --offline`で実行し、明示ID `asio:TOPPING Pro USB Audio Device`、48kHz／2ch／i32／128 framesを27.6767468秒で100回Start／Stop／Freeした。actual bufferは全回128、callback 200（各回2以上）、Stop 100、Free 100、warning／terminal／XRUN event・API／nonfinite／frame mismatchは全て0で、別driver／WASAPIへのfallbackも0だった。これは反復open/close gateの合格であり、第二の正常vendor、1時間soak、hot-plug、物理pixel latency、TouchDesigner同条件A/B、配布licenseの合格ではない
- Windows ASIO native UI: current-source QA buildを最大化し、F11で実1920x1080へ移行してから、VJ DeskでASIO／`TOPPING Pro USB Audio Device`／48.0kHz／128 frames／Average All→monoを明示選択した。Startで`ACTIVE`、OVR 0/0、XRUN 0、Stopで`READY`を確認し、Escで最大化windowへ復帰した。full native operator pathは合格だが、1時間soak、第二vendor、hot-plug、物理pixel latency、TouchDesigner同条件A/B、配布licenseは未合格である
- Auto VJ: CLOCKまたはLIVE INPUTのonsetをsourceに、seed／show revision／ordered candidateから再現可能なTakeを生成する。手動Take、Hold、blackoutを優先し、Program Audioはbackend coordinatorが世代token付きで一度だけ切り替えるため、device openやdecodeでTake操作を待たせない

検証記録:

- [qa/M4_IO_VALIDATION.md](qa/M4_IO_VALIDATION.md)
- [qa/M5_RELIABILITY_VALIDATION.md](qa/M5_RELIABILITY_VALIDATION.md)
- [qa/M6_RELEASE_VALIDATION.md](qa/M6_RELEASE_VALIDATION.md)
- [qa/NATIVE_WINDOW_ACCEPTANCE.md](qa/NATIVE_WINDOW_ACCEPTANCE.md)
- [qa/LIVE_AUDIO_FAIL_CLOSED.md](qa/LIVE_AUDIO_FAIL_CLOSED.md)
- [qa/AUDIO_REACTIVE_VJ_ACCEPTANCE.md](qa/AUDIO_REACTIVE_VJ_ACCEPTANCE.md)
- [qa/ASIO_INPUT_ACCEPTANCE.md](qa/ASIO_INPUT_ACCEPTANCE.md)
- [qa/VJ_OPERATOR_DESK_ACCEPTANCE.md](qa/VJ_OPERATOR_DESK_ACCEPTANCE.md)
- [qa/TOUCHDESIGNER_COMPETITIVE_AUDIT.md](qa/TOUCHDESIGNER_COMPETITIVE_AUDIT.md)
- [RELEASE_STATUS.md](RELEASE_STATUS.md) - v1.0完成判定、外部受入、次スレッド向けバックログ

## 既知の制限

- v1.0以降の外部映像I/OはDisplay、feature-gated NDI、Windows x86_64のSpoutに対応しています。Syphonはwgpu世代差とmacOS実装環境が必要なため未実装です。
- HAP Q Alpha、HAP R/BC7のGPU直接sampling + CPU fallback、安全境界付き最大8段のordered single-pass ISF stackは実装済みです。1 stackのsource合計は512 KiB、1段16 control、project内ISF source合計16 MiB、`.sdc`全体64 MiBに制限します。ISF multipass／persistent buffer／imported resource／audio input、100種級library、shared texture／任意node graph、headless readback後のnative別device再upload解消、4K multi-layer／実会場long soakは未対応または未受入です。
- Live FFT入力はCPALのdevice catalogとcapabilityを読み、Windowsの既定buildではWASAPI sharedのsystem defaultまたは列挙device、sample rate、requested buffer、全channel平均／単一channel／stereo pair downmixを選べます。選択rateごとにchannel数／sample format／buffer capabilityを`resolved_config`として先に確定し、Startでも同じ構成を再検証します。更新世代に結び付くopaque device IDを使い、古い／未知IDや非対応構成は別device・rate・bufferへ黙ってfallbackせずStartを拒否します。全11種のCPAL PCM sample formatを正規化し、通常data callbackは4本×2048-frameの事前確保slotへallocation-freeで格納します。workerは16対数band、RMS／Peak、spectral flux、適応onset、60–200 BPM／confidence／beat phase、centroid／density／kick／snareを生成し、同じframeから旧Node Graph用B/M/Hを派生します。Compiled Audio Reactive Rackは任意の対応照明属性／映像parameterへこれらを割り当て、Auto VJはCLOCKまたはLIVE INPUTのonsetを選び、seed／show revision／candidate順から決定的にTakeします。非既定のWindows ASIO featureは独立bridgeを動的loadし、明示driver／rate／channel／native format／fixed bufferをStart時に再検証、実buffer framesとbackend XRUNを表示します。TOPPINGの短時間実機smoke、100回Start／Stop／FreeとF11 1920x1080のfull native UI gateは通過しましたが、ASIOの配布ライセンス選択、第二の正常driver、同一device hot-plug自動復帰、engine出力適用ack、実機1時間／物理pixel latency／TouchDesigner同条件比較は未完です。通常MIT installerへASIO bridgeは同梱しません。
- Device Refresh後はbackendとdevice名が新旧catalogueの双方で一意な場合だけ明示選択を新IDへ引き継ぎます。曖昧・消失時はsystem defaultへ黙って落とさず、再選択するまでcapabilityを破棄してStartをロックします。
- 3Dビジュアライザは本体UIへ戻さず、`visualizer`データ境界から外部実装へ接続します。標準UIは2D Stage Mapです。
- Enttec Open DMXはOS/USBドライバ依存のbreak timingがあるため、最終現場ではUSB PRO系を推奨します。
- Art-Net/sACN/Serialの実機遅延は接続機材ごとに再測定してください。リポジトリの自動検証はloopback中心です。
- NDI SDK、コード署名、macOS notarizationは配布ライセンス/資格情報が必要で、リポジトリには含みません。

## ライセンス

Syndocal本体は[MIT License](LICENSE)です。一般動画デコード用の配布物は、
GPLコンポーネントを無効化したFFmpeg 8.1共有ライブラリを動的リンクします。
詳細とLGPLv3本文は[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)を参照してください。
