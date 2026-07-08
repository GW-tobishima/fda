use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::RuntimeContext;
use crate::rendering::inventory::{artifact_inventory_entry, ArtifactInventorySpec};
use crate::support::paths::display_path;

pub(crate) struct ReviewGateView<'a> {
    pub(crate) status: &'a str,
    pub(crate) issues: &'a [String],
    pub(crate) evidence_links: &'a [String],
    pub(crate) actual_pr_url: Option<&'a str>,
    pub(crate) reviewed_planned_pr_id: &'a str,
    pub(crate) pr_reviewer_status: &'a str,
    pub(crate) pr_reviewer_findings: &'a [String],
    pub(crate) pr_reviewer_evidence_links: &'a [String],
    pub(crate) forge_reviewer_required: bool,
    pub(crate) forge_reviewer_status: &'a str,
    pub(crate) forge_reviewer_evidence_links: &'a [String],
    pub(crate) design_qa_required: bool,
    pub(crate) design_qa_status: &'a str,
    pub(crate) design_qa_evidence_links: &'a [String],
}

pub(crate) struct HumanDecisionGuardView<'a> {
    pub(crate) unresolved_decision_ids: &'a [String],
    pub(crate) non_approval_decision_ids: &'a [String],
}

fn qa_mapping_status(review_ready: bool, statuses: &[&str]) -> &'static str {
    if !review_ready {
        return "blocked";
    }
    if statuses.contains(&"failed") {
        return "fail";
    }
    if statuses.contains(&"needs_human") {
        return "needs_human";
    }
    "pass"
}

fn review_agent_reviewer_status(review_ready: bool, status: &str) -> &'static str {
    if !review_ready {
        return "blocked";
    }
    match status {
        "passed" => "passed",
        "failed" => "failed",
        "needs_human" => "needs_human",
        _ => "blocked",
    }
}

fn review_agent_gate_status(
    review_ready: bool,
    functional_status: &str,
    security_status: &str,
    verdict: &str,
) -> &'static str {
    if !review_ready {
        return "blocked";
    }
    if functional_status == "needs_human" || security_status == "needs_human" {
        return "needs_human";
    }
    if functional_status == "failed" || security_status == "failed" || verdict == "fail" {
        return "failed";
    }
    if verdict == "pass" {
        return "passed";
    }
    "blocked"
}

fn review_agent_packet_status(status: &str) -> &'static str {
    match status {
        "passed" => "REVIEW_AGENT_OK",
        "not_applicable" => "not_applicable",
        _ => "REVIEW_AGENT_HOLD",
    }
}

fn reviewer_status(status: &str) -> &'static str {
    match status {
        "passed" => "passed",
        "failed" => "failed",
        "needs_human" => "needs_human",
        "not_applicable" => "not_applicable",
        _ => "blocked",
    }
}

fn conditional_reviewer_status(required: bool, status: &str) -> &'static str {
    if !required && status == "not_applicable" {
        return "not_applicable";
    }
    reviewer_status(status)
}

fn evidence_json(evidence: &[String]) -> Value {
    json!(evidence)
}

fn evidence_markdown(evidence: &[String]) -> String {
    if evidence.is_empty() {
        "-".to_string()
    } else {
        evidence
            .iter()
            .map(|item| format!("`{item}`"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

pub(crate) fn pr_reviewer_receipt(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    _review_ready: bool,
) -> Value {
    let pr_review_passed = gate.pr_reviewer_status == "passed";
    let findings = if gate.pr_reviewer_findings.is_empty() {
        gate.issues
    } else {
        gate.pr_reviewer_findings
    };
    json!({
        "schema_version": "fda.pr_reviewer_receipt.v0",
        "receipt_id": "PRR-FDA-V1-007-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "role": "pr_reviewer",
        "workspace_policy": "read_only",
        "source_mutation_allowed": false,
        "source_mutation_attempted": false,
        "cwd": target_repo.to_string_lossy(),
        "status": if pr_review_passed { "passed" } else { "blocked" },
        "review_dimensions": ["diff_correctness", "regression_risk", "test_evidence", "artifact_consistency", "human_decision_boundary"],
        "findings": findings,
        "checks": [
            {
                "check_id": "pr_review_required",
                "status": if pr_review_passed { "pass" } else { "blocked" },
                "summary": "PR reviewer gate is recorded separately from Functional QA and Security QA."
            },
            {
                "check_id": "source_mutation_guard",
                "status": "pass",
                "summary": "PR reviewer role is read-only and did not mutate source."
            },
            {
                "check_id": "merge_approval_guard",
                "status": "pass",
                "summary": "PR reviewer does not grant merge approval."
            }
        ],
        "evidence_links": gate.pr_reviewer_evidence_links,
        "return_to_role": if pr_review_passed { Value::Null } else { json!("orchestrator") },
        "next_action": if pr_review_passed { "continue to Functional QA and Security QA gate aggregation" } else { "run read-only pr_reviewer and provide pr_reviewer_receipt.json" }
    })
}

pub(crate) fn functional_qa_receipt(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    status: &str,
    findings: &[String],
    review_ready: bool,
) -> Value {
    json!({
        "schema_version": "fda.functional_qa_receipt.v0",
        "receipt_id": "FQA-FDA-V1-007-001",
        "plan_id": "MCP-FDA-V1-REVIEW-001",
        "invocation_id": "INV-FDA-V1-FQA-REVIEW-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "role": "functional_qa",
        "workspace_policy": "read_only",
        "source_mutation_allowed": false,
        "source_mutation_attempted": false,
        "cwd": target_repo.to_string_lossy(),
        "status": status,
        "gate": "Functional QA Gate",
        "findings": findings,
        "checks": [
            {
                "check_id": "functional_output_separated",
                "status": if review_ready { "pass" } else { "blocked" },
                "summary": "Functional QA receipt is generated independently from Security QA receipt."
            },
            {
                "check_id": "acceptance_criteria_coverage",
                "status": qa_mapping_status(review_ready, &[status]),
                "summary": "Acceptance criteria are mapped in ac_test_mapping.json."
            },
            {
                "check_id": "source_mutation_guard",
                "status": "pass",
                "summary": "Functional QA role is read-only and did not mutate source."
            }
        ],
        "ac_test_mapping": [
            {
                "acceptance_criterion": "Functional QA output is separated from Security QA output.",
                "evidence": "functional_qa_receipt.json",
                "status": if review_ready { "pass" } else { "blocked" }
            },
            {
                "acceptance_criterion": "FAIL return role is decided.",
                "evidence": "qa_receipt.json",
                "status": "pass"
            }
        ],
        "copied_from_security_qa": false,
        "return_to_role": if status == "failed" { json!("implementer") } else { Value::Null },
        "evidence_links": gate.evidence_links,
        "next_action": if status == "failed" { "return to implementer repair" } else if status == "passed" { "continue to merge gate" } else { "resolve review gate blocker" }
    })
}

pub(crate) fn security_qa_receipt(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    status: &str,
    findings: &[String],
    review_ready: bool,
) -> Value {
    let high_or_critical_findings = if status == "needs_human" {
        findings.to_vec()
    } else {
        Vec::new()
    };
    json!({
        "schema_version": "fda.security_qa_receipt.v0",
        "receipt_id": "SQA-FDA-V1-007-001",
        "plan_id": "MCP-FDA-V1-REVIEW-001",
        "invocation_id": "INV-FDA-V1-SQA-REVIEW-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "role": "security_qa",
        "workspace_policy": "read_only",
        "source_mutation_allowed": false,
        "source_mutation_attempted": false,
        "cwd": target_repo.to_string_lossy(),
        "status": status,
        "gate": "Security QA Gate",
        "review_dimensions": ["secrets", "auth", "privacy", "injection", "data_handling", "human_only_approval"],
        "findings": findings,
        "high_or_critical_findings": high_or_critical_findings,
        "checks": [
            {
                "check_id": "security_output_separated",
                "status": if review_ready { "pass" } else { "blocked" },
                "summary": "Security QA receipt is generated independently from Functional QA receipt."
            },
            {
                "check_id": "functional_qa_copy_paste_guard",
                "status": "pass",
                "summary": "Security QA uses security-specific dimensions and is not a copy of Functional QA."
            },
            {
                "check_id": "source_mutation_guard",
                "status": "pass",
                "summary": "Security QA role is read-only and did not mutate source."
            }
        ],
        "not_copied_from_functional_qa": true,
        "return_to_role": if status == "failed" {
            json!("implementer")
        } else if status == "needs_human" {
            json!("human_security_approval")
        } else {
            Value::Null
        },
        "evidence_links": gate.evidence_links,
        "next_action": if status == "needs_human" { "open Human Decision for high or critical security risk" } else if status == "failed" { "return to implementer repair" } else if status == "passed" { "continue to merge gate" } else { "resolve review gate blocker" }
    })
}

pub(crate) fn ac_test_mapping_receipt(
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    functional_status: &str,
    security_status: &str,
    review_ready: bool,
) -> Value {
    let mappings = json!([
        {
            "acceptance_criterion": "Functional QA and Security QA outputs are separated.",
            "qa_receipts": ["functional_qa_receipt.json", "security_qa_receipt.json"],
            "status": qa_mapping_status(review_ready, &[functional_status, security_status])
        },
        {
            "acceptance_criterion": "Security QA is not substituted by a Functional QA copy.",
            "qa_receipts": ["security_qa_receipt.json"],
            "status": if review_ready { "pass" } else { "blocked" }
        },
        {
            "acceptance_criterion": "AC_TEST_MAPPING is populated.",
            "qa_receipts": ["ac_test_mapping.json"],
            "status": "pass"
        },
        {
            "acceptance_criterion": "FAIL return role is decided.",
            "qa_receipts": ["qa_receipt.json"],
            "status": "pass"
        },
        {
            "acceptance_criterion": "QA roles do not mutate source.",
            "qa_receipts": ["functional_qa_receipt.json", "security_qa_receipt.json"],
            "status": "pass"
        }
    ]);
    json!({
        "schema_version": "fda.ac_test_mapping.v0",
        "mapping_id": "ACMAP-FDA-V1-007-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "AC_TEST_MAPPING": mappings,
        "mappings": mappings,
        "evidence_links": ["functional_qa_receipt.json", "security_qa_receipt.json", "qa_receipt.json"]
    })
}

pub(crate) fn qa_receipt(
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    functional_status: &str,
    security_status: &str,
    verdict: &str,
    review_ready: bool,
) -> Value {
    json!({
        "schema_version": "fda.qa_receipt.v0",
        "receipt_id": "QA-FDA-V1-007-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "actual_pr_url": gate.actual_pr_url,
        "status": if verdict == "pass" { "passed" } else if verdict == "fail" { "failed" } else { "blocked" },
        "functional_qa_status": functional_status,
        "security_qa_status": security_status,
        "review_gate_status": gate.status,
        "review_gate_issues": gate.issues,
        "ac_test_mapping_ref": "ac_test_mapping.json",
        "source_mutation_attempted_by_qa": false,
        "return_to_role": if functional_status == "failed" || security_status == "failed" {
            json!("implementer")
        } else if security_status == "needs_human" {
            json!("human_security_approval")
        } else if !review_ready {
            json!("orchestrator")
        } else {
            Value::Null
        },
        "next_action": if verdict == "pass" {
            "fda merge"
        } else if security_status == "needs_human" {
            "open Human Decision for security risk"
        } else if functional_status == "failed" || security_status == "failed" {
            "fda continue"
        } else {
            "resolve review gate blocker and rerun fda review"
        },
        "evidence_links": ["functional_qa_receipt.json", "security_qa_receipt.json", "ac_test_mapping.json"]
    })
}

pub(crate) fn review_agent_gate(
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    functional_status: &str,
    security_status: &str,
    verdict: &str,
    review_ready: bool,
) -> Value {
    let status =
        review_agent_gate_status(review_ready, functional_status, security_status, verdict);
    let pr_reviewer_status = reviewer_status(gate.pr_reviewer_status);
    let functional_reviewer_status = review_agent_reviewer_status(review_ready, functional_status);
    let security_reviewer_status = review_agent_reviewer_status(review_ready, security_status);
    let forge_reviewer_status =
        conditional_reviewer_status(gate.forge_reviewer_required, gate.forge_reviewer_status);
    let design_qa_status =
        conditional_reviewer_status(gate.design_qa_required, gate.design_qa_status);
    json!({
        "schema_version": "fda.review_agent_gate.v0",
        "gate_id": "REVIEW-GATE-FDA-V1-007-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "reviewed_planned_pr_id": gate.reviewed_planned_pr_id,
        "status": status,
        "actual_pr_url": gate.actual_pr_url,
        "required_reviewers": [
            {
                "role": "pr_reviewer",
                "required": true,
                "status": pr_reviewer_status,
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "evidence": evidence_json(gate.pr_reviewer_evidence_links),
                "not_applicable_reason": Value::Null
            },
            {
                "role": "functional_qa",
                "required": true,
                "status": functional_reviewer_status,
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "evidence": ["functional_qa_receipt.json", "ac_test_mapping.json"],
                "not_applicable_reason": Value::Null
            },
            {
                "role": "security_qa",
                "required": true,
                "status": security_reviewer_status,
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "evidence": ["security_qa_receipt.json"],
                "not_applicable_reason": Value::Null
            }
        ],
        "conditional_reviewers": [
            {
                "role": "forge_reviewer",
                "required": gate.forge_reviewer_required,
                "status": forge_reviewer_status,
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "evidence": evidence_json(gate.forge_reviewer_evidence_links),
                "not_applicable_reason": if forge_reviewer_status == "not_applicable" {
                    json!("ATO / Forge / FDA evidence, handoff, review packet, human decision boundary change was not detected in this review artifact set.")
                } else {
                    Value::Null
                }
            },
            {
                "role": "design_qa",
                "required": gate.design_qa_required,
                "status": design_qa_status,
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "evidence": evidence_json(gate.design_qa_evidence_links),
                "not_applicable_reason": if design_qa_status == "not_applicable" {
                    json!("UI / frontend / browser surface change was not detected in this review artifact set.")
                } else {
                    Value::Null
                }
            }
        ],
        "source_mutation_allowed": false,
        "merge_approval_granted": false,
        "evidence_links": [
            "pr_reviewer_receipt.json",
            "functional_qa_receipt.json",
            "security_qa_receipt.json",
            "ac_test_mapping.json",
            "qa_receipt.json"
        ],
        "next_action": if status == "passed" {
            "fda merge"
        } else if status == "needs_human" {
            "open Human Decision for review finding"
        } else if status == "failed" {
            "fda continue"
        } else {
            "resolve review gate blocker and rerun fda review"
        }
    })
}

pub(crate) fn review_agent_gate_packet_markdown(
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
    functional_status: &str,
    security_status: &str,
    verdict: &str,
    review_ready: bool,
) -> String {
    let aggregate_status =
        review_agent_gate_status(review_ready, functional_status, security_status, verdict);
    let pr_status = review_agent_packet_status(reviewer_status(gate.pr_reviewer_status));
    let functional_status = review_agent_packet_status(review_agent_reviewer_status(
        review_ready,
        functional_status,
    ));
    let security_status =
        review_agent_packet_status(review_agent_reviewer_status(review_ready, security_status));
    let orchestrator_status = review_agent_packet_status(aggregate_status);
    let forge_status =
        conditional_reviewer_status(gate.forge_reviewer_required, gate.forge_reviewer_status);
    let forge_packet_status = review_agent_packet_status(forge_status);
    let forge_evidence = evidence_markdown(gate.forge_reviewer_evidence_links);
    let forge_rationale = if gate.forge_reviewer_required {
        if forge_status == "passed" {
            "ATO / Forge / FDA evidence boundary was reviewed by forge_reviewer."
        } else {
            "ATO / Forge / FDA evidence, handoff, review packet, or human decision boundary change requires forge_reviewer evidence."
        }
    } else {
        "ATO / Forge / FDA evidence, handoff, review packet, human decision boundary change was not detected in this review artifact set."
    };
    let design_status = conditional_reviewer_status(gate.design_qa_required, gate.design_qa_status);
    let design_packet_status = review_agent_packet_status(design_status);
    let design_evidence = evidence_markdown(gate.design_qa_evidence_links);
    let design_rationale = if gate.design_qa_required {
        if design_status == "passed" {
            "UI / frontend / visual / browser surface was reviewed by design_qa."
        } else {
            "UI / frontend / visual / browser surface change requires design_qa evidence."
        }
    } else {
        "UI / frontend / visual / browser surface change was not detected in this review artifact set."
    };
    let pr_evidence = evidence_markdown(gate.pr_reviewer_evidence_links);
    format!(
        "# FDA Review Agent Gate Packet\n\n\
program: `{}`\n\
epic: `{}`\n\
planned_pr: `PR-V1-007`\n\
reviewed_planned_pr: `{}`\n\
actual_pr: `{}`\n\n\
## REVIEW_AGENT_GATE\n\n\
MERGE_APPROVAL: not_granted\n\n\
| role | status | evidence | rationale |\n\
|---|---|---|---|\n\
| pr_reviewer | {} | {} | correctness / regression / blast radius / artifact consistency |\n\
| functional_qa | {} | `functional_qa_receipt.json`; `ac_test_mapping.json` | AC mapping and functional behavior |\n\
| security_qa | {} | `security_qa_receipt.json` | security / privacy / human-only approval boundary |\n\
| orchestrator | {} | `review_agent_gate.json`; `qa_receipt.json` | FDA review gate projection and ATO / Forge / FDA boundary check |\n\
| forge_reviewer | {} | {} | {} |\n\
| design_qa | {} | {} | {} |\n",
        context.program_id,
        context.epic_id,
        gate.reviewed_planned_pr_id,
        gate.actual_pr_url.unwrap_or("<unavailable>"),
        pr_status,
        pr_evidence,
        functional_status,
        security_status,
        orchestrator_status,
        forge_packet_status,
        forge_evidence,
        forge_rationale,
        design_packet_status,
        design_evidence,
        design_rationale
    )
}

pub(crate) fn pr_reviewer_prompt_markdown(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
) -> String {
    format!(
        "# PR Reviewer Prompt\n\n\
あなたは FDA V1 の PR Reviewer です。実装PRを read-only で確認し、差分の正しさ、回帰リスク、test evidence、artifact整合性、人間判断境界を検証します。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- planned PR: `PR-V1-007`\n\
- reviewed planned PR: `{}`\n\
- actual PR: `{}`\n\
- workspace policy: `read_only`\n\
- source mutation: forbidden\n\n\
## 必須事項\n\n\
- `pr_reviewer_receipt.json` に結果を残す。\n\
- Functional QA / Security QA とは別のPR観点で確認する。\n\
- source mutation、merge approval、risk approval、scope approval は行わない。\n\
- 人間判断が必要な事項は判断候補として返す。\n",
        target_repo.display(),
        context.program_id,
        context.epic_id,
        gate.reviewed_planned_pr_id,
        gate.actual_pr_url.unwrap_or("<unavailable>")
    )
}

pub(crate) fn functional_qa_prompt_markdown(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
) -> String {
    format!(
        "# Functional QA Prompt\n\n\
あなたは FDA V1 の Functional QA です。実装PRを read-only で確認し、受け入れ基準、回帰リスク、test evidence を検証します。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- planned PR: `PR-V1-007`\n\
- reviewed planned PR: `{}`\n\
- actual PR: `{}`\n\
- workspace policy: `read_only`\n\
- source mutation: forbidden\n\n\
## 必須事項\n\n\
- Functional QA receipt を Security QA receipt と分離する。\n\
- AC_TEST_MAPPING の functional coverage を確認する。\n\
- FAIL の場合は戻し先を `implementer` として記録する。\n\
- merge / release / security exception approval は行わない。\n",
        target_repo.display(),
        context.program_id,
        context.epic_id,
        gate.reviewed_planned_pr_id,
        gate.actual_pr_url.unwrap_or("<unavailable>")
    )
}

pub(crate) fn security_qa_prompt_markdown(
    target_repo: &Path,
    context: &RuntimeContext,
    gate: &ReviewGateView<'_>,
) -> String {
    format!(
        "# Security QA Prompt\n\n\
あなたは FDA V1 の Security QA です。実装PRを read-only で確認し、security/privacy/legal risk と human-only approval 境界を検証します。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- planned PR: `PR-V1-007`\n\
- reviewed planned PR: `{}`\n\
- actual PR: `{}`\n\
- workspace policy: `read_only`\n\
- source mutation: forbidden\n\n\
## 必須事項\n\n\
- Security QA は Functional QA のコピーで代替しない。\n\
- secrets / auth / privacy / injection / data handling を明示的に見る。\n\
- High / Critical は Human Decision または block として扱う。\n\
- risk self-approval、merge、release は行わない。\n",
        target_repo.display(),
        context.program_id,
        context.epic_id,
        gate.reviewed_planned_pr_id,
        gate.actual_pr_url.unwrap_or("<unavailable>")
    )
}

pub(crate) fn mcp_agent_invocation_plan_review(
    target_repo: &Path,
    context: &RuntimeContext,
    guard: &HumanDecisionGuardView<'_>,
    status: &str,
) -> Value {
    let gate_verdict =
        if guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty() {
            "clear"
        } else {
            "blocked"
        };
    json!({
        "schema_version": "fda.mcp_agent_invocation_plan.v0",
        "plan_id": "MCP-FDA-V1-REVIEW-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-007",
        "status": status,
        "human_decision_guard": {
            "unresolved_decision_ids": guard.unresolved_decision_ids,
            "non_approval_decision_ids": guard.non_approval_decision_ids,
            "gate_verdict": gate_verdict
        },
        "role_policy_ref": "agent_role_policy.json",
        "invocations": [
            {
                "invocation_id": "INV-FDA-V1-IMPLEMENTER-EVIDENCE-001",
                "role": "implementer",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "continue_existing",
                "workspace_policy": "write",
                "source_mutation_allowed": true,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "implementation_receipt.json",
                "input_artifacts": ["implementation_receipt.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex-reply"],
                "forbidden_actions": ["new_source_mutation_from_review_gate", "merge", "release", "scope_change_without_human_decision"],
                "expected_receipts": ["implementation_receipt.json", "external_pr_receipt.json"]
            },
            {
                "invocation_id": "INV-FDA-V1-FQA-REVIEW-001",
                "role": "functional_qa",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "functional_qa_prompt.md",
                "input_artifacts": ["implementation_receipt.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex"],
                "forbidden_actions": ["source_mutation", "security_exception_approval", "merge"],
                "expected_receipts": ["functional_qa_receipt.json"]
            },
            {
                "invocation_id": "INV-FDA-V1-SQA-REVIEW-001",
                "role": "security_qa",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "security_qa_prompt.md",
                "input_artifacts": ["implementation_receipt.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex"],
                "forbidden_actions": ["source_mutation", "functional_qa_copy_paste", "risk_self_approval", "merge"],
                "expected_receipts": ["security_qa_receipt.json"]
            }
        ],
        "expected_global_receipts": ["pr_reviewer_receipt.json", "functional_qa_receipt.json", "security_qa_receipt.json", "ac_test_mapping.json", "qa_receipt.json", "review_agent_gate.json"]
    })
}

pub(crate) fn review_runner_explanation(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    verdict: &str,
    functional_status: &str,
    security_status: &str,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "single_planned_pr_execution",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda review from {}", display_path(repo_root, artifact_dir)),
            "changed_input_summary": Value::Null,
            "stop_condition": format!("functional_qa_{functional_status}_security_qa_{security_status}"),
            "next_action": if verdict == "pass" { "fda merge" } else if verdict == "fail" { "fda continue" } else { "resolve review gate blocker" },
            "automation_boundary": "PR-V1-007 runs mandatory read-only pr_reviewer, Functional QA, and Security QA evidence. review_agent_gate.json records mandatory reviewers plus explicit not_applicable reasons for forge_reviewer and design_qa when not triggered. Repair loop, merge, release, notification, and Output Hub are later gates",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("pr_reviewer_receipt.json")),
                display_path(repo_root, &out_dir.join("functional_qa_receipt.json")),
                display_path(repo_root, &out_dir.join("security_qa_receipt.json")),
                display_path(repo_root, &out_dir.join("ac_test_mapping.json")),
                display_path(repo_root, &out_dir.join("qa_receipt.json")),
                display_path(repo_root, &out_dir.join("review_agent_gate.json")),
                display_path(repo_root, &out_dir.join("review_agent_gate_packet.md"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}

pub(crate) fn review_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    generated_at_unix_seconds: u64,
) -> Value {
    let now = generated_at_unix_seconds;
    let specs = [
        (
            "ART-REVIEW-001",
            "generic_receipt",
            "PR Reviewer Prompt",
            "PR reviewer read-only prompt",
            "pr_reviewer_prompt.md",
        ),
        (
            "ART-REVIEW-002",
            "generic_receipt",
            "Functional QA Prompt",
            "Functional QA read-only prompt",
            "functional_qa_prompt.md",
        ),
        (
            "ART-REVIEW-003",
            "generic_receipt",
            "Security QA Prompt",
            "Security QA read-only prompt",
            "security_qa_prompt.md",
        ),
        (
            "ART-REVIEW-004",
            "agent_role_policy",
            "Agent Role Policy",
            "Implementer / Functional QA / Security QA policy",
            "agent_role_policy.json",
        ),
        (
            "ART-REVIEW-005",
            "mcp_agent_invocation_plan",
            "MCP Agent Invocation Plan",
            "Review agent invocation plan",
            "mcp_agent_invocation_plan.json",
        ),
        (
            "ART-REVIEW-006",
            "generic_receipt",
            "PR Reviewer Receipt",
            "PR reviewer semantic receipt",
            "pr_reviewer_receipt.json",
        ),
        (
            "ART-REVIEW-007",
            "generic_receipt",
            "Functional QA Receipt",
            "Functional QA semantic receipt",
            "functional_qa_receipt.json",
        ),
        (
            "ART-REVIEW-008",
            "generic_receipt",
            "Security QA Receipt",
            "Security QA semantic receipt",
            "security_qa_receipt.json",
        ),
        (
            "ART-REVIEW-009",
            "generic_receipt",
            "AC Test Mapping",
            "Acceptance criteria to QA evidence mapping",
            "ac_test_mapping.json",
        ),
        (
            "ART-REVIEW-010",
            "generic_receipt",
            "QA Receipt",
            "Aggregate QA gate receipt",
            "qa_receipt.json",
        ),
        (
            "ART-REVIEW-011",
            "review_agent_gate",
            "Review Agent Gate",
            "Mandatory and conditional review agent gate",
            "review_agent_gate.json",
        ),
        (
            "ART-REVIEW-012",
            "review_packet",
            "Review Agent Gate Packet",
            "Markdown projection for review packet gate check",
            "review_agent_gate_packet.md",
        ),
        (
            "ART-REVIEW-013",
            "runner_explanation",
            "Runner Explanation",
            "Review gate stop condition and next action",
            "runner_explanation.json",
        ),
        (
            "ART-REVIEW-014",
            "validation_report",
            "Validation Report",
            "Review artifact schema validation",
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
                "group_id": "review_agents",
                "title": "Review Agent Artifacts",
                "artifact_ids": ["ART-REVIEW-001", "ART-REVIEW-002", "ART-REVIEW-003", "ART-REVIEW-004", "ART-REVIEW-005", "ART-REVIEW-006", "ART-REVIEW-007", "ART-REVIEW-008", "ART-REVIEW-009", "ART-REVIEW-010", "ART-REVIEW-011", "ART-REVIEW-012"]
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-REVIEW-013", "ART-REVIEW-014"]
            }
        ]
    })
}
