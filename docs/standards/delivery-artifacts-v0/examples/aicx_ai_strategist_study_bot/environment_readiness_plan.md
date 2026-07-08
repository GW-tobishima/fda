---
artifact_type: environment_readiness_plan
version: v0
status: reviewed
---

# AICX AI ストラテジスト学習 Bot Environment Readiness Plan

## 0. Metadata

- Document ID: ENV-AICX-STUDY-BOT-001
- Version: v0
- Owner: forge-delivery-agent
- Status: reviewed
- Related Program: PROGRAM-AICX-STUDY-BOT-001
- Related Epic: EPIC-AICX-STUDY-BOT-POC-001

## 1. Readiness Goal

ローカル Codex CLI を実行 actor として、毎朝 4:30 Asia/Tokyo に問題を生成・送信し、回答後に採点と推奨 PDF ページを返せる最小環境を準備する。

## 2. 決定済み PoC 経路

初期 PoC は Slack-first に決定した。理由は、ローカル常駐プロセスが Slack Bot token で送信し、チャンネル履歴またはスレッド返信を polling すれば、外部公開 webhook を立てずに回答取り込みまで検証できるため。

LINE-first は、返信取り込みに Messaging API webhook が必要になる。完全ローカル運用なら Cloudflare Tunnel、ngrok、または小さな relay が必要で、PoC の環境準備が増える。

## 3. 必須チェックリスト

| Area | Check | Ready 条件 | Blocker |
|---|---|---|---|
| Runtime | Codex CLI がローカルで起動できる | `codex` または project wrapper が scheduler から実行できる | yes |
| Timezone | scheduler host の timezone | Asia/Tokyo で 4:30 に発火する | yes |
| Source PDF | PDF source | local-only copy `data/study_bot/source/pdf/aicx_ai_agent_strategist_official_text.pdf` を参照する | no |
| Text extraction | PDF ページ抽出 | page number と text chunk が対応する | yes |
| Topic map | 学習計画 | 日付、ページ範囲、topic、問題数が machine-readable | yes |
| Slack outbound | Slack 送信 | Bot token または incoming webhook が使える | yes |
| Slack inbound | Slack 回答取り込み | channel history / thread polling 権限がある | yes |
| LINE outbound | LINE 送信 | 後続 adapter 拡張で扱う | no |
| LINE inbound | LINE 回答取り込み | 後続 adapter 拡張で扱う | no |
| Secrets | secret 管理 | token は env または local secret file のみ | yes |
| Storage | local run log | 問題、回答、採点結果を repo 外または ignored path に保存 | yes |
| Safety | PDF 本文保護 | 長文抜粋を通知・ログ・外部 API に送らない | yes |
| Answer format | 回答形式 | A-D 選択式を採点対象にする | no |
| Schedule | 通知時刻 | 毎日 4:30 Asia/Tokyo に発火する | yes |

## 4. 予定ディレクトリ

```text
data/study_bot/
  source/
    pdf/                 # git ignore 前提
      aicx_ai_agent_strategist_official_text.pdf
    extracted_pages/     # page-level text or OCR output
  plans/
    study_schedule.json
    topic_map.json
  runs/
    2026-06-21/
      question_set.json
      delivery_receipt.json
      answers.json
      score_report.json
```

実装時は `data/study_bot/` を git 管理対象にしない。schema や fixture だけを repo に置く。

## 5. Secrets

| Name | 用途 | 保存方針 |
|---|---|---|
| `SLACK_BOT_TOKEN` | Slack送信・thread返信 | env または `.env.local` |
| `SLACK_CHANNEL_ID` | 送信先 channel | env または `.env.local` |
| `SLACK_APP_TOKEN` | Socket ModeでSlack返信を受信 | env または `.env.local` |
| `STUDY_BOT_PDF_PATH` | PDF source path | env または local config |

`.env.local` を使う場合は git ignore 対象にする。

## 6. Scheduler Plan

- Primary: systemd user timer。
- Fallback: cron。
- Trigger: `毎日 04:30 Asia/Tokyo`。
- Idempotency key: `daily-quiz:<yyyy-mm-dd>:<channel>`.
- Duplicate prevention: 同じ idempotency key の delivery_receipt がある場合は再送しない。
- Manual dry-run: 指定日を渡して問題セット生成と mock delivery を確認する。
- PoC-2Iの実運用手順は `daily_operation_runbook.md` に集約する。Windows環境ではTask SchedulerからWSL上の `daily-dispatch` を呼ぶ例を使う。

## 7. PDF / Content Readiness

- テキスト PDF なら `pdftotext` または PDF parser で page-level text を作る。
- scanned PDF なら OCR が必要。OCR 品質が低い場合は page topic map を手動作成する。
- 問題生成 prompt には対象ページの要約または短い chunk だけ渡す。
- 通知には PDF 本文の長文抜粋を含めず、`出典: p.258-263` のようなページ参照を中心にする。

## 8. Dry-run Gates

| Gate | Evidence | Pass 条件 |
|---|---|---|
| GATE-ASB-001 Source Ingest | extracted page index | 対象ページ範囲が page ID と topic に紐づく |
| GATE-ASB-002 Quiz Generation | question_set.json | 30 問、source_pages、answer_key、rationale が揃う |
| GATE-ASB-003 Delivery | delivery_receipt.json | mock または Slack sandbox に送信できる |
| GATE-ASB-004 Grading | score_report.json | fixture answer から正答率と推奨ページが出る |
| GATE-ASB-005 Safety | content review note | secret と PDF 本文が artifact に混入していない |

## 9. Go / No-Go

Go 条件:

- PDF source が local-only path に存在している。
- 初期チャネルが Slack-first として決まっている。
- 回答形式が A-D 選択式として決まっている。
- 通知頻度が毎日 4:30 Asia/Tokyo として決まっている。
- token を local secret として渡せる。
- mock delivery で 30 問の message split と回答フォーマットを確認済み。

No-Go 条件:

- PDF の利用権限が不明、または抽出した本文を外部送信する必要がある。
- Slack token または channel polling 権限がない。
- token を repo に置く必要がある運用しかない。
- 問題生成に必要な教材 page mapping が作れない。
