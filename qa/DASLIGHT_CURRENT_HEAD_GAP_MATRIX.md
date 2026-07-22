# Daslight parity current-HEAD gap matrix

- 監査日: 2026-07-23
- 監査基準: `adcb402` + T18作業差分
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
| Independent FX | Colour / Chaser / Move / Value / Curve / fixture-order Mapping / 2D Colour Mappingが独立body/runtime/editor。Colour Mappingは埋め込みimage/text/video、matrix cell、UV/sampling/playbackを持つ | Daslightは7独立generator群 | 構造上同・プリセット/実出力未計測 | T19-D/EとT23の物理/外部可視化受入へ分離 |
| Cue FX parameter transition | Cue所有paramsとselective recallあり、Cue間morphなし | あり | 劣 | T19-D |
| Scene Live | speed / size / phase latch、Matrix/Touch flash | direction / segment / strobe / soloを含む | 劣 | T20 |
| Fixture onboarding | GDTF import/cache、Share検索/download、custom builder | 商用統合catalogと復旧UXあり | 劣 | T21でfavorites/offline/health/repair/common pack |
| Workspace | Stage / Timeline popout、split保存 | 任意pane/workspace構成 | 劣 | T22で名前付きdetachable workspace |
| Operator policy | destructive confirmationはあるがpassword/partial lockなし | lock機能あり | 劣 | T22でcredential-safe full/partial lock |
| 3D visualization | 内蔵しない | 内蔵あり | 比較対象外 | Art-Net外部可視化をT23で実証 |
| DMX/network output | Art-Net / sACN / serial DMX、RDM、active/standby基盤 | 対応 | 実装済み・物理受入未完 | T23で利用可能な外部Art-Net経路、node soakは外部依存を分離 |
| Standalone hardware write | 対応ハード製品なし | 対応機器あり | PCソフト範囲外 | 未実装を隠さず製品境界として最終判定に明記 |

## T18 operation-count coverage

`qa/harnesses/check-operation-counts.mjs` は13タスクを実CDP gestureで実行し、結果のDOM/runtime反映までfail-closedで確認する。

- Daslight同一タスク実測あり: 3件（すべて同数）
- Syndocal回帰予算のみ、Daslight未計測: 10件
- Matrix/Touch flashのpointer down/up契約は、専用`--scene-live-modifier-only`でも5解像度を別途検証する
- patch、static programming、FX target適用、native Save/reopenはこの13件にまだ含めず、未計測のまま残す

この表は「13件PASS = 全体同等」を意味しない。比較判定を更新できるのは、同じ開始状態と完了条件でDaslight側の操作数も取得したタスクだけである。
