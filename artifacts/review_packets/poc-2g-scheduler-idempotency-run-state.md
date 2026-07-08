# PoC-2G Review Packet: Scheduler / Idempotency / Run State

## 目的

AICX学習Botを「毎朝4:30に10問をSlackへ送り、返信が来たら1回だけ採点返信する」運用に近づけるため、長時間daemonではなく、cron等から呼べる単発commandと `run_state.json` を追加する。

## 変更範囲

- `scripts/aicx_study_fixture.py`
  - `daily-dispatch` subcommandを追加
  - `daily-grade` subcommandを追加
  - 04:30 JSTのdue判定を追加
  - `run_state.json` に送信・返信・採点・採点返信のstepを保存
  - 同じrun_stateで二重dispatch / 二重gradingをskip
  - Slack outbound送信失敗時も `slack_delivery_receipt.json` を残す
  - `slack-smoke --env-file` を追加
- `docs/standards/delivery-artifacts-v0/schemas/`
  - `run_state.schema.json` を追加
  - `slack_delivery_receipt.schema.json` に `send_failed` を追加
  - `artifact_inventory.schema.json` に `daily_run_state` を追加
  - `runner_explanation.schema.json` にPoC-2G phaseを追加
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/`
  - example artifactを10問版に更新
  - `run_state.json` を追加
  - `artifact_inventory.json` / `runner_explanation.json` をPoC-2G境界へ更新
- `artifacts/runs/poc-2g-aicx-study-bot/`
  - validation report
  - daily-run dry-run evidence

## 非対象

- 長時間daemon
- Socket Mode listenerの常時再接続管理
- 複数ユーザー対応
- adaptive出題
- PDF/OCR ingest
- LINE連携

## 受入条件対応

- 04:30 JSTに1回だけ送信される
  - `daily-dispatch --now 2026-06-21T04:29:00+09:00` は `not_due`。
  - `daily-dispatch --now 2026-06-21T04:30:00+09:00` はdispatch境界を通す。
- 二重送信しない
  - 同じ `run_state.json` で2回目の `daily-dispatch` は `duplicate_dispatch_skipped=true`。
- 二重採点しない
  - 同じreply eventで2回目の `daily-grade` は `duplicate_grading_skipped=true`。
- 失敗時は理由が残る
  - Slack outbound失敗は `slack_delivery_receipt.json` に `blocked_missing_env_or_sdk` または `send_failed` を残す。
  - grading失敗は `run_state.failure` と `slack_grading_delivery_receipt.json` に残す。
- 前回runとの差分が分かる
  - `runner_explanation.json` にPoC-2Fとの差分、現在phase、次action、automation boundaryを記録。

## 検証

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2g-aicx-study-bot/validation_report.json`
  - 結果: `50 passed, 0 failed, 4 skipped`
- `cargo run -- validate-artifacts --artifacts artifacts/runs/poc-2g-aicx-study-bot/daily-run --out /tmp/poc-2g-daily-run-validation-report.json`
  - 結果: `40 passed, 0 failed, 13 skipped`

## 主要成果物

- `docs/standards/delivery-artifacts-v0/schemas/run_state.schema.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/run_state.json`
- `artifacts/runs/poc-2g-aicx-study-bot/daily-run/run_state.json`
- `artifacts/runs/poc-2g-aicx-study-bot/validation_report.json`

## 残リスク

- `daily-dispatch` / `daily-grade` は単発commandであり、daemonではない。実運用ではcron/systemd/launchdなどの外部schedulerから呼ぶ。
- `daily-grade` は `slack_reply_event.json` を入力にする。Socket Mode live listenerとrun_stateの直結は次段階で扱う。
- dry-runではSlackに実送信しないため、liveの二重送信防止は `run_state.json` を同じ保存先で運用することが前提。
