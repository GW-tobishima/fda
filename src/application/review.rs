use serde::Serialize;
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};

use crate::application::decisions::{value_string, value_string_array};
use crate::application::implement::{implement_human_decision_guard, implement_runtime_context};
use crate::application::ports::ArtifactStore;
use crate::application::profile::{
    ensure_repository_profile, ensure_target_repository_profile_if_present,
};
use crate::application::validate::{validate_artifacts, write_report};
use crate::cli::args::{AtoConfig, QaFixture, ReviewConfig, ValidateConfig};
use crate::domain::entities::RuntimeContext;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::json_file::{read_json_value, write_json_file};
use crate::infra::paths::canonicalize_existing_or_parent;
use crate::rendering::implement::implement_agent_role_policy;
use crate::rendering::review::*;
use crate::support::paths::{display_path, resolve_path};
use crate::{
    carry_forward_review_artifacts, normalize_github_pull_url, now_unix_seconds,
    remove_artifact_if_exists, write_text_file, HumanDecisionGuard, DEFAULT_MODEL_CONTRACT_DIRS,
    DEFAULT_SCHEMA_DIR,
};

#[derive(Debug, Serialize)]
pub(crate) struct ReviewResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: String,
    pub(crate) functional_qa_status: String,
    pub(crate) security_qa_status: String,
    pub(crate) artifact_dir: String,
    pub(crate) out_dir: String,
    pub(crate) target_repo: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) validation_report_path: Option<String>,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) struct ReviewGateStatus {
    pub(crate) status: String,
    pub(crate) issues: Vec<String>,
    pub(crate) evidence_links: Vec<String>,
    pub(crate) actual_pr_url: Option<String>,
    pub(crate) reviewed_planned_pr_id: String,
    pub(crate) pr_reviewer_status: String,
    pub(crate) pr_reviewer_findings: Vec<String>,
    pub(crate) pr_reviewer_evidence_links: Vec<String>,
    pub(crate) forge_reviewer_required: bool,
    pub(crate) forge_reviewer_status: String,
    pub(crate) forge_reviewer_evidence_links: Vec<String>,
    pub(crate) design_qa_required: bool,
    pub(crate) design_qa_status: String,
    pub(crate) design_qa_evidence_links: Vec<String>,
}

impl ReviewGateStatus {
    pub(crate) fn is_pass(&self) -> bool {
        self.status == "ready" && self.issues.is_empty()
    }
}

pub(crate) fn review(config: &ReviewConfig) -> Result<ReviewResult, String> {
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
    let target_repo_exists = target_repo_input.is_dir();
    let target_repo = if target_repo_exists {
        store.canonicalize(&target_repo_input).map_err(|e| {
            format!(
                "failed to resolve target repo {}: {e}",
                target_repo_input.display()
            )
        })?
    } else {
        target_repo_input.clone()
    };
    ensure_target_repository_profile_if_present(&store, &target_repo, &repo_root)?;
    let out_dir = match &config.out {
        Some(out) => resolve_path(&repo_root, out),
        None => env::temp_dir().join(format!("fda-review-{}", now_unix_seconds())),
    };
    let out_dir_for_safety = canonicalize_existing_or_parent(&out_dir).map_err(|e| {
        format!(
            "failed to resolve review output dir {} for containment check: {e}",
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
            "review output dir {} must not be inside target repo {}",
            out_dir.display(),
            target_repo.display()
        ));
    }
    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;
    let out_dir = store.canonicalize(&out_dir).unwrap_or(out_dir);
    remove_artifact_if_exists(&out_dir.join("repair_receipt.json"))?;

    let mut context = review_runtime_context(&artifact_dir);
    context.task_ids = vec!["PR-V1-007".to_string()];
    if context.case_ids.is_empty() {
        context.case_ids = vec!["CASE-FDA-V1-REVIEW-001".to_string()];
    }

    let guard = implement_human_decision_guard(&artifact_dir)?;
    let gate = review_gate_status_from_artifacts(&artifact_dir, target_repo_exists)?;
    let human_clear =
        guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty();
    let review_ready = human_clear && gate.is_pass() && target_repo_exists;
    let plan_status = if review_ready { "ready" } else { "blocked" };
    let functional_status =
        qa_status_from_fixture(review_ready, config.functional_qa_fixture.as_ref(), false);
    let security_status =
        qa_status_from_fixture(review_ready, config.security_qa_fixture.as_ref(), true);
    let verdict = review_verdict(&functional_status, &security_status);
    let gate_view = ReviewGateView {
        status: &gate.status,
        issues: &gate.issues,
        evidence_links: &gate.evidence_links,
        actual_pr_url: gate.actual_pr_url.as_deref(),
        reviewed_planned_pr_id: &gate.reviewed_planned_pr_id,
        pr_reviewer_status: &gate.pr_reviewer_status,
        pr_reviewer_findings: &gate.pr_reviewer_findings,
        pr_reviewer_evidence_links: &gate.pr_reviewer_evidence_links,
        forge_reviewer_required: gate.forge_reviewer_required,
        forge_reviewer_status: &gate.forge_reviewer_status,
        forge_reviewer_evidence_links: &gate.forge_reviewer_evidence_links,
        design_qa_required: gate.design_qa_required,
        design_qa_status: &gate.design_qa_status,
        design_qa_evidence_links: &gate.design_qa_evidence_links,
    };
    let guard_view = HumanDecisionGuardView {
        unresolved_decision_ids: &guard.unresolved_decision_ids,
        non_approval_decision_ids: &guard.non_approval_decision_ids,
    };
    let functional_findings =
        qa_findings(review_ready, &gate, config.functional_qa_fixture.as_ref());
    let security_findings = qa_findings(review_ready, &gate, config.security_qa_fixture.as_ref());
    let rendered_at = now_unix_seconds();
    let mut artifacts_written = Vec::new();

    write_text_file(
        &out_dir.join("pr_reviewer_prompt.md"),
        &pr_reviewer_prompt_markdown(&target_repo, &context, &gate_view),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("pr_reviewer_prompt.md"),
    ));

    write_text_file(
        &out_dir.join("functional_qa_prompt.md"),
        &functional_qa_prompt_markdown(&target_repo, &context, &gate_view),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("functional_qa_prompt.md"),
    ));

    write_text_file(
        &out_dir.join("security_qa_prompt.md"),
        &security_qa_prompt_markdown(&target_repo, &context, &gate_view),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("security_qa_prompt.md"),
    ));

    write_json_file(
        &out_dir.join("agent_role_policy.json"),
        &implement_agent_role_policy(),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("agent_role_policy.json"),
    ));

    write_json_file(
        &out_dir.join("mcp_agent_invocation_plan.json"),
        &mcp_agent_invocation_plan_review(&target_repo, &context, &guard_view, plan_status),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("mcp_agent_invocation_plan.json"),
    ));

    let pr_reviewer_receipt = pr_reviewer_receipt(&target_repo, &context, &gate_view, review_ready);
    write_json_file(
        &out_dir.join("pr_reviewer_receipt.json"),
        &pr_reviewer_receipt,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("pr_reviewer_receipt.json"),
    ));

    let functional_receipt = functional_qa_receipt(
        &target_repo,
        &context,
        &gate_view,
        &functional_status,
        &functional_findings,
        review_ready,
    );
    write_json_file(
        &out_dir.join("functional_qa_receipt.json"),
        &functional_receipt,
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("functional_qa_receipt.json"),
    ));

    let security_receipt = security_qa_receipt(
        &target_repo,
        &context,
        &gate_view,
        &security_status,
        &security_findings,
        review_ready,
    );
    write_json_file(&out_dir.join("security_qa_receipt.json"), &security_receipt)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("security_qa_receipt.json"),
    ));

    let ac_mapping = ac_test_mapping_receipt(
        &context,
        &gate_view,
        &functional_status,
        &security_status,
        review_ready,
    );
    write_json_file(&out_dir.join("ac_test_mapping.json"), &ac_mapping)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("ac_test_mapping.json"),
    ));

    let qa_receipt = qa_receipt(
        &context,
        &gate_view,
        &functional_status,
        &security_status,
        &verdict,
        review_ready,
    );
    write_json_file(&out_dir.join("qa_receipt.json"), &qa_receipt)?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("qa_receipt.json")));

    let review_agent_gate = review_agent_gate(
        &context,
        &gate_view,
        &functional_status,
        &security_status,
        &verdict,
        review_ready,
    );
    write_json_file(&out_dir.join("review_agent_gate.json"), &review_agent_gate)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("review_agent_gate.json"),
    ));

    write_text_file(
        &out_dir.join("review_agent_gate_packet.md"),
        &review_agent_gate_packet_markdown(
            &context,
            &gate_view,
            &functional_status,
            &security_status,
            &verdict,
            review_ready,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("review_agent_gate_packet.md"),
    ));

    carry_forward_review_artifacts(&repo_root, &artifact_dir, &out_dir, &mut artifacts_written)?;

    write_json_file(
        &out_dir.join("runner_explanation.json"),
        &review_runner_explanation(
            &repo_root,
            &artifact_dir,
            &out_dir,
            &context,
            &verdict,
            &functional_status,
            &security_status,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    write_json_file(
        &out_dir.join("artifact_inventory.json"),
        &review_artifact_inventory(&repo_root, &out_dir, &context, rendered_at),
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
    let next_actions = review_next_actions(
        &repo_root,
        &artifact_dir,
        &guard,
        &gate,
        &functional_status,
        &security_status,
        &verdict,
    );

    Ok(ReviewResult {
        schema_version: "fda.review_result.v0",
        verdict,
        functional_qa_status: functional_status,
        security_qa_status: security_status,
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        target_repo: display_path(&repo_root, &target_repo),
        artifacts_written,
        validation_report_path: Some(display_path(&repo_root, &validation_report_path)),
        next_actions,
    })
}

pub(crate) fn review_runtime_context(artifact_dir: &Path) -> RuntimeContext {
    let mut context = implement_runtime_context(artifact_dir);
    context.case_ids = vec!["CASE-FDA-V1-REVIEW-001".to_string()];
    context.task_ids = vec!["PR-V1-007".to_string()];
    context
}

fn review_gate_status_from_artifacts(
    artifact_dir: &Path,
    target_repo_exists: bool,
) -> Result<ReviewGateStatus, String> {
    let implementation_path = artifact_dir.join("implementation_receipt.json");
    let external_pr_path = artifact_dir.join("external_pr_receipt.json");
    let mut issues = Vec::new();
    let mut evidence_links = Vec::new();
    let mut actual_pr_url = None;
    let mut reviewed_planned_pr_id = "PR-V1-006".to_string();
    let mut changed_files = Vec::new();

    if !target_repo_exists {
        issues.push("target repo cwd does not exist".to_string());
    }

    if implementation_path.exists() {
        evidence_links.push("implementation_receipt.json".to_string());
        let implementation = read_json_value(&implementation_path)?;
        push_unique_all(
            &mut changed_files,
            value_string_array(&implementation, "changed_files"),
        );
        reviewed_planned_pr_id =
            value_string(&implementation, "planned_pr_id").unwrap_or(reviewed_planned_pr_id);
        match value_string(&implementation, "status").as_deref() {
            Some("succeeded") => {}
            Some(status) => issues.push(format!("implementation_receipt status is {status}")),
            None => issues.push("implementation_receipt status is missing".to_string()),
        }
    } else {
        issues.push(format!(
            "implementation_receipt.json is required before review: {}",
            implementation_path.display()
        ));
    }

    if external_pr_path.exists() {
        evidence_links.push("external_pr_receipt.json".to_string());
        let external_pr = read_json_value(&external_pr_path)?;
        push_unique_all(
            &mut changed_files,
            value_string_array(&external_pr, "changed_files"),
        );
        reviewed_planned_pr_id =
            value_string(&external_pr, "planned_pr_id").unwrap_or(reviewed_planned_pr_id);
        match value_string(&external_pr, "status").as_deref() {
            Some("opened") => {}
            Some(status) => issues.push(format!("external_pr_receipt status is {status}")),
            None => issues.push("external_pr_receipt status is missing".to_string()),
        }
        match value_string(&external_pr, "actual_pr_url")
            .and_then(|url| normalize_github_pull_url(&url))
        {
            Some(url) => actual_pr_url = Some(url),
            None => issues
                .push("external_pr_receipt actual_pr_url must be a GitHub pull URL".to_string()),
        }
        match external_pr
            .get("checks")
            .and_then(|checks| value_string(checks, "tests"))
            .as_deref()
        {
            Some("passed") => {}
            Some(status) => issues.push(format!("external_pr_receipt checks.tests is {status}")),
            None => issues.push("external_pr_receipt checks.tests is missing".to_string()),
        }
    } else {
        issues.push(format!(
            "external_pr_receipt.json is required before review: {}",
            external_pr_path.display()
        ));
    }

    let pr_reviewer = reviewer_receipt_status(
        artifact_dir,
        "pr_reviewer_receipt.json",
        "pr_reviewer",
        true,
        actual_pr_url.as_deref(),
        &reviewed_planned_pr_id,
    )?;
    issues.extend(pr_reviewer.issues.clone());
    evidence_links.extend(pr_reviewer.evidence_links.clone());

    let forge_reviewer_required = changed_files.iter().any(|path| requires_forge_review(path));
    let forge_reviewer = reviewer_receipt_status_any(
        artifact_dir,
        "forge_reviewer",
        &[
            ("forge_reviewer_receipt.json", "forge_reviewer"),
            ("qax2_receipt.json", "qax2"),
            ("orchestrator_review_receipt.json", "orchestrator"),
            ("orchestrator_receipt.json", "orchestrator"),
        ],
        forge_reviewer_required,
        actual_pr_url.as_deref(),
        &reviewed_planned_pr_id,
    )?;
    issues.extend(forge_reviewer.issues.clone());
    evidence_links.extend(forge_reviewer.evidence_links.clone());

    let design_qa_required = changed_files.iter().any(|path| requires_design_qa(path));
    let design_qa = reviewer_receipt_status(
        artifact_dir,
        "design_qa_receipt.json",
        "design_qa",
        design_qa_required,
        actual_pr_url.as_deref(),
        &reviewed_planned_pr_id,
    )?;
    issues.extend(design_qa.issues.clone());
    evidence_links.extend(design_qa.evidence_links.clone());

    let status = if issues.is_empty() {
        "ready".to_string()
    } else {
        "blocked".to_string()
    };
    Ok(ReviewGateStatus {
        status,
        issues,
        evidence_links,
        actual_pr_url,
        reviewed_planned_pr_id,
        pr_reviewer_status: pr_reviewer.status,
        pr_reviewer_findings: pr_reviewer.findings,
        pr_reviewer_evidence_links: pr_reviewer.evidence_links,
        forge_reviewer_required,
        forge_reviewer_status: forge_reviewer.status,
        forge_reviewer_evidence_links: forge_reviewer.evidence_links,
        design_qa_required,
        design_qa_status: design_qa.status,
        design_qa_evidence_links: design_qa.evidence_links,
    })
}

struct ReviewerReceiptStatus {
    status: String,
    issues: Vec<String>,
    findings: Vec<String>,
    evidence_links: Vec<String>,
}

fn reviewer_receipt_status(
    artifact_dir: &Path,
    file_name: &str,
    role: &str,
    required: bool,
    actual_pr_url: Option<&str>,
    reviewed_planned_pr_id: &str,
) -> Result<ReviewerReceiptStatus, String> {
    let path = artifact_dir.join(file_name);
    if !path.exists() {
        let issues = if required {
            vec![format!(
                "{file_name} from read-only {role} is required before review gate aggregation"
            )]
        } else {
            Vec::new()
        };
        return Ok(ReviewerReceiptStatus {
            status: if required {
                "blocked"
            } else {
                "not_applicable"
            }
            .to_string(),
            issues,
            findings: Vec::new(),
            evidence_links: Vec::new(),
        });
    }

    let receipt = read_json_value(&path)?;
    let mut issues = Vec::new();
    let status = value_string(&receipt, "status").unwrap_or_else(|| "unknown".to_string());
    if status != "passed" {
        issues.push(format!("{file_name} status must be passed, got {status}"));
    }
    match value_string(&receipt, "role").as_deref() {
        Some(receipt_role) if receipt_role == role => {}
        Some(receipt_role) => {
            issues.push(format!(
                "{file_name} role must be {role}, got {receipt_role}"
            ));
        }
        None => issues.push(format!("{file_name} role is missing")),
    }
    if receipt
        .get("source_mutation_attempted")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        issues.push(format!(
            "{file_name} must record source_mutation_attempted=false"
        ));
    }
    match value_string(&receipt, "workspace_policy").as_deref() {
        Some("read_only") => {}
        Some(policy) => issues.push(format!(
            "{file_name} workspace_policy must be read_only, got {policy}"
        )),
        None => issues.push(format!("{file_name} workspace_policy is missing")),
    }
    if receipt
        .get("source_mutation_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        issues.push(format!(
            "{file_name} must record source_mutation_allowed=false"
        ));
    }
    match value_string(&receipt, "reviewed_planned_pr_id").as_deref() {
        Some(receipt_pr_id) if receipt_pr_id == reviewed_planned_pr_id => {}
        Some(receipt_pr_id) => issues.push(format!(
            "{file_name} reviewed_planned_pr_id must be {reviewed_planned_pr_id}, got {receipt_pr_id}"
        )),
        None => issues.push(format!("{file_name} reviewed_planned_pr_id is missing")),
    }
    if let Some(expected_url) = actual_pr_url {
        match value_string(&receipt, "actual_pr_url").and_then(|url| normalize_github_pull_url(&url))
        {
            Some(receipt_url) if receipt_url == expected_url => {}
            Some(receipt_url) => issues.push(format!(
                "{file_name} actual_pr_url must match current PR {expected_url}, got {receipt_url}"
            )),
            None => issues.push(format!(
                "{file_name} actual_pr_url must be a GitHub pull URL matching current PR {expected_url}"
            )),
        }
    }

    Ok(ReviewerReceiptStatus {
        status: if issues.is_empty() {
            "passed".to_string()
        } else {
            "blocked".to_string()
        },
        issues,
        findings: value_string_array(&receipt, "findings"),
        evidence_links: vec![file_name.to_string()],
    })
}

fn reviewer_receipt_status_any(
    artifact_dir: &Path,
    label: &str,
    candidates: &[(&str, &str)],
    required: bool,
    actual_pr_url: Option<&str>,
    reviewed_planned_pr_id: &str,
) -> Result<ReviewerReceiptStatus, String> {
    let mut blocked_candidates = Vec::new();
    for (file_name, role) in candidates {
        if !artifact_dir.join(file_name).exists() {
            continue;
        }
        let status = reviewer_receipt_status(
            artifact_dir,
            file_name,
            role,
            required,
            actual_pr_url,
            reviewed_planned_pr_id,
        )?;
        if status.issues.is_empty() {
            return Ok(status);
        }
        blocked_candidates.push(status);
    }

    if !blocked_candidates.is_empty() {
        let mut issues = Vec::new();
        let mut evidence_links = Vec::new();
        let mut findings = Vec::new();
        for candidate in blocked_candidates {
            issues.extend(candidate.issues);
            evidence_links.extend(candidate.evidence_links);
            findings.extend(candidate.findings);
        }
        return Ok(ReviewerReceiptStatus {
            status: "blocked".to_string(),
            issues,
            findings,
            evidence_links,
        });
    }

    let issues = if required {
        let names = candidates
            .iter()
            .map(|(file_name, _)| *file_name)
            .collect::<Vec<_>>()
            .join(", ");
        vec![format!(
            "one of {names} from read-only {label} fallback roles is required before review gate aggregation"
        )]
    } else {
        Vec::new()
    };
    Ok(ReviewerReceiptStatus {
        status: if required {
            "blocked"
        } else {
            "not_applicable"
        }
        .to_string(),
        issues,
        findings: Vec::new(),
        evidence_links: Vec::new(),
    })
}

fn push_unique_all(values: &mut Vec<String>, candidates: Vec<String>) {
    for candidate in candidates {
        if !values.contains(&candidate) {
            values.push(candidate);
        }
    }
}

fn requires_forge_review(path: &str) -> bool {
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

fn requires_design_qa(path: &str) -> bool {
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

fn qa_status_from_fixture(
    review_ready: bool,
    fixture: Option<&QaFixture>,
    security_role: bool,
) -> String {
    if !review_ready {
        return "blocked".to_string();
    }
    let Some(fixture) = fixture else {
        return "passed".to_string();
    };
    if security_role && severity_is_high_or_critical(fixture) && !fixture.findings.is_empty() {
        return "needs_human".to_string();
    }
    match fixture.status.to_ascii_lowercase().as_str() {
        "pass" | "passed" | "succeeded" => "passed".to_string(),
        "fail" | "failed" => "failed".to_string(),
        "blocked" => "blocked".to_string(),
        "needs_human" | "human_turn" => "needs_human".to_string(),
        _ => "failed".to_string(),
    }
}

fn severity_is_high_or_critical(fixture: &QaFixture) -> bool {
    fixture
        .severity
        .as_deref()
        .map(|severity| matches!(severity.to_ascii_lowercase().as_str(), "high" | "critical"))
        .unwrap_or(false)
}

fn qa_findings(
    review_ready: bool,
    gate: &ReviewGateStatus,
    fixture: Option<&QaFixture>,
) -> Vec<String> {
    if !review_ready {
        return gate.issues.clone();
    }
    fixture
        .map(|fixture| fixture.findings.clone())
        .unwrap_or_default()
}

fn review_verdict(functional_status: &str, security_status: &str) -> String {
    if matches!(functional_status, "blocked" | "needs_human")
        || matches!(security_status, "blocked" | "needs_human")
    {
        return "blocked".to_string();
    }
    if functional_status == "failed" || security_status == "failed" {
        return "fail".to_string();
    }
    if functional_status == "passed" && security_status == "passed" {
        return "pass".to_string();
    }
    "fail".to_string()
}

fn review_next_actions(
    repo_root: &Path,
    artifact_dir: &Path,
    guard: &HumanDecisionGuard,
    gate: &ReviewGateStatus,
    functional_status: &str,
    security_status: &str,
    verdict: &str,
) -> Vec<String> {
    if !guard.unresolved_decision_ids.is_empty() || !guard.non_approval_decision_ids.is_empty() {
        return guard
            .unresolved_decision_ids
            .iter()
            .chain(guard.non_approval_decision_ids.iter())
            .map(|decision_id| {
                format!(
                    "fda decide {} --answer <answer> --artifacts {}",
                    decision_id,
                    display_path(repo_root, artifact_dir)
                )
            })
            .collect();
    }
    if !gate.is_pass() {
        return vec![
            "implementation_receipt.json と external_pr_receipt.json を成功状態にして fda review を再実行する".to_string(),
        ];
    }
    if security_status == "needs_human" {
        return vec![
            "Security High/Critical finding を Human Decision として開く".to_string(),
            "fda status".to_string(),
        ];
    }
    if functional_status == "failed" || security_status == "failed" || verdict == "fail" {
        return vec![
            "fda continue".to_string(),
            "Implementer へ QA FAIL repair を戻す".to_string(),
        ];
    }
    vec!["fda merge".to_string()]
}
