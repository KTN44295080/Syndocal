# Syndocal UI Redesign Continuation — T15 / T16 / T17

作成・承認: 2026-07-19  
状態: **承認済み** — ユーザーは下記の設計判断をすべて推奨案で承認。  
実施順: **T15-P 基盤改修 → T15 タイムライン減量 → T16 FXエディタ可視化 → T17 Scene Liveモディファイア**

## 目的

Daslight 5実機とSyndocal現行UIを比較し、演奏面の可読性と直接操作性を優先して次を完成させる。

1. PATCHを128chページ式から、1 Universeあたり512ch連続のコンパクトな内部スクロール面へ戻す。
2. ControlをMatrix主体の上段と、Groups＋Stage／Timelineの下段2ペインへ整理する。
3. 上下および下段左右を最小寸法付きスプリッターでリサイズ可能にする。
4. Timelineのブロック、ツール、タブ、ガター、グリッド、マーカーを減量する。
5. Daslightの9ファミリーを入口にしたグラフィカルFXエディタを作る。
6. Sceneセル／Touch padからspeed・size・phase・flashを即時操作できるLive modifierを作る。

## 合意済み設計判断

### ワークスペース構造

- 下段は **左＝Groups＋Stage、右＝Timeline** の2ペインとする。
- Selectionsは独立常設列をやめ、左ペイン内の折り畳みドロワーへ収納する。
- 上下スプリッターと下段左右スプリッターの両方をドラッグ可能にする。
- 比率はプロジェクト`.sdc`ではなく端末ローカルへ保存する。
- スプリッターのダブルクリックと`Reset Layout`で既定比率へ戻す。
- 1280x720でも各操作面の機能的最小寸法を割らないよう比率をclampする。
- T8のTimeline一時展開、Escape完全復元、T12のペインウィンドウを維持する。

### Control上段

- Scene Matrixを主面として最大化する。
- Transportは1行へ圧縮する。
- status群は常設の大きなサイドバーから、折り畳み可能な右インスペクターへ移す。
- Active/Next、安全操作、GO、blackoutの意味論と既存コマンド経路は変えない。

### Timeline情報設計

- 外側`Live Edit / Timeline / Mixer`は維持する。
- 内側`Cues`タブは廃止し、Cue編集はMatrixセルまたはBlock Propertiesの`Edit Source`から開くコンテキストドロワーへ移す。
- `Show / Automation / Playback`は同じコンパクトツール列のアイコントグルへ変える。
- ガターは番号＋展開＋eye＋lockを常設要素とする。
- 展開はレーン詳細を開閉する。solo、名前変更、追加、削除、並べ替えはレーンコンテキストメニューへ移す。
- Cueの保存済みidentity色をブロック全面へ使い、太い黒縁、1行目＝名前、2行目＝長さとする。
- playhead、選択ハンドル、point eventは常時維持する。
- overlap badgeとdrag live stampは必要時のみ表示し、minor gridをさらに減光する。
- Block PropertiesはTimeline右側のコンテキストドロワーとして表示する。

### PATCH表示

- 32列×16行でアドレス1～512を同じDOM／同じスクロール面へ描画する。
- セルは18～21px程度を基準とし、大画面でも引き伸ばさない。
- 画面上の番地は行頭・終端などの節目とhover/focus readoutを使う。
- 全512セルは完全な`title`、accessible name、キーボード到達経路を持つ。
- 512個のtab stopは作らず、roving tabindexと矢印/Home/End/Ctrl+Home/Ctrl+Endで移動する。
- 選択灯体、Next Free、Universe Overviewからの選択は対象Universeと対象アドレスを内部スクロールで表示する。
- document／`.app`スクロールは常に0とする。

### T16 FXエディタ

入口は次の9ファミリーをDaslightと同順で表示する。

1. STEPS
2. COLOR FX
3. CHASER FX
4. MOVE FX
5. VALUE FX
6. CURVE FX
7. MAPPINGS
8. COLOR MAPPINGS
9. SUPER SCENE

- STEPSとSUPER SCENEは既存編集経路へ遷移するナビゲーションファミリーとして同じchooserに含める。
- LFO／Curveは実パラメータから波形カーブを描画する。
- Colorは実パレットからグラデーションストリップを描画する。
- Moveは実頂点をXY pad上へ描画し、頂点を直接ドラッグ編集する。
- previewは飾りではなく、保存・runtimeへ渡る既存データと同じ値を使う。

### T17 Scene Liveモディファイア

- DVC由来またはSyndocalで編集したspeed・size・phase・flashの初期値はCueの保存属性とする。
- `.sdc v1`互換を守り、追加フィールドは`serde(default)`とする。
- speed・size・phaseはアクティブSceneに対するラッチ式Live overrideとする。
- flashは押下中だけ有効なモーメンタリ操作とする。
- Live overrideは`.sdc`へ自動保存しない。
- Scene release／再trigger／project load時のリセット規則を明示し、ハーネスとengine testで固定する。
- 新しい値検索、allocation、blockingを44 Hz hot pathへ追加しない。

## トランシェと受入条件

### T15-P: PATCH＋可変Control基盤

- PATCH page selectorが0件、DMX address cellが512件、1～512が一意に存在する。
- A512へ内部スクロールでき、外側スクロールは0のまま。
- 高番地セルをクリック／キーボード操作できる。
- 上下・左右スプリッターがpointer capture、Escape cancel、最小寸法clamp、比率保存、Resetを満たす。
- Setup／Control切替、T8展開／復元、T12ペインウィンドウが新契約でgreen。

### T15: タイムライン減量

- ブロックは太いidentityベタ塗り＋黒縁＋名前／長さの2行。
- 既存のmove、RATE/WINDOW resize、fade、Magnet、arm/place、zoom/panを失わない。
- 文字ツールの機能はaccessible name付きアイコンへ移し、機能自体を削らない。
- Cues保存値を新しいdrawer経路へmigrationし、旧localStorage値でも面が消えない。
- 500ブロック／1時間／6レーン／overlap fixtureのnode budgetと直接操作契約を維持する。

### T16: FXエディタ可視化

- 9ファミリーが常時識別可能で、既存7 FX＋STEPS＋SUPER SCENEへ到達できる。
- LFO curve、palette gradient、Move pathが実値と一致する。
- Move頂点dragはghost、live readout、Escape cancel、範囲clampを持つ。
- 全新規文字とaria labelをJAへlocalizeする。

### T17: Scene Liveモディファイア

- MatrixセルとTouch padの両方から同じruntime override経路を操作できる。
- speed／size／phaseはactive sceneへ即時反映し、flashはpress／releaseで確実に解除される。
- authored値とLive overrideをUIで区別できる。
- legacy `.sdc`は同じ初期挙動で読み込める。
- project round-trip、DVC import、runtime lifecycle、44 Hz非増加をテストする。

## 共通検証順

各トランシェは次を順番に実行し、後段失敗後は修正して先頭から再実行する。

1. `git diff --check`
2. frontend helper／focused harness
3. `pnpm exec tsc --noEmit`
4. `pnpm run check:localization`
5. `pnpm exec vite build --configLoader runner`
6. 当該focused viewport 5解像度
7. full viewport matrix
8. protocol／engine／project focused Rust suites（該当トランシェのみ）
9. ネイティブSyndocalでShinkan2026実データを目視・操作

各トランシェを独立コミットとし、次へ進む前に差分、断言変更、全ゲート結果を記録する。
