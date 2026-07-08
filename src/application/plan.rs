use std::path::PathBuf;

use serde::Serialize;

use crate::application::ports::ArtifactStore;
use crate::application::profile::ensure_repository_profile;
use crate::application::runtime::runtime_context_from_output_store;
use crate::application::validate::{validate_artifacts, write_report_with_store};
use crate::cli::args::{AtoConfig, PlanConfig, PlanMode, ValidateConfig};
use crate::infra::fs_store::FsArtifactStore;
use crate::rendering::inventory::{artifact_inventory, runner_explanation};
use crate::support::paths::{display_path, resolve_path};
use crate::{DEFAULT_MODEL_CONTRACT_DIRS, DEFAULT_SCHEMA_DIR};

#[derive(Serialize)]
pub(crate) struct PlanResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) mode: &'static str,
    pub(crate) requirements_path: String,
    pub(crate) out_dir: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) validation_report_path: String,
}

pub(crate) fn plan(config: &PlanConfig) -> Result<PlanResult, String> {
    match config.mode {
        PlanMode::Fixture => plan_fixture(config),
        PlanMode::Model => Err("plan --mode model is not implemented yet".to_string()),
    }
}

pub(crate) fn plan_fixture(config: &PlanConfig) -> Result<PlanResult, String> {
    let store = FsArtifactStore;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    ensure_repository_profile(&store, &repo_root)?;
    let requirements_path = resolve_path(&repo_root, &config.requirements);
    if !store.exists(&requirements_path) {
        return Err(format!(
            "requirements file does not exist: {}",
            requirements_path.display()
        ));
    }
    let fixture_dir = resolve_path(&repo_root, &config.fixture_dir);
    if !store.exists(&fixture_dir) {
        return Err(format!(
            "fixture directory does not exist: {}",
            fixture_dir.display()
        ));
    }
    let out_dir = resolve_path(&repo_root, &config.out);
    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;

    let mut artifacts_written = Vec::new();
    for artifact in [
        "epic_delivery_plan.json",
        "case_graph.json",
        "task_graph.json",
        "autonomy_contract.json",
        "human_decision_packet.json",
    ] {
        let src = fixture_dir.join(artifact);
        let dst = out_dir.join(artifact);
        store.copy(&src, &dst)?;
        artifacts_written.push(display_path(&repo_root, &out_dir.join(artifact)));
    }

    let runtime_context = runtime_context_from_output_store(&store, &out_dir)?;
    store.write_json(
        &out_dir.join("runner_explanation.json"),
        &runner_explanation(&repo_root, &requirements_path, &out_dir, &runtime_context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    store.write_json(
        &out_dir.join("artifact_inventory.json"),
        &artifact_inventory(&repo_root, &out_dir, &runtime_context),
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
    write_report_with_store(&store, &validation_report_path, &validation_report)?;
    artifacts_written.push(display_path(&repo_root, &validation_report_path));

    Ok(PlanResult {
        schema_version: "fda.plan_result.v0",
        verdict: if validation_report.verdict == "pass" {
            "pass"
        } else {
            "fail"
        },
        mode: "fixture",
        requirements_path: display_path(&repo_root, &requirements_path),
        out_dir: display_path(&repo_root, &out_dir),
        artifacts_written,
        validation_report_path: display_path(&repo_root, &validation_report_path),
    })
}
