# Backend Operator Command Contract

Syndocalのショー状態・DMX出力へ影響する操作は、UIイベントを直接状態へ書き込まず、型付きバックエンドコマンドから既存Engineコマンドへ到達させる。

## 必須契約

- Desktop UIは`invoke` / `tauriInvoke`経由でTauriバックエンドを呼ぶ。ネイティブバックエンドがない場合は成功扱いせずfail closedとする。
- Cue、Timeline、FX、Programmer、Patch、DMX/VJ出力、MIDI/OSC設定などのプロジェクト変更は`projectMutationCommands`を通り、Undo可能なバックエンドトランザクションになる。
- MIDI、OSC、Remote WebSocket、将来のAIオペレーターは、UIとは別のEngine再生経路を作らず、同じコマンド生成ヘルパーを共有する。
- パネルの開閉、選択枠、ズーム、ウィンドウ配置などDMXへ影響しない表示状態だけはフロントエンド専用でよい。
- 実行時選択コンテキストは`.sdc`へ保存しない。保存対象のCue/Timeline/FXデータとは混同しない。

## 選択中Feature Fader / DVC action 229

Daslightのaction 229は固定Dimmerではなく、現在表示中のFeatureフェーダー番号を指す。Syndocalは次の同一経路で扱う。

1. UIが選択中の`fixture_ids`と、表示中フェーダーの`attributes`順を`set_operator_selection_context`へ送る。
2. `set_operator_feature_fader(target_index, value)`が番号を属性へ解決し、選択灯体すべてがその属性を持つことを検証する。
3. 検証成功時だけ既存の`EngineCommand::SetFixtureAttributeBatch`を生成する。
4. Tauri、MIDI、OSC、Remote WebSocketは同じ`operator_feature_fader_command`を使う。選択なし、範囲外、非対応属性では出力せずfail closedにする。
5. MIDI feedbackも同じruntime selectionから現在値を解決する。複数選択の値が一致するときだけCC/Note値を返し、mixed valueでは誤ったモーターフェーダー位置を送らずfail closedにする。
6. Auto feedbackは`set_midi_feedback_auto`でbackend workerを構成する。workerはEngineと同じ44 Hzで公開snapshotを読み、変化したMIDI addressだけを送る。`midi_feedback_status`でfaultを外部から監視でき、`send_midi_feedback(force=true)`で全状態を明示再送できる。UI timerを正規実行経路にしない。

Remote WebSocketの外部呼び出し例:

```json
{"type":"setOperatorSelection","fixture_ids":[1,2],"attributes":["Dimmer","ColorRed","ColorGreen","ColorBlue"]}
{"type":"setOperatorFeatureFader","target_index":1,"value":0.5}
```

## 回帰ゲート

```powershell
pnpm --dir app run check:backend-operator-contract
pnpm --dir app run check:dvc-midi-shortcuts
```

前者はフロントエンドからリテラルで呼ぶコマンドおよびプロジェクト変更コマンドがTauri handlerへ登録済みであること、選択中Feature Faderの全入力経路が共通ヘルパーへ収束していることを静的に検証する。

## 現在の境界

Tauriコマンド群はDesktop内部からショー作成・編集・再生をバックエンド呼び出しできる。Remote WebSocketはライブ運用向けのサブセットであり、外部AIからプロジェクト作成・全編集コマンドを直接呼ぶ汎用RPCカタログまではまだ公開していない。将来それを追加する場合もTauriコマンドと同じ検証・Engine経路を再利用し、UI DOM操作を正規APIにしない。

全操作をAI駆動可能にする正式なControl Plane契約とMCP/APIの段階的実装は
`qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`を正とする。MCP、JSON-RPC/REST、
WebSocketは単一の型付きCommand/Query Registryへのアダプタであり、独自の
検証、既定値、再試行、Engine直通経路を持たない。MCPはリアルタイム本体へ
埋め込まず、localhost限定の任意sidecarとして障害・負荷・権限を分離する。
