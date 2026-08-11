# COLOR MAPPINGS Text ID45 — SourceExact no-op境界

## 結論

Daslight `RACK TYPE=5 / EFFECT TYPE=3 / ID=45` Textは、実保存で確認できた
`BEAMS NB=0`だけをstrictな`SourceExactBoundary` routeとして受理する。これはruntime effectを
生成しないsource no-opであり、文字列が空かどうかではなくowned BEAMSが0件かどうかを境界にする。

owned BEAMSが1件以上あるTextは、`populated Text requires unresolved deterministic font and
supported text-raster envelope`としてprecise fail-closedにする。Syndocalの既存フォントへ近似せず、
次の2裁定が揃うまでruntime/model/UIと44 Hz gateは追加しない。

1. Daslightと決定論的に一致させるfont/fallback規約。
2. support対象とするText長・glyph raster寸法の最大値。

## 実保存golden

- specimen: `qa/specimens/ColorMappings-Remaining7.dvc`
- SHA-256: `80CB936E0AE81BF2A809B49F381BCB6771373D95F91BE9DFA4D608FE93A936F9`
- `RACK EXPAND_RACK=1 TYPE=5`
- `EFFECT TYPE=3 ID=45 DURATION=5000`
- `PARAMS NB=11`
- `MAPPING Rectangle=(X=0,Y=0,SX=-1,SY=-1,ANGLE=0,LOCKED=0)`
- `BEAMS NB=0`、外部`SELECTIONS`なし

実保存PARAMは次の通りで、ラベルはfactory表示に合わせて訂正した。

| TYPE / ID | label | 保存値 / domain |
|---|---|---|
| `T4/1` | palette | 2色 / `2..255` |
| `T2/2` | Grayscale | `0` / `0..1` |
| `T6/3` | Transform | `0` / `0..2` |
| `T0/4` | Rotation | `0` / integer `0..360` |
| `T3/10` | Text | `Text` / 長さ上限は未裁定、空文字を含め保持 |
| `T0/11` | Size | `8` / integer `5..80` |
| `T2/12` | Anti Alias | `0` / `0..1` |
| `T10/13` | Direction | `1` / recovered integer `0..8` |
| `T2/14` | Write Vertically | `0` / `0..1` |
| `T0/15` | Vertical Offset | `0` / integer `-100..100` |
| `T0/16` | Horizontal Offset | `0` / integer `-100..100` |

旧表記の`Vertical`、`Vertical Offset`内の空白揺れは、factory labelの
`Write Vertically`、`Vertical Offset`、`Horizontal Offset`へ統一した。

## importer境界

- preclassification/report上は`Text`として識別し、validated empty sourceをConvertedへ集計する。
- RACK/EFFECT/PARAMS/PARAM/COLORS/COLOR/MAPPING/BEAMSの属性と直下containerをexact検証する。
- PARAM ID/TYPEは順序非依存でexact setを要求し、重複・不足・未知IDを拒否する。
- `TYPE3/ID10 Text`は専用文字列parserで`VAL`を読み、数値helperへ通さない。空文字と4 KiBを超える
  Unicode文字列を受理し、未裁定の長さcapをinventしない。
- palette、boolean、Transform、Rotation、Size、Direction、offset、positive integer DURATIONを
  全て検証した後にだけno-opを成立させる。
- positive Rectangle＋BEAMS=0と、native unset sentinel＋BEAMS=0を受理する。
- native unset sentinel＋owned BEAMSは配置不能としてfail-closed。positive Rectangle＋owned BEAMSは
  font/text-raster envelope未裁定のprecise fail-closed。
- 外部`SELECTIONS`、duplicate/missing RectangleまたはBEAMS、NB/body不一致、未知属性を拒否する。

## test / gate方針

`dvc_color_mappings_text_source_noop_is_strict_and_populated_text_fails_closed`はNB、ID/TYPE、palette
1/256、Size 4/81、Direction -1/9、offset -101/101、boolean、Transform、Rotation、nonfinite、
Rectangle、BEAMS、SELECTIONS、strict属性、DURATION、sentinel＋beam、populated fail-closedを固定する。
sentinel＋BEAMS=0でもdomain不正は拒否し、長いUnicode Textには暗黙capを設けない。

`dvc_local_golden_color_mappings_text_from_remaining7_specimen`は上記実保存schema、sentinel、BEAMS=0、
SourceExactBoundary report分類、runtime ID非採番を固定する。

このrouteはruntime evaluatorを生成せず、validated maximum Text長/raster envelopeも未裁定であるため、
44 Hz release gateを置かない。gateを捏造せず、上記2製品裁定後のpopulated trancheで最大行を追加する。
