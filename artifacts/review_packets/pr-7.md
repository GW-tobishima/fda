# PR-7 Review Packet

## CHANGE_INTENT

AICX AI ストラテジスト学習 Bot の PoC-2A として、問題文 fixture の品質を上げるため、5 topic x 2問の手作り question bank を追加し、runner が指定日の topic scope に合う問題を question bank から優先選択できるようにする。

## SCOPE_IN

- `question_bank.fixture.json` に10問の人間作成 fixture 問題を追加する。
- `question_bank.fixture.schema.json` を追加する。
- `scripts/aicx_study_fixture.py` の runner を、question bank 優先、topic_map template fallback の順で生成するよう更新する。
- `quiz_set.schema.json` に `question_bank_fixture` source と `scenario_tags` を追加する。
- `topic_map.json` の 5D 範囲 topic を `TOPIC-5D` に整理する。
- 2026-06-21 の fixture 出力を再生成する。
- artifact inventory / catalog / runner explanation / unittest を更新する。

## SCOPE_OUT

- Slack outbound smoke。
- Slack reply intake / grading。
- Slack SDK または Slack API key 取得。
- LINE integration。
- PDF/OCR ingest。
- 外部モデル呼び出しによる問題生成。
- 30問すべての人間品質化。

## AC_EVIDENCE

- 5 topic x 2問の fixture 問題を保持できる。
  - `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/question_bank.fixture.json` に10問を追加済み。
- question bank から指定日の topic に合う問題を選べる。
  - `generate_quiz_set(..., question_bank)` が対象週の topic scope から bank 問題を優先選択する。
- 2026-06-21 の30問 quiz_set を生成できる。
  - `quiz_set.json` は `question_count=30`。
  - 先頭10問は `source=question_bank_fixture`、残り20問は既存 fallback 生成。
- topic 別正答率と推奨ページが引き続き出る。
  - `grading_report.json` と `study_recommendation.json` を再生成済み。
- PDF本文なしで成立する。
  - `quiz_set.json` は `pdf_ingest_used=false`。
  - `runner_explanation.json` は question bank fixture と manual topic map を入力として説明。

## VALIDATION

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
  - pass
- `python3 -m unittest discover -s tests`
  - 3 tests OK
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2a-aicx-study-bot/validation_report.json`
  - 32 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 passed
- `git diff --check`
  - pass
- `git diff --cached --check`
  - pass

## SECURITY_EVIDENCE

- Slack/LINE token は追加していない。
- Slack bot token marker は新規追加していない。
- 元の Windows PDF path は tracked content に含めていない。
- 教材 PDF 本文および PDF ファイルは追加していない。
- `.ato/`、`data/study_bot/`、`.env.local` は `.gitignore` で除外済み。

## SELF_REVIEW

- verdict: pass
- 変更範囲は PoC-2A の question bank fixture と runner 選択順に限定。
- Slack outbound / reply intake / PDF ingest は後続 PoC に分離。
- schema、生成 artifact、unittest、cargo validation は更新済み。
- 未追跡の `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot.zip` は本 PR に含めていない。

## FORGE_PROMOTION_DECISION

- verdict: draft
- policy_version: delivery-artifacts-v0
- missing_proofs: []
- blocking_reasons: []
- note: この PR は PoC-2A の fixture quality improvement draft であり、merge approval ではない。

## TASK_TRACEABILITY

- PR: https://github.com/msamunetogetoge/forge-delivery-agent/pull/7
- Branch: `codex/aicx-study-bot-poc-2a`
- Commit: `7e8101f`
- ATO task: `poc-2a-question-bank-fixture-quality-20260621`
- Validation artifact: `artifacts/runs/poc-2a-aicx-study-bot/validation_report.json`

## HUMAN_DECISIONS_REQUIRED

- HUMAN_TURN_REASON: merge_approval
- REQUESTED_DECISION: この draft PR を ready-for-review / merge 対象にするか。
- RECOMMENDED_OPTION: まず question bank 10問と generated quiz_set を確認し、PoC-2B Slack outbound smoke へ進むか判断する。
- WHY_NOW: Slack outbound に進む前に、送信する fixture 問題の最低品質ラインを固定するため。
- IMPACT_IF_DELAYED: Slack smoke 実装時に送信 payload の人間向け品質が不安定になる。
- OWNER_ROLE: orchestrator

## OPEN_RISKS

- question bank は10問のみで、30問すべての人間品質化は未完了。
- PDF page mapping と本文抽出は未検証。
- Slack delivery / answer intake は未実装。
- 実学習効果は回答ログで未検証。

## ROLLBACK_PLAN

- 不要なら PR branch を close する。
- runner は `--question-bank` 未指定なら従来の topic_map template 生成に戻る。
- local PDF と `.ato/` state は git 管理外のため rollback 対象外。
