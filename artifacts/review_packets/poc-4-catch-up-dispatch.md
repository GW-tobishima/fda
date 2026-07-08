# PoC-4 Catch-up Dispatch Review Packet

## Summary

PC sleepやWSL停止で04:30 dispatchを逃した場合でも、復帰後に当日分を遅延配信として送れるようにした。

既存 `daily-dispatch` の通常挙動は維持し、`--allow-late` を指定した場合だけ遅延windowとSlack本文の遅延noticeを有効にする。

## Scope In

- `daily-dispatch --allow-late`
- `daily-dispatch --late-window-hours <hours>`
- 04:30前は `not_due`
- 04:30後かつwindow内は送信し、`run_state.last_event.status=late_dispatched`
- Slack本文に「遅れて配信」を追加
- window超過は `run_state.status=late_window_expired` で送信しない
- `slack_delivery_receipt.json` に `dispatch_timing_status` / `scheduled_at` / `late_by_seconds` / `late_window_hours` を追加
- runbook / runner_explanation更新

## Scope Out

- `daily-maintain`
- `daily-status`
- Windows Task Scheduler設定生成
- 長時間daemon
- Socket Mode再接続管理
- 実Slack live送信

## Verification

- `python3 -m unittest discover -s tests`
  - 33 passed
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out /tmp/poc4-catch-up-dispatch-validation.json`
  - 54 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 passed
- `git diff --check`
  - pass

## Review Focus

- `--allow-late` 指定時だけ遅延扱いにする互換方針でよいか
- `late_window_expired` を送信しない正常skipとして扱う方針でよいか
- 遅延notice文言がSlack本文として十分か
