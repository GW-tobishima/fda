use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    read_json_value, value_string,
};
use crate::application::ports::ArtifactStore;
use crate::cli::args::StatusConfig;
use crate::domain::entities::HumanDecisionSummary;
use crate::domain::policies::decision::decision_blockers;
use crate::infra::fs_store::FsArtifactStore;
use crate::support::paths::{display_path, resolve_path};

#[derive(Debug, Serialize)]
pub(crate) struct StatusResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) artifact_dir: String,
    pub(crate) current_phase: String,
    pub(crate) phase_reason: String,
    pub(crate) unresolved_decisions: Vec<HumanDecisionSummary>,
    pub(crate) notification: NotificationStatus,
    pub(crate) qa: QaStatus,
    pub(crate) repair: RepairStatus,
    pub(crate) merge: MergeStatus,
    pub(crate) next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NotificationStatus {
    pub(crate) request_status: String,
    pub(crate) receipt_status: String,
    pub(crate) channel: Option<String>,
    pub(crate) recipient: Option<String>,
    pub(crate) sent: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct QaStatus {
    pub(crate) functional_qa_status: String,
    pub(crate) security_qa_status: String,
    pub(crate) qa_status: String,
    pub(crate) return_to_role: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepairStatus {
    pub(crate) repair_loop_status: String,
    pub(crate) failure_classification: Option<String>,
    pub(crate) retry_attempt_count: Option<u64>,
    pub(crate) retry_limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MergeStatus {
    pub(crate) merge_gate_status: String,
    pub(crate) policy_disposition: Option<String>,
    pub(crate) ci_status: Option<String>,
    pub(crate) risk_classification: Option<String>,
    pub(crate) actual_pr_url: Option<String>,
}

struct NextActionInputs<'a, Store: ArtifactStore> {
    store: &'a Store,
    artifact_dir: &'a Path,
    repo_root: &'a Path,
    current_phase: &'a str,
    unresolved_decisions: &'a [HumanDecisionSummary],
    qa: &'a QaStatus,
    repair: &'a RepairStatus,
    merge: &'a MergeStatus,
}

pub(crate) fn status(config: &StatusConfig) -> Result<StatusResult, String> {
    let store = FsArtifactStore;
    status_with_store(config, &store)
}

fn status_with_store(
    config: &StatusConfig,
    store: &impl ArtifactStore,
) -> Result<StatusResult, String> {
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    if !store.exists(&artifact_dir) {
        return Err(format!(
            "artifact dir does not exist: {}",
            artifact_dir.display()
        ));
    }

    let unresolved_decisions = unresolved_decisions(store, &artifact_dir)?;
    let notification = notification_status(store, &artifact_dir)?;
    let qa = qa_status(store, &artifact_dir)?;
    let repair = repair_status(store, &artifact_dir)?;
    let merge = merge_status(store, &artifact_dir)?;
    let (current_phase, phase_reason) = infer_phase(
        store,
        &artifact_dir,
        &repo_root,
        &unresolved_decisions,
        &qa,
        &repair,
        &merge,
    )?;
    let next_actions = next_actions(NextActionInputs {
        store,
        artifact_dir: &artifact_dir,
        repo_root: &repo_root,
        current_phase: &current_phase,
        unresolved_decisions: &unresolved_decisions,
        qa: &qa,
        repair: &repair,
        merge: &merge,
    });

    Ok(StatusResult {
        schema_version: "fda.status_result.v0",
        verdict: "pass",
        artifact_dir: display_path(&repo_root, &artifact_dir),
        current_phase,
        phase_reason,
        unresolved_decisions,
        notification,
        qa,
        repair,
        merge,
        next_actions,
    })
}

fn unresolved_decisions(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
) -> Result<Vec<HumanDecisionSummary>, String> {
    let packet_path = artifact_dir.join("human_decision_packet.json");
    if !store.exists(&packet_path) {
        return Ok(Vec::new());
    }
    let packet = read_json_value(store, &packet_path)?;
    let decisions = decision_summaries_from_packet(&packet);
    let mut receipts = read_decision_receipts(store, &artifact_dir.join("decision_receipts.json"))?;
    for (decision_id, receipt) in
        crate::application::decisions::recorded_decision_receipts_from_packet(&packet)
    {
        receipts.entry(decision_id).or_insert(receipt);
    }
    Ok(decision_blockers(
        &decisions,
        &decision_answers_from_receipts(&receipts),
    ))
}

fn notification_status(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
) -> Result<NotificationStatus, String> {
    let request = optional_json(store, &artifact_dir.join("notification_request.json"))?;
    let receipt = optional_json(store, &artifact_dir.join("notification_receipt.json"))?;
    Ok(NotificationStatus {
        request_status: request
            .as_ref()
            .map(|request| {
                if request.get("sendable").and_then(Value::as_bool) == Some(false) {
                    "not_sendable".to_string()
                } else {
                    value_string(request, "status").unwrap_or_else(|| "present".to_string())
                }
            })
            .unwrap_or_else(|| "missing".to_string()),
        receipt_status: receipt
            .as_ref()
            .and_then(|receipt| value_string(receipt, "status"))
            .unwrap_or_else(|| "missing".to_string()),
        channel: request
            .as_ref()
            .and_then(|request| value_string(request, "channel"))
            .or_else(|| {
                receipt
                    .as_ref()
                    .and_then(|receipt| value_string(receipt, "channel"))
            }),
        recipient: request
            .as_ref()
            .and_then(|request| value_string(request, "recipient"))
            .or_else(|| {
                receipt
                    .as_ref()
                    .and_then(|receipt| value_string(receipt, "recipient"))
            }),
        sent: receipt
            .as_ref()
            .and_then(|receipt| receipt.get("sent"))
            .and_then(Value::as_bool),
    })
}

fn qa_status(store: &impl ArtifactStore, artifact_dir: &Path) -> Result<QaStatus, String> {
    let functional = optional_json(store, &artifact_dir.join("functional_qa_receipt.json"))?;
    let security = optional_json(store, &artifact_dir.join("security_qa_receipt.json"))?;
    let qa = optional_json(store, &artifact_dir.join("qa_receipt.json"))?;
    Ok(QaStatus {
        functional_qa_status: status_field(functional.as_ref()),
        security_qa_status: status_field(security.as_ref()),
        qa_status: status_field(qa.as_ref()),
        return_to_role: qa
            .as_ref()
            .and_then(|qa| value_string(qa, "return_to_role"))
            .or_else(|| {
                security
                    .as_ref()
                    .and_then(|security| value_string(security, "return_to_role"))
            })
            .or_else(|| {
                functional
                    .as_ref()
                    .and_then(|functional| value_string(functional, "return_to_role"))
            }),
    })
}

fn repair_status(store: &impl ArtifactStore, artifact_dir: &Path) -> Result<RepairStatus, String> {
    let repair = optional_json(store, &artifact_dir.join("repair_receipt.json"))?;
    Ok(RepairStatus {
        repair_loop_status: status_field(repair.as_ref()),
        failure_classification: repair
            .as_ref()
            .and_then(|repair| value_string(repair, "failure_classification")),
        retry_attempt_count: repair
            .as_ref()
            .and_then(|repair| repair.get("retry_attempt_count"))
            .and_then(Value::as_u64),
        retry_limit: repair
            .as_ref()
            .and_then(|repair| repair.get("retry_limit"))
            .and_then(Value::as_u64),
    })
}

fn merge_status(store: &impl ArtifactStore, artifact_dir: &Path) -> Result<MergeStatus, String> {
    let summary = optional_json(store, &artifact_dir.join("merge_gate_summary.json"))?;
    let receipt = optional_json(store, &artifact_dir.join("merge_receipt.json"))?;
    let source = summary.as_ref().or(receipt.as_ref());
    Ok(MergeStatus {
        merge_gate_status: status_field(source),
        policy_disposition: source.and_then(|value| value_string(value, "policy_disposition")),
        ci_status: source.and_then(|value| value_string(value, "ci_status")),
        risk_classification: source.and_then(|value| value_string(value, "risk_classification")),
        actual_pr_url: source.and_then(|value| value_string(value, "actual_pr_url")),
    })
}

fn infer_phase(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
    repo_root: &Path,
    unresolved_decisions: &[HumanDecisionSummary],
    qa: &QaStatus,
    repair: &RepairStatus,
    merge: &MergeStatus,
) -> Result<(String, String), String> {
    if !unresolved_decisions.is_empty() {
        return Ok((
            "human_turn".to_string(),
            "未解決 Human Decision があります。".to_string(),
        ));
    }
    if let Some((phase, reason)) = end_to_end_receipt_phase(store, artifact_dir)? {
        return Ok((phase, reason));
    }
    if merge.merge_gate_status != "missing" {
        return Ok((
            "merge".to_string(),
            "merge_gate_summary.json または merge_receipt.json があります。".to_string(),
        ));
    }
    if repair.repair_loop_status == "human_turn" {
        return Ok((
            "human_turn".to_string(),
            "repair loop が Human Decision を要求しています。".to_string(),
        ));
    }
    if repair.repair_loop_status == "repair_planned" {
        return Ok((
            "repair".to_string(),
            "repair_receipt.json が repair_planned です。".to_string(),
        ));
    }
    if repair.repair_loop_status == "blocked" {
        return Ok((
            "repair_blocked".to_string(),
            "repair_receipt.json が blocked です。".to_string(),
        ));
    }
    if matches!(
        repair.repair_loop_status.as_str(),
        "no_repair_needed" | "completed" | "repair_completed"
    ) {
        return Ok((
            "ready_to_merge".to_string(),
            "repair loop は閉じています。".to_string(),
        ));
    }
    if qa.qa_status != "missing"
        || qa.functional_qa_status != "missing"
        || qa.security_qa_status != "missing"
    {
        if qa_needs_human(qa) {
            return Ok((
                "human_turn".to_string(),
                "QA が Human Decision を要求しています。".to_string(),
            ));
        }
        if qa.qa_status == "passed"
            && qa.functional_qa_status == "passed"
            && qa.security_qa_status == "passed"
        {
            return Ok((
                "ready_to_merge".to_string(),
                "QA receipts は pass しています。".to_string(),
            ));
        }
        return Ok(("qa".to_string(), "QA receipts があります。".to_string()));
    }
    if store.exists(&artifact_dir.join("implementation_receipt.json"))
        || store.exists(&artifact_dir.join("external_pr_receipt.json"))
    {
        if !live_implementation_ready_for_review(store, artifact_dir)? {
            return Ok((
                "implementation_blocked".to_string(),
                "implementation / external PR receipt が review 前提を満たしていません。"
                    .to_string(),
            ));
        }
        return Ok((
            "ready_for_review".to_string(),
            "implementation / external PR receipt があります。".to_string(),
        ));
    }
    if let Some(receipt) = optional_json(store, &artifact_dir.join("dry_run_receipt.json"))? {
        let status = value_string(&receipt, "status").unwrap_or_else(|| "unknown".to_string());
        if status == "succeeded" && dry_run_ready_for_live(store, artifact_dir, repo_root)? {
            return Ok((
                "ready_for_live_implement".to_string(),
                "dry_run_receipt.json が succeeded です。".to_string(),
            ));
        }
        return Ok((
            "dry_run".to_string(),
            format!("dry_run_receipt.json status={status} です。"),
        ));
    }
    if store.exists(&artifact_dir.join("planned_pr_execution_packet.json"))
        || store.exists(&artifact_dir.join("mcp_agent_invocation_plan.json"))
    {
        return Ok((
            "ready_for_dry_run".to_string(),
            "implementer invocation artifact があります。".to_string(),
        ));
    }
    if store.exists(&artifact_dir.join("basic_design.md"))
        || store.exists(&artifact_dir.join("planned_prs.json"))
    {
        return Ok((
            "ready_for_implement".to_string(),
            "design / planning artifact があります。".to_string(),
        ));
    }
    if store.exists(&artifact_dir.join("requirements_definition.md"))
        || store.exists(&artifact_dir.join("human_decision_packet.json"))
    {
        return Ok((
            "intake".to_string(),
            "intake artifact があります。".to_string(),
        ));
    }
    Ok((
        "unknown".to_string(),
        "phase を推定できる主要artifactが見つかりません。".to_string(),
    ))
}

fn next_actions<Store: ArtifactStore>(inputs: NextActionInputs<'_, Store>) -> Vec<String> {
    let artifact_arg = display_path(inputs.repo_root, inputs.artifact_dir);
    if !inputs.unresolved_decisions.is_empty() {
        return inputs
            .unresolved_decisions
            .iter()
            .map(|decision| {
                format!(
                    "fda decide {} --answer <answer> --artifacts {}",
                    decision.decision_id, artifact_arg
                )
            })
            .collect();
    }
    match inputs.current_phase {
        "intake" => vec![format!("fda design --artifacts {artifact_arg}")],
        "operational_v1_complete" => {
            vec!["Operational V1 proof pack を Output Hub / PR で確認する".to_string()]
        }
        "operational_v1_blocked" | "operational_v1_failed" => {
            vec![end_to_end_next_action(inputs.store, inputs.artifact_dir).unwrap_or_else(
                || {
                    format!(
                        "end_to_end_receipt.json のblockerを解消して fda status --artifacts {artifact_arg}"
                    )
                },
            )]
        }
        "ready_for_implement" | "ready_for_dry_run" | "dry_run" => {
            vec![format!(
                "fda implement --dry-run --artifacts {artifact_arg}"
            )]
        }
        "ready_for_live_implement" => {
            let target_arg =
                dry_run_target_repo_arg(inputs.store, inputs.artifact_dir, inputs.repo_root);
            vec![format!(
                "fda implement --live{} --artifacts {artifact_arg}",
                target_arg
                    .as_deref()
                    .map(|target| format!(" --target-repo {target}"))
                    .unwrap_or_default()
            )]
        }
        "implementation_blocked" => {
            let target_arg = live_target_repo_arg(inputs.store, inputs.artifact_dir, inputs.repo_root)
                .or_else(|| {
                    dry_run_target_repo_arg(inputs.store, inputs.artifact_dir, inputs.repo_root)
                });
            vec![format!(
                "implementation receipt blocker を解消して fda implement --live{} --artifacts {artifact_arg}",
                target_arg
                    .as_deref()
                    .map(|target| format!(" --target-repo {target}"))
                    .unwrap_or_default()
            )]
        }
        "ready_for_review" => {
            let target_arg =
                live_target_repo_arg(inputs.store, inputs.artifact_dir, inputs.repo_root);
            vec![format!(
                "fda review{} --artifacts {artifact_arg}",
                target_arg
                    .as_deref()
                    .map(|target| format!(" --target-repo {target}"))
                    .unwrap_or_default()
            )]
        }
        "human_turn" if inputs.repair.repair_loop_status == "human_turn" => vec![format!(
            "repair retry limit / Human Decision を解決して fda continue --artifacts {artifact_arg}"
        )],
        "human_turn" if functional_qa_needs_human_without_repair_route(inputs.qa) => vec![format!(
            "Functional QA の Human Decision / approval を記録して fda review --artifacts {artifact_arg}"
        )],
        "human_turn" if qa_needs_human(inputs.qa) => vec![format!(
            "QA の Human Decision / approval を記録して fda continue --artifacts {artifact_arg}"
        )],
        "qa" if inputs.qa.qa_status == "failed"
            || inputs.qa.functional_qa_status == "failed"
            || inputs.qa.security_qa_status == "failed" =>
        {
            vec![format!("fda continue --artifacts {artifact_arg}")]
        }
        "qa" => vec![format!(
            "QA blocker を解消して fda review --artifacts {artifact_arg}"
        )],
        "repair" if inputs.repair.repair_loop_status == "repair_planned" => vec![
            "repair_prompt.md を Implementer へ渡す".to_string(),
            format!("修正後に fda review --artifacts {artifact_arg}"),
        ],
        "repair_blocked" => vec![format!(
            "repair gate blocker を解消して fda continue --artifacts {artifact_arg}"
        )],
        "ready_to_merge" => vec![format!("fda merge --artifacts {artifact_arg}")],
        "merge" if inputs.merge.merge_gate_status == "merge_ready" => vec![
            "actual PR を repository policy に従って merge し、merge結果receiptを記録する"
                .to_string(),
        ],
        "merge" if inputs.merge.merge_gate_status == "human_approval_required" => {
            vec!["merge_approval_packet.json をHuman merge approvalへ回す".to_string()]
        }
        "merge" => vec![format!(
            "merge gate blocker を解消して fda merge --artifacts {artifact_arg}"
        )],
        _ => vec![format!(
            "artifact を確認して fda status --artifacts {artifact_arg}"
        )],
    }
}

fn end_to_end_receipt_phase(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
) -> Result<Option<(String, String)>, String> {
    let Some(receipt) = optional_json(store, &artifact_dir.join("end_to_end_receipt.json"))? else {
        return Ok(None);
    };
    let status = value_string(&receipt, "status").unwrap_or_else(|| "unknown".to_string());
    let completion_block_reason = if status == "succeeded" {
        end_to_end_completion_block_reason(store, artifact_dir, &receipt)
    } else {
        None
    };
    let phase = match status.as_str() {
        "succeeded" if completion_block_reason.is_none() => "operational_v1_complete",
        "succeeded" => "operational_v1_blocked",
        "blocked" => "operational_v1_blocked",
        "failed" => "operational_v1_failed",
        _ => "operational_v1_unknown",
    };
    let reason = if let Some(reason) = completion_block_reason {
        format!("end_to_end_receipt.json status=succeeded ですが {reason}")
    } else {
        format!("end_to_end_receipt.json status={status} です。")
    };
    Ok(Some((phase.to_string(), reason)))
}

fn end_to_end_completion_block_reason(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
    receipt: &Value,
) -> Option<&'static str> {
    if receipt
        .get("required_for_operational_v1")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Some("required_for_operational_v1 が true ではありません。");
    }
    if !end_to_end_blocking_issues_are_empty(receipt) {
        return Some("blocking_issues が残っています。");
    }
    if end_to_end_status_array_is_missing(receipt, "stage_gates") {
        return Some("stage_gates が確認できません。");
    }
    if end_to_end_status_array_has_non_passing_status(receipt, "stage_gates", "pass") {
        return Some("stage_gates に未通過の gate が残っています。");
    }
    if !end_to_end_required_stage_gates_are_passing(receipt) {
        return Some("stage_gates に必須 gate の欠落があります。");
    }
    if !end_to_end_stage_gates_have_evidence_links(receipt) {
        return Some("stage_gates の証跡リンクが確認できません。");
    }
    if end_to_end_status_array_is_missing(receipt, "representative_runs") {
        return Some("representative_runs が確認できません。");
    }
    if end_to_end_status_array_has_non_passing_status(receipt, "representative_runs", "passed") {
        return Some("representative_runs に未完了の run が残っています。");
    }
    if !end_to_end_has_fixture_free_live_implementation_run(receipt) {
        return Some("fixture-free live implementation run が確認できません。");
    }
    if !end_to_end_has_required_non_implementation_runs(receipt) {
        return Some("非実装 mode の代表 run が確認できません。");
    }
    if end_to_end_output_hub_proof_is_missing(receipt) {
        return Some("output_hub_proof が確認できません。");
    }
    if end_to_end_output_hub_proof_is_blocking(receipt) {
        return Some("output_hub_proof が未通過です。");
    }
    if !end_to_end_output_hub_proof_has_existing_required_paths(store, artifact_dir, receipt) {
        return Some("output_hub_proof の必須パスが確認できません。");
    }
    None
}

fn end_to_end_blocking_issues_are_empty(receipt: &Value) -> bool {
    receipt
        .get("blocking_issues")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn end_to_end_status_array_is_missing(receipt: &Value, field: &str) -> bool {
    !matches!(
        receipt.get(field).and_then(Value::as_array),
        Some(items) if !items.is_empty()
    )
}

fn end_to_end_status_array_has_non_passing_status(
    receipt: &Value,
    field: &str,
    passing_status: &str,
) -> bool {
    receipt
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                let Some(status) = value_string(item, "status") else {
                    return true;
                };
                status != passing_status
            })
        })
}

fn end_to_end_required_stage_gates_are_passing(receipt: &Value) -> bool {
    const REQUIRED_STAGE_GATES: &[&str] = &[
        "notification_gate",
        "status_gate",
        "merge_execution_gate",
        "live_execution_gate",
        "ato_state_gate",
        "forge_gate",
        "v1_evidence_gate",
    ];
    let Some(gates) = receipt.get("stage_gates").and_then(Value::as_array) else {
        return false;
    };
    REQUIRED_STAGE_GATES.iter().all(|required_id| {
        gates.iter().any(|gate| {
            value_string(gate, "gate_id").as_deref() == Some(*required_id)
                && value_string(gate, "status").as_deref() == Some("pass")
        })
    })
}

fn end_to_end_stage_gates_have_evidence_links(receipt: &Value) -> bool {
    receipt
        .get("stage_gates")
        .and_then(Value::as_array)
        .is_some_and(|gates| gates.iter().all(end_to_end_value_has_evidence_links))
}

fn end_to_end_has_fixture_free_live_implementation_run(receipt: &Value) -> bool {
    let Some(runs) = receipt.get("representative_runs").and_then(Value::as_array) else {
        return false;
    };
    runs.iter().any(|run| {
        value_string(run, "kind").as_deref() == Some("implementation")
            && value_string(run, "status").as_deref() == Some("passed")
            && run.get("fixture_used").and_then(Value::as_bool) == Some(false)
            && end_to_end_evidence_links_include(run, "live_execution_evidence.json")
            && end_to_end_evidence_links_include_github_pull_url(run)
            && end_to_end_run_has_passing_test_status(run)
            && end_to_end_run_has_within_scope_disposition(run)
    })
}

fn end_to_end_has_required_non_implementation_runs(receipt: &Value) -> bool {
    const REQUIRED_NON_IMPLEMENTATION_RUNS: &[&str] = &["research", "uiux", "design_only"];
    let Some(runs) = receipt.get("representative_runs").and_then(Value::as_array) else {
        return false;
    };
    REQUIRED_NON_IMPLEMENTATION_RUNS
        .iter()
        .all(|required_kind| {
            runs.iter().any(|run| {
                value_string(run, "kind").as_deref() == Some(*required_kind)
                    && value_string(run, "status").as_deref() == Some("passed")
                    && end_to_end_value_has_evidence_links(run)
            })
        })
}

fn end_to_end_evidence_links_include(value: &Value, needle: &str) -> bool {
    value
        .get("evidence_links")
        .and_then(Value::as_array)
        .is_some_and(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .any(|link| link.contains(needle))
        })
}

fn end_to_end_evidence_links_include_github_pull_url(value: &Value) -> bool {
    value
        .get("evidence_links")
        .and_then(Value::as_array)
        .is_some_and(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .any(is_github_pull_url)
        })
}

fn end_to_end_value_has_evidence_links(value: &Value) -> bool {
    value
        .get("evidence_links")
        .and_then(Value::as_array)
        .is_some_and(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .any(|link| !link.trim().is_empty())
        })
}

fn end_to_end_run_has_passing_test_status(run: &Value) -> bool {
    let direct_status = value_string(run, "test_status").or_else(|| {
        run.get("checks")
            .and_then(|checks| value_string(checks, "tests"))
    });
    if direct_status
        .as_deref()
        .is_some_and(end_to_end_status_is_passing)
    {
        return true;
    }
    run.get("tests")
        .and_then(Value::as_array)
        .is_some_and(|tests| {
            !tests.is_empty()
                && tests.iter().all(|test| {
                    value_string(test, "status")
                        .as_deref()
                        .is_some_and(end_to_end_status_is_passing)
                })
        })
}

fn end_to_end_run_has_within_scope_disposition(run: &Value) -> bool {
    run.get("scope_disposition")
        .is_some_and(|scope| value_string(scope, "kind").as_deref() == Some("within_scope"))
}

fn end_to_end_status_is_passing(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "pass" | "passed" | "success"
    )
}

fn end_to_end_output_hub_proof_is_missing(receipt: &Value) -> bool {
    receipt
        .get("output_hub_proof")
        .and_then(Value::as_object)
        .is_none()
}

fn end_to_end_output_hub_proof_is_blocking(receipt: &Value) -> bool {
    receipt
        .get("output_hub_proof")
        .is_some_and(|proof| value_string(proof, "verdict").as_deref() != Some("pass"))
}

fn end_to_end_output_hub_proof_has_existing_required_paths(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
    receipt: &Value,
) -> bool {
    const REQUIRED_OUTPUT_HUB_PATHS: &[&str] = &[
        "output_hub_path",
        "decision_inbox_path",
        "execution_status_path",
        "output_hub_receipt_path",
        "status_summary_path",
    ];
    let Some(proof) = receipt.get("output_hub_proof") else {
        return false;
    };
    REQUIRED_OUTPUT_HUB_PATHS.iter().all(|field| {
        let Some(path) = value_string(proof, field) else {
            return false;
        };
        if path.trim().is_empty() {
            return false;
        }
        store.exists(&resolve_artifact_path(artifact_dir, &path))
    })
}

fn resolve_artifact_path(artifact_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        artifact_dir.join(path)
    }
}

fn end_to_end_next_action(store: &impl ArtifactStore, artifact_dir: &Path) -> Option<String> {
    optional_json(store, &artifact_dir.join("end_to_end_receipt.json"))
        .ok()
        .flatten()
        .as_ref()
        .and_then(|receipt| value_string(receipt, "next_action"))
}

fn optional_json(store: &impl ArtifactStore, path: &Path) -> Result<Option<Value>, String> {
    if store.exists(path) {
        read_json_value(store, path).map(Some)
    } else {
        Ok(None)
    }
}

fn status_field(value: Option<&Value>) -> String {
    value
        .and_then(|value| value_string(value, "status"))
        .unwrap_or_else(|| "missing".to_string())
}

fn qa_needs_human(qa: &QaStatus) -> bool {
    matches!(
        qa.qa_status.as_str(),
        "needs_human" | "human_turn" | "human_approval_required"
    ) || matches!(
        qa.functional_qa_status.as_str(),
        "needs_human" | "human_turn" | "human_approval_required"
    ) || matches!(
        qa.security_qa_status.as_str(),
        "needs_human" | "human_turn" | "human_approval_required"
    ) || qa
        .return_to_role
        .as_deref()
        .is_some_and(|role| role.starts_with("human"))
}

fn functional_qa_needs_human_without_repair_route(qa: &QaStatus) -> bool {
    matches!(
        qa.functional_qa_status.as_str(),
        "needs_human" | "human_turn" | "human_approval_required"
    ) && !matches!(
        qa.security_qa_status.as_str(),
        "needs_human" | "human_turn" | "human_approval_required"
    ) && !qa
        .return_to_role
        .as_deref()
        .is_some_and(|role| role.starts_with("human"))
}

fn live_implementation_ready_for_review(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
) -> Result<bool, String> {
    let implementation = optional_json(store, &artifact_dir.join("implementation_receipt.json"))?;
    let external_pr = optional_json(store, &artifact_dir.join("external_pr_receipt.json"))?;
    let Some(implementation) = implementation.as_ref() else {
        return Ok(false);
    };
    let Some(external_pr) = external_pr.as_ref() else {
        return Ok(false);
    };

    let implementation_status = value_string(implementation, "status")
        .unwrap_or_else(|| "missing".to_string())
        .to_ascii_lowercase();
    if implementation_status != "succeeded" {
        return Ok(false);
    }

    let external_status = value_string(external_pr, "status")
        .unwrap_or_else(|| "missing".to_string())
        .to_ascii_lowercase();
    if external_status != "opened" {
        return Ok(false);
    }

    let actual_pr_url = value_string(external_pr, "actual_pr_url");
    if actual_pr_url
        .as_deref()
        .is_none_or(|url| !is_github_pull_url(url))
    {
        return Ok(false);
    }

    let tests_status = external_pr
        .get("checks")
        .and_then(Value::as_object)
        .and_then(|checks| checks.get("tests"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    Ok(tests_status == "passed")
}

fn dry_run_ready_for_live(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
    repo_root: &Path,
) -> Result<bool, String> {
    let Some(receipt) = optional_json(store, &artifact_dir.join("dry_run_receipt.json"))? else {
        return Ok(false);
    };
    Ok(dry_run_receipt_issues(store, repo_root, &receipt).is_empty())
}

fn dry_run_receipt_issues(
    store: &impl ArtifactStore,
    repo_root: &Path,
    receipt: &Value,
) -> Vec<String> {
    let mut issues = Vec::new();
    let status = value_string(receipt, "status").unwrap_or_else(|| "unknown".to_string());
    if status != "succeeded" {
        issues.push(format!("dry_run_receipt status is {status}"));
    }
    match receipt.get("target_repo_mutated").and_then(Value::as_bool) {
        Some(false) => {}
        Some(true) => issues.push("dry_run_receipt indicates target_repo_mutated=true".to_string()),
        None => issues.push("dry_run_receipt target_repo_mutated must be false".to_string()),
    }
    match value_string(receipt, "cwd") {
        Some(cwd) if store.exists(&resolve_path(repo_root, Path::new(&cwd))) => {}
        Some(cwd) => issues.push(format!("dry_run_receipt cwd does not exist: {cwd}")),
        None => issues.push("dry_run_receipt cwd is missing or malformed".to_string()),
    }

    let expected_tools = string_array_field(receipt, "expected_tools", &mut issues);
    for required_tool in ["codex", "codex-reply"] {
        if !expected_tools.iter().any(|tool| tool == required_tool) {
            issues.push(format!(
                "dry_run_receipt expected_tools is missing required tool `{required_tool}`"
            ));
        }
    }
    let detected_tools = string_array_field(receipt, "detected_tools", &mut issues);
    for required_tool in ["codex", "codex-reply"] {
        if !detected_tools.iter().any(|tool| tool == required_tool) {
            issues.push(format!(
                "dry_run_receipt detected_tools is missing required tool `{required_tool}`"
            ));
        }
    }
    let missing_tools = string_array_field(receipt, "missing_tools", &mut issues);
    if !missing_tools.is_empty() {
        issues.push(format!(
            "dry_run_receipt missing tools: {}",
            missing_tools.join(", ")
        ));
    }

    let required_check_ids = [
        "human_decision_guard",
        "cwd",
        "prompt_artifact",
        "approval_policy",
        "forbidden_actions",
        "tools_list",
        "target_repo_mutation",
    ];
    match receipt.get("checks").and_then(Value::as_array) {
        Some(checks) if !checks.is_empty() => {
            let mut seen_check_ids = Vec::new();
            for check in checks {
                let check_id =
                    value_string(check, "check_id").unwrap_or_else(|| "unknown".to_string());
                seen_check_ids.push(check_id.clone());
                match check.get("status").and_then(Value::as_str) {
                    Some("pass") => {}
                    Some(status) => {
                        let summary = value_string(check, "summary").unwrap_or_default();
                        issues.push(format!(
                            "dry-run check failed: {check_id}: {status}: {summary}"
                        ));
                    }
                    None => issues.push(format!(
                        "dry-run check `{check_id}` is missing a valid status"
                    )),
                }
            }
            for required_check_id in required_check_ids {
                if !seen_check_ids
                    .iter()
                    .any(|check_id| check_id == required_check_id)
                {
                    issues.push(format!(
                        "dry_run_receipt checks is missing required check `{required_check_id}`"
                    ));
                }
            }
        }
        _ => issues.push("dry_run_receipt checks must be a non-empty array".to_string()),
    }
    issues
}

fn string_array_field(value: &Value, field: &str, issues: &mut Vec<String>) -> Vec<String> {
    match value.get(field).and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(|item| match item.as_str() {
                Some(value) => Some(value.to_string()),
                None => {
                    issues.push(format!(
                        "dry_run_receipt {field} contains a non-string item"
                    ));
                    None
                }
            })
            .collect(),
        None => {
            issues.push(format!("dry_run_receipt {field} must be an array"));
            Vec::new()
        }
    }
}

fn dry_run_target_repo_arg(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
    repo_root: &Path,
) -> Option<String> {
    optional_json(store, &artifact_dir.join("dry_run_receipt.json"))
        .ok()
        .flatten()
        .as_ref()
        .and_then(|receipt| value_string(receipt, "cwd"))
        .map(|target_repo| {
            display_path(
                repo_root,
                &canonical_or_resolved(store, repo_root, &target_repo),
            )
        })
}

fn live_target_repo_arg(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
    repo_root: &Path,
) -> Option<String> {
    for file_name in ["external_pr_receipt.json", "implementation_receipt.json"] {
        let Some(receipt) = optional_json(store, &artifact_dir.join(file_name))
            .ok()
            .flatten()
        else {
            continue;
        };
        if let Some(target_repo) =
            value_string(&receipt, "target_repo").or_else(|| value_string(&receipt, "cwd"))
        {
            return Some(display_path(
                repo_root,
                &canonical_or_resolved(store, repo_root, &target_repo),
            ));
        }
    }
    None
}

/// Resolve a receipt path and canonicalize it when it exists so that
/// `strip_prefix` against the canonicalized repo root works on Windows,
/// where receipts may carry 8.3 short paths while the repo root is verbatim.
fn canonical_or_resolved(
    store: &impl ArtifactStore,
    repo_root: &Path,
    receipt_path: &str,
) -> PathBuf {
    let resolved = resolve_path(repo_root, &PathBuf::from(receipt_path));
    store.canonicalize(&resolved).unwrap_or(resolved)
}

fn is_github_pull_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://github.com/") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    parts.len() >= 4
        && !parts[0].is_empty()
        && !parts[1].is_empty()
        && parts[2] == "pull"
        && pull_number_prefix(parts[3])
}

fn pull_number_prefix(value: &str) -> bool {
    let digits = value.chars().take_while(|ch| ch.is_ascii_digit()).count();
    digits > 0
        && value[digits..]
            .chars()
            .next()
            .is_none_or(|ch| matches!(ch, '?' | '#'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::AtoConfig;
    use crate::infra::clock::system_unix_seconds;
    use serde_json::json;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let unique = system_unix_seconds();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        FsArtifactStore.create_dir_all(&dir).unwrap();
        dir
    }

    fn write_json(path: &Path, value: &Value) {
        FsArtifactStore.write_json(path, value).unwrap();
    }

    fn write_text(path: &Path, body: &str) {
        FsArtifactStore.write_text(path, body).unwrap();
    }

    fn passing_stage_gates() -> Value {
        json!([
            {"gate_id": "notification_gate", "status": "pass", "evidence_links": ["notification_receipt.json"]},
            {"gate_id": "status_gate", "status": "pass", "evidence_links": ["status_summary.json"]},
            {"gate_id": "merge_execution_gate", "status": "pass", "evidence_links": ["github_merge_receipt.json"]},
            {"gate_id": "live_execution_gate", "status": "pass", "evidence_links": ["live_execution_evidence.json"]},
            {"gate_id": "ato_state_gate", "status": "pass", "evidence_links": ["ato_state_receipt.json"]},
            {"gate_id": "forge_gate", "status": "pass", "evidence_links": ["forge_gate_receipt.json"]},
            {"gate_id": "v1_evidence_gate", "status": "pass", "evidence_links": ["end_to_end_receipt.json"]}
        ])
    }

    fn passing_live_implementation_runs() -> Value {
        json!([
            {
                "run_id": "FDA-V1-IMPLEMENTATION-E2E-001",
                "kind": "implementation",
                "status": "passed",
                "fixture_used": false,
                "evidence_links": [
                    "live_execution_evidence.json",
                    "https://github.com/example/repo/pull/123"
                ],
                "test_status": "passed",
                "scope_disposition": {"kind": "within_scope"}
            },
            {
                "run_id": "FDA-V1-RESEARCH-001",
                "kind": "research",
                "status": "passed",
                "evidence_links": ["research_receipt.json"]
            },
            {
                "run_id": "FDA-V1-UIUX-001",
                "kind": "uiux",
                "status": "passed",
                "evidence_links": ["uiux_receipt.json"]
            },
            {
                "run_id": "FDA-V1-DESIGN-ONLY-001",
                "kind": "design_only",
                "status": "passed",
                "evidence_links": ["design_only_receipt.json"]
            }
        ])
    }

    fn passing_output_hub_proof() -> Value {
        json!({
            "verdict": "pass",
            "output_hub_path": "output_hub.html",
            "decision_inbox_path": "decision_inbox.html",
            "execution_status_path": "execution_status.html",
            "output_hub_receipt_path": "output_hub_receipt.json",
            "status_summary_path": "status_summary.json"
        })
    }

    fn write_output_hub_proof_files(artifacts: &Path) {
        for file_name in [
            "output_hub.html",
            "decision_inbox.html",
            "execution_status.html",
            "output_hub_receipt.json",
            "status_summary.json",
        ] {
            write_text(&artifacts.join(file_name), "{}");
        }
    }

    fn write_passing_dry_run_receipt(artifacts: &Path, target: &Path) {
        write_json(
            &artifacts.join("dry_run_receipt.json"),
            &json!({
                "status": "succeeded",
                "cwd": target.to_string_lossy(),
                "target_repo_mutated": false,
                "expected_tools": ["codex", "codex-reply"],
                "detected_tools": ["codex", "codex-reply"],
                "missing_tools": [],
                "checks": [
                    { "check_id": "human_decision_guard", "status": "pass", "summary": "clear" },
                    { "check_id": "cwd", "status": "pass", "summary": target.to_string_lossy() },
                    { "check_id": "prompt_artifact", "status": "pass", "summary": "codex_prompt.md generated" },
                    { "check_id": "approval_policy", "status": "pass", "summary": "on-request" },
                    { "check_id": "forbidden_actions", "status": "pass", "summary": "merge/release forbidden" },
                    { "check_id": "tools_list", "status": "pass", "summary": "codex and codex-reply detected" },
                    { "check_id": "target_repo_mutation", "status": "pass", "summary": "no mutation" }
                ]
            }),
        );
    }

    #[test]
    fn reports_human_turn_and_notification_state() {
        let repo = temp_dir("fda-status-human-turn");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "decision_packet_id": "HDP-001",
                "program_id": "FDA",
                "epic_id": "EPIC",
                "status": "waiting_human",
                "required_before": "Design Gate",
                "decision_needed": "Choose scope",
                "trigger": "intake",
                "context": "scope",
                "options": [{"id": "approve", "recommended": true}],
                "impact": "blocks design",
                "default_if_no_decision": "stop",
                "forge_mapping": {"human_decision_points": ["HD-FDA-001"]}
            }),
        );
        write_json(
            &artifacts.join("notification_request.json"),
            &json!({"channel": "email", "recipient": "dev@example.com", "sendable": true}),
        );
        write_json(
            &artifacts.join("notification_receipt.json"),
            &json!({"status": "blocked", "sent": false, "channel": "email"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: false,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "human_turn");
        assert_eq!(result.unresolved_decisions.len(), 1);
        assert_eq!(result.notification.receipt_status, "blocked");
        assert_eq!(
            result.next_actions[0],
            "fda decide HD-FDA-001 --answer <answer> --artifacts artifacts"
        );
    }

    #[test]
    fn reports_ready_to_merge_from_passing_qa() {
        let repo = temp_dir("fda-status-ready-merge");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("qa_receipt.json"),
            &json!({"status": "passed"}),
        );
        write_json(
            &artifacts.join("functional_qa_receipt.json"),
            &json!({"status": "passed"}),
        );
        write_json(
            &artifacts.join("security_qa_receipt.json"),
            &json!({"status": "passed"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "ready_to_merge");
        assert_eq!(result.qa.qa_status, "passed");
        assert_eq!(result.merge.merge_gate_status, "missing");
        assert_eq!(result.next_actions, vec!["fda merge --artifacts artifacts"]);
    }

    #[test]
    fn reports_blocked_live_receipts_before_review() {
        let repo = temp_dir("fda-status-live-blocked");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("implementation_receipt.json"),
            &json!({"status": "failed"}),
        );
        write_json(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "status": "opened",
                "actual_pr_url": "https://github.com/example/repo/pull/1",
                "checks": {"tests": "passed"}
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "implementation_blocked");
        assert_eq!(
            result.next_actions,
            vec![
                "implementation receipt blocker を解消して fda implement --live --artifacts artifacts"
            ]
        );
    }

    #[test]
    fn reports_security_qa_needs_human_as_human_turn() {
        let repo = temp_dir("fda-status-qa-human");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("qa_receipt.json"),
            &json!({"status": "blocked", "return_to_role": "human_security_approval"}),
        );
        write_json(
            &artifacts.join("functional_qa_receipt.json"),
            &json!({"status": "passed"}),
        );
        write_json(
            &artifacts.join("security_qa_receipt.json"),
            &json!({"status": "needs_human"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "human_turn");
        assert_eq!(
            result.qa.return_to_role.as_deref(),
            Some("human_security_approval")
        );
        assert_eq!(
            result.next_actions,
            vec!["QA の Human Decision / approval を記録して fda continue --artifacts artifacts"]
        );
    }

    #[test]
    fn reports_functional_qa_needs_human_as_human_turn() {
        let repo = temp_dir("fda-status-functional-qa-human");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("qa_receipt.json"),
            &json!({"status": "blocked"}),
        );
        write_json(
            &artifacts.join("functional_qa_receipt.json"),
            &json!({"status": "needs_human"}),
        );
        write_json(
            &artifacts.join("security_qa_receipt.json"),
            &json!({"status": "passed"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "human_turn");
        assert_eq!(
            result.next_actions,
            vec![
                "Functional QA の Human Decision / approval を記録して fda review --artifacts artifacts"
            ]
        );
    }

    #[test]
    fn reports_retry_limit_repair_as_human_turn() {
        let repo = temp_dir("fda-status-repair-human");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("qa_receipt.json"),
            &json!({"status": "failed", "return_to_role": "implementer"}),
        );
        write_json(
            &artifacts.join("repair_receipt.json"),
            &json!({"status": "human_turn", "failure_classification": "retry_limit_reached"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "human_turn");
        assert_eq!(result.repair.repair_loop_status, "human_turn");
        assert_eq!(
            result.next_actions,
            vec![
                "repair retry limit / Human Decision を解決して fda continue --artifacts artifacts"
            ]
        );
    }

    #[test]
    fn reports_blocked_repair_receipt_as_repair_blocked() {
        let repo = temp_dir("fda-status-repair-blocked");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("repair_receipt.json"),
            &json!({"status": "blocked", "failure_classification": "missing_qa_evidence"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "repair_blocked");
        assert_eq!(result.repair.repair_loop_status, "blocked");
        assert_eq!(
            result.next_actions,
            vec!["repair gate blocker を解消して fda continue --artifacts artifacts"]
        );
    }

    #[test]
    fn reports_merge_ready_as_external_merge_action() {
        let repo = temp_dir("fda-status-merge-ready");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("merge_gate_summary.json"),
            &json!({
                "status": "merge_ready",
                "policy_disposition": "auto_merge_candidate",
                "ci_status": "passed",
                "risk_classification": "low_risk",
                "actual_pr_url": "https://github.com/example/repo/pull/1"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "merge");
        assert_eq!(result.merge.merge_gate_status, "merge_ready");
        assert_eq!(
            result.next_actions,
            vec!["actual PR を repository policy に従って merge し、merge結果receiptを記録する"]
        );
    }

    #[test]
    fn reports_operational_v1_blocked_from_end_to_end_receipt() {
        let repo = temp_dir("fda-status-e2e-blocked");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "blocked",
                "next_action": "fixture-free live execution evidence を回収する"
            }),
        );
        write_json(
            &artifacts.join("merge_gate_summary.json"),
            &json!({"status": "merge_ready"}),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=blocked です。"
        );
        assert_eq!(
            result.next_actions,
            vec!["fixture-free live execution evidence を回収する"]
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_still_has_blockers() {
        let repo = temp_dir("fda-status-e2e-succeeded-with-blockers");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [{"issue_id": "BLO-001"}],
                "next_action": "残 blocker を解消する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが blocking_issues が残っています。"
        );
        assert_eq!(result.next_actions, vec!["残 blocker を解消する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_has_blocked_gate() {
        let repo = temp_dir("fda-status-e2e-succeeded-with-blocked-gate");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": [{"gate_id": "live_execution_gate", "status": "block"}],
                "next_action": "live gate を解消する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが stage_gates に未通過の gate が残っています。"
        );
        assert_eq!(result.next_actions, vec!["live gate を解消する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_has_failed_run() {
        let repo = temp_dir("fda-status-e2e-succeeded-with-failed-run");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": [{"run_id": "live", "kind": "implementation", "status": "failed"}],
                "next_action": "failed run を解消する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが representative_runs に未完了の run が残っています。"
        );
        assert_eq!(result.next_actions, vec!["failed run を解消する"]);
    }

    #[test]
    fn reports_operational_v1_complete_when_succeeded_receipt_has_no_failed_gates_or_runs() {
        let repo = temp_dir("fda-status-e2e-succeeded-complete");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_output_hub_proof_files(&artifacts);
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "Output Hubを確認する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_complete");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded です。"
        );
        assert_eq!(
            result.next_actions,
            vec!["Operational V1 proof pack を Output Hub / PR で確認する"]
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_is_missing_required_e2e_arrays() {
        let repo = temp_dir("fda-status-e2e-succeeded-missing-arrays");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "next_action": "必須E2E配列を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが stage_gates が確認できません。"
        );
        assert_eq!(result.next_actions, vec!["必須E2E配列を補完する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_has_blocked_output_hub_proof() {
        let repo = temp_dir("fda-status-e2e-succeeded-with-blocked-output-hub");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": {"verdict": "block"},
                "next_action": "Output Hub proof を解消する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが output_hub_proof が未通過です。"
        );
        assert_eq!(result.next_actions, vec!["Output Hub proof を解消する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_has_skipped_stage_gate() {
        let repo = temp_dir("fda-status-e2e-succeeded-with-skipped-stage-gate");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": [{"gate_id": "live_execution_gate", "status": "skip"}],
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "skipped gate を解消する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが stage_gates に未通過の gate が残っています。"
        );
        assert_eq!(result.next_actions, vec!["skipped gate を解消する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_is_missing_required_stage_gate() {
        let repo = temp_dir("fda-status-e2e-succeeded-missing-required-stage-gate");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": [{"gate_id": "status_gate", "status": "pass"}],
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "必須 gate を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが stage_gates に必須 gate の欠落があります。"
        );
        assert_eq!(result.next_actions, vec!["必須 gate を補完する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_has_skipped_representative_run() {
        let repo = temp_dir("fda-status-e2e-succeeded-with-skipped-run");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": [{"run_id": "live", "kind": "implementation", "status": "skipped"}],
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "skipped run を解消する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが representative_runs に未完了の run が残っています。"
        );
        assert_eq!(result.next_actions, vec!["skipped run を解消する"]);
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_lacks_fixture_free_live_run() {
        let repo = temp_dir("fda-status-e2e-succeeded-without-fixture-free-live-run");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": [
                    {
                        "run_id": "FDA-V1-IMPLEMENTATION-E2E-001",
                        "kind": "implementation",
                        "status": "passed",
                        "fixture_used": true,
                        "evidence_links": [
                            "live_execution_evidence.json",
                            "https://github.com/example/repo/pull/123"
                        ]
                    }
                ],
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "fixture-free live run を回収する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが fixture-free live implementation run が確認できません。"
        );
        assert_eq!(
            result.next_actions,
            vec!["fixture-free live run を回収する"]
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_output_hub_proof_lacks_required_paths() {
        let repo = temp_dir("fda-status-e2e-succeeded-output-hub-missing-paths");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": {"verdict": "pass"},
                "next_action": "Output Hub proof path を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが output_hub_proof の必須パスが確認できません。"
        );
        assert_eq!(
            result.next_actions,
            vec!["Output Hub proof path を補完する"]
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_succeeded_receipt_is_not_required_for_operational_v1() {
        let repo = temp_dir("fda-status-e2e-succeeded-not-required");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": false,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "Operational V1 required receipt を回収する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが required_for_operational_v1 が true ではありません。"
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_required_stage_gate_lacks_evidence_links() {
        let repo = temp_dir("fda-status-e2e-succeeded-gate-without-evidence");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": [
                    {"gate_id": "notification_gate", "status": "pass", "evidence_links": []},
                    {"gate_id": "status_gate", "status": "pass", "evidence_links": ["status_summary.json"]},
                    {"gate_id": "merge_execution_gate", "status": "pass", "evidence_links": ["github_merge_receipt.json"]},
                    {"gate_id": "live_execution_gate", "status": "pass", "evidence_links": ["live_execution_evidence.json"]},
                    {"gate_id": "ato_state_gate", "status": "pass", "evidence_links": ["ato_state_receipt.json"]},
                    {"gate_id": "forge_gate", "status": "pass", "evidence_links": ["forge_gate_receipt.json"]},
                    {"gate_id": "v1_evidence_gate", "status": "pass", "evidence_links": ["end_to_end_receipt.json"]}
                ],
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "gate evidence を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが stage_gates の証跡リンクが確認できません。"
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_github_pr_url_is_split_across_evidence_links() {
        let repo = temp_dir("fda-status-e2e-succeeded-split-pr-url");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": [
                    {
                        "run_id": "FDA-V1-IMPLEMENTATION-E2E-001",
                        "kind": "implementation",
                        "status": "passed",
                        "fixture_used": false,
                        "evidence_links": [
                            "live_execution_evidence.json",
                            "https://github.com/example/repo",
                            "docs/pull/evidence.md"
                        ],
                        "test_status": "passed",
                        "scope_disposition": {"kind": "within_scope"}
                    },
                    {"run_id": "FDA-V1-RESEARCH-001", "kind": "research", "status": "passed", "evidence_links": ["research_receipt.json"]},
                    {"run_id": "FDA-V1-UIUX-001", "kind": "uiux", "status": "passed", "evidence_links": ["uiux_receipt.json"]},
                    {"run_id": "FDA-V1-DESIGN-ONLY-001", "kind": "design_only", "status": "passed", "evidence_links": ["design_only_receipt.json"]}
                ],
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "actual PR URL を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが fixture-free live implementation run が確認できません。"
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_live_run_lacks_test_or_scope_evidence() {
        let repo = temp_dir("fda-status-e2e-succeeded-missing-test-scope");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": [
                    {
                        "run_id": "FDA-V1-IMPLEMENTATION-E2E-001",
                        "kind": "implementation",
                        "status": "passed",
                        "fixture_used": false,
                        "evidence_links": [
                            "live_execution_evidence.json",
                            "https://github.com/example/repo/pull/123"
                        ]
                    },
                    {"run_id": "FDA-V1-RESEARCH-001", "kind": "research", "status": "passed", "evidence_links": ["research_receipt.json"]},
                    {"run_id": "FDA-V1-UIUX-001", "kind": "uiux", "status": "passed", "evidence_links": ["uiux_receipt.json"]},
                    {"run_id": "FDA-V1-DESIGN-ONLY-001", "kind": "design_only", "status": "passed", "evidence_links": ["design_only_receipt.json"]}
                ],
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "test / scope evidence を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが fixture-free live implementation run が確認できません。"
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_non_implementation_representative_runs_are_missing() {
        let repo = temp_dir("fda-status-e2e-succeeded-missing-non-implementation");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": [
                    {
                        "run_id": "FDA-V1-IMPLEMENTATION-E2E-001",
                        "kind": "implementation",
                        "status": "passed",
                        "fixture_used": false,
                        "evidence_links": [
                            "live_execution_evidence.json",
                            "https://github.com/example/repo/pull/123"
                        ],
                        "test_status": "passed",
                        "scope_disposition": {"kind": "within_scope"}
                    }
                ],
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "非実装 mode evidence を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが 非実装 mode の代表 run が確認できません。"
        );
    }

    #[test]
    fn keeps_operational_v1_blocked_when_output_hub_proof_paths_do_not_exist() {
        let repo = temp_dir("fda-status-e2e-succeeded-output-hub-files-missing");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("end_to_end_receipt.json"),
            &json!({
                "schema_version": "fda.end_to_end_receipt.v0",
                "status": "succeeded",
                "required_for_operational_v1": true,
                "blocking_issues": [],
                "stage_gates": passing_stage_gates(),
                "representative_runs": passing_live_implementation_runs(),
                "output_hub_proof": passing_output_hub_proof(),
                "next_action": "Output Hub proof file を補完する"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "operational_v1_blocked");
        assert_eq!(
            result.phase_reason,
            "end_to_end_receipt.json status=succeeded ですが output_hub_proof の必須パスが確認できません。"
        );
    }

    #[test]
    fn reports_ready_for_live_with_target_repo_from_dry_run_receipt() {
        let repo = temp_dir("fda-status-ready-live-target");
        let artifacts = repo.join("artifacts");
        let target = repo.join("target");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        FsArtifactStore.create_dir_all(&target).unwrap();
        write_passing_dry_run_receipt(&artifacts, &target);

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "ready_for_live_implement");
        assert_eq!(
            result.next_actions,
            vec!["fda implement --live --target-repo target --artifacts artifacts"]
        );
    }

    #[test]
    fn validates_dry_run_receipt_before_live_handoff() {
        let repo = temp_dir("fda-status-dry-run-gate");
        let artifacts = repo.join("artifacts");
        let target = repo.join("target");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        FsArtifactStore.create_dir_all(&target).unwrap();
        write_json(
            &artifacts.join("dry_run_receipt.json"),
            &json!({
                "status": "succeeded",
                "cwd": target.to_string_lossy(),
                "target_repo_mutated": false,
                "expected_tools": ["codex", "codex-reply"],
                "detected_tools": ["codex", "codex-reply"],
                "missing_tools": [],
                "checks": [
                    { "check_id": "human_decision_guard", "status": "pass", "summary": "clear" }
                ]
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "dry_run");
        assert_eq!(
            result.next_actions,
            vec!["fda implement --dry-run --artifacts artifacts"]
        );
    }

    #[test]
    fn reports_ready_for_review_with_target_repo_from_external_receipt() {
        let repo = temp_dir("fda-status-review-target");
        let artifacts = repo.join("artifacts");
        let target = repo.join("target");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        FsArtifactStore.create_dir_all(&target).unwrap();
        write_json(
            &artifacts.join("implementation_receipt.json"),
            &json!({"status": "succeeded"}),
        );
        write_json(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "status": "opened",
                "target_repo": target.to_string_lossy(),
                "actual_pr_url": "https://github.com/example/repo/pull/123",
                "checks": {"tests": "passed"}
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "ready_for_review");
        assert_eq!(
            result.next_actions,
            vec!["fda review --target-repo target --artifacts artifacts"]
        );
    }

    #[test]
    fn reports_merge_human_approval_as_external_approval_action() {
        let repo = temp_dir("fda-status-merge-human-approval");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("merge_gate_summary.json"),
            &json!({
                "status": "human_approval_required",
                "policy_disposition": "human_approval_required",
                "ci_status": "passed",
                "risk_classification": "regulated",
                "actual_pr_url": "https://github.com/example/repo/pull/1"
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "merge");
        assert_eq!(result.merge.merge_gate_status, "human_approval_required");
        assert_eq!(
            result.next_actions,
            vec!["merge_approval_packet.json をHuman merge approvalへ回す"]
        );
    }

    #[test]
    fn blocks_ready_for_review_when_actual_pr_url_is_not_github_pull() {
        let repo = temp_dir("fda-status-invalid-pr-url");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("implementation_receipt.json"),
            &json!({"status": "succeeded"}),
        );
        write_json(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "status": "opened",
                "actual_pr_url": "unavailable:adapter",
                "checks": {"tests": "passed"}
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "implementation_blocked");
    }

    #[test]
    fn accepts_normalized_github_pull_url_variants_for_review_ready() {
        let repo = temp_dir("fda-status-pr-url-variants");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("implementation_receipt.json"),
            &json!({"status": "succeeded"}),
        );
        write_json(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "status": "opened",
                "actual_pr_url": "https://github.com/example/repo/pull/123?foo=bar#discussion",
                "checks": {"tests": "passed"}
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "ready_for_review");
        assert_eq!(
            result.next_actions,
            vec!["fda review --artifacts artifacts"]
        );
    }

    #[test]
    fn blocks_review_ready_when_receipt_status_synonyms_would_fail_review_gate() {
        let repo = temp_dir("fda-status-review-gate-statuses");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_json(
            &artifacts.join("implementation_receipt.json"),
            &json!({"status": "success"}),
        );
        write_json(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "status": "open",
                "actual_pr_url": "https://github.com/example/repo/pull/123",
                "checks": {"tests": "success"}
            }),
        );

        let config = StatusConfig {
            repo_root: repo.clone(),
            artifact_dir: std::path::PathBuf::from("artifacts"),
            ato: AtoConfig::default(),
            print_json: true,
        };
        let result = status(&config).unwrap();

        assert_eq!(result.current_phase, "implementation_blocked");
    }
}
