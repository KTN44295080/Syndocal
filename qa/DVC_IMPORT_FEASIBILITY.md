# Daslight 5 プロジェクト（.dvc）インポート実現可能性調査

- 調査日: 2026-07-18
- 検体: ユーザー実ファイル `C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc`（122,446 bytes、DASBUILD 25.0905.165.111）
  および `C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc`（344,765 bytes、タイムライン/音声入り完全版）
- 結論: **実現可能。しかも想定よりはるかに容易。** `.dvc` は平文XMLで、パッチ部分だけ base64+zlib の入れ子。
  灯体プロファイルは**ファイル内に完全埋め込み**されており、外部SSLライブラリ無しで自己完結変換できる。

## 1. フォーマット解明（実ファイルからの一次証拠）

### 1.1 全体構造 — 平文XML

```xml
<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="25.0905.165.111" VERSIONFILE="2">
    <CONFIGURATION .../>        <!-- ビューズーム等のUI状態 -->
    <PATCHS DATA="AAL4kXic..."/> <!-- ★唯一の圧縮部。下記1.2 -->
    <FIXTUREGROUPS>              <!-- グループ20個: NAME / DASUID / identity色相当のトグル群 -->
    <SCENES>                     <!-- バンク(グループ)13個 → シーン計30個。下記1.3 -->
    <SHORTCUTS>                  <!-- MIDI/キー割当 -->
    <TOUCH>                      <!-- Touchコンソールレイアウト -->
    <DEVICES>                    <!-- DMXインターフェース (DVC GOLD, serial 1021943, 4 universes) -->
    <TIMELINES>                  <!-- Super Scene タイムライン（完全版ファイルに存在）。下記1.4 -->
</DLMFILE>
```

.NET の XML パーサでそのまま well-formed として読める（検証済み）。

### 1.2 PATCHS DATA — base64 + 4バイトBEレングス + zlib

`DATA` 属性は base64。デコードすると `[u32 big-endian 非圧縮長][zlib stream (78 9C)]`。
実測: base64 18,832文字 → 14,123 bytes → inflate 194,705 bytes（ヘッダのBE長と厳密一致）。
展開結果は再びXML:

```xml
<PATCH NBFIXTURE="41">
  <FIXTURES>
    <SSLLIBRARY SSLFIXUID="..." SSLNAME="Eliminator Lighting/Mega Par Profile EP.ssl2">
      <SSLPROPERTIES SSLFIXTTYPE="2" SSLBEAMOPENING="30" SSLAMPLIPAN="360" SSLAMPLITILT="220" .../>
      <SSLMODES SSLNBMODE="1">
        <SSLMODE SSLMODEINDEX="1" SSLNBCHANNEL="5">
          <SSLCHANNEL SSLCHANNELTYPE="25" SSLCHANNELNAME="Red" SSLCHANNELMSB="0" SSLCHANNELLSB="0">
            <SSLPRESETS><SSLPRESET SSLPRESETTYPE="65" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" .../></SSLPRESETS>
          </SSLCHANNEL>
          ...
    <FIXTURE TYPE="0" DASUID="..." NAME="mega par profile ep" INDEX="1"
             ADDRESS="1" UNIVERS="1" POSX="740" POSY="500" ANGLE="237" ...>
      <BEAM INDEX="0" POSX="740" POSY="500"/>
    </FIXTURE>
```

- **プロファイルが丸ごと埋め込み**: Shinkan2026 は 15 種の SSLLIBRARY（ADJ Mega 系、Saber Spot RGBW、
  PinSpot LED Quad、Mega Hex L Par、Stage Evolution ePAR64、レーザー、スモーク等）を内包。
  チャンネル型番号（例 25=Red, 26=Green）、名前、MSB/LSB（16-bit）、プリセットDMX範囲、
  パン/チルト振幅、ビーム角まで揃う。
- **パッチが完全**: 41灯すべてに ADDRESS + UNIVERS + 2D配置（POSX/POSY/ANGLE）。
  Syndocal の 2D マッピング座標へ直接写像できる。
- 補足: `OUTMODE`/`FLAG` 属性は hex文字列の小さな zlib（"789c..."）。反転等の小フラグとみられ、初期実装では無視可。

### 1.3 SCENES — バンク → シーン（平文）

```xml
<BANK NAME="Strobe" COLOR="#ffffcf08" NBSCENE="2">
  <SCENE NAME="Fl-StrobeChase" COLOR="#ffffcf08" LOOP_MODE PLAY_TRIGGER="2" PLAY_DIVISION
         DIMMER SPEED PHASE SIZE FADE_IN FADE_OUT LTPPRIORITY ...>
    <FIXTUREDATAS NB="..."/>   <!-- 静的チャンネル値（直接変換可能） -->
    <RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="321" DURATION="5000">
      <PARAMS><PARAM TYPE="2" ID="10" VAL="1"/>...</PARAMS></EFFECT>
      <PRESETS>...</PRESETS>
      <BEAMS><BEAM FIXTURE="{fixture DASUID}" .../></BEAMS></RACK></RACKS>
```

- バンク=グループ、COLOR は identity 色 → Syndocal の group_colors / cue color（T7）へ直接写像。
- 再生パラメータ（loop、speed、phase、size、fade in/out、BPMトリガ PLAY_TRIGGER/PLAY_DIVISION、LTP優先度）は平文。
- `EFFECT TYPE/ID/PARAM` の数値コードは非公開 → **ここだけ近似変換**（有限個の型なので実測対応表を育てられる）。

### 1.4 TIMELINES — Super Scene（完全版ファイルで確認）

```xml
<TIMELINE NAME="0" INDEX="0" DASTLLOCKED DASTLMUTED DASTLFOLDED>
  <BLOCK TYPE="2" NAME="Sakanaction_..." START="0" END="305064" FADEIN="0" FADEOUT="0"
         DASTLMEDIAPATH="C:/Users/kouty/Desktop/Shinkan-Left/..."/>   <!-- 音声ブロック -->
  <BLOCK TYPE="1" NAME="ON" START="0" END="305064" SPEED="1" ALLOWLOOP="1"
         CONFORM_TO_TEMPO="1" SCENEUUID="cd7a755a-..."/>              <!-- シーンブロック -->
```

- 複数レーン + lock/mute/fold、音声ブロック（メディア絶対パス + START/END + fade）、
  シーンブロック（シーンUUID参照 + SPEED + ALLOWLOOP + **CONFORM_TO_TEMPO**）。
- Syndocal の Nレイヤータイムライン（F1/F2）、音声クリップ（F7）、conform-to-tempo（F3）、
  Super Scene（F6）と**概念が一対一対応**する。今シリーズで作った受け皿にそのまま流し込める。

## 2. 変換マッピング（.dvc → .sdc）

| Daslight | Syndocal | 忠実度 |
|---|---|---|
| SSLLIBRARY（埋め込みプロファイル） | 自己完結カスタムプロファイル（snapshot://fixture 方式） | **正確**（チャンネル型対応表が必要、有限） |
| FIXTURE ADDRESS/UNIVERS | DMXパッチ（universe/address） | **正確** |
| FIXTURE POSX/POSY/ANGLE | 2Dマッピング X/Z/Yaw | **正確**（座標系スケールのみ調整） |
| FIXTUREGROUPS / BANK COLOR | グループ + group_colors / cue color（T7） | **正確** |
| SCENE 静的値（FIXTUREDATAS） | Cue（マルチステップは F8 の steps） | **正確** |
| SCENE 再生パラメータ（loop/speed/fade/BPM分周） | Cue メタデータ + クロック同期 | **ほぼ正確** |
| RACK/EFFECT 数値コード | LFO/Color/Chaser/Move/Value/PositionWave へ近似 | **近似**（初期はプレースホルダ + 対応表を実測で拡充） |
| TIMELINE + BLOCK TYPE=2（音声） | タイムライン音声クリップ（F7） | **正確**（同一PCなら絶対パス解決可、無ければ再リンクUI） |
| TIMELINE + BLOCK TYPE=1（シーン） | タイムラインシーンブロック（F2、conform=F3） | **正確** |
| TOUCH | T11 touch_surface | **部分**（レイアウト対応表しだい） |
| DEVICES（DVC GOLD等） | DMX出力ルート設定の初期値 | **参考情報**（ハード非互換のため案内表示） |
| SHORTCUTS | MIDI/キー割当 | **部分・安全復元**（TYPE=1の107 Scene Play / 55 Tap Tempo / 108-110方向付きPlay / 113 Bank Next / 229 選択中Feature Fader / FLASH holdを復元。入力device affinityとDaslight固有feedback色はApproximate） |

## 3. 実装計画案（トランシェ1本 + 検証資産）

1. **crates 側**: `dvc-import` モジュール（`quick-xml` + `flate2` + `base64`、いずれも枯れた純Rust）。
   `.dvc` → 中間モデル → 既存 `.sdc` スナップショット構築。プロファイルは自己完結埋め込みで出力。
2. **Tauri コマンド**: `import_daslight_project(path)` → 変換レポート付きでロード
   （変換できた/近似した/落とした要素を明示するレポートUIが過大評価禁止の精神に合う）。
3. **UI**: プロジェクトメニュー LOAD の横に `Import .dvc`。音声パス不在時の再リンク行。
4. **テスト資産**: この PC には実 .dvc が10本ある（Shinkan2026 ×3版、homecoming2606 ×3、DFS2026、Sin、Panel、Left1）。
   ラウンドトリップ/カバレッジ回帰に最適。まず Documents 版 Shinkan2026（41灯/13バンク/30シーン）を金標準にする。
5. 注意: 逆方向（.sdc → .dvc）は当面スコープ外。ユーザー要望も「読み込んで変換」まで。

## 実装記録（2026-07-18 DVC-1 完了）

計画は同日 DVC-1 トランシェとして実装された（Codex gpt-5.6-sol 実装 × Fable 検証）。
`app/src-tauri/src/dvc_import.rs` + `import_daslight_project` コマンド + プロジェクトメニュー `Import .dvc` + `DvcImportReportPanel`。
FIXTUREDATA の実フォーマットは 4 検体で確定: **BE u16 チャンネル数 → チャンネルごとの BE u16 現在値（0xFFFF=未書込）→ BE u32 レコード数 → 27バイト不透明行×N**（290バイト検体 = 2+34×2+4+8×27 で厳密一致）。27バイト行はステップとみられるが証拠不足のため本トランシェでは解釈せず。
検証: `cargo test -p syndocal dvc` 6/6、フロントエンドビルド緑（466.75 kB、警告なし）、terminology 緑、フルビューポートマトリクス 232 pass / exit 0、ネイティブ実機で金標準 Shinkan2026.dvc をメニューからインポートし 41灯/15プロファイル/20灯体グループ/13バンク/30キュー/値22変換・0スキップ/音声1/シーンブロック42/エフェクト7スキップをレポート表示、cueパッドに Daslight バンク色と日本語シーン名（新宝島）を確認、DMX ルートは無効既定、証跡は `target/qa/ui-comparison/dvc1-*.png`。
インポート後の Engine 実発光確認（GO で DMX プレビュー値が出るか）と TOUCH/SHORTCUTS/エフェクト近似は後続トランシェ候補。

## 実装記録（2026-07-18 DVC-2 完了: 27バイト行の完全解読）

DVC-1 で「解釈保留」だった 27 バイト行を Fable 直接実装で解読・検証した。

**確定フォーマット**（5検体・525ペイロード・61行パターンの全数調査による）:
- レコード数 = 灯体の**ビーム数**（8セグメントの mega bar rgba のみ 8、他は全て 1。パッチ側 BEAM 数と完全一致）。**ステップではない** — 全10検体に STEP/SEGMENT 要素は存在せず、これらのショーにマルチステップ Static シーンは無い。
- 各行 = **13 × BE IEEE 半精度float + 末尾モードバイト**（観測値 0x00/0x02）。−1.0=未設定、設定値は 0..1 正規化。スロット 0/1/2 = R/G/B、スロット 11 = Dimmer（他スロットは未同定・未使用）。
- 行はチャンネル部（正値）の**特徴量空間ミラー**であり、DMX 状態としては冗長。チャンネル部が引き続き正となる。

**実装**: 行解釈は非致死（未知バリアントでもチャンネル値は常に保持し警告計上）。一意に対応付く R/G/B/Dimmer スロットとチャンネル値の整合性照合を行い、レポートへ `beam_records` / `beam_feature_checks` / `beam_feature_mismatches` を追加（EN/JA 対応）。
**検証**: `cargo test -p syndocal dvc` 9/9。金標準 Shinkan2026 で **221 レコード解読 / 109 件照合 / 不一致 0** — ミラー説が実ショーで無矛盾。合成テストは整合・不整合（警告化・値保持）の両経路をカバー。

## 実装記録（2026-08-09 DVC MIDI shortcut）

- 実DVCの`SHORTCUT TYPE="1"`を全数監査し、`EVENT DATA="status:channel:number:value:device"`を検証付きでMIDI mappingへ変換する経路を追加した。
- Action 107はScene Play、55はTap Tempoとして既存Touch actionとDaslight UI証拠が一致するものを変換する。`FLASH=1`は通常Scene Playへ丸めず、pressでTrigger・Note Off/CC zeroでReleaseする`FlashCue`へ保存する。
- Daslight実行ファイル内の連続action tableから108/109/110をScene Play Forwards/Backwards/Back & Forth、113をBank Nextと確定した。各DVC BankをCue Listへ1:1保存し、方向付きPlayはone-shot方向を既存Cue起動へ渡してpre-wait後も維持する。Homecoming実検体17件は全件復元し、合成検体で未使用109と非holdも固定した。
- 229は固定Dimmerへ推測せず、`TARGETINDEX`を選択中の表示Featureフェーダー番号として保存する。UIが現在の`fixture_ids`と表示`attributes`順をruntime-only backend contextへ同期し、Tauri/MIDI/OSC/Remoteは共通resolverから既存`SetFixtureAttributeBatch`へ到達する。MIDI feedbackも同じ選択から現在値を返し、複数灯体の値が不一致なら誤値を返さない。Laser実検体のCC8/9・index 0/1を2/2 goldenで固定した。
- `SETTINGS OUT` / `OUT1` / `OUT2`はそれぞれOFF / ON / Unknown（混在・不定）feedbackとして、message種別・出力channel・number・velocity/valueを丸めず保存する。連続controlではOFF/ON端点間を補間し、離散controlではlive stateに応じた厳密messageを送る。複数Cue List / groupで並列起動中のCueもactive判定へ含める。通常mappingにも同じ3状態editorを設け、`.midimap` / `.sdc` / Recovery / backupで往復する。
- 入出力device affinityだけはプロジェクト固有名へ自動bindingせず、Setup > I/Oで対象portを選ぶ外部境界としてApproximateに明記する。
- DVC import UIは旧showのmappingを消去した後、report内の復元mappingを本番stateへ設置する。その後の通常`.sdc` Save/Recovery/backupで保持される。

## 4. リスクと限界（正直な列挙）

- EFFECT 数値コードの意味は非公開。初期インポートではエフェクトシーンが「近似 or 静的スナップショット」になる。→ DVC-3 で忠実変換に取り組む（近似ではなく同一挙動の新規実装を許容する方針をユーザー承認済み）。
- ~~FIXTUREDATAS の内部表現はまだ未検証~~ → DVC-2 で完全確定（上記）。
- VERSIONFILE="2" 以外の旧版 .dvc は未検証（検体は全て VERSIONFILE=2）。
- SSLCHANNELTYPE の全番号表は Daslight 非公開。検体15プロファイルから帰納し、未知番号は Generic チャンネルへフォールバック。
- 行の未同定スロット（3..10, 12）とモードバイトの意味は将来の検体で拡充する。
