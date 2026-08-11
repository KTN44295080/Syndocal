# DVC COLOR MAPPINGS Bounce parity（ID 21）

## 保存schemaとroute境界

- familyは`RACK TYPE=5 / EFFECT TYPE=3 / ID=21`。正本
  `qa/specimens/ColorMappings-Remaining7.dvc`は88,780 bytes、SHA-256
  `80CB936E0AE81BF2A809B49F381BCB6771373D95F91BE9DFA4D608FE93A936F9`。
- `PARAMS NB=12`を順序非依存でexact検証する。共通baseは`T4/1 Palette 1..255`、
  `T2/2 Grayscale`、`T6/3 Transform 0..2`、`T0/4 integral Rotation 0..360`。
  固有値は`T6/10 Item（0=Shape, 1=Points）`、`T7/11 Shape 0..28`、
  `T0/12 Number 1..20`、`T0/13 Size 1..100`、`T0/14 Speed 0..10`、
  `T2/16 Collide`、`T2/17 Fill`、`T0/18 Points 2..10`。ID15は存在しない。
  inactive fieldも含め全fieldを検証してからtarget/no-opを分類する。
- RACK/EFFECT/PARAMS/PARAM/COLORS/COLOR/MAPPING/BEAMS/BEAMのcontainer数とattribute集合、
  `BEAMS@NB`、外部`SELECTIONS`不在をstrictに検証する。real defaultはItem=Shape、Shape=0、
  Number=6、Size=40、Speed=1、Collide=0、Fill=0、Points=3、native empty Rectangle、BEAMS=0。
  これはvalidated source no-opでruntime effect IDを割り当てない。
- populated routeはpalette 2..255、positive Rectangle、全owned BEAMSのconcrete Patch座標と
  verified color segment、Overrideを必須にする。Item=PointsはPoints 2..10とFill両値をrouteし、
  inactive Shape 0..28を許す。Item=ShapeはShape 0とCollide両値だけをrouteする。
  active Shape 1..28は`unsupported XEEL glyph geometry`としてprecise fail-closed。
  unresolved/external target、native empty Rectangle付きowned targetもfail-closed。

## SyndocalCorrected evaluator

Daslightのprocess-global qrand履歴はDVCに保存されないためSourceExactではない。既存の
`dvc_corrected_rng_seed`でscene/rack/effect identityからseedを作り、Fireと同じ
`word(k)=splitmix64(seed ^ k*0x9E3779B97F4A7C15)`、`q15=word>>49`、
`A[i]=q15(2i)/32767`を用いる。odd/B laneは生成して破棄する。

- Points item `i`のbaseは`2+10i`、vertex `j`のkeyは`(base+10j)%5000`。
  `x=1+99A[k-2]`、`y=1+99A[k-1]`、`vx=1+99A[k]`、`vy=1+99A[k+1]`。
  pathはclosed。Fill=1はeven-odd fill、Fill=0はSize幅のclosed stroke。Collideはinactive。
- Shape 0はbase `k=2+10i`、同じx/yと`vx=2A[k]-1`,`vy=2A[k+1]-1`、scale=Size/100、
  collision radius=Size。XEEL U+E900のouter/inner TrueType quadratic contoursを、連続off-curve間の
  implicit midpointとeven-odd holeを保持して`X=x+Size*(u-514.5)/400`、
  `Y=y+Size*(448-v)/400`へ写像する。
- generation 0を含む全generationはupdate-first。`x+=vx*Speed`,`y+=vy*Speed`後、
  low/high境界で進行方向だけ反転しclampしない。high側はsourceのW/H swap defect
  （x対H-Size、y対W-Size）を保持する。Pointsは境界Size=0。
- Shape+Collideはitem順i、j>iでapproach pairへ対称impulseを適用する。`dist2==0`は
  非finite化を防ぐCorrected skip。paintはsource order later-wins、background=palette0、
  foreground=`1+i%(N-1)`、qGrayは完成rasterの最後。

固定100x100 rasterのcoverageは決定論的pixel-centre/even-odd corrected rasterであり、Qtの
antialias edgeとframe-equivalentなSourceExact claimはしない。placementのTransform後Rotation、
Rectangle/Patch samplingは既存COLOR MAPPINGS共通経路を使う。

## runtime storage / gates

`F=min(max(1,floor(DURATION/40)),750)`。resolved targetが200以下ならsampled pixelだけの
RGB SoA全F frameをcompile時に作り、palette/random/frame backingをArc共有する。cue transitionの
actual clone machineryでpointer/strong-count不変を固定する。201以上はpreallocated dynamic SoA
cacheへpermissive fallbackし、同一generationをtargetごとに再buildせず、tick allocationもしない。

focused testsはstable q15 lane、mod5000、contour bbox/hole/centering、Points fill/outline/closed、
generation0 update-first、overshoot/no-clamp/non-square W/H swap、collision approach/separate/order/
dist2-zero、later-wins/qGray、F=1/125/750、全branch reference differential、>200 fallback parity、
actual cue transition Arc共有、strict synthetic mutations、Remaining7 raw goldenを固定する。

release gateはproduction `EngineRuntime` apply pathで64 effects x 200 fixtures x 44 Hz、
DURATION=30000/F750、palette=255、Number=20、Size=100、Speed=10、Points=10、
Rotation=360、Transform=2、Grayscale=1。32 Shape0+Collideと32 Points（Fill half split）を使い、
Points側inactive Shape=28をassertする。active Shape=28は別testでfail-closed。compile時間はtick指標と
分離し、release 1000/debug 20 samples、p95/p99/max 5/8/12 msを変更しない。

## 残る外部境界

`ColorMappings-Remaining7.dvc`はnative serializer/no-target defaultの正本だが、対象ありBounceを
Daslight GUIで保存したnative specimenは未取得。strict populated synthetic fixtureとrecovered evaluatorで
routeするが、Qt antialias edge、実機描画pixel差、Shape 1..28 glyphは未証明のまま明示する。
COLOR MAPPINGS Text ID45のpopulated font/fallback/text-raster envelope境界も引き続きfail-closedであり、
68/68 route coverageはこれらvariant boundaryの解消を意味しない。
