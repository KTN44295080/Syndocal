# Daslight parity current-HEAD gap matrix

- 監査日: 2026-07-23
- 監査基準: `83970ea` + T23受入差分
- 製品境界: 内蔵3Dビジュアライザーは対象外。Art-Netを外部ビジュアライザーへ送る経路を正式な可視化受入とする
- 判定規則: Syndocal実装、同一タスク計測、ネイティブ/物理出力を別々に扱い、証拠のないDaslight比較は`未計測`とする

## 現在地

| 領域 | 現HEAD | Daslight比較 | 判定 | 次の閉じ方 |
|---|---|---|---|---|
| Scene配置 | 常設Scene MatrixからTimelineへ1 drag | 実測1 drag | 同 | 13タスクoperation-count gateで回帰固定 |
| Timeline layer mute | 1 click | 実測1 click | 同 | gate維持 |
| Timeline pane expand/restore | 2操作 | 実測2操作 | 同 | gate維持 |
| Scene trigger / Live speed / reset | すべて1 gesture、DOM runtime truthまで検証 | 未計測 | 未計測 | Daslight同一開始状態を実測 |
| Touch live / flash / Edit / control追加 | 各1 gesture、flashはdown/up、追加は件数増加を検証 | 未計測 | 未計測 | Daslight remote/touch面を同一タスク実測 |
| FX family / recipe選択 | 各1 click、active familyとrecipe選択を検証 | 未計測 | 未計測 | 作成、target適用、Cue保存まで比較を拡張 |
| Independent FX | 7系統が独立body/runtime/editor。Colour Mappingは埋め込みimage/text/video、matrix cell、UV/sampling/playbackを持ち、GDTF CIE xyY付き3–16 emitterはcalibrated fixed-anchor mixingを共有する。metadata無し追加emittersはzero | Daslightは7独立generator群 | ソフトウェア構造差は解消。Shinkan動的FXの別プロセスArt-Net受信PASS | プリセット量と実灯体/商用visualizer目視は外部比較 |
| Cue FX parameter transition | Cue-owned Effectごとに任意fade、同一IDの直前live result→次state、連続/離散属性別policy、video target対応 | あり | ソフトウェア構造差は解消・Daslight同一タスク未計測 | 実灯体とDaslight操作を外部比較 |
| Scene Live | scene speed / size / phase / direction / segment / flash、group dimmer / strobe / solo。Shinkanでspeed x4→retrigger x1とArt-Net変化をnative実証 | 同等機能あり | ソフトウェア/実出力PASS・Daslight同一タスク未計測 | 実灯体とDaslight操作を外部比較 |
| Fixture onboarding | GDTF import、faceted Share検索、favorites、検証付きoffline cache/health、exact-layout repair、generic verified common pack、custom builder | 商用統合catalogと復旧UXあり | ソフトウェア構造差は解消・catalog規模/live account未計測 | live Shareとcatalog-scale比較を外部実施 |
| Workspace | 7主要paneの名前付きdetachable workspace、複数window、monitor placement復元 | detachable dual-monitor構成 | ソフトウェア差を解消 | 実2-monitor operator rehearsal |
| Operator policy | credential-safe PBKDF2 verifier、full / partial lock、main/pane共通policy | full / partial lock | ソフトウェア差を解消 | operator rehearsal |
| 3D visualization | 内蔵しない | 内蔵あり | 比較対象外 | 別プロセスArt-Net monitor受信PASS。商用visualizer描画は外部環境依存 |
| DMX/network output | Art-Net / sACN / serial DMX、RDM、active/standby基盤。Shinkan U0 512-byte ArtDMXを別プロセスで880/880 frame受信 | 対応 | software/process境界PASS・物理LAN受入未完 | 外部PC/node一時間soakを外部依存として分離 |
| Standalone hardware write | 対応ハード製品なし | 対応機器あり | PCソフト範囲外 | 未実装を隠さず製品境界として最終判定に明記 |

## T18 operation-count coverage

`qa/harnesses/check-operation-counts.mjs` は13タスクを実CDP gestureで実行し、結果のDOM/runtime反映までfail-closedで確認する。

- Daslight同一タスク実測あり: 3件（すべて同数）
- Syndocal回帰予算のみ、Daslight未計測: 10件
- Matrix/Touch flashのpointer down/up契約は、専用`--scene-live-modifier-only`でも5解像度を別途検証する
- patch、static programming、FX target適用、native Save/reopenはこの13件にまだ含めず、未計測のまま残す

この表は「13件PASS = 全体同等」を意味しない。比較判定を更新できるのは、同じ開始状態と完了条件でDaslight側の操作数も取得したタスクだけである。
