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
| 322 | 未照合（SS-Blue、Documents版のみ。[10=1]） | 保留 |

※ 10/11は両方1の検体しかなくOneWay/Fadingの順序は未分離（UI表示順から10=OneWay仮）。Feature（対象属性）はラックの Features 行（実測は全てDimmer）。対象ビームは `BEAMS` 要素。

### CURVE FX（RACK=8, EFFECT TYPE=5）→ Syndocal LFO/Value エンジン

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 7 | **Sinus** | **1=Rate**(10✓×2 / 2✓), **2=Size**(1⇔1.0✓), **3=Phase**(0.25⇔25.0✓ / 0.748⇔74.8✓ — 0..1正規化), **4=Offset**(0✓), **5=Phasing**(0✓) — **全一致・3検体** |
| 3 | **Inverse Ramp**（2026-07-19 homecomingで照合） | all_rampFlash: Rate=2⇔1=2✓, Size=1.56⇔2=1.562✓, Phase=49.5⇔3=0.495✓, Offset=-84.8⇔4=-0.848✓ — **全一致** |
| 10 | 波形未照合（Documents版Fl-Strobe） | 保留 |

時間換算: `EFFECT DURATION=5000` × Rate=N → 周期 = 5000/N ms（Sinus Rate=10 → 500ms周期）。
Phasing = 選択ビーム間の位相分散（Syndocal側はLFO位相 + 分散の実装が必要な可能性）。Attribute value=Absolute。

### COLOR FX（RACK=2, EFFECT TYPE=2）→ Syndocal Color エンジン + 新規パターンレシピ

共通ヘッダ: 1=空, 2=0, 3=0（意味未確定、全検体で同値）。

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 121 | **Burst** | 10=Color Width(50⇔50✓), 11=?（Gradient=100.0関連か） |
| 127 | **Knight Rider** | 10=Size(32/17/12 全✓), 11=One Way Only(1⇔ON✓/0⇔off✓), 12=Fading(1⇔ON✓), 13=Go Outside（B-WineRed 1⇔ON✓、BB-Amber 0⇔off✓。Bar-RedWaveのみUI ON vs 13=0の観測矛盾があり要再確認）, 14=Gradient(50✓×3) |
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

### MOVE FX（RACK=4, EFFECT TYPE=4）→ Syndocal Move エンジン — 照合済み（2026-07-19、全ファミリー完了）

| ID | ジェネレータ | パラメータ対応（実測） |
|---|---|---|
| 223 | **Line** | Left2Right: Phasing=1.0⇔id2=0.01（**表示=id2×100**）✓、M-CenterDivLoop: Phasing=17.6⇔id2=0.176✓、Symmetry=off⇔id3=0✓ |
| 224 | **Polygon** | M-PolyLoop: Phasing=2.0⇔id2=0.02✓、Symmetry=ON⇔id3=1✓ |

**軌道頂点**は `PARAM TYPE=5 ID=1` 内の `<POINTS NB=n><POINT X=.. Y=..>` — 正規化0..1のPan/Tilt座標列
（M-PolyLoopのダイヤ (0.25,0.5)(0.5,0.75)(0.75,0.5)(0.5,0.25) がUI表示と完全一致）。Attribute value=Absolute。
Syndocal Move エンジン（複数点 smooth/line closed path）が直接の受け皿。

### FXファミリー全カタログ（2026-07-19 実機のFX追加チューザーで観測）

STEPS / COLOR FX / CHASER FX / MOVE FX / VALUE FX / CURVE FX / MAPPINGS / COLOR MAPPINGS / SUPER SCENE の9種。
VALUE FX と COLOR MAPPINGS はユーザーの全ショーに検体なし（実装優先度低・将来検体待ち）。

## Syndocal実装方針（DVC-3分割）

1. **DVC-3a（確定分の変換）**: CHASER FX 321/325 → Chaserエフェクト、CURVE FX（ID=波形）→ LFO/Valueエフェクト。
   インポート時にcue所有FX（F4）として生成し、cueリコールで発動。レポートの「Skipped」から「Converted(effect)」へ。
2. **DVC-3b（新規パターンレシピ）**: COLOR FX Knight Rider / Burst / Random fill / Sparkle をビーム空間パターンとして
   Colorエンジンに新設（同一挙動目標）。パレットXML格納位置の特定を含む。
3. **DVC-3c（残り）**: MOVE FX / TYPE=8 / ID 322・129・130 / CURVE波形3・10 の照合と変換。要・別プロジェクトロード。

## 実機操作の記録（正直な状態申告）

2026-07-18（Shinkan2026 Desktop版）: EDITモードでのシーン選択クリック13回とキャプチャのみ。保存・GO・LIVEトグル・
スーパーシーン起動なし。編集対象選択は「Chaser」シーンに残置（元は「Shin」スーパーシーン。スーパーシーンのセルは
再生誘発リスクがあるため復元せず）。タイトルの未保存マーク（*）は作業前から存在。

2026-07-19（homecoming2606、ユーザーがロード）: ロード直後はLIVEモードだったため**先にEDITへ切替**（クリック1回）、
その後シーン選択5回とキャプチャのみ。保存・LIVEトグルなし。編集対象は「all_outIn」に残置。

2026-07-19（Shinkan2026 Desktop版、ユーザーが再ロードしMoving-Posバンクまで表示済み）: LIVE→EDIT切替1回、
シーン選択3回（Left2Right/M-PolyLoop/M-CenterDivLoop）とキャプチャのみ。保存なし。編集対象は「M-CenterDivLoop」に残置。
これでユーザーの全ショーに存在する全FXファミリーの照合が完了。
