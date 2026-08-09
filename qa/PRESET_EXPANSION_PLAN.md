# FXクイックスタート・プリセット拡充計画（P-EXP）

- 作成: 2026-08-10 Fable（ユーザー承認済み: 「拡充する（内容は任せる）」）
- 背景: ギャップマトリクス残件「プリセット量」。現行はseven-family 13カード
  （LFO系Dimmer中心）。T19/T25-G以降に独立化した各ファミリー、特にDVC-V3で
  証明済みのVALUEレシピ群にクイックスタートが無い。
- 原則: 全プリセットは**証明済みレシピの実用値**から構成（新規レシピ発明なし）。
  既存機構（samples/*.effect + main.rs埋め込みalias + カード + JAローカライズ +
  検証テスト）への追加のみ。protocol変更なし。

## 追加プリセット（12件）

### VALUE FX（5件 — DVC-V3証明レシピの活用が主眼）

| ファイル | レシピ | パラメータ設計 | 用途 |
|---|---|---|---|
| front-value-sweep.effect | Sweep | 白/灰/黒3値、direction_change=true、2 beats | Daslight 625パリティの代表例。バー系の往復ワイプ |
| front-value-plasma.effect | Plasma | Daslightコンストラクタ既定値（1,2,1,2,-1,2,1,-1）、8 beats | 有機的うねり。625/623標本と同値=検証容易 |
| front-value-knightrider.effect | KnightRider | size=2、fading=true、go_outside=false、1 bar | 定番スキャナー |
| front-value-sparkle.effect | Sparkle | number=6、lifespan=40%、width=1、1 beat | ランダムきらめき（LFO randomと差別化: strip cell単位） |
| front-value-burst.effect | Burst | color_width=0.5、gradient=1.0、4 beats | 中心から広がる放射 |

### CHASER FX（2件）

| ファイル | 設計 | 用途 |
|---|---|---|
| front-chaser-pixels.effect | Forward、Pixels on=2、duty=50%、Fading on、1 beat/step | 基本の流しチェイス |
| front-chaser-bounce.effect | Bounce、Pixels on=1、Wings=2、duty=75%、0.5 beat/step | 左右対称バウンス |

### MOVE FX（1件）

| ファイル | 設計 | 用途 |
|---|---|---|
| front-move-line.effect | Line path（pan横一往復）、4 beats、Smooth | circleに次ぐ基本。既存defaultMovePathPointsのline系を利用 |

### CURVE FX（2件）

| ファイル | 設計 | 用途 |
|---|---|---|
| front-curve-strobe.effect | Strobe curve、rate=25Hz相当、0.5 beat | 既存curve-sawと対のパルス系 |
| front-curve-ramp.effect | RampInv、2 beats | フェードイン反復 |

### MAPPINGS（2件）

| ファイル | 設計 | 用途 |
|---|---|---|
| front-mapping-rainbow.effect | Mapping Rainbow（521系レシピ既定、color_width=0.3、gradient=100） | 2Dレインボーの入口。**V5a placement body着地後はその既定窓も設定** |
| front-mapping-plasma.effect | Mapping系Plasma相当（既存レシピで表現可能な場合のみ。不可なら本件から除外し理由を記録） | 2D有機ノイズ |

## 受入条件

1. 各プリセットは既存 `load_sample_effect_preset` 経路で読み込め、`Target` 再ターゲットが
   既存プリセットと同じ制約で動く（whole-fixture系はfixture/group必須等）。
2. `cargo test -p syndocal sample_effect`系（プリセット妥当性・retarget・Engine挿入）へ
   新プリセットを全数追加。
3. カードUI: 既存カード様式のまま件数増（13→25前後）。チューザーの一画面契約を壊さない
   （溢れる場合はファミリー内スクロール——既存パネル内スクロール原則に従う）。
4. JAローカライズ100%維持（`check:localization`）。パラメータ語彙はT25-Gの英語不変語に従う。
5. README/samples README更新は従来どおり後回し可。

## 実施

- V5a（COLOR MAPPINGS protocol）着地後にCodexへ委任（同一ツリー順次原則）。
- MAPPINGSの2件はV5aの結果（placement bodyの形）を見て最終化。

## 実装転換と進捗（2026-08-10、Fableレーン fable/preset-expansion）

**面の転換**: 調査の結果、`samples/*.effect`＋`load_sample_effect_preset`経路はT26/T27
再設計以降フロントエンド呼び出しゼロの孤立コードと判明。プリセット拡充は生きている面
＝**Scene Settingsの各ファミリーエディタへの「クイックルック」行**（1クリックで
名前付きレシピ+パレット+クロック適用、`moveEffectClockPresets`様式再利用で
styles.css不変）として実装する。.effectファイル追加は行わない。

- **スライス1（45f29c4）**: VALUEクイックルック6種（Sweep Bounce / Plasma Drift＝
  Daslight実既定値 / Knight Rider Scan / Sparkle Rain / Burst Pulse / Random Fill Steps）。
  ハーネスへ実クリック断言 `valueQuickLooksApplyNamedRecipesFromOneClick` 追加。
- **スライス2（29b14d9）**: CURVEクイックルック3種（Ramp Up / Strobe Snap / Soft Breathe）。
- **スライス3（d410a7d）**: CHASERクイックルック3種（Pixel March / Wing Bounce /
  Random Sparkle——ステップは対象由来のまま再生パラメータのみ適用）。
- 計12ルック着地。残り（COLOR / MOVE / MAPPINGS / COLOR MAPPINGSのクイックルック）は
  V5a/V5bが同エディタ群を編集中のため、着地後の続きスライスとする。
- 各スライスのゲート: tsc / vite build / check:localization 100% /
  焦点viewport（scene-settings・scene-fx-block）5/5。マージはV5a後に合議。
