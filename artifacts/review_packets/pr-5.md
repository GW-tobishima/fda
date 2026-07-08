# PR-5 Review Packet

## CHANGE_INTENT

AICX AI ストラテジスト資格の学習支援 Bot について、PoC-0 の Human Input Spec から Requirements Definition、Epic Delivery Plan、Environment Readiness Plan、ATO Task Graph までを追跡できる planning artifacts として追加する。

## SCOPE_IN

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/` に PoC-0 artifact 一式を追加する。
- `artifacts/runs/poc-0-aicx-study-bot/validation_report.json` に schema validation の証跡を残す。
- `docs/knowledge/kn_01KVMFFJN08WYQ8HAXRFZ8N8AM.md` に Slack-first / LINE-first の local runtime 判断を local Knowledge として残す。
- 人間判断を反映し、初期チャネルを Slack、回答形式を A-D 選択式、通知頻度を毎日 4:30 Asia/Tokyo とする。
- `.ato/` をローカル専用の `.git/info/exclude` に追加し、PR には含めない。
- `data/study_bot/` をローカル専用の `.git/info/exclude` に追加し、PDF と派生学習データを PR に含めない。

## SCOPE_OUT

- 本番 Bot 実装。
- Slack App / LINE channel 作成。
- PDF OCR / text extraction の実行。
- 教材 PDF 本文の repo 保存。
- scheduled delivery の有効化。

## AC_EVIDENCE

- `cargo run -- validate-artifacts --artifacts docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot --out artifacts/runs/poc-0-aicx-study-bot/validation_report.json`
  - verdict: pass
  - summary: 19 passed, 0 failed, 7 skipped
- `cargo test`
  - 4 passed
- `git diff --cached --check`
  - pass

## FORGE_PROMOTION_DECISION

- verdict: draft
- policy_version: delivery-artifacts-v0
- missing_proofs: []
- blocking_reasons:
  - Human-only decisions remain before implementation: notification channel, PDF source/use, answer format, scheduler policy.
- note: この PR は planning artifact の draft PR であり、merge approval ではない。

## SECURITY_EVIDENCE

- Slack/LINE token は追加していない。
- PDF 本文および PDF ファイルは追加していない。
- PDF は local-only path `data/study_bot/source/pdf/aicx_ai_agent_strategist_official_text.pdf` にコピー済みで、git 管理対象外。
- `.ato/` local DB は `.git/info/exclude` に登録し、commit 対象外にした。
- `data/study_bot/` は `.git/info/exclude` に登録し、commit 対象外にした。
- Knowledge は `share_scope: local`, `visibility: pending`。

## TASK_TRACEABILITY

- PR: https://github.com/msamunetogetoge/forge-delivery-agent/pull/5
- Branch: `codex/aicx-study-bot-poc-0`
- Commit: `4ba8cf9`
- ATO publish task: `publish-poc-0-study-bot-pr-20260621`
- ATO source task: `poc-0-human-input-spec-to-task-graph-20260621`
- Validation artifact: `artifacts/runs/poc-0-aicx-study-bot/validation_report.json`
- Knowledge artifact: `docs/knowledge/kn_01KVMFFJN08WYQ8HAXRFZ8N8AM.md`

## DECISIONS_APPLIED

- PoC-0 artifact direction: OK
- Initial channel: Slack
- PDF source: local-only copy at `data/study_bot/source/pdf/aicx_ai_agent_strategist_official_text.pdf`
- Answer format: A-D 選択式
- Schedule: 毎日 4:30 Asia/Tokyo

## HUMAN_DECISIONS_REQUIRED

- HUMAN_TURN_REASON: merge_approval
- REQUESTED_DECISION: この draft PR を review / merge 対象にするか。
- RECOMMENDED_OPTION: まず draft のまま artifact 内容を確認する。
- WHY_NOW: PoC-0 の planning artifact を次の実装 task graph へ進める前に、人間の意図と範囲が合っているか確認するため。
- IMPACT_IF_DELAYED: Slack adapter、PDF ingest、scheduler 実装の着手が遅れる。
- OWNER_ROLE: orchestrator

## OPEN_RISKS

- PDF の text extraction 方式と page mapping は未検証。
- Slack token と channel polling 権限は未確認。
- 問題生成品質は PoC-0 では検証していない。

## ROLLBACK_PLAN

- この PR は artifact 追加のみ。不要なら PR branch を close する。
- local `.git/info/exclude` の `.ato/` 追加はローカル設定であり、PR rollback 対象ではない。
