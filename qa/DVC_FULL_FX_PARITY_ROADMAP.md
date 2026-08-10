# Daslight FX 全ジェネレータ実装ロードマップ（DVC-FULL）

- 作成: 2026-08-10 Fable
- **ユーザー指示（2026-08-10、原文）**: 「一応、Daslightからの互換性は持たせたいからFXを増やすのは
  いいんだがDaslightのものは全部実装しろよ」
- 方針転換: これまでの「実ファイル使用実績のあるものだけ厳密実装、他はfail-closed」から、
  **「Daslightの全FXジェネレータを厳密セマンティクスで実装」**へ格上げする。
  fail-closedは実装完了までの一時状態であり、終着点ではない。
- 実装規律（不変）: 静的証明（factory/property/evaluator）に基づく厳密実装のみ。
  推測実装・近似実装はしない。`.sdc` v1互換はserde default追加のみ。44Hz hot pathへ
  未計測コストを持ち込まない。protocol/engine追加は本指示を包括承認として扱う。

## カタログ確定と現行importer coverage（DVC-ENUM完了、2026-08-10）

Daslight 5.0.6.2 / FileVersion `25.0905.165.111`の実機dropdown全項目とfactory登録域を
一対一で照合し、名前付きgenerator 68 IDを全列挙した。未列挙欄は全familyでゼロ。
全IDのclass/property/evaluator入口は`qa/DVC_FULL_FX_CATALOG_PARITY.md`を正本とする。

| ファミリー | 完全factory集合 | 現行converter route | precise fail-closed / 未route |
|---|---|---|---|
| VALUE 7/7 | 621–628（8種） | 621 Rainbow, 623 Plasma, 624 Knight Rider, 625 Sweep | 622/628: non-serialized palette-wrap state、626/627: non-serialized per-thread RNG state/history |
| COLOR FX 2/2 | 121,127,128,129,130,131,133,**134** | 127,129,130 | 121/131/133 precise fail-closed。128 Perlin / 134 Sweepは未route。Sweep=132という旧推定は撤回 |
| MOVE 4/4 | 221 Circle, 222 Curve, 223 Line, 224 Polygon, 225 Points | 221,223,224 | 222,225未route |
| CHASER 3/6 | 321 #1, 322 #2, 323 #3, 324 #4, 325 random | 321,322,325 | 323,324未route。distinct evaluatorの演出意味が未証明 |
| CURVE 8/5 | 3–13（11種） | 3 Inverse Ramp, 7 Sinus, 10 Strobe | 4,5,6,8,9,11,12,13未route。Custom 13だけ別schema |
| MAPPINGS 6/8 | 521–530（10種） | 521 Rainbow | 530 precise fail-closed。522–529未route |
| COLOR MAPPINGS 5/3 | 21,22,23,29–37,40–42,44,45,47–50（21種） | 36 Rainbow | 残20種未route |

現行runtime converter routeは18/68 ID、precise fail-closedは8、未routeは42。これはID単位の入口coverageであり、共有raster classの再利用度や
STEPS/SUPER SCENEなど非generator構造を含む「製品完成率」ではない。未routeをUI名だけで近似せず、
各evaluatorの意味論を回収したトランシェだけを増やす。

### converter routeの厳密性監査

`converter routeあり`と`Daslight exact`は同義ではない。DVC-ENUM直後の逆照合では、strict TYPE/ID検証と
証明済みevaluator境界が揃うものを、次の条件付き6 routeとして分類した。

- VALUE 621 / 623 / 624、および625の`Transform=0`
- COLOR MAPPINGS 36（外部`SELECTIONS`参照なし）
- MOVE 221（`Phasing=0`）

残る16 routeは「変換できる」が、exact完成とは数えない。主な差分は次のとおり。

- COLOR 121/127/131/133はgenericまたは非exact evaluator。133はさらにfactoryの
  `LifeSpan=0..0.9`をpercentとして再解釈するscale不整合がある。129/130は式を証明済みだが、
  palette以外のPARAM TYPE検証が不足する。
- MAPPINGS 521はRainbow/placement coreを証明済みだが、`TYPE1/ID10=0..1`をpercentへ変換しておらず、
  530はgeneric fractal noiseでRectangle placementも読まない。
- CURVE 3/7/10は40ms評価式を証明済みだが、TYPE/range検証がfactory contractより緩い、または狭い。
- CHASER 322は評価意味を証明済みだがTYPE検証不足。321/325は範囲clampを伴い、325は
  Daslightのprocess/thread依存random順をstable permutationへ置換するため非exact。
- MOVE 223/224は`BEAMID` targetを破棄し、複数beamをApproximateへ落とす。224のpolygon count域も
  factory `2..255`と一致しない。

したがって、未route IDの追加より先に既存22 routeをstrict化し、再現不能な近似は理由付き
precise fail-closedへ戻す。22/68は入口coverage、6/68はDVC-ENUM直後の条件付きstrict-core数として
別々に報告する。なお横断監査で、COLOR系paletteはDaslight factory `1..255`に対し現protocol/engineが
`2..16`（VALUEは`2..32`）という既存の表現上限も判明した。以後はこのfull-domain境界を解消するまで、
strict-core数を製品全域のexact完成数とは呼ばない。

## 実施順

1. **DVC-V5a（完了）**: COLOR MAPPINGS placement protocol + ID36 + 521窓修正。
2. **DVC-V5b（完了）**: Move beam-target/circumcircle protocol + ID221。
3. **DVC-ENUM（完了）**: 全ファミリーのfactoryディスパッチ表、実機GUI順、
   ID→creator→constructor→class/vtable→evaluator→property schemaを完全列挙。
   実保存scratchと突合し、現行converter 22/68、precise fail-closed 4、未route 42を確定。
4. **DVC-V6（完了）**: VALUE ID624 Knight Riderをexact実装済み。VALUE残4種は
   評価器自体を回収済みだが、622/628はconstructor由来のpalette-wrap state、626/627は
   per-thread qrand state/historyが`.dvc`から復元不能なため、推測せずprecise fail-closedを維持する。
   追加のDaslight証跡で欠落stateの導出経路を確定できた場合だけexact実装へ戻す。
5. **DVC-CORRECTNESS（進行中）**: 上記16 routeをfactory TYPE/range/evaluator/beam-targetと再突合する。
   **C0a完了**: MAPPINGS 521のunit→percent、COLOR 129/130、CHASER 321/322/325、
   CURVE 3/7/10、MOVE 224のTYPE/domain/no-op順序をfactory contractへ一致させた。
   これにより従来の条件付き6 routeに、129/130、322、3/7/10、521の7 exact-core routeが加わった。
   ただし上記palette上限など横断表現域は残るため、13をfull-domain完成数にはしない。
   ゲートはDVC focused 71/71、Syndocal全体397 pass / 0 fail / 9 ignored、
   `pnpm --dir app tauri build --no-bundle`成功、exact checkout exe 1件のresponsive native windowで固定した。
   **C0b完了**: COLOR 121/131/133とMAPPINGS 530は、非等価generic/RNG/palette state/Rectangle欠落を
   class固有理由付きprecise fail-closedへ戻す。COLOR 127はVALUE 624と共有する回収済み評価器で
   exact化する。MOVE 223/224とCHASER 321/325は既存show互換を維持しつつ常時Approximateを明示する。
   `.dvc`だけではexact replay不能なqrand/palette-wrap/generic置換routeは、近似成功扱いを撤回して
   class固有理由付きprecise fail-closedへ戻す。
   COLOR 127は40ms floor、合成後qGray、Transform foldまで固定し、条件付きexact-coreは14へ増加。
   C0b分類はruntime-convert 18 / precise fail-closed 8 / 未route 42 / 明示compatibility 4。
   ゲートはDVC focused 71/71、workspace全体397 pass / 0 fail / 9 ignored、frontend build、
   localization 3045/3045、Value/FX可視化契約、`tauri build --no-bundle`を通過。exact checkoutの
   release exeをPID 73200で再起動し、`Syndocal` responsive windowが1件であることを確認した。
   **C0c完了**: COLOR / COLOR MAPPINGS / MAPPINGSのfactory palette cardinality `1..255`を、
   protocol、DVC parser、project/reference palette、runtime、editor/libraryの全経路へ通した。1色は
   constant output、255色はlossless、256色はprecise rejectionとし、hot-path lookupは二分探索へ変更した。
   COLOR 127の共有exact Knight evaluatorも0..254 laneを受理する。VALUEの`2..32`は独立factory contractを
   維持する。これで旧`2..16`横断境界は解消したが、残るPARAM/placement/beam/state境界があるためID分類と
   full-domain完成数はまだ増やさない。
6. **新規IDトランシェ**: correctness着地後、(a)既存Sweep evaluatorを再利用できるCOLOR FX 134
   → (b)Move残（222 Curve / 225 Points）→ (c)CURVE残 → (d)CHASER 323/324
   → (e)MAPPINGS/COLOR MAPPINGS 2D群
   （Media/Text等の埋め込み系はColour Mapping既存基盤を再利用）。
7. **P-EXP（完了）**: 23 quick looks（VALUE6 / CURVE3 / CHASER3 / COLOR5 /
   MAPPING3 / MOVE3）を統合済み。ColorMappingは埋め込みmediaのため別設計。

## 各トランシェの受入（共通）

- 静的証明の証跡文書（既存*_SOURCE_PARITY.md様式、アドレス付き）
- 実保存標本またはインポータgoldenでの製品間突合（標本は必要に応じFableが実機採取）
- `.sdc`レガシーbyte-shape回帰、44Hz/release予算ゲート非退行
- Scene Settingsエディタ露出（「検証済み全パラメータ露出」前例の維持）＋JAローカライズ
