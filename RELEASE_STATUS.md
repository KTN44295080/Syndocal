# Syndocal v1.0 Release Status / Next Thread Handoff

Updated: 2026-07-12  
Branch: `codex/syndocal-v1.0`  
Completion commit: `6c3de12`  
Final implementation commit: `1f04fd3`  
Final cross-platform CI: [run 29179218727](https://github.com/Seraf0-org/Rayard/actions/runs/29179218727)

## 結論

Syndocal v1.0は、`COMPLETION_PLAN.md`で定義した**ソフトウェア完成条件を満たしたリリース候補**である。

- 照明、映像、共有タイムライン/キュー/エフェクト、2D Stage Map、`.sdc`保存/復元、外部I/O境界を実装済み。
- Windows/macOS/LinuxでRust、libav、フロントエンド、Tauri、配布物生成とパッケージ起動CIがgreen。
- WindowsではNSISの実インストール/起動/アンインストール、MSIの内容、`.sdc`関連付けを確認済み。
- 1時間ソーク、クラッシュリカバリ、大規模ショー、DMXテレメトリ、複数解像度の一画面UIゲートを通過済み。

ただし、これは「全機材・全会場で検証済み」「署名済み一般公開製品」「今後追加不要」という意味ではない。以下の外部受入と公開運用は、コード完成とは分けて扱う。

## 一般公開・本番投入前の必須確認

### P0: 実機受入

- [ ] Art-Netノードと実灯体で、アドレス、色、Pan/Tilt、ブラックアウト、44Hz継続送信を確認する。
- [ ] sACNマルチキャスト対応ノードで、ユニバースルーティングとネットワークスイッチ越しの安定性を確認する。
- [ ] Enttec USB PRO / DMXKingで長時間送信を確認する。
- [ ] Enttec Open DMXはロジックアナライザでBreak 176us、MAB 16us、フレーム周期を測定する。重要現場ではPRO系を優先する。
- [ ] MIDIコントローラ、TouchOSC、iPad/Android Webリモートを実端末で往復確認し、Wi-Fi遅延を記録する。
- [ ] NDI 6をOBS/Resolume等と送受信し、探索、色、フレームレート、終了処理を確認する。
- [ ] macOS実機とLinux実機で、複数ディスプレイ、フルスクリーン、音声/動画素材、`.sdc`保存再読込を操作確認する。CIはパッケージ起動までである。

結果は `qa/M4_IO_VALIDATION.md` の Acceptance Capture に追記する。loopback結果を実機合格へ読み替えない。

### P0: Webリモートの公開範囲

現行の`RemoteControlConfig`は既定で`0.0.0.0:9100`へバインドし、認証情報を持たない。したがってv1.0のWebリモートは**信頼済みの専用LAN限定**とする。

不特定LAN、会場共有LAN、インターネットへ公開する前に、次を実装する。

- ペアリング用の短期トークンまたはPIN認証
- WebSocket/HTTPのOrigin/Host検証
- 接続数、メッセージサイズ、操作レートの上限
- TLS終端またはローカル専用運用の明示
- 接続クライアント一覧と強制切断UI

### P0: 配布と法務

- [ ] Windows Authenticode署名を行う。
- [ ] Apple Developer ID署名とNotarizationを行う。
- [ ] NDI SDK/runtimeを配布する場合、NewTek/NDIの再配布条件を確認する。
- [ ] FFmpeg LGPL動的リンク、ライセンス本文、第三者通知、対応するソース取得方法を公開成果物で再確認する。
- [ ] `v1.0.0`タグを固定し、Release Notes、SHA-256、NSIS/MSI/DMG/AppImage/debを同一リリースへ掲載する。
- [ ] Windows/macOSのクリーンな別PCで最終成果物をダウンロードから起動まで確認する。

個人利用または限定テスト配布では未署名のまま扱えるが、SmartScreen/Gatekeeper警告を利用者へ説明する。

## 入れる価値が高いv1.1候補

優先順位は次の通り。

1. Webリモート認証と接続制御
2. 操作履歴/Undo・Redo。特にFixture、Cue、Output、Effect削除とマッピング編集
3. Spout入力/出力(Windows)とSyphon入力/出力(macOS)
4. 自動バックアップ世代管理、クラッシュログ、診断パッケージ出力
5. HAP Q Alpha、HAP R/BC7、ISFシェーダー
6. 自動更新または更新通知、リリースチャンネル
7. 日本語UI/ローカライズ。現行UI用語は英語で統一済み
8. ユーザーテンプレート、ワークスペースレイアウト保存、MIDIマッピング共有

3Dビジュアライザは当面本体へ戻さず、既存の`visualizer`データ境界から外部実装へ接続する。標準操作面は2D Stage Mapを維持する。

## 回帰させない条件

- アプリ全体を縦スクロールさせない。一画面デスクを維持し、必要なスクロールは各パネル内に閉じる。
- SetupはLightingとVideoの専用画面を混在させず、タブで切り替える。
- Controlは上段のCue/Transport、左下の2D Stage Map、右下のLive Edit/Timeline/Mixerを維持する。
- 44Hz DMXスレッドに動画処理、ブロッキングI/O、動的アロケーションを持ち込まない。
- Windows/macOS/Linux固有機能は`cfg`/feature境界に置き、Unavailableなバックエンドでアプリ全体を起動失敗させない。
- プロジェクト名はSyndocal、拡張子は`.sdc`、発行者は`Seraf()のKTN`。Rayard/`.ry`/KDMXのユーザ向け表記を復活させない。
- 小変更は対象テスト、マイルストーンだけ全ビルドを行い、テストプロセスを多重常駐させない。

## 次スレッドの開始手順

まず次を確認する。

```powershell
git status --short
git branch --show-current
git log -5 --oneline
pnpm --dir app run check:release
```

参照順:

1. `RELEASE_STATUS.md` - 現在の判定と次の優先順位
2. `COMPLETION_PLAN.md` - v1.0 Definition of Doneと実装証跡
3. `qa/M4_IO_VALIDATION.md` - 外部I/O実機受入
4. `qa/M5_RELIABILITY_VALIDATION.md` - ソーク/回復/負荷
5. `qa/M6_RELEASE_VALIDATION.md` - 配布物/CI/署名判断
6. `CLAUDE.md` - 詳細な実装履歴。古い記述は末尾の新しい記録を優先する

次の作業は、v1.0コードを無目的に拡張するのではなく、次のいずれかを新しい目標として明示してから始める。

- 実機受入と正式リリース
- Webリモートのセキュリティ強化
- v1.1機能開発

