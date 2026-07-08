# Adapters

AI Delivery Runtime は adapter 経由で外部 system と接続します。
adapter は実行結果を ATO evidence / Forge proof に戻すための境界であり、ATO / Forge の正本を置換しません。

| adapter | 責務 | 書き戻し |
|---|---|---|
| `ato_cli` | work begin/checkpoint/complete、Task Graph materialization、Evidence attach | ATO task/run/evidence |
| `forge_cli` | Case / PromotionDecision / ReleasePromotionDecision evaluation | Forge gate verdict |
| `github` | issue / branch / PR / review metadata | PR link、review summary |
| `codex` | coding-agent execution packet の発行 | handoff、runtime evidence |
| `sandbox` | command execution、test、artifact collection | durable artifact path |
| `model_provider` | structured output generation | model run record |

## Adapter contract

各 adapter は次を返します。

- request_id
- adapter_run_id
- input_summary
- output_summary
- evidence_links
- validation_status
- retryable
- human_decision_required
- artifact_links
- artifact_inventory_delta
- runner_explanation, if applicable

stdout / stderr の全文は ATO DB に保存しません。durable artifact path と verdict を返します。

## Human-facing adapter rules

ATO-HQ UIUX Knowledge Pack に基づき、adapter は人間向け UI に内部実行をそのまま漏らしません。

- Task / Run / Agent の詳細は drill-down 用の evidence として保持する。
- 既定表示向けには artifact、decision、risk、outcome、evidence の summary を返す。
- 人間に Job Packet / execution packet を手で作らせない。
- `codex` adapter は自然言語 intent から Job Packet / Work Contract / Trace Keys を生成できる入力形式を受ける。
- `github` / `sandbox` / `codex` adapter は、生成 artifact を Output Hub / Artifact Inbox に載せるための artifact metadata を返す。

## Output Hub metadata

artifact を生成・更新する adapter は次を返します。

- artifact_id
- artifact_type
- title
- producer_adapter
- producer_agent, if available
- related_program_id / epic_id / case_id / task_id
- preview_summary
- path_or_url
- open_in_editor_link, if available
- open_in_browser_link, if available
- diff_link, if available
- evidence_links
- created_at / updated_at

## Runner explanation metadata

`codex` / `sandbox` / future runner adapter は、retry / resume / rerun で次を返します。

- current_phase
- previous_run_id
- diff_from_previous_run
- execution_actor
- input_summary
- changed_input_summary
- stop_condition
- next_action
- automation_boundary
- completion_evidence
- failure_evidence

opaque な `rerun` 成功/失敗だけを返さないでください。人間が「なぜ再実行するのか」「何が変わるのか」「どこまで自動で進むのか」を読める形にします。
