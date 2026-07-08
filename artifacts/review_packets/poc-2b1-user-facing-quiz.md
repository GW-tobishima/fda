# PoC-2B.1 Review Packet

## Scope

AICX Study Bot の Slack outbound smoke 前に、answer key なしの user-facing quiz artifact を追加する。

## Changes

- `quiz_prompt.json` / `quiz_prompt.md` を追加。
- `quiz_prompt.schema.json` を追加し、各問題に `correct_choice` / `rationale` を含めない契約にした。
- runner に `quiz-prompt` command と `build_quiz_prompt` / Markdown rendering を追加。
- `run-fixture` でも `quiz_prompt` artifact を生成するようにした。
- Slack outbound message の `全文` を `quiz_set.json` から `quiz_prompt.md` に変更。
- artifact catalog / inventory / runner explanation を更新。

## Validation

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
  - 6 tests OK
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2b1-aicx-study-bot/validation_report.json`
  - 38 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 tests OK
- `git diff --check`

## Non-goals

- Slack live send retry。
- Slack reply intake / grading。
- PDF ingest。
- 問題品質の追加改善。

## Residual Risk

PoC-2C の live smoke は、現在の `SLACK_BOT_TOKEN` が Slack Bot User OAuth Token として有効になるまで `invalid_auth` で止まる可能性がある。
