# PoC-3A Review Packet: Adaptive Daily Dispatch

## Scope

PoC-3で作った `adaptive_plan.json` を、日次dispatchのoptional入力として接続する。

対象:

- `daily-dispatch --adaptive-plan <path>` を追加
- adaptive plan指定時だけ `topic_allocations` に従って `quiz_set.json` を生成する
- adaptive plan未指定時の既存挙動を維持する
- adaptive入力の不整合をfail-closedにする
- runbookにadaptive dispatch手順を追記する

対象外:

- LLMによる新規問題生成
- PDF/OCR ingest
- Slack live smokeの再実行
- scheduler変更
- 長時間daemon
- 複数ユーザー対応

## Fail-Closed Rules

`daily-dispatch --adaptive-plan` は以下の場合に `quiz_set.json` を生成しない。

- `adaptive_plan.next_quiz_date` と `--date` が一致しない
- `adaptive_plan.question_count` とdispatch側の有効問題数が一致しない
- `adaptive_plan.topic_allocations` の合計が `adaptive_plan.question_count` と一致しない
- `adaptive_plan.topic_allocations` に重複topicがある
- `adaptive_plan.topic_allocations` に当日のtopic scope外topicが含まれる
- `adaptive_plan.study_window.week_id` とdispatch対象study windowが一致しない

## Expected Behavior

2026-06-22に `adaptive_plan.json` を指定した場合:

- 合計10問
- `TOPIC-5D`: 4問
- `TOPIC-ORG-KPI`: 3問
- `TOPIC-BRANCHING-CONTEXT`: 1問
- `TOPIC-WORKFLOW-CHECK`: 1問
- `TOPIC-ADOPTION-GOVERNANCE`: 1問
- question bank由来7問、topic_map fallback由来3問

## Artifacts

- `scripts/aicx_study_fixture.py`
- `tests/test_aicx_study_fixture.py`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/daily_operation_runbook.md`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/runner_explanation.json`
- `artifacts/runs/poc-3a-aicx-study-bot/validation_report.json`

## Verification

- `python3 scripts/aicx_study_fixture.py daily-dispatch ... --adaptive-plan ... --date 2026-06-22 --mode dry-run --force`
- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-3a-aicx-study-bot/validation_report.json`
- `cargo test`
- `git diff --check`

Result:

- adaptive dispatch smoke: 10問、5D=4、組織/KPI=3、他topic各1
- Python unittest: 27 passed
- validate-artifacts: 52 passed, 0 failed, 4 skipped
- cargo test: 4 passed
- diff whitespace: pass

## Review Focus

- `--adaptive-plan` をoptionalにした境界が自然か
- fail-closed条件が過不足ないか
- 次PRで前日grading_reportから当日adaptive_planを自動選択する方向でよいか
