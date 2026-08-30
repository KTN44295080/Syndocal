# Syndocal

Syndocal は、DMX照明とVJ映像を同じタイムライン、キュー、BPMクロック、エフェクトソースで駆動するデスクトップ制御ソフトウェアです。

- 製品名: **Syndocal 1.2.0-alpha.40**
- 開発: **Seraf()のKTN**
- プロジェクト: **`.sdc`** (可読JSON)
- Tier 1: Windows 10+ / macOS 12+
- Tier 2: Ubuntu 22.04+ / Arch Linux

Current product metadata is `1.2.0-alpha.40` on branch
`codex/syndocal-v1.2`; its native checkpoint is pending. The historical
alpha.39 native checkpoint used source/docs `HEAD`
and upstream `ec93e9160da853ad181de70aee4db7b4a75fafbb`, which were equal at
the build checkpoint. The exact MSVC `14.44.35207` Community linker was pinned
and first in `where.exe`; `pnpm --dir app tauri build --no-bundle` exited `0`
in `2m57s` with first-party warnings `0`. The exact executable
`target/release/syndocal.exe` is `61,108,736` bytes, reports
Product/FileVersion `1.2.0-alpha.39`, and has SHA-256
`7923728D6D4D8F4D51DE5BEF337006ADD7851DC5EF0C2F384BA1664F3213D0C2`.
Exactly one checkout-owned PID `87640` is responsive with one maximized
`Syndocal` window, id `2033716740`; Daslight was preserved. Native alpha9 UI
reverification also passed without clicking the output selector or Refresh. The
exact sequence was Play -> Pause -> status-only wait -> Play -> Pause ->
status-only wait; throughout it, explicit-device output and resolved output
stayed at `Music (Elgato Virtual Audio)`, lifecycle `実行中`, `rev1`, with
advancing output frames and no visible Backend, Local IPC, or CUE fault. The project remained
unsaved and final Timeline state was paused. Audible/device selection,
Show-ASIO, Unity/GPU Art-Net/Spout, DJ, camera, and other hardware acceptance
remain open. The reviewed native-evidence record was committed and pushed at
`94b362bd2d733e447feabf0a0a6158699da6a2bf`, and `HEAD`/upstream equality was
verified immediately after that push. The next safe action is operator audible
confirmation.

Historical alpha.38 corrects the real-machine Timeline CUE fault observed when Play/Pause
rotated Timeline transport authority without changing media source projection.
The old path compared Click/Guide transport events to the unrelated source
projection pair and could therefore report a false stale-authority fault. The
new path publishes transport epoch/generation separately, compares CUE events
only to that pair, retires old Guide queues on every transport/schedule/source/
count-in rotation, returns DirectChild scheduling to Root when the child source
ends, and hides a retired output device after Fault. Invalid, stale, mixed, or
ambiguous authority still fails closed; no permissive retry or fallback was
added. Exact-linker proof passes engine `927/0/2 ignored`, Timeline CUE `50/50`,
full no-default Syndocal `1212/0/7 ignored`, and ASIO Timeline CUE `60/60`, with
first-party warnings `0`; `check:release`, TypeScript, and the Vite production
build pass at alpha.38. Independent Terra xHigh review is GO. The native/window
gate is complete; audible media/Click/Guide playback on the selected physical
endpoint remains pending operator confirmation. A real-machine
Play/Pause/Play/Pause regression kept the authoring monitor running with
resolved output `Music (Elgato Virtual Audio)` and no CUE/backend/IPC fault,
but the output-device `<select>` later displayed `3 - PX160 WAV...` while the
persisted and resolved device remained Music. Preserve that historical UI truth
mismatch as fail-closed evidence; the alpha.39 native selector/UI reverify above
did not reproduce it and closes the selector/UI gate. Audible endpoint
acceptance remains pending operator confirmation.

Historical alpha.37 repaired two show-critical runtime boundaries. Timeline CUE audio now
publishes the exact-anchor/future event batch before a fresh Legacy or ASIO
source can enter its callback, skips only pre-anchor history, retires the
active source on publication/activation failure, and keeps watermarks
uncommitted on failure. Per-source instance tokens make the callback-race
proof independent even when two runtimes have value-equal engine identities.
The fixed show Spout pair now requires each cached opaque-black first frame and
post-send exact SDK name before R4 success; teardown and lost retirement ACKs
remain state-owned, and no snapshot observation or substituted receipt can
reopen the two fixed sender names. Independent Terra xHigh reviews returned GO
for both production diffs. Exact-linker combined focused proof passes Timeline
CUE `49/49`, media-audio `46/46`, strict Show Spout `17/17`, production R4
interleaving `1/1`, and generic strict-name `2/2`, with first-party warnings
`0`. The same pinned-linker integration then passed the complete no-default
surface `1211/0/7 ignored` and complete Spout surface `1276/0/12 ignored`;
the ASIO Timeline CUE gate passed `59/59`, and TypeScript plus the Vite
production build also pass. A combined Terra xHigh audit first rejected a
test-token field that would warn in release; moving the whole token seam under
`cfg(test)` produced a warning-free release check and a final GO with
P0/P1/P2 `0`. From clean pushed HEAD
`5626a9636003462a23daf0f3de67af3cc5060e29`, the exact MSVC 14.44 build
completed in `3m04s` with first-party warnings `0`. The `61,116,416`-byte
`1.2.0-alpha.37` executable has SHA-256
`7AC54394E41751126911E6DC338536B93E484A20934B4CA9A001EB8B9F3E209E`.
Exactly one checkout-owned process, PID `50864`, is responsive with title
`Syndocal`; Computer Use verified its system menu exposes Restore while
Maximize is disabled, proving the exact window is maximized. Audible,
Unity/GPU, and DJ hardware acceptance are still pending. DJ Link auto-start is
currently and visibly blocked by `trust_network_absent`, so no DJ readiness is
inferred from this native gate.
Alpha.32 established Timeline authoring monitor output, arbitrary PROGRAM/CUE
device assignment, canonical child-Timeline
audio varispeed, and fail-closed Windows candidate extraction/verification. `FollowProgram`
remains the existing Normal route; with `ExplicitDevice`, every Timeline media
clip (logical PROGRAM or CUE) plus generated Guide/Click material is sent to
one operator-selected WDM endpoint. Show ASIO remains separate: entering it
retires the authoring sinks, and returning requires an explicit Normal route
and output selection. Endpoint name/topology and session/generation are
revalidated; missing, ambiguous, stale, changed, or failed state stays silent
and fail-closed without default/PROGRAM fallback.

Historical alpha.36 completed Japanese coverage for the same-PC Art-Net/Spout activation
surface and status messages while keeping `Syndocal Background` and
`Syndocal Foreground` byte-for-byte visible as external sender identifiers.
Focused localization is `3632/3632` (`100.0%`); Timeline context-menu and I/O
remote-scroll browser contracts plus TypeScript pass. Independent Terra xHigh
rereview is GO with P0/P1/P2 `0`. From clean pushed HEAD `81a7a9c`, the exact
MSVC 14.44 linker build completed in `3m30s` with first-party warnings `0`.
The `61,049,856`-byte `1.2.0-alpha.36` executable has SHA-256
`150C30FE218C6C32A50169EFE7638AD0C5D1CDD8DCFDF5BCFCCEB6434DD4E79E`;
exactly one checkout-owned process, PID `109972`, is responsive with title
`Syndocal`, and Win32 `IsZoomed` verified its exact window is maximized.

The historical alpha.36 native checkpoint carries the alpha.34/alpha.33 camera-capture source
tranche, which replaces the old free-form DirectShow
camera route, which was fixed at `1280x720` / `30 fps`, with an explicit
current-generation device/profile catalog, opaque endpoint identities, and an
exact one-frame probe before Add. Its admission envelope is at most
`4096x2160`; profiles above `1920x1080` are admitted at no more than `30 fps`,
profiles above `1280x720` at no more than `60 fps`, and capture rates up to
`120 fps` only when the device advertises that profile. Output presentation
remains capped at `60 Hz`, while screen capture is unchanged at `1280x720` /
`30 fps`. The acceptance authority is
[qa/CAMERA_INPUT_ACCEPTANCE.md](qa/CAMERA_INPUT_ACCEPTANCE.md); the historical
alpha.34 and alpha.36 native builds passed. The now-historical alpha.37 native
build/maximized-window gate passed, and the now-historical alpha.38 normal
native/window gate is complete. Camera-specific UI interaction and hardware
probes remain pending.

The accepted alpha.32 source gates passed with first-party warnings 0: the exact
MSVC 14.44 Community linker was pinned and first in `where.exe`; Timeline audio
passed `31/31`, ASIO media playback passed `80/80`, and runtime-only protocol
nonserialization passed `1/1`. Windows candidate metadata/extractor/materialization/
artifact self-tests passed `125/43/4/140`; `check:release`, Timeline-audio,
format, and diff gates also passed. The alpha.32 normal Windows
`tauri build --no-bundle` passed from clean pushed HEAD `5d40874` with the
exact linker first and warnings 0. Its `60,713,984`-byte executable has SHA-256
`B04CE351A456C82715383A4430401F3DA4D824EC8813B32894ABE1BF98FBD90D`;
exactly one checkout-owned process (PID `89524`) is responsive with title
`Syndocal`, and Computer Use verified the window is maximized. Historical
alpha.31 normal Windows `tauri build --no-bundle`
passed from clean pushed HEAD `602b96a` with the
exact linker gate; its `60,756,480`-byte executable has SHA-256
`6F9BF17A2802A2FC5F8935E8EFFEB0C57CA4A3A21B245C68BFA62315152C7F4A`.
Exactly one process was launched, was responsive, and its exact native window
was maximized through the verified process handle. The current machine's WDM
catalog contained one selectable `Music (Elgato Virtual Audio)` endpoint. Its
exact name and topology fingerprint were saved while Syndocal was stopped, and
the app was relaunched successfully; audible readback, physical I/O, and show
acceptance remain open. Ox was unavailable
for this tranche; under the documented narrow
exception, an independent Terra xHigh source review returned GO with no P0/P1.
The oversized `app/src-tauri/src/main.rs` audio-runtime extraction is deferred
until after the show because changing ownership/lifecycle boundaries before
native and venue acceptance is a pre-show risk.
The earlier alpha.25 DJ session remains historical operational evidence only;
it was replaced by the verified alpha.32 native process and is not alpha.32 DJ
acceptance. The release metadata checker expects alpha.39 product and
installer naming below; that naming does not assert an alpha.39 installer
exists.

The final alpha.33 source gate passed capture-filtered `71 passed / 0 failed /
2 ignored`, full no-default-feature `1203 / 0 / 7`, full default-feature
`1239 / 0 / 12`, and control-plane `64 / 0 / 0` under the exact MSVC 14.44
linker pin, with first-party warnings `0`. The two unrelated parallel-suite
timeouts reproduced as passes individually and the complete suites passed with
`--test-threads=1`. Independent Terra xHigh rereview returned GO with
P0/P1/P2 `0`. The bounded listing/memory, child-process cleanup, deferred
reaper/quarantine ownership, and automatic stale Active-row replacement are
closed in source. App exit while a deferred reaper is still pending remains an
explicit unverified OS boundary because this tranche does not install a Windows
Job Object; sustained-4K performance also remains open. On 2026-08-29, the connected `Insta360 Link` advertised `3840x2160` at
`30 fps`, `1920x1440`, `1920x1080`, and `1280x720` at `60.0002 fps`, and no
`120 fps` profile. Direct FFmpeg preflight completed 150 RGBA frames at 4K30
and 300 RGBA frames at 1080p60 with exit `0`; the exact Syndocal profile probe
still proves one frame only. Sustained 4K
is unverified because the current RGBA `Vec` clone at `60 Hz` may approach
`1.98 GiB/s` of copy traffic. These source results do not claim native, UI, or
hardware acceptance.

The frozen alpha.34 same-PC output integration adds a strict local Art-Net show
route (`127.0.0.1:6454`, wire Universe 0, 512 bytes, channel 500 forced to zero)
and the fixed 1920×1080 Spout pair `Syndocal Background` / `Syndocal Foreground`.
Both senders remain opaque black until their durable output lease is committed
and continue black while Timeline output is stopped. The exact-linker final
matrix passed Syndocal no-default `1205/0/7`, Syndocal default `1263/0/12`, and
engine `920/0/2`; remaining workspace crates also exited successfully, with
first-party warnings `0`. This show checkpoint is same-PC only. Remote Art-Net,
NDI, and other cross-PC video transport are outside its acceptance boundary.
Alpha.35 additionally rejects activation when Universe 0 already has a DMX
input, rejects new U0 HTP/LTP input while the strict route is active, and skips
any stale U0 merge at the final render fence. U1 and ordinary non-strict input
merge remain available. Focused exact-linker proof passed `12/12` with
first-party warnings `0`, and independent Terra xHigh rereview returned GO with
no P0/P1/P2. The exact-linker full engine gate then passed
`924 passed / 0 failed / 2 ignored`, first-party warnings `0`.
Alpha.35 also binds the strict Spout pair to a physical first-frame barrier:
each sender must first transmit the cached 1920×1080 opaque-black RGBA frame,
retain its exact post-send SDK name, and join the two-ACK plus final R4
authority barrier before active/live handoff. The focused pair gate passed
`13/13`; full Syndocal passed no-default `1205/0/7` and default Spout/libav
`1265/0/12`, with first-party warnings `0`. Independent Terra xHigh rereview
returned GO with P0/P1 `0`.
The latest accepted, now-historical alpha.37 native build from clean pushed HEAD `5626a96` completed in
`3m04s` with the exact MSVC 14.44 Community linker pinned and first, and with
first-party warnings `0`. Its `61,116,416`-byte executable reports
Product/FileVersion `1.2.0-alpha.37` and SHA-256
`7AC54394E41751126911E6DC338536B93E484A20934B4CA9A001EB8B9F3E209E`.
Exactly one checkout-owned PID `50864` is responsive with title `Syndocal`;
Computer Use verified the exact window as maximized through its system-menu
state. No Unity/GPU, audible device, DJ, or camera hardware acceptance is
inferred from this native checkpoint.
The historical alpha.36 native build from clean pushed HEAD `81a7a9c` completed in
`3m30s` with the exact MSVC 14.44 Community linker pinned and first, and with
first-party warnings `0`. Its `61,049,856`-byte executable reports
Product/FileVersion `1.2.0-alpha.36` and SHA-256
`150C30FE218C6C32A50169EFE7638AD0C5D1CDD8DCFDF5BCFCCEB6434DD4E79E`.
Its historical checkout-owned PID `109972` was responsive and maximized before
being path-verified and stopped for the alpha.37 build.
The exact-linker native build from clean pushed HEAD `31a577c` completed in
`3m36s` with first-party warnings `0`. The resulting `61,045,760`-byte
`1.2.0-alpha.35` executable has SHA-256
`1144DBFA1AAD3E46E88F6F3A026218B8B7AB3C3E9A234270B75EDB41213A0E7A`.
Exactly one checkout-owned process, PID `72448`, is responsive. Computer Use
still did not expose the Tauri window as targetable, so maximized visual and
physical Unity/GPU acceptance remain open.
The alpha.34 exact-linker native build from pushed HEAD `138f6c3` produced a
`61,039,104`-byte `1.2.0-alpha.34` executable with SHA-256
`1FCB899E2B118B94F92B5D87ECD7A5EA3FFE32D33448319FE697D39841FA642F`.
Exactly one checkout-owned process launched responsive. Maximized-window and
Unity/GPU observation remain pending.

The preserved running alpha.25 DJ session was observed on 2026-08-28 with
Deck 1 playing `More One Night × 動く、動く (Agate Trance&Makina bootleg)`;
Artist was empty, Content ID was present, position revision was `149152`, BPM
was `140`, and `isPlaying=true`. The DJ Agent sent
`DJ_TRACK_ACTIVE` event `22bfdf30-94e0-4264-9519-0ae4fba6bd2c`, but delivery was
rejected; FOH remained `owner— / playing No / track—`. The temporary track had
no Timeline mapping, so the result is fail-closed. Computer Use reproduced the
Timeline Start dropdown returning to its placeholder immediately after
selecting Timeline 1 (UI remount bug; a separate Luna fix is in progress).
This is not HW-4 end-to-end acceptance; HW-4 remains exactly `0/12`. The DJ-PC
screenshot evidence is
`C:\TEMP\codex-clipboard-142f581b-eef4-41e9-80fa-a0a79b8c749e.png`,
`218629` bytes, SHA-256
`A217E511BD3905DE81148202E2746686CF320665BDAF5931ECEC0C2A9064F653`,
LastWrite `2026-08-28 07:10:32 JST`.

Historical alpha.25 native evidence remains immutable: source/native evidence
was `566a7101b0d5c9307e8d0efa5ccf499aba3eb404`, and the accepted artifact was
`target/release/syndocal.exe`, Product/FileVersion `1.2.0-alpha.25`,
`59,807,744` bytes, SHA-256
`D42B0A1B245197A76E3D3B5CC45DEEB1C6C33EDE6261C7DA94C34147141239C1`,
LastWrite `2026-08-28T06:20:27.6424691+09:00`, with exactly one responsive
maximized `Syndocal` window (PID `83252`, handle `60826240`). Its native I/O
DMX and Setup Lighting/Profile rendering were accepted. The StandardRelease
runner remains bound to that exact alpha.25 artifact and must not be relabelled
until a fresh alpha.26 native build is accepted.

Historical alpha.24 evidence is preserved, not promoted to alpha.25: source
build `5eb1888906613851c4c49e66717a270f13bbd7b5` with native-evidence descendant
`e4c02f8e2c9013bdb735b2b785f6c75599c0a950` produced
`target/release/syndocal.exe`, Product/FileVersion `1.2.0-alpha.24`,
`59,803,136` bytes, SHA-256
`8FF5CF6670F592DFFE53477E8E14BF95CD3FCF81916FC23CAD58E7354763874D`.
Its exact-checkout PID `63016` was responsive and maximized. The historical
alpha.24 Timeline Follow rearm and its deterministic three-display runner
self-test remain evidence for that artifact only; physical three-display Apply
was not run because the loaded project was `Untitled.sdc` with no real output
IDs/labels. `qa/harnesses/run-syndocal-three-display-show-acceptance.ps1` is now
the StandardRelease authority for `1.2.0-alpha.25`, `59,807,744` bytes, SHA-256
`D42B0A1B245197A76E3D3B5CC45DEEB1C6C33EDE6261C7DA94C34147141239C1`, source
HEAD `566a7101b0d5c9307e8d0efa5ccf499aba3eb404`; PowerShell 7 and Windows
PowerShell 5.1 self-tests both passed `88/88`, and syntax passed in both. No
Apply or hardware acceptance was run. DJ TCP has
re-established the `.50.2` peer, but the loaded project remains `Untitled.sdc`
and its mapping is not authoritative; HW-4 remains exactly `0/12`.
The historical alpha.21 Setup-I/O native artifact had Product/FileVersion
`1.2.0-alpha.21`, built from `536742db968b242164349c34dd6940fe3ced8e92`;
its exact identity and remaining hardware boundary are recorded in
`qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-26.md` sections 31-34 and
`qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` sections 54-57. Physical
DJ/MIDI/pedal, HW-4, real-output, target-DJ-PC/LAN/reconnect, and three-display
acceptance remain unverified; this is not a final-completion claim. The earlier
`1.2.0-alpha.19` checkpoint at `41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f` implemented the current
Follow-hold / Stage 2 DJ authority. The earlier `1.2.0-alpha.18` native evidence
remains historical and is not rebound to alpha.21.
The separate token-free show structural preflight is committed and pushed at
exact `b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`. Tool/helper `node --check`
checks and `node qa/tests/show-structural-preflight.mjs` passed; the valid
fixture exits `0`, while legacy, missing, usage, and unknown inputs exit `2`.
Independent Terra xHigh final review is GO with P0/P1/P2 all `0`. This tool is
read-only/authored-only, consumes no real token, launches no show process,
changes no runtime/project/hardware state, and cannot advance HW-4; the matrix
remains exactly `0/12`.
At the alpha.18 checkpoint, the file later overwritten at
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` was
Product/FileVersion `1.2.0-alpha.18`, `58,740,224` bytes, SHA-256
`841068E08F80EB877FBA919FB86D3F52B3D4444314B47ABEF995BFA593E8D4F9`,
LastWriteTimeUtc `2026-08-27T05:38:40.1840641Z`. Its launch observed exactly
one responsive process (PID `80264`, title `Syndocal`, window handle `854080`,
`IsMaximized=true`, start `2026-08-27T14:38:57.8350806+09:00`). Native visual
capture confirmed Web Remote/Security/Endpoints/DJ Link/Standby in the same
disclosure stack and two wired candidates. The wired-binding refresh button
click itself remains unconfirmed because foreground PID retrieval failed;
five-viewport setup harness evidence did confirm listener empty→count 0→
Ethernet4 `192.168.50.1`→count 1, one invoke per phase, and disabled mutation
controls. Target-DJ-PC deployment and the physical HW-4 matrix remain
unaccepted.

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
- マイクLive FFTの16 band／RMS／Peak／onset／BPM／kick／snareから照明・映像を同時駆動。WindowsではWASAPI sharedに加え、通常MIT版と分離したASIO bridge featureでdriver／sample rate／fixed buffer／channel mixを明示選択できる。ASIOは製品リリース要件であり、切断／250ms無入力時は共有0クリアを要求し、未受理を明示する。入力の未完了条件は `qa/ASIO_INPUT_ACCEPTANCE.md`、PROGRAM/CUE出力は `qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md` を参照する
- Tap / MIDI Clock / MTC / LTC / Ableton Link用共有クロック境界
- MIDI、OSC、WebSocket、iPad/Android向けPWAリモート
- 10秒間隔の自動Recovery、Recent Project、`.sdc` OS関連付け

## 画面構成

- **Setup**: Lighting / Video / Mapping / I/Oを分離し、パッチ、灯体プロファイル、出力、投影補正を設定
- **Control**: LIVEでは上段Timeline、左下の常設2D Stage / Groups、右下のSourcesを配置。Stage / Timelineは個別ウィンドウに分離でき、メイン側の残った領域は空白なく再配分。EDIT / MIXERでは同じ一画面枠内で編集・ミキサー面へ切り替え
- **Touch**: タブレットやタッチ操作向けのキュー、灯体、映像操作

アプリ本体は一画面内に固定されます。長い灯体ライブラリやリストだけが各パネル内部でスクロールします。

## インストール

CI/Release成果物は次の形式です。

- Windows: `Syndocal_1.2.0-alpha.40_x64-setup.exe` (NSIS)、`Syndocal_1.2.0-alpha.40_x64_ja-JP.msi`
- macOS: `.app`、DMG
- Linux: `.deb`、AppImage

### 未署名ビルド

現行のv1.2 alpha開発物と従来のv1.1個人配布物はコード署名されていません。

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

Windowsの完全libav bundleでは`FFMPEG_DIR`を共有FFmpeg SDKルートへ設定します。bundle直前にDLLがstageされ、MSI/NSISへ同梱されます。NDIは開発時の明示featureとしてのみ扱い、通常packageでは独立したライセンス済みruntime overlayのartifact proofが存在するまでfail-closedである。

ASIOは既定buildへ含めませんが、Windows製品のリリース完了条件です。canonical `syndocal_asio_bridge.dll` は入力／Reactive Capture用のABI/schema v2と、PROGRAM/CUE出力／full-duplex用のABI/schema v3を同居させ、両者を一つのprocess-wide leaseで排他します。入力側はdevice列挙・明示選択、sample rate／native format／channel／fixed buffer、低遅延callback I/O、exclusive open/start/stop/free、占有・不一致・切断・reset/resync・XRUN／no-callbackのfail-closed処理、選択設定の保存と再列挙時のstale IDロックを要求します。出力側は同一ASIOのshared-clock PROGRAM/CUEと、ASIO PROGRAM＋明示WDM CUEのsplit-device経路を持ち、後者は独立clockであることを表示して暗黙fallbackを禁止します。ABI v1時代のbridge/build、短時間smoke、100-cycle、native UIと第二vendor（HOTONE）の明示stream／100-cycleは歴史的証跡であり、現行v2/v3の全受入には読み替えません。旧名・旧ABI・欠落・load/symbol/ABI faultは明示分類してfail-closeします。配布ライセンス、advertised matrix、native show artifact、real-device output、hot-plug／fault recovery、soak、物理latencyが終わるまで完了扱いにしません。手動取得したSDKを`CPAL_ASIO_DIR`、LLVMの`libclang.dll`を`LIBCLANG_PATH`へ明示して検証します。SDK pinは[qa/ASIO_SDK_PIN.json](qa/ASIO_SDK_PIN.json)、入力境界は[qa/ASIO_INPUT_ACCEPTANCE.md](qa/ASIO_INPUT_ACCEPTANCE.md)、出力境界は[qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md](qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md)を正とします。`distribution_approved: false` の間、通常installer/updaterは canonical `syndocal_asio_bridge.dll`、旧 `syndocal-asio-bridge.dll`、その他のASIO DLL、DLL globをすべて拒否し、libavは検証済み7 DLLだけを明示同梱します。GPLv3版として分離するかSteinberg proprietary agreementを締結し、独立したartifact/notice/release workflowを承認するまで、通常のpackage flowはASIO配布artifactを生成・stage・publishしません。

```powershell
& .\qa\harnesses\check-asio-build.ps1
```

通常Windows bundleは[qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json](qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json)を唯一の実行時DLL inventoryとする。7 DLLは名前だけでなくサイズ、SHA-256、PE32+ AMD64 identityまで一致しなければstageできず、resource sourceは許可root内の通常ファイルのみである。このinventoryの承認値（サイズ／SHA-256／PE identity／common resource 4件の実体／既知ASIO bridge hash集合）は`app/scripts/windows-runtime-inventory.mjs`内の独立anchorとしても保持され、inventory JSONだけを編集して承認を再定義することはload時にfail-closedで拒否される。対象tripleは`x86_64-pc-windows-msvc`のみで、aarch64/ARM64はAMD64 (PE machine 0x8664) inventoryとの不整合として拒否し、AMD64 DLLをARM64へコピーする経路は存在しない。取り込む全ファイル／ディレクトリは、symbolic link・junction・mount point・cloud placeholder等のWindows reparse point属性をすべて拒否する（属性レベル監査＋hard-link nlink拒否）。通常common resourceも`THIRD_PARTY_NOTICES.md`、FFmpeg、Spout、bcdecの4個だけを同じく固定する。NDI featureを含むbundleは、独立したライセンス済みruntime overlayが実証されるまでfail-closedであり、既知4 signalに加え未知のNDI系環境変数、package/workflow/tauri conf/Cargo manifest上の`--features …ndi…`表記、bundle時のancestor process chain上のtauri/cargo CLI feature flagを検出して拒否する。`WINDIR`と`NDI_SDK_DIR`／`NDI_RUNTIME_DIR_V2`〜`V6`のみをSDK所在変数として許可する。

CIではcold clean Windows runnerでも、いかなるtestより先に`prepare:runtime-libs`がpinned runtimeをstageし、直後のassert stepがinventory由来の7 DLL名の存在を確認する。packaging self-testは未追跡の`target/`出力に依存せず、自前のtemp fixture（合成PE、junction、symlink、hard-link）でalias／tamper経路を検証し、作成できない場合はSKIPを目視可能な行として出力し、通過断言には数えない。

`pnpm --dir app run check:release`はこのpackage境界self-testを含む。NSIS/MSIの通常CI smokeは展開済み実体を再走査してASIO名、既知bridge SHA-256（改名された場合を含む）、および7 DLLのhash/PE mismatchを拒否する。release candidateはこれに加えて、`app/scripts/windows-candidate-extractor.mjs`によるNSIS・MSI・updaterのrepository-owned deterministic extractionと、同一release-evidenceへ結合された再検証型inventoryを要求する。RC evidence checkerとcandidate artifact checkerは、同一のbase64-wrapped Tauri minisign key parser、Ed25519 payload/trusted-comment verification、canonical decoded HTTPS channel/filename URL matcherを使う。runbookはmetadata crypto preflightを抽出より先に要求し、payload、exact adjacent `.sig`、key、manifestのcrypto gateが成功するまで抽出済み`syndocal.exe`のruntime identity diagnosticへ到達しない。候補/通常three-rootのinspectorはいずれも検証bytesからfresh `wx` materialized copyだけを受け、installerを実行する経路はない。抽出器はlisting/extraction/publicationを通して入力SHA-256を結び、各tool inputをpre/post hashしてrevalidated tombstone経由で削除する。不一致またはcleanup failureでは対象をvisible fail-closedする。same-volume rename/tombstoneはpathname-atomicのfenceであり、Nodeはhandle-atomicまたはWindows owner/DACLを証明できないため、repository-owned fresh single-writer stagingでのみ運用するP1境界を残す。現時点では実RCのrelease-evidence、署名済み実バンドル、Windows実行ファイルVersionInfo/runtime identityが未配置のため、candidate package acceptanceは意図的にfail-closedであり、任意installerの実行を代替証明として扱わない。通常のTauri v2 configでは直接の署名済みNSIS/MSI payloadだけをupdaterとして受け入れ、legacy zipは`v1Compatible`設定時だけ許可する。

## 性能・QA

- Engine: 44Hz、UIとは別スレッド、bounded lock-free command queue
- Windows: Pro Audio MMCSS Critical + 1ms timer、macOS: USER_INTERACTIVE QoS
- 1時間release soak: 108,001フレーム、drop 0、tick jitter p99 0.535ms、command-to-DMX p99 0.476ms、最大13.3MB (ハーネス構成)
- 大規模Engine gate: 200灯体、8ユニバース、100キュー、20,000ターゲット
- UI viewport policy: 現行Control上段workspaceのbrowser受入は3840x2160／2560x1440／1920x1080／1280x720の4解像度を必須とする。960x640は製品の最小構成として残すが、この4解像度gateの代替にはしない。860x520／1366x768は過去の補助回帰としてのみ残し、現行受入には数えない。browserのgeometry／screenshot PASSはnative PASSへ読み替えず、nativeはexact executable、最大化window、実画面captureを別に記録する
- Alpha.7 Control upper-workspace native checkpoint: stale selector／grid-flow／Timeline row ownershipを修復し、1920 classの最大化native画面でLighting Scene Matrix、Video Media Library／Import、Timeline lanes／Sources／Tools／expand／Esc復帰を確認した。2048x1104は補助確認であり、native 2560x1440／1280x720／3840x2160は未受入のまま残す。Guideは48kHzでtarget minus 7,200 frames、すなわち150ms先行をdeterministic gateで確認したが、audible device／実機outputの証明ではない
- Fullscreen VJ Focus gate: 1920x1080のfullscreen mode fixtureでは重複する外側headerを除き、Clips／Live Monitors + Outputs／Layersを684／811／405px、Preview／Programを317／476px（Program/Preview 1.502）に配分する。live-audio dockは92px、主操作は32px、設定操作は28pxとし、telemetryの切れ・critical overflowを0に固定する。1366x768／1280x720は従来のheader、64px audio rail、Preview／Program 1:1を維持する。専用identifierのcurrent-source native QA buildでも、最大化1913x1080からF11でexact 1920x1080へ移行し、このFocus配置と実ASIO telemetryを確認後、Escで通常配置へ復帰した
- Populated VJ operator gate: 7 layerを6+1 bankで扱い、各visible layerへBuilt-in FX／選択段のEnable・Bypass／Advancedを固定する。重いISF editorは閉じている間DOM 0件、開いた1 layerだけ最大8段のstack rowと選択段editorをmountする。Event／Bool／Long／Float／Point2D／Color、8/8 Add lock、Event 1→0と履歴不変／busy interlock、選択・focusを保つMove、Remove後の安全な隣接選択、GPU診断更新、段index付きBypass transaction、3 outputのLive／Off／BO rail同期を1920x1080／1366x768／1280x720の英日6ケースで検証し、全操作phaseでunsafe overflow 0／owner外rect 0を要求する
- ISF stack runtime: 有効段を最大8段まで一つのcommand encoderへ順番に積み、frameごとのhost upload 1回、GPU内ping-pong、最終readback 1回で処理する。変換shader／GPU pipelineは最大64件を再利用し、実行段数、stack render time、段別compile errorを診断へ出す。現行のheadless rendererは最終RGBAをCPUへreadbackし、別deviceのnative outputへ再uploadするため、outputまでのzero-copyやTouchDesigner級GPU-resident graphを示すものではない
- Live audio UI: VJ Desk内の通常／compact 64px railとfullscreen VJ Focusの92px dockでStart／Stop、device／rate／requested buffer／channel mix、16対数band、RMS／Peak、onset、BPM／confidenceとcallback／queue telemetryを常時確認する。軽量feature statusは約30Hz、詳細telemetryは約1Hzでsingle-flight更新し、5 viewport×英日stateful gateで固定する
- Live-audio viewport gateはDOMを直接改変せず、Tauri invoke mockから実Solid stateを駆動する。5解像度×英日でRefresh、generation ID再対応、同名曖昧時の再選択ロック、capability、192kHz／8192-frame／stereo pair Start request、42/58/76% meter、live telemetry、Stop／clear-pendingまで検証する
- Windows native live-audio acceptance: 最大化したcurrent-source buildでsystem defaultのWASAPI shared入力を48kHz monoでStart／Stopし、実callback 480/480/480 frames、capture-to-workerの採取時点current 0.1ms／max 10.1ms、OVR 0 chunks／0 frames、queue current/high-water/capacity 0/1/4を確認した。この1構成の測定をASIOや長時間・他deviceの性能証明には読み替えない
- Windows ASIO short smoke: 分離bridgeとapp loaderから`TOPPING Pro USB Audio Device`を明示指定し、48kHz／2ch／i32／requested・applied 128 framesでStart／Stopした。7 callbacks／896 frames、max capture delay 4166.7us、XRUN 0、nonfinite 0、terminal event 0。`Realtek ASIO`はhardware unavailableとして失敗し、別driverやWASAPIへfallbackしなかった。これは1台・短時間の構成確認であり、第二の正常driver、1時間soak、物理pixel latency、TouchDesigner同条件A/Bを満たさない
- Windows ASIO 100-cycle: bridgeのignored hardware testを`--locked --offline`で実行し、明示ID `asio:TOPPING Pro USB Audio Device`、48kHz／2ch／i32／128 framesを27.6767468秒で100回Start／Stop／Freeした。actual bufferは全回128、callback 200（各回2以上）、Stop 100、Free 100、warning／terminal／XRUN event・API／nonfinite／frame mismatchは全て0で、別driver／WASAPIへのfallbackも0だった。第二vendorの`HOTONE AUDIO USB Audio Device`も44.1kHz／2ch／i32／128 framesで100 clean cyclesを通過した。これは二vendorの反復open/close gateであり、advertised matrix、1時間soak、hot-plug／fault recovery、物理pixel latency、TouchDesigner同条件A/B、配布licenseの合格ではない
- Windows ASIO native UI: current-source QA buildを最大化し、F11で実1920x1080へ移行してから、VJ DeskでASIO／`TOPPING Pro USB Audio Device`／48.0kHz／128 frames／Average All→monoを明示選択した。Startで`ACTIVE`、OVR 0/0、XRUN 0、Stopで`READY`を確認し、Escで最大化windowへ復帰した。full native operator pathと第二vendorのbridge trialは合格だが、advertised matrix、1時間soak、hot-plug／fault recovery、物理pixel latency、TouchDesigner同条件A/B、配布licenseは未合格である
- Auto VJ: CLOCKまたはLIVE INPUTのonsetをsourceに、seed／show revision／ordered candidateから再現可能なTakeを生成する。手動Take、Hold、blackoutを優先し、Program Audioはbackend coordinatorが世代token付きで一度だけ切り替えるため、device openやdecodeでTake操作を待たせない

検証記録:

- [qa/M4_IO_VALIDATION.md](qa/M4_IO_VALIDATION.md)
- [qa/M5_RELIABILITY_VALIDATION.md](qa/M5_RELIABILITY_VALIDATION.md)
- [qa/M6_RELEASE_VALIDATION.md](qa/M6_RELEASE_VALIDATION.md)
- [qa/NATIVE_WINDOW_ACCEPTANCE.md](qa/NATIVE_WINDOW_ACCEPTANCE.md)
- [qa/LIVE_AUDIO_FAIL_CLOSED.md](qa/LIVE_AUDIO_FAIL_CLOSED.md)
- [qa/AUDIO_REACTIVE_VJ_ACCEPTANCE.md](qa/AUDIO_REACTIVE_VJ_ACCEPTANCE.md)
- [qa/ASIO_INPUT_ACCEPTANCE.md](qa/ASIO_INPUT_ACCEPTANCE.md)
- [qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md](qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md)
- [qa/VJ_OPERATOR_DESK_ACCEPTANCE.md](qa/VJ_OPERATOR_DESK_ACCEPTANCE.md)
- [qa/TOUCHDESIGNER_COMPETITIVE_AUDIT.md](qa/TOUCHDESIGNER_COMPETITIVE_AUDIT.md)
- [RELEASE_STATUS.md](RELEASE_STATUS.md) - v1.0完成判定、外部受入、次スレッド向けバックログ
- [qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md](qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md) - 現行v1.2 alphaから完成までの依存順、version、warning、commit、実機・配布gate

## 既知の制限

- v1.0以降の外部映像I/OはDisplay、feature-gated NDI、Windows x86_64のSpoutに対応しています。Syphonはwgpu世代差とmacOS実装環境が必要なため未実装です。
- HAP Q Alpha、HAP R/BC7のGPU直接sampling + CPU fallback、安全境界付き最大8段のordered single-pass ISF stackは実装済みです。1 stackのsource合計は512 KiB、1段16 control、project内ISF source合計16 MiB、`.sdc`全体64 MiBに制限します。ISF multipass／persistent buffer／imported resource／audio input、100種級library、shared texture／任意node graph、headless readback後のnative別device再upload解消、4K multi-layer／実会場long soakは未対応または未受入です。
- Live FFT入力はCPALのdevice catalogとcapabilityを読み、Windowsの既定buildではWASAPI sharedのsystem defaultまたは列挙device、sample rate、requested buffer、全channel平均／単一channel／stereo pair downmixを選べます。選択rateごとにchannel数／sample format／buffer capabilityを`resolved_config`として先に確定し、Startでも同じ構成を再検証します。更新世代に結び付くopaque device IDを使い、古い／未知IDや非対応構成は別device・rate・bufferへ黙ってfallbackせずStartを拒否します。全11種のCPAL PCM sample formatを正規化し、通常data callbackは4本×2048-frameの事前確保slotへallocation-freeで格納します。workerは16対数band、RMS／Peak、spectral flux、適応onset、60–200 BPM／confidence／beat phase、centroid／density／kick／snareを生成し、同じframeから旧Node Graph用B/M/Hを派生します。Compiled Audio Reactive Rackは任意の対応照明属性／映像parameterへこれらを割り当て、Auto VJはCLOCKまたはLIVE INPUTのonsetを選び、seed／show revision／candidate順から決定的にTakeします。非既定のWindows ASIO featureは独立bridgeを動的loadし、明示driver／rate／channel／native format／fixed bufferをStart時に再検証、実buffer framesとbackend XRUNを表示します。TOPPINGの短時間実機smoke、100回Start／Stop／Free、F11 1920x1080のfull native UI gate、および第二vendor HOTONEの44.1kHz／2ch／i32／128 frames・100-cycleは通過しました。ASIOの配布ライセンス選択、advertised rate／buffer／channel matrix、hot-plug／fault recovery、engine出力適用ack、実機1時間／物理pixel latency／TouchDesigner同条件比較は未完です。通常MIT installerへASIO bridgeは同梱しません。
- Device Refresh後はbackendとdevice名が新旧catalogueの双方で一意な場合だけ明示選択を新IDへ引き継ぎます。曖昧・消失時はsystem defaultへ黙って落とさず、再選択するまでcapabilityを破棄してStartをロックします。
- 3Dビジュアライザは本体UIへ戻さず、`visualizer`データ境界から外部実装へ接続します。標準UIは2D Stage Mapです。
- Enttec Open DMXはOS/USBドライバ依存のbreak timingがあるため、最終現場ではUSB PRO系を推奨します。
- Art-Net/sACN/Serialの実機遅延は接続機材ごとに再測定してください。リポジトリの自動検証はloopback中心です。
- NDI SDK、コード署名、macOS notarizationは配布ライセンス/資格情報が必要で、リポジトリには含みません。

## ライセンス

Syndocal本体は[MIT License](LICENSE)です。一般動画デコード用の配布物は、
GPLコンポーネントを無効化したFFmpeg 8.1共有ライブラリを動的リンクします。
詳細とLGPLv3本文は[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)を参照してください。
