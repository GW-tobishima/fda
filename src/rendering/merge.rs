use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::RuntimeContext;
use crate::rendering::inventory::{artifact_inventory_entry, ArtifactInventorySpec};
use crate::support::paths::display_path;

pub(crate) struct MergeGateView<'a> {
    pub(crate) status: &'a str,
    pub(crate) issues: &'a [String],
    pub(crate) qa_status: &'a str,
    pub(crate) repair_status: &'a str,
    pub(crate) ci_status: &'a str,
    pub(crate) forge_status: &'a str,
    pub(crate) forge_promotion_decision: &'a str,
    pub(crate) forge_claim_ids: &'a [String],
    pub(crate) forge_proof_obligations: &'a [String],
    pub(crate) risk_classification: &'a str,
    pub(crate) policy_disposition: &'a str,
    pub(crate) planned_pr_id: &'a str,
    pub(crate) actual_pr_url: Option<&'a str>,
    pub(crate) expected_head_sha: Option<&'a str>,
    pub(crate) evidence_links: &'a [String],
}

pub(crate) struct MergeExecutionView<'a> {
    pub(crate) requested: bool,
    pub(crate) method: &'a str,
    pub(crate) status: &'a str,
    pub(crate) merge_executed: bool,
    pub(crate) github_merge_receipt_path: Option<&'a Path>,
    pub(crate) failure_reason: Option<&'a str>,
    pub(crate) resume_command: Option<&'a str>,
}

pub(crate) struct GithubMergeDetailsView<'a> {
    pub(crate) merge_sha: &'a str,
    pub(crate) merged_at: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) actual_pr_url: &'a str,
}

pub(crate) struct GithubMergeReceiptCollectionFailureView<'a> {
    pub(crate) actual_pr_url: &'a str,
    pub(crate) failure_reason: &'a str,
    pub(crate) receipt_collection_command: &'a str,
}

pub(crate) fn github_merge_receipt_success(
    context: &RuntimeContext,
    gate: &MergeGateView<'_>,
    method: &str,
    started_at: &str,
    completed_at: &str,
    details: &GithubMergeDetailsView<'_>,
) -> Value {
    json!({
        "schema_version": "fda.github_merge_receipt.v0",
        "receipt_id": "GHMERGE-FDA-V1-014-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": details.actual_pr_url,
        "status": "succeeded",
        "merge_executed": true,
        "merge_method": method,
        "expected_head_sha": gate.expected_head_sha,
        "merge_sha": details.merge_sha,
        "merged_at": details.merged_at,
        "actor": details.actor,
        "started_at": started_at,
        "completed_at": completed_at,
        "failure_reason": Value::Null,
        "resume_command": Value::Null,
        "receipt_collection_command": Value::Null,
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn github_merge_receipt_collection_failure(
    context: &RuntimeContext,
    gate: &MergeGateView<'_>,
    method: &str,
    started_at: &str,
    completed_at: &str,
    details: &GithubMergeReceiptCollectionFailureView<'_>,
) -> Value {
    json!({
        "schema_version": "fda.github_merge_receipt.v0",
        "receipt_id": "GHMERGE-FDA-V1-014-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": details.actual_pr_url,
        "status": "receipt_collection_failed",
        "merge_executed": true,
        "merge_method": method,
        "expected_head_sha": gate.expected_head_sha,
        "merge_sha": Value::Null,
        "merged_at": Value::Null,
        "actor": Value::Null,
        "started_at": started_at,
        "completed_at": completed_at,
        "failure_reason": details.failure_reason,
        "resume_command": details.receipt_collection_command,
        "receipt_collection_command": details.receipt_collection_command,
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn github_merge_receipt_failure(
    context: &RuntimeContext,
    gate: &MergeGateView<'_>,
    method: &str,
    started_at: &str,
    completed_at: &str,
    failure_reason: &str,
    resume_command: &str,
) -> Value {
    json!({
        "schema_version": "fda.github_merge_receipt.v0",
        "receipt_id": "GHMERGE-FDA-V1-014-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "status": "failed",
        "merge_executed": false,
        "merge_method": method,
        "expected_head_sha": gate.expected_head_sha,
        "merge_sha": Value::Null,
        "merged_at": Value::Null,
        "actor": Value::Null,
        "started_at": started_at,
        "completed_at": completed_at,
        "failure_reason": failure_reason,
        "resume_command": resume_command,
        "receipt_collection_command": Value::Null,
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn merge_resume_command(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    target_repo: &Path,
    method: &str,
) -> String {
    let mut parts = vec![
        "fda".to_string(),
        "merge".to_string(),
        "--artifacts".to_string(),
        display_path(repo_root, artifact_dir),
        "--out".to_string(),
        display_path(repo_root, out_dir),
        "--target-repo".to_string(),
        display_path(repo_root, target_repo),
        "--execute".to_string(),
    ];
    if method != "merge" {
        parts.push("--merge-method".to_string());
        parts.push(method.to_string());
    }
    parts
        .iter()
        .map(|part| shell_arg(part))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn command_display(command: &[String]) -> String {
    command
        .iter()
        .map(|part| shell_arg(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_arg(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '=' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn merge_gate_summary(context: &RuntimeContext, gate: &MergeGateView<'_>) -> Value {
    json!({
        "schema_version": "fda.merge_gate_summary.v0",
        "summary_id": "MERGE-GATE-FDA-V1-009-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "expected_head_sha": gate.expected_head_sha,
        "status": gate.status,
        "qa_status": gate.qa_status,
        "repair_status": gate.repair_status,
        "ci_status": gate.ci_status,
        "forge_status": gate.forge_status,
        "forge_promotion_decision": gate.forge_promotion_decision,
        "forge_claim_ids": gate.forge_claim_ids,
        "forge_proof_obligations": gate.forge_proof_obligations,
        "risk_classification": gate.risk_classification,
        "policy_disposition": gate.policy_disposition,
        "issues": gate.issues,
        "checks": [
            {
                "check_id": "qa_receipt_passed",
                "status": if gate.qa_status == "passed" { "pass" } else { "fail" },
                "summary": format!("qa_status={}", gate.qa_status)
            },
            {
                "check_id": "repair_loop_closed",
                "status": if matches!(gate.repair_status, "not_provided" | "no_repair_needed" | "completed" | "repair_completed") { "pass" } else { "fail" },
                "summary": format!("repair_status={}", gate.repair_status)
            },
            {
                "check_id": "ci_passed",
                "status": if gate.ci_status == "passed" { "pass" } else { "fail" },
                "summary": format!("ci_status={}", gate.ci_status)
            },
            {
                "check_id": "forge_promotion",
                "status": if gate.forge_status == "promote" { "pass" } else { "fail" },
                "summary": format!("forge_status={}, promotion_decision={}", gate.forge_status, gate.forge_promotion_decision)
            },
            {
                "check_id": "merge_policy",
                "status": if gate.policy_disposition == "blocked" { "fail" } else { "pass" },
                "summary": format!("policy_disposition={}", gate.policy_disposition)
            }
        ],
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn merge_policy_decision(context: &RuntimeContext, gate: &MergeGateView<'_>) -> Value {
    json!({
        "schema_version": "fda.merge_policy_decision.v0",
        "decision_id": "MERGE-POLICY-FDA-V1-009-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "expected_head_sha": gate.expected_head_sha,
        "policy_disposition": gate.policy_disposition,
        "auto_merge_allowed": gate.policy_disposition == "auto_merge_candidate",
        "human_approval_required": gate.policy_disposition == "human_approval_required",
        "risk_classification": gate.risk_classification,
        "forge_status": gate.forge_status,
        "forge_promotion_decision": gate.forge_promotion_decision,
        "policy": {
            "auto_merge_disabled_by_default": true,
            "privacy_security_legal_requires_human": true,
            "merge_requires_ci_and_qa_receipts": true,
            "forge_promotion_required": true
        },
        "blocking_issues": gate.issues,
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn forge_promotion_receipt(context: &RuntimeContext, gate: &MergeGateView<'_>) -> Value {
    json!({
        "schema_version": "fda.forge_promotion_receipt.v0",
        "receipt_id": "FORGE-PROMOTION-FDA-V1-017-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "status": gate.forge_status,
        "promotion_decision": gate.forge_promotion_decision,
        "merge_allowed": gate.status == "merge_ready",
        "claim_ids": gate.forge_claim_ids,
        "proof_obligations": gate.forge_proof_obligations,
        "blocking_issues": gate.issues,
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn merge_approval_packet(context: &RuntimeContext, gate: &MergeGateView<'_>) -> Value {
    json!({
        "schema_version": "fda.merge_approval_packet.v0",
        "packet_id": "MERGE-APPROVAL-FDA-V1-009-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "expected_head_sha": gate.expected_head_sha,
        "required": gate.policy_disposition == "human_approval_required",
        "reason": if gate.policy_disposition == "human_approval_required" {
            json!("V1 merge policy requires human approval before merge")
        } else {
            Value::Null
        },
        "recommended_option": if gate.policy_disposition == "human_approval_required" {
            json!("approve merge only after human risk review")
        } else if gate.status == "merge_ready" {
            json!("merge only after recorded human merge approval")
        } else {
            json!("repair missing merge evidence before approval")
        },
        "options": ["approve_merge", "hold_for_repair", "reject_merge"],
        "evidence_links": gate.evidence_links
    })
}

pub(crate) fn merge_receipt(
    context: &RuntimeContext,
    gate: &MergeGateView<'_>,
    execution: &MergeExecutionView<'_>,
    repo_root: &Path,
) -> Value {
    let github_merge_receipt_path = execution
        .github_merge_receipt_path
        .map(|path| display_path(repo_root, path));
    json!({
        "schema_version": "fda.merge_receipt.v0",
        "receipt_id": "MERGE-FDA-V1-009-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": gate.planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "expected_head_sha": gate.expected_head_sha,
        "status": gate.status,
        "policy_disposition": gate.policy_disposition,
        "qa_status": gate.qa_status,
        "repair_status": gate.repair_status,
        "ci_status": gate.ci_status,
        "forge_status": gate.forge_status,
        "forge_promotion_decision": gate.forge_promotion_decision,
        "forge_claim_ids": gate.forge_claim_ids,
        "forge_proof_obligations": gate.forge_proof_obligations,
        "risk_classification": gate.risk_classification,
        "merge_execute_requested": execution.requested,
        "merge_method": execution.method,
        "merge_executed": execution.merge_executed,
        "merge_execution_status": execution.status,
        "github_merge_receipt_path": github_merge_receipt_path,
        "merge_execution_failure_reason": execution.failure_reason,
        "resume_command": execution.resume_command,
        "merge_execution_boundary": if execution.requested {
            "PR-V1-014 GitHub merge execution adapter was requested explicitly."
        } else {
            "fda merge emits merge gate receipts only unless --execute is specified."
        },
        "input_artifacts": ["qa_receipt.json", "external_pr_receipt.json", "repair_receipt.json", "risk_register.json", "forge_projection.json"],
        "output_artifacts": merge_output_artifacts(execution.github_merge_receipt_path.is_some()),
        "issues": gate.issues,
        "evidence_links": gate.evidence_links,
        "next_action": merge_receipt_next_action(gate, execution)
    })
}

pub(crate) fn merge_runner_explanation(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    gate: &MergeGateView<'_>,
    execution: &MergeExecutionView<'_>,
) -> Value {
    let mut completion_evidence = vec![
        display_path(repo_root, &out_dir.join("merge_gate_summary.json")),
        display_path(repo_root, &out_dir.join("merge_policy_decision.json")),
        display_path(repo_root, &out_dir.join("forge_promotion_receipt.json")),
        display_path(repo_root, &out_dir.join("merge_receipt.json")),
    ];
    if let Some(path) = execution.github_merge_receipt_path {
        completion_evidence.push(display_path(repo_root, path));
    }
    let mut failure_evidence = gate.issues.to_vec();
    if let Some(reason) = execution.failure_reason {
        failure_evidence.push(reason.to_string());
    }
    let next_action = merge_next_actions(gate, execution)
        .into_iter()
        .next()
        .unwrap_or_else(|| "no further action".to_string());
    let stop_condition = if execution.requested {
        format!("merge_execution_{}", execution.status)
    } else {
        format!("merge_gate_{}", gate.status)
    };
    json!({
        "runner_explanation": {
            "current_phase": "single_planned_pr_execution",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda merge from {}", display_path(repo_root, artifact_dir)),
            "changed_input_summary": Value::Null,
            "stop_condition": stop_condition,
            "next_action": next_action,
            "automation_boundary": if execution.requested {
                "PR-V1-014 may execute GitHub merge only after merge_ready and either auto_merge_candidate or recorded human_approval_granted preconditions pass."
            } else {
                "Default fda merge remains artifact-only; GitHub merge requires explicit --execute."
            },
            "completion_evidence": completion_evidence,
            "failure_evidence": failure_evidence,
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}

fn merge_output_artifacts(include_github_merge_receipt: bool) -> Vec<&'static str> {
    let mut artifacts = vec![
        "merge_gate_summary.json",
        "merge_policy_decision.json",
        "forge_promotion_receipt.json",
        "merge_approval_packet.json",
        "merge_receipt.json",
    ];
    if include_github_merge_receipt {
        artifacts.push("github_merge_receipt.json");
    }
    artifacts
}

pub(crate) fn merge_receipt_next_action(
    gate: &MergeGateView<'_>,
    execution: &MergeExecutionView<'_>,
) -> String {
    match execution.status {
        "succeeded" => "GitHub merge receiptをOutput Hub/notificationへ渡す".to_string(),
        "receipt_collection_failed" => execution
            .resume_command
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                "GitHub merge後のreceiptを再収集し、github_merge_receipt.jsonを更新する".to_string()
            }),
        "failed" => execution
            .resume_command
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                "GitHub merge failureを修正してfda merge --executeを再実行する".to_string()
            }),
        _ if gate.status == "merge_ready" => {
            "必要なら fda merge --execute を明示してGitHub mergeを実行する".to_string()
        }
        _ if gate.status == "human_approval_required" => "request Human merge approval".to_string(),
        _ if review_packet_reflection_required(gate) => review_packet_reflection_next_action(),
        _ => "repair missing QA/CI/PR evidence before merge".to_string(),
    }
}

pub(crate) fn merge_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    include_github_merge_receipt: bool,
    generated_at_unix_seconds: u64,
) -> Value {
    let now = generated_at_unix_seconds;
    let mut specs = vec![
        (
            "ART-MERGE-001",
            "generic_receipt",
            "Merge Gate Summary",
            "QA, CI, risk, and policy merge gate verdict",
            "merge_gate_summary.json",
        ),
        (
            "ART-MERGE-002",
            "generic_receipt",
            "Merge Policy Decision",
            "Auto merge eligibility or human approval route",
            "merge_policy_decision.json",
        ),
        (
            "ART-MERGE-003",
            "forge_promotion_receipt",
            "Forge Promotion Receipt",
            "Forge Claim, Proof, and PromotionDecision merge gate verdict",
            "forge_promotion_receipt.json",
        ),
        (
            "ART-MERGE-004",
            "generic_receipt",
            "Merge Approval Packet",
            "Human approval packet when policy requires it",
            "merge_approval_packet.json",
        ),
        (
            "ART-MERGE-005",
            "generic_receipt",
            "Merge Receipt",
            "Merge gate semantic receipt",
            "merge_receipt.json",
        ),
        (
            "ART-MERGE-006",
            "runner_explanation",
            "Runner Explanation",
            "Merge gate stop condition and next action",
            "runner_explanation.json",
        ),
        (
            "ART-MERGE-007",
            "validation_report",
            "Validation Report",
            "Merge gate artifact validation",
            "validation_report.json",
        ),
    ];
    if include_github_merge_receipt {
        specs.push((
            "ART-MERGE-008",
            "generic_receipt",
            "GitHub Merge Receipt",
            "GitHub merge execution result",
            "github_merge_receipt.json",
        ));
    }
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
    let merge_execution_artifact_ids = if include_github_merge_receipt {
        vec!["ART-MERGE-008"]
    } else {
        vec!["ART-MERGE-005"]
    };
    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": artifacts,
        "groups": [
            {
                "group_id": "merge_gate",
                "title": "Merge Gate Artifacts",
                "artifact_ids": ["ART-MERGE-001", "ART-MERGE-002", "ART-MERGE-003", "ART-MERGE-004", "ART-MERGE-005"]
            },
            {
                "group_id": "merge_execution",
                "title": "Merge Execution Artifacts",
                "artifact_ids": merge_execution_artifact_ids
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-MERGE-006", "ART-MERGE-007"]
            }
        ]
    })
}

pub(crate) fn merge_next_actions(
    gate: &MergeGateView<'_>,
    execution: &MergeExecutionView<'_>,
) -> Vec<String> {
    match execution.status {
        "succeeded" => vec!["github_merge_receipt.json をOutput Hub/通知へ渡す".to_string()],
        "receipt_collection_failed" => {
            vec![execution
                .resume_command
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| {
                    "GitHub merge後のreceiptを再収集し、github_merge_receipt.jsonを更新する"
                        .to_string()
                })]
        }
        "failed" => vec![execution
            .resume_command
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                "GitHub merge failureを修正してfda merge --executeを再実行する".to_string()
            })],
        _ => match gate.status {
            "merge_ready" => {
                vec!["fda merge --execute を明示してGitHub mergeを実行する".to_string()]
            }
            "human_approval_required" => {
                vec!["merge_approval_packet.json をHuman merge approvalへ回す".to_string()]
            }
            _ if review_packet_reflection_required(gate) => {
                vec![review_packet_reflection_next_action()]
            }
            _ => vec![
                "merge_gate_summary.json の failed check を修正して fda merge を再実行する"
                    .to_string(),
            ],
        },
    }
}

fn review_packet_reflection_required(gate: &MergeGateView<'_>) -> bool {
    gate.issues
        .iter()
        .any(|issue| issue.contains("review_agent_gate_packet.md"))
}

fn review_packet_reflection_next_action() -> String {
    "review_agent_gate_packet.md を artifacts/review_packets/pr-<PR番号>.md に明示反映し、python3 scripts/check_review_agent_gate.py --pr-number <PR番号> を通してから fda merge を再実行する".to_string()
}
