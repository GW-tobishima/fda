# PoC-2E Review Packet: Slack Socket Mode Live Reply Intake Smoke

## 目的

AICX学習BotのPoC-2Eとして、Slack Socket ModeのEvents API payloadを受け取り、thread返信の回答を採点artifactへ接続する。Slack threadへの採点返信送信はPoC-2Fへ分離する。

## 変更範囲

- `scripts/aicx_study_fixture.py`
  - `SLACK_APP_TOKEN` readinessを追加
  - `.env.local`を読み込める`--env-file`を追加
  - `socket-payload-fixture` subcommandを追加
  - `socket-reply-intake` subcommandを追加
  - `socket-reply-listen` subcommandを追加
  - Socket Mode `events_api` payloadからthread返信だけを正規化
  - 正規化した返信を既存の採点・推奨ページ生成へ接続
  - live modeでは実Slack返信を受け、`grading_report.json`と`slack_grading_response.json`まで生成
- `docs/standards/delivery-artifacts-v0/schemas/`
  - `slack_socket_mode_payload.schema.json`
  - `slack_reply_intake_receipt.schema.json`
  - Socket Mode由来sourceを許可するschema拡張
- `.env.example`
  - `SLACK_APP_TOKEN=xapp-...` の設定例を追加
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/`
  - `slack_socket_mode_payload.json`
  - Socket Mode由来の`slack_reply_event.json`
  - Socket Mode由来の`answer_submission.json`
  - `slack_reply_intake_receipt.json`
  - PoC-2E向け`artifact_inventory.json`と`runner_explanation.json`

## 非対象

- 毎朝4:30 JSTのscheduler
- 二重送信防止
- PDF/OCR ingest
- LINE連携
- 外部LLMによる作問・採点
- Slack appの設定自動化
- Slack threadへの採点返信送信

## 受入条件対応

- Slackのthread返信を受け取れる
  - `socket-reply-listen --mode live`でSocket Mode Events APIを待ち受ける実装を追加。
  - live実行には`SLACK_APP_TOKEN=xapp-***`が必要。
- `"1:B 2:C ..."`をparseできる
  - 既存parserをSocket Mode由来eventに接続し、12件のunit testで確認。
- `grading_report.json`が生成される
  - `socket-reply-intake`と`run-fixture`で生成確認。
- `slack_grading_response.json`が生成される
  - Slack threadへ返す本文artifactは生成するが、送信はPoC-2Fに残す。
  - このPRでは`SLACK_APP_TOKEN`未設定のため実Slack受信は未実行。

## 検証

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
- `python3 scripts/aicx_study_fixture.py socket-reply-intake --quiz-set docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json --socket-payload docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_socket_mode_payload.json --out-dir /tmp/aicx-poc2e-socket-intake-check --expected-channel-id C_FIXTURE --expected-thread-ts 1782074738.865009`
- `.venv/bin/python scripts/aicx_study_fixture.py socket-reply-listen --quiz-set docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json --out-dir artifacts/runs/poc-2e-aicx-study-bot --mode dry-run --timeout-seconds 5 --env-file .env.local`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2e-aicx-study-bot/validation_report.json`
- `cargo test`
- `git diff --check`

## 主要成果物

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_socket_mode_payload.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_reply_event.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/answer_submission.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/grading_report.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_recommendation.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_grading_response.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_reply_intake_receipt.json`
- `artifacts/runs/poc-2e-aicx-study-bot/slack_reply_intake_receipt.json`
- `artifacts/runs/poc-2e-aicx-study-bot/validation_report.json`

## 残リスク

- `.env.local`に`SLACK_APP_TOKEN`が未設定だったため、live WebSocket receiveは未検証。
- Slack app側でSocket Mode有効化、app-level token作成、Event Subscriptionsの`message.channels`または対象channelに合うmessage event購読が必要。
- `socket-reply-listen`は単一返信を処理して終了するPoC実装。常駐・再接続・二重処理防止はPoC-2G以降で扱う。
- Slack threadへの採点返信送信と不足回答時のerror responseはPoC-2Fで扱う。
