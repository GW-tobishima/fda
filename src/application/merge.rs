use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::{Component, Path, PathBuf};

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    recorded_decision_receipts_from_packet, value_string, value_string_array,
};
use crate::application::ports::{
    ArtifactStore, ArtifactValidator, CheckError, ProcessOutput, YamlValidator,
};
use crate::application::profile::{
    ensure_repository_profile, ensure_target_repository_profile_if_present,
};
use crate::application::validate::{validate_artifacts, write_report};
use crate::cli::args::{AtoConfig, MergeConfig, ValidateConfig};
use crate::domain::entities::{HumanDecisionSummary, RuntimeContext};
use crate::domain::policies::decision::answer_is_approval;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::json_file::{read_json_value, write_json_file};
use crate::infra::json_schema::JsonSchemaArtifactValidator;
use crate::infra::paths::canonicalize_existing_or_parent;
use crate::infra::process::{python_launcher, run_process_command};
use crate::infra::yaml::SerdeYamlValidator;
use crate::rendering::merge::*;
use crate::support::paths::{display_path, resolve_path};
use crate::{
    human_decision_guard_with, normalize_github_pull_url, now_unix_seconds, single_line,
    HumanDecisionGuard, DEFAULT_MODEL_CONTRACT_DIRS, DEFAULT_SCHEMA_DIR,
};

#[derive(Debug, Serialize)]
pub(crate) struct MergeResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: String,
    pub(crate) merge_gate_status: String,
    pub(crate) policy_disposition: String,
    pub(crate) ci_status: String,
    pub(crate) forge_status: String,
    pub(crate) forge_promotion_decision: String,
    pub(crate) risk_classification: String,
    pub(crate) merge_execute_requested: bool,
    pub(crate) merge_method: String,
    pub(crate) merge_executed: bool,
    pub(crate) merge_execution_status: String,
    pub(crate) github_merge_receipt_path: Option<String>,
    pub(crate) merge_failure_reason: Option<String>,
    pub(crate) artifact_dir: String,
    pub(crate) out_dir: String,
    pub(crate) target_repo: String,
    pub(crate) actual_pr_url: Option<String>,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) validation_report_path: Option<String>,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) struct MergeGateStatus {
    pub(crate) status: String,
    pub(crate) issues: Vec<String>,
    pub(crate) qa_status: String,
    pub(crate) repair_status: String,
    pub(crate) ci_status: String,
    pub(crate) forge_status: String,
    pub(crate) forge_promotion_decision: String,
    pub(crate) forge_claim_ids: Vec<String>,
    pub(crate) forge_proof_obligations: Vec<String>,
    pub(crate) risk_classification: String,
    pub(crate) policy_disposition: String,
    pub(crate) planned_pr_id: String,
    pub(crate) actual_pr_url: Option<String>,
    pub(crate) expected_head_sha: Option<String>,
    pub(crate) evidence_links: Vec<String>,
}

pub(crate) struct ForgeGateStatus {
    pub(crate) status: String,
    pub(crate) promotion_decision: String,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) proof_obligations: Vec<String>,
    pub(crate) issues: Vec<String>,
    pub(crate) evidence_links: Vec<String>,
}

pub(crate) struct MergeExecutionOutcome {
    pub(crate) requested: bool,
    pub(crate) method: String,
    pub(crate) status: String,
    pub(crate) merge_executed: bool,
    pub(crate) github_merge_receipt_path: Option<PathBuf>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) resume_command: Option<String>,
}

impl MergeExecutionOutcome {
    fn not_requested(method: &str) -> Self {
        Self {
            requested: false,
            method: method.to_string(),
            status: "not_requested".to_string(),
            merge_executed: false,
            github_merge_receipt_path: None,
            failure_reason: None,
            resume_command: None,
        }
    }
}

pub(crate) struct GithubMergeDetails {
    pub(crate) merge_sha: String,
    pub(crate) merged_at: String,
    pub(crate) actor: String,
    pub(crate) actual_pr_url: String,
}

enum GithubMergeAdapterOutcome {
    Succeeded(GithubMergeDetails),
    ReceiptCollectionFailed {
        actual_pr_url: String,
        failure_reason: String,
        receipt_collection_command: String,
    },
}

pub(crate) fn merge_run(config: &MergeConfig) -> Result<MergeResult, String> {
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
        None => env::temp_dir().join(format!("fda-merge-gate-{}", now_unix_seconds())),
    };
    let out_dir_for_safety = canonicalize_existing_or_parent(&out_dir).map_err(|e| {
        format!(
            "failed to resolve merge output dir {} for containment check: {e}",
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
            "merge output dir {} must not be inside target repo {}",
            out_dir.display(),
            target_repo.display()
        ));
    }
    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;
    let out_dir = store.canonicalize(&out_dir).unwrap_or(out_dir);

    let (context, context_issues) = merge_runtime_context(&artifact_dir);
    let gate = merge_gate_status_from_artifacts(
        &artifact_dir,
        &context,
        &repo_root,
        &target_repo,
        context_issues,
    )?;
    let verdict = if gate.status == "merge_ready" {
        "pass"
    } else if gate.status == "human_approval_required" || gate.status == "blocked" {
        "blocked"
    } else {
        "fail"
    }
    .to_string();

    let mut artifacts_written = Vec::new();
    let gate_view = merge_gate_view(&gate);
    let rendered_at = now_unix_seconds();
    let merge_gate_summary = merge_gate_summary(&context, &gate_view);
    write_json_file(
        &out_dir.join("merge_gate_summary.json"),
        &merge_gate_summary,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("merge_gate_summary.json"),
    ));

    let merge_policy_decision = merge_policy_decision(&context, &gate_view);
    write_json_file(
        &out_dir.join("merge_policy_decision.json"),
        &merge_policy_decision,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("merge_policy_decision.json"),
    ));

    let forge_promotion_receipt = forge_promotion_receipt(&context, &gate_view);
    write_json_file(
        &out_dir.join("forge_promotion_receipt.json"),
        &forge_promotion_receipt,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("forge_promotion_receipt.json"),
    ));

    let merge_approval_packet = merge_approval_packet(&context, &gate_view);
    write_json_file(
        &out_dir.join("merge_approval_packet.json"),
        &merge_approval_packet,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("merge_approval_packet.json"),
    ));

    let merge_execution = merge_execution_outcome(
        config,
        &repo_root,
        &artifact_dir,
        &target_repo,
        &out_dir,
        &context,
        &gate,
    )?;
    if let Some(path) = &merge_execution.github_merge_receipt_path {
        artifacts_written.push(display_path(&repo_root, path));
    }
    let execution_view = merge_execution_view(&merge_execution);

    let merge_receipt = merge_receipt(&context, &gate_view, &execution_view, &repo_root);
    write_json_file(&out_dir.join("merge_receipt.json"), &merge_receipt)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("merge_receipt.json"),
    ));

    write_json_file(
        &out_dir.join("runner_explanation.json"),
        &merge_runner_explanation(
            &repo_root,
            &artifact_dir,
            &out_dir,
            &context,
            &gate_view,
            &execution_view,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    write_json_file(
        &out_dir.join("artifact_inventory.json"),
        &merge_artifact_inventory(
            &repo_root,
            &out_dir,
            &context,
            merge_execution.github_merge_receipt_path.is_some(),
            rendered_at,
        ),
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

    let execution_verdict_ok = if merge_execution.requested {
        merge_execution.status == "succeeded"
    } else {
        true
    };
    let verdict = if validation_report.verdict == "pass" && execution_verdict_ok {
        verdict
    } else {
        "fail".to_string()
    };
    let next_actions = merge_next_actions(&gate_view, &execution_view);
    let github_merge_receipt_path = merge_execution
        .github_merge_receipt_path
        .as_ref()
        .map(|path| display_path(&repo_root, path));

    Ok(MergeResult {
        schema_version: "fda.merge_result.v0",
        verdict,
        merge_gate_status: gate.status,
        policy_disposition: gate.policy_disposition,
        ci_status: gate.ci_status,
        forge_status: gate.forge_status,
        forge_promotion_decision: gate.forge_promotion_decision,
        risk_classification: gate.risk_classification,
        merge_execute_requested: merge_execution.requested,
        merge_method: merge_execution.method,
        merge_executed: merge_execution.merge_executed,
        merge_execution_status: merge_execution.status,
        github_merge_receipt_path,
        merge_failure_reason: merge_execution.failure_reason,
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        target_repo: display_path(&repo_root, &target_repo),
        actual_pr_url: gate.actual_pr_url,
        artifacts_written,
        validation_report_path: Some(display_path(&repo_root, &validation_report_path)),
        next_actions,
    })
}

fn merge_human_decision_guard(artifact_dir: &Path) -> Result<HumanDecisionGuard, String> {
    human_decision_guard_with(artifact_dir, merge_answer_is_approval)
}

fn merge_approval_granted(artifact_dir: &Path) -> Result<bool, String> {
    let packet_path = artifact_dir.join("human_decision_packet.json");
    if !packet_path.exists() {
        return Ok(false);
    }
    let packet = read_json_value(&packet_path)?;
    let decisions = decision_summaries_from_packet(&packet);
    let receipts_path = artifact_dir.join("decision_receipts.json");
    let packet_resolved_without_receipts = !receipts_path.exists()
        && packet.get("status").and_then(Value::as_str) == Some("resolved")
        && packet.get("recorded_decision").is_some();
    let receipts = if packet_resolved_without_receipts {
        recorded_decision_receipts_from_packet(&packet)
    } else {
        read_decision_receipts(&FsArtifactStore, &receipts_path)?
    };
    let answers = decision_answers_from_receipts(&receipts);
    Ok(decisions
        .iter()
        .filter(|decision| is_merge_approval_decision(decision))
        .any(|decision| {
            decision_answer_for(decision, &answers)
                .is_some_and(|answer| merge_answer_is_approval(decision, answer))
        }))
}

fn is_merge_approval_decision(decision: &HumanDecisionSummary) -> bool {
    let fields = [
        decision.decision_id.as_str(),
        decision.summary.as_str(),
        decision.required_before.as_str(),
        decision.recommended_option_id.as_str(),
    ];
    fields
        .iter()
        .any(|field| field.to_ascii_lowercase().contains("merge"))
        || decision
            .option_ids
            .iter()
            .any(|option| option.to_ascii_lowercase().contains("merge"))
}

fn decision_answer_for<'a>(
    decision: &HumanDecisionSummary,
    answers: &'a HashMap<String, String>,
) -> Option<&'a str> {
    answers
        .get(&decision.decision_id)
        .or_else(|| {
            decision
                .alias_ids
                .iter()
                .find_map(|alias| answers.get(alias))
        })
        .map(String::as_str)
}

fn merge_gate_view(gate: &MergeGateStatus) -> MergeGateView<'_> {
    MergeGateView {
        status: &gate.status,
        issues: &gate.issues,
        qa_status: &gate.qa_status,
        repair_status: &gate.repair_status,
        ci_status: &gate.ci_status,
        forge_status: &gate.forge_status,
        forge_promotion_decision: &gate.forge_promotion_decision,
        forge_claim_ids: &gate.forge_claim_ids,
        forge_proof_obligations: &gate.forge_proof_obligations,
        risk_classification: &gate.risk_classification,
        policy_disposition: &gate.policy_disposition,
        planned_pr_id: &gate.planned_pr_id,
        actual_pr_url: gate.actual_pr_url.as_deref(),
        expected_head_sha: gate.expected_head_sha.as_deref(),
        evidence_links: &gate.evidence_links,
    }
}

fn merge_execution_view(execution: &MergeExecutionOutcome) -> MergeExecutionView<'_> {
    MergeExecutionView {
        requested: execution.requested,
        method: &execution.method,
        status: &execution.status,
        merge_executed: execution.merge_executed,
        github_merge_receipt_path: execution.github_merge_receipt_path.as_deref(),
        failure_reason: execution.failure_reason.as_deref(),
        resume_command: execution.resume_command.as_deref(),
    }
}

fn github_merge_details_view(details: &GithubMergeDetails) -> GithubMergeDetailsView<'_> {
    GithubMergeDetailsView {
        merge_sha: &details.merge_sha,
        merged_at: &details.merged_at,
        actor: &details.actor,
        actual_pr_url: &details.actual_pr_url,
    }
}

fn merge_runtime_context(artifact_dir: &Path) -> (RuntimeContext, Vec<String>) {
    let mut context = RuntimeContext {
        program_id: "FDA-V1".to_string(),
        epic_id: "EPIC-FDA-V1-MCP".to_string(),
        case_ids: vec!["CASE-FDA-V1-MERGE-001".to_string()],
        task_ids: vec!["PR-V1-009".to_string()],
    };
    let mut issues = Vec::new();
    let mut program_source: Option<(String, String)> = None;
    let mut epic_source: Option<(String, String)> = None;
    for artifact in ["qa_receipt.json", "external_pr_receipt.json"] {
        let path = artifact_dir.join(artifact);
        let Some(value) = path.exists().then(|| read_json_value(&path).ok()).flatten() else {
            continue;
        };
        if let Some(program_id) = value_string(&value, "program_id") {
            match program_source.as_ref() {
                Some((existing, source)) if existing != &program_id => issues.push(format!(
                    "{source} program_id {existing} does not match {artifact} program_id {program_id}"
                )),
                Some(_) => {}
                None => {
                    context.program_id = program_id.clone();
                    program_source = Some((program_id, artifact.to_string()));
                }
            }
        }
        if let Some(epic_id) = value_string(&value, "epic_id") {
            match epic_source.as_ref() {
                Some((existing, source)) if existing != &epic_id => issues.push(format!(
                    "{source} epic_id {existing} does not match {artifact} epic_id {epic_id}"
                )),
                Some(_) => {}
                None => {
                    context.epic_id = epic_id.clone();
                    epic_source = Some((epic_id, artifact.to_string()));
                }
            }
        }
    }
    context.case_ids = vec!["CASE-FDA-V1-MERGE-001".to_string()];
    context.task_ids = vec!["PR-V1-009".to_string()];
    (context, issues)
}

fn merge_gate_status_from_artifacts(
    artifact_dir: &Path,
    context: &RuntimeContext,
    repo_root: &Path,
    target_repo: &Path,
    context_issues: Vec<String>,
) -> Result<MergeGateStatus, String> {
    let mut issues = context_issues;
    let mut evidence_links = Vec::new();
    let mut qa_status = "missing".to_string();
    let mut repair_status = "not_provided".to_string();
    let mut ci_status = "missing".to_string();
    let mut actual_pr_url = None;
    let mut expected_head_sha = None;
    let mut planned_pr_id = "PR-V1-006".to_string();
    let mut qa_planned_pr_id = None;
    let mut qa_actual_pr_url = None;
    let mut changed_files = Vec::new();

    let qa_path = artifact_dir.join("qa_receipt.json");
    if qa_path.exists() {
        evidence_links.push("qa_receipt.json".to_string());
        let qa = read_json_value(&qa_path)?;
        qa_status = value_string(&qa, "status").unwrap_or_else(|| "unknown".to_string());
        if let Some(raw_url) = value_string(&qa, "actual_pr_url") {
            match normalize_github_pull_url(&raw_url) {
                Some(url) => {
                    qa_actual_pr_url = Some(url.clone());
                    actual_pr_url = Some(url);
                }
                None => issues
                    .push("qa_receipt.json actual_pr_url must be a GitHub pull URL".to_string()),
            }
        }
        qa_planned_pr_id = value_string(&qa, "reviewed_planned_pr_id")
            .or_else(|| value_string(&qa, "planned_pr_id"));
        if let Some(id) = qa_planned_pr_id.clone() {
            planned_pr_id = id;
        }
        if qa_status != "passed" {
            issues.push(format!(
                "qa_receipt.json status must be passed, got {qa_status}"
            ));
        }
    } else {
        issues.push(format!(
            "qa_receipt.json is required before fda merge: {}",
            qa_path.display()
        ));
    }

    let repair_path = artifact_dir.join("repair_receipt.json");
    if repair_path.exists() {
        evidence_links.push("repair_receipt.json".to_string());
        let repair = read_json_value(&repair_path)?;
        repair_status = value_string(&repair, "status").unwrap_or_else(|| "unknown".to_string());
        if matches!(
            repair_status.as_str(),
            "repair_planned" | "human_turn" | "blocked"
        ) {
            issues.push(format!(
                "repair_receipt.json status must be no_repair_needed or completed before merge, got {repair_status}"
            ));
        }
    }

    let external_pr_path = artifact_dir.join("external_pr_receipt.json");
    if external_pr_path.exists() {
        evidence_links.push("external_pr_receipt.json".to_string());
        let external_pr = read_json_value(&external_pr_path)?;
        let external_status =
            value_string(&external_pr, "status").unwrap_or_else(|| "unknown".to_string());
        if !is_open_external_pr_status(&external_status) {
            issues.push(format!(
                "external_pr_receipt.json status must be opened/open before merge, got {external_status}"
            ));
        }
        if let Some(external_planned_pr_id) = value_string(&external_pr, "planned_pr_id") {
            if let Some(qa_id) = qa_planned_pr_id.as_ref() {
                if qa_id != &external_planned_pr_id {
                    issues.push(format!(
                        "qa_receipt.json reviewed_planned_pr_id {qa_id} does not match external_pr_receipt.json planned_pr_id {external_planned_pr_id}"
                    ));
                }
            }
            planned_pr_id = external_planned_pr_id;
        } else {
            issues.push(
                "external_pr_receipt.json planned_pr_id is required before merge".to_string(),
            );
        }
        if let Some(raw_url) = value_string(&external_pr, "actual_pr_url") {
            match normalize_github_pull_url(&raw_url) {
                Some(url) => {
                    if let Some(qa_url) = qa_actual_pr_url.as_ref() {
                        if qa_url != &url {
                            issues.push(format!(
                                "qa_receipt.json actual_pr_url {qa_url} does not match external_pr_receipt.json actual_pr_url {url}"
                            ));
                        }
                    }
                    actual_pr_url = actual_pr_url.or(Some(url));
                }
                None => issues.push(
                    "external_pr_receipt.json actual_pr_url must be a GitHub pull URL".to_string(),
                ),
            }
        } else {
            issues.push(
                "external_pr_receipt.json actual_pr_url is required before merge".to_string(),
            );
        }
        if let Some(raw_target_pr_url) = nested_value_string(&external_pr, &["target_pr", "url"]) {
            match normalize_github_pull_url(&raw_target_pr_url) {
                Some(target_pr_url) => {
                    if let Some(current_url) = actual_pr_url.as_ref() {
                        if current_url != &target_pr_url {
                            issues.push(format!(
                                "external_pr_receipt.json target_pr.url {target_pr_url} does not match actual_pr_url {current_url}"
                            ));
                        }
                    }
                }
                None => issues.push(
                    "external_pr_receipt.json target_pr.url must be a GitHub pull URL".to_string(),
                ),
            }
        } else {
            issues.push(
                "external_pr_receipt.json target_pr.url is required before merge".to_string(),
            );
        }
        match nested_value_string(&external_pr, &["target_pr", "state"]) {
            Some(state) if state == "open" => {}
            Some(state) => issues.push(format!(
                "external_pr_receipt.json target_pr.state must be open before merge, got {state}"
            )),
            None => issues.push(
                "external_pr_receipt.json target_pr.state is required before merge".to_string(),
            ),
        }
        if let Some(head_sha) = nested_value_string(&external_pr, &["target_pr", "head_sha"]) {
            let head_sha = head_sha.trim().to_string();
            if head_sha.is_empty() {
                issues.push(
                    "external_pr_receipt.json target_pr.head_sha must not be empty before merge"
                        .to_string(),
                );
            } else {
                expected_head_sha = Some(head_sha);
            }
        } else {
            issues.push(
                "external_pr_receipt.json target_pr.head_sha is required before merge".to_string(),
            );
        }
        ci_status = ci_status_from_external_pr_receipt(&external_pr);
        push_unique_all(
            &mut changed_files,
            value_string_array(&external_pr, "changed_files"),
        );
        if ci_status != "passed" {
            issues.push(format!(
                "external_pr_receipt.json CI status must be passed, got {ci_status}"
            ));
        }
        if has_non_empty_array(&external_pr, "open_issues") {
            issues.push(
                "external_pr_receipt.json open_issues must be empty before merge".to_string(),
            );
        }
        if external_pr_scope_blocks_merge(&external_pr) {
            issues.push(
                "external_pr_receipt.json scope_disposition must be within_scope before merge"
                    .to_string(),
            );
        }
    } else {
        issues.push(format!(
            "external_pr_receipt.json is required before fda merge: {}",
            external_pr_path.display()
        ));
    }

    if actual_pr_url.is_none() {
        issues.push("actual_pr_url is required before merge".to_string());
    }

    enforce_review_agent_gate_before_merge(
        artifact_dir,
        repo_root,
        actual_pr_url.as_deref(),
        &planned_pr_id,
        &changed_files,
        &mut issues,
        &mut evidence_links,
    );

    let forge = forge_gate_status_from_artifacts(artifact_dir, &planned_pr_id, context, repo_root)?;
    evidence_links.extend(forge.evidence_links.clone());
    issues.extend(forge.issues.clone());

    let guard = merge_human_decision_guard(artifact_dir)?;
    if !guard.unresolved_decision_ids.is_empty() {
        evidence_links.push("human_decision_packet.json".to_string());
        issues.push(format!(
            "open Human Decisions must be resolved before merge: {}",
            guard.unresolved_decision_ids.join(", ")
        ));
    }
    if !guard.non_approval_decision_ids.is_empty() {
        evidence_links.push("decision_receipts.json".to_string());
        issues.push(format!(
            "non-approval Human Decisions block merge: {}",
            guard.non_approval_decision_ids.join(", ")
        ));
    }

    let (risk_classification, risk_evidence) = risk_classification_from_artifacts(artifact_dir)?;
    evidence_links.extend(risk_evidence);
    if risk_classification == "missing_risk_evidence" {
        issues.push(
            "risk_register.json or risk_register.md is required before fda merge".to_string(),
        );
    }
    let human_approval_risk =
        matches!(risk_classification.as_str(), "high_risk" | "regulated_risk");
    let auto_merge_allowed = repository_auto_merge_allowed(target_repo);
    let merge_approval_granted = merge_approval_granted(artifact_dir)?;
    let policy_disposition = if !issues.is_empty() {
        "blocked"
    } else if human_approval_risk || !auto_merge_allowed {
        if merge_approval_granted {
            "human_approval_granted"
        } else {
            "human_approval_required"
        }
    } else {
        "auto_merge_candidate"
    }
    .to_string();
    let status = if !issues.is_empty() {
        "blocked"
    } else if (human_approval_risk || !auto_merge_allowed) && !merge_approval_granted {
        "human_approval_required"
    } else {
        "merge_ready"
    }
    .to_string();

    Ok(MergeGateStatus {
        status,
        issues,
        qa_status,
        repair_status,
        ci_status,
        forge_status: forge.status,
        forge_promotion_decision: forge.promotion_decision,
        forge_claim_ids: forge.claim_ids,
        forge_proof_obligations: forge.proof_obligations,
        risk_classification,
        policy_disposition,
        planned_pr_id,
        actual_pr_url,
        expected_head_sha,
        evidence_links,
    })
}

fn enforce_review_agent_gate_before_merge(
    artifact_dir: &Path,
    repo_root: &Path,
    actual_pr_url: Option<&str>,
    planned_pr_id: &str,
    changed_files: &[String],
    issues: &mut Vec<String>,
    evidence_links: &mut Vec<String>,
) {
    let store = FsArtifactStore;
    let review_gate_path = artifact_dir.join("review_agent_gate.json");
    if !store.exists(&review_gate_path) {
        issues.push(format!(
            "review_agent_gate.json is required before fda merge: {}",
            review_gate_path.display()
        ));
    } else {
        evidence_links.push("review_agent_gate.json".to_string());
        match read_json_value(&review_gate_path) {
            Ok(gate) => validate_review_agent_gate_json(
                &gate,
                artifact_dir,
                actual_pr_url,
                planned_pr_id,
                changed_files,
                issues,
            ),
            Err(error) => issues.push(format!(
                "failed to read review_agent_gate.json before merge: {error}"
            )),
        }
    }

    let review_packet_path = artifact_dir.join("review_agent_gate_packet.md");
    if !store.exists(&review_packet_path) {
        issues.push(format!(
            "review_agent_gate_packet.md is required before fda merge: {}",
            review_packet_path.display()
        ));
        return;
    }

    evidence_links.push("review_agent_gate_packet.md".to_string());
    let current_packet_content = match store.read_text(&review_packet_path) {
        Ok(content) => content,
        Err(error) => {
            issues.push(format!(
                "failed to read review_agent_gate_packet.md before merge: {error}"
            ));
            return;
        }
    };
    let Some(current_gate_section) = review_agent_gate_section(&current_packet_content) else {
        issues.push(
            "review_agent_gate_packet.md must contain a REVIEW_AGENT_GATE section before merge"
                .to_string(),
        );
        return;
    };

    let Some(actual_pr_url) = actual_pr_url else {
        issues.push(
            "review_agent_gate_packet.md is present, but actual_pr_url is required to resolve artifacts/review_packets/pr-<PR番号>.md before merge"
                .to_string(),
        );
        return;
    };
    let Some(pr_number) = pr_number_from_github_pull_url(actual_pr_url) else {
        issues.push(format!(
            "actual_pr_url {actual_pr_url} cannot be mapped to artifacts/review_packets/pr-<PR番号>.md for REVIEW_AGENT_GATE reflection"
        ));
        return;
    };

    let reflected_packet = format!("artifacts/review_packets/pr-{pr_number}.md");
    let reflected_packet_path = repo_root.join(&reflected_packet);
    if !store.exists(&reflected_packet_path) {
        issues.push(format!(
            "review_agent_gate_packet.md must be reflected to {reflected_packet} before merge; run python3 scripts/check_review_agent_gate.py --pr-number {pr_number} after reflection"
        ));
        return;
    }

    evidence_links.push(reflected_packet.clone());
    match store.read_text(&reflected_packet_path) {
        Ok(content) => {
            match review_agent_gate_section(&content) {
                Some(reflected_gate_section)
                    if reflected_gate_section == current_gate_section => {}
                Some(_) => issues.push(format!(
                    "{reflected_packet} REVIEW_AGENT_GATE section is stale or differs from review_agent_gate_packet.md; rerun fda review reflection before merge"
                )),
                None => issues.push(format!(
                    "{reflected_packet} must contain a REVIEW_AGENT_GATE section before merge"
                )),
            }
            if let Err(error) = validate_review_agent_gate_packet_with_checker(
                repo_root,
                &reflected_packet_path,
                &content,
                pr_number.as_str(),
            ) {
                issues.push(format!(
                    "{reflected_packet} failed REVIEW_AGENT_GATE checker before merge: {error}"
                ));
            }
        }
        Err(error) => issues.push(format!(
            "failed to read {reflected_packet} for REVIEW_AGENT_GATE reflection: {error}"
        )),
    }
}

fn validate_review_agent_gate_json(
    gate: &Value,
    artifact_dir: &Path,
    actual_pr_url: Option<&str>,
    planned_pr_id: &str,
    changed_files: &[String],
    issues: &mut Vec<String>,
) {
    match value_string(gate, "status").as_deref() {
        Some("passed") => {}
        Some(status) => issues.push(format!(
            "review_agent_gate.json status must be passed before merge, got {status}"
        )),
        None => issues.push("review_agent_gate.json status is missing".to_string()),
    }
    if gate
        .get("source_mutation_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        issues.push("review_agent_gate.json source_mutation_allowed must be false".to_string());
    }
    if gate
        .get("merge_approval_granted")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        issues.push(
            "review_agent_gate.json merge_approval_granted must remain false before human merge approval"
                .to_string(),
        );
    }
    if let Some(expected_pr_url) = actual_pr_url {
        match value_string(gate, "actual_pr_url").and_then(|url| normalize_github_pull_url(&url)) {
            Some(url) if url == expected_pr_url => {}
            Some(url) => issues.push(format!(
                "review_agent_gate.json actual_pr_url {url} does not match current PR {expected_pr_url}"
            )),
            None => issues.push(
                "review_agent_gate.json actual_pr_url must match the current GitHub pull URL before merge"
                    .to_string(),
            ),
        }
    }
    match value_string(gate, "reviewed_planned_pr_id").as_deref() {
        Some(reviewed_id) if reviewed_id == planned_pr_id => {}
        Some(reviewed_id) => issues.push(format!(
            "review_agent_gate.json reviewed_planned_pr_id {reviewed_id} does not match current planned_pr_id {planned_pr_id}"
        )),
        None => issues.push(
            "review_agent_gate.json reviewed_planned_pr_id is required before merge".to_string(),
        ),
    }

    let required_reviewers = gate
        .get("required_reviewers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for role in ["pr_reviewer", "functional_qa", "security_qa"] {
        match required_reviewers
            .iter()
            .find(|reviewer| value_string(reviewer, "role").as_deref() == Some(role))
        {
            Some(reviewer) => validate_required_reviewer(role, reviewer, artifact_dir, issues),
            None => issues.push(format!(
                "review_agent_gate.json required reviewer {role} is missing"
            )),
        }
    }

    for reviewer in gate
        .get("conditional_reviewers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if reviewer
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let role = value_string(reviewer, "role").unwrap_or_else(|| "<missing>".to_string());
            validate_required_reviewer(&role, reviewer, artifact_dir, issues);
        }
    }

    let conditional_reviewers = gate
        .get("conditional_reviewers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if changed_files
        .iter()
        .any(|path| requires_forge_review_for_merge(path))
    {
        validate_required_conditional_reviewer(
            "forge_reviewer",
            &["forge_reviewer", "qax2", "orchestrator"],
            &conditional_reviewers,
            artifact_dir,
            issues,
        );
    }
    if changed_files
        .iter()
        .any(|path| requires_design_qa_for_merge(path))
    {
        validate_required_conditional_reviewer(
            "design_qa",
            &["design_qa"],
            &conditional_reviewers,
            artifact_dir,
            issues,
        );
    }
}

fn validate_required_conditional_reviewer(
    label: &str,
    accepted_roles: &[&str],
    reviewers: &[Value],
    artifact_dir: &Path,
    issues: &mut Vec<String>,
) {
    match reviewers.iter().find(|reviewer| {
        reviewer
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && value_string(reviewer, "role")
                .as_deref()
                .is_some_and(|role| accepted_roles.contains(&role))
    }) {
        Some(reviewer) => {
            let role = value_string(reviewer, "role").unwrap_or_else(|| label.to_string());
            validate_required_reviewer(&role, reviewer, artifact_dir, issues);
        }
        None => issues.push(format!(
            "review_agent_gate.json conditional reviewer {label} must be marked required for changed files before merge"
        )),
    }
}

fn validate_required_reviewer(
    role: &str,
    reviewer: &Value,
    artifact_dir: &Path,
    issues: &mut Vec<String>,
) {
    if !reviewer
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        issues.push(format!(
            "review_agent_gate.json reviewer {role} must be marked required"
        ));
    }
    match value_string(reviewer, "status").as_deref() {
        Some("passed") => {}
        Some(status) => issues.push(format!(
            "review_agent_gate.json reviewer {role} status must be passed, got {status}"
        )),
        None => issues.push(format!(
            "review_agent_gate.json reviewer {role} status is missing"
        )),
    }
    match value_string(reviewer, "workspace_policy").as_deref() {
        Some("read_only") => {}
        Some(policy) => issues.push(format!(
            "review_agent_gate.json reviewer {role} workspace_policy must be read_only, got {policy}"
        )),
        None => issues.push(format!(
            "review_agent_gate.json reviewer {role} workspace_policy is missing"
        )),
    }
    match reviewer.get("evidence").and_then(Value::as_array) {
        Some(evidence) if !evidence.is_empty() => {
            for item in evidence {
                match item.as_str() {
                    Some(evidence_path) => {
                        validate_reviewer_evidence_path(role, evidence_path, artifact_dir, issues)
                    }
                    None => issues.push(format!(
                        "review_agent_gate.json reviewer {role} evidence entries must be strings"
                    )),
                }
            }
        }
        _ => issues.push(format!(
            "review_agent_gate.json reviewer {role} must include evidence"
        )),
    }
    if reviewer
        .get("source_mutation_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        issues.push(format!(
            "review_agent_gate.json reviewer {role} source_mutation_allowed must be false"
        ));
    }
}

fn validate_reviewer_evidence_path(
    role: &str,
    evidence_path: &str,
    artifact_dir: &Path,
    issues: &mut Vec<String>,
) {
    let path = Path::new(evidence_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        issues.push(format!(
            "review_agent_gate.json reviewer {role} evidence {evidence_path} must be a relative path inside artifact_dir"
        ));
        return;
    }
    let candidate = artifact_dir.join(path);
    if !candidate.exists() {
        issues.push(format!(
            "review_agent_gate.json reviewer {role} evidence {evidence_path} does not exist in artifact_dir"
        ));
        return;
    }
    let canonical_artifact_dir = match artifact_dir.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            issues.push(format!(
                "failed to canonicalize artifact_dir {} for review_agent_gate.json reviewer {role} evidence check: {error}",
                artifact_dir.display()
            ));
            return;
        }
    };
    let canonical_candidate = match candidate.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            issues.push(format!(
                "failed to canonicalize review_agent_gate.json reviewer {role} evidence {evidence_path}: {error}"
            ));
            return;
        }
    };
    if !canonical_candidate.starts_with(&canonical_artifact_dir) {
        issues.push(format!(
            "review_agent_gate.json reviewer {role} evidence {evidence_path} must resolve inside artifact_dir"
        ));
        return;
    }
    match canonical_candidate.metadata() {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => issues.push(format!(
            "review_agent_gate.json reviewer {role} evidence {evidence_path} must resolve to a regular file inside artifact_dir"
        )),
        Err(error) => issues.push(format!(
            "failed to inspect review_agent_gate.json reviewer {role} evidence {evidence_path}: {error}"
        )),
    }
}

fn review_agent_gate_section(content: &str) -> Option<String> {
    // Windows の checkout (core.autocrlf=true) では反映済み packet が CRLF になる。
    // 生成側 packet (LF) との比較が改行差で stale 誤判定にならないよう正規化する。
    let normalized = content.replace("\r\n", "\n");
    let start = normalized.find("## REVIEW_AGENT_GATE")?;
    let tail = &normalized[start..];
    let end = tail
        .find("\n## ")
        .filter(|index| *index > 0)
        .unwrap_or(tail.len());
    Some(tail[..end].trim().to_string())
}

fn push_unique_all(values: &mut Vec<String>, candidates: Vec<String>) {
    for candidate in candidates {
        if !values.contains(&candidate) {
            values.push(candidate);
        }
    }
}

fn requires_forge_review_for_merge(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.starts_with(".fda/")
        || normalized.starts_with("artifacts/runs/")
        || normalized.starts_with("artifacts/review_packets/")
        || normalized.starts_with("handoffs/")
        || normalized.starts_with("docs/standards/fda-v1/")
        || normalized.starts_with("docs/standards/delivery-artifacts")
        || normalized.starts_with("src/application/merge")
        || normalized.starts_with("src/application/profile")
        || normalized.starts_with("src/application/review")
        || normalized.starts_with("src/application/validate")
        || normalized.starts_with("src/rendering/merge")
        || normalized.starts_with("src/rendering/review")
        || normalized == "scripts/check_review_agent_gate.py"
        || normalized.contains("review_agent_gate")
        || normalized.contains("human_decision")
        || normalized.contains("review_packet")
        || normalized.contains("handoff")
        || normalized.contains("receipt")
        || normalized.contains("validation_report")
        || normalized.contains("ato_state")
        || normalized.contains("forge_projection")
}

fn requires_design_qa_for_merge(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.starts_with("app/")
        || normalized.starts_with("pages/")
        || normalized.starts_with("public/")
        || normalized.starts_with("browser/")
        || normalized.starts_with("playwright/")
        || normalized.starts_with("frontend/")
        || normalized.starts_with("src/ui/")
        || normalized.starts_with("src/components/")
        || normalized.contains("/browser/")
        || normalized.contains("/playwright/")
        || normalized.ends_with(".tsx")
        || normalized.ends_with(".jsx")
        || normalized.ends_with(".css")
        || normalized.ends_with(".scss")
        || normalized.ends_with(".html")
        || normalized.ends_with(".vue")
        || normalized.ends_with(".svelte")
}

fn validate_review_agent_gate_packet_with_checker(
    repo_root: &Path,
    reflected_packet_path: &Path,
    content: &str,
    pr_number: &str,
) -> Result<(), String> {
    if !content.contains("## REVIEW_AGENT_GATE") {
        return Err("REVIEW_AGENT_GATE section is missing".to_string());
    }
    let mut command = python_launcher();
    command.extend([
        "scripts/check_review_agent_gate.py".to_string(),
        "--pr-number".to_string(),
        pr_number.to_string(),
    ]);
    match run_process_command(&command, repo_root) {
        Ok(output) if output.success => Ok(()),
        Ok(output) => Err(single_line(&format!(
            "python3 scripts/check_review_agent_gate.py --pr-number {pr_number} failed for {}: {} {}",
            reflected_packet_path.display(),
            output.stdout,
            output.stderr
        ))),
        Err(error) => Err(format!(
            "failed to run python3 scripts/check_review_agent_gate.py --pr-number {pr_number}: {error}"
        )),
    }
}

fn repository_auto_merge_allowed(repo_root: &Path) -> bool {
    let policy_path = repo_root.join(".fda/delivery_policy.yaml");
    let Ok(body) = FsArtifactStore.read_text(&policy_path) else {
        return false;
    };
    SerdeYamlValidator
        .parse_yaml_value(&policy_path, &body)
        .ok()
        .and_then(|policy| {
            policy
                .get("delivery_policy")
                .and_then(|delivery_policy| delivery_policy.get("auto_merge_allowed"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn pr_number_from_github_pull_url(actual_pr_url: &str) -> Option<String> {
    let (_, tail) = actual_pr_url.split_once("/pull/")?;
    let number = tail
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    (!number.is_empty()).then_some(number)
}

fn forge_gate_status_from_artifacts(
    artifact_dir: &Path,
    planned_pr_id: &str,
    context: &RuntimeContext,
    repo_root: &Path,
) -> Result<ForgeGateStatus, String> {
    let forge_path = artifact_dir.join("forge_projection.json");
    if !forge_path.exists() {
        return Ok(ForgeGateStatus {
            status: "adapter_unavailable".to_string(),
            promotion_decision: "missing".to_string(),
            claim_ids: Vec::new(),
            proof_obligations: Vec::new(),
            issues: vec![format!(
                "forge_projection.json is required before fda merge: {}",
                forge_path.display()
            )],
            evidence_links: Vec::new(),
        });
    }

    let projection = read_json_value(&forge_path)?;
    let mut issues = Vec::new();
    let mut evidence_links = vec!["forge_projection.json".to_string()];
    let mut claim_ids = Vec::new();
    let mut proof_obligations = Vec::new();
    issues.extend(forge_projection_schema_issues(repo_root, &projection)?);

    match value_string(&projection, "program_id") {
        Some(program_id) if program_id == context.program_id => {}
        Some(program_id) => issues.push(format!(
            "forge_projection.json program_id {program_id} does not match current program_id {}",
            context.program_id
        )),
        None => {
            issues.push("forge_projection.json program_id is required before merge".to_string())
        }
    }
    match value_string(&projection, "epic_id") {
        Some(epic_id) if epic_id == context.epic_id => {}
        Some(epic_id) => issues.push(format!(
            "forge_projection.json epic_id {epic_id} does not match current epic_id {}",
            context.epic_id
        )),
        None => issues.push("forge_projection.json epic_id is required before merge".to_string()),
    }

    let promotion_readiness = projection
        .get("promotion_readiness")
        .unwrap_or(&Value::Null);
    let promotion_decision =
        value_string(promotion_readiness, "verdict").unwrap_or_else(|| "missing".to_string());
    let gate_inputs_ready = promotion_readiness
        .get("gate_inputs_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if promotion_decision != "promote" {
        issues.push(format!(
            "forge_projection.json promotion_readiness.verdict must be promote before merge, got {promotion_decision}"
        ));
    }
    if !gate_inputs_ready {
        issues.push(
            "forge_projection.json promotion_readiness.gate_inputs_ready must be true before merge"
                .to_string(),
        );
    }

    let claims = projection
        .get("claim_contracts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let proofs = projection
        .get("proof_obligations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for claim in claims.iter().filter(|claim| {
        value_string_array(claim, "planned_pr_ids")
            .iter()
            .any(|id| id == planned_pr_id)
    }) {
        let claim_id =
            value_string(claim, "claim_id").unwrap_or_else(|| "missing_claim_id".to_string());
        push_unique(&mut claim_ids, claim_id.clone());
        let claim_proofs = value_string_array(claim, "proof_obligations");
        if claim_proofs.is_empty() {
            issues.push(format!(
                "forge_projection.json claim {claim_id} must declare at least one proof_obligation before merge"
            ));
        }

        for proof_id in claim_proofs {
            push_unique(&mut proof_obligations, proof_id.clone());
            let Some(proof) = proofs.iter().find(|proof| {
                value_string(proof, "proof_id").as_deref() == Some(proof_id.as_str())
            }) else {
                issues.push(format!(
                    "forge_projection.json proof_obligations is missing proof {proof_id} required by claim {claim_id}"
                ));
                continue;
            };

            match value_string(proof, "claim_id") {
                Some(proof_claim_id) if proof_claim_id == claim_id => {}
                Some(proof_claim_id) => issues.push(format!(
                    "forge_projection.json proof {proof_id} claim_id {proof_claim_id} does not match claim {claim_id}"
                )),
                None => issues.push(format!(
                    "forge_projection.json proof {proof_id} must include claim_id before merge"
                )),
            }

            let required_evidence = value_string_array(proof, "required_evidence");
            if required_evidence.is_empty() {
                issues.push(format!(
                    "forge_projection.json proof {proof_id} must declare required_evidence before merge"
                ));
            }
            for evidence in required_evidence {
                match verified_artifact_evidence_path(artifact_dir, &evidence) {
                    Ok(_) => push_unique(&mut evidence_links, evidence),
                    Err(reason) => {
                        issues.push(format!("forge_projection.json proof {proof_id} {reason}"))
                    }
                }
            }
        }
    }

    if claim_ids.is_empty() {
        issues.push(format!(
            "forge_projection.json claim_contracts must include a claim for planned_pr_id {planned_pr_id}"
        ));
    }

    let status = if issues.is_empty() {
        "promote".to_string()
    } else if matches!(
        promotion_decision.as_str(),
        "hold" | "reject" | "not_evaluated" | "missing"
    ) {
        promotion_decision.clone()
    } else {
        "hold".to_string()
    };

    Ok(ForgeGateStatus {
        status,
        promotion_decision,
        claim_ids,
        proof_obligations,
        issues,
        evidence_links,
    })
}

fn forge_projection_schema_issues(
    repo_root: &Path,
    projection: &Value,
) -> Result<Vec<String>, String> {
    let schema_path = repo_root
        .join(DEFAULT_SCHEMA_DIR)
        .join("forge_projection.schema.json");
    let schema = read_json_value(&schema_path)?;
    let validator = JsonSchemaArtifactValidator;
    let errors = validator
        .validate_json_schema(&schema, projection)
        .map_err(|error| {
            format!(
                "forge_projection.json schema validation could not run: {}",
                check_error_summary(&error)
            )
        })?;
    Ok(errors
        .into_iter()
        .map(|error| {
            format!(
                "forge_projection.json schema validation failed: {}",
                check_error_summary(&error)
            )
        })
        .collect())
}

fn check_error_summary(error: &CheckError) -> String {
    let mut parts = vec![error.message.clone()];
    if let Some(instance_path) = &error.instance_path {
        parts.push(format!("instance_path={instance_path}"));
    }
    if let Some(schema_path) = &error.schema_path {
        parts.push(format!("schema_path={schema_path}"));
    }
    parts.join("; ")
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn verified_artifact_evidence_path(artifact_dir: &Path, evidence: &str) -> Result<PathBuf, String> {
    let evidence_path = Path::new(evidence);
    // has_root() also rejects rooted unix-style paths (e.g. /etc/passwd),
    // which is_absolute() alone does not catch on Windows.
    if evidence_path.is_absolute()
        || evidence_path.has_root()
        || evidence_path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "required_evidence {evidence} must be a relative path inside artifact_dir"
        ));
    }

    let candidate = artifact_dir.join(evidence_path);
    if !candidate.exists() {
        return Err(format!("missing proof evidence {evidence}"));
    }

    let canonical_artifact_dir = artifact_dir.canonicalize().map_err(|e| {
        format!(
            "failed to canonicalize artifact_dir {}: {e}",
            artifact_dir.display()
        )
    })?;
    let canonical_candidate = candidate.canonicalize().map_err(|e| {
        format!(
            "failed to canonicalize required_evidence {evidence} at {}: {e}",
            candidate.display()
        )
    })?;
    if !canonical_candidate.starts_with(&canonical_artifact_dir) {
        return Err(format!(
            "required_evidence {evidence} must resolve inside artifact_dir"
        ));
    }
    let metadata = canonical_candidate.metadata().map_err(|e| {
        format!(
            "failed to inspect required_evidence {evidence} at {}: {e}",
            candidate.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "required_evidence {evidence} must resolve to a regular file inside artifact_dir"
        ));
    }
    if metadata.len() == 0 {
        return Err(format!(
            "required_evidence {evidence} must be a non-empty file"
        ));
    }

    Ok(candidate)
}

fn ci_status_from_external_pr_receipt(receipt: &Value) -> String {
    let Some(checks) = receipt.get("checks").and_then(Value::as_object) else {
        return "missing".to_string();
    };
    if checks.is_empty() {
        return "missing".to_string();
    }
    let mut has_pending = false;
    let mut has_missing = false;
    let mut tests_passed = false;
    for (name, value) in checks {
        let status = value.as_str().unwrap_or_default().to_lowercase();
        if matches!(status.as_str(), "failed" | "fail" | "error" | "cancelled") {
            return "failed".to_string();
        }
        if name == "tests" && matches!(status.as_str(), "passed" | "pass" | "success") {
            tests_passed = true;
        }
        if matches!(status.as_str(), "pending" | "queued" | "in_progress") {
            has_pending = true;
        } else if !matches!(status.as_str(), "passed" | "pass" | "success") {
            has_missing = true;
        }
    }
    if has_pending {
        "pending".to_string()
    } else if has_missing || !tests_passed {
        "missing".to_string()
    } else {
        "passed".to_string()
    }
}

fn is_open_external_pr_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "opened" | "open" | "ready" | "merge_ready"
    )
}

fn has_non_empty_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty())
}

fn external_pr_scope_blocks_merge(receipt: &Value) -> bool {
    let Some(scope) = receipt.get("scope_disposition") else {
        return true;
    };
    let kind = value_string(scope, "kind").unwrap_or_else(|| "unknown".to_string());
    kind != "within_scope"
}

fn risk_classification_from_artifacts(
    artifact_dir: &Path,
) -> Result<(String, Vec<String>), String> {
    let risk_path = artifact_dir.join("risk_register.json");
    let markdown_risk_path = artifact_dir.join("risk_register.md");
    if !risk_path.exists() && !markdown_risk_path.exists() {
        return Ok(("missing_risk_evidence".to_string(), Vec::new()));
    }

    let mut values = Vec::new();
    let mut evidence = Vec::new();
    if risk_path.exists() {
        let risk_register = read_json_value(&risk_path)?;
        collect_json_strings(&risk_register, &mut values);
        evidence.push("risk_register.json".to_string());
    }
    if markdown_risk_path.exists() {
        let body = FsArtifactStore
            .read_text(&markdown_risk_path)
            .map_err(|e| {
                format!(
                    "failed to read risk register {}: {e}",
                    markdown_risk_path.display()
                )
            })?;
        values.push(body);
        evidence.push("risk_register.md".to_string());
    }
    let normalized = values
        .iter()
        .map(|value| value.to_lowercase().replace(['-', '_'], " "))
        .collect::<Vec<_>>();
    let classification = if normalized.iter().any(|value| {
        value.contains("critical") || value.split_whitespace().any(|word| word == "high")
    }) {
        "high_risk"
    } else if normalized.iter().any(|value| {
        value.contains("security") || value.contains("privacy") || value.contains("legal")
    }) {
        "regulated_risk"
    } else {
        "low_risk"
    };
    Ok((classification.to_string(), evidence))
}

fn collect_json_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(string) => out.push(string.clone()),
        Value::Array(values) => {
            for value in values {
                collect_json_strings(value, out);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_json_strings(value, out);
            }
        }
        _ => {}
    }
}

fn merge_execution_outcome(
    config: &MergeConfig,
    repo_root: &Path,
    artifact_dir: &Path,
    target_repo: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    gate: &MergeGateStatus,
) -> Result<MergeExecutionOutcome, String> {
    let method = config.merge_method.as_str().to_string();
    let gate_view = merge_gate_view(gate);
    if !config.execute {
        return Ok(MergeExecutionOutcome::not_requested(&method));
    }

    let receipt_path = out_dir.join("github_merge_receipt.json");
    let resume_command =
        merge_resume_command(repo_root, artifact_dir, out_dir, target_repo, &method);
    let started_at = format!("unix:{}", now_unix_seconds());
    let result = match github_merge_precondition_failure(gate, target_repo) {
        Some(reason) => Err(reason),
        None => run_github_merge_adapter(
            config,
            target_repo,
            gate.actual_pr_url.as_deref().unwrap(),
            gate.expected_head_sha.as_deref().unwrap(),
        ),
    };
    let completed_at = format!("unix:{}", now_unix_seconds());

    match result {
        Ok(GithubMergeAdapterOutcome::Succeeded(details)) => {
            let details_view = github_merge_details_view(&details);
            let receipt = github_merge_receipt_success(
                context,
                &gate_view,
                &method,
                &started_at,
                &completed_at,
                &details_view,
            );
            write_json_file(&receipt_path, &receipt)?;
            Ok(MergeExecutionOutcome {
                requested: true,
                method,
                status: "succeeded".to_string(),
                merge_executed: true,
                github_merge_receipt_path: Some(receipt_path),
                failure_reason: None,
                resume_command: None,
            })
        }
        Ok(GithubMergeAdapterOutcome::ReceiptCollectionFailed {
            actual_pr_url,
            failure_reason,
            receipt_collection_command,
        }) => {
            let details_view = GithubMergeReceiptCollectionFailureView {
                actual_pr_url: &actual_pr_url,
                failure_reason: &failure_reason,
                receipt_collection_command: &receipt_collection_command,
            };
            let receipt = github_merge_receipt_collection_failure(
                context,
                &gate_view,
                &method,
                &started_at,
                &completed_at,
                &details_view,
            );
            write_json_file(&receipt_path, &receipt)?;
            Ok(MergeExecutionOutcome {
                requested: true,
                method,
                status: "receipt_collection_failed".to_string(),
                merge_executed: true,
                github_merge_receipt_path: Some(receipt_path),
                failure_reason: Some(failure_reason),
                resume_command: Some(receipt_collection_command),
            })
        }
        Err(reason) => {
            let receipt = github_merge_receipt_failure(
                context,
                &gate_view,
                &method,
                &started_at,
                &completed_at,
                &reason,
                &resume_command,
            );
            write_json_file(&receipt_path, &receipt)?;
            Ok(MergeExecutionOutcome {
                requested: true,
                method,
                status: "failed".to_string(),
                merge_executed: false,
                github_merge_receipt_path: Some(receipt_path),
                failure_reason: Some(reason),
                resume_command: Some(resume_command),
            })
        }
    }
}

fn github_merge_precondition_failure(gate: &MergeGateStatus, target_repo: &Path) -> Option<String> {
    let mut failures = Vec::new();
    if gate.status != "merge_ready" {
        failures.push(format!(
            "gate.status must be merge_ready, got {}",
            gate.status
        ));
    }
    if !matches!(
        gate.policy_disposition.as_str(),
        "auto_merge_candidate" | "human_approval_granted"
    ) {
        failures.push(format!(
            "policy_disposition must be auto_merge_candidate or human_approval_granted, got {}",
            gate.policy_disposition
        ));
    }
    if gate.actual_pr_url.is_none() {
        failures.push("actual_pr_url is required before GitHub merge execution".to_string());
    }
    if gate.expected_head_sha.is_none() {
        failures.push("target_pr.head_sha is required before GitHub merge execution".to_string());
    }
    if let Some(actual_pr_url) = gate.actual_pr_url.as_ref() {
        let pr_slug = github_repo_slug_from_pull_url(actual_pr_url);
        match (pr_slug, github_repo_slug_from_target_repo(target_repo)) {
            (Some(pr_slug), Ok(target_slug)) if pr_slug == target_slug => {}
            (Some(pr_slug), Ok(target_slug)) => failures.push(format!(
                "target repo origin {target_slug} does not match PR URL repository {pr_slug}"
            )),
            (None, _) => failures.push(format!(
                "actual_pr_url {actual_pr_url} is not a supported GitHub pull URL"
            )),
            (_, Err(reason)) => failures.push(reason),
        }
    }
    if failures.is_empty() {
        None
    } else {
        Some(format!(
            "merge execution precondition failed: {}",
            failures.join("; ")
        ))
    }
}

fn run_github_merge_adapter(
    config: &MergeConfig,
    target_repo: &Path,
    actual_pr_url: &str,
    expected_head_sha: &str,
) -> Result<GithubMergeAdapterOutcome, String> {
    if let Some(command) = &config.github_merge_command {
        return run_mock_github_merge_command(command, target_repo, actual_pr_url);
    }

    let merge_command = vec![
        "gh".to_string(),
        "pr".to_string(),
        "merge".to_string(),
        actual_pr_url.to_string(),
        config.merge_method.gh_flag().to_string(),
        "--match-head-commit".to_string(),
        expected_head_sha.to_string(),
    ];
    let view_command = vec![
        "gh".to_string(),
        "pr".to_string(),
        "view".to_string(),
        actual_pr_url.to_string(),
        "--json".to_string(),
        "mergeCommit,mergedAt,mergedBy,url".to_string(),
    ];
    run_github_merge_adapter_commands(merge_command, view_command, target_repo, actual_pr_url)
}

fn run_github_merge_adapter_commands(
    merge_command: Vec<String>,
    view_command: Vec<String>,
    target_repo: &Path,
    actual_pr_url: &str,
) -> Result<GithubMergeAdapterOutcome, String> {
    let merge_output = run_process_command(&merge_command, target_repo)?;
    if !merge_output.success {
        return Err(process_failure_reason(
            "GitHub merge command",
            &merge_command,
            &merge_output,
        ));
    }

    let receipt_collection_command = command_display(&view_command);
    let view_output = run_process_command(&view_command, target_repo)?;
    if !view_output.success {
        return Ok(GithubMergeAdapterOutcome::ReceiptCollectionFailed {
            actual_pr_url: actual_pr_url.to_string(),
            failure_reason: process_failure_reason(
                "GitHub merge receipt collection command",
                &view_command,
                &view_output,
            ),
            receipt_collection_command,
        });
    }
    let view = match serde_json::from_str::<Value>(view_output.stdout.trim()) {
        Ok(value) => value,
        Err(e) => {
            return Ok(GithubMergeAdapterOutcome::ReceiptCollectionFailed {
                actual_pr_url: actual_pr_url.to_string(),
                failure_reason: format!(
                    "failed to parse GitHub merge receipt JSON from `{}` after merge command succeeded: {e}",
                    command_display(&view_command)
                ),
                receipt_collection_command,
            });
        }
    };
    match github_merge_details_from_value(&view, actual_pr_url) {
        Ok(details) => Ok(GithubMergeAdapterOutcome::Succeeded(details)),
        Err(reason) => Ok(GithubMergeAdapterOutcome::ReceiptCollectionFailed {
            actual_pr_url: actual_pr_url.to_string(),
            failure_reason: format!(
                "GitHub merge receipt JSON from `{}` is incomplete after merge command succeeded: {reason}",
                command_display(&view_command)
            ),
            receipt_collection_command,
        }),
    }
}

fn github_repo_slug_from_pull_url(value: &str) -> Option<String> {
    let url = normalize_github_pull_url(value)?;
    let path = url.strip_prefix("https://github.com/")?;
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    github_repo_slug(owner, repo)
}

fn github_repo_slug_from_target_repo(target_repo: &Path) -> Result<String, String> {
    let command = vec![
        "git".to_string(),
        "remote".to_string(),
        "get-url".to_string(),
        "origin".to_string(),
    ];
    let output = run_process_command(&command, target_repo)?;
    if !output.success {
        return Err(process_failure_reason(
            "target repo origin lookup",
            &command,
            &output,
        ));
    }
    let remote = output.stdout.trim();
    github_repo_slug_from_remote_url(remote).ok_or_else(|| {
        format!(
            "target repo origin `{}` is not a supported GitHub repository URL",
            single_line(remote)
        )
    })
}

fn github_repo_slug_from_remote_url(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    let path = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))?;
    let path = path.trim_end_matches(".git");
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    github_repo_slug(owner, repo)
}

fn github_repo_slug(owner: &str, repo: &str) -> Option<String> {
    let owner = owner.trim();
    let repo = repo.trim().trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!(
        "{}/{}",
        owner.to_ascii_lowercase(),
        repo.to_ascii_lowercase()
    ))
}

fn run_mock_github_merge_command(
    command: &[String],
    target_repo: &Path,
    actual_pr_url: &str,
) -> Result<GithubMergeAdapterOutcome, String> {
    let output = run_process_command(command, target_repo)?;
    if !output.success {
        return Err(process_failure_reason(
            "GitHub merge mock command",
            command,
            &output,
        ));
    }
    let stdout = output.stdout.trim();
    if stdout.is_empty() {
        return Err(format!(
            "GitHub merge mock command `{}` succeeded but did not emit details JSON",
            command_display(command)
        ));
    }
    let value = serde_json::from_str::<Value>(stdout).map_err(|e| {
        format!(
            "failed to parse GitHub merge mock command JSON from `{}`: {e}",
            command_display(command)
        )
    })?;
    if value_string(&value, "status").as_deref() == Some("receipt_collection_failed") {
        let failure_reason = value_string(&value, "failure_reason").unwrap_or_else(|| {
            "GitHub merge receipt collection failed after mock merge command succeeded".to_string()
        });
        let receipt_collection_command = value_string(&value, "receipt_collection_command")
            .unwrap_or_else(|| {
                format!(
                    "gh pr view {} --json mergeCommit,mergedAt,mergedBy,url",
                    actual_pr_url
                )
            });
        let actual_pr_url = value_string(&value, "actual_pr_url")
            .and_then(|url| normalize_github_pull_url(&url))
            .unwrap_or_else(|| actual_pr_url.to_string());
        return Ok(GithubMergeAdapterOutcome::ReceiptCollectionFailed {
            actual_pr_url,
            failure_reason,
            receipt_collection_command,
        });
    }
    github_merge_details_from_value(&value, actual_pr_url).map(GithubMergeAdapterOutcome::Succeeded)
}

fn process_failure_reason(context: &str, command: &[String], output: &ProcessOutput) -> String {
    let detail = if output.stderr.trim().is_empty() {
        output.stdout.trim()
    } else {
        output.stderr.trim()
    };
    let detail = if detail.is_empty() {
        "<no command output>".to_string()
    } else {
        single_line(detail)
    };
    let exit = output
        .exit_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string());
    format!(
        "{context} `{}` failed with exit {exit}: {detail}",
        command_display(command)
    )
}

fn github_merge_details_from_value(
    value: &Value,
    fallback_actual_pr_url: &str,
) -> Result<GithubMergeDetails, String> {
    let merge_sha = value_string(value, "merge_sha")
        .or_else(|| nested_value_string(value, &["mergeCommit", "oid"]))
        .or_else(|| nested_value_string(value, &["mergeCommit", "sha"]))
        .ok_or_else(|| "GitHub merge receipt JSON is missing merge sha".to_string())?;
    let merged_at = value_string(value, "merged_at")
        .or_else(|| value_string(value, "mergedAt"))
        .ok_or_else(|| "GitHub merge receipt JSON is missing merged_at".to_string())?;
    let actor = value_string(value, "actor")
        .or_else(|| nested_value_string(value, &["mergedBy", "login"]))
        .or_else(|| env::var("GITHUB_ACTOR").ok())
        .or_else(|| env::var("USER").ok())
        .unwrap_or_else(|| "unknown".to_string());
    let actual_pr_url = value_string(value, "actual_pr_url")
        .or_else(|| value_string(value, "url"))
        .unwrap_or_else(|| fallback_actual_pr_url.to_string());
    let actual_pr_url = normalize_github_pull_url(&actual_pr_url).ok_or_else(|| {
        "GitHub merge receipt JSON actual_pr_url/url is not a GitHub pull URL".to_string()
    })?;

    Ok(GithubMergeDetails {
        merge_sha,
        merged_at,
        actor,
        actual_pr_url,
    })
}

fn nested_value_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

fn merge_answer_is_approval(decision: &HumanDecisionSummary, answer: &str) -> bool {
    if is_merge_approval_decision(decision) {
        return is_explicit_merge_approval_answer(answer);
    }
    answer_is_approval(decision, answer) && !is_merge_non_approval_answer(answer)
}

fn is_explicit_merge_approval_answer(answer: &str) -> bool {
    matches!(
        normalize_decision_token(answer).as_str(),
        "approve"
            | "approved"
            | "approve_merge"
            | "merge_approved"
            | "approve_pr_merge"
            | "approved_for_merge"
    )
}

fn is_merge_non_approval_answer(answer: &str) -> bool {
    let normalized = normalize_decision_token(answer);
    normalized.is_empty()
        || normalized == "hold"
        || normalized == "held"
        || normalized == "hold_for_repair"
        || normalized == "defer"
        || normalized == "deferred"
        || normalized == "reject"
        || normalized == "rejected"
        || normalized == "deny"
        || normalized == "denied"
        || normalized == "revise"
        || normalized == "no"
        || normalized == "n"
        || normalized == "block"
        || normalized == "blocked"
        || normalized == "cancel"
        || normalized == "canceled"
}

fn normalize_decision_token(answer: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;
    for character in answer.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}
