# FDA Mission Control UI/UX 設計（`fda ui`）

作成日: 2026-07-09
対象: GW-tobishima/fda フォーク。V1.5 WS-4（`docs/v1/fda_v1_next_phase_v1_5.md`）の先行実装。

## 1. 位置づけと非ゴール

FDA V1 の HDP-006 は「V1 UI は local HTML の Output Hub / Decision Inbox / status artifact に絞る」
と定めている。本設計はこれを **置換せず**、複数 run を横断して「人間が次に判断・確認すべきこと」を
1 画面に集約する **read-only local projection** を追加する。

守る原則（`docs/standards/delivery-artifacts-v0/uiux_mission_control_design.md` /
status report §6.13 と同方向）:

- **UI は正本ではない。** 正本は artifacts（FDA）・ATO（状態/証跡）・GitHub（コード/PR）。
  UI は毎リクエスト時にディスクの artifact を読み直す純粋な投影で、何も書き込まない。
- **主入口は CLI。** UI から実行・承認・merge は一切できない。UI が提示するのは
  「次に叩く CLI コマンド」（resume command）である。
- **Human Decision と AI Repair を混ぜない。** 人間レーンと AI レーンを別セクションにする。
- **Task / Run / Agent 粒度を主表示にしない。** 成果物・未解決判断・リスク・次アクション・
  最終結果を主役にする。
- **`fda status` と同じ真実を使う。** phase / 判断 / QA / merge の判定は
  `application::status` の同一ロジックを run ごとに呼んで得る（UI 専用の再実装をしない）。

非ゴール（V1.5 以降）: 常駐 Web サービス化、認証、リモート公開、UI からの decide/merge、
ATO / GitHub API の live 取得。

## 2. 利用者とユースケース

| 利用者 | ユースケース |
|---|---|
| 人間（依頼者/承認者） | 「今どこで止まっていて、私は何に答えればよいか」を 10 秒で把握する |
| 人間（レビュー担当） | run ごとの receipt / validation / PR URL に 1 クリックで到達する |
| current AI CLI（Claude Code） | `fda open` の run 単体 HTML より広い、全 run 横断の状態確認 |

## 3. 情報設計（画面構成）

単一ページ。上から重要度順。

```text
┌ ヘッダ: FDA Mission Control / repo 名 / 生成時刻 / [read-only projection] バッジ
├ サマリ行: 実行中 run 数 / 未解決 Human Decision 数 / AI Repair 中 / merge 待ち
├ ① Decision Inbox（人間レーン・最優先）
│    run / 判断ID / 判断 type（小表示） / 要約 / 期限ゲート(required_before) /
│    推奨オプション / resume command（コピー用 <code>） /
│    precedent（過去の同 type + 署名類似判断の折りたたみ。答え・誰が・帰結・一致理由） /
│    適用可能な委任契約の outline バッジ「DC-xxx 適用可」+ 常時可視「（提案・自動適用なし）」
├ ② AI Repair Lane（AIレーン）
│    run / repair状態 / 失敗分類 / retry n/limit / 次コマンド
├ ③ Epic 進捗（epic_progress_state.json 最新 1 件の投影。無ければセクション非表示）
│    冒頭に advisory（非権威の明文）を必ず表示。PR ごとの status バッジ列 + summary。
│    目的: Epic 全体の進捗を 1 目で見る（実装開始許可・merge 承認・merge の証明ではない）
├ ④ Runs（run カード一覧。判断待ち → repair → エラー → 前進可能 → その他 → 完了 の優先順）
│    アクティブな run（判断待ち・repair・エラー・前進可能）のみ常時表示。
│    完了・その他は「完了・その他の run (N件)」の折りたたみ（既定閉）に送り、
│    下段の道場・庭師への到達距離を短くする。
│    phase バッジ / phase 理由 / QA・merge 状態 / PR リンク /
│    次アクション / 主要成果物へのリンク（/artifact 経由）
├ ⑤ 道場（判断の振り返り）
│    回答済み判断 → その後（run の帰結）の時系列テーブル（新しい順・上限 50 件。
│    超過時は冒頭に「最新 N 件を表示中（全 M 件）」を明示）。
│    目的: 人間が自分の判断の帰結と向き合い判断力を育てる（良い判断も痛い判断も同列）。
│    帰結は判断単位ではなく run 単位の投影である（因果を主張しない）。
│    同一 run に複数判断がある場合は「run 内 N 判断で共通」と各行に注記する。
│    契約適用は「authority + 契約適用 DC-xxx（塗りバッジ）」で表示し、
│    inbox の「適用可」outline バッジと視覚区別する（生の decided_by 文字列は出さない）。
├ ⑥ 庭師（棚卸し docket）
│    gc_docket.json があれば候補テーブル（run / 理由 / 推奨 / needs_human バッジ）。
│    無ければ「docket なし（fda gc で生成）」。fda gc は削除・変更をしない旨を常に併記。
│    目的: stale / 不整合 run の例外だけを人間に提示する
└ フッタ: 正本の所在（artifacts・ATO・GitHub）と「UI からは何も変更できない」注記
```

配色・状態語彙（badge）:

| phase / 状態 | 色 | 意味 |
|---|---|---|
| human_turn / waiting_for_decision | 琥珀 | 人間の判断待ち（最優先で目立たせる） |
| repair_planned / qa_failed | 赤 | AI repair へ戻す |
| ready_for_* / merge_ready | 青 | 次コマンドで前進できる |
| merged / *_complete | 緑 | 完了 |
| blocked / adapter_unavailable | 灰 + 赤枠 | fail-closed 停止。理由を必ず併記 |

道場・precedent の帰結（outcome）バッジの意味論（phase バッジとは別系統。
run 単位の帰結を表し、判断単位の因果を主張しない）:

| outcome ラベル | 色 | 意味 |
|---|---|---|
| merged | 緑 | run は merge 済み（github_merge_receipt の succeeded / merge_executed で判定） |
| merge_ready | 青 | merge gate 通過・人間の merge 承認待ち |
| human_approval | 琥珀 | merge gate が human_approval_required |
| blocked | 赤 | QA failed または merge gate blocked / adapter_unavailable |
| repair | 紫（専用色 `--outcome-repair`・ライト/ダーク両対応） | run で repair が発生した「過去の痛い帰結」。琥珀（今の人間待ち）と視覚分離する |
| pending | 灰 | まだ帰結が出ていない |

precedent の一致判定は「decision type 一致 + summary 正規化署名の完全一致 / 接頭辞一致」で、
一致理由（完全一致 / 接頭辞一致）を各 precedent に小表示する。
既知の限界: 同型の定型文 summary（テンプレート文言）は署名が常時一致するため、
precedent は根拠（type + 署名）を添えた参考情報であり、判断の代行・自動適用ではない。

アクセシビリティ / 実装制約: 外部 CDN・フォント・JS ライブラリなし（オフライン完結）、
システムフォント、`prefers-color-scheme` でライト/ダーク両対応、日本語 UI。
15 秒ごとの自動再読込（meta refresh）で「開きっぱなしダッシュボード」用途に耐える。

## 4. ランタイム設計

```text
fda ui [--artifacts-root artifacts/runs] [--repo-root .] [--port 4870] [--open]
```

- `src/infra/ui_server.rs`: std::net::TcpListener の最小 HTTP/1.1 サーバ
  （127.0.0.1 bind 固定・依存クレート追加なし・逐次処理）。
- ルート:
  - `GET /` … Mission Control HTML（毎回スナップショット再構築）
  - `GET /api/state.json` … スナップショット JSON（機械可読。将来の外部 viewer 用）
  - `GET /artifact/<run>/<file>` … run 配下 artifact の raw 表示
    （`..`・絶対パス・サブディレクトリ越えを拒否する path traversal ガード付き）
- `src/application/ui.rs`: run 一覧を走査し、run ごとに `application::status::status()` を
  呼んでスナップショット（JSON Value）を構築する。
- `src/rendering/mission_control.rs`: スナップショット → HTML の純関数（HTML エスケープ必須）。
- module 境界: application → infra は allowlist 済み import のみ、rendering は純関数、
  server（I/O）は infra。`scripts/check_architecture_boundaries.py` に allowlist を追記する。

## 5. セキュリティ / fail-closed

- bind は 127.0.0.1 固定。外部公開しない。
- artifact 配信は runs root 直下の `<run>/<file>` 1 階層のみ。パス正規化後に
  runs root 配下であることを検証する。
- スナップショット構築に失敗した run はエラーとして UI に表示し、握りつぶさない。
- 書き込み系エンドポイントは存在しない（POST 等は 405）。

## 6. 受入条件

1. `fda ui` で起動し、ブラウザで `/` を開くと全 run の phase・未解決判断・次アクションが見える。
2. Decision Inbox に `fda decide` の resume command が表示され、コピーして実行できる。
3. Human Decision と AI Repair が別セクションに分離されている。
4. `/artifact/<run>/<file>` で receipt / validation_report 等の原文に到達できる。
5. `..` を含むパスは 404/400 で拒否される。
6. cargo test / fmt / architecture gate / validate-artifacts がすべて pass。
7. UI からいかなる状態変更もできない。
