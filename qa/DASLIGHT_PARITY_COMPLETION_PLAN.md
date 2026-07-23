# Syndocal Lighting Product Completion Plan

- 作成日: 2026-07-23
- 基準線: `adcb402` (`codex/syndocal-v1.0`)
- 目標: SyndocalをDaslight 5と同一タスクで比較可能な照明ソフトウェアとして完成させる
- 3D方針: **内蔵3Dビジュアライザーは実装対象外**。Art-Netを外部ビジュアライザーへ送信して確認する
- 規律: 内部実装量を競合同等性へ読み替えない。操作手数、ネイティブ実データ、実出力証拠が無い軸は未計測のまま扱う

## 完了判定（2026-07-23）

**T18-T23のソフトウェア計画は完走。** 内蔵3Dを持たないPCソフトという製品境界で、独立FX、Scene Live / Live Mixer、灯体導入、名前付きdetachable workspace、full / partial operator lock、Shinkan実データ、独立プロセスArt-Net受信、操作手数、Rust/frontend/viewport/native gateを閉じた。これは「Daslight 5の全機能・商用catalog・ハードウェア・実績まで同等/超過」という宣言ではない。Daslight側未計測10タスク、実LAN node/灯体/controller、live GDTF Share account、standalone hardware write、会場/operator rehearsalは外部/競合証拠として残す。

## 1. 完走の定義

次をすべて満たした時点をソフトウェア完走とする。

1. 主要な照明作業を同一タスクで計測し、重大な操作手数劣位を残さない。
2. Colour / Chaser / Move / Value / Curve / Mappings / Colour Mappingsを、表示上の別名ではなく独立した保存body、validation、runtime、editorとして提供する。
3. Cue所有FXのパラメータ遷移と、Scene Liveのscene単位speed / size / phase / direction / segment / flash、およびLive Mixerのgroup単位strobe / soloを演奏中に直接操作できる。
4. GDTF Shareを単なる検索・download経路で終わらせず、favorites、offline cache、profile health、missing-profile repair、common-rig packを備えた灯体導入面にする。
5. Stage / Timeline限定ではない名前付きdetachable workspaceと、full / partial operator lockを備える。
6. `.sdc v1`互換、Undo/Redo、project validation、44 Hz予算、JA/EN、viewport、native WebView2を各変更で維持する。
7. 実ショーを読み込み、Art-Net外部ビジュアライザー経路でCue、FX、Scene Live、timelineを確認する。
8. 最終競合監査を現HEADでやり直し、見た目・操作性・見やすさ・機能面を証拠付きで更新する。

対応ハードへのstandalone scene書込みはハードウェア製品開発を要するため、本リポジトリだけで実装済みと扱わない。PCソフト製品として対象外にするか、対応ハード製品を別途作るかを最終判定に明記する。未保有DMX node、灯体、controller、会場による受入も外部依存ゲートとして残し、software PASSへ混ぜない。

## 2. 現HEADで閉じている差

- Scene Matrix主体のControl、統一workspace shell、内部スクロールと可変splitter
- source-linked Scene Block、layer、loop/jump、Super Scene child timeline、tempo conform
- independent Colour / Chaser / Move / Value / Curve / fixture-order Mapping / 2D Colour Mapping runtimeとeditor
- cue-owned FX params、selective Effect Recall、Effects Only
- editable project-saved Touch surfaceとLAN remote反映
- Scene Live speed / size / phase latch、flash momentary
- Scene Live direction / segment、Live Mixer group dimmer / strobe / solo
- GDTF import/cache、GDTF Share検索/download、custom fixture builder
- faceted Share検索、favorites、検証済みoffline cache/health、exact-layout repair、common-rig pack
- 名前付き7-pane detachable workspace、monitor placement復元
- credential-safe full / partial operator lock
- Art-Net / sACN / serial DMX、RDM、MIDI/OSC/WebSocket、active/standby基盤
- Shinkan2026 native import（41灯体/30キュー）から別プロセスArt-Net monitorへの512ch動的出力

## 3. ソフトウェア完走後に残る競合・外部差

### P0 操作性と証拠

- T18差分でSyndocal側を13タスクへ拡張したが、Daslight同一タスク実測は3件のみ。
- patch、static programming、FX target適用、native show保存/再開はoperation-count未計測。
- 4軸の最終再判定は完了したが、未計測10タスクを競合同等へ数えない。

### P1 FX

- Cue所有paramsはactivationごとの独立runtimeで、同一Effect IDのCue List GO間に任意の0–600000 ms transitionを持てる。Timeline Scene Blockは重複activationとblock fadeの別モデル。
- GDTF CIE xyY metadataを持つ3–16 emitterのcalibrated mixingをcommand/rebuild時に固定anchorへcompileする。metadata無しのAmber/Lime/UVは名前から推測せずzero、UVのColor省略も可視色へ合成しない。
- Colour Mappingのimage/text/videoは最大64×64・64埋め込みフレームへcommand-time変換する。独立Art-Net受信は完了したが、色・方向・UVの実灯体/商用visualizer目視は外部受入に残る。

### P2 Scene Live / Live Mixer

- software実装はT20-A/Bで完了。scene単位speed / size / phase / direction / segment / flashと、別境界のLive Mixer group dimmer / strobe / soloを直接操作できる。
- DVC import済み実ショーでCue/動的FX/Scene Live speed x4/retrigger x1と外部プロセスArt-Net受信を完了。direction/segment/flash/group strobeは専用runtime/browser gate済みで、実灯体受入だけを外部に残す。

### P3 灯体導入

- T21 software完了。GDTF Shareのfaceted search、machine-local favorites、検証付きoffline cache、project/cache health、exact-layout missing-profile repair、4件のgeneric verified common-rig packをSetup / Libraryへ統合した。
- 大規模商用catalogそのものの件数では競合しないため、open GDTF経路の品質と復旧性で評価する。Shinkan native / Art-Net出力は完了。live Share accountと大規模catalog比較は外部証拠として残る。

### P4 運用面

- T22で7主要paneの名前付きworkspace、複数window、monitor placement復元、full / partial operator lock、credential-safe policyを完了。
- 128 universeのsoftware tick/UIは通るが、実node一時間soakは外部未完。

## 4. 実施トランシェ

### T18: Operability Baseline

1. operation-count harnessを現行UIへ修復する。
2. Scene MatrixからTimelineへの配置を1 dragで証明する。
3. 最低12タスクへ計測を拡張し、Daslight実測が無い項目は`未計測`として別欄にする。
4. current-HEAD gap matrixを生成し、旧監査の閉じた項目を更新する。

受入: harness fail-closed、対象結果のDOM/engine反映、操作予算回帰、1920x1080 primary + compact containment、JA/EN。

進捗（2026-07-23）: 現行UIへ修復し、13タスクが実CDP gestureと結果反映を含めてPASS。Daslight実測のある3タスクは同数、残る10タスクは`未計測`のまま分離した。current-HEAD監査は`qa/DASLIGHT_CURRENT_HEAD_GAP_MATRIX.md`へ固定。patch / static programming / FX target適用 / native Save-reopenの操作計測は次のT18追加対象として残る。

### T19: Independent FX Completion

- T19-A: Curve protocol/runtime/editor/preset/Cue params
- T19-B: fixture-order Mapping protocol/runtime/editor
- T19-C: Colour Mapping protocol/runtime/editor
- T19-D: Cue parameter morph/fade
- T19-E: calibrated multi-emitter policy（profile metadataが無い場合は推測せずfail-closed）

受入: additive serde defaults、legacy `.sdc` byte-shape、project/preset round-trip、200 fixtures x 64 effects regression、release 44 Hz mixed benchmark、physical acceptance分離。

進捗（2026-07-23）: **T19-A/B/C/D/E完了**。T19-Aは独立Curve、T19-Bはfixture-order Mapping、T19-Cは埋め込みimage/text/video 2D Colour Mapping、T19-DはCue-owned target間transitionを追加した。T19-EはGDTF EmitterのCIE xyYをprofileから保持し、3–16 visible emitterの非負mixingをcommand/rebuild時にRGB cubeの固定anchorへcompileする。44Hz側は4 anchorの固定補間のみで、emitter検索、solver、allocation、文字列正規化を行わない。metadata無しのAmber/Lime/UVとColor無しUVはzeroへfail-closedする。protocol 39/39、GDTF 18/18、engine 392 pass + 1 manual ignore、Tauri 326 pass + 9実機依存ignore、full Rust workspace、5解像度full viewport、2701/2701 localization、frontend buildがgreen。calibrated 64 FX×200灯体releaseは通常p95 7.672 / p99 8.052 / max 9.660ms、全64 FX同時transitionはp95 17.489 / p99 18.119 / max 18.390msで各専用gateを通過した。既存非較正5/8/12ms gateは同一ホストA/Bで旧HEAD p95 5.113ms、新差分p95 5.110msと非増加だが両方とも絶対p95 gateを0.11ms超えたためgreenとは扱わない。詳細は`qa/CUE_EFFECT_TRANSITION_ACCEPTANCE.md`と`qa/CALIBRATED_MULTI_EMITTER_ACCEPTANCE.md`。T23でShinkan動的FXの別プロセスArt-Net受信を完了し、物理/色再現だけを外部に残した。

### T20: Complete Scene Live

T20-Aはscene playback direction / Cue Step segmentをT17と同じcommand-time rebuild方式で追加する。T20-BはLive Mixer group strobeを追加し、既存group soloを同じ演奏面で直接検証する。scene controlとgroup controlを混同しない。44 Hz tickでmap lookup、allocation、direction branchingを増やさない。release/retrigger/load resetと非永続latchを維持する。

進捗（2026-07-23）: **T20-A/B software完了**。Direction（Authored / Forward / Reverse / Bounce）と1-based Cue Step Segment（Auto=0）をadditive optional fieldとしてCue defaultへ保存し、runtime latchは非永続のまま維持した。direction-aware Cue-owned FXはcommand/rebuild時に変換し、手動Cue Stepだけを固定sequenceへ再コンパイルする。Timeline / Super Scene child Timelineはauthored順を維持する。Live Mixerにはgroup Dimmer / Strobe / Soloを直接配置し、Touchにも同じStrobeを追加した。Strobeはcanonical GDTF Shutter/Strobe属性と物理Hz範囲がある灯体だけを対象にし、名前・DMX範囲から推測しない。nested groupは灯体ごとに最大Hzをcommand-time compileし、project loadでOffへ戻す。4 active scenes x 8 steps x 200 fixturesのrelease実測はp95 0.276 / p99 0.340 / max 0.365ms、group strobe 200灯体activeはp95 0.006 / p99 0.010 / max 0.012ms。protocol 41/41、engine 399 pass + 1 manual ignore、Tauri 326 pass + 9実機依存ignore、5解像度のfocused/full viewport、2721/2721 localization、frontend buildがgreen。詳細は`qa/SCENE_LIVE_PLAYBACK_ACCEPTANCE.md`。T23でShinkan native speed x4 / retrigger x1と別プロセスArt-Net変化を完了した。

### T21: Fixture Onboarding

GDTF Share catalogをfaceted search、favorites、offline cache/health、missing repair、verified packへ拡張する。認証情報はprojectへ保存しない。

進捗（2026-07-23）: **T21 software完了**。global/manufacturer/fixture/mode/footprint facets、serviceがoptional metadataを返す場合だけfail-closedで使うrelease/Visualizer-tested/real-life-tested filters、favorites、application-localの検証済みGDTF cacheとidentity-only sidecar、project/cache health、exact identity + exact DMX layoutでfail-closedするprofile repair、4件のgeneric verified common-rig packを追加した。Share credentialはcomponent memory以外へ保存せず、T21による`.sdc` schema追加は無い。downloadは公式public APIのrevision-ID GET契約に合わせた。repairはfixture ID、patch、値、limits、group/Cue参照を維持し、44Hz hot pathではなくcommand-time cacheだけを再構築する。full Rust workspace、engine 400 pass + 1 manual ignore、Tauri 330 pass + 9外部依存ignore、helper 29、5解像度focused/full viewport、2766/2766 localization、490.79 kB main production buildがgreen。詳細は`qa/FIXTURE_ONBOARDING_ACCEPTANCE.md`。Shinkan native / Art-Net出力はT23で完了し、live Share account、native offline rehearsal、実灯体だけを外部に残した。

### T22: Workspace and Operator Policy

名前付きpopout workspace、任意主要pane、monitor placement復元、full/partial operator lock、credential-safe policyを追加する。

進捗（2026-07-23）: **完了、commit `83970ea`**。7主要pane、名前付きworkspaceの保存/切替/削除、window/monitor placement復元、main/pane双方のfull / partial lock、projectにsecretを保存しないPBKDF2 verifier policyを実装。engine 400 pass + 1 manual ignore、Tauri 332 pass + 9外部依存ignore、video 114 pass + 1 GPU ignore、full viewport 232 pass、workspace/operator helper 27、native 7 pane、2802/2802 localization、506.33 kB main buildがgreen。詳細は`qa/WORKSPACE_OPERATOR_ACCEPTANCE.md`。

### T23: Final Acceptance

全Rust/frontend/viewport/native gates、Shinkan実データ、Art-Net外部ビジュアライザー、操作手数、release performanceを実行する。`qa/DASLIGHT_COMPARISON_VERDICT.md`、`qa/LIGHTING_COMPETITIVE_AUDIT.md`、`qa/GOAL_COMPLETION_AUDIT.md`を現HEAD証拠で更新する。

進捗（2026-07-23）: **ソフトウェア受入完了**。Shinkan2026（41灯体/30キュー）をnative WebView2へimportし、`1.1 新宝島`、動的`2.1 Fl-StrobeChase`、Scene Live speed x4、retrigger x1を実行。別プロセスmonitorの最終20秒証拠はArtDMX 880 frame / changed 880 / rejected 0 / stream discontinuity 0 / max gap 25 ms / U0 512 bytes。同一PCの物理NIC自己宛 `192.168.1.34` は受信せず、loopback `127.0.0.1` は即受信したため、ArtDMXではなくsame-host NIC return/bind/firewall経路の差と切り分けた。13 operation-countは全PASS（Daslight実測3件は同数、10件未計測）。full Rust workspace、5解像度full viewport、focused Scene Live 5/5、2802/2802 localization、506.33 kB main production buildはgreen。現HEAD release 64 FX×200灯体はp95 3.801 / p99 4.398 / max 5.291ms、全Cue transition負荷は11.378 / 12.858 / 12.994msで各固定gate内。詳細は`qa/ARTNET_EXTERNAL_VISUALIZER_ACCEPTANCE.md`と最終3監査。

## 5. 共通停止条件

- legacy `.sdc`が同じ入力で意味変更または不要な差分を生む
- 44 Hz hot pathへ未計測の検索、確保、clone、lockを追加する
- browser fixtureだけでnative/physical完了を主張する
- Daslight実測の無いタスクを優位として数える
- 外部Art-Netビジュアライザー方針に反して内蔵3Dへ工数を使う
