use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    read_json_value, value_string,
};
use crate::application::ports::{ArtifactStore, Clock};
use crate::application::profile::ensure_repository_profile;
use crate::application::validate::{validate_artifacts, write_report_with_store};
use crate::cli::args::{AtoConfig, DesignConfig, ValidateConfig};
use crate::domain::entities::{HumanDecisionSummary, RuntimeContext};
use crate::domain::policies::decision::decision_blockers;
use crate::infra::clock::SystemClock;
use crate::infra::fs_store::FsArtifactStore;
use crate::rendering::design::{
    basic_design_markdown, design_autonomy_contract, design_case_graph, design_planned_prs,
    design_runner_explanation, design_task_graph, detailed_design_markdown,
    functional_qa_brief_markdown, security_qa_brief_markdown,
};
use crate::rendering::forge::design_forge_projection;
use crate::rendering::inventory::design_artifact_inventory;
use crate::support::paths::{display_path, resolve_path};
use crate::{DEFAULT_MODEL_CONTRACT_DIRS, DEFAULT_SCHEMA_DIR};

#[derive(Serialize)]
pub(crate) struct DesignResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) design_gate_status: &'static str,
    pub(crate) artifact_dir: String,
    pub(crate) out_dir: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) blocked_decisions: Vec<HumanDecisionSummary>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) validation_report_path: Option<String>,
}

pub(crate) fn design(config: &DesignConfig) -> Result<DesignResult, String> {
    let store = FsArtifactStore;
    let clock = SystemClock;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    ensure_repository_profile(&store, &repo_root)?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let out_dir = match &config.out {
        Some(out) => resolve_path(&repo_root, out),
        None if config.artifact_dir == Path::new(".") => repo_root
            .join("artifacts")
            .join("runs")
            .join(format!("fda-design-{}", clock.now_unix_seconds())),
        None => artifact_dir.clone(),
    };

    let decision_packet_path = artifact_dir.join("human_decision_packet.json");
    if store.exists(&decision_packet_path) {
        let decision_packet = read_json_value(&store, &decision_packet_path)?;
        let status = decision_packet.get("status").and_then(Value::as_str);
        let decisions = decision_summaries_from_packet(&decision_packet);
        let receipts_path = artifact_dir.join("decision_receipts.json");
        let receipts = read_decision_receipts(&store, &receipts_path)?;
        let receipt_blockers = if store.exists(&receipts_path) {
            decision_blockers(&decisions, &decision_answers_from_receipts(&receipts))
        } else {
            Vec::new()
        };
        if status != Some("resolved") || !receipt_blockers.is_empty() {
            let blocked_decisions = if receipt_blockers.is_empty() {
                decisions
            } else {
                receipt_blockers
            };
            return Ok(DesignResult {
                schema_version: "fda.design_result.v0",
                verdict: "blocked",
                design_gate_status: "waiting_human_decision",
                artifact_dir: display_path(&repo_root, &artifact_dir),
                out_dir: display_path(&repo_root, &out_dir),
                artifacts_written: Vec::new(),
                next_actions: blocked_decisions
                    .iter()
                    .map(|decision| {
                        format!(
                            "fda decide {} --answer <answer> --artifacts {}",
                            decision.decision_id,
                            display_path(&repo_root, &artifact_dir)
                        )
                    })
                    .collect(),
                blocked_decisions,
                validation_report_path: None,
            });
        }
    }

    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;

    let input_summary = design_input_summary(&store, &artifact_dir);
    let context = design_runtime_context(&store, &artifact_dir);
    let mut artifacts_written = Vec::new();

    store.write_text(
        &out_dir.join("basic_design.md"),
        &basic_design_markdown(&input_summary),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("basic_design.md")));

    store.write_text(
        &out_dir.join("detailed_design.md"),
        &detailed_design_markdown(&input_summary),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("detailed_design.md"),
    ));

    store.write_text(
        &out_dir.join("functional_qa_brief.md"),
        &functional_qa_brief_markdown(),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("functional_qa_brief.md"),
    ));

    store.write_text(
        &out_dir.join("security_qa_brief.md"),
        &security_qa_brief_markdown(),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("security_qa_brief.md"),
    ));

    store.write_json(
        &out_dir.join("case_graph.json"),
        &design_case_graph(&context),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("case_graph.json")));

    store.write_json(
        &out_dir.join("task_graph.json"),
        &design_task_graph(&context),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("task_graph.json")));

    store.write_json(
        &out_dir.join("planned_prs.json"),
        &design_planned_prs(&context),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("planned_prs.json")));

    store.write_json(
        &out_dir.join("autonomy_contract.json"),
        &design_autonomy_contract(&context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("autonomy_contract.json"),
    ));

    store.write_json(
        &out_dir.join("forge_projection.json"),
        &design_forge_projection(&context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("forge_projection.json"),
    ));

    store.write_json(
        &out_dir.join("runner_explanation.json"),
        &design_runner_explanation(&repo_root, &artifact_dir, &out_dir, &context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    store.write_json(
        &out_dir.join("artifact_inventory.json"),
        &design_artifact_inventory(&repo_root, &out_dir, &context),
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

    Ok(DesignResult {
        schema_version: "fda.design_result.v0",
        verdict: if validation_report.verdict == "pass" {
            "pass"
        } else {
            "fail"
        },
        design_gate_status: "passed",
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        artifacts_written,
        blocked_decisions: Vec::new(),
        next_actions: vec!["fda implement --dry-run".to_string()],
        validation_report_path: Some(display_path(&repo_root, &validation_report_path)),
    })
}

fn design_input_summary(store: &impl ArtifactStore, artifact_dir: &Path) -> String {
    let requirements_path = artifact_dir.join("requirements_definition.md");
    match store.read_text(&requirements_path) {
        Ok(body) => body
            .lines()
            .find(|line| line.starts_with("- 入力要約:"))
            .map(|line| line.trim_start_matches("- 入力要約:").trim().to_string())
            .unwrap_or_else(|| "既存 Intake artifact から Design Gate を生成する。".to_string()),
        Err(_) => {
            "既存 Intake artifact なしで Design Gate dry-run artifact を生成する。".to_string()
        }
    }
}

fn design_runtime_context(store: &impl ArtifactStore, artifact_dir: &Path) -> RuntimeContext {
    let packet_path = artifact_dir.join("human_decision_packet.json");
    let packet = store
        .exists(&packet_path)
        .then(|| read_json_value(store, &packet_path).ok())
        .flatten();
    let mut context = RuntimeContext::for_v1_design();
    if let Some(packet) = packet {
        if let Some(program_id) = value_string(&packet, "program_id") {
            context.program_id = program_id;
        }
        if let Some(epic_id) = value_string(&packet, "epic_id") {
            context.epic_id = epic_id;
        }
    }
    context
}
