# Daslight MOVE 221–225 exact parity (DVC-MOVE-EXACT)

- 対象: Daslight 5.0.6.2 / FileVersion `25.0905.165.111`
- binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- 実保存標本: `qa/specimens/DVC-MOVE-EXACT-25.0905.165.111.dvc`
- 標本SHA-256: `69C52DB07DB2A883632FE34444D402066FA116218E13F56A5DDFA3441E53F4E8`

## 結論

MOVE factoryの5 IDは、共通schemaだけでなく個別evaluator、40 ms frame契約、外側の
frame補間、Phasing、Symmetry、ordered beam/selection identityまで回収した。importerは
`DaslightCircle / Curve / Line / Polygon / Points`という専用modeへrouteし、既存の
Syndocal `Line / Smooth / Circle`とは混ぜない。これにより221–225は全PARAM domainで
runtime-convertされ、旧223/224 Approximateと222/225未route、221 nonzero-Phasing
fail-closedは解消する。

## factoryと共通schema

| ID | UI名 | evaluator |
|---:|---|---:|
| 221 | Circle | `0x140349650` |
| 222 | Curve | `0x14034A0E0` |
| 223 | Line | `0x14034A460` |
| 224 | Polygon | `0x14034A8F0` |
| 225 | Points | `0x14034A790` |

共通position constructor `0x140347890`は次の3 propertyだけを登録する。

- `TYPE5/ID1 POINTS`: normalized X/Y、NB `2..255`。Line evaluatorだけは実保存/UI契約上exactly 2。
- `TYPE1/ID2 Phasing`: `0..1`、default 0。
- `TYPE2/ID3 Symmetry`: binary、default false。

scene `ATTRIBUTEVALUE_MODE=0/1`はそれぞれAbsolute/Relativeとして保持する。標本は汎用18ch
moving-head profileを使い、profile channel type 1がPan/X、type 2がTilt/Yであること、および
sceneのID221 bodyが`DURATION=5000`で保存されることを実機経路で固定した。

## 時間契約

Daslightのframe quantumはbinary global `0x1408DAC38 = 40 ms`。duration setterは
`max(1, floor(DURATION / 40))` framesを作る。MOVE evaluatorはこの整数frameを入力にし、
Circle/Curve/Line/Polygonの外側wrapperは隣接整数frameの出力を実時刻で線形補間する。
Pointsだけはheld outputで、外側補間を行わない。

Syndocalはこの40 ms量子化を`.dvc`由来の5専用modeにだけ適用する。新規authoringの既定
`Line / Smooth / Circle`は連続・高分解能のEnhanced evaluatorを維持する。ユーザーがimport済み
bodyをEnhanced modeへ切り替えれば、その時点からDaslightの時間量子化・quirkを外せる。
互換性のための欠陥をSyndocal全体の既定値にはしない。

## evaluator式

### 221 Circle

既存DVC-V5bで回収した解析的circumcircle evaluatorを整数frameで評価する。2点は一周円、
3点以上は隣接tripleのsigned radius、同符号arc、変曲時half-arc reflection、degenerate linear
fallbackを用い、各authored edgeへ等時間を与える。Daslight専用modeは旧Enhanced `Circle`と
geometry coreを共有するが、時間入力だけは40 ms frame契約に従う。

### 222 Curve

各区間をuniform Catmull-Rom（tension 0.5、端点duplicate）で16分割し、そのQPainterPath相当
polylineを累積arc lengthでsampleする。時間は先頭から末尾、次に末尾から先頭へ戻るtriangle
waveである。Syndocal Enhanced `Smooth`のcentripetal Catmull-Rom/32分割とは別modeである。

### 223 Line

exactly 2点を使う。一般形はauthored point列にreverse interiorを連結し、
`remainder = frame % max(cycle - 1, 1)`、`q = point_count * remainder / cycle`でcyclic edgeを
線形補間する。このためcycle末端にDaslight固有のstart duplicate/endpoint quirkがある。

### 224 Polygon

authored point列をclosed cyclic edgeとして、`q = point_count * (frame % cycle) / cycle`で
sampleする。各辺は物理長ではなく同じframe数を受け取る。

### 225 Points

`index = floor(point_count * (frame % cycle) / cycle)`の頂点をholdする。次frameへの補間はない。

全点同値の合法bodyは5 modeともconstant pathとして保持し、旧Enhanced pathの
「distinct point必須」validationは変更しない。

## Phasing / Symmetry / target identity

`step = cycle * raw_Phase_ID2`。selection identityはXML `BEAMID`と`IDSELECTION`のorderを保持し、
first-seen selection rankごとに`round_half_up(wrap(base - rank * step))`した整数frameを選ぶ。
同じselection IDは同じrankと位相を共有する。

Symmetryはselection数2以上で2 wingに分ける。前半はascending rank、後半はtarget orderを反転し、
Circle/Curve/Line/Polygonは`ceil(cycle/2) - frame`、Pointsは`cycle - frame`をreverse wingのbaseに
する。これは旧Syndocal Line/SmoothのPan mirrorではなく、Daslight evaluatorのreverse-time契約である。

## protocol / persistence boundary

5専用variantは`MoveInterpolation`へadditiveに追加した。既存`Line / Smooth / Circle`と
`beam_targets=[]`のJSON shapeは変えず、旧`.sdc`を同じ意味でdeserialize/serializeする。
imported bodyはordered `{fixture_id, beam_index, selection_index}`を保存し、復元時に全beamを
atomicにbindする。profile不一致ならidentityを壊さずdormantにし、一部beamだけへ縮退しない。

## assertion delta

| 契約 | 旧 | 新 | 理由 |
|---|---|---|---|
| ID221 nonzero Phasing | dedicated `Skipped` | raw `0..1`をinteger-frame fan-outへ保持 | scalar compositionとhalf-up frame契約を回収した |
| ID222 / 225 | unknown Move `Skipped` | distinct exact Curve / Points body | evaluator意味論を静的回収した |
| ID223 / 224 | legacy Line近似、beam identity lossを`Approximate` | dedicated exact evaluator + ordered beam targets | evaluatorとfan-outの非等価境界を解消した |
| unknown Move regression | ID222 mutation | ID226 mutation | 222が合法routeになったためfactory外IDへ移した |
| authoring mode | Line/Smooth/Circle | Enhanced 3種を維持しDaslight exact 5種を追加 | 互換quirkを新規authoring既定にしない |
| viewport interpolation control | visible button count 3 | visible button count 8 | Enhanced 3種を削らずexact 5種を一対一で選べるようにした。overflow/containment/reachability assertionは変更しない |

## 受入ゲート

- protocol full: `49/49` pass。legacy bytesと5 exact variantのserde名を固定。
- engine full: `511` pass / `2` manual benchmark ignore。MOVE path、整数frame、raw Phasing、
  two-wing Symmetry、量子化periodを含む。
- Tauri/backend full: `400` pass / `9` hardware/long-running ignore。full `dvc`は`73/73` pass。
- frontend: TypeScript `--noEmit`、production build、MOVE helper、effect visualization、
  localization `3044/3044`、Scene FX / FX visual / control-modeの5 viewportをpass。
- full viewport matrix: exit `0`。interpolation button `3 -> 8`以外のcontainment/reachability契約は維持。
- exact MOVE venue regression: 200 fixtures x 64 effects x 10 framesをreleaseで`14.529 ms`
  （上限5秒）。target frame offset、tick time、同一frame path/transformをprecompute/cacheし、44 Hz側に
  beam探索・float modulo・重複circle sampleを残さない。
- mixed 64-effect x 200-fixture release A/B: Enhanced Circle `p95 5.515 ms`、exact Circle
  `p95 5.049..5.307 ms`。exact化は同一ホストで非増加だが、測定時のsystem CPUが`33..37%`で
  固定absolute gate `p95 <= 5 ms`を最大0.307 ms超えたためgreenとは記録しない。gateは緩和せず、
  controlled-host rerun境界として残す（p99/maxは8/12 ms内、1回の外乱maxを除く）。
- native release: `pnpm --dir app tauri build --no-bundle` pass。ビルド直前に絶対パスが
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`と一致するprocessだけを停止し、
  再生成exeを明示パスで起動。返却app IDも同じexe、title `Syndocal`のwindowはexactly 1、
  screenshotとaccessibility treeの取得に成功しresponsiveであることを確認した。
