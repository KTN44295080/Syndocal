# DVC COLOR MAPPINGS Rain parity（ID 35）

- 対象: Daslight 5.0.6.2 `Daslight 5.exe`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- factory constructor: `0x1403546C0`
- evaluator: `0x140365660`
- class vtable: `0x140696A70`
- 実保存標本: `qa/specimens/ColorMappings-Remaining7.dvc`

## Strict schema

COLOR MAPPINGS family 5 / generator ID 35はNB=9を要求する。converterは共通baseとRain固有PARAMのexact ID / TYPE集合を順序非依存で検証する。実保存標本で観測した次のPARAM順はgolden testが別途固定する。

| TYPE / ID | 意味 | 合法域 | factory default |
|---|---|---:|---:|
| T4 / 1 | Palette | 1..255 colors | specimen依存 |
| T2 / 2 | Grayscale | 0 / 1 | 0 |
| T6 / 3 | Transform | 0 / 1 / 2 | 0 |
| T0 / 4 | Rotation | integral 0..360 | 0 |
| T0 / 13 | Number | 1..100 | 50 |
| T0 / 10 | Speed | 0..10 | 1 |
| T0 / 11 | Width | 5..10 | 5 |
| T0 / 12 | Height | 10..30 | 10 |
| T0 / 14 | Trail | 1..30 | 10 |

実保存標本はnative empty Rectangle sentinel `(X,Y,SX,SY,ANGLE)=(0,0,-1,-1,0)`と`BEAMS=0`を持つ。この組合せはstrict validation後のsource no-opとして保持する。target付きで負extentをsilent fallbackしない。positive Rectangleはfixed 100x100 sourceを`Override`でowned COLOR beamsへsampleする。

## Recovered source grammarとCorrected clock

sourceのprocess-global qrand履歴はserializeされないため、その履歴自体は再生不能である。製品runtimeではsource identity由来のstable seedから既存qrand互換pair tableを生成し、先頭100 pairを使う。各particleの初期値は次である。

```text
x0 = floor(100 * qx)
y0 = floor(100 * qy)
palette_word = low_u16(1 + trunc(65535 * qy))
fall = 1 + qx
source per-frame rate = (100 / F) * Speed
```

40 ms timerとduration frame remainderは保存演出の意味ではないためruntime contractにしない。`P=max(authored_duration,10 ms)`、`phi`を連続period phaseとして、明示的なf32積順とfloorを保つ。

```text
y(phi) = floor(y0 + (100 * Speed) * fall * phi) mod 100
```

## Painter contract

- particleはsource orderで描画し、後のparticleが先のpixelを置換する。
- Xは`x0..x0+Width`を右端でclipし、horizontal wrapしない。
- Yはheadから下向きに`Height+Trail`行を描き、100行でwrapする。
- `row < Height`はweight 1。trailは`1 - (row - Height + 1) / Trail`で、終端のweight 0 pixelも置換する。
- 各particle colorはcompiled paletteから一度だけmixする。pixelはpalette0 backgroundからその色へRGB weight補間し、全paint後にqGrayを一度適用する。
- Transform / Rotation / Rectangle samplingは既存の共有placement契約を使う。

hot pathはcompile時の100列particle coverage bit maskと、同一phaseの100 current-Y固定長cacheを使う。tick中のallocationはない。maskは昇順particle indexを保持するためlater-wins順序を変えない。全100x100 rasterを複数phaseでsource-order reference painterと比較するdifferential testが最適化前後の挙動同一性を固定する。

## Tests and release gate

- protocol: legacy default omissionとvalidated maximaのserde round-trip。
- engine: full-domain 100x100 differential、continuous phase、qGray、全PARAM境界。
- importer: strict NB/type/id/domain/duplicate/unknown/placement mutation rejection。
- golden: `ColorMappings-Remaining7.dvc`の9 PARAM順、empty Rectangle sentinel、BEAMS=0、source no-op。
- release: 64 simultaneous effects x 200 fixtures、44 Hz、palette 255、Number 100、Speed 10、Width 10、Height 30、Trail 30。固定閾値p95/p99/max=`5/8/12 ms`。

最適化前の診断走は`p95=8161 us / p99=8797 us / max=9534 us`でp95のみ不合格。上記allocation-free mask/cache後の実装最終走は`p95=4046 us / p99=4397 us / max=4797 us`、独立reviewer再走は`p95=3989 us / p99=4265 us / max=4531 us`、root最終再走は`p95=3912 us / p99=4231 us / max=4805 us`で、すべて合格した。
