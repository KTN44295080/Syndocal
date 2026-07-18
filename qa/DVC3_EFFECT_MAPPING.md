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
| 7 | **Sinus** | **1=Rate**(10✓×2検体), **2=Size**(1⇔1.0✓), **3=Phase**(0.25⇔25.0✓ / 0⇔0.0✓ — 0..1正規化), **4=Offset**(0✓), **5=Phasing**(0✓) — **全一致** |
| 3 | 波形未照合（all_rampFlash、homecoming） | Ramp系と推定。id2=1.562,id3=0.495,id4=-0.848 |
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

### MOVE FX（RACK=4, EFFECT TYPE=4）→ Syndocal Move エンジン — 未照合

ID 223（Left2Right, id2=0.01=速度?）/ 224（M-PolyLoop, id2=0.02）。Documents版のみのため実UI未照合。
照合にはDaslightへの別プロジェクトロードが必要（ユーザー同席時に実施）。

### RACK=6 / EFFECT TYPE=8 — 未照合

ID 521（all_outIn）/ 530（all_random）。homecoming2606のみ。同上の理由で保留。

## Syndocal実装方針（DVC-3分割）

1. **DVC-3a（確定分の変換）**: CHASER FX 321/325 → Chaserエフェクト、CURVE FX（ID=波形）→ LFO/Valueエフェクト。
   インポート時にcue所有FX（F4）として生成し、cueリコールで発動。レポートの「Skipped」から「Converted(effect)」へ。
2. **DVC-3b（新規パターンレシピ）**: COLOR FX Knight Rider / Burst / Random fill / Sparkle をビーム空間パターンとして
   Colorエンジンに新設（同一挙動目標）。パレットXML格納位置の特定を含む。
3. **DVC-3c（残り）**: MOVE FX / TYPE=8 / ID 322・129・130 / CURVE波形3・10 の照合と変換。要・別プロジェクトロード。

## 実機操作の記録（正直な状態申告）

Daslightに対して行ったのはEDITモードでのシーン選択クリック13回とキャプチャのみ。保存・GO・シーンセルのLIVEトグル・
スーパーシーン起動は行っていない。編集対象選択は最後に「Chaser」シーンに残っている（元は「Shin」スーパーシーンだったが、
スーパーシーンのセルクリックはタイムライン再生を誘発しうるため意図的に復元していない）。タイトルの未保存マーク（*）は
作業前から存在した。
