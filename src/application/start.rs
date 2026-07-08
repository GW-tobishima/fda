use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::application::ports::{ArtifactStore, Clock};
use crate::application::profile::ensure_repository_profile;
use crate::application::validate::{validate_artifacts, write_report_with_store};
use crate::cli::args::{AtoConfig, StartConfig, StartInput, ValidateConfig};
use crate::domain::entities::{HumanDecisionSummary, IntakeInput, RuntimeContext};
use crate::domain::policies::intake::{classify_intake, intake_decisions, intake_mode_name};
use crate::infra::clock::SystemClock;
use crate::infra::fs_store::FsArtifactStore;
use crate::rendering::intake::{
    human_decision_packet_json, human_decision_packet_markdown,
    non_functional_requirements_markdown, non_implementation_artifacts,
    requirements_definition_markdown, risk_register_markdown, start_runner_explanation,
};
use crate::rendering::inventory::start_artifact_inventory;
use crate::support::paths::{display_path, resolve_path};
use crate::{DEFAULT_MODEL_CONTRACT_DIRS, DEFAULT_SCHEMA_DIR};

#[derive(Serialize)]
pub(crate) struct StartResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) mode: String,
    pub(crate) implementation_classification: String,
    pub(crate) input_source: String,
    pub(crate) out_dir: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) human_decisions: Vec<HumanDecisionSummary>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) validation_report_path: String,
}

pub(crate) fn start(config: &StartConfig) -> Result<StartResult, String> {
    let store = FsArtifactStore;
    let clock = SystemClock;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    ensure_repository_profile(&store, &repo_root)?;
    let input = read_start_input(&store, &repo_root, &config.input)?;
    let out_dir = match &config.out {
        Some(out) => resolve_path(&repo_root, out),
        None => repo_root
            .join("artifacts")
            .join("runs")
            .join(format!("fda-start-{}", clock.now_unix_seconds())),
    };
    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;

    let classification = classify_intake(config.mode, &input.body);
    let context = RuntimeContext::for_v1_intake();
    let decisions = intake_decisions(&classification);

    let mut artifacts_written = Vec::new();
    store.write_text(
        &out_dir.join("requirements_definition.md"),
        &requirements_definition_markdown(&input, &classification, &decisions),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("requirements_definition.md"),
    ));

    store.write_text(
        &out_dir.join("non_functional_requirements.md"),
        &non_functional_requirements_markdown(&input, &classification),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("non_functional_requirements.md"),
    ));

    store.write_text(
        &out_dir.join("risk_register.md"),
        &risk_register_markdown(&classification, &decisions),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("risk_register.md")));

    store.write_text(
        &out_dir.join("human_decision_packet.md"),
        &human_decision_packet_markdown(&decisions),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("human_decision_packet.md"),
    ));

    store.write_json(
        &out_dir.join("human_decision_packet.json"),
        &human_decision_packet_json(&input, &classification, &decisions, &context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("human_decision_packet.json"),
    ));

    let non_implementation_outputs =
        non_implementation_artifacts(&input, &classification, &decisions);
    for (file_name, body) in &non_implementation_outputs {
        store.write_text(&out_dir.join(file_name), body)?;
        artifacts_written.push(display_path(&repo_root, &out_dir.join(file_name)));
    }

    store.write_json(
        &out_dir.join("runner_explanation.json"),
        &start_runner_explanation(&repo_root, &input, &out_dir, &classification, &context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    store.write_json(
        &out_dir.join("artifact_inventory.json"),
        &start_artifact_inventory(&repo_root, &out_dir, &context, classification.mode),
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

    Ok(StartResult {
        schema_version: "fda.start_result.v0",
        verdict: if validation_report.verdict == "pass" {
            "pass"
        } else {
            "fail"
        },
        mode: intake_mode_name(config.mode).to_string(),
        implementation_classification: classification.name.to_string(),
        input_source: input.source,
        out_dir: display_path(&repo_root, &out_dir),
        artifacts_written,
        next_actions: start_next_actions(&repo_root, &out_dir, &decisions),
        human_decisions: decisions,
        validation_report_path: display_path(&repo_root, &validation_report_path),
    })
}

fn start_next_actions(
    repo_root: &Path,
    out_dir: &Path,
    decisions: &[HumanDecisionSummary],
) -> Vec<String> {
    let artifact_dir = display_path(repo_root, out_dir);
    let mut actions = decisions
        .iter()
        .map(|decision| {
            format!(
                "fda decide {} --answer <answer> --artifacts {}",
                decision.decision_id, artifact_dir
            )
        })
        .collect::<Vec<_>>();
    actions.push(format!("fda design --artifacts {artifact_dir}"));
    actions
}

fn read_start_input(
    store: &impl ArtifactStore,
    repo_root: &Path,
    input: &StartInput,
) -> Result<IntakeInput, String> {
    match input {
        StartInput::Goal(goal) => Ok(IntakeInput {
            source: "cli_goal".to_string(),
            body: goal.trim().to_string(),
        }),
        StartInput::File(path) => {
            let path = resolve_path(repo_root, path);
            let body = store
                .read_text(&path)
                .map_err(|e| format!("failed to read input file {}: {e}", path.display()))?;
            Ok(IntakeInput {
                source: display_path(repo_root, &path),
                body: body.trim().to_string(),
            })
        }
    }
}
