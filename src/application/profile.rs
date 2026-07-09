use std::env;
use std::path::{Path, PathBuf};

use crate::application::ports::ArtifactStore;
use crate::application::validate::{
    validate_optional_repository_profile, validate_repository_profile,
    REPOSITORY_PROFILE_SCHEMA_DIR,
};
use crate::infra::json_schema::JsonSchemaArtifactValidator;
use crate::infra::yaml::SerdeYamlValidator;
use crate::support::paths::display_path;

pub(crate) const REPOSITORY_PROFILE_FILES: [&str; 7] = [
    "repo.yaml",
    "delivery_policy.yaml",
    "skills.lock",
    "agent_roles.yaml",
    "gates.yaml",
    "artifact_map.yaml",
    "notification.yaml",
];

pub(crate) fn ensure_repository_profile(
    store: &impl ArtifactStore,
    repo_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    let schema_repo_root = resolve_schema_repo_root(store, repo_root);
    ensure_repository_profile_with_schema_root(store, repo_root, &schema_repo_root)
}

fn ensure_repository_profile_with_schema_root(
    store: &impl ArtifactStore,
    repo_root: &Path,
    schema_repo_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    if !repo_root.is_dir() {
        return Err(format!(
            "FDA repository profile requires an existing repository directory: {}",
            repo_root.display()
        ));
    }

    let profile_dir = repo_root.join(".fda");
    store.create_dir_all(&profile_dir).map_err(|e| {
        format!(
            "failed to create FDA profile dir {}: {e}",
            profile_dir.display()
        )
    })?;

    let repo_name = repo_name(repo_root);
    let stack = detect_stack(store, repo_root);
    let mut created = Vec::new();
    for file_name in REPOSITORY_PROFILE_FILES {
        let path = profile_dir.join(file_name);
        if store.exists(&path) {
            continue;
        }
        store.write_text(&path, &profile_file_body(file_name, &repo_name, &stack)?)?;
        created.push(path);
    }
    validate_profile(store, repo_root, schema_repo_root)?;
    Ok(created)
}

pub(crate) fn ensure_target_repository_profile_if_present(
    store: &impl ArtifactStore,
    target_repo: &Path,
    schema_repo_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    if target_repo.is_dir() {
        ensure_repository_profile_with_schema_root(store, target_repo, schema_repo_root)
    } else {
        Ok(Vec::new())
    }
}

fn validate_profile(
    store: &impl ArtifactStore,
    repo_root: &Path,
    schema_repo_root: &Path,
) -> Result<(), String> {
    let profile_dir = repo_root.join(".fda");
    let schema_dir = schema_repo_root.join(REPOSITORY_PROFILE_SCHEMA_DIR);
    let mut checks = validate_repository_profile(
        store,
        &JsonSchemaArtifactValidator,
        &SerdeYamlValidator,
        repo_root,
        &profile_dir,
        &schema_dir,
    );
    // 任意ファイル（.fda/delegation_contract.yaml 等）は存在すれば検証する。
    checks.extend(validate_optional_repository_profile(
        store,
        &JsonSchemaArtifactValidator,
        &SerdeYamlValidator,
        repo_root,
        &profile_dir,
        &schema_dir,
    ));
    let failures = checks
        .iter()
        .filter(|check| check.status == "fail")
        .flat_map(|check| {
            if check.errors.is_empty() {
                vec![format!("{} failed", check.check_id)]
            } else {
                check
                    .errors
                    .iter()
                    .map(|error| format!("{}: {}", check.check_id, error.message))
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        return Ok(());
    }
    Err(format!(
        "FDA repository profile validation failed for {}:\n- {}",
        display_path(repo_root, &profile_dir),
        failures.join("\n- ")
    ))
}

fn resolve_schema_repo_root(store: &impl ArtifactStore, repo_root: &Path) -> PathBuf {
    for variable in ["FDA_SCHEMA_REPO_ROOT", "FDA_REPO_ROOT"] {
        if let Some(candidate) = env::var_os(variable).map(PathBuf::from) {
            if has_repository_profile_schemas(store, &candidate) {
                return candidate;
            }
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if has_repository_profile_schemas(store, &current_dir) {
            return current_dir;
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if has_repository_profile_schemas(store, &manifest_dir) {
        return manifest_dir;
    }

    repo_root.to_path_buf()
}

fn has_repository_profile_schemas(store: &impl ArtifactStore, candidate: &Path) -> bool {
    store.exists(&candidate.join(REPOSITORY_PROFILE_SCHEMA_DIR))
}

fn repo_name(repo_root: &Path) -> String {
    let raw_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository");
    let sanitized = raw_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "repository".to_string()
    } else {
        sanitized
    }
}

struct StackDefaults {
    language: &'static str,
    framework: Option<&'static str>,
    package_manager: Option<&'static str>,
    commands: &'static [(&'static str, &'static str)],
}

fn detect_stack(store: &impl ArtifactStore, repo_root: &Path) -> StackDefaults {
    if store.exists(&repo_root.join("Cargo.toml")) {
        return StackDefaults {
            language: "rust",
            framework: Some("cli"),
            package_manager: Some("cargo"),
            commands: &[
                ("test", "cargo test"),
                ("check", "cargo check"),
                ("fmt", "cargo fmt --all"),
            ],
        };
    }
    if store.exists(&repo_root.join("package.json")) {
        return StackDefaults {
            language: "typescript",
            framework: None,
            package_manager: Some("npm"),
            commands: &[("test", "npm test"), ("check", "npm run lint")],
        };
    }
    StackDefaults {
        language: "unknown",
        framework: None,
        package_manager: None,
        commands: &[("check", "echo \"No FDA check command configured\"")],
    }
}

fn profile_file_body(
    file_name: &str,
    repo_name: &str,
    stack: &StackDefaults,
) -> Result<String, String> {
    match file_name {
        "repo.yaml" => Ok(repo_yaml(repo_name, stack)),
        "delivery_policy.yaml" => Ok(DELIVERY_POLICY_YAML.to_string()),
        "skills.lock" => Ok(SKILLS_LOCK_YAML.to_string()),
        "agent_roles.yaml" => Ok(AGENT_ROLES_YAML.to_string()),
        "gates.yaml" => Ok(GATES_YAML.to_string()),
        "artifact_map.yaml" => Ok(ARTIFACT_MAP_YAML.to_string()),
        "notification.yaml" => Ok(NOTIFICATION_YAML.to_string()),
        other => Err(format!("unknown FDA profile file `{other}`")),
    }
}

fn repo_yaml(repo_name: &str, stack: &StackDefaults) -> String {
    let mut body = format!(
        "repo:\n  id: {}\n  name: {}\n  default_branch: {}\n  stack:\n    language: {}\n",
        yaml_string(repo_name),
        yaml_string(repo_name),
        yaml_string("main"),
        yaml_string(stack.language)
    );
    if let Some(framework) = stack.framework {
        body.push_str(&format!("    framework: {}\n", yaml_string(framework)));
    }
    if let Some(package_manager) = stack.package_manager {
        body.push_str(&format!(
            "    package_manager: {}\n",
            yaml_string(package_manager)
        ));
    }
    body.push_str(
        "  risk_profile:\n    repository_profile_required: true\n    human_decision_sensitive: true\n  commands:\n",
    );
    for (name, command) in stack.commands {
        body.push_str(&format!("    {name}: {}\n", yaml_string(command)));
    }
    body
}

fn yaml_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

const DELIVERY_POLICY_YAML: &str = r#"delivery_policy:
  default_autonomy_level: pr_open_allowed
  auto_merge_allowed: false
  human_required_for:
    - scope_change
    - privacy_policy_change
    - terms_change
    - legal_judgment
    - security_boundary_change
    - security_high_or_critical
    - user_data_exposure
    - public_api_breaking_change
    - risk_approval
    - merge_approval
    - release_approval
    - precedent_conflict
  low_risk_paths:
    - docs/**
    - tests/**
    - .fda/**
    - artifacts/runs/**
  forbidden_without_human:
    - database_destructive_migration
    - raw_user_log_exposure
    - third_party_data_export
    - merge_approval
    - release_approval
"#;

const SKILLS_LOCK_YAML: &str = r#"skills:
  requirement_to_epic_plan: v0.3.0
  epic_to_task_graph: v0.1.0
  epic_to_planned_prs: v0.2.0
  forge_projection: v0.1.0
  external_implementation_handoff: v0.1.0
  external_pr_receipt_collect: v0.1.0
  ato_cli_materialization: v0.1.0
  human_decision_triage: v0.2.0
  proof_gap_detection: v0.1.0
"#;

const AGENT_ROLES_YAML: &str = r#"agent_roles:
  default_orchestrator:
    executor: current_codex_cli
    workspace_policy: workspace_write
    can_edit: true
    required_checkpoint: true
  implementer:
    executor: current_codex_cli
    workspace_policy: workspace_write
    can_edit: true
    role_switch_allowed: true
    required_handoff: implementation_handoff.md
    required_checkpoint: true
  pr_reviewer:
    executor: codex_subagent
    workspace_policy: read_only
    can_edit: false
    required_for_pr: true
  functional_qa:
    executor: codex_subagent
    workspace_policy: read_only
    can_edit: false
    required_for_pr: true
  security_qa:
    executor: codex_subagent
    workspace_policy: read_only
    can_edit: false
    required_for_pr: true
  forge_reviewer:
    executor: codex_subagent
    workspace_policy: read_only
    can_edit: false
    required_when:
      - ato_forge_fda_evidence_changed
      - handoff_changed
      - review_packet_changed
      - human_decision_boundary_changed
    fallback_role: qax2
  design_qa:
    executor: codex_subagent
    workspace_policy: read_only
    can_edit: false
    required_when:
      - ui_surface_changed
      - frontend_changed
      - browser_surface_changed
    not_applicable_requires_reason: true
  merge_manager:
    executor: current_codex_cli
    workspace_policy: controlled_write
    can_edit: false
    requires_human_approval: true
"#;

const GATES_YAML: &str = r#"gates:
  pre_work_profile_gate:
    required: true
    description: FDA target repository must have a .fda profile before work starts.
    required_files:
      - .fda/repo.yaml
      - .fda/delivery_policy.yaml
      - .fda/skills.lock
      - .fda/agent_roles.yaml
      - .fda/gates.yaml
      - .fda/artifact_map.yaml
      - .fda/notification.yaml
    if_missing: create_before_work
  human_decision_gate:
    required: true
    blocks:
      - unresolved_scope_change
      - unresolved_security_high_or_critical
      - unresolved_privacy_or_legal
      - unresolved_merge_approval
      - unresolved_release_approval
  design_gate:
    required_before:
      - implementation
    required_artifacts:
      - requirements_definition.md
      - human_decision_packet.md
      - basic_design.md
      - detailed_design.md
      - planned_prs.json
  review_agent_gate:
    required_before:
      - pr_ready
      - merge
    required_reviewers:
      - pr_reviewer
      - functional_qa
      - security_qa
    conditional_reviewers:
      - forge_reviewer
      - design_qa
    check_command: python3 scripts/check_review_agent_gate.py --pr-number <PR_NUMBER>
  merge_gate:
    auto_merge_allowed: false
    requires_human_approval: true
    fail_closed_on:
      - missing_review_agent_gate
      - unresolved_human_decision
      - missing_current_test_evidence
      - missing_forge_evidence
      - security_high_or_critical
"#;

const ARTIFACT_MAP_YAML: &str = r#"artifact_map:
  run_root: artifacts/runs
  review_packet_root: artifacts/review_packets
  handoff_root: handoffs/records
  sandbox_root: artifacts/sandbox
  docs:
    v1_architecture: docs/standards/fda-v1/architecture.md
    codex_cli_primary_architecture: docs/v1/codex_cli_primary_architecture.md
    repository_profile: docs/standards/fda-v1/repository_profile.md
    output_hub: docs/standards/fda-v1/output_hub.md
  standard_run_artifacts:
    - requirements_definition.md
    - human_decision_packet.md
    - basic_design.md
    - detailed_design.md
    - planned_prs.json
    - implementation_handoff.md
    - review_receipt.json
    - merge_receipt.json
    - output_hub.html
    - status.json
  evidence_policy:
    store_raw_stdout_stderr: false
    prefer_artifact_links: true
    require_verdict: true
    require_freshness: true
"#;

const NOTIFICATION_YAML: &str = r#"notification:
  default_channels:
    - cli
    - local_output_hub
  human_turn_channels:
    - cli
    - slack
  slack:
    enabled: true
    credential_source: environment
    webhook_url_env: FDA_SLACK_WEBHOOK_URL
    channel_label_env: FDA_SLACK_CHANNEL_LABEL
    allowed_domains:
      - hooks.slack.com
    store_secret_values: false
  email:
    enabled: false
    credential_source: environment
    required_env:
      - FDA_SMTP_HOST
      - FDA_SMTP_PORT
      - FDA_SMTP_USERNAME
      - FDA_SMTP_PASSWORD
      - FDA_SMTP_FROM
    recipient_env: FDA_NOTIFY_EMAIL
    store_secret_values: false
  output_hub:
    enabled: true
    default_artifact: output_hub.html
  fail_closed_on:
    - missing_required_decision_payload
    - missing_resume_command
"#;
