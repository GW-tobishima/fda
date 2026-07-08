use serde::Serialize;
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};

use crate::application::decisions::{value_string, value_string_array};
use crate::application::ports::ArtifactStore;
use crate::application::profile::{
    ensure_repository_profile, ensure_target_repository_profile_if_present,
};
use crate::application::review::review_runtime_context;
use crate::application::validate::{validate_artifacts, write_report};
use crate::cli::args::{AtoConfig, ContinueConfig, ValidateConfig};
use crate::domain::entities::RuntimeContext;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::json_file::{read_json_value, write_json_file};
use crate::infra::paths::canonicalize_existing_or_parent;
use crate::rendering::repair::*;
use crate::support::paths::{display_path, resolve_path};
use crate::{
    carry_forward_repair_artifacts, now_unix_seconds, write_text_file, DEFAULT_MODEL_CONTRACT_DIRS,
    DEFAULT_SCHEMA_DIR,
};

#[derive(Debug, Serialize)]
pub(crate) struct ContinueResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: String,
    pub(crate) repair_loop_status: String,
    pub(crate) artifact_dir: String,
    pub(crate) out_dir: String,
    pub(crate) target_repo: String,
    pub(crate) retry_attempt_count: u32,
    pub(crate) retry_limit: u32,
    pub(crate) failure_classification: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) validation_report_path: Option<String>,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) struct RepairGateStatus {
    pub(crate) status: String,
    pub(crate) issues: Vec<String>,
    pub(crate) qa_status: String,
    pub(crate) functional_qa_status: String,
    pub(crate) security_qa_status: String,
    pub(crate) return_to_role: Option<String>,
    pub(crate) actual_pr_url: Option<String>,
    pub(crate) reviewed_planned_pr_id: String,
    pub(crate) findings: Vec<String>,
    pub(crate) evidence_links: Vec<String>,
}

pub(crate) fn continue_run(config: &ContinueConfig) -> Result<ContinueResult, String> {
    let store = FsArtifactStore;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    ensure_repository_profile(&store, &repo_root)?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let target_repo_input = resolve_path(&repo_root, &config.target_repo);
    let target_repo = store
        .canonicalize(&target_repo_input)
        .unwrap_or(target_repo_input);
    ensure_target_repository_profile_if_present(&store, &target_repo, &repo_root)?;
    let out_dir = match &config.out {
        Some(out) => resolve_path(&repo_root, out),
        None => env::temp_dir().join(format!("fda-repair-loop-{}", now_unix_seconds())),
    };
    let out_dir_for_safety = canonicalize_existing_or_parent(&out_dir).map_err(|e| {
        format!(
            "failed to resolve continue output dir {} for containment check: {e}",
            out_dir.display()
        )
    })?;
    let target_repo_for_safety = canonicalize_existing_or_parent(&target_repo).map_err(|e| {
        format!(
            "failed to resolve target repo {} for containment check: {e}",
            target_repo.display()
        )
    })?;
    if out_dir_for_safety.starts_with(&target_repo_for_safety) {
        return Err(format!(
            "continue output dir {} must not be inside target repo {}",
            out_dir.display(),
            target_repo.display()
        ));
    }
    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;
    let out_dir = store.canonicalize(&out_dir).unwrap_or(out_dir);

    let context = repair_runtime_context(&artifact_dir);
    let gate = repair_gate_status_from_artifacts(&artifact_dir)?;
    let failure_classification = classify_repair_failure(&gate);
    let existing_attempts = read_retry_attempts(&artifact_dir)?;
    let same_cause_retry_count = existing_attempts
        .iter()
        .filter(|attempt| {
            attempt
                .get("failure_classification")
                .and_then(Value::as_str)
                == Some(failure_classification.as_str())
        })
        .count() as u32;
    let retry_limit_reached =
        gate.status == "repair_required" && same_cause_retry_count >= config.max_retries;
    let repair_loop_status = repair_loop_status(&gate, retry_limit_reached);
    let retry_attempt_count = if gate.status == "repair_required"
        && matches!(repair_loop_status.as_str(), "repair_planned" | "human_turn")
    {
        same_cause_retry_count.saturating_add(1)
    } else {
        same_cause_retry_count
    };
    let verdict =
        if repair_loop_status == "repair_planned" || repair_loop_status == "no_repair_needed" {
            "pass"
        } else if repair_loop_status == "blocked" || repair_loop_status == "human_turn" {
            "blocked"
        } else {
            "fail"
        }
        .to_string();
    let gate_view = RepairGateView {
        status: &gate.status,
        issues: &gate.issues,
        qa_status: &gate.qa_status,
        functional_qa_status: &gate.functional_qa_status,
        security_qa_status: &gate.security_qa_status,
        return_to_role: gate.return_to_role.as_deref(),
        actual_pr_url: gate.actual_pr_url.as_deref(),
        reviewed_planned_pr_id: &gate.reviewed_planned_pr_id,
        findings: &gate.findings,
        evidence_links: &gate.evidence_links,
    };
    let rendered_at = now_unix_seconds();

    let mut artifacts_written = Vec::new();
    write_text_file(
        &out_dir.join("repair_prompt.md"),
        &repair_prompt_markdown(
            &target_repo,
            &context,
            &gate_view,
            &failure_classification,
            retry_attempt_count,
            config.max_retries,
        ),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("repair_prompt.md")));

    let failure_receipt =
        failure_classification_receipt(&context, &gate_view, &failure_classification);
    write_json_file(
        &out_dir.join("failure_classification.json"),
        &failure_receipt,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("failure_classification.json"),
    ));

    let retry_history = retry_history_receipt(RetryHistoryInputs {
        context: &context,
        existing_attempts: &existing_attempts,
        gate: &gate_view,
        failure_classification: &failure_classification,
        repair_loop_status: &repair_loop_status,
        retry_attempt_count,
        retry_limit: config.max_retries,
        created_at_unix_seconds: rendered_at,
    });
    write_json_file(&out_dir.join("retry_history.json"), &retry_history)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("retry_history.json"),
    ));

    let repair_receipt = repair_receipt(
        &context,
        &gate_view,
        &failure_classification,
        &repair_loop_status,
        retry_attempt_count,
        config.max_retries,
    );
    write_json_file(&out_dir.join("repair_receipt.json"), &repair_receipt)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("repair_receipt.json"),
    ));

    carry_forward_repair_artifacts(&repo_root, &artifact_dir, &out_dir, &mut artifacts_written)?;

    write_json_file(
        &out_dir.join("runner_explanation.json"),
        &repair_runner_explanation(
            &repo_root,
            &artifact_dir,
            &out_dir,
            &context,
            &repair_loop_status,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    write_json_file(
        &out_dir.join("artifact_inventory.json"),
        &repair_artifact_inventory(&repo_root, &out_dir, &context, rendered_at),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("artifact_inventory.json"),
    ));

    let validation_report_path = out_dir.join("validation_report.json");
    let validation_config = ValidateConfig {
        repo_root: repo_root.clone(),
        schema_dir: PathBuf::from(DEFAULT_SCHEMA_DIR),
        artifact_dir: out_dir.clone(),
        out: Some(validation_report_path.clone()),
        ato: AtoConfig::default(),
        print_json: false,
        model_contract_dirs: DEFAULT_MODEL_CONTRACT_DIRS
            .iter()
            .map(PathBuf::from)
            .collect(),
    };
    let validation_report = validate_artifacts(&validation_config)?;
    write_report(&validation_report_path, &validation_report)?;
    artifacts_written.push(display_path(&repo_root, &validation_report_path));

    let verdict = if validation_report.verdict == "pass" {
        verdict
    } else {
        "fail".to_string()
    };
    let next_actions = repair_next_actions(&repair_loop_status);

    Ok(ContinueResult {
        schema_version: "fda.continue_result.v0",
        verdict,
        repair_loop_status,
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        target_repo: display_path(&repo_root, &target_repo),
        retry_attempt_count,
        retry_limit: config.max_retries,
        failure_classification,
        artifacts_written,
        validation_report_path: Some(display_path(&repo_root, &validation_report_path)),
        next_actions,
    })
}

pub(crate) fn repair_runtime_context(artifact_dir: &Path) -> RuntimeContext {
    let mut context = review_runtime_context(artifact_dir);
    context.case_ids = vec!["CASE-FDA-V1-REPAIR-001".to_string()];
    context.task_ids = vec!["PR-V1-008".to_string()];
    context
}

fn repair_gate_status_from_artifacts(artifact_dir: &Path) -> Result<RepairGateStatus, String> {
    let qa_path = artifact_dir.join("qa_receipt.json");
    let mut issues = Vec::new();
    let mut findings = Vec::new();
    let mut evidence_links = Vec::new();
    let mut qa_status = "missing".to_string();
    let mut functional_qa_status = "unknown".to_string();
    let mut security_qa_status = "unknown".to_string();
    let mut return_to_role = None;
    let mut actual_pr_url = None;
    let mut reviewed_planned_pr_id = "PR-V1-006".to_string();

    if qa_path.exists() {
        evidence_links.push("qa_receipt.json".to_string());
        let qa = read_json_value(&qa_path)?;
        qa_status = value_string(&qa, "status").unwrap_or_else(|| "unknown".to_string());
        functional_qa_status =
            value_string(&qa, "functional_qa_status").unwrap_or_else(|| "unknown".to_string());
        security_qa_status =
            value_string(&qa, "security_qa_status").unwrap_or_else(|| "unknown".to_string());
        return_to_role = value_string(&qa, "return_to_role");
        actual_pr_url = value_string(&qa, "actual_pr_url");
        reviewed_planned_pr_id =
            value_string(&qa, "reviewed_planned_pr_id").unwrap_or(reviewed_planned_pr_id);
        findings.extend(value_string_array(&qa, "review_gate_issues"));
    } else {
        issues.push(format!(
            "qa_receipt.json is required before fda continue: {}",
            qa_path.display()
        ));
    }

    for (file_name, aggregate_status) in [
        ("functional_qa_receipt.json", functional_qa_status.clone()),
        ("security_qa_receipt.json", security_qa_status.clone()),
    ] {
        let path = artifact_dir.join(file_name);
        if path.exists() {
            evidence_links.push(file_name.to_string());
            let receipt = read_json_value(&path)?;
            let receipt_status =
                value_string(&receipt, "status").unwrap_or_else(|| "unknown".to_string());
            if aggregate_status != "unknown" && aggregate_status != receipt_status {
                issues.push(format!(
                    "qa_receipt.json {file_name} status mismatch: aggregate={aggregate_status}, receipt={receipt_status}"
                ));
            }
            if file_name == "functional_qa_receipt.json" {
                functional_qa_status = receipt_status.clone();
            } else {
                security_qa_status = receipt_status.clone();
            }
            if receipt_status == "failed" || receipt_status == "needs_human" {
                findings.extend(value_string_array(&receipt, "findings"));
            }
        } else if qa_path.exists() {
            issues.push(format!(
                "{file_name} is required before fda continue because qa_receipt.json references per-role QA status"
            ));
        }
    }

    if qa_status == "passed" && (functional_qa_status != "passed" || security_qa_status != "passed")
    {
        issues.push(format!(
            "qa_receipt.json status is passed but per-role receipts are functional={functional_qa_status}, security={security_qa_status}"
        ));
    }

    let status = if !issues.is_empty() {
        "blocked"
    } else if qa_status == "passed" {
        "no_repair_needed"
    } else if return_to_role.as_deref() == Some("implementer") && qa_status == "failed" {
        "repair_required"
    } else if return_to_role
        .as_deref()
        .is_some_and(|role| role.contains("human"))
        || security_qa_status == "needs_human"
    {
        "human_turn"
    } else {
        "blocked"
    }
    .to_string();

    Ok(RepairGateStatus {
        status,
        issues,
        qa_status,
        functional_qa_status,
        security_qa_status,
        return_to_role,
        actual_pr_url,
        reviewed_planned_pr_id,
        findings,
        evidence_links,
    })
}

fn classify_repair_failure(gate: &RepairGateStatus) -> String {
    if gate.status == "no_repair_needed" {
        return "none".to_string();
    }
    if gate.status == "blocked" {
        return "missing_or_blocked_qa_evidence".to_string();
    }
    if gate.security_qa_status == "needs_human"
        || gate
            .return_to_role
            .as_deref()
            .is_some_and(|role| role.contains("human"))
    {
        return "security_human_decision_required".to_string();
    }
    if gate.functional_qa_status == "failed" && gate.security_qa_status == "failed" {
        return "functional_and_security_regression".to_string();
    }
    if gate.functional_qa_status == "failed" {
        return "functional_acceptance_gap".to_string();
    }
    if gate.security_qa_status == "failed" {
        return "security_repairable_gap".to_string();
    }
    "qa_failure".to_string()
}

fn read_retry_attempts(artifact_dir: &Path) -> Result<Vec<Value>, String> {
    let path = artifact_dir.join("retry_history.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let history = read_json_value(&path)?;
    Ok(history
        .get("attempts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

fn repair_loop_status(gate: &RepairGateStatus, retry_limit_reached: bool) -> String {
    match gate.status.as_str() {
        "no_repair_needed" => "no_repair_needed".to_string(),
        "repair_required" if retry_limit_reached => "human_turn".to_string(),
        "repair_required" => "repair_planned".to_string(),
        "human_turn" => "human_turn".to_string(),
        _ => "blocked".to_string(),
    }
}

fn repair_next_actions(repair_loop_status: &str) -> Vec<String> {
    match repair_loop_status {
        "repair_planned" => vec![
            "repair_prompt.md を Implementer へ渡す".to_string(),
            "fda review".to_string(),
        ],
        "no_repair_needed" => vec!["fda merge".to_string()],
        "human_turn" => vec!["Human Decision を開いて retry 継続可否を判断する".to_string()],
        _ => vec![
            "qa_receipt.json と retry_history.json を確認して fda continue を再実行する"
                .to_string(),
        ],
    }
}
