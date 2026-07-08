# PoC-2I Review Packet: Daily Operation Runbook

## 目的

AICX学習Botを実運用に近づけるため、04:30 JSTのdispatch、Socket Mode single-run listener、run_state保存先、dry-run/live分離、失敗時再実行を日次運用runbookとして固定する。

## 変更範囲

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/daily_operation_runbook.md`
  - `.env.local` の必要項目
  - live / dry-run の保存先規約
  - 04:30 JST dispatch command
  - `socket-reply-listen --single-run --run-state` の実行手順
  - Windows Task SchedulerからWSL上のrepoを起動する実例
  - dry-runとliveのrun_stateを分ける運用ルール
  - dispatch失敗、listener timeout、invalid replyの再実行手順
  - 当日運用チェックリスト
- `environment_readiness_plan.md`
  - PoC-2I runbookへの導線を追加
- `artifact_inventory.json`
  - `daily_operation_runbook` artifactを追加
  - PoC-2I groupを追加
- `runner_explanation.json`
  - current phaseをPoC-2Iへ更新
- schema
  - `artifact_inventory.schema.json` に `daily_operation_runbook` を追加
  - `runner_explanation.schema.json` に `daily_operation_runbook` phaseを追加

## 非対象

- 新しいdaemon
- Socket Mode再接続管理
- scheduler wrapper script
- systemd/cron/launchdの複数実例
- 複数ユーザー対応
- adaptive出題
- PDF/OCR ingest
- LINE連携

## 受入条件対応

- 04:30 dispatchを外部schedulerに載せる
  - Windows Task Schedulerから `wsl.exe bash -lc ... daily-dispatch --mode live` を起動する例を記載。
- `socket-reply-listen --single-run --run-state` の実行手順
  - dispatch後の同じ `RUN_DIR/run_state.json` を渡して、対象threadをrun_stateから解決する手順を記載。
- Windows Task Scheduler / cron / launchd のどれか1つの実例
  - Windows Task Scheduler実例を採用。
- `.env.local` の必要項目
  - `SLACK_BOT_TOKEN` / `SLACK_CHANNEL_ID` / `SLACK_APP_TOKEN` を記載。
- run_state保存先の規約
  - `data/study_bot/runs/live/YYYY-MM-DD/` と `data/study_bot/runs/dry-run/YYYY-MM-DD/` を分離。
- dry-run と live のrun_stateを分ける運用ルール
  - live/dry-run共有禁止を明記。
- 失敗時の再実行手順
  - dispatch失敗、listener timeout、invalid replyごとに確認artifactと再実行方針を記載。

## 検証

- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2i-aicx-study-bot/validation_report.json`
  - 結果: `50 passed, 0 failed, 4 skipped`
- `python3 -m unittest discover -s tests`
- `cargo test`
- `git diff --check`

## 主要成果物

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/daily_operation_runbook.md`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/artifact_inventory.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/runner_explanation.json`
- `artifacts/runs/poc-2i-aicx-study-bot/validation_report.json`

## 残リスク

- Windows Task Scheduler例はdispatchのみ。Socket Mode listenerの常駐・再接続・定期再起動は後続PRで扱う。
- runbookは運用手順であり、scheduler wrapper scriptは追加していない。
- dry-runではSlackに実送信しないため、live運用前に `.env.local` とSlack app権限を別途確認する。
