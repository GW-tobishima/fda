# PoC-4 daily-grade-poll Review Packet

## Summary

PoC-4のcatch-up運用として、既存 `run_state.json` のSlack threadをpollingし、未処理の人間返信を採点してSlack threadへ返す `daily-grade-poll` commandを追加した。

Socket Mode listenerを長時間維持できない場合でも、PC復帰後や手動再実行時に `conversations.replies` から返信を取り直して追いつける。

## Scope In

- `scripts/aicx_study_fixture.py daily-grade-poll`
- `run_state.slack.channel_id` / `run_state.slack.thread_ts` に基づくthread取得
- 親投稿、bot投稿、subtype付きmessage、処理済みreply_event_idの除外
- 未処理返信を既存 `process_slack_reply_event_with_run_state` に渡す
- `slack_thread_poll_receipt.json` / schema / example
- `no_reply_found`、`duplicate_reply_skipped`、`invalid_reply_found`、`reply_found_graded_and_sent` のreceipt化
- Slack API failureの `blocked_missing_env_or_sdk`、`thread_not_found`、`rate_limited`、`poll_failed` への分類
- runbook、runner_explanation、artifact_inventory更新

## Scope Out

- 04:30 late dispatch / catch-up dispatch
- `daily-maintain`
- `daily-status`
- 長時間daemon / Socket Mode再接続管理
- PDF/OCR ingest
- LLMによる問題生成

## Verification

- `python3 -m unittest discover -s tests`
  - 30 passed
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out /tmp/poc4-daily-grade-poll-validation.json`
  - 54 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 passed
- `git diff --check`
  - pass

## Live Slack Status

実Slack APIへの `conversations.replies` 呼び出しは、このPRでは未実行。テストでは `--thread-messages-fixture` でSlack APIレスポンス相当のmessagesを渡し、polling境界、採点、invalid reply、duplicate skipを検証した。

live実行例:

```bash
.venv/bin/python scripts/aicx_study_fixture.py daily-grade-poll \
  --run-state data/study_bot/runs/live/YYYY-MM-DD/run_state.json \
  --out-dir data/study_bot/runs/live/YYYY-MM-DD \
  --mode live \
  --env-file .env.local
```

## Review Focus

- `daily-grade-poll` のstatus名とrunbook上の説明が運用判断に十分か
- 既に `graded=true` のrunを即duplicate skipする挙動でよいか
- invalid reply後に、別の新しい未処理返信で再採点できる設計でよいか

