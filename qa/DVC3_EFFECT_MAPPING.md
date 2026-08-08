# DVC-3: Daslight FXジェネレータ数値コード対応表（実機照合済み）

- 照合日: 2026-07-19
- 方法: 実機 Daslight 5（Shinkan2026.dvc Desktop版ロード中）のEDITモードで各FXシーンを選択し、
  右パネルのFXラック実UI表示と、同ファイルXMLの `RACK/EFFECT/PARAM` 数値を突き合わせ（13シーン、保存なし・読み取りのみ）。
- 証跡: `target/qa/ui-comparison/dvc3-fx-*.png`（各シーンのFXラック切り出し）
- 方針（ユーザー承認済み）: 近似ではなく同一挙動を目標とし、Syndocal側に無いジェネレータは新規実装する。

## 確定した構造

シーンの `RACKS/RACK TYPE=r` がFXファミリー、`EFFECT TYPE=t ID=n` がファミリー内の**名前付きジェネレータ**、
`PARAM ID=k VAL=v` がそのジェネレータのUIパラメータに1対1対応する。

### CHASER FX（RACK=3, EFFECT TYPE=6）→ Syndocal Chaser エンジン

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 321 | **Chaser #1** | 10=One Way Only(1/0)※, 11=Fading(1/0)※, **12=Nb pixels on**（Moving Chaser2で12=2⇔UI 2を確認。Documents版BackBar-Amberの12=18=18chバー全点灯とも整合） |
| 325 | **Chaser random** | 11=Fading(0⇔off✓), 12=Nb pixels on(1✓), 13=Flash(100⇔100.0✓), 14=Random sequence(0✓), 15=Nb cycles(1✓) — **全一致** |
| 322 | **Chaser #2** | **10=Fading**(1⇔ON✓)。選択順に1灯ずつ積み上げ、全点灯後に同じ順で1灯ずつ消す build/clear 周期 — LIVEのDMX Levelsで確定 |

※ 10/11は両方1の検体しかなくOneWay/Fadingの順序は未分離（UI表示順から10=OneWay仮）。Feature（対象属性）はラックの Features 行（実測は全てDimmer）。対象ビームは `BEAMS` 要素。

### CURVE FX（RACK=8, EFFECT TYPE=5）→ Syndocal LFO/Value エンジン

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 7 | **Sinus** | **1=Rate**(10✓×2 / 2✓), **2=Size**(1⇔1.0✓), **3=Phase**(0.25⇔25.0✓ / 0.748⇔74.8✓ — 0..1正規化), **4=Offset**(0✓), **5=Phasing**(0✓) — **全一致・3検体** |
| 3 | **Inverse Ramp**（2026-07-19 homecomingで照合） | all_rampFlash: Rate=2⇔1=2✓, Size=1.56⇔2=1.562✓, Phase=49.5⇔3=0.495✓, Offset=-84.8⇔4=-0.848✓ — **全一致** |
| 10 | 波形未照合（Documents版Fl-Strobe） | 保留 |

時間換算: `EFFECT DURATION=5000` × Rate=N → 周期 = 5000/N ms（Sinus Rate=10 → 500ms周期）。
Phasing = 選択ビーム順の位相分散。2026-08-08 に Syndocal LFO の `fixture_spread` へ 0..1 の正規化値を無変換で保存し、`phase_i = global_phase + i / target_count * Phasing` として command-time に灯体順をコンパイルする実装へ更新した。直接指定灯体の後に group を patch 順で展開し、group membership 変更時も再コンパイルする。Sinus / Inverse Ramp / Strobe の非ゼロ値 0.2 / 0.6 / 0.4 と、3灯体の DMX8 出力 `[64, 0, 128]`（authored order `[2,1,3]`, Saw, spread=0.75, t=0）を回帰試験で固定。旧 `.sdc` は欠落時 0、0 は従来どおり非出力。Attribute value=Absolute。

### COLOR FX（RACK=2, EFFECT TYPE=2）→ Syndocal Color エンジン + 新規パターンレシピ

共通ヘッダ: 1=空, 2=0, 3=0（意味未確定、全検体で同値）。

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 121 | **Burst** | 10=Color Width(50⇔50✓), 11=?（Gradient=100.0関連か） |
| 127 | **Knight Rider** | 10=Size（選択ビーム列に対する百分率。32%/17%/12% 全✓。48-beam `BB-Amber-Chaser` の実出力は Size=32 で約15–16 beams）、11=One Way Only(1⇔ON✓/0⇔off✓), 12=Fading(1⇔ON✓), 13=Go Outside（B-WineRed 1⇔ON✓、BB-Amber 0⇔off✓。Bar-RedWaveのみUI ON vs 13=0の観測矛盾があり要再確認）, 14=Gradient(50✓×3) |
| 131 | **Random fill** | 10=Point Width(1✓) |
| 133 | **Sparkle** | 10=Sparkle Number(5✓), 11=Sparkle LifeSpan(0✓), 12=Sparkle Width(1✓) |
| 129/130 | 未照合（WineRed系、Documents版） | パレット/変種サブレコードの可能性 |

カラーパレットはUI上5枠程度のスウォッチ列（実測: WineRed系5色、FillCyan青系5色+等）。XML上の格納位置は
RACK内の PRESETS/BEAMS 以外の要素にあるとみられ、実装時に特定する。Beams は 4（ムービング群）〜64（8バー×8セグメント）
— **DVC-2で確定したビーム構造の上でセグメント単位に色パターンが走る**。

### MAPPINGS FX（RACK=6, EFFECT TYPE=8）→ Syndocal PositionWave/空間パターン — 照合済み（2026-07-19）

2D空間マッピング系ファミリー。ビーム位置の上をパターンが走る（Beams=38 = 全灯体規模）。

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 521 | **Rainbow** | 3=Transform(1⇔Vertical symmetry✓), **4=Rotation**(171⇔171✓), 10=Color Width(0⇔0.0✓), 11=Angle(0✓), 12=Gradient/100?(1⇔100.0) |
| 530 | **Perlin** | **10=Octaves**(5✓), **11=Zoom**(20✓), **12=Direction**(1✓), **13=Speed**(1✓), **14=Amplitude**(100✓) — **全一致** |

Syndocal受け皿: PositionWave（空間走査）+ Perlin LFO形状 + Transform（対称/回転）は新規要素。

### MOVE FX（RACK=4, EFFECT TYPE=4）→ Syndocal Move エンジン — 照合済み（2026-08-08、Symmetry出力まで実測）

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 223 | **Line** | Left2Right: Phasing=1.0⇔id2=0.01（**表示=id2×100**）✓、M-CenterDivLoop: Phasing=17.6⇔id2=0.176✓、Symmetry=off⇔id3=0✓ |
| 224 | **Polygon** | M-PolyLoop: Phasing=2.0⇔id2=0.02✓、Symmetry=ON⇔id3=1✓ |

**軌道頂点**は `PARAM TYPE=5 ID=1` 内の `<POINTS NB=n><POINT X=.. Y=..>` — 正規化0..1のPan/Tilt座標列
（M-PolyLoopのダイヤ (0.25,0.5)(0.5,0.75)(0.75,0.5)(0.5,0.25) がUI表示と完全一致）。Attribute value=Absolute。
Syndocal Move エンジン（複数点 smooth/line closed path）が直接の受け皿。

2026-08-08にDaslight 5の `M-PolyLoop` を同一時刻帯で `Symmetry=ON/OFF` 切替し、DMX Levelsを直接比較した。
選択順 `[address 138, 120, 111, 129]` の前半2灯はPanが同方向、後半2灯はON時だけPanが中心線を挟んで反転し、
Tiltは同じ軌道位相を維持した。観測例はON=`63/67/186/190`、OFF=`63/67/73/78`（各灯Panの8bit値）。
Syndocalはこの規則を「解決済み灯体順の後半のみPan delta反転」としてruntime、DVC body、Scene Settings編集UI、
Stage previewに実装し、legacy bodyは`false`へdefaultする。証跡は
`target/qa/current-head/daslight-mpoly-sym-on-levels.png`、`daslight-mpoly-sym-off-levels.png`。

### FXファミリー全カタログ（2026-07-19 実機のFX追加チューザーで観測）

STEPS / COLOR FX / CHASER FX / MOVE FX / VALUE FX / CURVE FX / MAPPINGS / COLOR MAPPINGS / SUPER SCENE の9種。
VALUE FX と COLOR MAPPINGS はユーザーの全ショーに検体なし（実装優先度低・将来検体待ち）。

Syndocalの現行Scene Settingsでは、作成時のMAPPINGS / COLOR MAPPINGSを
`2D MAPPING`の1入口へ統合し、既定の新規作成を`ColorMapping`とする。これは
入口の整理であり、`.dvc` importerと保存済みcue-owned FXでは`Mapping`と
`ColorMapping`を別body/editor/runtimeとして保持するため、9種のDaslight
入力語彙と既存DVCの互換性は失わない。

## Syndocal実装方針（DVC-3分割）

1. **DVC-3a（確定分の変換）**: CHASER FX 321/325 → Chaserエフェクト、CURVE FX（ID=波形）→ LFO/Valueエフェクト。
   インポート時にcue所有FX（F4）として生成し、cueリコールで発動。レポートの「Skipped」から「Converted(effect)」へ。
   **→ 2026-07-19 完了**（Codex実装 × Fable検証）: 厳密パラメータ検証つき変換、近似は明示計上（セグメント選択→fixture化、
   pixelsクランプ、Flash 0%床上げ等）。ステップ周期式 = round(EFFECT DURATION / SCENE SPEED / selection_steps)。
   protocol/engine差分なし。検証: `cargo test -p syndocal dvc` 14/14 — 金標準で effects_converted=5 / skipped=2
   （Fl-Strobe ID=10 は未照合波形として正直にSkipped）、合成のリコール発動/リリース停止テスト、
   **GO実経路（TriggerCueListNext）での金標準チェイサー実動テスト**（ステップ境界でDMX変化を断言）。
   ネイティブ実機でもインポートレポートに Effects converted 5 と変換式の明示を確認。
   既知の表示限界（DVC-3aの回帰ではない・別タスク化済み）: 2DマップのLevels/ビームはfixture属性状態由来のため、
   エフェクト変調中の実DMXに追従しない。
2. **DVC-3b（新規パターンレシピ）**: COLOR FX Knight Rider / Burst / Random fill / Sparkle をビーム空間パターンとして
   Colorエンジンに新設（同一挙動目標）。パレットXML格納位置の特定を含む。
   **→ 2026-07-19 完了**（Codex実装 × Fable検証、コミット b4b6a7d）: 6ジェネレータ全て実装・変換。
   パレット実在位置 = `EFFECT/PARAMS/PARAM[@TYPE=4][@ID=1]/COLORS/COLOR@VAL`。COLOR系はprofile順ビームストリップ、
   MAPPINGSは正規化ステージX/Z空間+シード付きフラクタルノイズ。protocol追加はserde(default)のみでv1互換29/29維持。
   金標準変換数: 127:5 / 121:4 / 131:1 / 133:1 / 521:1 / 530:1。dvc 22/22（Knight Rider実発光・時間掃引の
   金標準テスト含む）、engine 370/370、マトリクス232全緑。同条件A/Bベンチ +3.8%（誤差内）で既存スタック非劣化。
   絶対値2ms予算は環境負荷40%のため未計測 — クリーン環境での再計測が残件。
3. **DVC-3c（照合完了）**: MOVE FX / TYPE=8、ID 322・129・130・CURVE波形10を照合済み。
   VALUE FX / COLOR MAPPINGS はユーザーのDVCに検体がないため、既存のネイティブ実装を維持しDVC逆変換の対象外とする。

4. **DVC-3d（2026-08-08、ビーム選択の厳密保持）**: CURVE FXのSinus / Inverse Ramp / Strobeと
   CHASER FX 321 / 322 / 325は、`BEAM@FIXTURE`、`BEAMID`、最初に現れた`IDSELECTION`順を
   `EffectBeamTarget`としてcue所有bodyへ保存する。複数セグメントRGB/RGBA灯体のDimmerは、全体Dimmerへ
   平坦化せず、選択セグメントの現在色を同率でスケールする。単一セグメント灯体で実Dimmerがある場合は
   従来どおりその属性を直接駆動する。旧`.sdc`はフィールド欠落を空配列として読み、通常LFOの新しい
   スナップショットbodyは保存、プリセット化、複製、再読込でビーム列を保持する。

   フル`Shinkan2026.dvc`の固定証拠は、`Bar-StrobeAMber` Curveが64 beam、BEAMID 0..7、
   48 selectionを保持し、`Bar-Side / New Scene` Chaserが16 step / 16 beam、BEAMID 0..7を保持すること。
   これらについてインポートレポートの`segment selection approximated to fixture`は消えた。Move FXは
   Pan/Tiltが灯体単位のため、将来BEAMID>0のMove検体が現れた場合だけ従来の近似報告を残す。

## 実機操作の記録（正直な状態申告）

2026-07-18（Shinkan2026 Desktop版）: EDITモードでのシーン選択クリック13回とキャプチャのみ。保存・GO・LIVEトグル・
スーパーシーン起動なし。編集対象選択は「Chaser」シーンに残置（元は「Shin」スーパーシーン。スーパーシーンのセルは
再生誘発リスクがあるため復元せず）。タイトルの未保存マーク（*）は作業前から存在。

2026-07-19（homecoming2606、ユーザーがロード）: ロード直後はLIVEモードだったため**先にEDITへ切替**（クリック1回）、
その後シーン選択5回とキャプチャのみ。保存・LIVEトグルなし。編集対象は「all_outIn」に残置。

2026-07-19（Shinkan2026 Desktop版、ユーザーが再ロードしMoving-Posバンクまで表示済み）: LIVE→EDIT切替1回、
シーン選択3回（Left2Right/M-PolyLoop/M-CenterDivLoop）とキャプチャのみ。保存なし。編集対象は「M-CenterDivLoop」に残置。
これでユーザーの全ショーに存在する全FXファミリーの照合が完了。

## DVC-3c 完了: 残4種の実機照合（2026-07-31）

ユーザー許可のもと監督が全自動操作（Shinkan-Left版をユーザーが手動ロード→観察後、新規スクラッチプロジェクトで逆引き）。
既存プロジェクトへの保存は一切なし。証跡: `target/qa/ui-comparison/fx-id-*.png`, `wave-*-name.png`, `wavegraph-*.png`,
スクラッチ検体 `target/qa/dvc-id-map/`（dvc-id-map.dvc + wave-1..12.dvc）。

### CURVE FX 波形ID完全対応表（スクラッチ保存差分で全11種確定）

新規プロジェクトにCURVE FXを置き、波形コンボを{DOWN}で1段ずつ進めてはCtrl+S保存→ファイル複製→ID読取。
コンボ表示名はピクセルキャプチャで裏取り。アンカー2点（ID3=Inverse Ramp・ID7=Sinus、過去実測）と完全一致。

| UI名 | ID | | UI名 | ID |
|---|---|---|---|---|
| Sinus | 7 | | Ramp | 5 |
| Sinus3 | 8 | | Inverse Ramp | 3 ✓アンカー |
| Tangeant | 11 | | Random | 6 |
| Triangle | 12 | | **Strobe** | **10** ← Documents版Fl-Strobe |
| Pulse | 4 | | Custom | 13 |
| Square | 9 | | | |

- グラフ窓の表示規則: **窓 = Rate/2 周期**（Rate=1で半周期、Rate=2で1周期、Rate=10で5周期を実測確認）。
- **Strobe波形の実測**: 周期あたり**10本の等間隔パルス**（Rate=2窓で両端含む11エッジ、Rate=1半周期窓で5本）。
  パルス幅は極細（周期の1〜2%程度、描画線幅レベル）。ベースは下限、ピークはSize=1.0でゼロ軸（半振幅）。
  → **Size=2でフルレンジフラッシュ**。Documents版Fl-Strobeは Rate=2/Size=2/Phase=0/Offset=0/Phasing=0/
  DURATION=5000 = 周期2500ms・250ms間隔のフルフラッシュ×10。
- Pulse(4)は矩形ではなく減衰振動波、Square(9)は50%デューティ矩形（キャプチャ有）。波形切替でRate等は既定値へ戻る。

### COLOR FX ID=129 = Plasma（B-WineRed (2) 実UI照合・全11パラメータ一致）

UI: Size X=1/Param=2/Size Y=1/Param=2/Speed X=-1/Param=2/Speed Y=1/Param=-1、パレット5色、Grayscale off、
Transform None、64 Beams。XML: ID1=COLORS(TYPE=4)パレット、ID2=Grayscale、ID3=Transform、
ID10..17=(SizeX, ParamX, SizeY, ParamY, SpeedX, ParamSX, SpeedY, ParamSY) — 全て1対1一致。
**パレット格納場所を確定**: `PARAM TYPE=4 ID=1` 内 `<COLORS NB=n><COLOR VAL="r/g/b/…(18要素)"/>`、
先頭3要素が正規化RGB（スウォッチ5色と画素一致）。

### COLOR FX ID=130 = Rainbow（PS-WineRed 実UI照合・全パラメータ一致）

UI: Color Width=0.0/Angle=0/Gradient=100.0、パレット10色、2 Beams。XML: ID1=COLORSパレット(10色)、ID2=Grayscale、
ID3=Transform、ID10=Color Width、ID11=Angle、ID12=Gradient(UI=VAL×100)。MAPPINGSの521 Rainbowとは別物
（COLOR FX系はビームストリップ上の掃引、パラメータ構成も異なる）。

### CHASER FX ID=322 = Chaser #2（SS-Blue実UI + populatedスクラッチDMX照合）

Daslight UI自身が「0 Beam(s)・Features空」を表示する個体は原本でも空（無発光）。321系と同じ
無対象FXとして、ランタイムへ架空の対象やブラックアウトを作らず `source no-op` 変換記録だけを保持する。
Documents版Shinkan2026の Bar / New Scene (321)、SaberSpot / SS-Blue (322)、Par / Par-Chaser (321) は
`BEAMS NB="0"` を実XMLで確認済み。Desktopフル版にはこれらとは別に16 beamのBar-Side / New Scene (321)があり、
DVC-3dで16 stepのビーム列として保持する。空個体は原本どおり無発光で、欠損/Skippedには数えない。

2026-08-08、最大化したDaslight 5のスクラッチ `Codex-Chaser322-Probe.dvc` で4台のSaberSpotを選択し、
Chaser #2 / Fading ONをLIVE再生してDMX Levelsを連続採取した。対象Dimmerは ch86/91/96/101。
観測は「ch86をフェード点灯→ch86保持のままch91→ch96→ch101を追加→全点灯→ch86から同順にフェード消灯→全消灯」。
EDIT時に残留した255値は編集バッファ汚染として棄却し、LIVEでゼロから始まる周期だけを採用した。
したがってID322は専用 `BuildUpDown` 方向へ厳密変換し、周期スロット数は `2 × BEAMS`、
`step_duration_ms = round(EFFECT DURATION / generator_slots)` とする。1 Beamでも架空のブラックアウトを追加しない。
Fading OFFは各追加/消去境界で即時切替、ONはスロット全長の線形フェード。物理灯体は未使用で、DMX値上の照合である。

### FXファミリー選択肢の全貌（新規シーンのFX追加メニュー実観察）

STEPS / COLOR FX / CHASER FX / MOVE FX / VALUE FX / CURVE FX / MAPPINGS / COLOR MAPPINGS / SUPER SCENE。

### 実機操作の記録（正直な状態申告・2026-07-31）

Shinkan-Left版（ユーザー手動ロード）: シーン選択ダブルクリック3回（PS-WineRed/SS-Blue/Fl-Strobe）、
波形ドロップダウン開閉（Esc復帰）、キャプチャのみ。**保存なし・未保存編集なし**。その後 File→New で
スクラッチ「dvc-id-map.dvc」をデスクトップへ新規保存し（既存ファイル非接触）、波形切替+Ctrl+S×12回は
全てスクラッチのみ。誤操作でOpen...ダイアログを1回開いたがESCで即キャンセル（ファイル未選択）。
終了状態: Daslightはスクラッチプロジェクトを表示したままプライマリモニタに最大化（Shinkan-Leftは
無変更のままクローズ済み。必要なら手動で開き直し）。スクラッチ検体はtarget/qa/dvc-id-map/へ回収済み。
