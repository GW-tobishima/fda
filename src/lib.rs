use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) const DEFAULT_SCHEMA_DIR: &str = "docs/standards/delivery-artifacts-v0/schemas";
pub(crate) const DEFAULT_ARTIFACT_DIR: &str =
    "docs/standards/delivery-artifacts-v0/examples/forge_dashboard_epic";
pub(crate) const DEFAULT_MODEL_CONTRACT_DIRS: [&str; 2] = [
    "model_contracts",
    "docs/standards/delivery-artifacts-v0/model_contracts",
];

pub(crate) mod application;
pub mod cli;
pub(crate) mod domain;
pub(crate) mod infra;
pub(crate) mod rendering;
pub(crate) mod support;

pub(crate) use application::merge::MergeResult;
pub(crate) use application::notify::NotifyResult;
pub(crate) use application::output_hub::OpenResult;
pub(crate) use application::repair::ContinueResult;
pub(crate) use application::review::ReviewResult;

#[cfg(test)]
pub(crate) use application::decide::decide;
#[cfg(test)]
pub(crate) use application::design::design;
#[cfg(test)]
pub(crate) use application::notify::{
    live_notification_receipt, notification_request, resolve_notification_recipient,
    successful_slack_notification_receipt,
};
#[cfg(test)]
pub(crate) use application::output_hub::decision_rows_from_artifacts;
#[cfg(test)]
pub(crate) use application::plan::plan_fixture;
#[cfg(test)]
pub(crate) use application::start::start;
#[cfg(test)]
pub(crate) use infra::json_file::write_json_file;
#[cfg(test)]
pub(crate) use infra::slack::{slack_response_digest, slack_webhook_url, SlackSendResponse};
#[cfg(test)]
pub(crate) use infra::smtp::{
    smtp_envelope_address, smtp_message_id, smtp_resolve_addresses, SmtpConfig,
};
#[cfg(test)]
pub(crate) use rendering::notify::{
    base64_encode, slack_message_payload, smtp_message_body, smtp_plain_message_text,
};

use application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    recorded_decision_receipts_from_packet,
};
use cli::args::{
    ContinueConfig, ImplementConfig, MergeConfig, NotifyConfig, OpenConfig, ReviewConfig,
};
use domain::entities::{CodexLiveStatus, HumanDecisionSummary};
use domain::policies::decision::decision_receipt_answer;
use infra::json_file::read_json_value;
use infra::process::CodexMcpProcessAdapter;
use support::paths::display_path;

#[cfg(test)]
use cli::args::AtoConfig;
#[cfg(test)]
use std::path::PathBuf;

pub(crate) fn ui_serve(config: &application::ui::UiConfig) -> Result<(), String> {
    infra::ui_server::serve(config)
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn write_text_file(path: &Path, body: &str) -> Result<(), String> {
    fs::write(path, body).map_err(|e| format!("failed to write text file {}: {e}", path.display()))
}

fn ensure_artifact_dir_exists(artifact_dir: &Path) -> Result<(), String> {
    if artifact_dir.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "artifact dir {} must exist before reading artifacts",
            artifact_dir.display()
        ))
    }
}

fn copy_artifact_if_exists(
    repo_root: &Path,
    from_dir: &Path,
    to_dir: &Path,
    file_name: &str,
    artifacts_written: &mut Vec<String>,
) -> Result<(), String> {
    let from = from_dir.join(file_name);
    let to = to_dir.join(file_name);
    if !from.exists() {
        if to.exists() {
            fs::remove_file(&to).map_err(|e| {
                format!(
                    "failed to remove stale carried artifact {}: {e}",
                    to.display()
                )
            })?;
        }
        return Ok(());
    }
    if from == to {
        return Ok(());
    }
    if file_name == "human_decision_packet.json" {
        let packet = read_json_value(&from)?;
        if !human_decision_packet_is_projectable(&packet) {
            if to.exists() {
                fs::remove_file(&to).map_err(|e| {
                    format!(
                        "failed to remove stale carried artifact {}: {e}",
                        to.display()
                    )
                })?;
            }
            return Ok(());
        }
    }
    fs::copy(&from, &to).map_err(|e| {
        format!(
            "failed to carry forward artifact {} to {}: {e}",
            from.display(),
            to.display()
        )
    })?;
    artifacts_written.push(display_path(repo_root, &to));
    Ok(())
}

fn human_decision_packet_is_projectable(packet: &Value) -> bool {
    [
        "decision_packet_id",
        "program_id",
        "epic_id",
        "status",
        "required_before",
        "decision_needed",
        "trigger",
        "context",
        "options",
        "impact",
        "default_if_no_decision",
        "forge_mapping",
    ]
    .iter()
    .all(|key| packet.get(*key).is_some())
}

fn remove_artifact_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| format!("failed to remove stale artifact {}: {e}", path.display()))?;
    }
    Ok(())
}

fn carry_forward_artifacts(
    repo_root: &Path,
    from_dir: &Path,
    to_dir: &Path,
    file_names: &[&str],
    artifacts_written: &mut Vec<String>,
) -> Result<(), String> {
    for file_name in file_names {
        copy_artifact_if_exists(repo_root, from_dir, to_dir, file_name, artifacts_written)?;
    }
    Ok(())
}

fn carry_forward_implement_artifacts(
    repo_root: &Path,
    from_dir: &Path,
    to_dir: &Path,
    artifacts_written: &mut Vec<String>,
) -> Result<(), String> {
    carry_forward_artifacts(
        repo_root,
        from_dir,
        to_dir,
        &[
            "risk_register.json",
            "risk_register.md",
            "forge_projection.json",
            "human_decision_packet.json",
            "decision_receipts.json",
        ],
        artifacts_written,
    )
}

fn carry_forward_review_artifacts(
    repo_root: &Path,
    from_dir: &Path,
    to_dir: &Path,
    artifacts_written: &mut Vec<String>,
) -> Result<(), String> {
    carry_forward_artifacts(
        repo_root,
        from_dir,
        to_dir,
        &[
            "implementation_receipt.json",
            "external_pr_receipt.json",
            "forge_reviewer_receipt.json",
            "design_qa_receipt.json",
            "retry_history.json",
            "risk_register.json",
            "risk_register.md",
            "forge_projection.json",
            "human_decision_packet.json",
            "decision_receipts.json",
        ],
        artifacts_written,
    )
}

fn carry_forward_repair_artifacts(
    repo_root: &Path,
    from_dir: &Path,
    to_dir: &Path,
    artifacts_written: &mut Vec<String>,
) -> Result<(), String> {
    carry_forward_artifacts(
        repo_root,
        from_dir,
        to_dir,
        &[
            "implementation_receipt.json",
            "external_pr_receipt.json",
            "qa_receipt.json",
            "functional_qa_receipt.json",
            "security_qa_receipt.json",
            "ac_test_mapping.json",
            "risk_register.json",
            "risk_register.md",
            "forge_projection.json",
            "human_decision_packet.json",
            "decision_receipts.json",
        ],
        artifacts_written,
    )
}
#[derive(Debug, Serialize)]
pub(crate) struct ImplementResult {
    schema_version: &'static str,
    mode: String,
    verdict: String,
    dry_run_gate_status: String,
    development_gate_status: Option<String>,
    artifact_dir: String,
    out_dir: String,
    target_repo: String,
    artifacts_written: Vec<String>,
    detected_tools: Vec<String>,
    missing_tools: Vec<String>,
    actual_pr_url: Option<String>,
    thread_id: Option<String>,
    validation_report_path: Option<String>,
    next_actions: Vec<String>,
}

struct HumanDecisionGuard {
    unresolved_decision_ids: Vec<String>,
    non_approval_decision_ids: Vec<String>,
}

pub(crate) fn implement(config: &ImplementConfig) -> Result<ImplementResult, String> {
    application::implement::implement(config, &CodexMcpProcessAdapter)
}

pub(crate) fn review(config: &ReviewConfig) -> Result<ReviewResult, String> {
    application::review::review(config)
}

pub(crate) fn continue_run(config: &ContinueConfig) -> Result<ContinueResult, String> {
    application::repair::continue_run(config)
}

pub(crate) fn merge_run(config: &MergeConfig) -> Result<MergeResult, String> {
    application::merge::merge_run(config)
}

pub(crate) fn open_output_hub(config: &OpenConfig) -> Result<OpenResult, String> {
    application::output_hub::open_output_hub(config)
}

pub(crate) fn notify_test(config: &NotifyConfig) -> Result<NotifyResult, String> {
    application::notify::notify_test(config)
}

struct DryRunGateStatus {
    status: String,
    issues: Vec<String>,
    evidence_links: Vec<String>,
}

impl DryRunGateStatus {
    fn is_pass(&self) -> bool {
        self.status == "succeeded" && self.issues.is_empty()
    }
}

fn implementation_status(
    status: CodexLiveStatus,
    actual_pr_url: Option<&str>,
    test_status: &str,
) -> &'static str {
    match status {
        CodexLiveStatus::AdapterUnavailable => "adapter_unavailable",
        CodexLiveStatus::Blocked => "blocked",
        CodexLiveStatus::Failed => "failed",
        CodexLiveStatus::Succeeded => {
            if actual_pr_url.is_some() && test_status == "passed" {
                "succeeded"
            } else if actual_pr_url.is_some() {
                "failed"
            } else {
                "blocked"
            }
        }
    }
}

fn implementation_semantic_verdict(status: &str) -> &'static str {
    match status {
        "succeeded" => "pass",
        "adapter_unavailable" => "adapter_unavailable",
        "blocked" => "blocked",
        _ => "fail",
    }
}

fn implementation_gate_effect(status: &str) -> &'static str {
    match status {
        "succeeded" => "advance",
        "failed" => "repair",
        _ => "hold",
    }
}

fn parse_actual_pr_url(content: &str) -> Option<String> {
    if let Some(value) = marker_value(content, "FDA_ACTUAL_PR_URL:") {
        return normalize_github_pull_url(&value);
    }
    content
        .split_whitespace()
        .find_map(normalize_github_pull_url)
}

fn normalize_github_pull_url(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | ')' | ']' | '}' | ',' | '.' | ';'));
    if matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "none" | "n/a" | "na" | "null" | "unavailable" | "-"
    ) {
        return None;
    }
    let path = trimmed.strip_prefix("https://github.com/")?;
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    let pull = parts.next()?;
    let number = parts.next()?;
    if owner.is_empty() || repo.is_empty() || pull != "pull" {
        return None;
    }
    parse_pr_number_from_url(trimmed)?;
    if number.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn parse_pr_number_from_url(url: &str) -> Option<u64> {
    url.split("/pull/")
        .nth(1)
        .and_then(|tail| tail.split(|ch: char| !ch.is_ascii_digit()).next())
        .and_then(|number| number.parse::<u64>().ok())
}

fn parse_codex_test_status(content: &str) -> String {
    marker_value(content, "FDA_TEST_STATUS:")
        .map(|value| value.to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "passed" | "pass" => Some("passed".to_string()),
            "failed" | "fail" => Some("failed".to_string()),
            "not_run" | "not run" => Some("not_run".to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "not_run".to_string())
}

fn parse_scope_drift(content: &str) -> Vec<String> {
    marker_value(content, "FDA_SCOPE_DRIFT:")
        .map(|value| {
            if matches!(value.to_ascii_lowercase().as_str(), "none" | "なし" | "n/a") {
                Vec::new()
            } else {
                vec![value]
            }
        })
        .unwrap_or_default()
}

fn parse_marker_list(content: &str, marker: &str) -> Vec<String> {
    marker_value(content, marker)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty() && *item != "NONE")
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn marker_value(content: &str, marker: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix(marker)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn human_decision_guard_with(
    artifact_dir: &Path,
    approval_predicate: fn(&HumanDecisionSummary, &str) -> bool,
) -> Result<HumanDecisionGuard, String> {
    let packet_path = artifact_dir.join("human_decision_packet.json");
    if !packet_path.exists() {
        return Ok(HumanDecisionGuard {
            unresolved_decision_ids: Vec::new(),
            non_approval_decision_ids: Vec::new(),
        });
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
        let store = infra::fs_store::FsArtifactStore;
        read_decision_receipts(&store, &receipts_path)?
    };
    let answers = decision_answers_from_receipts(&receipts);
    let mut unresolved_decision_ids = Vec::new();
    let mut non_approval_decision_ids = Vec::new();
    for decision in decisions {
        match decision_receipt_answer(&decision, &answers) {
            Some(answer) if approval_predicate(&decision, &answer) => {}
            Some(_) => non_approval_decision_ids.push(decision.decision_id),
            None => unresolved_decision_ids.push(decision.decision_id),
        }
    }
    Ok(HumanDecisionGuard {
        unresolved_decision_ids,
        non_approval_decision_ids,
    })
}

fn now_unix_seconds() -> u64 {
    infra::clock::system_unix_seconds()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::ArtifactValidator;
    use crate::application::validate::{artifact_name_from_schema, validate_case_pr_links};
    use crate::cli::args::{
        parse_args, CodexLiveFixture, Command, ContinueConfig, DecideConfig, DesignConfig,
        ImplementConfig, MergeConfig, MergeMethod, NotifyConfig, OpenConfig, PlanConfig, PlanMode,
        QaFixture, ReviewConfig, StartConfig, StartInput,
    };
    use crate::domain::value_objects::IntakeMode;
    use crate::infra::json_schema::JsonSchemaArtifactValidator;
    use serde_json::json;
    use std::env;
    use std::process::Command as ProcessCommand;
    use std::sync::{Mutex, MutexGuard};

    fn slack_env_lock() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn init_merge_target_repo(target: &Path, origin_url: &str) {
        fs::create_dir_all(target).unwrap();
        let init_status = ProcessCommand::new("git")
            .args(["-c", "init.defaultBranch=main", "init", "-q"])
            .current_dir(target)
            .status()
            .unwrap();
        assert!(init_status.success());
        let remote_status = ProcessCommand::new("git")
            .args(["remote", "add", "origin", origin_url])
            .current_dir(target)
            .status()
            .unwrap();
        assert!(remote_status.success());
    }

    #[test]
    fn derives_artifact_name_from_schema_file() {
        let path = Path::new("schemas/epic_delivery_plan.schema.json");
        assert_eq!(
            artifact_name_from_schema(path).unwrap(),
            "epic_delivery_plan"
        );
    }

    #[test]
    fn detects_missing_case_reference_from_epic_plan() {
        let plan = json!({
            "status": "ready",
            "claim_tree": [{"claim_id": "CLM-001"}],
            "case_graph": [{
                "case_id": "CASE-001",
                "depends_on": ["CASE-MISSING"],
                "claim_ids": ["CLM-001"],
                "planned_pr": "PR-001"
            }],
            "pr_plan": [{"planned_pr_id": "PR-001", "case_id": "CASE-001"}],
            "proof_strategy": [{"claim_id": "CLM-001"}]
        });

        let errors = validate_case_pr_links(&plan);
        assert_eq!(
            errors,
            vec!["case CASE-001 depends on missing case CASE-MISSING"]
        );
    }

    #[test]
    fn accepts_consistent_ready_epic_plan_links() {
        let plan = json!({
            "status": "ready",
            "claim_tree": [{"claim_id": "CLM-001"}],
            "case_graph": [{
                "case_id": "CASE-001",
                "depends_on": [],
                "claim_ids": ["CLM-001"],
                "planned_pr": "PR-001"
            }],
            "pr_plan": [{"planned_pr_id": "PR-001", "case_id": "CASE-001"}],
            "proof_strategy": [{"claim_id": "CLM-001"}]
        });

        assert!(validate_case_pr_links(&plan).is_empty());
    }

    #[test]
    fn carry_forward_artifacts_overwrites_and_removes_stale_outputs() {
        let base = env::temp_dir().join(format!(
            "fda-carry-forward-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let source = base.join("source");
        let out = base.join("out");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&out).unwrap();
        fs::write(source.join("external_pr_receipt.json"), "{\"version\":2}").unwrap();
        fs::write(out.join("external_pr_receipt.json"), "{\"version\":1}").unwrap();
        fs::write(out.join("risk_register.md"), "stale risk").unwrap();
        let mut artifacts_written = Vec::new();

        carry_forward_artifacts(
            Path::new("."),
            &source,
            &out,
            &["external_pr_receipt.json", "risk_register.md"],
            &mut artifacts_written,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(out.join("external_pr_receipt.json")).unwrap(),
            "{\"version\":2}"
        );
        assert!(!out.join("risk_register.md").exists());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn repository_profile_gate_creates_missing_fda_profile_without_overwriting_existing_files() {
        let base = env::temp_dir().join(format!(
            "fda-profile-gate-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }
        fs::create_dir_all(base.join(".fda")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"sample\"\n").unwrap();
        fs::write(
            base.join(".fda").join("repo.yaml"),
            "repo:\n  id: 'existing'\n  name: 'existing'\n  default_branch: 'main'\n  stack:\n    language: 'rust'\n  commands:\n    test: 'cargo test'\n",
        )
        .unwrap();
        let store = crate::infra::fs_store::FsArtifactStore;

        let created = crate::application::profile::ensure_target_repository_profile_if_present(
            &store,
            &base,
            &env::current_dir().unwrap(),
        )
        .unwrap();

        assert_eq!(
            created.len(),
            crate::application::profile::REPOSITORY_PROFILE_FILES.len() - 1
        );
        assert_eq!(
            fs::read_to_string(base.join(".fda").join("repo.yaml")).unwrap(),
            "repo:\n  id: 'existing'\n  name: 'existing'\n  default_branch: 'main'\n  stack:\n    language: 'rust'\n  commands:\n    test: 'cargo test'\n"
        );
        for file_name in crate::application::profile::REPOSITORY_PROFILE_FILES {
            assert!(base.join(".fda").join(file_name).exists());
        }

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn repository_profile_gate_uses_fda_schema_root_for_external_repo() {
        let base = env::temp_dir().join(format!(
            "fda-profile-schema-root-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"sample\"\n").unwrap();
        let store = crate::infra::fs_store::FsArtifactStore;

        let created = crate::application::profile::ensure_repository_profile(&store, &base)
            .expect("external repo profile should validate with FDA schema root");

        assert_eq!(
            created.len(),
            crate::application::profile::REPOSITORY_PROFILE_FILES.len()
        );
        assert!(!base
            .join("docs/standards/fda-v1/schemas/repository-profile")
            .exists());
        for file_name in crate::application::profile::REPOSITORY_PROFILE_FILES {
            assert!(base.join(".fda").join(file_name).exists());
        }

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn repository_profile_gate_rejects_invalid_existing_profile() {
        let base = env::temp_dir().join(format!(
            "fda-profile-invalid-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }
        fs::create_dir_all(base.join(".fda")).unwrap();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"sample\"\n").unwrap();
        fs::write(base.join(".fda").join("repo.yaml"), "repo: existing\n").unwrap();
        let store = crate::infra::fs_store::FsArtifactStore;

        let error = crate::application::profile::ensure_target_repository_profile_if_present(
            &store,
            &base,
            &env::current_dir().unwrap(),
        )
        .unwrap_err();

        assert!(error.contains("FDA repository profile validation failed"));
        assert!(error.contains("repo.yaml"));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn parses_start_goal_args() {
        let command = parse_args(vec![
            "start".to_string(),
            "oshi-noteでVTuber紹介リンクを作りたい".to_string(),
            "--out".to_string(),
            "tmp/intake".to_string(),
        ])
        .unwrap();

        match command {
            Command::Start(config) => {
                assert!(matches!(config.input, StartInput::Goal(_)));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/intake"));
                assert!(matches!(config.mode, IntakeMode::Auto));
            }
            _ => panic!("expected start command"),
        }
    }

    #[test]
    fn parses_design_args() {
        let command = parse_args(vec![
            "design".to_string(),
            "--artifacts".to_string(),
            "tmp/intake".to_string(),
            "--out".to_string(),
            "tmp/design".to_string(),
        ])
        .unwrap();

        match command {
            Command::Design(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/intake"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/design"));
            }
            _ => panic!("expected design command"),
        }
    }

    #[test]
    fn parses_decide_args() {
        let command = parse_args(vec![
            "decide".to_string(),
            "HD-FDA-001".to_string(),
            "--answer".to_string(),
            "yes".to_string(),
            "--artifacts".to_string(),
            "tmp/intake".to_string(),
        ])
        .unwrap();

        match command {
            Command::Decide(config) => {
                assert_eq!(config.decision_id, "HD-FDA-001");
                assert_eq!(config.answer, "yes");
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/intake"));
            }
            _ => panic!("expected decide command"),
        }
    }

    #[test]
    fn parses_implement_dry_run_args() {
        let command = parse_args(vec![
            "implement".to_string(),
            "--dry-run".to_string(),
            "--artifacts".to_string(),
            "tmp/design".to_string(),
            "--out".to_string(),
            "tmp/dry-run".to_string(),
            "--target-repo".to_string(),
            "/tmp/target-repo".to_string(),
        ])
        .unwrap();

        match command {
            Command::Implement(config) => {
                assert!(config.dry_run);
                assert!(!config.live);
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/design"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/dry-run"));
                assert_eq!(config.target_repo, PathBuf::from("/tmp/target-repo"));
                assert_eq!(config.live_timeout_seconds, 1800);
            }
            _ => panic!("expected implement command"),
        }
    }

    #[test]
    fn parses_implement_live_args() {
        let command = parse_args(vec![
            "implement".to_string(),
            "--live".to_string(),
            "--artifacts".to_string(),
            "tmp/dry-run".to_string(),
            "--out".to_string(),
            "tmp/live".to_string(),
            "--target-repo".to_string(),
            "/tmp/target-repo".to_string(),
            "--live-timeout-seconds".to_string(),
            "30".to_string(),
        ])
        .unwrap();

        match command {
            Command::Implement(config) => {
                assert!(!config.dry_run);
                assert!(config.live);
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/dry-run"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/live"));
                assert_eq!(config.target_repo, PathBuf::from("/tmp/target-repo"));
                assert_eq!(config.live_timeout_seconds, 30);
            }
            _ => panic!("expected implement command"),
        }
    }

    #[test]
    fn implement_rejects_zero_live_timeout() {
        let result = parse_args(vec![
            "implement".to_string(),
            "--live".to_string(),
            "--live-timeout-seconds".to_string(),
            "0".to_string(),
        ]);
        let error = match result {
            Ok(_) => panic!("expected parse error"),
            Err(error) => error,
        };

        assert!(error.contains("positive integer"));
    }

    #[test]
    fn implement_rejects_excessive_live_timeout() {
        let result = parse_args(vec![
            "implement".to_string(),
            "--live".to_string(),
            "--live-timeout-seconds".to_string(),
            "86401".to_string(),
        ]);
        let error = match result {
            Ok(_) => panic!("expected parse error"),
            Err(error) => error,
        };

        assert!(error.contains("must be <="));
    }

    #[test]
    fn parses_review_args() {
        let command = parse_args(vec![
            "review".to_string(),
            "--artifacts".to_string(),
            "tmp/live".to_string(),
            "--out".to_string(),
            "tmp/review".to_string(),
            "--target-repo".to_string(),
            "/tmp/target-repo".to_string(),
        ])
        .unwrap();

        match command {
            Command::Review(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/live"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/review"));
                assert_eq!(config.target_repo, PathBuf::from("/tmp/target-repo"));
            }
            _ => panic!("expected review command"),
        }
    }

    #[test]
    fn parses_continue_args() {
        let command = parse_args(vec![
            "continue".to_string(),
            "--artifacts".to_string(),
            "tmp/review".to_string(),
            "--out".to_string(),
            "tmp/repair".to_string(),
            "--target-repo".to_string(),
            "/tmp/target-repo".to_string(),
            "--max-retries".to_string(),
            "5".to_string(),
        ])
        .unwrap();

        match command {
            Command::Continue(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/review"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/repair"));
                assert_eq!(config.target_repo, PathBuf::from("/tmp/target-repo"));
                assert_eq!(config.max_retries, 5);
            }
            _ => panic!("expected continue command"),
        }
    }

    #[test]
    fn parses_merge_args() {
        let command = parse_args(vec![
            "merge".to_string(),
            "--artifacts".to_string(),
            "tmp/review".to_string(),
            "--out".to_string(),
            "tmp/merge".to_string(),
            "--target-repo".to_string(),
            "/tmp/target-repo".to_string(),
            "--execute".to_string(),
            "--merge-method".to_string(),
            "squash".to_string(),
        ])
        .unwrap();

        match command {
            Command::Merge(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/review"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/merge"));
                assert_eq!(config.target_repo, PathBuf::from("/tmp/target-repo"));
                assert!(config.execute);
                assert_eq!(config.merge_method, crate::cli::args::MergeMethod::Squash);
            }
            _ => panic!("expected merge command"),
        }
    }

    #[test]
    fn parses_open_args() {
        let command = parse_args(vec![
            "open".to_string(),
            "--artifacts".to_string(),
            "tmp/intake".to_string(),
            "--out".to_string(),
            "tmp/hub".to_string(),
        ])
        .unwrap();

        match command {
            Command::Open(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/intake"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/hub"));
            }
            _ => panic!("expected open command"),
        }
    }

    #[test]
    fn parses_status_args() {
        let command = parse_args(vec![
            "status".to_string(),
            "--artifacts".to_string(),
            "tmp/intake".to_string(),
            "--repo-root".to_string(),
            "/tmp/repo".to_string(),
            "--json".to_string(),
        ])
        .unwrap();

        match command {
            Command::Status(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/intake"));
                assert_eq!(config.repo_root, PathBuf::from("/tmp/repo"));
                assert!(config.print_json);
            }
            _ => panic!("expected status command"),
        }
    }

    #[test]
    fn parses_notify_test_args() {
        let command = parse_args(vec![
            "notify".to_string(),
            "test".to_string(),
            "--artifacts".to_string(),
            "tmp/intake".to_string(),
            "--out".to_string(),
            "tmp/notify".to_string(),
            "--channel".to_string(),
            "email".to_string(),
            "--to".to_string(),
            "user@example.com".to_string(),
            "--live".to_string(),
        ])
        .unwrap();

        match command {
            Command::NotifyTest(config) => {
                assert_eq!(config.artifact_dir, PathBuf::from("tmp/intake"));
                assert_eq!(config.out.unwrap(), PathBuf::from("tmp/notify"));
                assert_eq!(config.channel, "email");
                assert_eq!(config.recipient.as_deref(), Some("user@example.com"));
                assert!(config.live);
            }
            _ => panic!("expected notify test command"),
        }
    }

    #[test]
    fn implement_rejects_ambiguous_mode_args() {
        let result = parse_args(vec![
            "implement".to_string(),
            "--dry-run".to_string(),
            "--live".to_string(),
        ]);
        let error = match result {
            Ok(_) => panic!("expected parse error"),
            Err(error) => error,
        };

        assert!(error.contains("exactly one of --dry-run or --live"));
    }

    #[test]
    fn implement_dry_run_writes_artifacts_preserves_source_files_and_creates_target_profile() {
        let base = env::temp_dir().join(format!(
            "fda-implement-dry-run-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("sentinel.txt"), "unchanged").unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts.clone(),
            out: Some(out.clone()),
            target_repo: target.clone(),
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.dry_run_gate_status, "succeeded");
        assert_eq!(
            fs::read_to_string(target.join("sentinel.txt")).unwrap(),
            "unchanged"
        );
        for file_name in crate::application::profile::REPOSITORY_PROFILE_FILES {
            assert!(target.join(".fda").join(file_name).exists());
        }
        for file_name in [
            "implementation_handoff.md",
            "codex_prompt.md",
            "functional_qa_prompt.md",
            "security_qa_prompt.md",
            "agent_role_policy.json",
            "current_codex_cli_handoff.json",
            "planned_pr_execution_packet.json",
            "mcp_agent_invocation_plan.json",
            "mcp_tool_call_receipt.json",
            "dry_run_receipt.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        let dry_run_receipt = read_json_value(&out.join("dry_run_receipt.json")).unwrap();
        assert_eq!(
            read_json_value(&out.join("current_codex_cli_handoff.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("ready")
        );
        assert_eq!(
            dry_run_receipt
                .get("target_repo_mutated")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            read_json_value(&out.join("validation_report.json"))
                .unwrap()
                .get("verdict")
                .and_then(Value::as_str),
            Some("pass")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    fn write_successful_live_dry_run_receipt(artifacts: &Path, target: &Path) {
        write_json_file(
            &artifacts.join("dry_run_receipt.json"),
            &json!({
                "schema_version": "fda.mcp_dry_run_receipt.v0",
                "receipt_id": "MCPDRY-FDA-TEST-001",
                "plan_id": "MCP-FDA-TEST-001",
                "invocation_id": "INV-FDA-TEST-001",
                "provider": "codex",
                "mcp_server_command": ["codex", "mcp-server"],
                "cwd": target.to_string_lossy(),
                "status": "succeeded",
                "started_at": "unix:1",
                "completed_at": "unix:2",
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
                ],
                "evidence_links": ["mcp_tool_call_receipt.json"]
            }),
        )
        .unwrap();
    }

    fn write_successful_review_inputs(artifacts: &Path, target: &Path) {
        write_json_file(
            &artifacts.join("implementation_receipt.json"),
            &json!({
                "schema_version": "fda.implementation_receipt.v0",
                "receipt_id": "IMPL-FDA-TEST-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "planned_pr_id": "PR-V1-006",
                "provider": "codex",
                "mcp_server_command": ["codex", "mcp-server"],
                "cwd": target.to_string_lossy(),
                "status": "succeeded",
                "started_at": "unix:1",
                "completed_at": "unix:2",
                "thread_id": "thread-test-001",
                "dry_run_gate": {
                    "status": "succeeded",
                    "issues": [],
                    "evidence_links": ["dry_run_receipt.json"]
                },
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "changed_files": ["src/lib.rs"],
                "tests": [
                    { "command": "cargo test", "status": "passed", "summary": "passed" }
                ],
                "scope_drift": [],
                "input_artifacts": ["implementation_handoff.md"],
                "output_artifacts": ["external_pr_receipt.json"],
                "evidence_links": ["mcp_tool_call_receipt.json", "external_pr_receipt.json"],
                "next_action": "fda review",
                "summary": "implementation succeeded"
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "schema_version": "external_pr_receipt.v0",
                "receipt_id": "EXTPR-FDA-TEST-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "source_packet_id": "PPEXEC-FDA-TEST-001",
                "target_repo": target.to_string_lossy(),
                "planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "status": "opened",
                "checks": {
                    "tests": "passed",
                    "lint": "passed",
                    "security": "passed"
                },
                "evidence": ["implementation_receipt.json"],
                "human_decisions_resolved": [],
                "open_issues": [],
                "scope_disposition": {
                    "kind": "within_scope",
                    "closure_recommendation": "close_planned_pr",
                    "affected_planned_pr_ids": ["PR-V1-006"],
                    "summary": "No scope drift"
                },
                "target_pr": {
                    "url": "https://github.com/msamunetogetoge/example/pull/123",
                    "number": 123,
                    "head_sha": "abc123",
                    "state": "open"
                },
                "changed_files": ["src/lib.rs"],
                "acceptance_criteria_status": [
                    {
                        "criterion": "planned PR と actual PR が対応し、test 結果が receipt に残る",
                        "status": "pass",
                        "evidence": "implementation_receipt.json"
                    }
                ],
                "validation": [
                    { "command": "cargo test", "status": "pass", "summary": "passed" }
                ],
                "security_privacy_legal_review": "PR-V1-007で確認する。",
                "rollback_plan": "close PR if needed",
                "notes": "external PR opened"
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("pr_reviewer_receipt.json"),
            &json!({
                "schema_version": "fda.pr_reviewer_receipt.v0",
                "receipt_id": "PRR-FDA-TEST-001",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "role": "pr_reviewer",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "status": "passed",
                "findings": [],
                "source_mutation_attempted": false,
                "evidence_links": ["external_pr_receipt.json", "implementation_receipt.json"]
            }),
        )
        .unwrap();
    }

    fn write_review_receipts(
        artifacts: &Path,
        functional_status: &str,
        security_status: &str,
        return_to_role: Option<&str>,
        findings: &[&str],
    ) {
        let qa_status = if functional_status == "passed" && security_status == "passed" {
            "passed"
        } else if security_status == "needs_human" {
            "needs_human"
        } else {
            "failed"
        };
        write_json_file(
            &artifacts.join("pr_reviewer_receipt.json"),
            &json!({
                "schema_version": "fda.pr_reviewer_receipt.v0",
                "receipt_id": "PRR-FDA-TEST-001",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "role": "pr_reviewer",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "status": "passed",
                "findings": [],
                "source_mutation_attempted": false,
                "evidence_links": ["external_pr_receipt.json"]
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("qa_receipt.json"),
            &json!({
                "schema_version": "fda.qa_receipt.v0",
                "receipt_id": "QA-FDA-TEST-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "status": qa_status,
                "functional_qa_status": functional_status,
                "security_qa_status": security_status,
                "return_to_role": return_to_role,
                "review_gate_issues": findings,
                "evidence_links": ["functional_qa_receipt.json", "security_qa_receipt.json"],
                "next_action": if qa_status == "passed" { "fda merge" } else { "fda continue" }
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("functional_qa_receipt.json"),
            &json!({
                "schema_version": "fda.functional_qa_receipt.v0",
                "receipt_id": "FQA-FDA-TEST-001",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "role": "functional_qa",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "status": functional_status,
                "findings": findings,
                "return_to_role": if functional_status == "failed" { json!("implementer") } else { Value::Null },
                "source_mutation_attempted": false
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("security_qa_receipt.json"),
            &json!({
                "schema_version": "fda.security_qa_receipt.v0",
                "receipt_id": "SQA-FDA-TEST-001",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "role": "security_qa",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "status": security_status,
                "findings": if security_status == "failed" || security_status == "needs_human" { findings } else { &[] as &[&str] },
                "return_to_role": if security_status == "failed" { json!("implementer") } else if security_status == "needs_human" { json!("human_security_approval") } else { Value::Null },
                "source_mutation_attempted": false,
                "not_copied_from_functional_qa": true
            }),
        )
        .unwrap();
    }

    fn write_merge_inputs(artifacts: &Path, qa_status: &str, risk_category: &str) {
        write_review_receipts(
            artifacts,
            if qa_status == "passed" {
                "passed"
            } else {
                "failed"
            },
            "passed",
            if qa_status == "passed" {
                None
            } else {
                Some("implementer")
            },
            if qa_status == "passed" {
                &[]
            } else {
                &["acceptance criterion is not covered"]
            },
        );
        write_json_file(
            &artifacts.join("external_pr_receipt.json"),
            &json!({
                "schema_version": "external_pr_receipt.v0",
                "receipt_id": "EXTPR-FDA-TEST-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "source_packet_id": "PPEXEC-FDA-TEST-001",
                "target_repo": "/tmp/target",
                "planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "status": "opened",
                "checks": {
                    "tests": "passed",
                    "lint": "passed",
                    "security": "passed"
                },
                "evidence": ["implementation_receipt.json"],
                "human_decisions_resolved": [],
                "open_issues": [],
                "scope_disposition": {
                    "kind": "within_scope",
                    "closure_recommendation": "close_planned_pr",
                    "affected_planned_pr_ids": ["PR-V1-006"],
                    "summary": "No scope drift"
                },
                "target_pr": {
                    "url": "https://github.com/msamunetogetoge/example/pull/123",
                    "number": 123,
                    "head_sha": "abc123",
                    "state": "open"
                },
                "changed_files": ["src/lib.rs"],
                "acceptance_criteria_status": [
                    {
                        "criterion": "merge gate test",
                        "status": "pass",
                        "evidence": "qa_receipt.json"
                    }
                ],
                "validation": [
                    { "command": "cargo test", "status": "pass", "summary": "passed" }
                ],
                "security_privacy_legal_review": "Merge Gateで確認する。",
                "rollback_plan": "revert merge if needed",
                "notes": "external PR opened"
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("repair_receipt.json"),
            &json!({
                "schema_version": "fda.repair_receipt.v0",
                "receipt_id": "REPAIR-FDA-TEST-001",
                "status": "no_repair_needed",
                "failure_classification": "none",
                "retry_attempt_count": 0,
                "retry_limit": 3,
                "retry_limit_reached": false
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("risk_register.json"),
            &json!({
                "schema_version": "fda.risk_register.v0",
                "risks": [
                    {
                        "risk_id": "RISK-FDA-TEST-001",
                        "category": risk_category,
                        "severity": "low",
                        "summary": "test risk"
                    }
                ]
            }),
        )
        .unwrap();
        write_promoted_forge_projection(artifacts);
        write_review_agent_gate_inputs(artifacts, qa_status);
    }

    fn write_review_agent_gate_inputs(artifacts: &Path, qa_status: &str) {
        let functional_status = if qa_status == "passed" {
            "passed"
        } else {
            "failed"
        };
        let gate_status = if qa_status == "passed" {
            "passed"
        } else {
            "failed"
        };
        let pr_packet_status = "REVIEW_AGENT_OK";
        let functional_packet_status = if functional_status == "passed" {
            "REVIEW_AGENT_OK"
        } else {
            "REVIEW_AGENT_HOLD"
        };
        write_json_file(
            &artifacts.join("ac_test_mapping.json"),
            &json!({
                "schema_version": "fda.ac_test_mapping.v0",
                "mappings": [
                    {
                        "acceptance_criterion": "merge gate test",
                        "test_or_check": "cargo test",
                        "status": "pass",
                        "evidence": "qa_receipt.json"
                    }
                ]
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("review_agent_gate.json"),
            &json!({
                "schema_version": "fda.review_agent_gate.v0",
                "gate_id": "REVIEW-GATE-FDA-TEST-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "status": gate_status,
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "required_reviewers": [
                    {
                        "role": "pr_reviewer",
                        "required": true,
                        "status": "passed",
                        "workspace_policy": "read_only",
                        "source_mutation_allowed": false,
                        "evidence": ["pr_reviewer_receipt.json"],
                        "not_applicable_reason": Value::Null
                    },
                    {
                        "role": "functional_qa",
                        "required": true,
                        "status": functional_status,
                        "workspace_policy": "read_only",
                        "source_mutation_allowed": false,
                        "evidence": ["functional_qa_receipt.json", "ac_test_mapping.json"],
                        "not_applicable_reason": Value::Null
                    },
                    {
                        "role": "security_qa",
                        "required": true,
                        "status": "passed",
                        "workspace_policy": "read_only",
                        "source_mutation_allowed": false,
                        "evidence": ["security_qa_receipt.json"],
                        "not_applicable_reason": Value::Null
                    }
                ],
                "conditional_reviewers": [
                    {
                        "role": "forge_reviewer",
                        "required": false,
                        "status": "not_applicable",
                        "workspace_policy": "read_only",
                        "source_mutation_allowed": false,
                        "evidence": [],
                        "not_applicable_reason": "ATO / Forge / FDA evidence, handoff, review packet, human decision boundary change was not detected."
                    },
                    {
                        "role": "design_qa",
                        "required": false,
                        "status": "not_applicable",
                        "workspace_policy": "read_only",
                        "source_mutation_allowed": false,
                        "evidence": [],
                        "not_applicable_reason": "UI / frontend / browser surface change was not detected."
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
                "next_action": if gate_status == "passed" { "fda merge" } else { "fda continue" }
            }),
        )
        .unwrap();
        write_text_file(
            &artifacts.join("review_agent_gate_packet.md"),
            &format!(
                "# FDA Review Agent Gate Packet\n\n\
## REVIEW_AGENT_GATE\n\n\
MERGE_APPROVAL: not_granted\n\n\
| role | status | evidence | rationale |\n\
|---|---|---|---|\n\
| pr_reviewer | {pr_packet_status} | `pr_reviewer_receipt.json` | PR review completed read-only. |\n\
| functional_qa | {functional_packet_status} | `functional_qa_receipt.json`; `ac_test_mapping.json` | Functional QA receipt. |\n\
| security_qa | REVIEW_AGENT_OK | `security_qa_receipt.json` | Security QA receipt. |\n\
| orchestrator | {pr_packet_status} | `review_agent_gate.json`; `qa_receipt.json` | Review Agent Gate aggregation. |\n\
| forge_reviewer | not_applicable | - | ATO / Forge / FDA evidence change was not detected. |\n\
| design_qa | not_applicable | - | UI / frontend / browser surface change was not detected. |\n"
            ),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "waiting_human",
                "decisions": [
                    {
                        "decision_id": "HD-FDA-MERGE-APPROVAL-001",
                        "summary": "merge approval required",
                        "required_before": "Merge Gate",
                        "recommended_option_id": "approve_merge",
                        "options": [
                            { "id": "approve_merge" },
                            { "id": "hold_for_repair" }
                        ]
                    }
                ]
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("decision_receipts.json"),
            &json!({
                "schema_version": "fda.decision_receipts.v0",
                "receipts": [
                    {
                        "decision_id": "HD-FDA-MERGE-APPROVAL-001",
                        "answer": "approve_merge"
                    }
                ]
            }),
        )
        .unwrap();
    }

    fn retarget_merge_artifacts_to_pr(artifacts: &Path, pr_number: &str) -> String {
        let actual_pr_url = format!("https://github.com/msamunetogetoge/example/pull/{pr_number}");
        let mut qa_receipt = read_json_value(&artifacts.join("qa_receipt.json")).unwrap();
        qa_receipt["actual_pr_url"] = json!(actual_pr_url);
        write_json_file(&artifacts.join("qa_receipt.json"), &qa_receipt).unwrap();

        let mut external_pr_receipt =
            read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        external_pr_receipt["actual_pr_url"] = json!(actual_pr_url);
        external_pr_receipt["target_pr"]["url"] = json!(actual_pr_url);
        external_pr_receipt["target_pr"]["number"] =
            json!(pr_number.parse::<u64>().unwrap_or_default());
        write_json_file(
            &artifacts.join("external_pr_receipt.json"),
            &external_pr_receipt,
        )
        .unwrap();

        let mut gate = read_json_value(&artifacts.join("review_agent_gate.json")).unwrap();
        gate["actual_pr_url"] = json!(actual_pr_url);
        write_json_file(&artifacts.join("review_agent_gate.json"), &gate).unwrap();
        actual_pr_url
    }

    fn write_promoted_forge_projection(artifacts: &Path) {
        write_json_file(
            &artifacts.join("forge_projection.json"),
            &json!({
                "schema_version": "forge_projection.v0",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "source_artifacts": [
                    "qa_receipt.json",
                    "external_pr_receipt.json",
                    "repair_receipt.json",
                    "risk_register.json"
                ],
                "claim_contracts": [
                    {
                        "claim_id": "CLAIM-FDA-MERGE-001",
                        "type": "operational",
                        "statement": "PR-V1-006 is ready for merge only when QA, CI, risk, and repair evidence are complete.",
                        "blocking": true,
                        "case_ids": ["CASE-FDA-V1-MERGE-001"],
                        "planned_pr_ids": ["PR-V1-006"],
                        "proof_obligations": ["PROOF-FDA-MERGE-001"]
                    }
                ],
                "proof_obligations": [
                    {
                        "proof_id": "PROOF-FDA-MERGE-001",
                        "claim_id": "CLAIM-FDA-MERGE-001",
                        "type": "merge_gate_evidence",
                        "required_evidence": [
                            "qa_receipt.json",
                            "external_pr_receipt.json",
                            "repair_receipt.json",
                            "risk_register.json"
                        ],
                        "blocking": true,
                        "owner_agent": "forge_delivery_agent",
                        "validation_method": "fda merge"
                    }
                ],
                "promotion_readiness": {
                    "verdict": "promote",
                    "reason": "All merge gate proof evidence is present.",
                    "evaluated_at": "2026-06-28T00:00:00Z",
                    "gate_inputs_ready": true
                },
                "gate_requirements": ["fda merge confirms Forge PromotionDecision before merge"]
            }),
        )
        .unwrap();
    }

    fn merge_config(artifacts: PathBuf, out: PathBuf, target: PathBuf) -> MergeConfig {
        MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        }
    }

    fn merge_execute_config(
        artifacts: PathBuf,
        out: PathBuf,
        target: PathBuf,
        command: Vec<String>,
    ) -> MergeConfig {
        let mut config = merge_config(artifacts, out, target);
        config.execute = true;
        config.merge_method = MergeMethod::Squash;
        config.github_merge_command = Some(command);
        config
    }

    #[cfg(unix)]
    fn github_merge_success_command(_base: &Path) -> Vec<String> {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\n' '{\"merge_sha\":\"merge-sha-123\",\"merged_at\":\"2026-06-28T00:00:00Z\",\"actor\":\"test-actor\",\"actual_pr_url\":\"https://github.com/msamunetogetoge/example/pull/123\"}'".to_string(),
        ]
    }

    #[cfg(unix)]
    fn github_merge_failure_command(_base: &Path) -> Vec<String> {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo merge denied by mock >&2; exit 2".to_string(),
        ]
    }

    #[cfg(unix)]
    fn github_merge_receipt_collection_failure_command(_base: &Path) -> Vec<String> {
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf '%s\n' '{\"status\":\"receipt_collection_failed\",\"actual_pr_url\":\"https://github.com/msamunetogetoge/example/pull/123\",\"failure_reason\":\"gh pr view returned incomplete JSON\",\"receipt_collection_command\":\"gh pr view https://github.com/msamunetogetoge/example/pull/123 --json mergeCommit,mergedAt,mergedBy,url\"}'".to_string(),
        ]
    }

    // Windows has no `sh`, so the mock GitHub merge commands are materialized
    // as .cmd scripts inside the test base directory instead.
    #[cfg(windows)]
    fn mock_merge_command_script(base: &Path, name: &str, body: &str) -> Vec<String> {
        fs::create_dir_all(base).unwrap();
        let path = base.join(name);
        fs::write(&path, body).unwrap();
        vec![path.display().to_string()]
    }

    #[cfg(windows)]
    fn github_merge_success_command(base: &Path) -> Vec<String> {
        mock_merge_command_script(
            base,
            "mock-merge-success.cmd",
            "@echo off\r\necho {\"merge_sha\":\"merge-sha-123\",\"merged_at\":\"2026-06-28T00:00:00Z\",\"actor\":\"test-actor\",\"actual_pr_url\":\"https://github.com/msamunetogetoge/example/pull/123\"}\r\n",
        )
    }

    #[cfg(windows)]
    fn github_merge_failure_command(base: &Path) -> Vec<String> {
        mock_merge_command_script(
            base,
            "mock-merge-failure.cmd",
            "@echo off\r\necho merge denied by mock 1>&2\r\nexit /b 2\r\n",
        )
    }

    #[cfg(windows)]
    fn github_merge_receipt_collection_failure_command(base: &Path) -> Vec<String> {
        mock_merge_command_script(
            base,
            "mock-merge-receipt-collection-failure.cmd",
            "@echo off\r\necho {\"status\":\"receipt_collection_failed\",\"actual_pr_url\":\"https://github.com/msamunetogetoge/example/pull/123\",\"failure_reason\":\"gh pr view returned incomplete JSON\",\"receipt_collection_command\":\"gh pr view https://github.com/msamunetogetoge/example/pull/123 --json mergeCommit,mergedAt,mergedBy,url\"}\r\n",
        )
    }

    #[test]
    fn review_writes_separate_functional_and_security_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-review-pass-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.functional_qa_status, "passed");
        assert_eq!(result.security_qa_status, "passed");
        for file_name in [
            "pr_reviewer_prompt.md",
            "functional_qa_prompt.md",
            "security_qa_prompt.md",
            "agent_role_policy.json",
            "mcp_agent_invocation_plan.json",
            "pr_reviewer_receipt.json",
            "functional_qa_receipt.json",
            "security_qa_receipt.json",
            "ac_test_mapping.json",
            "qa_receipt.json",
            "review_agent_gate.json",
            "review_agent_gate_packet.md",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        assert!(
            out.join("external_pr_receipt.json").exists(),
            "review should carry implementation PR receipt forward for merge"
        );

        let functional = read_json_value(&out.join("functional_qa_receipt.json")).unwrap();
        let security = read_json_value(&out.join("security_qa_receipt.json")).unwrap();
        assert_eq!(
            functional.get("role").and_then(Value::as_str),
            Some("functional_qa")
        );
        assert_eq!(
            security.get("role").and_then(Value::as_str),
            Some("security_qa")
        );
        assert_eq!(
            security
                .get("not_copied_from_functional_qa")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            functional
                .get("source_mutation_attempted")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            security
                .get("source_mutation_attempted")
                .and_then(Value::as_bool),
            Some(false)
        );
        let review_agent_gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        assert_eq!(
            review_agent_gate.get("status").and_then(Value::as_str),
            Some("passed")
        );
        assert_eq!(
            review_agent_gate
                .get("source_mutation_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            review_agent_gate
                .get("merge_approval_granted")
                .and_then(Value::as_bool),
            Some(false)
        );
        let required_reviewer_roles = review_agent_gate
            .get("required_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(|reviewer| reviewer.get("role").and_then(Value::as_str))
            .collect::<Vec<_>>();
        for role in ["pr_reviewer", "functional_qa", "security_qa"] {
            assert!(
                required_reviewer_roles.contains(&role),
                "{role} should be required"
            );
        }
        assert!(read_json_value(&out.join("ac_test_mapping.json"))
            .unwrap()
            .get("mappings")
            .and_then(Value::as_array)
            .is_some_and(|mappings| !mappings.is_empty()));
        assert_eq!(
            read_json_value(&out.join("validation_report.json"))
                .unwrap()
                .get("verdict")
                .and_then(Value::as_str),
            Some("pass")
        );
        let gate_check = ProcessCommand::new("python3")
            .arg("scripts/check_review_agent_gate.py")
            .arg("--packet-path")
            .arg(out.join("review_agent_gate_packet.md"))
            .output()
            .unwrap();
        assert!(
            gate_check.status.success(),
            "review agent gate packet should pass checker: stderr={}",
            String::from_utf8_lossy(&gate_check.stderr)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_blocks_when_pr_reviewer_receipt_is_missing() {
        let base = env::temp_dir().join(format!(
            "fda-review-missing-pr-reviewer-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        fs::remove_file(artifacts.join("pr_reviewer_receipt.json")).unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let pr_reviewer = read_json_value(&out.join("pr_reviewer_receipt.json")).unwrap();
        assert_eq!(
            pr_reviewer.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let pr_row = gate
            .get("required_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("pr_reviewer"))
            .unwrap();
        assert_eq!(
            pr_row.get("status").and_then(Value::as_str),
            Some("blocked")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_blocks_reviewer_receipt_without_read_only_policy() {
        let base = env::temp_dir().join(format!(
            "fda-review-reviewer-policy-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        let mut receipt = read_json_value(&artifacts.join("pr_reviewer_receipt.json")).unwrap();
        receipt["workspace_policy"] = json!("workspace_write");
        receipt["source_mutation_allowed"] = json!(true);
        write_json_file(&artifacts.join("pr_reviewer_receipt.json"), &receipt).unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let pr_reviewer = read_json_value(&out.join("pr_reviewer_receipt.json")).unwrap();
        assert_eq!(
            pr_reviewer.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        let findings = pr_reviewer
            .get("findings")
            .and_then(Value::as_array)
            .unwrap();
        assert!(findings.iter().any(|finding| finding
            .as_str()
            .is_some_and(|text| { text.contains("workspace_policy must be read_only") })));
        assert!(findings.iter().any(|finding| finding
            .as_str()
            .is_some_and(|text| text.contains("source_mutation_allowed=false"))));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_blocks_reviewer_receipt_from_another_pr() {
        let base = env::temp_dir().join(format!(
            "fda-review-reviewer-pr-identity-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        let mut receipt = read_json_value(&artifacts.join("pr_reviewer_receipt.json")).unwrap();
        receipt["actual_pr_url"] = json!("https://github.com/msamunetogetoge/example/pull/999");
        receipt["reviewed_planned_pr_id"] = json!("PR-V1-999");
        write_json_file(&artifacts.join("pr_reviewer_receipt.json"), &receipt).unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let pr_row = gate
            .get("required_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("pr_reviewer"))
            .unwrap();
        assert_eq!(
            pr_row.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        let pr_reviewer = read_json_value(&out.join("pr_reviewer_receipt.json")).unwrap();
        assert!(pr_reviewer
            .get("findings")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|finding| finding.as_str().is_some_and(|text| {
                text.contains("actual_pr_url must match current PR")
                    || text.contains("reviewed_planned_pr_id must be PR-V1-006")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_requires_forge_reviewer_when_fda_evidence_changes() {
        let base = env::temp_dir().join(format!(
            "fda-review-forge-required-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        for file_name in ["implementation_receipt.json", "external_pr_receipt.json"] {
            let mut receipt = read_json_value(&artifacts.join(file_name)).unwrap();
            receipt["changed_files"] = json!([".fda/gates.yaml"]);
            write_json_file(&artifacts.join(file_name), &receipt).unwrap();
        }

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let forge_row = gate
            .get("conditional_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("forge_reviewer"))
            .unwrap();
        assert_eq!(
            forge_row.get("required").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            forge_row.get("status").and_then(Value::as_str),
            Some("blocked")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_accepts_qax2_fallback_for_required_forge_review() {
        let base = env::temp_dir().join(format!(
            "fda-review-qax2-fallback-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["changed_files"] = json!([".fda/gates.yaml"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();
        write_json_file(
            &artifacts.join("qax2_receipt.json"),
            &json!({
                "schema_version": "fda.qax2_receipt.v0",
                "receipt_id": "QAX2-FDA-TEST-001",
                "planned_pr_id": "PR-V1-007",
                "reviewed_planned_pr_id": "PR-V1-006",
                "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
                "role": "qax2",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "status": "passed",
                "findings": [],
                "source_mutation_attempted": false
            }),
        )
        .unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let forge_row = gate
            .get("conditional_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("forge_reviewer"))
            .unwrap();
        assert_eq!(
            forge_row.get("status").and_then(Value::as_str),
            Some("passed")
        );
        assert!(forge_row
            .get("evidence")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|evidence| evidence.as_str() == Some("qax2_receipt.json")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_requires_forge_reviewer_when_run_artifact_evidence_changes() {
        let base = env::temp_dir().join(format!(
            "fda-review-run-artifact-forge-required-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["changed_files"] = json!(["artifacts/runs/run-1/validation_report.json"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let forge_row = gate
            .get("conditional_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("forge_reviewer"))
            .unwrap();
        assert_eq!(
            forge_row.get("required").and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_requires_design_qa_for_ui_surface_changes() {
        let base = env::temp_dir().join(format!(
            "fda-review-design-required-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["changed_files"] = json!(["src/components/Button.tsx"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let design_row = gate
            .get("conditional_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("design_qa"))
            .unwrap();
        assert_eq!(
            design_row.get("required").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            design_row.get("status").and_then(Value::as_str),
            Some("blocked")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_requires_design_qa_for_root_browser_surface_changes() {
        let base = env::temp_dir().join(format!(
            "fda-review-root-browser-design-required-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["changed_files"] = json!(["browser/routes.ts"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let gate = read_json_value(&out.join("review_agent_gate.json")).unwrap();
        let design_row = gate
            .get("conditional_reviewers")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|reviewer| reviewer.get("role").and_then(Value::as_str) == Some("design_qa"))
            .unwrap();
        assert_eq!(
            design_row.get("required").and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_does_not_carry_stale_repair_receipt_into_passing_review() {
        let base = env::temp_dir().join(format!(
            "fda-review-stale-repair-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&out).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);
        for dir in [&artifacts, &out] {
            write_json_file(
                &dir.join("repair_receipt.json"),
                &json!({
                    "schema_version": "fda.repair_receipt.v0",
                    "receipt_id": "REPAIR-FDA-STALE-001",
                    "status": "repair_planned",
                    "failure_classification": "functional_qa_failed",
                    "retry_attempt_count": 1,
                    "retry_limit": 3,
                    "retry_limit_reached": false
                }),
            )
            .unwrap();
        }

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert!(
            !out.join("repair_receipt.json").exists(),
            "passing review must not leave stale repair receipt in output"
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_fail_routes_to_implementer() {
        let base = env::temp_dir().join(format!(
            "fda-review-fail-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: Some(QaFixture {
                status: "failed".to_string(),
                findings: vec!["acceptance criterion is not covered".to_string()],
                severity: Some("medium".to_string()),
            }),
            security_qa_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "fail");
        assert_eq!(result.functional_qa_status, "failed");
        assert!(result
            .next_actions
            .iter()
            .any(|action| action == "fda continue"));
        assert_eq!(
            read_json_value(&out.join("functional_qa_receipt.json"))
                .unwrap()
                .get("return_to_role")
                .and_then(Value::as_str),
            Some("implementer")
        );
        assert_eq!(
            read_json_value(&out.join("qa_receipt.json"))
                .unwrap()
                .get("return_to_role")
                .and_then(Value::as_str),
            Some("implementer")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn review_security_high_blocks_for_human_decision() {
        let base = env::temp_dir().join(format!(
            "fda-review-security-high-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_review_inputs(&artifacts, &target);

        let result = review(&ReviewConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            ato: AtoConfig::default(),
            print_json: false,
            functional_qa_fixture: None,
            security_qa_fixture: Some(QaFixture {
                status: "failed".to_string(),
                findings: vec!["high severity secret exposure risk".to_string()],
                severity: Some("high".to_string()),
            }),
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.security_qa_status, "needs_human");
        assert_eq!(
            read_json_value(&out.join("qa_receipt.json"))
                .unwrap()
                .get("return_to_role")
                .and_then(Value::as_str),
            Some("human_security_approval")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn continue_plans_repair_from_qa_fail() {
        let base = env::temp_dir().join(format!(
            "fda-continue-repair-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_review_receipts(
            &artifacts,
            "failed",
            "passed",
            Some("implementer"),
            &["acceptance criterion is not covered"],
        );

        let result = continue_run(&ContinueConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            max_retries: 3,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.repair_loop_status, "repair_planned");
        assert_eq!(result.failure_classification, "functional_acceptance_gap");
        assert_eq!(result.retry_attempt_count, 1);
        for file_name in [
            "repair_prompt.md",
            "failure_classification.json",
            "retry_history.json",
            "repair_receipt.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        assert_eq!(
            read_json_value(&out.join("repair_receipt.json"))
                .unwrap()
                .get("return_to_role")
                .and_then(Value::as_str),
            Some("implementer")
        );
        assert!(read_json_value(&out.join("retry_history.json"))
            .unwrap()
            .get("attempts")
            .and_then(Value::as_array)
            .is_some_and(|attempts| attempts.len() == 1));
        assert_eq!(
            read_json_value(&out.join("validation_report.json"))
                .unwrap()
                .get("verdict")
                .and_then(Value::as_str),
            Some("pass")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn continue_hits_human_turn_after_retry_limit() {
        let base = env::temp_dir().join(format!(
            "fda-continue-limit-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_review_receipts(
            &artifacts,
            "failed",
            "passed",
            Some("implementer"),
            &["acceptance criterion is not covered"],
        );
        write_json_file(
            &artifacts.join("retry_history.json"),
            &json!({
                "schema_version": "fda.retry_history.v0",
                "attempts": [
                    { "attempt": 1, "failure_classification": "functional_acceptance_gap" },
                    { "attempt": 2, "failure_classification": "functional_acceptance_gap" },
                    { "attempt": 3, "failure_classification": "functional_acceptance_gap" }
                ]
            }),
        )
        .unwrap();

        let result = continue_run(&ContinueConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            max_retries: 3,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.repair_loop_status, "human_turn");
        assert_eq!(result.retry_attempt_count, 4);
        assert_eq!(
            read_json_value(&out.join("repair_receipt.json"))
                .unwrap()
                .get("retry_limit_reached")
                .and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn continue_passes_through_when_qa_passed() {
        let base = env::temp_dir().join(format!(
            "fda-continue-pass-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_review_receipts(&artifacts, "passed", "passed", None, &[]);

        let result = continue_run(&ContinueConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            max_retries: 3,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.repair_loop_status, "no_repair_needed");
        assert_eq!(result.failure_classification, "none");
        assert_eq!(result.retry_attempt_count, 0);
        assert_eq!(
            read_json_value(&out.join("retry_history.json"))
                .unwrap()
                .get("current_attempt")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert!(read_json_value(&out.join("retry_history.json"))
            .unwrap()
            .get("attempts")
            .and_then(Value::as_array)
            .is_some_and(|attempts| attempts.is_empty()));
        assert!(result
            .next_actions
            .iter()
            .any(|action| action == "fda merge"));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn continue_blocks_missing_per_role_qa_receipt() {
        let base = env::temp_dir().join(format!(
            "fda-continue-missing-role-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_review_receipts(
            &artifacts,
            "failed",
            "passed",
            Some("implementer"),
            &["acceptance criterion is not covered"],
        );
        fs::remove_file(artifacts.join("functional_qa_receipt.json")).unwrap();

        let result = continue_run(&ContinueConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            max_retries: 3,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.repair_loop_status, "blocked");
        assert_eq!(
            result.failure_classification,
            "missing_or_blocked_qa_evidence"
        );
        assert!(fs::read_to_string(out.join("repair_prompt.md"))
            .unwrap()
            .contains("planned PR: `PR-V1-006`"));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn continue_blocks_stale_aggregate_qa_pass_when_role_failed() {
        let base = env::temp_dir().join(format!(
            "fda-continue-stale-qa-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_review_receipts(&artifacts, "passed", "passed", None, &[]);
        write_json_file(
            &artifacts.join("functional_qa_receipt.json"),
            &json!({
                "schema_version": "fda.functional_qa_receipt.v0",
                "role": "functional_qa",
                "status": "failed",
                "findings": ["stale aggregate QA hid this failure"],
                "source_mutation_attempted": false
            }),
        )
        .unwrap();

        let result = continue_run(&ContinueConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            max_retries: 3,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.repair_loop_status, "blocked");
        assert_eq!(
            result.failure_classification,
            "missing_or_blocked_qa_evidence"
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_marks_low_risk_pr_ready() {
        let base = env::temp_dir().join(format!(
            "fda-merge-ready-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.merge_gate_status, "merge_ready");
        assert_eq!(result.policy_disposition, "human_approval_granted");
        assert_eq!(result.ci_status, "passed");
        assert_eq!(result.forge_status, "promote");
        assert_eq!(result.forge_promotion_decision, "promote");
        assert!(!result.merge_execute_requested);
        assert!(!result.merge_executed);
        assert_eq!(result.merge_execution_status, "not_requested");
        assert!(!out.join("github_merge_receipt.json").exists());
        for file_name in [
            "merge_gate_summary.json",
            "merge_policy_decision.json",
            "forge_promotion_receipt.json",
            "merge_approval_packet.json",
            "merge_receipt.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        assert_eq!(
            read_json_value(&out.join("merge_policy_decision.json"))
                .unwrap()
                .get("auto_merge_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        assert_eq!(
            forge_receipt.get("status").and_then(Value::as_str),
            Some("promote")
        );
        assert_eq!(
            forge_receipt.get("merge_allowed").and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_requires_human_approval_when_auto_merge_disabled() {
        let base = env::temp_dir().join(format!(
            "fda-merge-no-auto-human-required-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        fs::remove_file(artifacts.join("human_decision_packet.json")).unwrap();
        fs::remove_file(artifacts.join("decision_receipts.json")).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "human_approval_required");
        assert_eq!(result.policy_disposition, "human_approval_required");
        let decision = read_json_value(&out.join("merge_policy_decision.json")).unwrap();
        assert_eq!(
            decision.get("auto_merge_allowed").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            decision
                .get("human_approval_required")
                .and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_does_not_treat_non_explicit_merge_policy_answer_as_approval() {
        let base = env::temp_dir().join(format!(
            "fda-merge-explicit-approval-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut packet = read_json_value(&artifacts.join("human_decision_packet.json")).unwrap();
        packet["decisions"][0]["recommended_option_id"] = json!("manual_review_required");
        packet["decisions"][0]["options"] =
            json!([{ "id": "manual_review_required" }, { "id": "hold_for_repair" }]);
        write_json_file(&artifacts.join("human_decision_packet.json"), &packet).unwrap();
        let mut receipts = read_json_value(&artifacts.join("decision_receipts.json")).unwrap();
        receipts["receipts"][0]["answer"] = json!("manual_review_required");
        write_json_file(&artifacts.join("decision_receipts.json"), &receipts).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.policy_disposition, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("non-approval Human Decisions block merge")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_does_not_treat_merge_approval_reason_token_as_approval() {
        let base = env::temp_dir().join(format!(
            "fda-merge-reason-token-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipts = read_json_value(&artifacts.join("decision_receipts.json")).unwrap();
        receipts["receipts"][0]["answer"] = json!("merge_approval");
        write_json_file(&artifacts.join("decision_receipts.json"), &receipts).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.policy_disposition, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("non-approval Human Decisions block merge")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_missing_review_agent_gate() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-review-agent-gate-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        fs::remove_file(artifacts.join("review_agent_gate.json")).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        let issues = summary
            .get("issues")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(issues.iter().any(|issue| {
            issue
                .as_str()
                .is_some_and(|text| text.contains("review_agent_gate.json is required"))
        }));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_non_open_target_pr_state() {
        let base = env::temp_dir().join(format!(
            "fda-merge-target-pr-state-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["target_pr"]["state"] = json!("draft");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("target_pr.state must be open before merge, got draft")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_unreflected_review_agent_gate_packet() {
        let base = env::temp_dir().join(format!(
            "fda-merge-unreflected-review-agent-gate-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        fs::write(
            artifacts.join("review_agent_gate_packet.md"),
            "# Review Agent Gate\n\n## REVIEW_AGENT_GATE\n\nMERGE_APPROVAL: not_granted\n",
        )
        .unwrap();

        let actual_pr_url = "https://github.com/msamunetogetoge/example/pull/999999999";
        let mut qa_receipt = read_json_value(&artifacts.join("qa_receipt.json")).unwrap();
        qa_receipt["actual_pr_url"] = json!(actual_pr_url);
        write_json_file(&artifacts.join("qa_receipt.json"), &qa_receipt).unwrap();
        let mut external_pr_receipt =
            read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        external_pr_receipt["actual_pr_url"] = json!(actual_pr_url);
        external_pr_receipt["target_pr"]["url"] = json!(actual_pr_url);
        external_pr_receipt["target_pr"]["number"] = json!(999999999);
        write_json_file(
            &artifacts.join("external_pr_receipt.json"),
            &external_pr_receipt,
        )
        .unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert!(result.next_actions.iter().any(|action| action.contains(
            "review_agent_gate_packet.md を artifacts/review_packets/pr-<PR番号>.md に明示反映"
        )));
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        let issues = summary
            .get("issues")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(issues.iter().any(|issue| {
            issue.as_str().is_some_and(|text| {
                text.contains(
                    "review_agent_gate_packet.md must be reflected to artifacts/review_packets/pr-999999999.md before merge",
                )
            })
        }));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_stale_reflected_review_agent_gate_packet() {
        let base = env::temp_dir().join(format!(
            "fda-merge-stale-review-agent-gate-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        write_text_file(
            &artifacts.join("review_agent_gate_packet.md"),
            "# FDA Review Agent Gate Packet\n\n## REVIEW_AGENT_GATE\n\nMERGE_APPROVAL: not_granted\n\n| role | status | evidence | rationale |\n|---|---|---|---|\n| pr_reviewer | REVIEW_AGENT_OK | `pr_reviewer_receipt.json` | New local packet not yet reflected. |\n| functional_qa | REVIEW_AGENT_OK | `functional_qa_receipt.json`; `ac_test_mapping.json` | Functional QA receipt. |\n| security_qa | REVIEW_AGENT_OK | `security_qa_receipt.json` | Security QA receipt. |\n| orchestrator | REVIEW_AGENT_OK | `review_agent_gate.json`; `qa_receipt.json` | Review Agent Gate aggregation. |\n| forge_reviewer | not_applicable | - | ATO / Forge / FDA evidence change was not detected. |\n| design_qa | not_applicable | - | UI / frontend / browser surface change was not detected. |\n",
        )
        .unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue
                .as_str()
                .is_some_and(|text| text.contains("is stale or differs"))));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_accepts_reflected_review_packet_with_extra_sections_when_gate_section_matches() {
        let base = env::temp_dir().join(format!(
            "fda-merge-section-reflection-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let pr_number = format!("98{}{}", std::process::id(), now_unix_seconds());
        retarget_merge_artifacts_to_pr(&artifacts, &pr_number);
        let reflected_packet_path =
            Path::new("artifacts/review_packets").join(format!("pr-{pr_number}.md"));
        let current_packet =
            fs::read_to_string(artifacts.join("review_agent_gate_packet.md")).unwrap();
        write_text_file(
            &reflected_packet_path,
            &format!(
                "# Review Packet fixture\n\n## 対象\n\nextra context\n\n{current_packet}\n\n## Human Decision\n\nMERGE_APPROVAL: not_granted\n"
            ),
        )
        .unwrap();

        let result = merge_run(&merge_config(artifacts, out, target));

        fs::remove_file(reflected_packet_path).unwrap();
        let result = result.unwrap();
        assert_eq!(result.verdict, "pass");
        assert_eq!(result.merge_gate_status, "merge_ready");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_review_agent_gate_from_another_pr() {
        let base = env::temp_dir().join(format!(
            "fda-merge-gate-pr-identity-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut gate = read_json_value(&artifacts.join("review_agent_gate.json")).unwrap();
        gate["actual_pr_url"] = json!("https://github.com/msamunetogetoge/example/pull/999");
        write_json_file(&artifacts.join("review_agent_gate.json"), &gate).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("review_agent_gate.json actual_pr_url")
                    && text.contains("does not match current PR")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_enforces_read_only_review_agent_gate_reviewers() {
        let base = env::temp_dir().join(format!(
            "fda-merge-reviewer-read-only-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut gate = read_json_value(&artifacts.join("review_agent_gate.json")).unwrap();
        gate["required_reviewers"][0]["workspace_policy"] = json!("workspace_write");
        write_json_file(&artifacts.join("review_agent_gate.json"), &gate).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue
                .as_str()
                .is_some_and(|text| { text.contains("workspace_policy must be read_only") })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn merge_blocks_review_agent_gate_evidence_symlink_escape() {
        use std::os::unix::fs::symlink;

        let base = env::temp_dir().join(format!(
            "fda-merge-reviewer-evidence-symlink-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        let outside = base.join("outside-reviewer-evidence.json");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(&outside, "{\"status\":\"passed\"}").unwrap();
        symlink(&outside, artifacts.join("linked_reviewer_evidence.json")).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut gate = read_json_value(&artifacts.join("review_agent_gate.json")).unwrap();
        gate["required_reviewers"][0]["evidence"] = json!(["linked_reviewer_evidence.json"]);
        write_json_file(&artifacts.join("review_agent_gate.json"), &gate).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("linked_reviewer_evidence.json must resolve inside artifact_dir")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_recomputes_conditional_forge_reviewer_from_changed_files() {
        let base = env::temp_dir().join(format!(
            "fda-merge-conditional-forge-review-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut external_pr = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        external_pr["changed_files"] = json!([".fda/gates.yaml"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &external_pr).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("conditional reviewer forge_reviewer must be marked required")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_low_risk_tier_relaxes_conditional_reviewers() {
        let base = env::temp_dir().join(format!(
            "fda-merge-low-tier-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        // forge_reviewer を要求する変更 (.fda/gates.yaml)。tier 無しなら blocked になる。
        let mut external_pr = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        external_pr["changed_files"] = json!([".fda/gates.yaml"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &external_pr).unwrap();
        write_json_file(
            &artifacts.join("risk_tier.json"),
            &json!({
                "schema_version": "fda.risk_tier.v1",
                "tier": "low",
                "reasons": ["全 scope パスが delivery_policy.low_risk_paths に一致します"],
                "matched_low_risk_paths": [".fda/gates.yaml"],
                "policy_source": ".fda/delivery_policy.yaml"
            }),
        )
        .unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.merge_gate_status, "merge_ready");
        assert_eq!(result.risk_tier.as_deref(), Some("low"));
        assert!(result
            .proportional_gate_notes
            .iter()
            .any(|note| note.contains("forge_reviewer") && note.contains("not_applicable")));
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert_eq!(
            summary.get("risk_tier").and_then(Value::as_str),
            Some("low")
        );
        assert!(!summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue
                .as_str()
                .is_some_and(|text| text.contains("forge_reviewer must be marked required"))));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_without_risk_tier_keeps_conditional_reviewer_enforcement() {
        let base = env::temp_dir().join(format!(
            "fda-merge-no-tier-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut external_pr = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        external_pr["changed_files"] = json!([".fda/gates.yaml"]);
        write_json_file(&artifacts.join("external_pr_receipt.json"), &external_pr).unwrap();
        // risk_tier.json は置かない -> 現行動作 (standard 扱い) を維持する。

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.risk_tier, None);
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("conditional reviewer forge_reviewer must be marked required")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_missing_review_agent_gate_reviewer_evidence() {
        let base = env::temp_dir().join(format!(
            "fda-merge-review-agent-evidence-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut gate = read_json_value(&artifacts.join("review_agent_gate.json")).unwrap();
        gate["required_reviewers"][0]["evidence"] = json!(["missing_pr_review.json"]);
        write_json_file(&artifacts.join("review_agent_gate.json"), &gate).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        let summary = read_json_value(&out.join("merge_gate_summary.json")).unwrap();
        assert!(summary
            .get("issues")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().is_some_and(|text| {
                text.contains("evidence missing_pr_review.json does not exist")
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_reads_auto_merge_policy_from_target_repo() {
        let base = env::temp_dir().join(format!(
            "fda-merge-target-policy-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        fs::remove_file(artifacts.join("human_decision_packet.json")).unwrap();
        fs::remove_file(artifacts.join("decision_receipts.json")).unwrap();

        let store = crate::infra::fs_store::FsArtifactStore;
        crate::application::profile::ensure_repository_profile(&store, &target).unwrap();
        fs::write(
            target.join(".fda/delivery_policy.yaml"),
            "delivery_policy:\n  default_autonomy_level: merge_allowed\n  auto_merge_allowed: true\n  human_required_for:\n    - merge_approval\n  forbidden_without_human:\n    - release_approval\n",
        )
        .unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.merge_gate_status, "merge_ready");
        let decision = read_json_value(&out.join("merge_policy_decision.json")).unwrap();
        assert_eq!(
            decision.get("auto_merge_allowed").and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_missing_forge_projection() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-forge-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        fs::remove_file(artifacts.join("forge_projection.json")).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.policy_disposition, "blocked");
        assert_eq!(result.forge_status, "adapter_unavailable");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        assert_eq!(
            forge_receipt.get("status").and_then(Value::as_str),
            Some("adapter_unavailable")
        );
        assert!(forge_receipt
            .get("blocking_issues")
            .and_then(Value::as_array)
            .is_some_and(|issues| issues.iter().any(|issue| issue
                .as_str()
                .is_some_and(|issue| issue.contains("forge_projection.json is required")))));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_hold_forge_promotion() {
        let base = env::temp_dir().join(format!(
            "fda-merge-hold-forge-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut forge = read_json_value(&artifacts.join("forge_projection.json")).unwrap();
        forge["promotion_readiness"]["verdict"] = json!("hold");
        forge["promotion_readiness"]["reason"] = json!("Manual Forge hold for test.");
        write_json_file(&artifacts.join("forge_projection.json"), &forge).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.forge_status, "hold");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        assert_eq!(
            forge_receipt
                .get("promotion_decision")
                .and_then(Value::as_str),
            Some("hold")
        );
        assert_eq!(
            forge_receipt.get("merge_allowed").and_then(Value::as_bool),
            Some(false)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_missing_forge_proof_evidence() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-forge-proof-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut forge = read_json_value(&artifacts.join("forge_projection.json")).unwrap();
        forge["proof_obligations"][0]["required_evidence"]
            .as_array_mut()
            .unwrap()
            .push(json!("missing_proof.json"));
        write_json_file(&artifacts.join("forge_projection.json"), &forge).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.forge_status, "hold");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        assert!(forge_receipt
            .get("blocking_issues")
            .and_then(Value::as_array)
            .is_some_and(|issues| issues.iter().any(|issue| {
                issue.as_str().is_some_and(|issue| {
                    issue.contains("missing proof evidence missing_proof.json")
                })
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_invalid_forge_projection_schema() {
        let base = env::temp_dir().join(format!(
            "fda-merge-invalid-forge-schema-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut forge = read_json_value(&artifacts.join("forge_projection.json")).unwrap();
        forge.as_object_mut().unwrap().remove("schema_version");
        write_json_file(&artifacts.join("forge_projection.json"), &forge).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.forge_status, "hold");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        assert!(forge_receipt
            .get("blocking_issues")
            .and_then(Value::as_array)
            .is_some_and(|issues| issues.iter().any(|issue| {
                issue.as_str().is_some_and(|issue| {
                    issue.contains("forge_projection.json schema validation failed")
                        && issue.contains("schema_version")
                })
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_forge_proof_evidence_outside_artifact_dir() {
        let base = env::temp_dir().join(format!(
            "fda-merge-forge-proof-outside-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut forge = read_json_value(&artifacts.join("forge_projection.json")).unwrap();
        forge["proof_obligations"][0]["required_evidence"]
            .as_array_mut()
            .unwrap()
            .push(json!("/etc/passwd"));
        write_json_file(&artifacts.join("forge_projection.json"), &forge).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.forge_status, "hold");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        assert!(forge_receipt
            .get("blocking_issues")
            .and_then(Value::as_array)
            .is_some_and(|issues| issues.iter().any(|issue| {
                issue.as_str().is_some_and(|issue| {
                    issue.contains(
                        "required_evidence /etc/passwd must be a relative path inside artifact_dir",
                    )
                })
            })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_forge_projection_from_other_program_or_epic() {
        let base = env::temp_dir().join(format!(
            "fda-merge-forge-context-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut forge = read_json_value(&artifacts.join("forge_projection.json")).unwrap();
        forge["program_id"] = json!("OTHER-PROGRAM");
        forge["epic_id"] = json!("OTHER-EPIC");
        write_json_file(&artifacts.join("forge_projection.json"), &forge).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.forge_status, "hold");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        let issues = forge_receipt
            .get("blocking_issues")
            .and_then(Value::as_array)
            .unwrap();
        assert!(issues.iter().any(|issue| issue
            .as_str()
            .is_some_and(|issue| issue.contains("program_id OTHER-PROGRAM"))));
        assert!(issues.iter().any(|issue| issue
            .as_str()
            .is_some_and(|issue| issue.contains("epic_id OTHER-EPIC"))));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_mismatched_qa_external_pr_context() {
        let base = env::temp_dir().join(format!(
            "fda-merge-qa-external-context-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut qa = read_json_value(&artifacts.join("qa_receipt.json")).unwrap();
        qa["program_id"] = json!("OTHER-PROGRAM");
        qa["epic_id"] = json!("OTHER-EPIC");
        write_json_file(&artifacts.join("qa_receipt.json"), &qa).unwrap();

        let result = merge_run(&merge_config(artifacts, out.clone(), target)).unwrap();

        assert_eq!(result.verdict, "blocked");
        let forge_receipt = read_json_value(&out.join("forge_promotion_receipt.json")).unwrap();
        let issues = forge_receipt
            .get("blocking_issues")
            .and_then(Value::as_array)
            .unwrap();
        assert!(issues.iter().any(|issue| issue.as_str().is_some_and(|issue| {
            issue.contains("qa_receipt.json program_id OTHER-PROGRAM does not match external_pr_receipt.json program_id FDA-V1")
        })));
        assert!(issues.iter().any(|issue| issue.as_str().is_some_and(|issue| {
            issue.contains("qa_receipt.json epic_id OTHER-EPIC does not match external_pr_receipt.json epic_id EPIC-FDA-V1-MCP")
        })));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_execute_writes_success_receipt_from_mock_command() {
        let base = env::temp_dir().join(format!(
            "fda-merge-execute-success-receipt-extra-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        init_merge_target_repo(&target, "https://github.com/msamunetogetoge/example.git");
        write_merge_inputs(&artifacts, "passed", "delivery");

        let result = merge_run(&merge_execute_config(
            artifacts,
            out.clone(),
            target,
            github_merge_success_command(&base),
        ))
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert!(result.merge_execute_requested);
        assert!(result.merge_executed);
        assert_eq!(result.merge_execution_status, "succeeded");
        assert_eq!(result.merge_method, "squash");
        assert!(result.github_merge_receipt_path.is_some());
        assert!(result.merge_failure_reason.is_none());

        let github_receipt = read_json_value(&out.join("github_merge_receipt.json")).unwrap();
        assert_eq!(
            github_receipt.get("status").and_then(Value::as_str),
            Some("succeeded")
        );
        assert_eq!(
            github_receipt.get("merge_sha").and_then(Value::as_str),
            Some("merge-sha-123")
        );
        assert_eq!(
            github_receipt.get("merge_method").and_then(Value::as_str),
            Some("squash")
        );
        assert_eq!(
            github_receipt.get("actor").and_then(Value::as_str),
            Some("test-actor")
        );

        let merge_receipt = read_json_value(&out.join("merge_receipt.json")).unwrap();
        assert_eq!(
            merge_receipt
                .get("merge_execution_status")
                .and_then(Value::as_str),
            Some("succeeded")
        );
        assert_eq!(
            merge_receipt.get("merge_executed").and_then(Value::as_bool),
            Some(true)
        );
        assert!(merge_receipt
            .get("github_merge_receipt_path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.ends_with("github_merge_receipt.json")));

        let inventory = read_json_value(&out.join("artifact_inventory.json")).unwrap();
        assert!(inventory
            .get("artifacts")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|artifact| artifact
                .get("path_or_url")
                .and_then(Value::as_str)
                .is_some_and(|path| path.ends_with("github_merge_receipt.json"))));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_execute_writes_failure_receipt_from_mock_command() {
        let base = env::temp_dir().join(format!(
            "fda-merge-execute-failure-receipt-extra-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        init_merge_target_repo(&target, "https://github.com/msamunetogetoge/example.git");
        write_merge_inputs(&artifacts, "passed", "delivery");

        let result = merge_run(&merge_execute_config(
            artifacts,
            out.clone(),
            target,
            github_merge_failure_command(&base),
        ))
        .unwrap();

        assert_eq!(result.verdict, "fail");
        assert_eq!(result.merge_gate_status, "merge_ready");
        assert!(!result.merge_executed);
        assert_eq!(result.merge_execution_status, "failed");
        assert!(result
            .merge_failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("merge denied by mock")));
        assert!(result
            .next_actions
            .iter()
            .any(|action| action.contains("fda merge") && action.contains("--execute")));

        let github_receipt = read_json_value(&out.join("github_merge_receipt.json")).unwrap();
        assert_eq!(
            github_receipt.get("status").and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            github_receipt
                .get("merge_executed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(github_receipt
            .get("failure_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("merge denied by mock")));
        assert!(github_receipt
            .get("resume_command")
            .and_then(Value::as_str)
            .is_some_and(|command| command.contains("fda merge") && command.contains("--execute")));

        let merge_receipt = read_json_value(&out.join("merge_receipt.json")).unwrap();
        assert_eq!(
            merge_receipt
                .get("merge_execution_status")
                .and_then(Value::as_str),
            Some("failed")
        );
        assert!(merge_receipt
            .get("merge_execution_failure_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("merge denied by mock")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_execute_records_receipt_collection_failure_as_executed() {
        let base = env::temp_dir().join(format!(
            "fda-merge-execute-receipt-collection-failure-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        init_merge_target_repo(&target, "https://github.com/msamunetogetoge/example.git");
        write_merge_inputs(&artifacts, "passed", "delivery");

        let result = merge_run(&merge_execute_config(
            artifacts,
            out.clone(),
            target,
            github_merge_receipt_collection_failure_command(&base),
        ))
        .unwrap();

        assert_eq!(result.verdict, "fail");
        assert_eq!(result.merge_execution_status, "receipt_collection_failed");
        assert!(result.merge_executed);
        assert!(result
            .merge_failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("incomplete JSON")));
        assert!(result
            .next_actions
            .iter()
            .any(|action| action.contains("gh pr view")));
        assert!(!result
            .next_actions
            .iter()
            .any(|action| action.contains("fda merge") && action.contains("--execute")));

        let github_receipt = read_json_value(&out.join("github_merge_receipt.json")).unwrap();
        assert_eq!(
            github_receipt.get("status").and_then(Value::as_str),
            Some("receipt_collection_failed")
        );
        assert_eq!(
            github_receipt
                .get("merge_executed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            github_receipt
                .get("expected_head_sha")
                .and_then(Value::as_str),
            Some("abc123")
        );
        assert!(github_receipt
            .get("receipt_collection_command")
            .and_then(Value::as_str)
            .is_some_and(|command| command.contains("gh pr view")));

        let merge_receipt = read_json_value(&out.join("merge_receipt.json")).unwrap();
        assert_eq!(
            merge_receipt
                .get("merge_execution_status")
                .and_then(Value::as_str),
            Some("receipt_collection_failed")
        );
        assert_eq!(
            merge_receipt.get("merge_executed").and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_execute_fails_precondition_before_mock_command() {
        let base = env::temp_dir().join(format!(
            "fda-merge-execute-precondition-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "failed", "delivery");

        let result = merge_run(&merge_execute_config(
            artifacts,
            out.clone(),
            target,
            github_merge_failure_command(&base),
        ))
        .unwrap();

        assert_eq!(result.verdict, "fail");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.merge_execution_status, "failed");
        assert!(result
            .merge_failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("precondition failed")));
        assert!(!result
            .merge_failure_reason
            .as_deref()
            .unwrap()
            .contains("merge denied by mock"));

        let github_receipt = read_json_value(&out.join("github_merge_receipt.json")).unwrap();
        assert!(github_receipt
            .get("failure_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("gate.status must be merge_ready")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_execute_blocks_when_target_repo_origin_does_not_match_pr_url() {
        let base = env::temp_dir().join(format!(
            "fda-merge-execute-target-repo-mismatch-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        init_merge_target_repo(&target, "https://github.com/msamunetogetoge/different.git");
        write_merge_inputs(&artifacts, "passed", "delivery");

        let result = merge_run(&merge_execute_config(
            artifacts,
            out,
            target,
            github_merge_success_command(&base),
        ))
        .unwrap();

        assert_eq!(result.verdict, "fail");
        assert_eq!(result.merge_execution_status, "failed");
        assert!(!result.merge_executed);
        assert!(result
            .merge_failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("target repo origin")
                && reason.contains("does not match PR URL repository")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_execute_blocks_when_head_sha_is_missing() {
        let base = env::temp_dir().join(format!(
            "fda-merge-execute-missing-head-sha-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        init_merge_target_repo(&target, "https://github.com/msamunetogetoge/example.git");
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt
            .get_mut("target_pr")
            .and_then(Value::as_object_mut)
            .unwrap()
            .remove("head_sha");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&merge_execute_config(
            artifacts,
            out,
            target,
            github_merge_success_command(&base),
        ))
        .unwrap();

        assert_eq!(result.verdict, "fail");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.merge_execution_status, "failed");
        assert!(!result.merge_executed);
        assert!(result
            .merge_failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("target_pr.head_sha")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_requires_human_for_security_risk() {
        let base = env::temp_dir().join(format!(
            "fda-merge-human-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "security");
        fs::remove_file(artifacts.join("human_decision_packet.json")).unwrap();
        fs::remove_file(artifacts.join("decision_receipts.json")).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "human_approval_required");
        assert_eq!(result.policy_disposition, "human_approval_required");
        assert_eq!(result.risk_classification, "regulated_risk");
        assert_eq!(
            read_json_value(&out.join("merge_approval_packet.json"))
                .unwrap()
                .get("required")
                .and_then(Value::as_bool),
            Some(true)
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_when_qa_is_not_passed() {
        let base = env::temp_dir().join(format!(
            "fda-merge-blocked-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "failed", "delivery");

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");
        assert_eq!(result.policy_disposition, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_non_open_external_pr_receipt_status() {
        let base = env::temp_dir().join(format!(
            "fda-merge-external-status-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["status"] = json!("blocked");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_mismatched_qa_and_external_pr_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-merge-mismatch-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["planned_pr_id"] = json!("PR-V1-DIFFERENT");
        receipt["actual_pr_url"] = json!("https://github.com/msamunetogoge/example/pull/456");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_requires_all_ci_checks_to_pass() {
        let base = env::temp_dir().join(format!(
            "fda-merge-ci-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["checks"]["security"] = json!("not_run");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.ci_status, "missing");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_requires_tests_check_in_external_pr_receipt() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-tests-check-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["checks"] = json!({ "lint": "passed", "security": "passed" });
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.ci_status, "missing");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_requires_external_pr_identity_fields() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-external-identity-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        let receipt_object = receipt.as_object_mut().unwrap();
        receipt_object.remove("planned_pr_id");
        receipt_object.remove("actual_pr_url");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_requires_scope_disposition_evidence() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-scope-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt.as_object_mut().unwrap().remove("scope_disposition");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_scope_deviation_and_open_external_issues() {
        let base = env::temp_dir().join(format!(
            "fda-merge-scope-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        let mut receipt = read_json_value(&artifacts.join("external_pr_receipt.json")).unwrap();
        receipt["open_issues"] = json!(["scope review required"]);
        receipt["scope_disposition"]["kind"] = json!("scope_deviation");
        write_json_file(&artifacts.join("external_pr_receipt.json"), &receipt).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_held_human_decision_receipt() {
        let base = env::temp_dir().join(format!(
            "fda-merge-held-decision-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "waiting_human",
                "decisions": [
                    {
                        "decision_id": "HD-FDA-MERGE-001",
                        "summary": "merge approval required",
                        "required_before": "Merge Gate",
                        "recommended_option_id": "approve",
                        "options": [{ "id": "approve" }, { "id": "hold" }]
                    }
                ]
            }),
        )
        .unwrap();
        write_json_file(
            &artifacts.join("decision_receipts.json"),
            &json!({
                "schema_version": "fda.decision_receipts.v0",
                "receipts": [
                    {
                        "decision_id": "HD-FDA-MERGE-001",
                        "answer": "hold"
                    }
                ]
            }),
        )
        .unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_open_human_decisions() {
        let base = env::temp_dir().join(format!(
            "fda-merge-human-decision-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "waiting_human",
                "decisions": [
                    {
                        "decision_id": "HD-FDA-MERGE-001",
                        "summary": "merge approval required",
                        "required_before": "Merge Gate",
                        "recommended_option_id": "approve",
                        "options": [{ "id": "approve" }, { "id": "hold" }]
                    }
                ]
            }),
        )
        .unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "blocked");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_blocks_missing_risk_register() {
        let base = env::temp_dir().join(format!(
            "fda-merge-missing-risk-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "delivery");
        fs::remove_file(artifacts.join("risk_register.json")).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.risk_classification, "missing_risk_evidence");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn merge_reads_markdown_and_hyphenated_high_risk_registers() {
        let base = env::temp_dir().join(format!(
            "fda-merge-md-risk-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_merge_inputs(&artifacts, "passed", "high-risk");
        fs::remove_file(artifacts.join("human_decision_packet.json")).unwrap();
        fs::remove_file(artifacts.join("decision_receipts.json")).unwrap();
        fs::remove_file(artifacts.join("risk_register.json")).unwrap();
        write_text_file(
            &artifacts.join("risk_register.md"),
            "- category: high-risk\n- summary: legal review required\n",
        )
        .unwrap();
        let mut forge = read_json_value(&artifacts.join("forge_projection.json")).unwrap();
        forge["proof_obligations"][0]["required_evidence"] = json!([
            "qa_receipt.json",
            "external_pr_receipt.json",
            "repair_receipt.json",
            "risk_register.md"
        ]);
        write_json_file(&artifacts.join("forge_projection.json"), &forge).unwrap();

        let result = merge_run(&MergeConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            execute: false,
            merge_method: crate::cli::args::MergeMethod::Merge,
            github_merge_command: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.merge_gate_status, "human_approval_required");
        assert_eq!(result.risk_classification, "high_risk");

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_writes_receipts_from_codex_fixture() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_live_dry_run_receipt(&artifacts, &target);

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts.clone(),
            out: Some(out.clone()),
            target_repo: target,
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: Some(CodexLiveFixture {
                thread_id: Some("thread-live-001".to_string()),
                status: CodexLiveStatus::Succeeded,
                content: [
                    "Implementation completed.",
                    "FDA_ACTUAL_PR_URL: https://github.com/msamunetogetoge/example/pull/123",
                    "FDA_TEST_STATUS: passed",
                    "FDA_TESTS_RUN: cargo test",
                    "FDA_CHANGED_FILES: src/lib.rs, tests/live.rs",
                    "FDA_SCOPE_DRIFT: none",
                ]
                .join("\n"),
            }),
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.dry_run_gate_status, "succeeded");
        assert_eq!(result.development_gate_status.as_deref(), Some("succeeded"));
        assert_eq!(
            result.actual_pr_url.as_deref(),
            Some("https://github.com/msamunetogetoge/example/pull/123")
        );
        for file_name in [
            "implementation_handoff.md",
            "codex_live_prompt.md",
            "agent_role_policy.json",
            "planned_pr_execution_packet.json",
            "mcp_agent_invocation_plan.json",
            "mcp_tool_call_receipt.json",
            "implementation_receipt.json",
            "external_pr_receipt.json",
            "coding_agent_thread_state.json",
            "live_execution_evidence.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        assert_eq!(
            read_json_value(&out.join("implementation_receipt.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("succeeded")
        );
        assert_eq!(
            read_json_value(&out.join("external_pr_receipt.json"))
                .unwrap()
                .get("actual_pr_url")
                .and_then(Value::as_str),
            Some("https://github.com/msamunetogetoge/example/pull/123")
        );
        let live_evidence = read_json_value(&out.join("live_execution_evidence.json")).unwrap();
        assert_eq!(
            live_evidence.get("status").and_then(Value::as_str),
            Some("fixture_mode")
        );
        assert_eq!(
            live_evidence.get("fixture_used").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            live_evidence
                .get("fixture_free_gate")
                .and_then(|gate| gate.get("status"))
                .and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            live_evidence
                .get("mcp")
                .and_then(|mcp| mcp.get("codex_tool_call_sent"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            read_json_value(&out.join("validation_report.json"))
                .unwrap()
                .get("verdict")
                .and_then(Value::as_str),
            Some("pass")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_blocks_invalid_actual_pr_marker() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-invalid-pr-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_live_dry_run_receipt(&artifacts, &target);

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: Some(CodexLiveFixture {
                thread_id: Some("thread-live-invalid-pr".to_string()),
                status: CodexLiveStatus::Succeeded,
                content: [
                    "Implementation completed.",
                    "FDA_ACTUAL_PR_URL: N/A",
                    "FDA_TEST_STATUS: passed",
                    "FDA_TESTS_RUN: cargo test",
                    "FDA_CHANGED_FILES: src/lib.rs",
                    "FDA_SCOPE_DRIFT: none",
                ]
                .join("\n"),
            }),
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.development_gate_status.as_deref(), Some("blocked"));
        assert_eq!(result.actual_pr_url, None);
        assert_eq!(
            read_json_value(&out.join("external_pr_receipt.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("blocked")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_blocks_actual_pr_url_without_repo() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-malformed-pr-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_successful_live_dry_run_receipt(&artifacts, &target);

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: Some(CodexLiveFixture {
                thread_id: Some("thread-live-malformed-pr".to_string()),
                status: CodexLiveStatus::Succeeded,
                content: [
                    "Implementation completed.",
                    "FDA_ACTUAL_PR_URL: https://github.com/foo/pull/123",
                    "FDA_TEST_STATUS: passed",
                    "FDA_TESTS_RUN: cargo test",
                    "FDA_CHANGED_FILES: src/lib.rs",
                    "FDA_SCOPE_DRIFT: none",
                ]
                .join("\n"),
            }),
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.development_gate_status.as_deref(), Some("blocked"));
        assert_eq!(result.actual_pr_url, None);

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_blocks_dry_run_cwd_mismatch() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-cwd-mismatch-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        let other_target = base.join("other-target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&other_target).unwrap();
        write_successful_live_dry_run_receipt(&artifacts, &other_target);

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: None,
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.development_gate_status.as_deref(), Some("blocked"));
        assert!(result.detected_tools.is_empty());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_blocks_missing_dry_run_checks() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-missing-checks-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_json_file(
            &artifacts.join("dry_run_receipt.json"),
            &json!({
                "schema_version": "fda.mcp_dry_run_receipt.v0",
                "receipt_id": "MCPDRY-FDA-TEST-001",
                "plan_id": "MCP-FDA-TEST-001",
                "invocation_id": "INV-FDA-TEST-001",
                "provider": "codex",
                "mcp_server_command": ["codex", "mcp-server"],
                "cwd": target.to_string_lossy(),
                "status": "succeeded",
                "started_at": "unix:1",
                "completed_at": "unix:2",
                "target_repo_mutated": false,
                "expected_tools": ["codex", "codex-reply"],
                "detected_tools": ["codex", "codex-reply"],
                "missing_tools": [],
                "evidence_links": ["mcp_tool_call_receipt.json"]
            }),
        )
        .unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target,
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: None,
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.development_gate_status.as_deref(), Some("blocked"));
        assert!(result.detected_tools.is_empty());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_rejects_output_inside_missing_target_repo() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-missing-target-out-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let target = base.join("missing-target");
        let out = target.join("fda-out");
        fs::create_dir_all(&artifacts).unwrap();

        let error = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out),
            target_repo: target.clone(),
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: None,
            codex_live_fixture: None,
        })
        .unwrap_err();

        assert!(error.contains("must not be inside target repo"));
        assert!(!target.exists());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_live_blocks_without_passing_dry_run_receipt() {
        let base = env::temp_dir().join(format!(
            "fda-implement-live-blocked-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            dry_run: false,
            live: true,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: None,
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.development_gate_status.as_deref(), Some("blocked"));
        assert!(result.detected_tools.is_empty());
        assert_eq!(
            read_json_value(&out.join("implementation_receipt.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("blocked")
        );
        let live_evidence = read_json_value(&out.join("live_execution_evidence.json")).unwrap();
        assert_eq!(
            live_evidence.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            live_evidence.get("fixture_used").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            live_evidence
                .get("mcp")
                .and_then(|mcp| mcp.get("tools_list_source"))
                .and_then(Value::as_str),
            Some("not_invoked")
        );
        assert_eq!(
            live_evidence
                .get("mcp")
                .and_then(|mcp| mcp.get("codex_tool_call_sent"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            read_json_value(&out.join("validation_report.json"))
                .unwrap()
                .get("verdict")
                .and_then(Value::as_str),
            Some("pass")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn implement_dry_run_rejects_symlinked_output_inside_target_repo() {
        use std::os::unix::fs::symlink;

        let base = env::temp_dir().join(format!(
            "fda-implement-symlink-out-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let target = base.join("target");
        let out_alias = base.join("out-alias");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("sentinel.txt"), "unchanged").unwrap();
        symlink(&target, &out_alias).unwrap();

        let error = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out_alias),
            target_repo: target.clone(),
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: None,
        })
        .unwrap_err();

        assert!(error.contains("must not be inside target repo"));
        assert!(!target.join("dry_run_receipt.json").exists());
        assert!(!target.join(".fda").exists());
        assert_eq!(
            fs::read_to_string(target.join("sentinel.txt")).unwrap(),
            "unchanged"
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_dry_run_blocks_unresolved_human_decision_before_tools_list() {
        let base = env::temp_dir().join(format!(
            "fda-implement-blocked-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "waiting_human",
                "decisions": [
                    {
                        "decision_id": "HD-FDA-001",
                        "summary": "dry-run前にscopeを固定してよいか",
                        "recommended_option_id": "approve_scope",
                        "required_before": "MCP Dry-run Gate",
                        "options": [
                            { "id": "approve_scope", "description": "承認する", "recommended": true },
                            { "id": "revise", "description": "修正する", "recommended": false }
                        ]
                    }
                ]
            }),
        )
        .unwrap();
        write_text_file(
            &artifacts.join("risk_register.md"),
            "# Risk Register\n\n- RISK-FDA-TEST-001: delivery risk / low\n",
        )
        .unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts.clone(),
            out: Some(out.clone()),
            target_repo: target.clone(),
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: None,
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.dry_run_gate_status, "blocked");
        assert!(result.detected_tools.is_empty());
        assert_eq!(
            read_json_value(&out.join("mcp_agent_invocation_plan.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            read_json_value(&out.join("dry_run_receipt.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("blocked")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_dry_run_honors_resolved_packet_without_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-implement-resolved-packet-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "decision_packet_id": "HDP-FDA-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-MCP",
                "case_id": Value::Null,
                "status": "resolved",
                "required_before": "MCP Dry-run Gate",
                "decision_needed": "dry-run前にscopeを固定してよいか",
                "trigger": "implement dry-run before tools/list",
                "context": {
                    "current_state": "resolved scope decision",
                    "relevant_requirement": "MCP Dry-run Gate",
                    "relevant_evidence": ["human_decision_packet.json"]
                },
                "options": [
                    {
                        "id": "approve_scope",
                        "description": "承認する",
                        "pros": ["MCP dry-runへ進める"],
                        "cons": ["後続のscope変更は別Decisionになる"],
                        "recommended": true
                    },
                    {
                        "id": "revise",
                        "description": "修正する",
                        "pros": ["scopeを見直せる"],
                        "cons": ["dry-runが停止する"],
                        "recommended": false
                    }
                ],
                "impact": {
                    "scope": "scope fixed",
                    "security": "security reviewed later",
                    "schedule": "dry-run can proceed",
                    "ux": "CLI can continue",
                    "operations": "decision evidence is carried"
                },
                "default_if_no_decision": "waiting_human",
                "forge_mapping": {
                    "claim_ids": ["CLM-FDA-DRY-RUN"],
                    "proof_obligations": ["dry_run_receipt.json"],
                    "human_decision_points": ["HD-FDA-001"],
                    "ato_task_graph": ["TASK-FDA-DRY-RUN"],
                    "planned_prs": ["PR-V1-005"],
                    "gate_requirements": ["Human Decision must be resolved before MCP dry-run"]
                },
                "recorded_decision": {
                    "decision": "HD-FDA-001=approve_scope",
                    "decided_by": "human",
                    "decided_at": "unix:1",
                    "rationale": "existing artifact compatibility"
                }
            }),
        )
        .unwrap();
        write_text_file(
            &artifacts.join("risk_register.md"),
            "# Risk Register\n\n- RISK-FDA-TEST-001: delivery risk / low\n",
        )
        .unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts.clone(),
            out: Some(out.clone()),
            target_repo: target,
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.dry_run_gate_status, "succeeded");
        assert!(!artifacts.join("decision_receipts.json").exists());
        assert_eq!(
            read_json_value(&out.join("mcp_agent_invocation_plan.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("ready")
        );
        assert!(
            out.join("risk_register.md").exists(),
            "implement output should carry risk evidence forward"
        );
        assert!(
            out.join("human_decision_packet.json").exists(),
            "implement output should carry decision evidence forward"
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_dry_run_blocks_non_approval_recorded_decision_without_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-implement-recorded-revise-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "resolved",
                "recorded_decision": {
                    "decision": "HD-FDA-001=revise",
                    "decided_by": "human",
                    "decided_at": "unix:1",
                    "rationale": "legacy packet"
                },
                "decisions": [
                    {
                        "decision_id": "HD-FDA-001",
                        "summary": "dry-run前にscopeを固定してよいか",
                        "recommended_option_id": "approve_scope",
                        "required_before": "MCP Dry-run Gate",
                        "options": [
                            { "id": "approve_scope", "description": "承認する", "recommended": true },
                            { "id": "revise", "description": "修正する", "recommended": false }
                        ]
                    }
                ]
            }),
        )
        .unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let plan = read_json_value(&out.join("mcp_agent_invocation_plan.json")).unwrap();
        assert_eq!(plan.get("status").and_then(Value::as_str), Some("blocked"));
        assert_eq!(
            plan.pointer("/human_decision_guard/non_approval_decision_ids/0")
                .and_then(Value::as_str),
            Some("HD-FDA-001")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_dry_run_blocks_partial_recorded_decisions_without_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-implement-recorded-partial-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "resolved",
                "recorded_decision": {
                    "decision": "HD-FDA-001=approve_scope",
                    "decided_by": "human",
                    "decided_at": "unix:1",
                    "rationale": "legacy packet"
                },
                "decisions": [
                    {
                        "decision_id": "HD-FDA-001",
                        "summary": "scopeを固定してよいか",
                        "recommended_option_id": "approve_scope",
                        "required_before": "MCP Dry-run Gate",
                        "options": [
                            { "id": "approve_scope", "description": "承認する", "recommended": true }
                        ]
                    },
                    {
                        "decision_id": "HD-FDA-002",
                        "summary": "target repoを固定してよいか",
                        "recommended_option_id": "approve_repo",
                        "required_before": "MCP Dry-run Gate",
                        "options": [
                            { "id": "approve_repo", "description": "承認する", "recommended": true }
                        ]
                    }
                ]
            }),
        )
        .unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        let plan = read_json_value(&out.join("mcp_agent_invocation_plan.json")).unwrap();
        assert_eq!(plan.get("status").and_then(Value::as_str), Some("blocked"));
        assert_eq!(
            plan.pointer("/human_decision_guard/unresolved_decision_ids/0")
                .and_then(Value::as_str),
            Some("HD-FDA-002")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn implement_dry_run_accepts_recorded_decision_alias_without_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-implement-recorded-alias-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let out = base.join("out");
        let target = base.join("target");
        fs::create_dir_all(&artifacts).unwrap();
        fs::create_dir_all(&target).unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "decision_packet_id": "HDPACKET-FDA-001",
                "decision_needed": "top-level approval",
                "status": "resolved",
                "recorded_decision": {
                    "decision": "HDPACKET-FDA-001=approve_top_level",
                    "decided_by": "human",
                    "decided_at": "unix:1",
                    "rationale": "legacy top-level packet"
                },
                "options": [
                    { "id": "approve_top_level", "description": "承認する", "recommended": true },
                    { "id": "revise_top_level", "description": "修正する", "recommended": false }
                ],
                "forge_mapping": {
                    "human_decision_points": ["HD-FDA-001"]
                }
            }),
        )
        .unwrap();

        let result = implement(&ImplementConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(out.clone()),
            target_repo: target,
            dry_run: true,
            live: false,
            live_timeout_seconds: 1800,
            ato: AtoConfig::default(),
            print_json: false,
            tools_list_fixture: Some(vec!["codex".to_string(), "codex-reply".to_string()]),
            codex_live_fixture: None,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(
            read_json_value(&out.join("mcp_agent_invocation_plan.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("ready")
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn planned_pr_execution_packet_schema_allows_blocked_unresolved_only() {
        let schema = compile_test_schema(
            "docs/standards/delivery-artifacts-v0/schemas/planned_pr_execution_packet.schema.json",
        );
        let mut packet = json!({
            "schema_version": "planned_pr_execution_packet.v0",
            "packet_id": "PPEXEC-FDA-TEST-001",
            "status": "blocked",
            "source_repo": "msamunetogetoge/forge-delivery-agent",
            "target_repo": {
                "name": "msamunetogetoge/target",
                "local_path": "../target"
            },
            "program_id": "FDA-V1",
            "epic_id": "EPIC-FDA-V1-MCP",
            "planned_pr_id": "PR-V1-004",
            "planned_pr_title": "MCP invocation schemas",
            "handoff_kind": "mcp_implementation",
            "source_artifacts": ["implementation_handoff.md"],
            "resolved_human_decisions": [],
            "human_decision_dependencies": ["HD-FDA-001"],
            "scope_in": ["MCP invocation contract を定義する"],
            "scope_out": ["未解決判断のまま実装を開始しない"],
            "acceptance_criteria": ["未解決判断がある packet は blocked として表現できる"],
            "security_privacy_legal_checks": ["QA role は read-only policy を持つ"],
            "expected_evidence_from_target_pr": ["dry_run_receipt.json"],
            "implementation_start_gate": {
                "human_decisions_resolved": false,
                "target_repo_changes_allowed_by_this_repo": false,
                "target_repo_pr_creation_allowed_by_this_packet": false,
                "notes": "未解決判断があるため実装開始不可"
            }
        });

        assert_schema_valid(&schema, &packet);
        packet["status"] = json!("ready_for_external_implementation");
        assert_schema_invalid(&schema, &packet);
        packet["implementation_start_gate"]["human_decisions_resolved"] = json!(true);
        assert_schema_valid(&schema, &packet);
    }

    #[test]
    fn mcp_tool_call_receipt_schema_rejects_failed_pass_receipts() {
        let schema = compile_test_schema(
            "docs/standards/delivery-artifacts-v0/schemas/mcp_tool_call_receipt.schema.json",
        );
        let mut receipt = json!({
            "schema_version": "fda.mcp_tool_call_receipt.v0",
            "receipt_id": "MCPR-FDA-TEST-001",
            "plan_id": "MCPPLAN-FDA-TEST-001",
            "invocation_id": "MCPINV-FDA-TEST-001",
            "role": "implementer",
            "provider": "codex",
            "mcp_server": "codex mcp-server",
            "tool_name": "codex",
            "cwd": "/workspace/target",
            "status": "failed",
            "started_at": "2026-06-27T00:00:00Z",
            "completed_at": "2026-06-27T00:01:00Z",
            "input_artifacts": ["implementation_handoff.md"],
            "output_artifacts": [],
            "tool_result_digest": {
                "raw_result_stored": false,
                "summary": "tool call failed",
                "exit_code": 1
            },
            "semantic_result": {
                "semantic_verdict": "pass",
                "summary": "誤って pass と解釈された",
                "scope_drift": [],
                "tests": [],
                "gate_effect": "advance"
            },
            "evidence_links": ["MCPR-FDA-TEST-001"],
            "next_action": "repair"
        });

        assert_schema_invalid(&schema, &receipt);
        receipt["semantic_result"]["semantic_verdict"] = json!("fail");
        receipt["semantic_result"]["gate_effect"] = json!("repair");
        assert_schema_valid(&schema, &receipt);

        receipt["status"] = json!("adapter_unavailable");
        receipt["semantic_result"]["semantic_verdict"] = json!("pass");
        receipt["semantic_result"]["gate_effect"] = json!("advance");
        assert_schema_invalid(&schema, &receipt);

        receipt["status"] = json!("succeeded");
        assert_schema_valid(&schema, &receipt);
    }

    #[test]
    fn agent_roles_profile_schema_requires_pr_reviewer() {
        let schema = compile_test_schema(
            "docs/standards/fda-v1/schemas/repository-profile/agent_roles_yaml.schema.json",
        );
        let mut profile = json!({
            "agent_roles": {
                "default_orchestrator": {
                    "executor": "current_codex_cli",
                    "workspace_policy": "workspace_write",
                    "can_edit": true
                },
                "implementer": {
                    "executor": "current_codex_cli",
                    "workspace_policy": "workspace_write",
                    "can_edit": true
                },
                "pr_reviewer": {
                    "executor": "codex_subagent",
                    "workspace_policy": "read_only",
                    "can_edit": false
                },
                "functional_qa": {
                    "executor": "codex_subagent",
                    "workspace_policy": "read_only",
                    "can_edit": false
                },
                "security_qa": {
                    "executor": "codex_subagent",
                    "workspace_policy": "read_only",
                    "can_edit": false
                },
                "merge_manager": {
                    "executor": "current_codex_cli",
                    "workspace_policy": "controlled_write",
                    "can_edit": false
                }
            }
        });

        assert_schema_valid(&schema, &profile);
        profile["agent_roles"]
            .as_object_mut()
            .unwrap()
            .remove("pr_reviewer");
        assert_schema_invalid(&schema, &profile);
    }

    #[test]
    fn review_agent_gate_schema_allows_forge_fallback_roles() {
        let schema = compile_test_schema(
            "docs/standards/delivery-artifacts-v0/schemas/review_agent_gate.schema.json",
        );
        let gate = json!({
            "schema_version": "fda.review_agent_gate.v0",
            "gate_id": "REVIEW-GATE-FDA-TEST-001",
            "program_id": "FDA-V1",
            "epic_id": "EPIC-FDA-V1",
            "planned_pr_id": "PR-V1-007",
            "reviewed_planned_pr_id": "PR-V1-006",
            "status": "passed",
            "actual_pr_url": "https://github.com/msamunetogetoge/example/pull/123",
            "required_reviewers": [
                {
                    "role": "pr_reviewer",
                    "required": true,
                    "status": "passed",
                    "workspace_policy": "read_only",
                    "source_mutation_allowed": false,
                    "evidence": ["pr_reviewer_receipt.json"],
                    "not_applicable_reason": Value::Null
                },
                {
                    "role": "functional_qa",
                    "required": true,
                    "status": "passed",
                    "workspace_policy": "read_only",
                    "source_mutation_allowed": false,
                    "evidence": ["functional_qa_receipt.json"],
                    "not_applicable_reason": Value::Null
                },
                {
                    "role": "security_qa",
                    "required": true,
                    "status": "passed",
                    "workspace_policy": "read_only",
                    "source_mutation_allowed": false,
                    "evidence": ["security_qa_receipt.json"],
                    "not_applicable_reason": Value::Null
                }
            ],
            "conditional_reviewers": [
                {
                    "role": "qax2",
                    "required": true,
                    "status": "passed",
                    "workspace_policy": "read_only",
                    "source_mutation_allowed": false,
                    "evidence": ["qax2_receipt.json"],
                    "not_applicable_reason": Value::Null
                },
                {
                    "role": "orchestrator",
                    "required": true,
                    "status": "passed",
                    "workspace_policy": "read_only",
                    "source_mutation_allowed": false,
                    "evidence": ["orchestrator_review_receipt.json"],
                    "not_applicable_reason": Value::Null
                }
            ],
            "source_mutation_allowed": false,
            "merge_approval_granted": false,
            "evidence_links": ["review_agent_gate.json"],
            "next_action": "fda merge"
        });

        assert_schema_valid(&schema, &gate);
    }

    fn compile_test_schema(path: &str) -> Value {
        read_json_value(Path::new(path)).unwrap()
    }

    fn assert_schema_valid(schema: &Value, value: &Value) {
        let validator = JsonSchemaArtifactValidator;
        match validator.validate_json_schema(schema, value) {
            Ok(errors) if errors.is_empty() => {}
            Ok(errors) => {
                let messages = errors
                    .into_iter()
                    .map(|error| error.message)
                    .collect::<Vec<_>>();
                panic!("expected schema-valid value, got errors: {messages:?}");
            }
            Err(error) => {
                let messages = vec![error.message];
                panic!("expected schema-valid value, got errors: {messages:?}");
            }
        }
    }

    fn assert_schema_invalid(schema: &Value, value: &Value) {
        let validator = JsonSchemaArtifactValidator;
        match validator.validate_json_schema(schema, value) {
            Ok(errors) => assert!(!errors.is_empty(), "expected schema-invalid value"),
            Err(_) => {
                panic!("expected schema-invalid value, got schema compile error");
            }
        }
    }

    #[test]
    fn start_dry_run_writes_intake_outputs() {
        let out = env::temp_dir().join(format!(
            "fda-start-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }

        let result = start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("oshi-noteでVTuber紹介リンク/PRページを作りたい".to_string()),
            out: Some(out.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(
            result.implementation_classification,
            "implementation_candidate"
        );
        for file_name in [
            "requirements_definition.md",
            "non_functional_requirements.md",
            "risk_register.md",
            "human_decision_packet.md",
            "human_decision_packet.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }

        let requirements = fs::read_to_string(out.join("requirements_definition.md")).unwrap();
        assert!(requirements.contains("HD-FDA-001"));
        assert!(requirements.contains("fda design"));

        let decision_packet = read_json_value(&out.join("human_decision_packet.json")).unwrap();
        assert_eq!(
            decision_packet.get("status").and_then(Value::as_str),
            Some("waiting_human")
        );
        assert_eq!(
            decision_packet
                .get("decisions")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(3)
        );

        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn start_research_mode_writes_non_implementation_outputs() {
        let out = env::temp_dir().join(format!(
            "fda-start-research-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }

        let result = start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("VTuber PRページの法務リスクを調査したい".to_string()),
            out: Some(out.clone()),
            mode: IntakeMode::Research,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.implementation_classification, "research_ready");
        for file_name in [
            "research_report.md",
            "source_refs.md",
            "artifact_inventory.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        let inventory = read_json_value(&out.join("artifact_inventory.json")).unwrap();
        assert!(inventory
            .get("groups")
            .and_then(Value::as_array)
            .is_some_and(|groups| groups
                .iter()
                .any(|group| group.get("group_id").and_then(Value::as_str)
                    == Some("non_implementation_outputs"))));

        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn start_uiux_mode_writes_mock_outputs() {
        let out = env::temp_dir().join(format!(
            "fda-start-uiux-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }

        let result = start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("この機能のUIUXとモックを作って".to_string()),
            out: Some(out.clone()),
            mode: IntakeMode::Uiux,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.implementation_classification, "uiux_ready");
        for file_name in [
            "uiux_brief.md",
            "user_flow.md",
            "mock.html",
            "mock.excalidraw",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        assert!(fs::read_to_string(out.join("mock.html"))
            .unwrap()
            .contains("UIUX Mode Mock"));

        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn start_design_only_mode_writes_design_outputs() {
        let out = env::temp_dir().join(format!(
            "fda-start-design-only-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }

        let result = start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("この要件の基本設計まで作って".to_string()),
            out: Some(out.clone()),
            mode: IntakeMode::DesignOnly,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.implementation_classification, "design_only_ready");
        for file_name in [
            "basic_design.md",
            "detailed_design.md",
            "implementation_readiness_report.md",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }
        assert!(
            fs::read_to_string(out.join("implementation_readiness_report.md"))
                .unwrap()
                .contains("not_ready_until_human_decisions_resolved")
        );

        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn open_output_hub_writes_html_views() {
        let base = env::temp_dir().join(format!(
            "fda-open-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let hub = base.join("hub");
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("Output Hubを確認したい".to_string()),
            out: Some(artifacts.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let result = open_output_hub(&OpenConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(hub.clone()),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        for file_name in [
            "output_hub.html",
            "decision_inbox.html",
            "execution_status.html",
            "output_hub_receipt.json",
        ] {
            assert!(hub.join(file_name).exists(), "{file_name} should exist");
        }
        assert!(fs::read_to_string(hub.join("output_hub.html"))
            .unwrap()
            .contains("FDA Output Hub"));
        assert!(fs::read_to_string(hub.join("decision_inbox.html"))
            .unwrap()
            .contains("HD-FDA-001"));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn open_output_hub_rejects_missing_artifact_dir() {
        let base = env::temp_dir().join(format!(
            "fda-open-missing-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let result = open_output_hub(&OpenConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: base.join("missing-artifacts"),
            out: Some(base.join("hub")),
            ato: AtoConfig::default(),
            print_json: false,
        });

        assert!(result.is_err());
    }

    #[test]
    fn decision_rows_include_top_level_packet_and_skip_resolved_receipts() {
        let base = env::temp_dir().join(format!(
            "fda-decision-row-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        fs::create_dir_all(&artifacts).unwrap();
        write_json_file(
            &artifacts.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "decision_packet_id": "HDP-FDA-TOP-001",
                "status": "waiting_human",
                "decision_needed": "top-level packet decision",
                "required_before": "Design Gate",
                "forge_mapping": {
                    "human_decision_points": ["HD-FDA-TOP-001"]
                },
                "options": [
                    { "id": "approve_scope", "recommended": true },
                    { "id": "hold_scope" }
                ]
            }),
        )
        .unwrap();

        let rows = decision_rows_from_artifacts(&artifacts).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].decision_id, "HD-FDA-TOP-001");
        assert_eq!(rows[0].recommended_option_id, "approve_scope");
        assert_eq!(rows[0].options, vec!["approve_scope", "hold_scope"]);

        write_json_file(
            &artifacts.join("decision_receipts.json"),
            &json!({
                "schema_version": "fda.decision_receipts.v0",
                "receipts": [
                    {
                        "decision_id": "HD-FDA-TOP-001",
                        "answer": "approve_scope"
                    }
                ]
            }),
        )
        .unwrap();
        assert!(decision_rows_from_artifacts(&artifacts).unwrap().is_empty());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn notify_test_writes_request_receipt_and_notice() {
        let base = env::temp_dir().join(format!(
            "fda-notify-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let notify = base.join("notify");
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("通知テストをしたい".to_string()),
            out: Some(artifacts.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let result = notify_test(&NotifyConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts.clone(),
            out: Some(notify.clone()),
            channel: "slack".to_string(),
            recipient: Some("#fda-human-turn".to_string()),
            live: false,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.notification_status, "skipped");
        for file_name in [
            "notification_request.json",
            "notification_receipt.json",
            "human_turn_notice.md",
        ] {
            assert!(notify.join(file_name).exists(), "{file_name} should exist");
        }
        assert_eq!(
            read_json_value(&notify.join("notification_receipt.json"))
                .unwrap()
                .get("sent")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            read_json_value(&notify.join("notification_receipt.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("skipped")
        );
        let request = read_json_value(&notify.join("notification_request.json")).unwrap();
        assert_eq!(
            request.get("recipient_source").and_then(Value::as_str),
            Some("cli")
        );
        assert_eq!(
            request.get("channel").and_then(Value::as_str),
            Some("slack")
        );
        let expected_repo_root = fs::canonicalize(".").unwrap();
        let expected_repo_root = expected_repo_root.to_string_lossy().to_string();
        assert_eq!(
            request.get("repo_root").and_then(Value::as_str),
            Some(expected_repo_root.as_str())
        );
        let repo_name = request.get("repo_name").and_then(Value::as_str).unwrap();
        assert!(!repo_name.is_empty());
        assert_ne!(repo_name, "unknown-repo");
        assert_eq!(
            request.get("project").and_then(Value::as_str),
            Some(repo_name)
        );
        assert_eq!(
            request.get("artifact_dir").and_then(Value::as_str),
            Some(artifacts.to_string_lossy().as_ref())
        );
        let expected_decision_document_path = artifacts.join("human_decision_packet.md");
        assert_eq!(
            request
                .get("decision_document_path")
                .and_then(Value::as_str),
            Some(expected_decision_document_path.to_string_lossy().as_ref())
        );
        assert!(request
            .get("decisions")
            .and_then(Value::as_array)
            .is_some_and(|decisions| decisions
                .first()
                .and_then(|decision| decision.get("options"))
                .and_then(Value::as_array)
                .is_some_and(|options| !options.is_empty())));
        assert!(request
            .get("options")
            .and_then(Value::as_array)
            .is_some_and(|options| !options.is_empty()));
        assert_eq!(
            request.get("recommended_option").and_then(Value::as_str),
            Some("approve_scope")
        );
        assert_eq!(
            request.get("resume_command").and_then(Value::as_str),
            Some("fda decide HD-FDA-001 --answer <answer>")
        );
        assert!(fs::read_to_string(notify.join("human_turn_notice.md"))
            .unwrap()
            .contains("HD-FDA-001"));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn notification_request_uses_common_repo_name_for_git_worktree() {
        let base = env::temp_dir().join(format!(
            "fda-notify-worktree-name-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let repo_root = base.join("feature-worktree");
        let common_repo = base.join("forge-delivery-agent");
        let artifacts = repo_root.join("artifacts");
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }
        fs::create_dir_all(&artifacts).unwrap();
        fs::write(
            repo_root.join(".git"),
            format!(
                "gitdir: {}/.git/worktrees/feature-worktree\n",
                common_repo.display()
            ),
        )
        .unwrap();

        let recipient = resolve_notification_recipient("slack", None, None, None, true);
        let request = notification_request(&repo_root, &artifacts, "slack", &recipient, &[], true);

        assert_eq!(
            request.get("repo_name").and_then(Value::as_str),
            Some("forge-delivery-agent")
        );
        assert_eq!(
            request.get("project").and_then(Value::as_str),
            Some("forge-delivery-agent")
        );
        assert_eq!(
            request
                .get("decision_document_path")
                .and_then(Value::as_str),
            Some(
                artifacts
                    .join("human_decision_packet.md")
                    .to_string_lossy()
                    .as_ref()
            )
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn notify_recipient_resolution_uses_env_and_default_candidate() {
        let from_env =
            resolve_notification_recipient("email", None, Some("ops@example.com"), None, false);
        assert_eq!(from_env.recipient, "ops@example.com");
        assert_eq!(from_env.recipient_source, "env:FDA_NOTIFY_EMAIL");
        assert!(from_env.sendable);

        let missing = resolve_notification_recipient("email", None, None, None, false);
        assert_eq!(missing.recipient, "kenjiii534@gmail.com");
        assert_eq!(missing.recipient_source, "default:candidate");
        assert!(!missing.sendable);
        let slack_from_env =
            resolve_notification_recipient("slack", None, None, Some("#fda-human-turn"), true);
        assert_eq!(slack_from_env.recipient, "#fda-human-turn");
        assert_eq!(
            slack_from_env.recipient_source,
            "env:FDA_SLACK_CHANNEL_LABEL"
        );
        assert!(slack_from_env.sendable);

        let slack_default = resolve_notification_recipient("slack", None, None, None, false);
        assert_eq!(slack_default.recipient, "slack:webhook");
        assert_eq!(slack_default.recipient_source, "channel-default");
        assert!(!slack_default.sendable);

        let slack_cli_without_webhook =
            resolve_notification_recipient("slack", Some("#fda-human-turn"), None, None, false);
        assert_eq!(slack_cli_without_webhook.recipient, "#fda-human-turn");
        assert_eq!(slack_cli_without_webhook.recipient_source, "cli");
        assert!(!slack_cli_without_webhook.sendable);

        let slack_cli_with_webhook =
            resolve_notification_recipient("slack", Some("#fda-human-turn"), None, None, true);
        assert_eq!(slack_cli_with_webhook.recipient, "#fda-human-turn");
        assert_eq!(slack_cli_with_webhook.recipient_source, "cli");
        assert!(slack_cli_with_webhook.sendable);
    }

    #[test]
    fn notify_test_treats_invalid_slack_webhook_as_not_sendable() {
        let _slack_env = slack_env_lock();
        env::set_var("FDA_SLACK_WEBHOOK_URL", "https://example.com/hook");
        env::remove_var("FDA_SLACK_CHANNEL_LABEL");

        let base = env::temp_dir().join(format!(
            "fda-notify-invalid-slack-webhook-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let notify = base.join("notify");
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("Slack webhook検証をしたい".to_string()),
            out: Some(artifacts.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        notify_test(&NotifyConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(notify.clone()),
            channel: "slack".to_string(),
            recipient: None,
            live: false,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        let request = read_json_value(&notify.join("notification_request.json")).unwrap();
        assert_eq!(
            request.get("sendable").and_then(Value::as_bool),
            Some(false)
        );

        env::remove_var("FDA_SLACK_WEBHOOK_URL");
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn parses_notify_test_defaults_to_slack() {
        let command = parse_args(vec!["notify".to_string(), "test".to_string()]).unwrap();

        match command {
            Command::NotifyTest(config) => {
                assert_eq!(config.channel, "slack");
                assert!(!config.live);
            }
            _ => panic!("expected notify test command"),
        }
    }

    fn test_decision_view() -> crate::rendering::output_hub::DecisionView {
        crate::rendering::output_hub::DecisionView {
            decision_id: "HD-001".to_string(),
            summary: "Choose <threshold> & notify <!here>".to_string(),
            required_before: "Design Gate".to_string(),
            status: "waiting_human".to_string(),
            options: vec!["10 users".to_string(), "20 users".to_string()],
            recommended_option_id: "10 users".to_string(),
            resume_command: "fda decide HD-001 --answer \"10 users\"".to_string(),
        }
    }

    #[test]
    fn live_slack_blocks_missing_webhook_url() {
        let _slack_env = slack_env_lock();
        env::remove_var("FDA_SLACK_WEBHOOK_URL");
        env::remove_var("FDA_SLACK_CHANNEL_LABEL");

        let recipient = resolve_notification_recipient("slack", None, None, None, false);
        let decisions = vec![test_decision_view()];
        let request = notification_request(
            Path::new("/tmp/example-repo"),
            Path::new("/tmp/example-repo/artifacts/run"),
            "slack",
            &recipient,
            &decisions,
            true,
        );
        let receipt = live_notification_receipt(&request);

        assert_eq!(
            receipt.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            receipt.get("adapter").and_then(Value::as_str),
            Some("slack_incoming_webhook")
        );
        assert_eq!(
            receipt.get("failure_reason").and_then(Value::as_str),
            Some("missing required Slack env: FDA_SLACK_WEBHOOK_URL")
        );
    }

    #[test]
    fn live_slack_skips_when_no_open_decisions() {
        let _slack_env = slack_env_lock();
        env::remove_var("FDA_SLACK_WEBHOOK_URL");
        env::remove_var("FDA_SLACK_CHANNEL_LABEL");

        let recipient =
            resolve_notification_recipient("slack", Some("#fda-human-turn"), None, None, true);
        let request = notification_request(
            Path::new("/tmp/example-repo"),
            Path::new("/tmp/example-repo/artifacts/run"),
            "slack",
            &recipient,
            &[],
            true,
        );
        let receipt = live_notification_receipt(&request);

        assert_eq!(
            receipt.get("status").and_then(Value::as_str),
            Some("skipped")
        );
        assert_eq!(
            receipt.get("skip_reason").and_then(Value::as_str),
            Some("no_open_human_decision")
        );
        assert_eq!(receipt.get("sent").and_then(Value::as_bool), Some(false));
        assert_eq!(
            receipt.get("recipient").and_then(Value::as_str),
            Some("#fda-human-turn")
        );
    }

    #[test]
    fn live_email_blocks_default_candidate_recipient() {
        let recipient = resolve_notification_recipient("email", None, None, None, false);
        let request = notification_request(
            Path::new("/tmp/example-repo"),
            Path::new("/tmp/example-repo/artifacts/run"),
            "email",
            &recipient,
            &[],
            true,
        );
        let receipt = live_notification_receipt(&request);

        assert_eq!(
            receipt.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            receipt.get("failure_reason").and_then(Value::as_str),
            Some("live_email_requires_explicit_recipient")
        );
        assert_eq!(
            receipt.get("sendable").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn slack_helpers_reject_unexpected_url_and_render_payload() {
        assert_eq!(
            slack_webhook_url(" https://hooks.slack.com/services/T/B/X ").unwrap(),
            "https://hooks.slack.com/services/T/B/X"
        );
        assert!(
            slack_webhook_url("https://hooks.slack-gov.com/services/T/B/X")
                .unwrap_err()
                .contains("hooks.slack.com")
        );
        assert!(slack_webhook_url("https://example.com/hook")
            .unwrap_err()
            .contains("hooks.slack.com"));
        assert!(
            slack_webhook_url("https://hooks.slack.com/services/T/B/X\r\nbad")
                .unwrap_err()
                .contains("CR/LF")
        );

        let request = json!({
            "repo_name": "forge-delivery-agent",
            "repo_root": "/tmp/fda-v1-slack-p0-notification",
            "artifact_dir": "/tmp/fda-slack-live-smoke/artifacts",
            "decision_document_path": "/tmp/fda-slack-live-smoke/artifacts/human_decision_packet.md",
            "summary": "Choose <threshold> & notify",
            "decision_ids": ["HD-001"],
            "decisions": [
                {
                    "decision_id": "HD-001",
                    "summary": "Choose <threshold> & notify <!here>",
                    "recommended_option_id": "10 users",
                    "resume_command": "fda decide HD-001 --answer \"10 users\""
                }
            ],
            "options": ["10 users", "20 users"],
            "recommended_option": "10 users",
            "resume_command": "fda decide HD-001 --answer \"10 users\""
        });
        let payload = slack_message_payload(&request);
        assert_eq!(
            payload.get("text").and_then(Value::as_str),
            Some("[FDA] Human Decision requires attention in forge-delivery-agent: HD-001. Decision actions: HD-001: recommended=10 users, resume=fda decide HD-001 --answer \"10 users\". Decision summary: HD-001: Choose &lt;threshold&gt; &amp; notify &lt;!here&gt;. Decision document: /tmp/fda-slack-live-smoke/artifacts/human_decision_packet.md")
        );
        let block_text = payload
            .get("blocks")
            .and_then(Value::as_array)
            .and_then(|blocks| blocks.first())
            .and_then(|block| block.get("text"))
            .and_then(|text| text.get("text"))
            .and_then(Value::as_str)
            .unwrap();
        assert!(block_text.contains("Choose &lt;threshold&gt; &amp; notify"));
        assert!(block_text.contains("HD-001: Choose &lt;threshold&gt; &amp; notify &lt;!here&gt;"));
        assert!(block_text.contains(
            "HD-001: recommended=10 users, resume=fda decide HD-001 --answer \"10 users\""
        ));
        assert!(block_text.contains("*Repository / Project:* forge-delivery-agent"));
        assert!(block_text.contains(
            "*Decision document:* `/tmp/fda-slack-live-smoke/artifacts/human_decision_packet.md`"
        ));
        assert_eq!(slack_response_digest("ok"), "fnv1a64:08b05d07b5566bef");
    }

    #[test]
    fn slack_payload_truncates_block_text_to_slack_limit() {
        let request = json!({
            "repo_name": "forge-delivery-agent",
            "repo_root": "/tmp/fda-v1-slack-p0-notification",
            "artifact_dir": "/tmp/fda-slack-live-smoke/artifacts",
            "decision_document_path": "/tmp/fda-slack-live-smoke/artifacts/human_decision_packet.md",
            "summary": "Human Decision requires attention",
            "decision_ids": ["HD-LONG"],
            "decisions": [
                {
                    "decision_id": "HD-LONG",
                    "summary": "x".repeat(4000),
                    "recommended_option_id": "approve",
                    "resume_command": "fda decide HD-LONG --answer approve"
                }
            ],
            "options": ["approve"],
            "recommended_option": "approve",
            "resume_command": "fda decide HD-LONG --answer approve"
        });
        let payload = slack_message_payload(&request);
        let block_text = payload
            .get("blocks")
            .and_then(Value::as_array)
            .and_then(|blocks| blocks.first())
            .and_then(|block| block.get("text"))
            .and_then(|text| text.get("text"))
            .and_then(Value::as_str)
            .unwrap();

        assert!(block_text.len() <= 3000);
        assert!(block_text.ends_with("..."));
        assert!(block_text
            .contains("HD-LONG: recommended=approve, resume=fda decide HD-LONG --answer approve"));
    }

    #[test]
    fn slack_success_receipt_preserves_request_recipient() {
        let request = json!({
            "notification_id": "NOTIFY-FDA-V1-011-001",
            "recipient": "#cli-human-turn",
            "recipient_source": "cli",
            "sendable": true
        });
        let receipt = successful_slack_notification_receipt(
            &request,
            SlackSendResponse {
                http_status: 200,
                provider_response_digest: "fnv1a64:08b05d07b5566bef".to_string(),
            },
            "unix:1",
        );

        assert_eq!(
            receipt.get("recipient").and_then(Value::as_str),
            Some("#cli-human-turn")
        );
        assert_eq!(
            receipt.get("recipient_source").and_then(Value::as_str),
            Some("cli")
        );
    }

    #[test]
    fn smtp_helpers_reject_injected_addresses_and_encode_body() {
        assert_eq!(
            smtp_envelope_address(" ops@example.com ", "sender").unwrap(),
            "ops@example.com"
        );
        assert!(
            smtp_envelope_address("ops@example.com\r\nRCPT TO:<bad@example.com>", "sender")
                .unwrap_err()
                .contains("CR/LF")
        );
        assert!(smtp_envelope_address("not-an-address", "recipient")
            .unwrap_err()
            .contains("missing @"));

        let request = json!({
            "summary": "Line 1\r\n.Subject line injection",
            "decision_ids": ["HD-001"],
            "decisions": [
                {
                    "decision_id": "HD-001",
                    "summary": "Choose threshold",
                    "required_before": "Design Gate",
                    "options": ["10 users", "20 users"],
                    "recommended_option_id": "10 users",
                    "resume_command": "fda decide HD-001 --answer \"10 users\""
                }
            ],
            "resume_commands": [".quit", "fda decide HD-001 --answer yes"]
        });
        let plain = smtp_plain_message_text(&request);
        assert!(plain.contains("Line 1\n.Subject line injection"));
        assert!(plain.contains("Decision: HD-001\nSummary: Choose threshold\n"));
        assert!(plain.contains("Options:\n- 10 users\n- 20 users\n"));
        assert!(plain.contains("Recommended option: 10 users\n"));

        let body = smtp_message_body(
            "ops@example.com",
            "user@example.com",
            &request,
            "<fda-test@example.local>",
        );
        assert!(body.contains("Subject: [FDA] Human Decision requires attention\r\n"));
        assert!(body.contains("Content-Transfer-Encoding: base64\r\n"));
        assert!(!body.contains("Subject line injection"));
        assert!(!body.contains(".quit"));
        assert!(body.ends_with("\r\n.\r\n"));
        let encoded = body
            .split("\r\n\r\n")
            .nth(1)
            .unwrap()
            .trim_end_matches("\r\n.\r\n");
        assert!(encoded.lines().all(|line| line.len() <= 76));
        assert!(encoded.is_ascii());
    }

    #[test]
    fn smtp_message_id_is_unique_per_send() {
        assert_ne!(smtp_message_id(), smtp_message_id());
    }

    #[test]
    fn smtp_resolve_addresses_uses_bounded_worker() {
        let addresses = smtp_resolve_addresses(&SmtpConfig {
            host: "127.0.0.1".to_string(),
            port: 25,
            username: "user".to_string(),
            password: "password".to_string(),
            from: "ops@example.com".to_string(),
            tls_mode: "none".to_string(),
        })
        .unwrap();

        assert!(addresses.iter().any(|address| address.ip().is_loopback()));
    }

    #[test]
    fn base64_encoder_matches_known_smtp_auth_values() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn notify_test_live_email_without_credentials_writes_blocked_receipt() {
        for key in [
            "FDA_SMTP_HOST",
            "FDA_SMTP_PORT",
            "FDA_SMTP_USERNAME",
            "FDA_SMTP_PASSWORD",
            "FDA_SMTP_FROM",
            "FDA_SMTP_TLS_MODE",
        ] {
            env::remove_var(key);
        }

        let base = env::temp_dir().join(format!(
            "fda-notify-live-missing-creds-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let notify = base.join("notify");
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("通知テストをしたい".to_string()),
            out: Some(artifacts.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let result = notify_test(&NotifyConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(notify.clone()),
            channel: "email".to_string(),
            recipient: Some("user@example.com".to_string()),
            live: true,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.notification_status, "blocked");
        let receipt = read_json_value(&notify.join("notification_receipt.json")).unwrap();
        assert_eq!(
            receipt.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(receipt.get("adapter").and_then(Value::as_str), Some("smtp"));
        assert_eq!(receipt.get("sent").and_then(Value::as_bool), Some(false));
        assert_eq!(receipt.get("dry_run").and_then(Value::as_bool), Some(false));
        assert!(receipt
            .get("failure_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("missing required SMTP env")));
        assert!(receipt.get("sent_at").and_then(Value::as_str).is_some());

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn notify_test_live_slack_without_credentials_writes_blocked_receipt() {
        let _slack_env = slack_env_lock();
        env::remove_var("FDA_SLACK_WEBHOOK_URL");
        env::remove_var("FDA_SLACK_CHANNEL_LABEL");

        let base = env::temp_dir().join(format!(
            "fda-notify-live-slack-missing-creds-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let artifacts = base.join("artifacts");
        let notify = base.join("notify");
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("Slack通知テストをしたい".to_string()),
            out: Some(artifacts.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let result = notify_test(&NotifyConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: artifacts,
            out: Some(notify.clone()),
            channel: "slack".to_string(),
            recipient: Some("#fda-human-turn".to_string()),
            live: true,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.notification_status, "blocked");
        let receipt = read_json_value(&notify.join("notification_receipt.json")).unwrap();
        assert_eq!(
            receipt.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            receipt.get("adapter").and_then(Value::as_str),
            Some("slack_incoming_webhook")
        );
        assert_eq!(receipt.get("sent").and_then(Value::as_bool), Some(false));
        assert_eq!(receipt.get("dry_run").and_then(Value::as_bool), Some(false));
        assert_eq!(
            receipt.get("webhook_source").and_then(Value::as_str),
            Some("env:FDA_SLACK_WEBHOOK_URL")
        );
        assert!(receipt
            .get("failure_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("missing required Slack env")));

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn design_blocks_when_human_decision_is_unresolved() {
        let out = env::temp_dir().join(format!(
            "fda-design-blocked-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("oshi-noteでVTuber紹介リンク/PRページを作りたい".to_string()),
            out: Some(out.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let result = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: out.clone(),
            out: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.design_gate_status, "waiting_human_decision");
        assert_eq!(result.blocked_decisions.len(), 3);
        assert!(result.artifacts_written.is_empty());

        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn design_writes_required_outputs_when_decisions_are_resolved() {
        let intake = env::temp_dir().join(format!(
            "fda-design-intake-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let design_out = env::temp_dir().join(format!(
            "fda-design-output-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        for path in [&intake, &design_out] {
            if path.exists() {
                fs::remove_dir_all(path).unwrap();
            }
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("oshi-noteでVTuber紹介リンク/PRページを作りたい".to_string()),
            out: Some(intake.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        for (decision_id, answer) in [
            ("HD-FDA-001", "yes"),
            ("HD-FDA-002", "accept"),
            ("HD-FDA-003", "confirm before design"),
        ] {
            decide(&DecideConfig {
                repo_root: PathBuf::from("."),
                artifact_dir: intake.clone(),
                decision_id: decision_id.to_string(),
                answer: answer.to_string(),
                decided_by: "test".to_string(),
                ato: AtoConfig::default(),
                print_json: false,
            })
            .unwrap();
        }

        let result = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            out: Some(design_out.clone()),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(
            read_json_value(&intake.join("human_decision_packet.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("resolved")
        );
        assert!(intake.join("decision_receipts.json").exists());
        for file_name in [
            "basic_design.md",
            "detailed_design.md",
            "functional_qa_brief.md",
            "security_qa_brief.md",
            "case_graph.json",
            "task_graph.json",
            "planned_prs.json",
            "autonomy_contract.json",
            "forge_projection.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(
                design_out.join(file_name).exists(),
                "{file_name} should exist"
            );
        }

        let basic_design = fs::read_to_string(design_out.join("basic_design.md")).unwrap();
        assert!(basic_design.contains("OPEN_QUESTIONS"));
        assert!(basic_design.contains("Given Intake"));

        fs::remove_dir_all(&intake).unwrap();
        fs::remove_dir_all(&design_out).unwrap();
    }

    #[test]
    fn revise_answer_keeps_human_decision_packet_blocked() {
        let intake = env::temp_dir().join(format!(
            "fda-revise-block-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if intake.exists() {
            fs::remove_dir_all(&intake).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("oshi-noteでVTuber紹介リンク/PRページを作りたい".to_string()),
            out: Some(intake.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        for (decision_id, answer) in [
            ("HD-FDA-001", "revise"),
            ("HD-FDA-002", "accept"),
            ("HD-FDA-003", "confirm before design"),
        ] {
            decide(&DecideConfig {
                repo_root: PathBuf::from("."),
                artifact_dir: intake.clone(),
                decision_id: decision_id.to_string(),
                answer: answer.to_string(),
                decided_by: "test".to_string(),
                ato: AtoConfig::default(),
                print_json: false,
            })
            .unwrap();
        }

        let decision_packet = read_json_value(&intake.join("human_decision_packet.json")).unwrap();
        assert_eq!(
            decision_packet.get("status").and_then(Value::as_str),
            Some("waiting_human")
        );

        let result = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            out: None,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.blocked_decisions.len(), 1);
        assert_eq!(result.blocked_decisions[0].decision_id, "HD-FDA-001");

        fs::remove_dir_all(&intake).unwrap();
    }

    #[test]
    fn revoked_approval_resets_packet_to_waiting_human() {
        let intake = env::temp_dir().join(format!(
            "fda-revoked-approval-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if intake.exists() {
            fs::remove_dir_all(&intake).unwrap();
        }

        start(&StartConfig {
            repo_root: PathBuf::from("."),
            input: StartInput::Goal("oshi-noteでVTuber紹介リンク/PRページを作りたい".to_string()),
            out: Some(intake.clone()),
            mode: IntakeMode::Auto,
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        for (decision_id, answer) in [
            ("HD-FDA-001", "yes"),
            ("HD-FDA-002", "accept"),
            ("HD-FDA-003", "confirm before design"),
        ] {
            decide(&DecideConfig {
                repo_root: PathBuf::from("."),
                artifact_dir: intake.clone(),
                decision_id: decision_id.to_string(),
                answer: answer.to_string(),
                decided_by: "test".to_string(),
                ato: AtoConfig::default(),
                print_json: false,
            })
            .unwrap();
        }
        assert_eq!(
            read_json_value(&intake.join("human_decision_packet.json"))
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("resolved")
        );

        decide(&DecideConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            decision_id: "HD-FDA-001".to_string(),
            answer: "revise".to_string(),
            decided_by: "test".to_string(),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let decision_packet = read_json_value(&intake.join("human_decision_packet.json")).unwrap();
        assert_eq!(
            decision_packet.get("status").and_then(Value::as_str),
            Some("waiting_human")
        );
        assert!(decision_packet.get("recorded_decision").is_none());

        fs::remove_dir_all(&intake).unwrap();
    }

    #[test]
    fn top_level_human_decision_packet_can_be_resolved() {
        let intake = env::temp_dir().join(format!(
            "fda-top-level-decision-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let design_out = env::temp_dir().join(format!(
            "fda-top-level-design-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        for path in [&intake, &design_out] {
            if path.exists() {
                fs::remove_dir_all(path).unwrap();
            }
        }
        fs::create_dir_all(&intake).unwrap();
        write_json_file(
            &intake.join("human_decision_packet.json"),
            &json!({
                "decision_packet_id": "HDP-FDA-TOP-001",
                "program_id": "FDA-V1",
                "epic_id": "EPIC-FDA-V1-INTAKE",
                "case_id": Value::Null,
                "status": "waiting_human",
                "required_before": "Design Gate",
                "decision_needed": "top-level の Human Decision Packet を採用してよいか",
                "trigger": "schema-valid top-level packet",
                "context": {
                    "current_state": "top-level decision only",
                    "relevant_requirement": "schema-compatible packet",
                    "relevant_evidence": ["human_decision_packet.json"]
                },
                "options": [
                    {
                        "id": "approve_top_level",
                        "description": "採用する",
                        "pros": ["Design Gateへ進める"],
                        "cons": ["修正要求は別途反映する"],
                        "recommended": true
                    },
                    {
                        "id": "revise_top_level",
                        "description": "修正する",
                        "pros": ["誤った前提を修正できる"],
                        "cons": ["Design Gateが遅れる"],
                        "recommended": false
                    }
                ],
                "impact": {
                    "scope": "scope fixed",
                    "security": "security reviewed later",
                    "schedule": "blocks design until resolved",
                    "ux": "CLI can show resume command",
                    "operations": "decision can be recorded"
                },
                "default_if_no_decision": "waiting_human",
                "forge_mapping": {
                    "claim_ids": ["CLM-FDA-TOP"],
                    "proof_obligations": ["decision recorded"],
                    "human_decision_points": ["HDP-FDA-TOP-001"],
                    "ato_task_graph": ["TASK-FDA-TOP"],
                    "planned_prs": ["PR-FDA-TOP"],
                    "gate_requirements": ["Design Gate"]
                }
            }),
        )
        .unwrap();

        let blocked = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            out: Some(design_out.clone()),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        assert_eq!(blocked.verdict, "blocked");
        assert_eq!(blocked.blocked_decisions.len(), 1);
        assert_eq!(blocked.blocked_decisions[0].decision_id, "HDP-FDA-TOP-001");

        decide(&DecideConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            decision_id: "HDP-FDA-TOP-001".to_string(),
            answer: "approve_top_level".to_string(),
            decided_by: "test".to_string(),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        let result = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            out: Some(design_out.clone()),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        assert_eq!(result.verdict, "pass");

        fs::remove_dir_all(&intake).unwrap();
        fs::remove_dir_all(&design_out).unwrap();
    }

    #[test]
    fn top_level_packet_accepts_valid_non_recommended_option() {
        let intake = env::temp_dir().join(format!(
            "fda-nonrecommended-option-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        let design_out = env::temp_dir().join(format!(
            "fda-nonrecommended-design-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        for path in [&intake, &design_out] {
            if path.exists() {
                fs::remove_dir_all(path).unwrap();
            }
        }
        fs::create_dir_all(&intake).unwrap();
        write_json_file(
            &intake.join("human_decision_packet.json"),
            &json!({
                "decision_packet_id": "HDPACKET-FD-001",
                "program_id": "PROGRAM-FORGE-DASHBOARD-001",
                "epic_id": "EPIC-FORGE-DASHBOARD-001",
                "case_id": "CASE-FD-002",
                "status": "waiting_human",
                "required_before": "Mission Control UI v0 implementation",
                "decision_needed": "Mission Control UI の最初の実装面を選ぶ。",
                "trigger": "UI 実装に着手する前",
                "context": {
                    "current_state": "UI実装前",
                    "relevant_requirement": "FR-002",
                    "relevant_evidence": ["requirements_definition.md"]
                },
                "options": [
                    {
                        "id": "A",
                        "description": "Web UI として実装する。",
                        "pros": ["レビューしやすい"],
                        "cons": ["デスクトップ統合は別途必要"],
                        "recommended": true
                    },
                    {
                        "id": "B",
                        "description": "Tauri UI として実装する。",
                        "pros": ["Windows desktop 運用に近い"],
                        "cons": ["初期検証のビルド負荷が高い"],
                        "recommended": false
                    }
                ],
                "impact": {
                    "scope": "UI 実装 Phase の対象技術が決まる。",
                    "security": "認証とローカルファイルアクセスの扱いが変わる。",
                    "schedule": "Web の方が初期検証は短い見込み。",
                    "ux": "Mission Control の密度と操作モデルを先に検証できる。",
                    "operations": "配布方法と observability の設計が変わる。"
                },
                "default_if_no_decision": "UI 実装には進まない。",
                "forge_mapping": {
                    "claim_ids": ["CLM-002"],
                    "proof_obligations": ["PRF-002"],
                    "human_decision_points": ["HDP-001"],
                    "ato_task_graph": ["TASK-FD-002"],
                    "planned_prs": [],
                    "gate_requirements": ["Human decision must be resolved before UI implementation starts"]
                }
            }),
        )
        .unwrap();

        let blocked = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            out: Some(design_out.clone()),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        assert_eq!(blocked.verdict, "blocked");
        assert_eq!(blocked.blocked_decisions[0].decision_id, "HDP-001");

        let decide_result = decide(&DecideConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            decision_id: "HDP-001".to_string(),
            answer: "B".to_string(),
            decided_by: "test".to_string(),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        assert_eq!(decide_result.packet_status, "resolved");

        let result = design(&DesignConfig {
            repo_root: PathBuf::from("."),
            artifact_dir: intake.clone(),
            out: Some(design_out.clone()),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();
        assert_eq!(result.verdict, "pass");

        fs::remove_dir_all(&intake).unwrap();
        fs::remove_dir_all(&design_out).unwrap();
    }

    #[test]
    fn fixture_plan_writes_required_outputs() {
        let out = env::temp_dir().join(format!(
            "fda-fixture-plan-test-{}-{}",
            std::process::id(),
            now_unix_seconds()
        ));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }

        let result = plan_fixture(&PlanConfig {
            repo_root: PathBuf::from("."),
            requirements: PathBuf::from(
                "docs/standards/delivery-artifacts-v0/examples/forge_dashboard_epic/requirements_definition.md",
            ),
            out: out.clone(),
            mode: PlanMode::Fixture,
            fixture_dir: PathBuf::from(DEFAULT_ARTIFACT_DIR),
            ato: AtoConfig::default(),
            print_json: false,
        })
        .unwrap();

        assert_eq!(result.verdict, "pass");
        for file_name in [
            "epic_delivery_plan.json",
            "case_graph.json",
            "task_graph.json",
            "autonomy_contract.json",
            "human_decision_packet.json",
            "artifact_inventory.json",
            "runner_explanation.json",
            "validation_report.json",
        ] {
            assert!(out.join(file_name).exists(), "{file_name} should exist");
        }

        fs::remove_dir_all(&out).unwrap();
    }
}
