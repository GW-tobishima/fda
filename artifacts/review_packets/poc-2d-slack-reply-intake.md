# PoC-2D Review Packet: Slack Reply Intake / Grading

## 目的

AICX学習BotのPoC-2Dとして、Slack返信本文をローカルfixtureとして受け取り、30問分のA-D回答に変換し、既存の採点・topic別正答率・推奨ページ生成へ接続する。

## 変更範囲

- `scripts/aicx_study_fixture.py`
  - `slack-reply-fixture` subcommandを追加
  - `reply-intake` subcommandを追加
  - `1:B 2:C ...`、`Q2:C`、`3.D`、`4)A`、`5-A`形式の回答parserを追加
  - Slack返信由来の`answer_submission.json`生成を追加
  - Slack thread向けの`slack_grading_response.json`生成を追加
- `docs/standards/delivery-artifacts-v0/schemas/`
  - `slack_reply_event.schema.json`
  - `slack_grading_response.schema.json`
  - `answer_submission.schema.json`のsource拡張
  - `artifact_inventory.schema.json`と`runner_explanation.schema.json`のenum拡張
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/`
  - `slack_reply_event.json`
  - `slack_grading_response.json`
  - Slack返信由来の`answer_submission.json`
  - PoC-2D向け`artifact_inventory.json`と`runner_explanation.json`

## 非対象

- Slack Events API / Socket Mode / `conversations.replies`によるlive reply取得
- Slackへの採点返信送信
- PDF/OCR ingest
- LINE連携
- 毎朝4:30 scheduler
- 外部LLMによる作問や採点

## 受入条件

- 2026-06-21の`quiz_set.json`に対してSlack返信fixtureを生成できる。
- 返信本文から30問分の回答を抽出できる。
- 不足、重複、範囲外、`quiz_set_id`不一致はfail-closedする。
- `answer_submission.json`を採点できる。
- topic別正答率と推奨ページが出る。
- Slack thread向け返信文をartifactとして生成できる。
- PDF本文なし、Slack APIなしで成立する。

## 検証

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2d-aicx-study-bot/validation_report.json`
- `cargo test`
- `git diff --check`

## 主要成果物

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_reply_event.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/answer_submission.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/grading_report.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_recommendation.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_grading_response.json`
- `artifacts/runs/poc-2d-aicx-study-bot/validation_report.json`

## 残リスク

- 実Slack返信の取得方式は未決定。次PRでEvents API、Socket Mode、または`conversations.replies` pollingのどれを採用するか決める。
- parserはA-D選択式の簡易形式に限定している。自由文、全角英字、複数行の曖昧回答はまだ扱わない。
- `slack_grading_response.json`は送信予定文のartifactであり、Slack投稿は行わない。
