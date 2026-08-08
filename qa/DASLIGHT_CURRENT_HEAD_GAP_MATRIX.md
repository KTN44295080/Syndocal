# Daslight parity current-HEAD gap matrix

- 監査日: 2026-08-08
- 監査基準: 本ファイルを含む現HEAD（Move Symmetry照合 + DVC beam-target保持）
- 製品境界: 内蔵3Dビジュアライザーは対象外。Art-Netを外部ビジュアライザーへ送る経路を正式な可視化受入とする
- 判定規則: Syndocal実装、同一タスク計測、ネイティブ/物理出力を別々に扱い、証拠のないDaslight比較は`未計測`とする

## 現在地

| 領域 | 現HEAD | Daslight比較 | 判定 | 次の閉じ方 |
|---|---|---|---|---|
| Scene配置 | 常設Scene MatrixからTimelineへ1 drag | 実測1 drag | 同 | 13タスクoperation-count gateで回帰固定 |
| Timeline layer mute | 1 click | 実測1 click | 同 | gate維持 |
| Timeline pane expand/restore | 2操作 | 実測2操作 | 同 | gate維持 |
| Scene trigger / Live speed / reset | trigger、speed、全modifier reset、releaseが各1 gesture。DOM runtime truthまで検証 | **2026-08-08最大化実測**: trigger=1 click、live speed=1 drag、release=1 click。Daslightは4ダイヤルの一括Resetなし。右クリックはTouch追加、double-clickは値変更であり、manual v1.4もLive Control Dial resetを定義しない | trigger/live speed/releaseは**同**。一括resetはSyndocalが**1 click少ない経路を独占** | `qa/DASLIGHT_OPERATOR_COUNT_AUDIT_2026-08-08.md`で回帰根拠を固定 |
| Touch live / flash / Edit / control追加 | 各1 gesture、flashはdown/up、追加は件数増加を検証 | **2026-08-08最大化実測**: workspace=1 click、scene trigger=1 click、Flash=1 press/release、Edit=1 click、control追加=2 clicks。FlashはAdvanced Propertiesで有効化したsceneをTouchへ追加し、解放後に非latchedとなることを確認 | 4件同、control追加はSyndocalが1 click少ない | `qa/DASLIGHT_OPERATOR_COUNT_AUDIT_2026-08-08.md`で回帰根拠を固定 |
| FX family / recipe選択 | 各1 click、active familyとrecipe選択を検証 | **2026-08-08最大化実測**: 開いたFX追加menuからMAPPINGSを1 clickで生成し、MAPPINGS editor表示まで確認 | Mapping FX family作成は**同** | target適用/Cue保存は別タスクとして比較を拡張 |
| Independent FX | 7系統が独立body/runtime/editor。Colour Mappingは埋め込みimage/text/video、matrix cell、UV/sampling/playbackを持つ。Move SymmetryはDaslight実測どおり後半Panのみ鏡像化。Curve/ChaserはDVCのfixture/BEAMID/selection順を保持し、RGB/RGBAの選択セグメントだけを変調する | Daslightは7独立generator群。Move SymmetryとChaser #2は最大化UI + DMX Levelsで実測 | 対象検体のソフトウェア/DMX規則差は解消。Shinkan動的FXの別プロセスArt-Net受信PASS | プリセット量と実灯体/商用visualizer目視は外部比較 |
| DVC dynamic FX import | フルShinkanの30 effectをSkipped 0で変換。Bar-StrobeAMber=64 beams / 48 selections / BEAMID 0..7、Bar-Side Chaser=16 steps / 16 beams / BEAMID 0..7を保存・再読込・DMX隔離テストで固定 | 原XMLのBEAMS列とDaslightのgenerator/bodyを比較 | Curve/Chaserの旧fixture平坦化を解消 | VALUE FX / COLOR MAPPINGSはユーザーDVCに検体なし。MoveのBEAMID>0検体のみ未証明 |
| Cue FX parameter transition | Cue-owned Effectごとに任意fade、同一IDの直前live result→次state、連続/離散属性別policy、video target対応 | あり | ソフトウェア構造差は解消・Daslight同一タスク未計測 | 実灯体とDaslight操作を外部比較 |
| Scene Live | scene speed / size / phase / direction / segment / flash、group dimmer / strobe / solo。Shinkanでspeed x4→retrigger x1とArt-Net変化をnative実証 | 同等機能あり | ソフトウェア/実出力PASS・Daslight同一タスク未計測 | 実灯体とDaslight操作を外部比較 |
| Fixture onboarding | GDTF import、faceted Share検索、favorites、検証付きoffline cache/health、exact-layout repair、generic verified common pack、custom builder | 商用統合catalogと復旧UXあり | ソフトウェア構造差は解消・catalog規模/live account未計測 | live Shareとcatalog-scale比較を外部実施 |
| Workspace | 7主要paneの名前付きdetachable workspace、複数window、monitor placement復元 | detachable dual-monitor構成 | ソフトウェア差を解消 | 実2-monitor operator rehearsal |
| Operator policy | credential-safe PBKDF2 verifier、full / partial lock、main/pane共通policy | full / partial lock | ソフトウェア差を解消 | operator rehearsal |
| 3D visualization | 内蔵しない | 内蔵あり | 比較対象外 | 別プロセスArt-Net monitor受信PASS。商用visualizer描画は外部環境依存 |
| DMX/network output | Art-Net / sACN / serial DMX、RDM、active/standby基盤。Shinkan U0 512-byte ArtDMXを別プロセスで880/880 frame受信 | 対応 | software/process境界PASS・物理LAN受入未完 | 外部PC/node一時間soakを外部依存として分離 |
| Standalone hardware write | 対応ハード製品なし | 対応機器あり | PCソフト範囲外 | 未実装を隠さず製品境界として最終判定に明記 |

## T18 operation-count coverage

`qa/harnesses/check-operation-counts.mjs` は13タスクを実CDP gestureで実行し、結果のDOM/runtime反映までfail-closedで確認する。さらにSave/Openは`app/scripts/check-project-shortcuts.mjs`がグローバルkeyboard controllerの1 gesture dispatchを実行検証し、Windowsネイティブ版で`.sdc`の実write/readを確認する。

- Daslight同一タスク実測あり: 14件（13件同数、1件はSyndocalが少ない）
- Daslightに同等の1操作がない: 1件（全scene live modifier reset）
- Matrix/Touch flashのpointer down/up契約は、専用`--scene-live-modifier-only`でも5解像度を別途検証する
- Saveは両者`Ctrl+S`の1操作、名前付きproject再読込は両者`Ctrl+O`→path→`Enter`の3操作。SyndocalはWebView2への修飾キー注入が効かなかったため、shortcut dispatch gateとnative menu persistenceで経路を分けてfail-closed検証した
- patch、static programming、FX target適用はまだ含めず、未計測のまま残す

この表は「15件で優位または同数 = 全体同等」を意味しない。比較判定を更新できるのは、同じ開始状態と完了条件でDaslight側の操作数も取得したタスクだけである。直接実測の詳細は`qa/DASLIGHT_OPERATOR_COUNT_AUDIT_2026-08-08.md`を正本とする。
