# PR #22 Review Packet: daily-maintain

## Scope

PoC-4 の `daily-maintain` を追加する。

目的は、ローカルPCがsleepして04:30実行を逃しても、PC復帰後に同じ単発commandを何度実行しても安全に追いつける状態にすること。

対象:

- `daily-maintain` command
- `maintenance_receipt.json`
- `maintenance_receipt.schema.json`
- `run_state.artifacts.maintenance_receipt`
- daily operation runbook更新
- AC-01からAC-09の単体テスト

対象外:

- 長時間daemon
- Socket Mode listenerの再接続管理
- `daily-status`
- PDF/OCR ingest
- LLM問題生成

## Behavior

`daily-maintain` は `--date` と `--out-root` から当日run directoryを決め、`run_state.json` を見て次を判断する。

- 04:30前で未送信なら `not_due`
- 未送信なら `daily-dispatch` 相当を実行
- allow-late window内なら `late_dispatched`
- 送信済み未採点なら `daily-grade-poll` 相当を実行
- 未処理返信があれば採点してSlack thread返信
- 返信がなければ `no_reply_found`
- 採点済みなら `already_graded`
- 失敗時も `maintenance_receipt.json` に理由を残す

ユーザー向け標準コマンド:

```bash
python scripts/aicx_study_fixture.py daily-maintain \
  --date 2026-06-23 \
  --out-root data/study_bot/runs/live \
  --env-file .env.local \
  --allow-late \
  --late-window-hours 12
```

## Validation

Passed:

- `python3 -m unittest discover -s tests`
  - 39 tests
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out /tmp/poc4-daily-maintain-validation.json`
  - 56 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 passed
- `git diff --check`
- CLI dry-run smoke:
  - `daily-maintain --mode dry-run --allow-late --late-window-hours 12 --now 2026-06-21T06:30:00+09:00`
  - `maintenance_receipt.status=late_dispatched`

## Review Notes

- `daily-maintain` は既存の `daily-dispatch` / `daily-grade-poll` を再利用し、新しい長時間runtimeは追加していない。
- live Slack APIはこのPRのローカル検証では実行していない。
- live modeでSlack envまたはSDKが不足した場合は `blocked_missing_env_or_sdk` として `maintenance_receipt.json` に残す。
