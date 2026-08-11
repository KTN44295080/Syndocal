# Daslight 5 FX 全カタログ静的・実機証明（DVC-ENUM）

- 確定日: 2026-08-10
- 対象: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- ProductVersion: `5.0.6.2`
- FileVersion: `25.0905.165.111`
- size: `9,778,688 bytes`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- 実機保存標本: `qa/specimens/DVC-ENUM-25.0905.165.111.dvc`
- 元にした既存golden: `qa/specimens/ValueCatalog-Sweep-Plasma.dvc`

## 結論

Daslight 5.0.6.2 が新規シーンで提示する9ファミリーを実機GUIで全て開き、
ドロップダウンの全UI名を最下端まで観測した。同じ実行ファイルのfactory登録域を機械走査し、
全ての名前付きFXについて `family/type + ID -> creator -> constructor -> RTTI/vtable -> evaluator`
を静的に回収した。現バイナリについて**未列挙の名前付きFXはない**。

この文書の「完全」はカタログ、class identity、serialized property schema、evaluator入口の完全性を指す。
全evaluatorの演出式を意味論まで復元済み、または全FXをSyndocalへ実装済み、という意味ではない。
未復元境界は末尾に分離する。

## 証跡手順と非破壊境界

1. 追跡済みgoldenを直接編集せず、`target/qa/dvc-enum/` に新規scratch copyを作成した。
2. Daslightのscratch専用ウィンドウでファミリーチューザーと全ドロップダウンを観測した。
3. COLOR FX / CHASER FX / CURVE FX / MAPPINGS / STEPS / SUPER SCENEを新規sceneとして追加し、
   `Ctrl+S`はscratchにだけ実行した。元goldenのSHA-256は
   `0A4785BC8F2929AA781AC2CD2D5121D63424371EC3421BBD3C95D7F13E6493B4` のまま不変である。
4. 保存後scratch（78,758 bytes）のSHA-256は
   `29411CE809DB298A573AECAA40666B2AB8D186352A9F8BABEE64899C35E8EC2A`。
5. scratchの実保存 `RACK/EFFECT/PARAM` とfactory走査結果を突合した。

実機GUIの全9入口は次のとおり。

| GUI入口 | 保存形 | 名前付きgenerator数 |
|---|---|---:|
| STEPS | `RACK TYPE=9 / STEPS/STEP` | 専用構造 |
| COLOR FX | `RACK TYPE=2 / EFFECT TYPE=2` | 8 |
| CHASER FX | `RACK TYPE=3 / EFFECT TYPE=6` | 5 |
| MOVE FX | `RACK TYPE=4 / EFFECT TYPE=4` | 5 |
| VALUE FX | `RACK TYPE=7 / EFFECT TYPE=7` | 8 |
| CURVE FX | `RACK TYPE=8 / EFFECT TYPE=5` | 11 |
| MAPPINGS | `RACK TYPE=6 / EFFECT TYPE=8` | 10 |
| COLOR MAPPINGS | `RACK TYPE=5 / EFFECT TYPE=3` | 21 |
| SUPER SCENE | `RACK TYPE=1 / TIMELINES/TIMELINE/BLOCKS` | 専用構造 |

scratch実保存のdefault anchorは COLOR FX Rainbow=130、CHASER #1=321、CURVE Sinus=7、
MAPPINGS Rainbow=521。STEPSは `WAITTIME=25 / FADETIME=0` の1 step、SUPER SCENEは
空の1 timelineとして保存された。

## VALUE FX — family/type 7

GUI順とfactory集合は完全一致する。

| ID | UI名 | class | evaluator |
|---:|---|---|---:|
| 621 | Rainbow | `CRainbowEffect` | `0x140365A00` |
| 622 | Burst | `CBurstEffect` | `0x140362B70` |
| 623 | Plasma | `CPlasmaEffect` | `0x1403654F0` |
| 624 | Knight Rider | `CKnightRiderEffect` | `0x140363FE0` |
| 625 | Sweep | `CSweepEffect` | `0x1403665A0` |
| 626 | Sparkle | `CSparklesEffect` | `0x1403660F0` |
| 627 | Random fill | `CRandomFillEffect` | `0x140365C70` |
| 628 | Perlin | `CPerlinEffect` | `0x140365090` |

全propertyのTYPE/ID/default/domain、factory/constructor/vtable、評価式、現行境界は
`qa/DVC_VALUE_CATALOG_PARITY.md`を正本とする。DVC-V6で624、DVC-C0bで同じ回収済み評価器を
共有するCOLOR 127、DVC-BURST-EXACTで622と共有するCOLOR 121を実装した。当時は回収した
40 ms evaluator/cacheも互換契約に含めたため、これらBurst/Knight Rider/Sweepの内部artifactは
現行にも残る。ただしimport requestのperiod自体は、その後authored `DURATION`保持へ訂正済みである。
共有constructor `0x140350BE0` の `0x14035155A` がpalette-wrap `+0x12c=true`を無条件に
初期化することを再証明し、旧「未初期化」判定を撤回した。DVC-V3/V6当時は628を未実装の
固有Perlin evaluator差、626/627を非serialize Qt/CRT RNG履歴によりfail-closedとしたが、
その境界はその後supersededになった。現行は628をcontinuous/analyticなDVC Corrected Perlin、
626/627をstable source seedのCorrected evaluatorとしてimportする。

## COLOR FX — family/type 2

factory登録集合は `{121,127,128,129,130,131,133,134}`。旧推定のSweep=132は誤りで、
**Sweep=134**。122–126と132はこのバイナリのfactoryに登録がない空隙であり、
「予約」「廃止」といった製品履歴までは断言しない。

共通property:

- `TYPE4/ID1 Color Palette`: 1..255 colors
- `TYPE2/ID2 Grayscale`: default false
- `TYPE6/ID3 Transform`: default 0、`0=None / 1=Vertical symmetry`
- Horizontal symmetryはfamily 3/8だけに追加され、COLOR FXにはない

| ID / UI名 | factory ID-store -> creator -> ctor | class / vtable | evaluator | 固有schema（TYPE/ID） |
|---|---|---|---:|---|
| 121 Burst | `0x14036A296 -> 0x14036CA20 -> 0x1403503C0` | `CBurstEffect / 0x140695D08` | `0x140362B70` | `T0/10 Color Width=50[10..900]`; `T1/11 Gradient=1[0..1]` |
| 127 Knight Rider | `0x14036A0A6 -> 0x14036D040 -> 0x140352ED0` | `CKnightRiderEffect / 0x140696548` | `0x140363FE0` | `T0/10 Size=1[1..100]`; `T2/11 One Way=false`; `T2/12 Fading=true`; `T2/13 Go Outside=false`; `T0/14 Gradient=50[0..100]` |
| 128 Perlin | `0x140369CE4 -> 0x14036D200 -> 0x140353F50` | `CPerlinEffect / 0x140696860` | `0x140365090` | `T0/10 Octaves=4[2..10]`; `11 Zoom=75[1..100]`; `12 Direction=2[1..100]`; `13 Speed=1[1..10]`; `14 Amplitude=70[5..100]` |
| 129 Plasma | `0x14036A19E -> 0x14036D270 -> 0x1403542A0` | `CPlasmaEffect / 0x140695F18` | `0x1403654F0` | `T0/10..17 Size X=1, Param X=2, Size Y=1, Param Y=2, Speed X=-1, Param SX=2, Speed Y=1, Param SY=-1`; domains `0..20` / `-5..5` |
| 130 Rainbow | `0x14036A38E -> 0x14036D4A0 -> 0x140354A80` | `CRainbowEffect / 0x140695AF8` | `0x140365A00` | `T1/10 Color Width=0[0..1]`; `T0/11 Angle=0[0..360]`; `T1/12 Gradient=1[0..1]` |
| 131 Random fill | `0x140369DD2 -> 0x14036D660 -> 0x140354D60` | `CRandomFillEffect / 0x140696650` | `0x140365C70` | `T0/10 Point Width=1[1..10]`; family2ではID11 Heightを登録しない |
| 133 Sparkle | `0x140369EC0 -> 0x14036D7B0 -> 0x140354ED0` | `CSparklesEffect / 0x140696B78` | `0x1403660F0` | `T0/10 Number=5[1..10]`; `T1/11 LifeSpan=0[0..0.9]`; `T0/12 Width=1[1..90]`; family2ではID13 Heightなし |
| 134 Sweep | `0x140369FB5 -> 0x14036D9E0 -> 0x140355D30` | `CSweepEffect / 0x140696440` | `0x1403665A0` | `T2/10 Direction Change=false` |

## MOVE FX — family/type 4

GUI順は Circle / Curve / Polygon / Line / Points。factory集合は221–225の連番で空隙なし。
全classは共通constructor `0x140347890` だけを使用し、派生固有propertyはない。

- `TYPE5/ID1 POINTS`: 2..255 normalized points
- `TYPE1/ID2 Phasing`: default 0、0..1
- `TYPE2/ID3 Symmetry`: default false
- property ID4は存在しない

| ID / UI名 | creator -> ctor | class / vtable | evaluator | default POINTS |
|---|---|---|---:|---|
| 221 Circle | `0x14036CD30 -> 0x1403476C0` | `CCirclePosEffect / 0x1406954C0` | `0x140349650` | diamond 4点 |
| 222 Curve | `0x14036CDA0 -> 0x140347720` | `CCurvePosEffect / 0x140695598` | `0x14034A0E0` | `(1/6,.25),(1/3,.75),(2/3,.25),(5/6,.75)` |
| 223 Line | `0x14036D120 -> 0x140347AF0` | `CLinePosEffect / 0x140695748` | `0x14034A460` | `(.5,.25),(.5,.75)` |
| 224 Polygon | `0x14036D350 -> 0x140347D60` | `CPolygonPosEffect / 0x140695670` | `0x14034A8F0` | diamond 4点 |
| 225 Points | `0x14036D2E0 -> 0x140347C20` | `CPointsPosEffect / 0x140695820` | `0x14034A790` | `(.5,.25),(.5,.75)` |

DVC-MOVE-EXACTで5 evaluatorの式、Daslightの40 ms整数frame、外側補間、raw Phasing、
2-wing Symmetryを静的回収した。当時の最初のruntimeはそのframe契約も再現したが、その時間契約は
corrected timing trancheでsupersededになった。現行はauthored `DURATION`、continuous Phasing、
連続path評価を使い、Pointsだけを意図した頂点holdとして残す。ordered beam/selection identityと
geometryは維持する。詳細とassertion deltaは`qa/DVC_MOVE_EXACT_PARITY.md`を正本とする。

## CHASER FX — family/type 6

GUI順とfactory集合は321–325の連番で一致し、空隙なし。共通base `0x140374F80` はpropertyを登録しない。

| ID / UI名 | creator -> ctor | class / vtable | evaluator | schema |
|---|---|---|---:|---|
| 321 Chaser #1 | `0x14036CB00 -> 0x140374780` | `CChaserType1Effect / 0x1406C88C8` | `0x140376540` | `T2/10 One Way=true`; `T2/11 Fading=false`; `T0/12 Nb pixels on=1[0..1000]` |
| 322 Chaser #2 | `0x14036CB70 -> 0x1403748F0` | `CChaserType2Effect / 0x1406C8998` | `0x140376FA0` | `T2/10 Fading=true` |
| 323 Chaser #3 | `0x14036CBE0 -> 0x1403749B0` | `CChaserType3Effect / 0x1406C8A68` | `0x1403775F0` | 321と同じ3 property |
| 324 Chaser #4 | `0x14036CC50 -> 0x140374B20` | `CChaserType4Effect / 0x1406C8B38` | `0x140378400` | 321と同じ3 property |
| 325 Chaser random | `0x14036CCC0 -> 0x140374C90` | `CChaserType5EffectRandom / 0x1406C8C08` | `0x1403791F0` | `T2/11 Fading=false`; `T0/12 Nb pixels on=1[0..1000]`; `T1/13 Flash=100[0..100]`; `T0/14 Random sequence=0[0..255]`; `T0/15 Nb cycles=1[1..255]` |

321–325の全5種にconverter routeがある。323/324はpinned binaryのevaluatorから、保存beam順を
両端から中央へpair化するoutside-in順と、その正確な逆順であるcenter-out順を回収した。import時に
既存Chaser stepへ一度だけ変換し、One Way / Fading / Nb pixels onも共通runtimeへ結線する。
40 ms work-frameと折返し重複frameはCorrected連続clockへ置換する。式、実保存32-beam specimen、
偶奇topology regressionの正本は`qa/DVC_CHASER_SYMMETRIC_PARITY.md`とする。

## CURVE FX — family/type 5

GUI順は Sinus / Sinus3 / Tangeant / Triangle / Pulse / Square / Ramp /
Inverse Ramp / Random / Strobe / Custom。全11項目をスクロール最下端まで実機観測した。

ID3–12は共通constructor `0x14036E500`:

- `TYPE0/ID1 Rate=2 [1..10]`
- `TYPE1/ID2 Size=1 [0..2]`
- `TYPE1/ID3 Phase=0 [0..1]`
- `TYPE1/ID4 Offset=0 [-1..1]`
- `TYPE1/ID5 Phasing=0 [0..1]`

Custom ID13だけはこの5-property schemaではない。base listを消去し、
`TYPE5/ID1 Points=[(0,.5),(1,.5)] [NB 2..255]` と `TYPE1/ID2 Phasing=0[0..1]` を登録する。

| ID / UI名 | factory ID-store -> creator -> ctor | class / vtable | evaluator |
|---|---|---|---:|
| 3 Inverse Ramp | `0x140369547 -> 0x14036D580 -> 0x14036E860` | `CRampInvEffect / 0x1406C8258` | `0x14036FCB0` |
| 4 Pulse | `0x140369826 -> 0x14036D3C0 -> 0x14036E7A0` | `CPulseEffect / 0x1406C83F8` | `0x14036F8D0` |
| 5 Ramp | `0x14036963C -> 0x14036D510 -> 0x14036E800` | `CRampEffect / 0x1406C8188` | `0x14036FAE0` |
| 6 Random | `0x140369452 -> 0x14036D5F0 -> 0x14036E8C0` | `CRandomEffect / 0x1406C8328` | `0x14036FE90` |
| 7 Sinus | `0x140369BFA -> 0x14036D740 -> 0x14036E9E0` | `CSinusEffect / 0x1406C7E48` | `0x140370250` |
| 8 Sinus3 | `0x140369B05 -> 0x14036D6D0 -> 0x14036E980` | `CSinus3Effect / 0x1406C7F18` | `0x140370070` |
| 9 Square | `0x140369731 -> 0x14036D890 -> 0x14036EA40` | `CSquareEffect / 0x1406C80B8` | `0x140370420` |
| 10 Strobe | `0x14036935D -> 0x14036D970 -> 0x14036EAA0` | `CStrobeEffect / 0x1406C84C8` | `0x1403705B0` |
| 11 Tangeant | `0x140369A10 -> 0x14036DA50 -> 0x14036EB00` | `CTangeantEffect / 0x1406C7FE8` | `0x140370760` |
| 12 Triangle | `0x14036991B -> 0x14036DB30 -> 0x14036EB60` | `CTriangleEffect / 0x1406C8598` | `0x140370930` |
| 13 Custom | `0x140369268 -> 0x14036CE10 -> 0x14036E270` | `CCustomCurveEffect / 0x1406C8668` | `0x14036F1F0` |

ID3–12の式は`qa/DVC_CURVE_SOURCE_PARITY.md`で意味論まで確定済み。Ramp / Random /
Sinus3 / Tangeant / Triangleは2026-08-11の追加逆アセンブルで確定し、Random constructorの
400-entry `qrand()%100` tableと非serialize process historyも分離した。Custom ID13だけは
factory入口とserialized points schemaまでの回収で、evaluator意味論は別トランシェに残る。

## MAPPINGS / COLOR MAPPINGS — family/type 8 / 3

両familyは同じraster effect class群を再利用するが、保存family、base property、利用class集合が異なる。

MAPPINGS base:

- `TYPE4/ID1 Values`: default white / 0.498 gray / black
- `TYPE6/ID3 Transform`: `0=None / 1=Vertical / 2=Horizontal`
- `TYPE0/ID4 Rotation=0 [0..360]`
- ID2は登録しない

COLOR MAPPINGS baseは上記に `TYPE2/ID2 Grayscale=false` を追加し、default paletteは8色rainbow。

### 完全ID表

| class / UI名 | MAPPINGS ID | COLOR MAPPINGS ID | evaluator |
|---|---:|---:|---:|
| `CRainbowEffect` / Rainbow | 521 | 36 | `0x140365A00` |
| `CSpiralEffect` / Spiral | 522 | 42 | `0x140366240` |
| `CBurstEffect` / Burst | 523 | 22 | `0x140362B70` |
| `CButterflyEffect` / Butterfly | 524 | 23 | `0x140362EB0` |
| `CPlasmaEffect` / Plasma | 525 | 34 | `0x1403654F0` |
| `CMediaEffect` / Media | 526 | 33 | `0x140364940` |
| `CKnightRiderEffect` / Knight Rider | 527 | 30 | `0x140363FE0` |
| `CSweepEffect` / Sweep | 528 | 44 | `0x1403665A0` |
| `CSparklesEffect` / Sparkle | 529 | 40 | `0x1403660F0` |
| `CPerlinEffect` / Perlin | 530 | 32 | `0x140365090` |
| `CBounceEffect` / Bounce | — | 21 | `0x1403627E0` |
| `CFireEffect` / Fire | — | 29 | `0x140363720` |
| `CLineEffect` / Lines | — | 31 | `0x1403641D0` |
| `CRainEffect` / Rain | — | 35 | `0x140365660` |
| `CRandomFillEffect` / Random fill | — | 37 | `0x140365C70` |
| `CTubeEffect` / Tube | — | 41 | `0x1403660F0` |
| `CTextEffect` / Text | — | 45 | `0x140366940` |
| `CExplosionEffect` / Explosion | — | 47 | `0x1403635E0` |
| `CStarfieldEffect` / Starfield | — | 48 | `0x140366460` |
| `CGraphEffect` / Graph | — | 49 | `0x1403638A0` |
| `CGridEffect` / Grid | — | 50 | `0x140363C00` |

factoryのexact ID-storeは、MAPPINGSが
`521@0x1403687E1, 522@0x1403686E9, 523@0x1403685F7, 524@0x140368514,
525@0x140368437, 526@0x140368361, 527@0x140368284, 528@0x1403681AE,
529@0x1403680D8, 530@0x140368002`。COLOR MAPPINGSが
`21@0x14036B2F1, 22@0x14036B5D2, 23@0x14036B4DA, 29@0x14036B1F9,
30@0x14036B010, 31@0x14036A586, 32@0x14036A959, 33@0x14036B101,
34@0x14036B3E9, 35@0x14036AA51, 36@0x14036B7BB, 37@0x14036AB42,
40@0x14036AD2F, 41@0x14036AC37, 42@0x14036B6C3, 44@0x14036AE20,
45@0x14036AF18, 47@0x14036A868, 48@0x14036A770, 49@0x14036A67B,
50@0x14036A491`。

共有raster classのcreator/constructor/vtable chain:

| class | creator -> ctor | vtable |
|---|---|---:|
| `CBounceEffect` | `0x14036C9B0 -> 0x14034FA50` | `0x140696968` |
| `CBurstEffect` | `0x14036CA20 -> 0x1403503C0` | `0x140695D08` |
| `CButterflyEffect` | `0x14036CA90 -> 0x140350540` | `0x140695E10` |
| `CExplosionEffect` | `0x14036CE80 -> 0x140351690` | `0x140696D98` |
| `CFireEffect` | `0x14036CEF0 -> 0x1403520C0` | `0x140696020` |
| `CGraphEffect` | `0x14036CF60 -> 0x140352530` | `0x140696FA8` |
| `CGridEffect` | `0x14036CFD0 -> 0x140352B60` | `0x1406971B8` |
| `CKnightRiderEffect` | `0x14036D040 -> 0x140352ED0` | `0x140696548` |
| `CLineEffect` | `0x14036D0B0 -> 0x140353300` | `0x1406970B0` |
| `CMediaEffect` | `0x14036D190 -> 0x140353840` | `0x140696230` |
| `CPerlinEffect` | `0x14036D200 -> 0x140353F50` | `0x140696860` |
| `CPlasmaEffect` | `0x14036D270 -> 0x1403542A0` | `0x140695F18` |
| `CRainEffect` | `0x14036D430 -> 0x1403546C0` | `0x140696A70` |
| `CRainbowEffect` | `0x14036D4A0 -> 0x140354A80` | `0x140695AF8` |
| `CRandomFillEffect` | `0x14036D660 -> 0x140354D60` | `0x140696650` |
| `CSparklesEffect` | `0x14036D7B0 -> 0x140354ED0` | `0x140696B78` |
| `CSpiralEffect` | `0x14036D820 -> 0x140355230` | `0x140695C00` |
| `CStarfieldEffect` | `0x14036D900 -> 0x140355410` | `0x140696EA0` |
| `CSweepEffect` | `0x14036D9E0 -> 0x140355D30` | `0x140696440` |
| `CTextEffect` | `0x14036DAC0 -> 0x140355E10` | `0x140696338` |
| `CTubeEffect` | `0x14036DBA0 -> 0x140356240` | `0x140696C88` |

Tubeのvtable evaluator slotがSparklesと同じ`0x1403660F0`を指すことは観測値のまま記録する。
直感で別addressへ「修正」しない。

COLOR MAPPINGSの実機GUI順は Rainbow / Spiral / Burst / Butterfly / Plasma / Bounce /
Fire / Media / Knight Rider / Text / Sweep / Sparkle / Tube / Random fill / Rain / Perlin /
Explosion / Starfield / Graph / Lines / Grid。21項目を最下端Gridまで観測した。

### class固有serialized schema

| class | 固有schema（baseへ加算） |
|---|---|
| Bounce | `T6/10 Item={Points,Shape}`; `T7/11 Shape`; `T0/12 Number=6[1..20]`; `13 Size=40[1..100]`; `14 Speed=1[0..10]`; `T2/16 Collide=false`; `T2/17 Fill=false`; `T0/18 Points=3[2..10]`; ID15なし |
| Burst | `T0/10 Color Width=50[10..900]`; `T1/11 Gradient=1[0..1]` |
| Butterfly | `T0/10 Color Width=100[1..100]`; `T1/11 Gradient=.5[0..1]`; `T2/12 Clockwise=true` |
| Explosion | `T7/10 Shape`; `T0/11 Explosion number=5[1..50]`; `12 size=5[0..100]`; `13 particles=10[1..100]`; `14 particle size=10[1..100]`; `T1/15 life=0[0..0.9]`; `T0/16 trail=10[1..25]`; `T1/17 gravity=0[0..10]` |
| Fire | `T0/10 Flames=20[1..100]`; `11 Width=20[10..200]`; `12 Height=50[1..100]`; `13 Hotspot=250[10..255]` |
| Graph | `T0/10 Height=10[1..100]`; `11 Width=10[1..100]`; `12 Pitch=10[0..100]`; `13 Frequency=2[0..10]`; `T1/14 Amplitude=1[0..2]`; `15 Offset=0[-1..1]` |
| Grid | `T0/10 Size=1[1..5]`; `11 Width=2[2..20]` |
| Knight Rider | `T0/10 Size=1[1..100]`; `T2/11 One Way=false`; `12 Fading=true`; `13 Go Outside=false`; `T0/14 Gradient=50[0..100]` |
| Lines | `T0/10 Size=2[2..20]` |
| Media | `T8/10 Media Path`; COLOR MAPPINGSはさらに`T1/11 Colorize=0[0..1]` |
| Perlin | `T0/10 Octaves=4[2..10]`; `11 Zoom=75[1..100]`; `12 Direction=2[1..100]`; `13 Speed=1[1..10]`; `14 Amplitude=70[5..100]` |
| Plasma | `T0/10..17 Size X=1, Param X=2, Size Y=1, Param Y=2, Speed X=-1, Param SX=2, Speed Y=1, Param SY=-1`; domains `0..20` / `-5..5` |
| Rain | `T0/10 Speed=1[0..10]`; `11 Width=5[5..10]`; `12 Height=10[10..30]`; `13 Number=50[1..100]`; `14 Trail=10[1..30]` |
| Rainbow | `T1/10 Color Width=0[0..1]`; `T0/11 Angle=0[0..360]`; `T1/12 Gradient=1[0..1]` |
| Random fill | `T0/10 Point Width=1[1..10]`; `11 Point Height=1[1..10]` |
| Sparkle | `T0/10 Number=5[1..10]`; `T1/11 LifeSpan=0[0..0.9]`; `T0/12 Width=1[1..90]`; `13 Height=1[1..90]` |
| Spiral | `T0/10 Radius=30[0..200]`; `11 Arms=1[1..10]`; `T1/12 Gradient=1[0..1]` |
| Starfield | `T7/10 Shape`; `T0/11 Particles=1[1..10]`; `12 Size=10[1..100]`; `13 Trail=10[1..25]`; `T1/14 Rotation=0[-5..5]` |
| Sweep | `T2/10 Direction Change=false` |
| Text | `T3/10 Text="Text"`; `T0/11 Size=8[5..80]`; `T2/12 Anti Alias=false`; `T10/13 Direction=1`; `T2/14 Vertical=false`; `T0/15 Vertical Offset=0[-100..100]`; `16 Horizontal Offset=0[-100..100]` |
| Tube | 固有PARAMなし |

Mediaの旧表記`T2/10`はcatalog転記誤りだった。`CMediaEffect` constructor
`0x140353840`はID10を生成後、`0x14034F590`経由でTYPE8へ設定する。COLOR MAPPINGS側は
共通baseとColorizeを加えたNB=6になる。family 5の実保存specimenはまだないため、ID33 routeは
このconstructor schemaをsynthetic importer fixtureで固定し、Media Pathが空の場合だけno-opとする。

## 現行Syndocal importerとの逆照合

68 IDのうち現行converter coverageは64 ID（うちMAPPINGS 526とCOLOR MAPPINGS 33、COLOR MAPPINGS 41/47/48の空targetは
source no-op）、random-state由来のprecise fail-closedは0、残る4 IDは未routeでgeneric
`Skipped`になる。Media 33/526のnon-empty pathはroute内でprecise fail-closedを維持する。
64 routeはfull-domain完成数ではなく、
strict/exactを別に監査した。

| route群 | 現在の境界 |
|---|---|
| VALUE 621–628 | strict。626/627はstable source seed、628はcontinuous fixed-hash/cosine/analytic palette/active DirectionのCorrected evaluator。対象ゼロのno-opもschema検証後に成立 |
| COLOR MAPPINGS 22/23/30/32/34/36/37/40/42/44 | strict PARAM ID/TYPE/domain、palette 1..255、Rectangle、owned COLOR beam target、Override mergeを保持。外部`SELECTIONS`はprecise fail-closed。23/37/42も共通qGray post-processを通る。37は100x100のflat-cell no-replacement permutationをstable source seedで生成するCorrected route |
| COLOR MAPPINGS 31/49/50 | Lines/Graph/Grid専用classをstrict route。共通base、class固有parameter、owned COLOR target順、Rectangle/Patch、Transform後Rotation、Override、qGrayを保持する。Lines palette 2..255、Graph palette 2..10、Grid palette 2..5をclass域として検証し、固定100x100 rasterを連続解析samplingするSyndocalCorrected実装。GraphはHeight=1のsource invisible defectだけを一行opaqueへ訂正する |
| COLOR MAPPINGS 41 Tube | strict NB=7（T4/1、T2/2、T6/3、T0/4、T0/10 Number 1..10、T1/11 LifeSpan 0..0.9、T0/12 Width 1..90）。shared Sparkle retained stateを`TubeFullRasterHeight`で区別し、fixed 100x100/full-height、qGray、owned COLOR target、Overrideを保持する。paletteはTube固有の2..255。BEAMS=0は全schema後のsource no-opで、native unset Rectangle sentinelはempty targetだけplacement Noneとして許可する |
| COLOR MAPPINGS 47/48 Explosion/Starfield | strict NB=12/NB=9、palette 2..255、TYPE7 Shape 0..29、class固有domain、owned COLOR target、fixed 100x100、Overrideを保持する。populated Shape 0だけを共通filled ellipseとしてrouteし、Shape 1..29はproprietary glyph未回収のためprecise fail-closed。BEAMS=0 + native unset Rectangleは全schema後のsource no-op。非serialize qrand table、source green lifetime defect、Starfield palette endpointは`SyndocalCorrected`で安定seed/alpha/clampへ訂正する。supported 200 target以下はsampled-only RGB全世代table、超過時は同じSoA generation cacheへfallbackし、いずれもtick allocationなし。supported routeのpalette/random/framesはArc共有、dynamic cache不在のためcue transition cloneもparticle backing allocationなし |
| COLOR MAPPINGS 33 | ctorで確定したNB=6（T4/1、T2/2、T6/3、T0/4、T8/10、T1/11）をstrict検証。空Media Pathだけsource no-op、non-empty pathはprecise fail-closed |
| MOVE 221–225 | strict。5 distinct geometry、authored DURATION、continuous Phasing/Symmetry、ordered beam targetを保持。Pointsのみ等時間vertex hold |
| COLOR 129/130 | strict schemaと証明済みevaluator式を実装 |
| COLOR 127 | strict schema、authored DURATIONを保持。合成後qGray、Transform foldと内部40 ms tableはlegacy互換 evaluatorに残る |
| COLOR 121 | strict schema、authored DURATIONを保持。cyclic palette、raw pixel radius、内部40 ms/cache、qGray、Transform foldはlegacy Burst evaluatorに残る |
| COLOR 134 | strict schema、authored DURATIONを保持。hard boundary、Direction Change、qGray、Transform foldと内部40 ms tableはlegacy Sweep evaluatorに残る |
| COLOR 128 | strict schema、固有Perlin evaluator、連続時間、qGray、Transform foldを実装 |
| COLOR 131/133 | strict schema、recovered random grammar、stable source seed、qGray/TransformをCorrected evaluatorで実装 |
| CHASER 321–325 | 323/324はstrict schemaと回収済み対称pair topologyをCorrected連続clockへ変換。321はcompatibility、325の既存stable permutationはCorrectedとして報告 |
| CURVE 3–12 | strict factory schemaと回収式をCorrected continuous evaluatorで実装。6 Randomだけ非serialize qrand履歴をstable source seedへ置換 |
| CURVE 13 | 別schemaのTYPE5 Points 2..255とPhasingをstrict route。raw X/Yと右点easingを保持し、adjacent-target lagを専用sourceで表現。40ms sampled approximationとDURATION remainder lossだけをcontinuous evaluatorで訂正。実保存32-beam specimenあり。正本は`qa/DVC_CUSTOM_CURVE_SOURCE_PARITY.md` |
| MAPPINGS 521 | strict schema、unit→percent、Rectangle placementを実装 |
| MAPPINGS 522/524 | strict schema、回収済みconical ring / opposite-sector geometryを連続解析evaluatorで実装 |
| MAPPINGS 523/525/527/528 | strict schema、100x100 Rectangle-local 2D placement、共通Burst/Plasma/Knight/Sweep corrected evaluatorを実装 |
| MAPPINGS 526 | 実保存の空Media Pathをsource no-opとして保持。non-empty pathは埋め込みmedia decode証明待ちでprecise fail-closed |
| MAPPINGS 529 | strict schema、100x100 retained-particle state、Width/Height、stable source seedのCorrected evaluatorを実装 |
| MAPPINGS 530 | strict schema、Rectangle placement、固有Perlin evaluator、穴のない解析的0..360度inverse rotationを実装 |

この監査により、DVC-ENUM直後の条件付きstrict-coreは6 ID、要correctness routeは16 IDだった。
未route追加より先に後者をstrict化し、再現不能な近似はprecise fail-closedへ戻す。

### DVC-C0a hardening（2026-08-10）

- MAPPINGS 521は`TYPE1/ID10=0..1`を`*100`でruntime percentへ変換し、全PARAM TYPE、
  Rotation/Angle integer `0..360`を検証する。
- COLOR 129/130は全TYPEとfactory domainを検証する。Plasma 10..13=`0..20`、14..17=`-5..5`、
  Rainbow Width/Gradient=`0..1`、Angle integer `0..360`。
- CURVE 3/7/10は共通`T0/ID1 integer 1..10`, `T1/ID2 0..2`, `ID3 0..1`,
  `ID4 -1..1`, `ID5 0..1`へ統一した。
- CHASER 321/322/325はempty BEAMSより前にID/TYPE/domainを検証する。321/325の
  `Nb pixels on=0..1000`をschema上受理し、empty targetならno-op、targetあり0は表現不能理由付き
  fail-closed。322は`T2/ID10` binaryを厳密検証する。
- MOVE 224のPOINTS域を旧`3..256`からfactory `2..255`へ訂正した。

これで129/130、322、CURVE 3/7/10、521の7 routeは評価器／入力coreがstrict化された。
ただしCOLOR paletteはfactory `1..255`に対し現protocol/engine `2..16`、VALUE paletteは`2..32`という
横断表現域が残る。Line 223のPOINTS exact-2制約、CHASER 321/325のclamp/意味論差も既存境界である。
したがって、条件付きexact-coreは13 routeへ増えたが、13/68をfull-domain完成率とは呼ばない。

### DVC-C0b correctness disposition（2026-08-10）

- C0b当時、COLOR 127は`CKnightRiderEffect / 0x140363FE0`へ切替え、factory TYPE/domain、
  importer periodの40ms floor、Transform=1、共通Grayscaleを保持した。GrayscaleはDaslight同様、
  lane source-over完了後の8-bit qGrayとして適用し、`.sdc`はdefault falseの加算フィールドで互換を保つ。
- C0b当時、COLOR 121は`CBurstEffect / 0x140362B70`とgeneric radial Burstが非等価、COLOR 131は
  `CRandomFillEffect / 0x140365C70`、COLOR 133は`CSparklesEffect / 0x1403660F0`の
  per-thread qrand state/historyが非serialize、MAPPINGS 530は`CPerlinEffect / 0x140365090`が
  Rectangleを評価するのにgeneric stage-space noiseが捨てるため、全schema/domain検証後にprecise
  fail-closedとした。
- これらは当時のdispositionである。その後、authored DURATION period保持、COLOR 131/133のstable
  source seed、MAPPINGS 530のcontinuous fixed-hash Perlinとinverse-local rotationを実装し、該当する
  period量子化／RNG／Perlin fail-closed境界はsupersededになった。COLOR 121/127/134のevaluator内部に
  残る40 ms table/cacheはまだcorrectedではない。
- MOVE 223/224とCHASER 321/325は既存showの互換bodyを壊さず、distinct evaluatorまたはrandom順、
  beam identity損失を全populated targetで常時`Approximate`として可視化する。empty Chaser no-opは
  runtime bodyを作らないためApproximateを付けない。
- 分類はruntime-convert 18、precise fail-closed 8、未route 42。18のうち条件付きexact-coreは14、
  明示compatibility routeは4。palette 1..255等の横断表現域が残るため14/68をfull-domain完成率とは呼ばない。

### DVC-C0c COLOR palette full-domain（2026-08-10）

- COLOR / COLOR MAPPINGS / MAPPINGSの`TYPE4/ID1` factory cardinality `1..255`を、protocol、
  `.sdc` reference palette、DVC parser、Tauri command validation、runtime、editor、palette libraryの
  全経路で同じ共有境界へ統一した。256以上は理由付きで拒否する。
- 1色paletteはposition 0のconstant outputとして保持し、255色paletteは順序・16-bit RGB・等間隔positionを
  losslessに保持する。UI position精度を4桁へ上げ、255 stopを相互に区別できるようにした。
- 44Hz側の通常palette lookupを線形走査から`partition_point`へ変更した。COLOR 127 exact Knight Riderも
  0..254 laneを受理し、frame tableをcommand/rebuild時に完成させる。VALUE importerの`2..32`は別factory
  contractなので拡張していない。
- これでC0a/C0b時点の「COLOR palette 1..255対2..16」という横断境界は解消した。runtime-convert 18、
  precise fail-closed 8、未route 42、条件付きexact-core 14、compatibility 4というID分類自体は変わらない。
  他のPARAM、placement、beam identity、stateful RNG境界が残るため、なお14/68をfull-domain完成率とは呼ばない。

### DVC-MOVE recovered source and corrected runtime（2026-08-10）

- MOVE 221–225をそれぞれ`DaslightCircle / Curve / Line / Polygon / Points`へrouteした。
- DVC-MOVE-EXACT当時はbinary global `0x1408DAC38=40 ms`、durationのfloor/min-1 frame、
  4 evaluatorの外側frame補間とPointsのheld outputまでruntimeに保持した。
- Circleの解析arc、Curveのuniform Catmull-Rom 16-slice + arc length triangle traversal、
  Lineのcycle-minus-one quirk、Polygonのequal-edge time、Pointsのheld-index式を個別に実装する。
- 当時のraw Phasingは`cycle * ID2`をselection rankごとにhalf-up整数frameへ落とし、Symmetryは
  reverse orderの2-wing evaluatorとして保持した。BEAMID/selection identityはXML順を失わない。
- runtime-convertは18→20、未routeは42→40、条件付きexact-coreは14→18、明示compatibilityは
  4→2となる。残るcompatibilityはCHASER 321/325だけである。
- その後のcorrected timing trancheが40 ms period短縮、frame-rounded Phasing、Line endpoint lossを
  supersedeした。現行importはauthored DURATION（共通min 10 ms）とcontinuous phaseを使い、
  Circle/Curve/Line/Polygonを連続評価する。Pointsだけは各authored vertexを等時間holdする。
  Syndocal Enhanced `Line / Smooth / Circle`も従来どおり連続時間である。

### DVC-BURST-EXACT（2026-08-10）

- 共有constructor `0x140350BE0`末尾の`0x14035155A`は`+0x12c=true`を無条件writeする。
  C0b/V6時点の「uninitialized palette-wrap」は誤りであり、COLOR 121 / VALUE 622のblockerではない。
- `CBurstEffect / 0x140362B70`をcyclic 16-bit palette cache、raw pixel radius、
  `QImage::Format_RGBA64`、Qt 1024-entry gradient table、`t-1e-5` seam、40 ms scheduler、
  qGray、Transform foldまで専用modeで実装した。44Hz側は完成frame tableの2色参照だけを行い、
  同一immutable tableを共有する。200灯体×64 exact Burstのrelease計測は
  p95 1.967 ms / p99 3.829 ms / max 4.467 msで5/8/12 ms gateを通過した。
- runtime-convertは20→22、precise fail-closedは8→6、条件付きexact-coreは18→20。
  明示compatibilityはCHASER 321/325の2 routeのまま、未routeは40のままである。
- 新規authoringはEnhancedを既定とし、import bodyはDVC recovered coreを保持する。editorの明示切替で
  Enhancedへ移行できる。
- その後の共通duration correctionでrequest periodはauthored DURATION保持へ変わったが、Burstの
  40 ms scheduler、750 cap、16-bit/Qt palette・gradient cache、seamは現行にも残るlegacy artifactである。

### DVC-SWEEP-EXACT（2026-08-10）

- `CSweepEffect / 0x1403665A0`を共有するVALUE 625とCOLOR 134を、signed 40 ms frame grid、
  `F=min(R,750)`生成表とmixed-precision temporal interpolation、palette数倍phase、整数hard
  boundary、次色fill＋現色overlay、transitionごとの180度Direction Changeまで同一evaluatorで実装した。
- 共通Transform 1はQt nearestの半幅forward/reverse copyを使い、奇数幅の末尾と幅1をtransparent
  blackのまま残す。COLORのGrayscaleは合成後にQt整数qGrayを適用する。VALUEにはID2がないためfalse固定。
- VALUE 625の旧`Transform=1` fail-closedを解消し、COLOR 134をstrict route化した。protocolは
  `grayscale` / `vertical_symmetry`をdefault false・false時omitで加算し、legacy `.sdc` shapeを保つ。
- runtime-convertは22→23、未routeは40→39、条件付きexact-coreは20→21。precise fail-closed 6、
  明示compatibility 2は不変。MAPPINGS 528 / COLOR MAPPINGS 44の2D placementは本claimに含めない。
- その後の共通duration correctionで`.dvc` request periodはauthored DURATIONを保持する。
  `daslight_exact=true`のSweep evaluator内部には粗い40 ms grid、750 cap、Qt foldが現行にも残り、
  新規authoringは連続時間Enhancedを既定とする。editorから明示的に相互切替できる。
- 200灯体 x 64 exact Sweepのrelease 44Hz計測はp95 4.023 ms / p99 5.080 ms /
  max 6.112 msで5/8/12 ms gateを通過した。全workspace test、production frontend build、
  localization 3066/3066、Scene Settings 5 viewport、`tauri build --no-bundle`とexact checkoutの
  responsive native windowまで確認した。

### DVC-PERLIN recovered source and corrected runtime（2026-08-10）

- `CPerlinEffect / 0x140365090`からsigned fixed hash、cosine interpolation、octave attenuation、
  Transform、qGray、palette orderを回収した。これらは見た目を構成する演出契約としてVALUE 628 /
  COLOR 128 / MAPPINGS 530の共有evaluatorに保持する。serialized field名`daslight_exact`はlegacy
  `.sdc`互換のため残すが、`true`の製品上の意味はDVC corrected evaluatorである。
- 回収時に証明した40 ms scheduler、750生成frame cap、65,536-entry integer palette cache、
  degree-truncated sine lookup、Qt nearest-neighbour raster rotationは生成・timer・cache実装由来であり、
  runtime契約にはしない。authored `DURATION`を連続phaseとして評価し、fixed hashとcosine weightへ
  解析的sineを直接適用する。paletteは同じcyclic authored orderを解析的に補間し、Transform後の
  palette合成色へqGrayを適用する。
- DVC `Direction`の整数域`1..100`は`degrees = (raw - 1) * 360 / 99`で線形変換する。
  したがって`1 = 0°`、factory default `2 = 3.636...°`、`100 = 360°`であり、defaultは回収元の
  +X向きに近い。spatial phaseはzoom後座標`(x,y)`に対し
  `TAU * (x * cos(theta) + y * sin(theta))`を加える。`Speed`はauthored period内の整数cycle数なので、
  Directionを実際に効かせてもperiod境界は連続する。
- MAPPINGS rotationは100 x 100 Qt imageを作らず、中心`(0.5,0.5)`まわりに連続座標をinverse rotate
  してunbounded Perlin fieldを直接sampleする。これによりnearest-neighbour hole、edge smear、固定小数
  boundaryを除去する。Rectangle placementも同じinverse-local座標を使うため、ANGLE付きmaskとsampling
  frameは一致する。
- Qt5Gui.dllで全比較した3,610,000座標とFNV-1a64 `b8c647db6b017785`は、旧挙動の逆解析が正しかった
  ことを示すrecovery oracleとして保存するが、production regression contractにはしない。
  runtime-convertは23→26、precise fail-closedは6→4、未routeは39→38、条件付きexact-coreは21→24となる。

### DVC-CURVE recovered source and corrected runtime（2026-08-10）

- `CSquareEffect / 0x140370420`のsample count、400-cell整数grid、`floor(400/Rate)` band幅、
  float Phase加算後のtruncation、band parity、Size/Offset後clampを静的解析から復元した。
- CURVE ID9をstrict共通schema、回収元40 ms grid provenance、ordered beam target、Phasing保持で
  既存LFO bodyへrouteした。その後のcorrected-import監査で、DVC Squareも連続時間の等幅bandへ変更し、
  `floor(400/Rate)`の終端residueを除去した。native Syndocal Squareは従来の連続時間を維持する。
- 同じ監査でID3 Inverse Ramp / 7 Sinusの40ms hold、ID10 Strobeの`floor(25/Rate)`周波数誤差を
  除去した。初期回収時に保持したStrobeの40ms最小flash幅はその後supersededになり、現行はexact
  authored `1/Rate` intervalとRate非依存のdimensionless base duty 20%（Phaseで`max(0.2, Phase/2)`）を
  使う。ID4 Pulseは回収carrierを保ち、固定0.005 windowと40ms holdをnormalized continuous windowへ
  訂正してrouteした。
- runtime-convertは26→27、未routeは38→37、条件付きexact-coreは24→25。precise fail-closed 4と
  明示compatibility 2は不変。

### DVC-COLOR-MAPPINGS shared evaluator tranche（2026-08-11）

- COLOR MAPPINGS 22 Burst / 23 Butterfly / 30 Knight Rider / 32 Perlin / 34 Plasma /
  40 Sparkle / 42 Spiral / 44 Sweepを、対応するMAPPINGS 523/524/527/530/525/529/522/528の
  共通recipe/evaluatorへrouteした。familyは`Color Mappings`、beam targetはowned COLOR segment、
  mergeは`Override`であり、MAPPINGS側のDimmer target / `Multiply`へ変換しない。
- 全routeはCOLOR MAPPINGS baseのT4/1 palette、T2/2 Grayscale、T6/3 Transform、T0/4 Rotation、
  class固有ID/TYPE/domain、positive Rectangle、Patch-canvas beam座標をstrict検証する。paletteは
  factory域1/8/255を受理し、0/256を拒否する。外部`SELECTIONS`はsilent fallbackせずfail-closed。
- Spiral / Butterflyへdefault false・false時omitのGrayscale wire fieldを加え、旧`.sdc` JSON shapeを
  保持した。両evaluatorともpalette/geometry合成後に共通qGrayを適用する。
- ID33 Mediaはconstructorで確定したT8/10 Media PathとT1/11 Colorizeを含むNB=6だけを認識する。
  empty pathはRectangleと全base/domainを検証したsource no-op、non-empty pathはembedded-media decode
  証明待ちのprecise fail-closedである。COLOR MAPPINGS family 5の実保存specimenは存在しないため、
  native保存互換を主張せずsynthetic strict-schema regressionだけを追加した。
- ID37 Random fillはstrict NB=6（共通base + T0/10 Point Width 1..10 + T0/11 Point Height
  1..10）でrouteした。Rectangle配置と`source_point_height=Some`が同時にある場合だけ、高さをliveに
  使う100x100の2D evaluatorとなる。X/Yを`floor(normalized*100)`（99へclamp）でpixel化し、
  `div_ceil`した全2D cellへ独立axis shuffleではなく一つのflat rank permutationを割り当てる。
  右端・下端のpartial tailもcoverageへ含む。非serialize qrand/historyはsource familyとgenerator IDを
  含むstable seedへ置換するため`SyndocalCorrected`であり、process-history exactとは主張しない。
  placementなしの旧JSONは、`source_point_height`が存在しても従来どおり1D/evaluator-deadである。
  family 5の実保存specimenは存在しないため、constructor/static evidenceとsynthetic strict importer
  fixtureだけを根拠とする。当時の専用classの残りは21/29/35/41/45/47/48、別schemaは
  CURVE Custom 13である。
- ID31 Lines / ID49 Graph / ID50 Gridはbinary 5.0.6.2のclass bodyから回収した固定100x100
  grammarをallocation-free analytic evaluatorへ移した。全routeとも`implementation=SyndocalCorrected`で、
  40ms work imageではなくauthored DURATIONを10ms共通floor付きcontinuous timeとして保持する。
  Graphはpalette0 background、`max(Pitch,1)` anchor、sinusoidal Y、triangle row weight、later-wins
  clipを保持し、sourceで不可視になる合法Height=1だけをopaque一行へ訂正した。family 5のRainbow
  実保存標本はあるがLines/Graph/Gridのnative保存標本はないため、claimはbinary/static
  evidenceとsynthetic strict importer/runtime regressionに限定する。正本は
  `qa/DVC_COLOR_MAPPINGS_GRID_LINES_GRAPH_PARITY.md`。
- ID41 Tubeは実保存`qa/specimens/ColorMappings-Remaining7.dvc`でNB=7のT4/1、T2/2、T6/3、T0/4、
  T0/10 Number、T1/11 LifeSpan、T0/12 Width、positive Rectangle、empty BEAMSを固定した。`TubeFullRasterHeight`
  は通常Sparkleと同じretained-particle stateを使うが、target countと独立したfixed 100x100 source、full-height
  paint、右端clip、qGray、palette lane `(particle_index % (palette_count - 1)) + 1`を使う。sourceの100-pair
  lookup grammar `q_i=pair[i%100].first; j=trunc(100*q_i*(epoch%floor(period_ms/40)))%100; x=trunc(100*pair[j].second)`は保持し、
  non-serialize qrand historyだけをstable source seed tableへCorrected置換する。BEAMS=0のpositive Rectangleは
  validated source no-op、exact unset sentinel `(0,0,-1,-1,0,0)`はempty targetだけplacement None、non-emptyは
  fail-closedにする。64 effect x 200 fixture、255 palette、Number 10、LifeSpan .9、Width 90のrelease gateは
  5/8/12 msをassertする。
- Tube追加でruntime-convertは61→62、未routeは7→6となる。
- Explosion/Starfield追加でruntime-convertは62→64、未routeは6→4となる。実保存
  `ColorMappings-Remaining7.dvc`のempty sentinel/no-targetをstrict source no-opとして固定し、populated
  Shape 0はretained-particle evaluatorへrouteする。32+32 effect x 200 fixture、255 paletteのrelease gateは
  Arc transition修正後の再走でfrequent F2がp95/p99/max 0.894/1.062/1.977 ms、Trail 25を含むdeep F125が
  1.557/1.807/2.332 msとなり、5/8/12 msを維持する。正本は
  `qa/DVC_COLOR_MAPPINGS_EXPLOSION_STARFIELD_PARITY.md`。

### DVC-CURVE Custom 13（2026-08-11）

- 別schemaの`TYPE5/ID1 Points=2..255`と`TYPE1/ID2 Phasing=0..1`、raw Y decadeに埋め込まれた
  5種のright-point cubic easing、source-order interval、last-point fallback、adjacent-target lagを
  pinned binaryから回収した。
- 実保存`qa/specimens/CurveCatalog-Custom.dvc`のdefault 2点とordered 32-beam Dimmer targetをstrict
  importer regressionへ固定した。duplicate container、unknown attribute、external/missing selection、
  invalid point/easing/duration、同一fixture内のdropped beamもfail-closedにする。
- runtimeはraw point orderを保存したままcommand/rebuild時にdecodeし、`partition_point`でallocation-free
  lookupする。40ms sampled approximationとDURATION remainder lossだけをcontinuous exact-easingへ訂正した。
  専用255-point / 64-effect / 200-fixture release gateは5/8/12ms閾値を維持する。正本は
  `qa/DVC_CUSTOM_CURVE_SOURCE_PARITY.md`。現行routeは60→61、未routeは8→7、条件付きexact-coreは
  25→26となる。

## 未実装・未証明境界

カタログ列挙は完了したが、次は別軸で残る。

1. CURVE 3–13は全route済み。Custom 13は別schema、raw point/easing、adjacent-target Phasing、
   実保存specimenを`qa/DVC_CUSTOM_CURVE_SOURCE_PARITY.md`で固定した。
2. CHASER #3/#4は式と対称pair topologyを回収し、実保存specimen付きでroute済み。実灯体の
   photometryとdevice latencyだけは物理受入に残る。
3. MAPPINGS 521–530は全route済み。COLOR MAPPINGSの共有class
   22/23/30/31/32/34/36/37/40/41/42/44/47/48/49/50もroute済み。Media 33/526は空sourceだけno-op、non-empty sourceは
   decode/timing証明待ち。COLOR MAPPINGS専用class 21/29/35/45は
   引き続きraster algorithm bodyまたは2D stateの意味論回収が必要。
4. `TYPE7 Shape`のserialized glyph表現と`TYPE10 Text Direction`の合法enum域は未証明。
5. Sparkle/Random fillの外部per-thread qrand履歴そのものはreplay不能だが、作者が保存した
   schemaと演出grammarをstable source seedでCorrected実装済み。正本は
   `qa/DVC_RANDOM_CORRECTED_PARITY.md`。
6. factoryにないID空隙が予約か廃止かは製品履歴の問題で、現バイナリからは断言しない。

この境界より内側だけを次トランシェへ渡し、UI名の類似やSyndocal既存recipeへの近似で埋めない。
