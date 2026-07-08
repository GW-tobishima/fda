# PR-8 Review Packet

## CHANGE_INTENT

AICX AI ストラテジスト学習 Bot の PoC-2B として、Slack API key がない状態でも送信予定 payload と env readiness を確認できる Slack outbound smoke を追加する。

## SCOPE_IN

- `.env.example` に Slack smoke 用 env 変数を追加する。
- `.gitignore` に `.env` を追加し、実 token の誤コミットを防ぐ。
- `scripts/aicx_study_fixture.py` に `slack-smoke` command を追加する。
- `slack_outbound_message.json` を dry-run artifact として生成する。
- `slack_delivery_receipt.json` を dry-run artifact として生成する。
- Slack outbound message / delivery receipt の JSON Schema を追加する。
- artifact inventory / catalog / runner explanation / README を PoC-2B に合わせて更新する。
- unit test で summary + 先頭3問 preview と dry-run receipt を確認する。

## SCOPE_OUT

- Slack API key の取得。
- 実 Slack workspace への送信確認。
- Slack reply intake / grading。
- 朝4:30 scheduler。
- PDF/OCR ingest。
- LINE integration。

## AC_EVIDENCE

- Slack token なしで送信 payload を生成できる。
  - `slack-smoke --mode dry-run` が `slack_outbound_message.json` を生成済み。
- Slack へいきなり30問全文を送らず、summary + 先頭3問 + artifact path にできる。
  - `slack_outbound_message.json` は `total_questions=30`、`preview_count=3`、`artifact_path=.../quiz_set.json`。
- env readiness を secret 非含有で確認できる。
  - `slack_delivery_receipt.json` は `SLACK_BOT_TOKEN` / `SLACK_CHANNEL_ID` の present/missing だけを出力し、実値は含めない。
- dry-run は Slack API を呼ばない。
  - `slack_delivery_receipt.json` は `mode=dry-run`、`slack_used=false`、`status=dry_run_ready`。
- live mode は fail-closed である。
  - `SLACK_BOT_TOKEN` / `SLACK_CHANNEL_ID` / `slack_sdk` が揃わない場合は `blocked_missing_env_or_sdk` receipt を書いて失敗する。

## VALIDATION

- `python3 -m py_compile scripts/aicx_study_fixture.py tests/test_aicx_study_fixture.py`
  - pass
- `python3 -m unittest discover -s tests`
  - 5 tests OK
- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-2b-aicx-study-bot/validation_report.json`
  - 36 passed, 0 failed, 4 skipped
- `cargo test`
  - 4 passed
- `git diff --check`
  - pass

## SECURITY_EVIDENCE

- 実 Slack token は追加していない。
- `.env` / `.env.local` / `.env.*.local` は git ignore 対象。
- `slack_delivery_receipt.json` は token/channel の実値を保存せず、present/missing のみ保存する。
- 元の Windows PDF path は tracked content に含めていない。
- 教材 PDF 本文および PDF ファイルは追加していない。
- Slack bot token marker の新規追加はない。既存 PR-6 review packet の説明文だけが検出される。

## SELF_REVIEW

- verdict: pass
- 変更範囲は PoC-2B Slack outbound smoke に限定。
- live mode は未実行だが、dry-run artifact と fail-closed readiness は確認済み。
- Slack reply intake / grading は PoC-2C に分離。
- 未追跡の `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot.zip` は本 PR に含めていない。

## FORGE_PROMOTION_DECISION

- verdict: draft
- policy_version: delivery-artifacts-v0
- missing_proofs: []
- blocking_reasons: []
- note: この PR は PoC-2B の Slack outbound smoke draft であり、merge approval ではない。

## TASK_TRACEABILITY

- PR: https://github.com/msamunetogetoge/forge-delivery-agent/pull/8
- Branch: `codex/aicx-study-bot-poc-2b-slack-outbound`
- Commit: `ec28b17`
- ATO task: `poc-2b-slack-outbound-smoke-20260621`
- Validation artifact: `artifacts/runs/poc-2b-aicx-study-bot/validation_report.json`

## HUMAN_DECISIONS_REQUIRED

- HUMAN_TURN_REASON: merge_approval
- REQUESTED_DECISION: この draft PR を ready-for-review / merge 対象にするか。
- RECOMMENDED_OPTION: まず dry-run Slack payload と receipt を確認し、live Slack smoke に進む前に token / channel / SDK readiness を決める。
- WHY_NOW: Slack reply intake に進む前に、送信 payload と env readiness の境界を固定するため。
- IMPACT_IF_DELAYED: Slack reply intake 実装時に送信 message id、artifact path、env readiness の前提が不安定になる。
- OWNER_ROLE: orchestrator

## OPEN_RISKS

- live Slack 送信は未実行。
- `slack_sdk` は repo dependency として固定していない。
- Slack reply intake / grading は未実装。
- 朝4:30 scheduler と二重送信防止は未実装。

## ROLLBACK_PLAN

- 不要なら PR branch を close する。
- dry-run artifact は generated fixture であり、Slack API や token state には副作用がない。
- local `.env.local` と `.ato/` state は git 管理外のため rollback 対象外。
