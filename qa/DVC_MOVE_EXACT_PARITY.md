# Daslight MOVE 221–225 exact parity (DVC-MOVE-EXACT)

- 対象: Daslight 5.0.6.2 / FileVersion `25.0905.165.111`
- binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- 実保存標本: `qa/specimens/DVC-MOVE-EXACT-25.0905.165.111.dvc`
- 標本SHA-256: `69C52DB07DB2A883632FE34444D402066FA116218E13F56A5DDFA3441E53F4E8`

## 結論

MOVE factoryの5 IDは、共通schemaだけでなく個別evaluator、40 ms生成契約、外側の
frame補間、Phasing、Symmetry、ordered beam/selection identityまで回収した。importerは
`DaslightCircle / Curve / Line / Polygon / Points`という専用modeへrouteし、既存の
Syndocal `Line / Smooth / Circle`とは混ぜない。これにより221–225は全PARAM domainで
runtime-convertされ、旧223/224 Approximateと222/225未route、221 nonzero-Phasing
fail-closedは解消する。回収した40 ms timer/tableは互換性の証跡として保持するが、Syndocal
runtimeはauthored `DURATION`と連続位相を使い、見た目を保ったまま量子化欠陥を再現しない。

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

Daslightのframe quantumはbinary global `0x1408DAC38 = 40 ms`。元実装のduration setterは
`max(1, floor(DURATION / 40))` framesを作る。MOVE evaluatorはこの整数frameを入力にし、
Circle/Curve/Line/Polygonの外側wrapperは隣接整数frameの出力を実時刻で線形補間する。
Pointsだけはheld outputで、外側補間を行わない。

Syndocalは`.dvc`に保存された正の`DURATION`をそのまま取り込み、共通runtime下限の10 msだけを
適用する。Circle/Curve/Line/Polygonは連続位相で評価し、Pointsだけはデザイン上のstep動作として
authored頂点を等時間holdする。40 ms由来のframe countはimport reportのprovenanceに残すが、
period短縮・階段状hold・cycle-minus-one endpoint quirkには使わない。

## evaluator式

### 221 Circle

既存DVC-V5bで回収した解析的circumcircle evaluatorを連続位相で評価する。2点は一周円、
3点以上は隣接tripleのsigned radius、同符号arc、変曲時half-arc reflection、degenerate linear
fallbackを用い、各authored edgeへ等時間を与える。Daslight専用modeは旧Enhanced `Circle`と
geometry coreを共有し、authored periodの連続時間を使う。

### 222 Curve

各区間をuniform Catmull-Rom（tension 0.5、端点duplicate）で16分割し、そのQPainterPath相当
polylineを累積arc lengthでsampleする。時間は先頭から末尾、次に末尾から先頭へ戻るtriangle
waveである。Syndocal Enhanced `Smooth`のcentripetal Catmull-Rom/32分割とは別modeである。

### 223 Line

exactly 2点を使う。回収式の`cycle-minus-one` endpoint quirkは再現せず、往路と復路の2辺を
連続位相で等時間線形補間する。両端への到達とauthored period終端での閉路を保証する。

### 224 Polygon

authored point列をclosed cyclic edgeとして、`q = point_count * phase`でsampleする。
各辺は物理長ではなく同じ時間を受け取る。

### 225 Points

`index = floor(point_count * phase)`の頂点をholdする。これはtimer量子化ではなくPointsの意図した
step表現なので、連続runtimeでも補間しない。

全点同値の合法bodyは5 modeともconstant pathとして保持し、旧Enhanced pathの
「distinct point必須」validationは変更しない。

## Phasing / Symmetry / target identity

`step = raw_Phase_ID2`。selection identityはXML `BEAMID`と`IDSELECTION`のorderを保持し、
first-seen selection rankごとに`wrap(base - rank * step)`した連続位相を選ぶ。
同じselection IDは同じrankと位相を共有する。

Symmetryはselection数2以上で2 wingに分ける。前半はascending rank、後半はtarget orderを反転し、
Circle/Curve/Line/Polygonは`wrap(0.5 - phase)`、Pointsは`wrap(-phase)`をreverse wingのbaseに
する。これは旧Syndocal Line/SmoothのPan mirrorではなく、回収したreverse-time構造を連続化した契約である。

## protocol / persistence boundary

5専用variantは`MoveInterpolation`へadditiveに追加した。既存`Line / Smooth / Circle`と
`beam_targets=[]`のJSON shapeは変えず、旧`.sdc`を同じ意味でdeserialize/serializeする。
imported bodyはordered `{fixture_id, beam_index, selection_index}`を保存し、復元時に全beamを
atomicにbindする。profile不一致ならidentityを壊さずdormantにし、一部beamだけへ縮退しない。

## assertion delta

| 契約 | 旧 | 新 | 理由 |
|---|---|---|---|
| ID221 nonzero Phasing | dedicated `Skipped` | raw `0..1`をcontinuous fan-outへ保持 | scalar compositionとselection orderを回収しtimer量子化は除外した |
| ID222 / 225 | unknown Move `Skipped` | distinct exact Curve / Points body | evaluator意味論を静的回収した |
| ID223 / 224 | legacy Line近似、beam identity lossを`Approximate` | dedicated exact evaluator + ordered beam targets | evaluatorとfan-outの非等価境界を解消した |
| unknown Move regression | ID222 mutation | ID226 mutation | 222が合法routeになったためfactory外IDへ移した |
| authoring mode | Line/Smooth/Circle | Enhanced 3種を維持しDaslight exact 5種を追加 | 互換quirkを新規authoring既定にしない |
| viewport interpolation control | visible button count 3 | visible button count 8 | Enhanced 3種を削らずexact 5種を一対一で選べるようにした。overflow/containment/reachability assertionは変更しない |
| imported runtime timing | 40 ms frame countへperiodを短縮 | authored `DURATION`（min 10 ms）と連続位相 | 保存値を変えるtimer-grid欠陥と階段状holdを再現しない。Pointsの頂点holdだけは表現として維持 |

## 受入ゲート

- protocol full: `49/49` pass。legacy bytesと5 exact variantのserde名を固定。
- engine focused gateはMOVE path、continuous Phasing、two-wing Symmetry、authored period、
  Points-only holdを対象にする。全workspace件数はcorrected timing tranche統合後に更新する。
- Tauri/backend full: `400` pass / `9` hardware/long-running ignore。full `dvc`は`73/73` pass。
- frontend: TypeScript `--noEmit`、production build、MOVE helper、effect visualization、
  localization `3044/3044`、Scene FX / FX visual / control-modeの5 viewportをpass。
- full viewport matrix: exit `0`。interpolation button `3 -> 8`以外のcontainment/reachability契約は維持。
- 旧integer-frame cacheで測定したvenue/mixed A/B値はcorrected continuous runtimeの性能証跡には
  流用しない。200 fixtures x 64 effects gateとnative release acceptanceは統合後に再実行する。
