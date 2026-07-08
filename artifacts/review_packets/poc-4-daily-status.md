# PR #23 Review Packet: daily-status

## Scope

PoC-4 の `daily-status` を追加する。

目的は、`run_state.json` から現在状態、最後のevent、次にやること、主要artifact pathを人間がすぐ確認できるようにすること。

対象:

- `daily-status --run-state <path>`
- `run_state.steps` と `last_event` からの運用状態導出
- `waiting_for_reply`、`graded`、`not_due` などの表示
- Slack channel/thread、last event、next action、主要artifact pathの表示
- 成功した `no_reply_found` poll後に、古い `thread_poll` failureを解除して `dispatched` に戻す小修正

対象外:

- scheduler登録
- Slack API呼び出し
- Socket Mode listener
- adaptive出題
- PDF/OCR ingest

## User Command

```bash
python scripts/aicx_study_fixture.py daily-status \
  --run-state data/study_bot/runs/live/2026-06-24/run_state.json
```

## Output Shape

```text
AICX Study Bot - 2026-06-24

Status:
  waiting_for_reply

Slack:
  sent: true
  channel_id: C...
  thread_ts: ...

Last Event:
  no_reply_found at 2026-06-24 04:36:19 JST

Next Action:
  Slack threadへ回答する。返信済みなら daily-maintain を再実行する。

Artifacts:
  quiz_prompt_md: ...
  run_state: ...
  slack_delivery_receipt: ...
```

## Validation

Passed:

- `python3 -m unittest discover -s tests`
  - 41 tests
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out /tmp/poc4-daily-status-validation.json`
  - 56 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 passed
- `git diff --check`
- CLI smoke against live `2026-06-24/run_state.json`
  - effective status: `waiting_for_reply`
  - thread_ts: `1782243008.159149`
  - next action: Slack threadへ回答、または `daily-maintain` 再実行

## Review Notes

- `daily-status` はSlack APIへ接続しない。
- raw `run_state.status` と導出した運用状態が違う場合は `raw_run_state` を表示する。
- 今日のlive runでは、sandbox DNS失敗の古いfailureが残っていたため、CLI smokeでは `raw_run_state=failed` かつ effective status `waiting_for_reply` と表示された。
- このPRの小修正により、今後は成功した `no_reply_found` pollで古い `thread_poll` failureが解除される。
