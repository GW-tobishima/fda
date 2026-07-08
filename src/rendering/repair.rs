use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::RuntimeContext;
use crate::rendering::inventory::{artifact_inventory_entry, ArtifactInventorySpec};
use crate::support::paths::display_path;

pub(crate) struct RepairGateView<'a> {
    pub(crate) status: &'a str,
    pub(crate) issues: &'a [String],
    pub(crate) qa_status: &'a str,
    pub(crate) functional_qa_status: &'a str,
    pub(crate) security_qa_status: &'a str,
    pub(crate) return_to_role: Option<&'a str>,
    pub(crate) actual_pr_url: Option<&'a str>,
    pub(crate) reviewed_planned_pr_id: &'a str,
    pub(crate) findings: &'a [String],
    pub(crate) evidence_links: &'a [String],
}

pub(crate) struct RetryHistoryInputs<'a> {
    pub(crate) context: &'a RuntimeContext,
    pub(crate) existing_attempts: &'a [Value],
    pub(crate) gate: &'a RepairGateView<'a>,
    pub(crate) failure_classification: &'a str,
    pub(crate) repair_loop_status: &'a str,
    pub(crate) retry_attempt_count: u32,
    pub(crate) retry_limit: u32,
    pub(crate) created_at_unix_seconds: u64,
}

pub(crate) fn repair_prompt_markdown(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &RepairGateView<'_>,
    failure_classification: &str,
    retry_attempt_count: u32,
    retry_limit: u32,
) -> String {
    let findings = if gate.findings.is_empty() {
        "- <none>".to_string()
    } else {
        gate.findings
            .iter()
            .map(|finding| format!("- {finding}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# Repair Prompt\n\n\
あなたは FDA V1 の Implementer です。QA FAIL を読み、同じ planned PR の範囲内で修正してください。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- repair loop artifact: `PR-V1-008`\n\
- planned PR: `{}`\n\
- actual PR: `{}`\n\
- failure classification: `{}`\n\
- retry: `{}/{}`\n\n\
## QA findings\n\n\
{}\n\n\
## 必須事項\n\n\
- Human Decision 未解決の scope 変更をしない。\n\
- 同じ原因の retry 上限を超えた場合は修正せず Human Turn に戻す。\n\
- 修正後は test を再実行し、`fda review` へ戻す。\n\
- merge / release / risk self-approval は行わない。\n",
        target_repo.display(),
        context.program_id,
        context.epic_id,
        gate.reviewed_planned_pr_id,
        gate.actual_pr_url.unwrap_or("<unavailable>"),
        failure_classification,
        retry_attempt_count,
        retry_limit,
        findings
    )
}

pub(crate) fn failure_classification_receipt(
    context: &RuntimeContext,
    gate: &RepairGateView<'_>,
    failure_classification: &str,
) -> Value {
    json!({
        "schema_version": "fda.failure_classification.v0",
        "classification_id": "FAIL-FDA-V1-008-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-008",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "classification": failure_classification,
        "qa_status": gate.qa_status,
        "functional_qa_status": gate.functional_qa_status,
        "security_qa_status": gate.security_qa_status,
        "return_to_role": gate.return_to_role,
        "findings": gate.findings,
        "issues": gate.issues,
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn retry_history_receipt(input: RetryHistoryInputs<'_>) -> Value {
    let mut attempts = input.existing_attempts.to_vec();
    if input.gate.status == "repair_required"
        && matches!(input.repair_loop_status, "repair_planned" | "human_turn")
    {
        attempts.push(json!({
            "attempt": input.retry_attempt_count,
            "failure_classification": input.failure_classification,
            "status": if input.repair_loop_status == "repair_planned" { "planned" } else { "retry_limit_or_human_turn" },
            "return_to_role": input.gate.return_to_role,
            "evidence": ["qa_receipt.json", "failure_classification.json"],
            "created_at": format!("unix:{}", input.created_at_unix_seconds)
        }));
    }
    json!({
        "schema_version": "fda.retry_history.v0",
        "history_id": "RETRY-FDA-V1-008-001",
        "program_id": input.context.program_id,
        "epic_id": input.context.epic_id,
        "planned_pr_id": "PR-V1-008",
        "retry_limit": input.retry_limit,
        "current_attempt": input.retry_attempt_count,
        "attempts": attempts,
        "same_cause_retry_count": input.retry_attempt_count,
        "evidence_links": ["qa_receipt.json", "repair_receipt.json", "failure_classification.json"]
    })
}

pub(crate) fn repair_receipt(
    context: &RuntimeContext,
    gate: &RepairGateView<'_>,
    failure_classification: &str,
    repair_loop_status: &str,
    retry_attempt_count: u32,
    retry_limit: u32,
) -> Value {
    json!({
        "schema_version": "fda.repair_receipt.v0",
        "receipt_id": "REPAIR-FDA-V1-008-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-008",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "status": repair_loop_status,
        "failure_classification": failure_classification,
        "retry_attempt_count": retry_attempt_count,
        "retry_limit": retry_limit,
        "retry_limit_reached": repair_loop_status == "human_turn" && gate.status == "repair_required",
        "return_to_role": if repair_loop_status == "repair_planned" {
            json!("implementer")
        } else if repair_loop_status == "human_turn" {
            json!("human")
        } else {
            Value::Null
        },
        "human_turn_condition": if repair_loop_status == "human_turn" {
            json!("retry limit reached or security/high-risk decision required")
        } else {
            Value::Null
        },
        "source_mutation_by_qa": false,
        "input_artifacts": ["qa_receipt.json", "functional_qa_receipt.json", "security_qa_receipt.json"],
        "output_artifacts": ["repair_prompt.md", "retry_history.json", "failure_classification.json"],
        "evidence_links": gate.evidence_links,
        "next_action": if repair_loop_status == "repair_planned" {
            "send repair_prompt.md to implementer via codex-reply, then rerun fda review"
        } else if repair_loop_status == "no_repair_needed" {
            "fda merge"
        } else if repair_loop_status == "human_turn" {
            "open Human Decision"
        } else {
            "resolve QA evidence blocker"
        }
    })
}

pub(crate) fn repair_runner_explanation(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    repair_loop_status: &str,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "single_planned_pr_execution",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda continue from {}", display_path(repo_root, artifact_dir)),
            "changed_input_summary": Value::Null,
            "stop_condition": format!("repair_loop_{repair_loop_status}"),
            "next_action": if repair_loop_status == "repair_planned" { "fda review after implementer repair" } else if repair_loop_status == "no_repair_needed" { "fda merge" } else { "resolve repair loop blocker or Human Decision" },
            "automation_boundary": "PR-V1-008 plans repair and retry history; merge, release, notification, and Output Hub are later gates",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("repair_receipt.json")),
                display_path(repo_root, &out_dir.join("retry_history.json")),
                display_path(repo_root, &out_dir.join("failure_classification.json"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}

pub(crate) fn repair_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    generated_at_unix_seconds: u64,
) -> Value {
    let now = generated_at_unix_seconds;
    let specs = [
        (
            "ART-REPAIR-001",
            "generic_receipt",
            "Repair Prompt",
            "Implementer repair prompt generated from QA failure",
            "repair_prompt.md",
        ),
        (
            "ART-REPAIR-002",
            "generic_receipt",
            "Failure Classification",
            "QA failure classification and routing",
            "failure_classification.json",
        ),
        (
            "ART-REPAIR-003",
            "generic_receipt",
            "Retry History",
            "Retry attempt count and same-cause cap",
            "retry_history.json",
        ),
        (
            "ART-REPAIR-004",
            "generic_receipt",
            "Repair Receipt",
            "Repair loop semantic receipt",
            "repair_receipt.json",
        ),
        (
            "ART-REPAIR-005",
            "runner_explanation",
            "Runner Explanation",
            "Repair loop stop condition and next action",
            "runner_explanation.json",
        ),
        (
            "ART-REPAIR-006",
            "validation_report",
            "Validation Report",
            "Repair loop artifact validation",
            "validation_report.json",
        ),
    ];
    let artifacts = specs
        .iter()
        .map(
            |(artifact_id, artifact_type, title, preview_summary, file_name)| {
                artifact_inventory_entry(
                    repo_root,
                    out_dir,
                    context,
                    ArtifactInventorySpec {
                        artifact_id,
                        artifact_type,
                        title,
                        preview_summary,
                        file_name,
                        timestamp: now,
                    },
                )
            },
        )
        .collect::<Vec<_>>();
    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": artifacts,
        "groups": [
            {
                "group_id": "repair_loop",
                "title": "Repair Loop Artifacts",
                "artifact_ids": ["ART-REPAIR-001", "ART-REPAIR-002", "ART-REPAIR-003", "ART-REPAIR-004"]
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-REPAIR-005", "ART-REPAIR-006"]
            }
        ]
    })
}
