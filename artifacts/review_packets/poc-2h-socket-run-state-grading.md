# PoC-2H Review Packet: Socket Mode Run State Grading

## 目的

AICX学習Botを「Slack threadへの実返信を受けて、その日のrun_stateへ1回だけ採点結果を戻す」運用に近づけるため、長時間daemonではなく、Socket Modeの単発受信と `run_state.json` の結線を追加する。

## 変更範囲

- `scripts/aicx_study_fixture.py`
  - `socket-reply-listen` に `--run-state` / `--single-run` / `--expected-channel-id` / `--now` を追加
  - `socket-reply-intake` に `--run-state` / `--mode` / `--env-file` / `--now` を追加
  - `run_state.slack.thread_ts` から対象threadを解決
  - Socket Mode payloadから `slack_reply_event.json` を作成
  - 返信を採点し、`slack_grading_delivery_receipt.json` でthread返信結果を保存
  - `run_state.json` を `graded` / `invalid_reply` / `failed` へ更新
  - 同一reply eventまたは既にgradedのrunを `duplicate_reply_skipped` としてskip
- `docs/standards/delivery-artifacts-v0/schemas/`
  - `slack_reply_intake_receipt.schema.json` に `duplicate_reply_skipped` / `run_state_thread_missing` / `thread_mismatch` を追加
  - `run_state.schema.json` に `slack_reply_intake_receipt` artifact pathを追加
  - `runner_explanation.schema.json` に `slack_socket_run_state_grading` phaseを追加
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/`
  - example artifactをPoC-2H境界へ更新
  - `runner_explanation.json` / `artifact_inventory.json` をPoC-2Hへ更新
  - `environment_readiness_plan.md` に `SLACK_APP_TOKEN` を追加
- `artifacts/runs/poc-2h-aicx-study-bot/`
  - docs example validation report
  - Socket run_state grading dry-run evidence

## 非対象

- 長時間daemon
- Socket Mode再接続管理
- cron/systemd/launchd/Windows Task Scheduler設定
- 複数ユーザー対応
- adaptive出題
- PDF/OCR ingest
- LINE連携

## 受入条件対応

- daily-dispatch後の `run_state.thread_ts` を使う
  - fixture evidenceでは `run_state.slack.thread_ts=1782074738.865009` を対象threadとして使い、明示 `--expected-thread-ts` なしで `socket-reply-intake --run-state` を通す。
- Socket Mode listenerが対象thread replyを受ける
  - `socket-reply-listen --single-run --run-state ...` がrun_stateから対象channel/threadを解決する。
  - liveでrun_stateがない場合は従来どおり `--expected-thread-ts` 必須。
- `reply_event` artifactを作る
  - `slack_reply_event.json` を生成。
- `daily-grade`相当の採点へ渡す
  - 共通処理で `answer_submission.json` / `grading_report.json` / `study_recommendation.json` / `slack_grading_response.json` を生成。
- grading resultをthreadへ返す
  - `slack_grading_delivery_receipt.json` に `grading_result` または `invalid_reply_error` を保存。
- `run_state`をgradedへ更新
  - 成功時は `run_state.status=graded`、`last_event.status=received_graded_and_sent`。
- duplicate replyをskip
  - 同一reply eventまたは既にgradedのrunは `duplicate_reply_skipped`。

## 検証

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
  - 結果: `21 tests OK`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2h-aicx-study-bot/validation_report.json`
  - 結果: `50 passed, 0 failed, 4 skipped`
- `cargo run -- validate-artifacts --artifacts artifacts/runs/poc-2h-aicx-study-bot/socket-run-state --out artifacts/runs/poc-2h-aicx-study-bot/socket_run_state_validation_report.json`
  - 結果: `42 passed, 0 failed, 11 skipped`
- `cargo test`
  - 結果: `4 passed`
- `git diff --check`

## 主要成果物

- `scripts/aicx_study_fixture.py`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/run_state.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_reply_intake_receipt.json`
- `artifacts/runs/poc-2h-aicx-study-bot/socket-run-state/run_state.json`
- `artifacts/runs/poc-2h-aicx-study-bot/socket_run_state_validation_report.json`

## 残リスク

- dry-run evidenceはSocket Mode WebSocketへ接続しない。live確認は `SLACK_BOT_TOKEN` / `SLACK_CHANNEL_ID` / `SLACK_APP_TOKEN` / `slack_sdk` が揃った環境で `socket-reply-listen --mode live --single-run --run-state ...` を実行する。
- `socket-reply-listen` は1件受信して終了する単発commandであり、常駐daemonや再接続管理はPoC-2I以降で扱う。
- `run_state.json` の保存先を毎日同じ運用ディレクトリにすることが、二重採点防止の前提になる。
