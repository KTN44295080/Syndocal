# Syndocal v1.0 Release Status / Next Thread Handoff

Updated: 2026-07-13
Branch: `codex/syndocal-v1.0`  
Original completion commit: `6c3de12`
Production-quality baseline commit: `edfc93c`
Final cross-platform CI: [run 29179218727](https://github.com/Seraf0-org/Rayard/actions/runs/29179218727)

## 結論

Syndocal v1.0は、`COMPLETION_PLAN.md`で定義した**ソフトウェア完成条件を満たしたリリース候補**である。

- 照明、映像、共有タイムライン/キュー/エフェクト、2D Stage Map、`.sdc`保存/復元、外部I/O境界を実装済み。
- Windows/macOS/LinuxでRust、libav、フロントエンド、Tauri、配布物生成とパッケージ起動CIがgreen。
- WindowsではNSISの実インストール/起動/アンインストール、MSIの内容、`.sdc`関連付けを確認済み。
- 1時間ソーク、クラッシュリカバリ、大規模ショー、DMXテレメトリ、複数解像度の一画面UIゲートを通過済み。

ただし、これは「全機材・全会場で検証済み」「署名済み一般公開製品」「今後追加不要」という意味ではない。以下の外部受入と公開運用は、コード完成とは分けて扱う。

## 一般公開・本番投入前の必須確認

### P0: 実機受入

- [ ] Art-Netノードと実灯体で、アドレス、色、Pan/Tilt、ブラックアウト、44Hz継続送信を確認する。
- [ ] sACNマルチキャスト対応ノードで、ユニバースルーティングとネットワークスイッチ越しの安定性を確認する。
- [ ] Enttec USB PRO / DMXKingで長時間送信を確認する。
- [ ] Enttec Open DMXはロジックアナライザでBreak 176us、MAB 16us、フレーム周期を測定する。重要現場ではPRO系を優先する。
- [ ] MIDIコントローラを実操作してnote/CC/clock/MTCとfeedbackを往復確認する。`SMC-Mixer-bt`の物理input/output列挙・open・All Notes Off送信までは2026-07-13に通過した。
- [ ] TouchOSC、iPad/Android Webリモートを実端末で往復確認し、Wi-Fi遅延を記録する。
- [x] NDI 6 Test Patterns/Studio Monitorと送受信し、探索、RGBA色、30000/1001入力、30fps出力、640x360→1280x720、接続維持を2026-07-13に確認した。
- [x] SpoutはURL To Spout/Syphon→Syndocal入力と再接続、Syndocal→TouchDesigner出力、RGBA、640x360→1280x720、送信再起動後の自動再接続を2026-07-13に確認した。
- [ ] 物理cameraのunplug/replugを確認する。`nuroum Webcam V15AF`の1280x720 RGBA取得、format negotiation、2回のstart/stopは2026-07-13に通過した。
- [ ] macOS実機とLinux実機で、複数ディスプレイ、フルスクリーン、音声/動画素材、`.sdc`保存再読込を操作確認する。CIはパッケージ起動までである。

結果は `qa/M4_IO_VALIDATION.md` の Acceptance Capture に追記する。loopback結果を実機合格へ読み替えない。

### P0: Webリモートの公開範囲

2026-07-12のv1.1作業でアプリ内の公開境界を強化した。

- 既定は`127.0.0.1:9100`のローカル専用。LAN待受は警告付きの明示オプトイン。
- 起動ごとの6桁ペアリングPINをHTTP Remote画面とWebSocketの両方で検証。
- WebSocket/HTTPのOrigin/Host検証。
- 接続数、メッセージサイズ、操作レートのサーバー強制上限。
- 接続クライアント一覧、拒否数、受信メッセージ数、強制切断UI。

信頼済みLANモードはHTTPのため、会場共有LANやインターネットへ直接公開しない。公開が必要な場合はTLS終端を持つリバースプロキシまたはVPNの背後へ置く。

### P0: 配布と法務

- [ ] Windows Authenticode署名を行う。
- [ ] Apple Developer ID署名とNotarizationを行う。
- [ ] NDI SDK/runtimeを配布する場合、NewTek/NDIの再配布条件を確認する。
- [ ] FFmpeg LGPL動的リンク、ライセンス本文、第三者通知、対応するソース取得方法を公開成果物で再確認する。
- [ ] `v1.0.0`タグを固定し、Release Notes、SHA-256、NSIS/MSI/DMG/AppImage/debを同一リリースへ掲載する。
- [ ] Windows/macOSのクリーンな別PCで最終成果物をダウンロードから起動まで確認する。

個人利用または限定テスト配布では未署名のまま扱えるが、SmartScreen/Gatekeeper警告を利用者へ説明する。

## v1.1候補の現在地

優先順位は次の通り。

1. Webリモート認証と接続制御（実装済み）
2. 操作履歴/Undo・Redo。Fixture、Cue、Output、Effect削除とマッピング編集（実装済み）
3. Syphon入力/出力(macOS)。Spout入力/出力(Windows x86_64)は実装・local GPU loopback済み
4. 自動バックアップ世代管理、クラッシュログ、診断パッケージ出力（実装済み）
5. HAP Q Alpha、HAP R/BC7、安全境界付きsingle-pass ISFシェーダー（実装済み）
6. 署名付き更新通知／install、stable・beta・nightly release channel（実装済み。公開endpoint／署名資格情報は外部運用）
7. 日本語UI/ローカライズ（実装済み。静的UI coverage 100%、主要動的状態、永続・可逆locale切替）
8. ユーザーテンプレート、MIDI／OSCマッピング共有（実装済み）。ワークスペースレイアウト保存も実装済み

### v1.1実装進捗

2026-07-12にWebリモートのセキュリティ強化を開始した。

- [x] 起動ごとに生成できる6桁ペアリングPINと、PINを含む共有用URLを追加。
- [x] Remote HTMLとWebSocket接続を同じPINで保護し、未指定・不一致を拒否。
- [x] HTTP HostとWebSocket Origin/Hostを実際の待受エンドポイントに限定。
- [x] PIN付きRemote HTMLをService Workerへ保存しないようにし、古い認証ページのキャッシュ再利用を防止。
- [x] 接続数、メッセージサイズ、操作レートの上限。
- [x] 接続クライアント一覧と強制切断UI。
- [x] 既定をローカル専用に変更し、信頼済みLANアクセスは未暗号化警告付きの明示オプトインに限定。

Webリモートのアプリ内P0境界は完了した。インターネット公開用のTLS終端はSyndocal自身へ秘密鍵を保持させず、リバースプロキシ/VPN等の運用境界として別途構成する。

2026-07-12にプロジェクト回復と診断基盤を追加した。

- [x] ブラウザ内の即時Recoveryに加え、デスクトップのアプリデータへProjectFileを原子的に保存。
- [x] 変更中のプロジェクトを最大30世代保持し、破損世代を一覧から除外して古い正常世代から復元可能にした。
- [x] Projectメニューに直近5世代の日時、元プロジェクト、復元、個別削除を追加。
- [x] Rust panic hookでアプリ情報、スレッド、panic、backtraceをクラッシュログへ記録。
- [x] エンジンテレメトリ、映像runtime、プロジェクト件数、実行環境、クラッシュログをまとめた診断ZIP出力を追加。プロジェクト本体とメディア内容は含めない。
- [x] 世代保持、検証、並び順、破損ファイル除外、診断ZIP必須エントリをRustテストで固定。

バックアップは未保存変更のRecovery用途であり、ユーザーが明示保存する`.sdc`の代替ではない。復元後はdirty状態を維持し、明示保存を要求する。

2026-07-12に編集トランザクションとSnapshot同期を更新した。

- [x] Fixture、Cue、Timeline、Video、Output、Effect、Node Graph、Stage Map編集を共通トランザクション境界で記録。
- [x] 成功した変更だけを最大100操作保持し、失敗・無変更操作は履歴へ入れない。
- [x] 同一対象の連続フェーダー／ドラッグ操作を750ms単位で1操作へcoalesce。
- [x] Undo/RedoのProjectメニュー表示、履歴件数、操作名、`Ctrl/Cmd+Z`、`Ctrl+Y`、`Cmd+Shift+Z`を追加。入力欄自身のUndoは奪わない。
- [x] 削除確認を「undo不可」から実際の復元経路を案内する文言へ変更。
- [x] 250ms pollの全EngineSnapshot転送を、revision付きtop-level deltaへ変更。不一致・再接続時だけfull snapshotで再同期。
- [x] no-op除外、連続操作coalesce、deltaの変更フィールド限定をRustテストで固定。

2026-07-12にUI密度と基本アクセシビリティを更新した。

- [x] Projectメニューから90% / 100% / 110%のUIスケールを選択し、端末内へ保存できるようにした。
- [x] 110%を含む実画面containmentゲートを追加し、1366x768で一画面デスクを維持。
- [x] 8-10px指定の可視テキストを11pxへ引き上げ、見出しのbalance、本文のpretty、数値のtabular表示、safe-area paddingを追加。
- [x] 既存のfocus-visibleを全標準入力と操作要素へ維持し、入力欄内のネイティブUndoを保護。
- [x] DMX Patch Gridを512 button同時生成から128アドレス×4ページへ変更し、全アドレス到達性を保ったままDOM／アクセシビリティツリーを削減。
- [x] 灯体一覧は80件、Stage Mapの実表示Fixture一覧は60件を超えるとfixed-row windowingへ切替え、Cue編集は12件単位、Clip Gridは12件単位、映像Layer／Outputはページ単位でDOM上限を固定。
- [x] 実ブラウザへ2,000灯体を投入し、1366x768でMapping Fixture DOMが先頭／末尾とも9行、`aria-rowcount=2000`、末尾灯体到達、画面containmentを確認。
- [x] 1280x720、1366x768、2048x1129の全Setup/Control/Touch containmentを再確認。

2026-07-13に端末ローカルのワークスペースレイアウト保存を追加した。

- [x] Setup／Control／Touch、Setup内タブ、Controlモード、Timeline／Playback面、Live Edit面、属性カテゴリを操作のたびに自動保存し、次回起動時に復元。
- [x] `.sdc`やUndo/Redoとは分離した端末設定とし、別会場・別端末へプロジェクト由来の画面状態を持ち込まない。
- [x] 未知値や将来fieldは安全に正規化し、破損JSONは削除して既定のSetup > Patchへfallback。localStorage拒否時もセッション内操作を継続。
- [x] ProjectメニューへReset Layoutを追加し、保存→page reload→Control > Timeline > Playback復元と既定配置resetを実ブラウザで検証。
- [x] 1280x720、1366x768（UI 110%を含む）、2048x1129の全画面containmentを再通過。

Lighting Cue Engine v2として、Cue番号、pre-wait、follow、tracking/block、notes、Intensity／Focus（Pan/Tilt/Position）／Color／Beam別のFade／Delayを追加した。非tracking Cueは未指定属性を灯体defaultへ戻し、pre-wait／followはengine時刻で実行する。最大16 Cue Partへfixture、video layer、video outputをそれぞれ重複なしで割り当てできる。照明はPart DelayをIFCB Delayへ加算し、Part FadeをIFCB継承元にする。映像はPart Delayまで現在state（再生、enabled、blackoutを含む）を保持してから、Part Fadeでlayer visual state／output opacityを独立補間する。target外参照、重複割当、重複Part番号は保存時に拒否し、Cue Updateやtarget削除では安全に清掃する。

複数named Cue ListとListごとの独立Back／GO位置に加え、最大64 Playback Executorを99 page×16 slotへ割り当てできる。同じCue Listを複数faderへ割り当てると位置を共有し、levelはHTP mergeされる。Playback Masterとfader levelはCue由来Intensityだけへ作用し、Position／Color／BeamとProgrammer overrideを減衰しない。参照型Palette/PresetはIntensity／Position／Color／Beam／Allを最大128件保存し、選択fixtureの現在値（Programmer staged値を含む）からCapture／Recapture、単灯／groupへDirect／Programmer適用できる。CueはPalette値をコピーせずGOごとに最新版を解決し、Cue固有の明示値を優先する。

Mark/MIBは直前Cue完了時にIntensity controlが全て0のfixtureだけ次Cueの非Intensity値を先行適用する。対象filterが空なら全targetを自動判定し、任意fixtureを選ぶとその集合だけへ限定するが、暗転安全条件は常に維持する。ProgrammerはDirect／Live Preview／Blindを切替でき、staged buffer、専用Blind DMX Preview、Commit／Clear、Cue Store/Updateへのstaged captureを提供する。全追加状態は`.sdc`、差分同期、Undo/Redo、project validationへ含め、旧`.sdc`はMain Cue List、空Part／Palette／MIB filter、Main Executor／Playback Master 100%、Programmerなしへ自動移行する。List migration、Part/IFCB／映像時間合成、MIB自動／手動filter、Blind分離、Palette参照解決、duplicate Executor HTP／MasterとProgrammer非干渉を自動テスト済み。

DMX入力の第一段としてArt-NetとsACN E1.31の受信、ユニバースフィルタ、HTP/LTPマージ、信号断タイムアウト、自動マージ解除、受信元／packet／invalid packet状態を追加した。入力は専用threadで受信し、engine command queue経由で44Hz renderへ渡す。アプリblackoutは入力マージ後にも優先して0を出す。Setup > DMXから起動・停止・状態確認でき、UDP loopback、HTP/LTP、signal clear、blackoutを自動テスト済み。

DMX出力経路は、初期化／送信失敗時にsenderを破棄し、250msから最大10秒までの指数バックオフで自動再接続するようにした。待機中は44Hz engine threadをブロックせず、連続失敗数、再接続試行数、次回試行までの時間、最終成功時刻を経路別telemetryに表示する。複数経路を併用すれば一方の障害中も他方への送信を継続できる。回復状態遷移と上限付きbackoffは自動テスト済み。

時刻同期はManual／Tap／MIDI Clock／MTC／LTC timecode注入／Ableton Linkを共通ClockSnapshotへ集約し、Timeline、Cue発火、BPM同期Effect／映像Layerが同じsourceを参照する。外部pulse/timecode受信時刻をmonotonic clockで保持し、MIDI Clockは500ms、MTC／LTC／Linkは1,000msを超えるとLOCKからSTALEへ遷移する。Runtimeはsource、最終同期からのms、LOCK／STALE／INTERNALを表示し、Live Show面は`HH:MM:SS.mmm` Timeline timecode、Clock source、Sync状態を常時表示する。旧`.sdc v1`で新しい同期health fieldがない場合は外部同期なしへ互換読込する。MIDI pulse、Link freshness／stale、MTC／LTC timeline同期とCue発火を自動テスト済み。

Active / Standby冗長化の第一段として、専用共有フォルダー（UNC／mounted shareを含む）へPrimaryが2秒ごとに完全な`.sdc` checkpointを発行し、Standbyが500msごとに追従するwarm standbyを追加した。checkpoint本体とmanifestは世代別の不変fileとして本体→manifestの順でatomic commitし、FNV-1a checksum、byte長、ProjectFile validationを通過した世代だけを読む。破損した最新世代は直前の正常世代へfallbackし、各Primary sessionを5世代保持する。Standbyへ適用するsnapshotはlegacy DMX output、全DMX route、全video outputを無効化し、lighting／video blackoutを強制するため、追従だけでは送信を開始しない。heartbeat停止をStandby側のmonotonic clockで5秒判定した後に手動Take Overでき、heartbeat生存中または複数Primary検知中は旧Primaryを物理的／ネットワーク的にfenceしたことを明示確認しない限り拒否する。世代保持、integrity fallback、全出力disarm、split-brain検知はRustテスト済み。これは共有storageを用いたmanual failoverであり、自動fencing、quorum、WAN越しのconsensus、無停止のaudio/video sample-clock引継ぎは未実装なので、二重送信を防ぐ運用手順と実会場LANでのfailover計測は正式受入に残る。

RDMはE1.20 packet core（48-bit UID、GET/SET/DISCOVERY command class、PID、可変parameter data、message length、16-bit additive checksum）、DEVICE_INFO decode、collision-safe DISC_UNIQUE_BRANCH response decodeを追加し、破損長／checksum／未知commandを拒否するテストを追加した。さらにArt-Net 4 OpRdm（0x8300）のheader、15-bit Port-Address、FIFO情報、StartCode除外／復元を実装し、unicast UDP request/responseはtransaction number、送受信UID、gateway IP、Port-Addressが一致した応答だけを採用する。ArtTodRequest/Dataは最大200 UIDのmulti-block、out-of-order block、UID total/lengthを検証してgatewayのTable of Devicesを自動取得し、ArtTodControl.AtcFlushからfull physical discoveryも開始できる。USB経路はENTTEC DMX USB Pro API v1.44のframe（0x7E／label／little-endian length／payload／0xE7）を実装し、Label 7でRDM requestを送り、Label 5のreceive statusとRDM packetを検証する。Label 11の38-byte DISC_UNIQUE_BRANCHを48-bit範囲へbinary probeし、collision分割、発見UIDのDISC_MUTE、開始時のbroadcast DISC_UN_MUTEを行うfull physical discoveryも追加した。probeは総timeoutと16,384回上限で暴走を防ぐ。選択COMはexclusiveにopenし、同じportでENTTEC／DMXKing DMX outputが有効なら二重所有を拒否する。ACK_OVERFLOWは同一GET/PIDを新しいtransaction numberで最終ACKまで再要求し、全blockのparameter dataを連結する。ACK_TIMERは16-bitの100ms単位見積りを総timeout内で待ち、GET QUEUED_MESSAGE（STATUS_ERROR）を取得する。status/unrelated queued responseは25ms間隔、全follow-upは64回上限で、NACK reasonもdecode／表示する。RDM consoleはArt-Net gatewayとENTTEC USB Proを切替でき、50ms-120秒timeout、response block／timer／queue回数、Art-Netでは5秒間隔のTOD inventory監視と追加／削除数を表示し、USB discovery結果もUID selectorへ反映する。SETとdisruptiveなfull discoveryは送信前確認を要求する。loopback transport、header/checksum破損、multi-block TOD、ACK_OVERFLOW連結、ACK_TIMER→QUEUED_MESSAGE、USB frame codec／receive payload／full 48-bit DUB request、UID/hex parserを自動テスト済み。RDM-capable firmwareの実機応答、collisionを含む複数実灯体discovery、Art-Net／USB双方の継続inventory／応答時間受入は未完了のため、現時点では「全RDM機材対応」とは扱わない。

VJ操作として、動画／静止画を最大64件ずつ複数選択してメタデータ付きレイヤーへ一括取込できる。Control > Mixerには12 pad単位のClip Gridを備え、全clipへbank切替で到達できる。各padはLIVE/READY、直接launch、PreviewからのCut／Take、Stopを提供する。直接launchはoverlayを維持し、Cut／Takeは他clipを停止またはfade outして選択clipだけをProgramへ出す。Deck A/Bへ別clipをロードして同時再生し、crossfaderでlinear opacity mix、Program audio有効時はequal-power音声mixを行う。専用160x90 thumbnail経路はlive visibility／opacity／master／blackoutに依存せず、source identity単位のcacheと44Hz engine thread外のdecodeで操作面を安定させる。

ffprobeでvideo/audio streamを分けて素材metadataへ保存し、silent素材のaudio操作を事前disableする。埋め込み音声はsystem defaultまたは選択deviceへmonitorでき、videoのplay／pause／0.25-4x speed／loop／positionと0-200% gainを追従する。Program audioは明示ON時だけLaunch／Take／Stopへ連動する。音声monitorが有効な間はUI pollに依存しない専用workerが25ms周期でA/V driftを監視し、75ms超を250ms cooldown付きで映像位置へ再同期する。無音時は250ms idle周期へ下げ、44Hz engine threadとaudio callbackの双方をブロックしない。現在drift、最大絶対drift、再同期回数、直近エラーはClip Gridへ表示する。

選択video outputは1-60fps、最大4096角でbackground rendererからFFmpeg raw RGBA pipeへ送り、H.264/yuv420p MP4として記録できる。Program audio toggleが有効なら、選択outputのcompositionに属し、再生中／有効／monitor中のlocal audio sourceだけを開始位置、loop、gain、0.25-4x speed付きでFFmpegへ入力し、AAC 192kbpsへmixしてMP4へmuxする。対象音声がなければ明示的にsilent recordingへfallbackする。UIはaudio requested／included／track countとwritten／dropped frameを表示し、停止時にencoder結果を確定する。FFmpeg入力選別、seek／loop／volume／atempo／amix graphとsilent fallbackの自動テストに加え、実際のraw RGBA + PCM toneからH.264/AAC MP4を生成し、FFprobeでvideo/audio両streamを確認するlocal runtime testも通過した。ただし映像decoder、monitor audio device、recording encoderを単一のsample clockで駆動するsample-accurate同期は未実装であり、長時間素材の実会場受入に残る。

Windows x86_64のSpout input/outputは標準featureとしてSpout2 2.007.017を静的リンクした。外部映像route起動時に専用DirectX 11 workerを生成し、入力は送信元の解像度とRGBA/BGRA format変更を追従してlatest frameだけをdecoderへ渡す。出力は選択compositionを60fpsで合成し、R8G8B8A8 senderとして公開する。通常previewとnative outputもSpout input frameを利用でき、停止時はworkerとframe registryを解放する。実GPU loopbackに加え、2026-07-13にURL To Spout/Syphonの1920x1080 DXGI 28入力、外部sender停止／再起動後の同一worker復帰、TouchDesigner 2023.12000への8-bit RGBA出力、640x360→1280x720変更追従、sender再起動後の自動再接続が通過した。Syphonはschema／route／Unavailable診断を維持するが、現行`syphon-wgpu 0.3`がwgpu 29を要求し、本体のwgpu 25と同一graphへ安全に導入できない。macOSの`Syphon.framework`と対応Metal integrationを揃えたhostでの実装・受入を外部残件とする。

NDI 6はlocal loopbackに加え、2026-07-13にNDI Test Patternsからproduction `NdiInput`へ1920x1080 RGBA・30000/1001を受信し、production `NdiOutput`からStudio MonitorへQA color barsを送信した。SDK senderの接続数1-2を確認し、640x360から1280x720への変更後も接続を維持した。このhostはmachine-wideの.NET 7 WindowsDesktop runtimeconfigが破損しているため、`qa/harnesses`のlauncherが子processだけ`DOTNET_ROLL_FORWARD=LatestMajor`を設定し、Program Filesは変更していない。

Camera inputとScreen Captureをlive video sourceへ追加した。レイヤー有効時だけFFmpegを持続起動し、WindowsはDirectShow／gdigrab、macOSはAVFoundation、LinuxはV4L2／X11Grabから1280x720・30fps RGBAを読み取る。フレームごとのprocess起動は行わず、workerとdecoder間でlatest frame bufferを交換・再利用する。Preview、native display、NDI／Spout outputも同じcapture registryを参照し、route停止時はFFmpegをkill／waitしてframeを破棄する。endpointは単一Command argumentとして渡しshell展開しない。command構築、未知backend拒否、project source名正規化、Camera／Screen decoder経路を自動テストし、Windows実desktopを取得して最初の完全frameと停止処理を確認するlocal runtime testも通過した。2026-07-13には物理`nuroum Webcam V15AF`をproduction workerで2回start/stopし、各回1280x720 RGBAを取得、MJPEG 1280x720／1920x1080／2560x1440の30-60fpsとYUYV modeのformat列挙も確認した。物理unplug/replugとmacOS／Linux実hostはM4受入へ残る。

HAP Q Alphaはin-process MOV parserが分離したYCoCg DXT5色面とBC4 alpha面を検証し、既存YCoCg変換とBC4 endpoint／3bit index補間でRGBA8へ合成するようにした。HAP Rは公式仕様の`Hap7`／RGBA BC7 frameをin-processで保持し、BC texture対応GPUでは圧縮blockを`Bc7RgbaUnorm` textureへ直接uploadしてhardware samplingする。Preview、ISF、またはBC非対応GPUではsafe pure-Rust `bcdec_rs`でRGBA8へ展開し、4の倍数でない端blockもcropする。HAP in-process経路が未対応formatや破損frameを返した場合はlibav、続いてFFmpeg CLIへfallbackする。HAP Q Alpha dual-plane、Hap7 MOV、BC7 alpha、CPU fallback、GPU/CPU parityと実GPU 4K BC7 frameを自動テスト済み。実エンコーダー由来HAP R素材の長時間multi-layer再生はM4受入へ残る。

ISF 2 single-pass image filterをレイヤー単位でimport、埋込保存、bypass、削除、parameter操作できるようにした。Event／Bool／Long／Float／Point2D／Colorを最大16 controlまで解釈し、GLSLをNagaで検証済みWGSLへ変換してwgpu上で実行する。変換結果とGPU pipelineは最大64 shaderまで再利用し、Preview、native display、NDI／Spout、recordingへ渡す共通frameに一度だけ適用する。不正shaderやGPU初期化失敗時は映像を止めず元frameへfail-openし、診断画面へ直近errorとpipeline数を出す。`.sdc v1`へshader sourceとcontrol値を埋め込み、旧layerはeffectなしで互換読込する。sourceは512KiB、image input 1個、single passに限定し、multipass／persistent buffer／imported resource／audio・audioFFT／loop／atomic／imageStore／明示binding・uniform・include・extensionを拒否する。実GPU threshold変換、cache再利用、fail-open、複製、control clamp、保存／旧file互換、不安全shader拒否を自動テスト済み。実在ISF packの互換範囲と4K長時間負荷はM4受入へ残る。

Application UpdateはTauri updater 2.2のminisign検証付きdownload／installをProjectメニューへ統合した。release buildがcompile時にHTTPS endpoint、base64化した公開鍵、`stable`／`beta`／`nightly` channelを全て埋め込んだ場合だけ起動後に非同期checkし、未設定buildは理由付きdisabledとして動作する。endpointはHTTPS／host必須、credential／fragment禁止、公開鍵はbase64 decode後にminisign keyとして事前検証する。Installはcheck時のversionを再照合し、manifestが差し替わっていれば拒否する。download前に未保存状態を含む検証済みProjectFile backupを`before update <version>`として原子的に保存し、download／verification状態をUIへ通知する。公開鍵だけをbinaryへ持ち、private keyはrelease secretに限定する。Updater artifact生成用config overlayとkey custody、manifest、改竄／wrong key／platform signing受入を`qa/UPDATE_RELEASE_RUNBOOK.md`へ固定した。公開release endpoint、private signing key、Windows Authenticode、Apple notarization資格情報は外部release operationsの残件である。

User TemplateはProjectメニューの`Save as Template`／`New from Template`から`.sdctemplate` v1として共有できる。現在の`.sdc`互換ProjectFile、埋め込みfixture profile、MIDI mapping、OSC mappingを一体で保存し、読込時にapp／version／project参照／mapping値域と64MiB上限を検証する。旧templateでmapping fieldが省略された場合は空として互換読込する。templateから作成したshowは必ず未保存projectになり、legacy DMX output、全DMX route、全video outputをdisabled、lighting／video blackoutをONにして意図しない送出を防ぐ。既存の`.midimap`／`.oscmap`単体共有も維持する。roundtrip、旧field省略、identity／拡張子拒否、全出力disarmをRust testで固定した。

UI localeはProjectメニューからEnglish／Japaneseを即時切替でき、`syndocal.uiLocale.v1`として端末へ保存する。`.sdc`やworkspace layoutとは分離し、保存拒否時もsession内の選択を維持する。翻訳境界は静的text、動的status text、title／aria-label／placeholderを扱い、初回またはlocale切替時だけ全treeを処理し、通常更新はMutationObserverが変更nodeだけをmicrotask単位で処理するためtelemetry更新ごとの全DOM走査を行わない。英語sourceは保持され、同一sessionで英語へ可逆復帰できる。Project／Setup／Control／Touch／照明／映像／Mapping／I/Oの操作語と説明、Project／Template／Updater状態を日本語化し、storage fallback、動的pattern、英語fallbackをhelper test、`lang=ja`／日本語Project menu／1366x768 containmentを実browser testで固定した。TSX ASTで静的textとtitle／ARIA／placeholderを数えるcoverage gateは、単位・規格名・座標略号・入力例の意図的共通表記を含め1913/1913（100%）を要求する。灯体／キュー／映像／出力／パレット／デバイス名等のユーザーデータは`data-no-localize`境界へ分離し、既知の静的preset以外に裸のuser labelが残ると検査を失敗させる。実browserでは`Video`／`Save`／`Output`という衝突しやすい灯体名が日本語localeでも変化しないことを確認する。新しい静的UI文字列を日本語化または明示的な共通表記へ分類しない限りCI検査を通らない。外部backendが返す未知の詳細errorは誤訳せず英語fallbackを維持する。

音声解析はwaveform／beat／BPMに加え、1024 sample Hann windowとin-place radix-2 FFTで時間別Bass（20-250Hz）／Mid（250Hz-2kHz）／High（2-10kHz）を生成し、0-1へ正規化してProjectFileへ保存するようにした。旧projectでは`audio.spectrum`を空配列として読む。Timeline Audioに最大128点へ間引いた3 band表示を追加し、100Hz／1kHz／5kHz toneのband分離、有限値、範囲を自動テスト済み。local video音声の再生／device選択、play/pause/speed追従、75ms drift補正、loop終端再起動、同期診断はVJ Clip Gridへ実装済み。通常運用での長時間driftは抑制するが、映像decoderとaudio deviceを単一のsample clockで駆動するsample-accurate同期ではない。

Projector出力へ左右上下それぞれのEdge Blend幅、blend gamma、Black Levelを追加した。設定は`.sdc`／mapping presetへ保存され、旧fileはblend 0・gamma 2.2・black 0で互換読込する。3-8点の任意polygon output mask、反転、0-0.5 UVのedge featherに加え、PNG／JPEGから輝度を16x16・4bitへ量子化してproject内へ埋め込むbitmap luma maskを追加した。外部file pathへ依存せず`.sdc`／mapping presetで共有でき、polygonとbitmapは乗算合成後に共通invertを適用する。Projection Mapping UIからImport／Clearと埋込状態を操作できる。CPU preview／test patternとnative GPU compute出力は同じbilinear bitmap sampling、polygon、feather、edge blend gamma、black liftを適用する。GPU/CPU parity、実PNG resize／量子化、bitmap補間／反転、polygon内外／反転／feather、値域validation、preset／旧file互換を自動テスト済み。固定16x16は会場輪郭とsoft maskのportableな実装であり、pixel単位の高精細maskが必要な案件は高解像度GPU texture maskを将来拡張として扱う。

Node Graph SourceへAudio FFTを追加し、Timeline AnalysisまたはLive Input、Bass/Mid/High、gain、biasを選んで照明属性と映像parameterへ接続できるようにした。Timeline spectrum point間は線形補間し、解析dataなしは0、gain/bias後は0-1へclampする。Live InputはCPALのsystem defaultまたは列挙deviceからbounded 8-chunk queueへmono downmixし、専用workerで1024 sample Hann FFTを最大約30Hzで計算する。RMS silence gate、0.65/0.35 smoothing、drop countと3-band meterを備え、audio callbackと44Hz engine threadの双方でFFTを実行しない。停止／project load時はephemeral spectrumをclearし、`.sdc`にはdevice状態を保存しない。旧Audio nodeはserde defaultでTimelineを維持する。Timeline補間DMX 128、Live Mid DMX 191、silence/tone分離、legacy roundtripを自動テスト済み。

Web Remoteは信頼済みLAN用、Active / Standbyは専用共有storage用の境界である。不特定LANやインターネットへ直接公開せず、Standby昇格前には旧PrimaryのDMX／video送信経路を必ずfenceする。

3Dビジュアライザは当面本体へ戻さず、既存の`visualizer`データ境界から外部実装へ接続する。標準操作面は2D Stage Mapを維持する。

## 回帰させない条件

- アプリ全体を縦スクロールさせない。一画面デスクを維持し、必要なスクロールは各パネル内に閉じる。
- SetupはLightingとVideoの専用画面を混在させず、タブで切り替える。
- Controlは上段のCue/Transport、左下の2D Stage Map、右下のLive Edit/Timeline/Mixerを維持する。
- 44Hz DMXスレッドに動画処理、ブロッキングI/O、動的アロケーションを持ち込まない。
- Windows/macOS/Linux固有機能は`cfg`/feature境界に置き、Unavailableなバックエンドでアプリ全体を起動失敗させない。
- プロジェクト名はSyndocal、拡張子は`.sdc`、発行者は`Seraf()のKTN`。Rayard/`.ry`/KDMXのユーザ向け表記を復活させない。
- 小変更は対象テスト、マイルストーンだけ全ビルドを行い、テストプロセスを多重常駐させない。

## 次スレッドの開始手順

まず次を確認する。

```powershell
git status --short
git branch --show-current
git log -5 --oneline
pnpm --dir app run check:release
```

参照順:

1. `RELEASE_STATUS.md` - 現在の判定と次の優先順位
2. `COMPLETION_PLAN.md` - v1.0 Definition of Doneと実装証跡
3. `qa/M4_IO_VALIDATION.md` - 外部I/O実機受入
4. `qa/M5_RELIABILITY_VALIDATION.md` - ソーク/回復/負荷
5. `qa/M6_RELEASE_VALIDATION.md` - 配布物/CI/署名判断
6. `CLAUDE.md` - 詳細な実装履歴。古い記述は末尾の新しい記録を優先する

次の作業は、v1.0コードを無目的に拡張するのではなく、次のいずれかを新しい目標として明示してから始める。

- 実機受入と正式リリース
- Webリモートのセキュリティ強化
- v1.1機能開発
