# PoC-2F Review Packet: Slack Thread Grading Response Live Send

## 目的

AICX学習BotのPoC-2Fとして、Slack返信採点後に生成された`slack_grading_response.response_text`を元のSlack threadへ投稿し、Slack APIの送信結果をartifactとして残す。不足回答やinvalid replyの場合も、同じthreadへエラー本文を返せる境界を追加する。

## 変更範囲

- `scripts/aicx_study_fixture.py`
  - `slack-grading-send` subcommandを追加
  - `socket-reply-listen --send-thread-response`を追加
  - `run-fixture`の既定問題数は`study_schedule.json`へ委ね、日次Slack運用は10問を標準化
  - `chat.postMessage(thread_ts=...)`で採点結果またはinvalid reply errorをthreadへ投稿
  - `slack_grading_delivery_receipt.json`を生成
  - Slack送信失敗時は`send_failed` receiptを残してfail-closed
  - bot自身のSlack message eventは`bot_id`で無視
- `docs/standards/delivery-artifacts-v0/schemas/`
  - `slack_grading_delivery_receipt.schema.json`を追加
  - `slack_reply_intake_receipt.schema.json`にPoC-2F statusを追加
  - `artifact_inventory.schema.json`に`slack_grading_delivery_receipt`を追加
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/`
  - `slack_grading_delivery_receipt.json`
  - `study_schedule.json`の日次出題数を10問標準へ変更し、必要時は5問×2 batchに分ける方針を追加
  - PoC-2F向け`artifact_inventory.json`と`runner_explanation.json`

## 非対象

- 04:30 JST scheduler
- 二重送信・二重採点防止
- run state永続化
- 長時間daemon
- PDF/OCR ingest
- LINE連携

## 受入条件対応

- `received_and_graded`の`response_text`をthreadへ投稿
  - `slack-grading-send --mode live`で実Slack threadへ投稿済み。
- 不足回答 / invalid replyのerror responseをthreadへ投稿
  - `socket-reply-listen --send-thread-response`でinvalid reply時に形式案内つきerror textを投稿する実装を追加。
  - dry-run receiptとunit testでcontractを確認。
- `slack_response` receiptを保存
  - `slack_grading_delivery_receipt.json`に`ok/channel/ts/thread_ts`を保存。
- 送信成功/失敗をartifactに残す
  - statusは`sent` / `send_failed` / `blocked_missing_env_or_sdk` / `dry_run_ready`。
- 毎日Slackで完結しやすい問題数へ変更
  - 日次標準は10問。
  - 完了しづらい場合は5問×2 batchへ分割する。
  - 既存の30問fixture/live smokeは明示指定互換として維持。

## 検証

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2f-aicx-study-bot/validation_report.json`
- `cargo test`
- `git diff --check`
- 10問デフォルト確認:
  - `.venv/bin/python scripts/aicx_study_fixture.py run-fixture --study-schedule docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_schedule.json --topic-map docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/topic_map.json --question-bank docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/question_bank.fixture.json --date 2026-06-21 --out-dir /tmp/aicx-poc2f-default-10`
  - 結果: `quiz_set.question_count=10`, `quiz_prompt.question_count=10`
- live smoke:
  - `.venv/bin/python scripts/aicx_study_fixture.py slack-grading-send --quiz-set docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json --slack-grading-response artifacts/runs/poc-2e-aicx-study-bot-live-retry-3/slack_grading_response.json --out-dir artifacts/runs/poc-2f-aicx-study-bot-live --mode live --env-file .env.local`
  - 結果: `artifacts/runs/poc-2f-aicx-study-bot-live/slack_grading_delivery_receipt.json` が `status=sent`
- 10問版 live smoke:
  - `artifacts/runs/poc-2f-aicx-study-bot-10q-live/quiz_set.json` と `quiz_prompt.md` を生成。
  - `slack-smoke --mode live`で10問版の朝トレ本文をSlackへ投稿。
  - `slack-grading-send --mode live --thread-ts 1782087536.889139`で同じthreadへ採点結果を投稿。
  - 結果: `artifacts/runs/poc-2f-aicx-study-bot-10q-live/slack_delivery_receipt.json` と `slack_grading_delivery_receipt.json` がどちらも `status=sent`。

## 主要成果物

- `docs/standards/delivery-artifacts-v0/schemas/slack_grading_delivery_receipt.schema.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_grading_delivery_receipt.json`
- `artifacts/runs/poc-2f-aicx-study-bot/slack_grading_delivery_receipt.json`
- `artifacts/runs/poc-2f-aicx-study-bot/validation_report.json`
- `artifacts/runs/poc-2f-aicx-study-bot-live/slack_grading_delivery_receipt.json`
- `artifacts/runs/poc-2f-aicx-study-bot-10q-live/slack_delivery_receipt.json`
- `artifacts/runs/poc-2f-aicx-study-bot-10q-live/slack_grading_delivery_receipt.json`

## 残リスク

- `socket-reply-listen`は単一eventを処理して終了するPoC実装。常駐、再接続、二重処理防止はPoC-2G以降で扱う。
- invalid replyのlive送信は実Slack手入力ではなく、dry-run contractと実装経路で確認している。
- Slack app側の`chat:write`、対象channelのhistory event購読、bot参加は環境前提。
