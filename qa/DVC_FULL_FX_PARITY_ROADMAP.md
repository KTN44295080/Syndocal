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

## 既知のカタログ状況（2026-08-10時点）

| ファミリー | family/type | 実装済み | fail-closed（欠落証跡待ち） | 未列挙 |
|---|---|---|---|---|
| VALUE | 7/7 | 621 Rainbow, 623 Plasma, 624 Knight Rider（DVC-V6 exact整数評価器＋Transform 0/1）, 625 Sweep | 622 Burst / 628 Perlin（object `+0x12c` palette-wrap値がconstructor未初期化かつ`.dvc`未serialize）、626 Sparkles / 627 Random fill（per-thread qrand初期state・prior draw historyが`.dvc`にない） | なし（factory全域列挙済み） |
| COLOR FX | 2/2 | 121, 127, 129, 130, 131, 133 | — | ID空隙（122-126, 128, 132等）の有無をfactory列挙で確定 |
| MOVE | 4/4 | 221 Circle, 223, 224 | — | CLine/CPoints/CPolygon/CCurvePos系のID列挙 |
| CHASER | 3/6 | 321, 322, 325 | — | ID空隙の列挙 |
| CURVE | 8/5 | 3, 7, 10 | — | CRamp/CSquare/CTriangle/CTangeant/CCustomCurve/CGraph系のID列挙 |
| MAPPINGS | 6/8 | 521, 530 | — | 2D系カタログ全列挙 |
| COLOR MAPPINGS | 5/3 | 36 Rainbow | UI実測: Spiral/Burst/Butterfly/Plasma/Bounce/Fire/Media/Knight Rider/…（スクロール続き） | factory全列挙 |

RTTI既知クラス（未対応分の候補）: CSpiralEffect, CButterflyEffect, CBounceEffect,
CFireEffect, CMediaEffect, CRainEffect, CGridEffect, CStarfieldEffect, CTubeEffect,
CExplosionEffect, CTextEffect, CPulseEffect, CSquareEffect, CTriangleEffect,
CTangeantEffect, CRampEffect, CCustomCurveEffect, CGraphEffect, CBasicBlockEffect,
CLinePosEffect, CPointsPosEffect, CPolygonPosEffect, CCurvePosEffect, CRandomEffect,
CLineEffect, CCircleEffect, CStarfieldEffect。

## 実施順

1. **DVC-V5a（完了）**: COLOR MAPPINGS placement protocol + ID36 + 521窓修正。
2. **DVC-V5b（完了）**: Move beam-target/circumcircle protocol + ID221。
3. **DVC-ENUM**: 全ファミリーのfactoryディスパッチ表を静的全列挙し、
   ID→クラス→プロパティスキーマの完全カタログを確定（本書の未列挙欄を埋める）。
   実装済みIDとの突合で残作業リストを確定。
4. **DVC-V6以降**: DVC-V6でVALUE ID624 Knight Riderをexact実装済み。VALUE残4種は
   評価器自体を回収済みだが、622/628はconstructor由来のpalette-wrap state、626/627は
   per-thread qrand state/historyが`.dvc`から復元不能なため、推測せずprecise fail-closedを維持する。
   追加のDaslight証跡で欠落stateの導出経路を確定できた場合だけexact実装へ戻す。
   以降の優先順: (a)Move残（Line/Points/Polygon/Curve）
   → (b)CURVE残 → (c)CHASER/COLOR FXの空隙 → (d)MAPPINGS/COLOR MAPPINGS 2D群
   （Media/Text等の埋め込み系はColour Mapping既存基盤を再利用）。
5. **P-EXP（プリセット拡充）**: 実装済みファミリーから順次（qa/PRESET_EXPANSION_PLAN.md）。

## 各トランシェの受入（共通）

- 静的証明の証跡文書（既存*_SOURCE_PARITY.md様式、アドレス付き）
- 実保存標本またはインポータgoldenでの製品間突合（標本は必要に応じFableが実機採取）
- `.sdc`レガシーbyte-shape回帰、44Hz/release予算ゲート非退行
- Scene Settingsエディタ露出（「検証済み全パラメータ露出」前例の維持）＋JAローカライズ
