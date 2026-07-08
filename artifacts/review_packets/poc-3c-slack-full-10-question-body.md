# PoC-3C Review Packet: Slack本文10問化

## Scope

実運用で見つかった仕様ズレを修正する。

対象:

- Slack outbound message の本文に、日次10問すべてを含める
- answer key / rationale は引き続き Slack本文にも message artifact にも含めない
- `slack_outbound_message` schema/example を10問本文に合わせる
- artifact catalog / inventory の「先頭3問preview」表現を更新する
- daily-dispatch dry-run test で `Q10` まで本文に入ることを確認する

対象外:

- 既存の採点ロジック変更
- Socket Mode listener変更
- scheduler/cron登録
- 問題品質改善
- Slack Block Kit化

## Root Cause

PoC-2B の `summary + 先頭3問 preview + artifact path` 仕様が、PoC-3C の「Slack内で毎日10問を解く」運用仕様に残っていた。

`quiz_prompt.md` には10問分が生成されていたが、Slackへ送る `message_text` は `preview_count=3` の先頭3問だけを本文に含めていた。

## Expected Behavior

`daily-dispatch` の既定10問運用では、Slack本文に以下が入る。

- 範囲
- 今日の問題数
- Bank/Fallback内訳
- 回答形式
- Q1からQ10までの問題文とA-D選択肢
- answer-key-freeな `quiz_prompt.md` path

## Artifacts

- `scripts/aicx_study_fixture.py`
- `tests/test_aicx_study_fixture.py`
- `docs/standards/delivery-artifacts-v0/schemas/slack_outbound_message.schema.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/slack_outbound_message.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/artifact_inventory.json`
- `docs/standards/delivery-artifacts-v0/artifact_catalog.md`
- `artifacts/runs/poc-3c-slack-full-body/validation_report.json`

## Verification

- `python3 -m unittest discover -s tests`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-3c-slack-full-body/validation_report.json`
- `cargo test`
- `git diff --check`

Result:

- Python unittest: 27 passed
- validate-artifacts: 52 passed, 0 failed, 4 skipped
- cargo test: 4 passed
- diff whitespace: pass

## Review Focus

- Slack本文に10問すべてが入り、Slack内完結の体験に戻っているか
- answer key / rationale が outbound artifact に混入していないか
- `preview_count` / `preview_questions` の既存field名を維持した互換対応で十分か
