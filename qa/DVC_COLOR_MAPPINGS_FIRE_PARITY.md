# DVC COLOR MAPPINGS Fire parity（ID 29）

- 対象: Daslight 5.0.6.2 `Daslight 5.exe`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- factory constructor: `0x1403520C0`
- evaluator: `0x140363720`
- class vtable: `0x140696020`
- 実保存標本: `qa/specimens/ColorMappings-Remaining7.dvc`
- 標本SHA-256: `80CB936E0AE81BF2A809B49F381BCB6771373D95F91BE9DFA4D608FE93A936F9`

## Strict schema

COLOR MAPPINGS family 5 / generator ID 29はNB=8を要求する。converterは共通baseとFire固有PARAMのexact ID / TYPE集合を順序非依存で検証する。実保存標本のPARAM順、default、source identity seedはgolden testで固定する。

| TYPE / ID | 意味 | 合法域 | factory default |
|---|---|---:|---:|
| T4 / 1 | Palette | 2..4 colors | specimen依存 |
| T2 / 2 | Grayscale | 0 / 1 | 0 |
| T6 / 3 | Transform | 0 / 1 / 2 | 0 |
| T0 / 4 | Rotation | integral 0..360 | 0 |
| T0 / 10 | Flames | 1..100 | 20 |
| T0 / 11 | Width | 10..200 | 20 |
| T0 / 12 | Height | 1..100 | 50 |
| T0 / 13 | Hotspot | 10..255 | 250 |

実保存標本はnative empty Rectangle sentinel `(X,Y,SX,SY,ANGLE)=(0,0,-1,-1,0)`と`BEAMS=0`を持つ。この組合せは全schema検証後のsource no-opとして保持する。target付きで負extentをsilent fallbackせず、他family / rack / effectからID29へ流入させない。positive Rectangleはfixed 100x100 sourceを`Override`でowned COLOR beamsへsampleする。

## Stable corrected random table

sourceのprocess-global qrand履歴はserializeされず、source LUTにも欠陥があるため、その状態自体は再生しない。`implementation=SyndocalCorrected`はsource identity由来のstable seedと次の5000組のq15 laneを使う。

```text
word(k) = splitmix64(u64(seed) ^ (k * 0x9E3779B97F4A7C15))
q15(k) = u16(word(k) >> 49)
A[i] = q15(2*i) / 32767f32
B[i] = q15(2*i+1) / 32767f32  // source generation orderのため計算し、Fireでは破棄
```

Remaining7標本のidentityは
`scene=7d4d8d19-22eb-4c11-be48-607476b9379d|New Scene;rack=1|5;effect=0|3|29`、seedは`0xDCE515F0`である。even A lane先頭6値は`[16650,26834,3970,3050,6255,15119]`としてalgorithm goldenへ固定する。

## Recovered 100x100 heat raster

各generation `g`で102行分のflattened padded image `I[10200]`を更新する。まずrow 1をclearし、Xを0から昇順に走査する。

```text
a = A[(x + 100*g) % 5000]
if 100*a < Flames:
  run = trunc((Width - 1)*a + 1), remaining widthへcap
  I[row1, x..x+run] = Hotspot
  x += run
else:
  x += 1
```

続いてX-major、Y=1..100 ascendingの順で、同じflattened bufferへin-place diffusionする。

```text
I[y+1,x] = floor((I[y-1,x] + I[y,x-2] + I[y,x-1] + I[y,x]
                  + I[y,x+1] + I[y,x+2] + I[y+1,x]) / 7)
```

X端も二次元clampせずcontiguous addressingのspillを保持する。出力Yは反転し、authored pixel `(x,y)`から`I[101-y,x]`を読む。テストは注入run、edge-spill、in-place順、Y flipを独立referenceと比較する。

## Corrected palette / Height policy

source LUTは利用しない。authored stop位置をclampし、非wrap RGB補間した256-entry LUTをcompileする。`cutoff=100-Height`とし、`heat < cutoff`はpalette 0、それ以外は` t=(heat-cutoff)/(255-cutoff)`で補間する。したがってsourceでevaluator-deadだったHeightはlive heat cutoff / rescaleとなる。LUT 255は必ずlast stopで、Grayscaleはlookup後にqGrayを一度だけ適用する。f32積、trunc、整数除算の順序はreference testで固定する。

## Runtime storage policy

source frame数は`F=min(max(1,floor(period_ms/40)),750)`。supported target数200以下ではcompile時に全F heat frameをsampled-only tableへ展開し、random q15、palette LUT、frame tableを`Arc`共有する。tick時のallocation / raster rebuildは0で、cue transition cloneも同じbacking pointerを共有する。200超は拒否せず、preallocated 10200-byte rasterとsample bufferを持つdynamic fallbackで同じ生成順を評価する。

## Tests and release gate

- protocol: legacy default omissionとFire maximaのserde round-trip。
- engine: q15 even lane、注入、edge-spill diffusion、Y flip、generation order、LUT endpoints / Height / qGray、hostile validator、supported全frameとdynamic fallbackのbit parity。
- transition: cue transition clone前後でq15 / LUT / precomputed frameのpointer一致と`Arc::strong_count`増加、deep generation後も再確保なし。
- importer: strict NB / type / id / domain / duplicate / unknown / placement / family mutation rejectionと、empty sentinel source no-op。
- golden: `ColorMappings-Remaining7.dvc`の8 PARAM順、default、empty Rectangle sentinel、BEAMS=0、seed `0xDCE515F0`。
- release: 64 simultaneous effects x 200 resolved fixtures、44 Hz、palette 4、Flames 100、Width 200、Height 100、Hotspot 255、period 5000 / F125、Grayscale 1、Transform 2、Rotation 360。固定閾値p95/p99/max=`5/8/12 ms`。

実装最終release走は`compile=604 ms / p95=1034 us / p99=1774 us / max=2191 us`、root最終再走は`compile=613 ms / p95=1281 us / p99=1806 us / max=2152 us`で、ともに合格した。gateは200 distinct authored coordinatesと200 runtime spatial targets、Transform / Rotation後のsampled pixel数が1..200、全125 generation到達、Arc pointer不変、dynamic cache不在、runtime rebuild 0をassertする。
