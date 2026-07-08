---
artifact_type: human_input_spec
version: v0
status: reviewed
---

# AICX AI ストラテジスト学習 Bot Human Input Spec

## 0. Metadata

- Document ID: HIS-AICX-STUDY-BOT-001
- Version: v0
- Owner: forge-delivery-agent
- Status: reviewed
- Source: user request on 2026-06-21
- Related Program: PROGRAM-AICX-STUDY-BOT-001
- Related Epic: EPIC-AICX-STUDY-BOT-POC-001

## 1. 入力メモの要約

ユーザーは AICX 協会の AI ストラテジスト資格取得に向けて勉強しているが、毎週読む必要があるページ数を消化できていない。学習を継続しやすくするため、毎朝 4:30 に実践的な問題を約 30 問 Slack または LINE で送り、ユーザーが回答したら正答率と読むべき PDF ページを返す Bot を求めている。

Codex CLI は常にローカルで動いている前提としてよい。

## 2. 正規化した要求

- 毎朝 4:30 Asia/Tokyo に当日の学習範囲から 30 問程度を生成する。
- 問題は実践的な形式にする。単純な用語暗記だけでなく、業務設計、RAG、法務、KPI、ワークフロー分岐などの判断問題を含める。
- 問題送信先は Slack とする。
- ユーザーの回答を受け取り、採点結果、正答率、弱点領域、推奨復習ページを返す。
- PDF ページと問題、解説、弱点判定を紐づける。
- 回答形式は A-D 選択式を初期 PoC の既定にする。
- ローカルの Codex CLI を問題生成・解説生成の実行 actor として使う。

## 3. 入力された学習計画

| 期間 | 読む・復習する範囲 | 週末の理解度テスト |
|---|---|---|
| 期間未指定の初期範囲 | BPR、As-Is/To-Be、HTA、ECRS、データ、RAG、法務、成功定義、ワークフロー導入部。p.93-254 中心。 | 業務設計、RAG、法務、成功定義、トリガー/アクションから 20-30 問。 |
| 2026-06-17 から 2026-06-23 | p.258-368。条件分岐、コンテキストエンジニアリング、組織設計、KPI、チェック関連。 | 第 1 回模試。全範囲から 40-60 問。弱点ページを特定。 |
| 2026-07-01 から 2026-07-05 | p.93-132、p.133-183、p.217-228、p.258-288、p.312-327、p.351-400 を横断復習。 | 第 2 回模試・受験日判断。90% 以上なら 2026-07-10 本命確定、85% 未満なら 2026-07-17 検討。 |
| 2026-07-06 から 2026-07-10 | 直前仕上げ。5D、KPI、ワークフロー分岐、RAG、リスク分類のみ。 | 試験前は軽め。新規学習なし。5D 順序、KPI、リスク分類だけ確認。 |

## 4. すぐ作れる範囲の判断

Slack-first の PoC なら小さく作れる。Slack はローカル常駐プロセスから Bot token で送信し、チャンネル履歴またはスレッド返信を polling すれば、外部公開 webhook なしで回答取り込みまで実装しやすい。

LINE-first の PoC は送信自体は可能だが、返信取り込みに公開 HTTPS webhook または tunnel/relay が必要になるため、完全ローカル前提では環境準備が重い。

2026-06-21 の人間判断により、初期 PoC は Slack-first とする。LINE は後続 adapter 拡張候補に残す。

## 5. 制約

- PDF は local-only path `data/study_bot/source/pdf/aicx_ai_agent_strategist_official_text.pdf` にコピー済み。`data/study_bot/` は git 管理対象外とする。
- PDF のテキスト抽出可否、ページ番号と抽出テキストの対応は未確認。
- PDF の著作権上、長い本文抜粋を Slack/LINE や外部モデルへ送らない。問題文は生成物として短くし、出典ページ番号だけを付ける。
- Slack token は repo に保存しない。
- 毎日 4:30 の scheduler は Asia/Tokyo timezone を前提にする。
- Codex CLI がローカルで常駐または定期起動できることを前提にする。

## 6. Human Decisions

| ID | Decision | Selected | Options | Required Before |
|---|---|---|---|---|
| HDP-ASB-001 | 初期通知チャネルを選ぶ | Slack-first | Slack-first, LINE-first, adapter 抽象だけ作って後で選ぶ | 通知・返信取り込みの実装 |
| HDP-ASB-002 | PDF の利用方法を確定する | local-only PDF copy | ローカル PDF を使う, OCR 済みテキストを使う, ページ範囲だけ手動管理する | ページ索引作成 |
| HDP-ASB-003 | 回答形式を決める | A-D 選択式 | A-D 選択式, 短答式, 混合式 | 採点器の実装 |
| HDP-ASB-004 | 毎朝 4:30 の運用ポリシーを決める | 毎日 4:30 | 毎日送る, 平日のみ送る, 失敗時だけ再送する | scheduler 有効化 |

## 7. PoC-0 の出力

- Human Input Spec
- Requirements Definition
- Epic Delivery Plan
- Environment Readiness Plan
- ATO Task Graph
