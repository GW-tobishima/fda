# PoC-3 Review Packet: Adaptive Study Loop Fixture

## Scope

PoC-3の初期版として、LLM問題生成やPDF ingestには進まず、既存の採点履歴とquestion bankだけで翌日10問の配分計画を作る。

対象:

- 過去の `grading_report.json` を読む
- topic別正答率を集計する
- 80%未満のtopicを弱点として扱う
- 翌日の10問配分を弱点topic寄りに変える
- `adaptive_plan.json` を生成する
- 問題選択は `question_bank.fixture.json` を優先し、不足分だけ `topic_map` fallbackにする

対象外:

- LLMによる新規問題生成
- PDF/OCR ingest
- question bank自動更新
- Slack live送信
- scheduler / daemon追加
- 複数ユーザー対応

## Implementation Notes

- `scripts/aicx_study_fixture.py` に `adaptive-plan` commandを追加した。
- `build_adaptive_plan()` は指定日のstudy windowを解決し、入力されたgrading report群からtopic別に `total` / `correct` / `accuracy` を集計する。
- 10問配分は、まず対象topicへ最低1問ずつ割り当て、残りを80%未満topicへ低正答率順で配る。
- PoC exampleでは、2026-06-21の結果から `TOPIC-5D` と `TOPIC-ORG-KPI` が弱点になり、2026-06-22の配分は 5D=4問、組織/KPI=3問、その他=各1問になる。
- `question_selection` は各topicでquestion bankを先に選び、bankが足りない場合だけ `manual_fixture_topic_map` のfallback questionを使う。

## Artifacts

- `docs/standards/delivery-artifacts-v0/schemas/adaptive_plan.schema.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/adaptive_plan.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/artifact_inventory.json`
- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/runner_explanation.json`
- `artifacts/runs/poc-3-aicx-study-bot/validation_report.json`

## Verification

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
- `python3 -m unittest discover -s tests`
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-3-aicx-study-bot/validation_report.json`
- `cargo test`
- `git diff --check`

Result:

- Python unittest: 23 passed
- validate-artifacts: 52 passed, 0 failed, 4 skipped
- cargo test: 4 passed
- diff whitespace: pass

## Review Focus

- 80%未満topicの扱いが学習運用として自然か
- 10問配分がSlack完結の学習量として妥当か
- `question_bank_fixture` 優先、fallback許容の境界がPoC-3初期版として十分か
- 次PRで `adaptive_plan` を `daily-dispatch` のquiz生成に接続する方針でよいか
