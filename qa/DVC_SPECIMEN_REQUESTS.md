# Daslight実保存標本の採取記録（全項目完遂）

- 作成: 2026-08-09 Fable（FABLE_HANDOFF_2026-08-09 未解決証拠1〜3の解消手段）
- 背景: 当初は競合アプリへ保存操作を行わない境界だったため、以下をユーザー向け採取手順として作成した。
  2026-08-10の明示許可後はFable/Codexがscratchだけを操作し、全標本を採取・追跡済みにした。
- 対象Daslight: `25.0905.165.111`（監査済みバイナリと同一ビルド。バージョンが違う場合はその旨併記）
- 共通手順（履歴）: **Shinkan2026等の実ショーは使わず**、新規の使い捨てプロジェクトで作成し、
  `C:\Users\kouty\Documents\Daslight 5\Projects\specimens\` （無ければ任意の場所）へ保存。
  完了後、ファイルパスをFableセッションへ伝えるだけでよい（以後の解析・golden化はこちらで行う）。

## 進捗（2026-08-10、Fable・権限昇格後に自ら採取）

ユーザーの権限昇格を受け、確立済みネイティブ経路（capture-window + click-window-point +
物理クリック/貼り付け）で使い捨てscratchプロジェクト（Shinkan2026は不使用）に
VALUE FX 2種を作成し `qa/specimens/ValueCatalog-Sweep-Plasma.dvc` へ保存した。

- **標本#2（VALUE Sweep 625 golden）＝スキーマ取得済み**: 実保存XMLが
  `PARAMS NB=3`（TYPE4/ID1・TYPE6/ID3=0・TYPE2/ID10=1）でCodex静的証明とバイト一致。
  Direction change=1の直列化も確認。詳細は `qa/DVC_VALUE_CATALOG_PARITY.md` 末尾。
- **標本#1（対象ありVALUE FX）＝BEAMS取得済み**: `BEAMS NB=1`＋
  `FIXTURE=... BEAMID=0 IDSELECTION=1` の非空ターゲット直列化を確認。
- **追加でID623 Plasma golden**: `PARAMS NB=10`（TYPE0/ID10..17 = 1,2,1,2,-1,2,1,-1）が
  Codexの新規importスキーマ＋逆アセンブル既定値と完全一致。Plasmaライブエディタの
  既定値も逆アセンブル値と一致を目視確認。
- **残**: 上記2シーンはVALUE feature未束縛のためimporterは正しくskipする（＝実レンダ用
  fixtureではなくスキーマgolden）。feature束縛済みの完全importable標本、および下記
  標本#3（COLOR MAPPINGS）・#4（Move BEAMID>0）は次サイクルで採取する。

## 完遂（2026-08-10 同日第2サイクル、全4標本クローズ）

feature束縛UXを解明（**フェーダーチャンネルアイコン右上の小FXバッジ**が属性のfeature束縛。
アイコン本体クリックは値トグル）。同一標本ファイルに追記して全項目を実データ化した:

- **標本#1完遂**: strongpoint 2灯対象のVALUE Sweep/Plasmaシーンを追加し、Dimmer featureを
  束縛（`PRESET SSLPRESET="4" SSLCHANNEL="-1" MIN="0" MAX="1"`として直列化）。importerが
  実際にSweep{direction_change:true}/Plasma{既定8値}へ変換することをrepo可搬goldenテスト
  `dvc_local_golden_value_sweep_and_plasma_import_from_saved_specimen`で固定（80c82be）。
  f3200aレーザー対象の2シーンは「profileがPRESET type 4非搭載」でfail-closedのまま残し、
  実ファイルでの失敗経路カバレッジとして断言化した。
- **標本#3完遂（COLOR MAPPINGS family 5/3 初の実保存body）**: `RACK TYPE=5 EFFECT TYPE=3
  ID=36`（Rainbow）。PARAMS NB=7 = TYPE4/ID1（8色パレット）+ **TYPE2/ID2 Grayscale
  （family 3は直列化する——family 7の非直列化ガードと対照、静的解析と整合）** +
  TYPE6/ID3 Transform + TYPE0/ID4 Rotation + TYPE1/ID10 Color Width + TYPE0/ID11 Angle +
  TYPE1/ID12 Gradient。**`<MAPPING NAME="Rectangle" DASUID=... TYPE="0" X/Y/SX/SY/ANGLE
  /LOCKED>`が2D配置の正体**——「任意2D配置」検証を阻んでいたper-beam X/Yレイアウト
  メタデータの実物。BEAMS NB=2（strongpoint×2、IDSELECTION順序付き）。
  当時のジェネレータカタログUI観測は部分的だったが、2026-08-10のDVC-ENUMで
  RainbowからGridまで全21種を最下端まで観測し、全factory ID/class/schema/evaluator入口を
  `qa/DVC_FULL_FX_CATALOG_PARITY.md`へ固定した。
- **標本#4完遂（Move BEAMID>0）**: メガバー1本のセル3〜7をマーキー部分選択し
  MOVE FX Circleを作成。`RACK TYPE=4 EFFECT TYPE=4 ID=221`、**BEAMS NB=6で
  BEAMID=1..6（全て>0）**＋IDSELECTION 1..6の順序保存。副産物として当時未対応だった
  Move Circle=ID221のPARAM TYPE5/ID1=POINTS
  （4点パス: (0.25,0.5)(0.5,0.75)(0.75,0.5)(0.5,0.25)）+ TYPE1/ID2 + TYPE2/ID3という
  bodyも同時取得。ID221はDVC-V5bでcircumcircle/beam-targetまで実装済み。
- 全標本は単一ファイル `qa/specimens/ValueCatalog-Sweep-Plasma.dvc` に集約
  （`.gitattributes`で`-text`バイト厳密）。これを起点にDVC-V4/V5a/V5bは完了し、
  続くDVC-ENUMの全family default保存形は`qa/specimens/DVC-ENUM-25.0905.165.111.dvc`へ分離した。

## 完遂前に使用した採取手順（履歴）

以下は再依頼ではなく、上記goldenを再現するときの履歴手順である。

### 標本1: 対象ありVALUE FX（`ValueTargeted.dvc`）

1. 新規プロジェクトに適当な灯体（できればマルチビームのBar系）を1〜2台パッチ。
2. シーンを作成し、灯体/ビームを**選択した状態**でVALUE FXの`Rainbow`を追加
   （Selected beamsに選択が入っていること）。
3. 保存。→ これで「BEAMSが空でない実保存VALUE FX」のserializer形が確定する。

### 標本2: VALUE Sweep実保存golden（`Sweep625.dvc`）

1. 同様にシーンへVALUE FXの`Sweep`を追加。
2. シーンを2つ作り、片方は`Direction change`OFF、もう片方はONで保存。
3. **もしSweepのエディタに`Transform`（反転/対称）系の操作が見えるなら**、
   それをONにした3つ目のシーンも追加してほしい——現在fail-closedにしている
   `Transform=1`の実保存証拠になり、扱いを実データで決められる。

### 標本3: COLOR MAPPINGS実保存標本（`ColorMappings.dvc`）

1. マトリクス/マルチビーム灯体を選択した状態で`COLOUR MAPPINGS`系FXを追加
   （どのジェネレータでも可。複数シーンで別ジェネレータを入れると更に良い）。
2. 保存。→ family/type `5/3` の実保存bodyが初めて確定する。

補足（2026-08-11）: family 5/3のRainbow標本は取得済みだが、専用classのLines ID31 / Grid ID50
native保存bodyは未取得。今回のrouteはbinary/static evidenceとsynthetic strict fixtureに限定する。
将来再採取する場合はLinesとGridを各1 scene、palette/Size/Width/Transform/Rotationを非defaultにし、
対象ありと`BEAMS NB=0`を分けて保存するとnative serializerの最終goldenになる。

補足（2026-08-12）: `ColorMappings-Remaining7.dvc`でBounce ID21のnative NB=12/default/
empty Rectangle/BEAMS=0は取得済み。次の優先標本は対象ありBounceをShape0 Collide OFF/ON、Points
Fill OFF/ON（Points=2と10を含む）で各sceneへ保存したもの。可能ならShape1も別sceneへ保存し、
現行の`unsupported XEEL glyph geometry`境界を維持したままserializer値を固定する。これが得られるまで
Shape0/Pointsはrecovered `SyndocalCorrected` route、Qt antialias edgeは非SourceExactと明記する。

### 標本4: Move FXのBEAMID>0（`MoveBeamId.dvc`）

1. マルチビームのムービング系灯体を1台パッチし、**一部のビームだけ**を選択。
2. その選択でMOVE FX（Circle等）を追加して保存。
   → BEAMS内に`BEAMID>0`エントリを持つ実保存Move rackが得られる。

## 採取後の処理（Fable/Codex側）

各標本はXML監査→既存インポータとの突合→golden回帰テスト化
（`dvc_local_golden_*`系のファイル存在ゲート式）→未証明でfail-closedにしている
変種の解放判断、の順で処理する。標本が無い間は該当変種をSkipped/fail-closedの
まま維持する（FABLE_HANDOFF_2026-08-09の規律どおり）。
