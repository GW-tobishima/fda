use serde::Serialize;
use serde_json::Value;
use std::path::Path;

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    recorded_decision_receipts_from_packet, value_string,
};
use crate::application::ports::ArtifactStore;
use crate::application::profile::ensure_repository_profile;
use crate::cli::args::OpenConfig;
use crate::domain::policies::decision::decision_receipt_answer;
use crate::infra::fs_store::{list_file_names, FsArtifactStore};
use crate::infra::json_file::{read_json_value, write_json_file};
use crate::rendering::output_hub::{
    decision_inbox_html, execution_status_html, output_hub_html, output_hub_receipt, DecisionView,
    HubArtifactView, StatusView,
};
use crate::support::paths::{display_path, resolve_path};
use crate::{ensure_artifact_dir_exists, write_text_file};

#[derive(Debug, Serialize)]
pub(crate) struct OpenResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: String,
    pub(crate) artifact_dir: String,
    pub(crate) out_dir: String,
    pub(crate) output_hub_path: String,
    pub(crate) decision_inbox_path: String,
    pub(crate) execution_status_path: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) fn open_output_hub(config: &OpenConfig) -> Result<OpenResult, String> {
    let store = FsArtifactStore;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    ensure_repository_profile(&store, &repo_root)?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let out_dir = config
        .out
        .as_ref()
        .map(|out| resolve_path(&repo_root, out))
        .unwrap_or_else(|| artifact_dir.clone());
    ensure_artifact_dir_exists(&artifact_dir)?;
    store
        .create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {}: {e}", out_dir.display()))?;

    let artifacts = output_hub_artifact_rows(&artifact_dir)?;
    let decisions = decision_rows_from_artifacts(&artifact_dir)?;
    let status_rows = execution_status_rows_from_artifacts(&artifact_dir)?;
    let artifact_dir_display = display_path(&repo_root, &artifact_dir);

    let mut artifacts_written = Vec::new();
    let output_hub_path = out_dir.join("output_hub.html");
    write_text_file(
        &output_hub_path,
        &output_hub_html(&artifact_dir_display, &artifacts, &decisions, &status_rows),
    )?;
    artifacts_written.push(display_path(&repo_root, &output_hub_path));

    let decision_inbox_path = out_dir.join("decision_inbox.html");
    write_text_file(&decision_inbox_path, &decision_inbox_html(&decisions))?;
    artifacts_written.push(display_path(&repo_root, &decision_inbox_path));

    let execution_status_path = out_dir.join("execution_status.html");
    write_text_file(&execution_status_path, &execution_status_html(&status_rows))?;
    artifacts_written.push(display_path(&repo_root, &execution_status_path));

    write_json_file(
        &out_dir.join("output_hub_receipt.json"),
        &output_hub_receipt(&artifact_dir_display, &artifacts, &decisions, &status_rows),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("output_hub_receipt.json"),
    ));

    Ok(OpenResult {
        schema_version: "fda.open_result.v0",
        verdict: "pass".to_string(),
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        output_hub_path: display_path(&repo_root, &output_hub_path),
        decision_inbox_path: display_path(&repo_root, &decision_inbox_path),
        execution_status_path: display_path(&repo_root, &execution_status_path),
        artifacts_written,
        next_actions: vec!["output_hub.html をブラウザで確認する".to_string()],
    })
}

pub(crate) fn output_hub_artifact_rows(
    artifact_dir: &Path,
) -> Result<Vec<HubArtifactView>, String> {
    let inventory_path = artifact_dir.join("artifact_inventory.json");
    if inventory_path.exists() {
        let inventory = read_json_value(&inventory_path)?;
        let mut rows = inventory
            .get("artifacts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|artifact| HubArtifactView {
                title: value_string(artifact, "title").unwrap_or_else(|| "<untitled>".to_string()),
                artifact_type: value_string(artifact, "artifact_type")
                    .unwrap_or_else(|| "artifact".to_string()),
                path_or_url: value_string(artifact, "path_or_url").unwrap_or_default(),
                preview_summary: value_string(artifact, "preview_summary").unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.title.cmp(&b.title));
        return Ok(rows);
    }

    let rows = if artifact_dir.exists() {
        list_file_names(artifact_dir)?
            .into_iter()
            .map(|file_name| HubArtifactView {
                title: file_name.clone(),
                artifact_type: "file".to_string(),
                path_or_url: file_name,
                preview_summary: "artifact_inventory.json がないためファイル一覧から生成"
                    .to_string(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(rows)
}

pub(crate) fn decision_rows_from_artifacts(
    artifact_dir: &Path,
) -> Result<Vec<DecisionView>, String> {
    let packet_path = artifact_dir.join("human_decision_packet.json");
    if !packet_path.exists() {
        return Ok(Vec::new());
    }
    let packet = read_json_value(&packet_path)?;
    let status = value_string(&packet, "status").unwrap_or_else(|| "unknown".to_string());
    let decisions = decision_summaries_from_packet(&packet);
    let receipts_path = artifact_dir.join("decision_receipts.json");
    let packet_resolved_without_receipts = !receipts_path.exists()
        && packet.get("status").and_then(Value::as_str) == Some("resolved")
        && packet.get("recorded_decision").is_some();
    if status == "resolved" && !packet_resolved_without_receipts && !receipts_path.exists() {
        return Ok(Vec::new());
    }
    let receipts = if packet_resolved_without_receipts {
        recorded_decision_receipts_from_packet(&packet)
    } else {
        let store = FsArtifactStore;
        read_decision_receipts(&store, &receipts_path)?
    };
    let answers = decision_answers_from_receipts(&receipts);
    Ok(decisions
        .into_iter()
        .filter(|decision| decision_receipt_answer(decision, &answers).is_none())
        .map(|decision| {
            let resume_command = format!("fda decide {} --answer <answer>", decision.decision_id);
            DecisionView {
                decision_id: decision.decision_id,
                summary: decision.summary,
                required_before: decision.required_before,
                status: status.clone(),
                options: decision.option_ids,
                recommended_option_id: decision.recommended_option_id,
                resume_command,
            }
        })
        .collect())
}

pub(crate) fn execution_status_rows_from_artifacts(
    artifact_dir: &Path,
) -> Result<Vec<StatusView>, String> {
    let mut rows = Vec::new();
    let runner_path = artifact_dir.join("runner_explanation.json");
    if runner_path.exists() {
        let runner = read_json_value(&runner_path)?;
        if let Some(explanation) = runner.get("runner_explanation") {
            for (label, key) in [
                ("Current Phase", "current_phase"),
                ("Stop Condition", "stop_condition"),
                ("Next Action", "next_action"),
            ] {
                rows.push(StatusView {
                    label: label.to_string(),
                    value: value_string(explanation, key)
                        .unwrap_or_else(|| "<unknown>".to_string()),
                });
            }
        }
    }
    let validation_path = artifact_dir.join("validation_report.json");
    if validation_path.exists() {
        let report = read_json_value(&validation_path)?;
        rows.push(StatusView {
            label: "Validation Verdict".to_string(),
            value: value_string(&report, "verdict").unwrap_or_else(|| "<unknown>".to_string()),
        });
        if let Some(summary) = report.get("summary") {
            rows.push(StatusView {
                label: "Validation Summary".to_string(),
                value: format!(
                    "passed={}, failed={}, skipped={}",
                    summary.get("passed").and_then(Value::as_u64).unwrap_or(0),
                    summary.get("failed").and_then(Value::as_u64).unwrap_or(0),
                    summary.get("skipped").and_then(Value::as_u64).unwrap_or(0)
                ),
            });
        }
    }
    if rows.is_empty() {
        rows.push(StatusView {
            label: "Status".to_string(),
            value: "artifact evidence pending".to_string(),
        });
    }
    Ok(rows)
}
