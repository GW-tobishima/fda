use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::application::decisions::{value_string, value_string_array};
use crate::application::ports::{
    ArtifactStore, ArtifactValidator, CheckError, Clock, YamlValidator,
};
use crate::cli::args::ValidateConfig;
use crate::domain::entities::{TraceCase, TraceClaim, TracePlan, TracePlannedPr, TraceProof};
use crate::domain::policies::trace_links::validate_case_pr_links as validate_trace_case_pr_links;
use crate::infra::clock::SystemClock;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::json_schema::{read_json, JsonSchemaArtifactValidator};
use crate::infra::yaml::{validate_yaml_dir, SerdeYamlValidator};
use crate::support::paths::{display_path, resolve_path};

pub(crate) const REPOSITORY_PROFILE_SCHEMA_DIR: &str =
    "docs/standards/fda-v1/schemas/repository-profile";
const REPOSITORY_PROFILE_FILES: [(&str, &str); 7] = [
    ("repo.yaml", "repo_yaml.schema.json"),
    ("delivery_policy.yaml", "delivery_policy_yaml.schema.json"),
    ("skills.lock", "skills_lock_yaml.schema.json"),
    ("agent_roles.yaml", "agent_roles_yaml.schema.json"),
    ("gates.yaml", "gates_yaml.schema.json"),
    ("artifact_map.yaml", "artifact_map_yaml.schema.json"),
    ("notification.yaml", "notification_yaml.schema.json"),
];

#[derive(Serialize)]
pub(crate) struct ValidationReport {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) generated_at_unix_seconds: u64,
    pub(crate) repo_root: String,
    pub(crate) schema_dir: String,
    pub(crate) artifact_dir: String,
    pub(crate) summary: ValidationSummary,
    pub(crate) checks: Vec<ValidationCheck>,
}

#[derive(Default, Serialize)]
pub(crate) struct ValidationSummary {
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
}

#[derive(Serialize)]
pub(crate) struct ValidationCheck {
    pub(crate) check_id: String,
    pub(crate) kind: &'static str,
    pub(crate) status: &'static str,
    pub(crate) schema_path: Option<String>,
    pub(crate) artifact_path: Option<String>,
    pub(crate) errors: Vec<CheckError>,
}

pub(crate) fn validate(config: &ValidateConfig) -> Result<ValidationReport, String> {
    let report = validate_artifacts(config)?;
    if let Some(out) = &config.out {
        write_report(out, &report)?;
    }
    Ok(report)
}

pub(crate) fn validate_artifacts(config: &ValidateConfig) -> Result<ValidationReport, String> {
    let store = FsArtifactStore;
    let validator = JsonSchemaArtifactValidator;
    let yaml_validator = SerdeYamlValidator;
    let clock = SystemClock;
    validate_artifacts_with_ports(config, &store, &validator, &yaml_validator, &clock)
}

pub(crate) fn validate_artifacts_with_ports(
    config: &ValidateConfig,
    store: &impl ArtifactStore,
    validator: &impl ArtifactValidator,
    yaml_validator: &impl YamlValidator,
    clock: &impl Clock,
) -> Result<ValidationReport, String> {
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    let schema_dir = resolve_path(&repo_root, &config.schema_dir);
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);

    let mut checks = Vec::new();
    for schema_path in store.schema_files(&schema_dir)? {
        let artifact_name = artifact_name_from_schema(&schema_path)?;
        let artifact_path = artifact_dir.join(format!("{artifact_name}.json"));
        let schema_display = display_path(&repo_root, &schema_path);
        let artifact_display = display_path(&repo_root, &artifact_path);
        let schema_json = match read_json(store, &schema_path) {
            Ok(value) => value,
            Err(error) => {
                checks.push(failed_check(
                    format!("schema:{artifact_name}"),
                    "schema_compile",
                    Some(schema_display),
                    None,
                    error,
                ));
                continue;
            }
        };

        match validator.compile_schema(&schema_json) {
            Ok(()) => {
                checks.push(passed_check(
                    format!("schema:{artifact_name}"),
                    "schema_compile",
                    Some(schema_display.clone()),
                    None,
                ));
            }
            Err(error) => {
                checks.push(failed_check(
                    format!("schema:{artifact_name}"),
                    "schema_compile",
                    Some(schema_display),
                    None,
                    error,
                ));
                continue;
            }
        };

        if !store.exists(&artifact_path) {
            checks.push(skipped_check(
                format!("artifact:{artifact_name}"),
                "json_schema_validation",
                Some(schema_display),
                Some(artifact_display),
                format!("no matching example JSON for schema `{artifact_name}`"),
            ));
            continue;
        }

        let artifact_json = match read_json(store, &artifact_path) {
            Ok(value) => value,
            Err(error) => {
                checks.push(failed_check(
                    format!("artifact:{artifact_name}"),
                    "json_schema_validation",
                    Some(schema_display),
                    Some(artifact_display),
                    error,
                ));
                continue;
            }
        };

        let errors = match validator.validate_json_schema(&schema_json, &artifact_json) {
            Ok(errors) => errors,
            Err(error) => {
                checks.push(failed_check(
                    format!("artifact:{artifact_name}"),
                    "json_schema_validation",
                    Some(schema_display),
                    Some(artifact_display),
                    error,
                ));
                continue;
            }
        };
        if errors.is_empty() {
            checks.push(passed_check(
                format!("artifact:{artifact_name}"),
                "json_schema_validation",
                Some(schema_display),
                Some(artifact_display),
            ));
        } else {
            checks.push(ValidationCheck {
                check_id: format!("artifact:{artifact_name}"),
                kind: "json_schema_validation",
                status: "fail",
                schema_path: Some(schema_display),
                artifact_path: Some(artifact_display),
                errors,
            });
        }
    }

    let epic_path = artifact_dir.join("epic_delivery_plan.json");
    if store.exists(&epic_path) {
        checks.push(validate_epic_delivery_plan(store, &repo_root, &epic_path));
    }
    for dir in &config.model_contract_dirs {
        checks.extend(validate_yaml_dir(
            store,
            yaml_validator,
            &repo_root,
            &resolve_path(&repo_root, dir),
        )?);
    }
    checks.extend(validate_repository_profile(
        store,
        validator,
        yaml_validator,
        &repo_root,
        &repo_root.join(".fda"),
        &repo_root.join(REPOSITORY_PROFILE_SCHEMA_DIR),
    ));

    let summary = summarize(&checks);
    let verdict = if summary.failed == 0 { "pass" } else { "fail" };

    Ok(ValidationReport {
        schema_version: "fda.validation_report.v0",
        verdict,
        generated_at_unix_seconds: clock.now_unix_seconds(),
        repo_root: display_path(&repo_root, &repo_root),
        schema_dir: display_path(&repo_root, &schema_dir),
        artifact_dir: display_path(&repo_root, &artifact_dir),
        summary,
        checks,
    })
}

pub(crate) fn validate_repository_profile(
    store: &impl ArtifactStore,
    validator: &impl ArtifactValidator,
    yaml_validator: &impl YamlValidator,
    repo_root: &Path,
    profile_dir: &Path,
    profile_schema_dir: &Path,
) -> Vec<ValidationCheck> {
    REPOSITORY_PROFILE_FILES
        .into_iter()
        .map(|(profile_file, schema_file)| {
            validate_repository_profile_file(
                store,
                validator,
                yaml_validator,
                repo_root,
                &profile_dir.join(profile_file),
                &profile_schema_dir.join(schema_file),
            )
        })
        .collect()
}

fn validate_repository_profile_file(
    store: &impl ArtifactStore,
    validator: &impl ArtifactValidator,
    yaml_validator: &impl YamlValidator,
    repo_root: &Path,
    profile_path: &Path,
    schema_path: &Path,
) -> ValidationCheck {
    let artifact_display = display_path(repo_root, profile_path);
    let schema_display = display_path(repo_root, schema_path);
    let check_id = format!("repository_profile:{artifact_display}");
    if !store.exists(profile_path) {
        return failed_check(
            check_id,
            "repository_profile_validation",
            Some(schema_display),
            Some(artifact_display),
            CheckError {
                message: "required .fda profile file is missing".to_string(),
                instance_path: None,
                schema_path: None,
            },
        );
    }

    let schema_json = match read_json(store, schema_path) {
        Ok(value) => value,
        Err(error) => {
            return failed_check(
                check_id,
                "repository_profile_schema",
                Some(schema_display),
                Some(artifact_display),
                error,
            );
        }
    };
    if let Err(error) = validator.compile_schema(&schema_json) {
        return failed_check(
            check_id,
            "repository_profile_schema",
            Some(schema_display),
            Some(artifact_display),
            error,
        );
    }

    let body = match store.read_text(profile_path) {
        Ok(body) => body,
        Err(message) => {
            return failed_check(
                check_id,
                "repository_profile_validation",
                Some(schema_display),
                Some(artifact_display),
                CheckError {
                    message,
                    instance_path: None,
                    schema_path: None,
                },
            );
        }
    };
    let profile_json = match yaml_validator.parse_yaml_value(profile_path, &body) {
        Ok(value) => value,
        Err(message) => {
            return failed_check(
                check_id,
                "repository_profile_validation",
                Some(schema_display),
                Some(artifact_display),
                CheckError {
                    message,
                    instance_path: None,
                    schema_path: None,
                },
            );
        }
    };

    match validator.validate_json_schema(&schema_json, &profile_json) {
        Ok(errors) if errors.is_empty() => passed_check(
            check_id,
            "repository_profile_validation",
            Some(schema_display),
            Some(artifact_display),
        ),
        Ok(errors) => ValidationCheck {
            check_id,
            kind: "repository_profile_validation",
            status: "fail",
            schema_path: Some(schema_display),
            artifact_path: Some(artifact_display),
            errors,
        },
        Err(error) => failed_check(
            check_id,
            "repository_profile_validation",
            Some(schema_display),
            Some(artifact_display),
            error,
        ),
    }
}

pub(crate) fn write_report(out: &Path, report: &ValidationReport) -> Result<(), String> {
    let store = FsArtifactStore;
    write_report_with_store(&store, out, report)
}

pub(crate) fn write_report_with_store(
    store: &impl ArtifactStore,
    out: &Path,
    report: &ValidationReport,
) -> Result<(), String> {
    if let Some(parent) = out.parent() {
        store
            .create_dir_all(parent)
            .map_err(|e| format!("failed to create report dir {}: {e}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    store
        .write_text(out, &format!("{body}\n"))
        .map_err(|e| format!("failed to write report {}: {e}", out.display()))
}

pub(crate) fn artifact_name_from_schema(schema_path: &Path) -> Result<String, String> {
    let file_name = schema_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid schema path {}", schema_path.display()))?;
    file_name
        .strip_suffix(".schema.json")
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("schema file does not end with .schema.json: {file_name}"))
}

fn validate_epic_delivery_plan(
    store: &impl ArtifactStore,
    repo_root: &Path,
    epic_path: &Path,
) -> ValidationCheck {
    let artifact_display = display_path(repo_root, epic_path);
    let epic_json = match read_json(store, epic_path) {
        Ok(value) => value,
        Err(error) => {
            return failed_check(
                "artifact:epic_delivery_plan:trace_links".to_string(),
                "epic_delivery_plan_trace_links",
                None,
                Some(artifact_display),
                error,
            );
        }
    };

    let errors: Vec<CheckError> = validate_case_pr_links(&epic_json)
        .into_iter()
        .map(|message| CheckError {
            message,
            instance_path: None,
            schema_path: None,
        })
        .collect();

    if errors.is_empty() {
        passed_check(
            "artifact:epic_delivery_plan:trace_links".to_string(),
            "epic_delivery_plan_trace_links",
            None,
            Some(artifact_display),
        )
    } else {
        ValidationCheck {
            check_id: "artifact:epic_delivery_plan:trace_links".to_string(),
            kind: "epic_delivery_plan_trace_links",
            status: "fail",
            schema_path: None,
            artifact_path: Some(artifact_display),
            errors,
        }
    }
}

pub(crate) fn validate_case_pr_links(plan: &Value) -> Vec<String> {
    validate_trace_case_pr_links(&trace_plan_from_value(plan))
}

fn trace_plan_from_value(plan: &Value) -> TracePlan {
    TracePlan {
        status: value_string(plan, "status").unwrap_or_default(),
        cases: plan
            .get("case_graph")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|case| TraceCase {
                case_id: value_string(case, "case_id").unwrap_or_else(|| "<missing>".to_string()),
                depends_on: value_string_array(case, "depends_on"),
                claim_ids: value_string_array(case, "claim_ids"),
                planned_pr: value_string(case, "planned_pr"),
            })
            .collect(),
        claims: plan
            .get("claim_tree")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|claim| value_string(claim, "claim_id"))
            .map(|claim_id| TraceClaim { claim_id })
            .collect(),
        planned_prs: plan
            .get("pr_plan")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|pr| {
                value_string(pr, "planned_pr_id").map(|planned_pr_id| TracePlannedPr {
                    planned_pr_id,
                    case_id: value_string(pr, "case_id"),
                })
            })
            .collect(),
        proofs: plan
            .get("proof_strategy")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|proof| value_string(proof, "claim_id"))
            .map(|claim_id| TraceProof { claim_id })
            .collect(),
    }
}

pub(crate) fn passed_check(
    check_id: String,
    kind: &'static str,
    schema_path: Option<String>,
    artifact_path: Option<String>,
) -> ValidationCheck {
    ValidationCheck {
        check_id,
        kind,
        status: "pass",
        schema_path,
        artifact_path,
        errors: Vec::new(),
    }
}

pub(crate) fn failed_check(
    check_id: String,
    kind: &'static str,
    schema_path: Option<String>,
    artifact_path: Option<String>,
    error: CheckError,
) -> ValidationCheck {
    ValidationCheck {
        check_id,
        kind,
        status: "fail",
        schema_path,
        artifact_path,
        errors: vec![error],
    }
}

pub(crate) fn skipped_check(
    check_id: String,
    kind: &'static str,
    schema_path: Option<String>,
    artifact_path: Option<String>,
    reason: String,
) -> ValidationCheck {
    ValidationCheck {
        check_id,
        kind,
        status: "skipped",
        schema_path,
        artifact_path,
        errors: vec![CheckError {
            message: reason,
            instance_path: None,
            schema_path: None,
        }],
    }
}

fn summarize(checks: &[ValidationCheck]) -> ValidationSummary {
    let mut summary = ValidationSummary::default();
    for check in checks {
        match check.status {
            "pass" => summary.passed += 1,
            "fail" => summary.failed += 1,
            "skipped" => summary.skipped += 1,
            _ => {}
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn repository_profile_validation_passes_for_repo_profile() {
        let repo_root = env::current_dir().unwrap();
        let checks = validate_repository_profile(
            &FsArtifactStore,
            &JsonSchemaArtifactValidator,
            &SerdeYamlValidator,
            &repo_root,
            &repo_root.join(".fda"),
            &repo_root.join(REPOSITORY_PROFILE_SCHEMA_DIR),
        );

        assert_eq!(checks.len(), REPOSITORY_PROFILE_FILES.len());
        assert!(checks.iter().all(|check| check.status == "pass"));
    }

    #[test]
    fn repository_profile_validation_fails_when_profile_is_missing() {
        let repo_root = env::current_dir().unwrap();
        let missing_profile_dir = env::temp_dir().join("fda-missing-profile");
        let checks = validate_repository_profile(
            &FsArtifactStore,
            &JsonSchemaArtifactValidator,
            &SerdeYamlValidator,
            &repo_root,
            &missing_profile_dir,
            &repo_root.join(REPOSITORY_PROFILE_SCHEMA_DIR),
        );

        assert_eq!(checks.len(), REPOSITORY_PROFILE_FILES.len());
        assert!(checks.iter().all(|check| check.status == "fail"));
        assert!(checks
            .iter()
            .all(|check| check.kind == "repository_profile_validation"));
    }
}
