use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::RuntimeContext;
use crate::support::paths::display_path;

pub(crate) fn basic_design_markdown(input_summary: &str) -> String {
    format!(
        "# Basic Design\n\n\
## 1. 目的\n\n\
Intake で定義された目的を、実装前に検証可能な Design Gate artifact へ落とす。\n\n\
## 2. 入力要約\n\n\
{}\n\n\
## 3. Scope In\n\n\
- Basic Design と Detailed Design を作る。\n\
- Case Graph、Task Graph、Planned PRs、Autonomy Contract、Forge Projection を作る。\n\
- Functional QA brief と Security QA brief を作る。\n\n\
## 4. Scope Out\n\n\
- この command は target repo の実装、MCP agent invocation、PR 作成、merge を行わない。\n\
- 実装者、Functional QA、Security QA の実起動は後続 PR の責務とする。\n\n\
## 5. Acceptance Criteria\n\n\
- Given Intake の Human Decision が解決済み、When `fda design` を実行する、Then Design Gate artifact が生成される。\n\
- Given Human Decision が未解決、When `fda design` を実行する、Then Design Gate は停止し、判断 ID と再開 command を表示する。\n\
- Given 生成 JSON artifact、When `validate-artifacts` を実行する、Then schema validation が pass する。\n\n\
## 6. OPEN_QUESTIONS\n\n\
- 実装PRの分割数は後続 Planned PR refinement で確定する。\n\
- MCP 実装 agent の実 tool capability は PR-V1-005 の dry-run で確認する。\n\n\
## 7. Risk And Mitigation\n\n\
- Risk: 設計 artifact が入力意図を過剰に具体化する。Mitigation: Scope Out と Human Decision dependency を Planned PRs に残す。\n\
- Risk: Security QA が Functional QA と混ざる。Mitigation: brief を分離し、Task Graph 上も role を分ける。\n",
        input_summary
    )
}

pub(crate) fn detailed_design_markdown(input_summary: &str) -> String {
    format!(
        "# Detailed Design\n\n\
## 1. Input Contract\n\n\
- Source summary: {}\n\
- Required before design: unresolved Human Decision がないこと。\n\n\
## 2. Artifact Contract\n\n\
- `basic_design.md`: Scope、AC、OPEN_QUESTIONS、risk を持つ。\n\
- `case_graph.json`: Case と Planned PR の対応を持つ。\n\
- `task_graph.json`: Implementer / Functional QA / Security QA を分離する。\n\
- `planned_prs.json`: 受入条件、証跡、Human Decision dependency を持つ。\n\
- `autonomy_contract.json`: allowed / forbidden / escalation / evidence policy を持つ。\n\
- `forge_projection.json`: ClaimContract と Proof Obligation を持つ。\n\n\
## 3. Execution Boundary\n\n\
Design Gate は planning-only である。実装、テスト実行、PR作成、merge、通知送信は行わない。\n\n\
## 4. QA Brief Linkage\n\n\
Functional QA は受入条件の充足、Security QA は権限、個人情報、外部API、秘密情報の扱いを確認する。\n",
        input_summary
    )
}

pub(crate) fn functional_qa_brief_markdown() -> String {
    "# Functional QA Brief\n\n\
- 受入条件が Given / When / Then 形式で追跡できることを確認する。\n\
- Planned PR ごとの expected evidence が実装後に回収可能であることを確認する。\n\
- Human Decision dependency が未解決のまま実装へ進んでいないことを確認する。\n"
        .to_string()
}

pub(crate) fn security_qa_brief_markdown() -> String {
    "# Security QA Brief\n\n\
- 外部API、個人情報、秘密情報、法務制約が未記載の場合は Human Decision または Security Gate へ戻す。\n\
- QA role は read-only とし、source mutation、merge approval、risk self-approval を行わない。\n\
- High / Critical security finding は自動 repair ではなく Human Turn 条件にする。\n"
        .to_string()
}

pub(crate) fn design_case_graph(context: &RuntimeContext) -> Value {
    json!({
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "cases": [
            {
                "case_id": "CASE-FDA-V1-DESIGN-001",
                "purpose": "Design Gate artifact を実装前の正本として生成する",
                "depends_on": [],
                "claim_ids": ["CLM-FDA-V1-DESIGN-001"],
                "planned_pr": "PPR-FDA-V1-IMPLEMENT-001",
                "risk": "medium",
                "state": "planned"
            }
        ]
    })
}

pub(crate) fn design_task_graph(context: &RuntimeContext) -> Value {
    json!({
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "tasks": [
            design_task(context, "TASK-FDA-V1-IMPLEMENT-001", "Implementer MCP handoff を作る", "implementer"),
            design_task(context, "TASK-FDA-V1-FQA-001", "Functional QA brief に基づいて実装PRを検証する", "functional_qa"),
            design_task(context, "TASK-FDA-V1-SQA-001", "Security QA brief に基づいて実装PRを検証する", "security_qa")
        ]
    })
}

fn design_task(context: &RuntimeContext, task_id: &str, title: &str, owner_agent: &str) -> Value {
    json!({
        "task_id": task_id,
        "title": title,
        "parent_type": "case",
        "parent_id": "CASE-FDA-V1-DESIGN-001",
        "status": "ready_to_work",
        "forge": {
            "program_id": context.program_id,
            "epic_id": context.epic_id,
            "case_id": "CASE-FDA-V1-DESIGN-001",
            "claim_ids": ["CLM-FDA-V1-DESIGN-001"],
            "promotion_state": "draft",
            "proof_state": "partial"
        },
        "execution": {
            "owner_agent": owner_agent,
            "branch": Value::Null,
            "pr_number": Value::Null,
            "run_id": Value::Null
        },
        "human": {
            "decision_required": false,
            "decision_packet_id": Value::Null
        }
    })
}

pub(crate) fn design_planned_prs(context: &RuntimeContext) -> Value {
    json!({
        "schema_version": "forge_delivery.planned_prs.v0",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "source_requirements": ["requirements_definition.md", "basic_design.md", "detailed_design.md"],
        "planned_prs": [
            {
                "planned_pr_id": "PPR-FDA-V1-IMPLEMENT-001",
                "sequence": 1,
                "title": "Implement approved Design Gate slice",
                "case_id": "CASE-FDA-V1-DESIGN-001",
                "purpose": "Design Gate で定義された最小実装 slice を Codex MCP dry-run へ渡せる形にする。",
                "scope": {
                    "in": ["Implementation handoff を作れる契約", "Functional QA と Security QA の証跡要求"],
                    "out": ["MCP live execution", "PR merge approval", "通知送信"]
                },
                "risk": {
                    "level": "medium",
                    "summary": "実装 agent の capability は後続 dry-run まで未検証。",
                    "mitigations": ["PR-V1-005 で MCP tools/list と cwd / approval policy を検証する"]
                },
                "claim_ids": ["CLM-FDA-V1-DESIGN-001"],
                "proof_strategy": [
                    {
                        "proof_id": "PROOF-FDA-V1-DESIGN-001",
                        "type": "schema_validation",
                        "description": "Design Gate JSON artifact が schema validation を通る。",
                        "blocking": true,
                        "expected_evidence": ["validation_report.json"]
                    }
                ],
                "acceptance_criteria": [
                    "Given resolved Human Decision, When fda design runs, Then Design Gate artifacts are generated.",
                    "Given generated JSON artifacts, When validate-artifacts runs, Then schema validation passes."
                ],
                "expected_files": ["implementation_handoff.md", "mcp_agent_invocation_plan.json"],
                "auto_merge_candidate": false,
                "human_decision_dependencies": []
            }
        ]
    })
}

pub(crate) fn design_autonomy_contract(context: &RuntimeContext) -> Value {
    json!({
        "contract_id": "AUTONOMY-FDA-V1-DESIGN-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "autonomy_level": "planning_only",
        "status": "approved",
        "applies_to": ["PR-V1-003", "Design Gate"],
        "expires_at": Value::Null,
        "allowed_actions": ["Generate design artifacts", "Validate generated JSON artifacts", "Prepare next dry-run handoff"],
        "forbidden_actions": ["Modify target repo implementation", "Invoke live MCP implementer", "Create implementation PR", "Merge PR", "Approve security/legal risk"],
        "escalation_rules": [
            {
                "trigger": "Scope In / Scope Out の変更が必要",
                "required_action": "Human Decision を開く",
                "human_decision_packet_required": true
            },
            {
                "trigger": "Security High / Critical risk が見つかる",
                "required_action": "Security QA から Human Turn へ戻す",
                "human_decision_packet_required": true
            }
        ],
        "auto_merge_policy": {
            "allowed": false,
            "conditions": [],
            "forbidden": ["Design Gate は merge approval ではない", "privacy/security/legal risk は human approval 必須"]
        },
        "evidence_policy": {
            "citations_required": false,
            "source_trace_required": true,
            "verification_required": true,
            "summary_update_required": true
        },
        "next_allowed_phase": ["fda implement --dry-run"],
        "forge_mapping": {
            "claim_ids": ["CLM-FDA-V1-DESIGN-001"],
            "proof_obligations": ["PROOF-FDA-V1-DESIGN-001"],
            "human_decision_points": [],
            "ato_task_graph": ["TASK-FDA-V1-IMPLEMENT-001", "TASK-FDA-V1-FQA-001", "TASK-FDA-V1-SQA-001"],
            "planned_prs": ["PPR-FDA-V1-IMPLEMENT-001"],
            "gate_requirements": ["Design Gate", "Functional QA brief", "Security QA brief"]
        }
    })
}

pub(crate) fn design_runner_explanation(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "planning",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda design generated Design Gate artifacts from {}", display_path(repo_root, artifact_dir)),
            "changed_input_summary": Value::Null,
            "stop_condition": "design_gate_artifacts_generated_and_validated",
            "next_action": "fda implement --dry-run",
            "automation_boundary": "PR-V1-003 design command only; no target repo implementation, MCP agent invocation, GitHub PR execution, merge, release, deploy, notification, or Output Hub generation",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("basic_design.md")),
                display_path(repo_root, &out_dir.join("planned_prs.json")),
                display_path(repo_root, &out_dir.join("validation_report.json"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}
