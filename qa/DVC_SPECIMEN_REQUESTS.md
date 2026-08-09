# Daslight実保存標本の採取依頼（ユーザー実行用）

- 作成: 2026-08-09 Fable（FABLE_HANDOFF_2026-08-09 未解決証拠1〜3の解消手段）
- 背景: 監督/実装エージェントは競合アプリでの保存操作を行わない規律のため、
  実保存`.dvc`標本の作成はユーザー操作でのみ採取できる。
- 対象Daslight: `25.0905.165.111`（監査済みバイナリと同一ビルド。バージョンが違う場合はその旨併記）
- 共通手順: **Shinkan2026等の実ショーは使わず**、新規の使い捨てプロジェクトで作成し、
  `C:\Users\kouty\Documents\Daslight 5\Projects\specimens\` （無ければ任意の場所）へ保存。
  完了後、ファイルパスをFableセッションへ伝えるだけでよい（以後の解析・golden化はこちらで行う）。

## 標本1: 対象ありVALUE FX（`ValueTargeted.dvc`）

1. 新規プロジェクトに適当な灯体（できればマルチビームのBar系）を1〜2台パッチ。
2. シーンを作成し、灯体/ビームを**選択した状態**でVALUE FXの`Rainbow`を追加
   （Selected beamsに選択が入っていること）。
3. 保存。→ これで「BEAMSが空でない実保存VALUE FX」のserializer形が確定する。

## 標本2: VALUE Sweep実保存golden（`Sweep625.dvc`）

1. 同様にシーンへVALUE FXの`Sweep`を追加。
2. シーンを2つ作り、片方は`Direction change`OFF、もう片方はONで保存。
3. **もしSweepのエディタに`Transform`（反転/対称）系の操作が見えるなら**、
   それをONにした3つ目のシーンも追加してほしい——現在fail-closedにしている
   `Transform=1`の実保存証拠になり、扱いを実データで決められる。

## 標本3: COLOR MAPPINGS実保存標本（`ColorMappings.dvc`）

1. マトリクス/マルチビーム灯体を選択した状態で`COLOUR MAPPINGS`系FXを追加
   （どのジェネレータでも可。複数シーンで別ジェネレータを入れると更に良い）。
2. 保存。→ family/type `5/3` の実保存bodyが初めて確定する。

## 標本4: Move FXのBEAMID>0（`MoveBeamId.dvc`）

1. マルチビームのムービング系灯体を1台パッチし、**一部のビームだけ**を選択。
2. その選択でMOVE FX（Circle等）を追加して保存。
   → BEAMS内に`BEAMID>0`エントリを持つ実保存Move rackが得られる。

## 採取後の処理（Fable/Codex側）

各標本はXML監査→既存インポータとの突合→golden回帰テスト化
（`dvc_local_golden_*`系のファイル存在ゲート式）→未証明でfail-closedにしている
変種の解放判断、の順で処理する。標本が無い間は該当変種をSkipped/fail-closedの
まま維持する（FABLE_HANDOFF_2026-08-09の規律どおり）。
