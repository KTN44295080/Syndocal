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
| SHORTCUTS | MIDI/キー割当 | **部分** |

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

## 4. リスクと限界（正直な列挙）

- EFFECT 数値コードの意味は非公開。初期インポートではエフェクトシーンが「近似 or 静的スナップショット」になる。
- FIXTUREDATAS の内部表現（チャンネル値配列の並び）はまだ未検証 — 実装第一歩で検体から確定させる。
- VERSIONFILE="2" 以外の旧版 .dvc は未検証（検体は全て VERSIONFILE=2）。
- SSLCHANNELTYPE の全番号表は Daslight 非公開。検体15プロファイルから帰納し、未知番号は Generic チャンネルへフォールバック。
