# DVC COLOR MAPPINGS Explosion / Starfield parity

## 範囲と証拠

- 対象はCOLOR MAPPINGS `RACK TYPE=5 / EFFECT TYPE=3`のExplosion ID47とStarfield ID48。
- 実保存正本は`qa/specimens/ColorMappings-Remaining7.dvc`、SHA-256は
  `80CB936E0AE81BF2A809B49F381BCB6771373D95F91BE9DFA4D608FE93A936F9`。
- binary 5.0.6.2の`CExplosionEffect` / `CStarfieldEffect`、共通retained-particle update/paintを
  逆照合した。保存されないprocess-global qrand履歴とsource defectだけを訂正するため、runtime noteは
  `implementation=SyndocalCorrected`とする。

## strict importer schema

| generator | exact PARAMS |
|---|---|
| Explosion 47 | NB=12: T4/1 Palette、T2/2 Grayscale、T6/3 Transform、T0/4 Rotation、T7/10 Shape 0..29、T0/11 Explosion Number 1..50、T0/12 Explosion Size 0..100、T0/13 Particles 1..100、T0/14 Particle Size 1..100、T1/15 Life 0..0.9、T0/16 Trail 1..25、T1/17 Gravity 0..10 |
| Starfield 48 | NB=9: T4/1 Palette、T2/2 Grayscale、T6/3 Transform、T0/4 Rotation、T7/10 Shape 0..29、T0/11 Particles 1..10、T0/12 Size 1..100、T0/13 Trail 1..25、T1/14 Rotation -5..5 |

- Paletteは2..255。PARAM ID/TYPE/NB、COLORS NB/body、Rectangle、direct BEAMS、external
  SELECTIONS不在をstrict検証する。
- 実保存の`Rectangle=(0,0,-1,-1,0,LOCKED=0)`かつdirect `BEAMS NB=0`は、全schema検証後だけ
  source no-opとして受理し、runtime effect/IDを割り当てない。populated targetとのsentinel併用は拒否する。
- TYPE7 Shape 0は`addEllipse(0,0,100,100)`相当のfilled ellipseをsize/100でscaleし、particle
  `(x,y)`をtop-leftとして配置する。Shape 1..29は合法なserialized domainだがproprietary glyph pathが
  未回収のため、empty source no-opでは保持し、populated routeではprecise fail-closedにする。

## corrected deterministic evaluator

- source frame数`R=max(1,floor(duration_ms/40))`、描画frame数`F=min(R,750)`。5000組の
  deterministic `(f32,f32)` tableをsource identity由来`rng_seed`から一度だけcompileする。
- 乱数、速度、gravity、rotation、radians/trig/trunc、世代updateは回収どおりf32順序を保持し、
  固定位置が決まった後だけellipse renderer用f64へ拡張する。
- 共通paint順はspawn後、oldest child→newer child→parent、その後update。履歴はupdate前parent状態をcloneし、
  Trail上限で最古を破棄する。sourceがlifetimeをgreen channelへ書いていたdefectだけをopacityへ訂正し、
  RGB source-over完了後にqGrayを適用する。

Explosion:

- `period=max(1,F/ExplosionNumber)`、`frame % period == 0`でspawnする。この整数周期は個数上限ではなく、
  例えば`F=125, Number=50`では63 spawnになるsource quirkを保持する。
- centerは各axis `W/6 + rand * (5W/6-W/6)`、particle table indexは`frame+particle`、
  `vx=4*rx-2`、`vy=4*ry-2`。
- paletteは`((frame/period) % (N-1))+1`、sizeはParticle Size、scalarは1、
  alpha decayは`(1-Life)*0.05`、motion倍率は`Explosion Size/10`。位置update後に`Gravity/50`を
  velocityへ加える。

Starfield:

- intervalは`12-Particles`でframe 0からspawnする。paletteは
  `min(N-1, 1+trunc((N-1)*rx))`としてf32 endpointをclampする。
- radiusは`trunc(rx*10+5)`、angleは`ry*360`。初期位置は
  `floor(W/2)+trunc(cos(angle)*radius)` / `floor(H/2)+trunc(sin(angle)*radius)`。
- sizeはSize、scalarは1、alpha decayは`1/R`。各generationはradiusを1増やし、現angleで位置更新後、
  次generation向けにRotationを加える。

## sampled-only cacheと44 Hz境界

- effectごとに5000-pair tableと、resolved targetが参照するunique pixelだけのRGB SoA cacheをcompile時に
  一度だけ確保する。各rowはX昇順slotへ連続配置し、row offset + lower-bit popcountでO(1) lookupする。
  tick中の再確保はない。
- production compileはplacement/symmetry/raster rotation適用後のresolved target `x/z`を同じ
  `floor(clamp(normalized)*100)`でpixel化し、`[u128;100]` row maskへ固定する。
- ellipseは従来と同じ`floor/ceil`、normalized Y、sqrt、row min/max演算でspanを求め、sampled row/spanの
  積集合bitだけをX昇順でsource-overする。連続X runは既存のbit-exact u16 channel-sliceをRGB各SoAへ適用する。
  spawn/particle/child/parentの順序、edge coverage、alpha roundingは変更しない。
- resolved target数がsupported envelopeの200以下なら、exact sampled-only rendererで全`F<=750`世代のRGBを
  compile時に生成し、runtimeはgenerationとslotからO(1)で読む。200超も拒否せず同じSoA rendererの
  generation cacheへfallbackする。最大200 unique pixel時はF125で150 KB/effect、F750で900 KB/effect、
  64 effectのF750上限は57.6 MB。qGrayはprecomputed RGB取得後の最終処理として維持する。
- supported envelopeではpalette、5000-pair table、全世代RGBをimmutable `Arc` backingで共有し、unused
  dynamic cacheは保持しない。実cue transition source→transition cloneでも全backing pointerと出力が一致し、
  particle backingのallocation/copyはない。200 target超のpermissive dynamic fallbackはgeneration cacheを
  所有するため、このcue-clone allocation保証の対象外とする。steady evaluatorは両routeともallocation-free。
- reference full dense AoS painterとのedge/fractional/random ellipse差分、dynamic-vs-precomputedの
  全generation 0..124、deep generation 124/125/749 golden、top-left footprint、child-before-parent、
  qGray-lastを固定する。

専用production gateはExplosion 32 + Starfield 32、200 fixture、Palette 255、100x100、全recipe literalを
最大値、`R=F=2`でgeneration transitionを頻発させ、5/8/12 msをassertする。これは最大parameter域を
検証するgateであり、Trail 25の最大temporal populationを主張しない。sibling deep gateはcaptured
`R=F=125`、staggered generation 93..124で、Explosionの63 spawn x 100 particleとTrail 25を含む。

- eager full raster初回release: p95 192,811 us / p99 215,849 us / max 220,355 us（赤）。
- sampled-row + one-pass trajectoryのdeep release: p95 92,290 us / p99 98,332 us /
  max 101,766 us（赤）。
- 全世代sampled-only precompute後のdeep release初回green: compile 5,167 ms、p95 1,475 us /
  p99 1,759 us / max 2,367 us。最終再走はcompile 5,151 ms、p95 1,740 us / p99 2,080 us /
  max 2,485 us。Arc transition修正後はcompile 5,166 ms、p95 1,557 us / p99 1,807 us /
  max 2,332 us。root最終再走はcompile 5,228 ms、p95 1,865 us / p99 2,205 us /
  max 2,618 us、generation 124 hit 560（緑）。
- 同じ実装のfrequent-transition release初回green: compile 37 ms、p95 996 us / p99 1,431 us /
  max 2,293 us。最終再走はcompile 36 ms、p95 944 us / p99 1,207 us / max 1,983 us、
  Arc transition修正後はcompile 35 ms、p95 894 us / p99 1,062 us / max 1,977 us、
  root最終再走はcompile 36 ms、p95 887 us / p99 978 us / max 1,961 us、
  569 transition / 1,000 samples（緑）。両gateともruntime rebuild 0で、Arc palette、random table、
  precomputed RGB tableのpointerは全sample前後で不変、dynamic cacheは不在。

## 残る境界

- Shape 1..29のglyph pathは未回収であり、populated effectは引き続きfail-closed。
- qrandのprocess-historyそのものはファイルにないためreplayせず、stable source seedで訂正する。
- native window/browser/灯体受入はこのstatic/runtime trancheの範囲外。
