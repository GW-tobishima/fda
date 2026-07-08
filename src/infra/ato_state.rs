use serde::Serialize;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::application::ports::AtoConfig;
use crate::infra::json_file::{read_json_value, write_json_file};
use crate::support::paths::display_path;

#[derive(Clone, Debug)]
pub(crate) struct AtoDecisionAnswer {
    pub(crate) decision_id: String,
    pub(crate) answer: String,
    pub(crate) answered_by: String,
}

#[derive(Debug)]
pub(crate) struct AtoSyncRequest<'a> {
    pub(crate) config: &'a AtoConfig,
    pub(crate) stage: &'a str,
    pub(crate) repo_root: &'a Path,
    pub(crate) artifact_dir: &'a Path,
    pub(crate) previous_artifact_dir: Option<&'a Path>,
    pub(crate) result: &'a Value,
    pub(crate) decision_answer: Option<AtoDecisionAnswer>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AtoStateReceipt {
    pub(crate) schema_version: &'static str,
    pub(crate) receipt_id: String,
    pub(crate) stage: String,
    pub(crate) status: String,
    pub(crate) adapter: &'static str,
    pub(crate) task_key: String,
    pub(crate) run_id: Option<String>,
    pub(crate) artifact_dir: String,
    pub(crate) receipt_path: String,
    pub(crate) checkpoint_id: Option<String>,
    pub(crate) decision_mappings: Vec<AtoDecisionMapping>,
    pub(crate) decision_sync: Option<AtoDecisionSync>,
    pub(crate) evidence_edges: Vec<AtoEvidenceEdge>,
    pub(crate) commands: Vec<AtoCommandReceipt>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) resume_command: Option<String>,
    pub(crate) raw_output_stored: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AtoDecisionMapping {
    pub(crate) fda_decision_id: String,
    pub(crate) ato_decision_id: String,
    pub(crate) status: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AtoDecisionSync {
    pub(crate) fda_decision_id: String,
    pub(crate) ato_decision_id: String,
    pub(crate) answer_recorded: bool,
    pub(crate) applied: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AtoEvidenceEdge {
    pub(crate) evidence_surface: String,
    pub(crate) evidence_id: String,
    pub(crate) verdict: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AtoCommandReceipt {
    pub(crate) command_kind: String,
    pub(crate) command: Vec<String>,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout_json_detected: bool,
    pub(crate) stdout_top_level_keys: Vec<String>,
    pub(crate) stderr_summary: Option<String>,
}

struct AtoCommandResult {
    receipt: AtoCommandReceipt,
    json: Option<Value>,
    failure_reason: Option<String>,
}

struct AtoProcessFailure {
    status: String,
    stderr_summary: Option<String>,
    failure_reason: String,
}

struct PreviousAtoReceipt {
    task_key: Option<String>,
    run_id: Option<String>,
    decision_mappings: Vec<AtoDecisionMapping>,
}

struct AtoHumanDecision {
    decision_id: String,
    summary: String,
    recommended_option_id: String,
    option_ids: Vec<String>,
    reason: String,
}

pub(crate) fn sync_ato_state(
    request: AtoSyncRequest<'_>,
) -> Result<Option<AtoStateReceipt>, String> {
    if !request.config.enabled {
        return Ok(None);
    }

    fs::create_dir_all(request.artifact_dir).map_err(|e| {
        format!(
            "failed to create artifact dir {} for ATO receipt: {e}",
            request.artifact_dir.display()
        )
    })?;

    let previous_lookup_dir = request
        .previous_artifact_dir
        .unwrap_or(request.artifact_dir);
    let mut previous_receipt = previous_ato_receipt(previous_lookup_dir);
    let task_key = resolve_task_key(
        request.config,
        request.stage,
        request.artifact_dir,
        &previous_receipt,
    );
    if previous_receipt.task_key.as_deref() != Some(task_key.as_str()) {
        previous_receipt.run_id = None;
        previous_receipt.decision_mappings.clear();
    }
    let run_id = resolve_run_id(request.config, &previous_receipt);
    if run_id.is_some() && previous_receipt.run_id.as_deref() != run_id.as_deref() {
        previous_receipt.decision_mappings.clear();
    }
    let mut receipt = AtoStateReceipt {
        schema_version: "fda.ato_state_receipt.v0",
        receipt_id: format!("ATO-FDA-{}-001", request.stage.to_ascii_uppercase()),
        stage: request.stage.to_string(),
        status: "succeeded".to_string(),
        adapter: "ato_cli",
        task_key: task_key.clone(),
        run_id,
        artifact_dir: display_path(request.repo_root, request.artifact_dir),
        receipt_path: display_path(
            request.repo_root,
            &request.artifact_dir.join("ato_state_receipt.json"),
        ),
        checkpoint_id: None,
        decision_mappings: previous_receipt.decision_mappings,
        decision_sync: None,
        evidence_edges: Vec::new(),
        commands: Vec::new(),
        failure_reason: None,
        resume_command: None,
        raw_output_stored: false,
    };

    if receipt.run_id.is_none() {
        let begin = run_ato_command(
            request.config,
            "work_begin",
            &[
                "work".to_string(),
                "begin".to_string(),
                "--task".to_string(),
                task_key.clone(),
                "--agent-id".to_string(),
                "fda-cli".to_string(),
                "--role".to_string(),
                "orchestrator".to_string(),
                "--capability-profile".to_string(),
                "fda-v1".to_string(),
                "--workspace-policy".to_string(),
                "dedicated_worktree".to_string(),
                "--idempotency-key".to_string(),
                format!(
                    "fda:{}:{}",
                    request.stage,
                    sanitize_id(&receipt.artifact_dir)
                ),
                "--json".to_string(),
            ],
        );
        let begin_json = begin.json.clone();
        apply_command_result(&mut receipt, begin);
        if receipt.status != "succeeded" {
            finalize_receipt(
                request.repo_root,
                request.artifact_dir,
                request.config,
                &mut receipt,
            )?;
            return Ok(Some(receipt));
        }
        receipt.run_id = begin_json
            .as_ref()
            .and_then(|value| find_string_key(value, "run_id"));
        if receipt.run_id.is_none() {
            receipt.status = "failed".to_string();
            receipt.failure_reason =
                Some("ATO work begin succeeded but no run_id was found in JSON output".to_string());
            finalize_receipt(
                request.repo_root,
                request.artifact_dir,
                request.config,
                &mut receipt,
            )?;
            return Ok(Some(receipt));
        }
    }

    let run_id = receipt
        .run_id
        .clone()
        .ok_or_else(|| "ATO run_id missing after begin".to_string())?;
    let evidence_id = primary_evidence_id(request.repo_root, request.artifact_dir, request.result);
    receipt.evidence_edges.push(AtoEvidenceEdge {
        evidence_surface: "fda_artifact".to_string(),
        evidence_id: evidence_id.clone(),
        verdict: result_verdict(request.result),
    });

    let checkpoint = run_ato_command(
        request.config,
        "work_checkpoint",
        &[
            "work".to_string(),
            "checkpoint".to_string(),
            "--task".to_string(),
            task_key.clone(),
            "--run-id".to_string(),
            run_id.clone(),
            "--kind".to_string(),
            request.stage.to_string(),
            "--summary".to_string(),
            stage_summary(request.stage, request.result),
            "--evidence-surface".to_string(),
            "fda_artifact".to_string(),
            "--evidence-id".to_string(),
            evidence_id,
            "--evidence-verdict".to_string(),
            result_verdict(request.result),
            "--validation-status".to_string(),
            validation_status(request.result),
            "--freshness".to_string(),
            "current".to_string(),
            "--durability-class".to_string(),
            "artifact".to_string(),
            "--trust-level".to_string(),
            "self_reported".to_string(),
            "--json".to_string(),
        ],
    );
    let checkpoint_json = checkpoint.json.clone();
    apply_command_result(&mut receipt, checkpoint);
    if let Some(value) = checkpoint_json {
        receipt.checkpoint_id = find_string_key(&value, "checkpoint_id");
    }
    if receipt.status != "succeeded" {
        finalize_receipt(
            request.repo_root,
            request.artifact_dir,
            request.config,
            &mut receipt,
        )?;
        return Ok(Some(receipt));
    }
    if receipt.checkpoint_id.is_none() {
        receipt.status = "failed".to_string();
        receipt.failure_reason = Some(
            "ATO work checkpoint succeeded but no checkpoint_id was found in JSON output"
                .to_string(),
        );
        finalize_receipt(
            request.repo_root,
            request.artifact_dir,
            request.config,
            &mut receipt,
        )?;
        return Ok(Some(receipt));
    }

    if should_sync_human_decisions(request.stage, request.artifact_dir) {
        sync_human_decisions(
            request.config,
            request.result,
            request.artifact_dir,
            &task_key,
            &run_id,
            &mut receipt,
        );
    }

    if let Some(answer) = request.decision_answer.clone() {
        sync_decision_answer(request.config, answer, &mut receipt);
    }

    finalize_receipt(
        request.repo_root,
        request.artifact_dir,
        request.config,
        &mut receipt,
    )?;
    Ok(Some(receipt))
}

pub(crate) fn canonicalize_repo_root_for_sync(repo_root: &Path) -> PathBuf {
    fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf())
}

fn sync_human_decisions(
    config: &AtoConfig,
    result: &Value,
    artifact_dir: &Path,
    task_key: &str,
    run_id: &str,
    receipt: &mut AtoStateReceipt,
) {
    let decisions = human_decisions_for_sync(result, artifact_dir);
    if decisions.is_empty() {
        return;
    };
    for decision in decisions {
        let fda_decision_id = decision.decision_id;
        if fda_decision_id.is_empty() {
            continue;
        }
        if receipt
            .decision_mappings
            .iter()
            .any(|mapping| mapping.fda_decision_id == fda_decision_id && mapping_is_usable(mapping))
        {
            continue;
        }
        receipt.decision_mappings.retain(|mapping| {
            mapping.fda_decision_id != fda_decision_id || mapping_is_usable(mapping)
        });
        let summary = if decision.summary.is_empty() {
            fda_decision_id.clone()
        } else {
            decision.summary
        };
        let mut args = vec![
            "work".to_string(),
            "block".to_string(),
            "--task".to_string(),
            task_key.to_string(),
            "--run-id".to_string(),
            run_id.to_string(),
            "--reason".to_string(),
            decision.reason,
            "--title".to_string(),
            fda_decision_id.clone(),
            "--question".to_string(),
            summary,
        ];
        if !decision.recommended_option_id.is_empty() {
            args.push("--recommended-option".to_string());
            args.push(decision.recommended_option_id);
        }
        if decision.option_ids.is_empty() {
            args.push("--option".to_string());
            args.push("approve".to_string());
        } else {
            for option in decision.option_ids {
                args.push("--option".to_string());
                args.push(option);
            }
        }
        args.push("--json".to_string());

        let block = run_ato_command(config, "work_block", &args);
        let ato_decision_id = block
            .json
            .as_ref()
            .and_then(|value| find_string_key(value, "decision_id"));
        apply_command_result(receipt, block);
        let Some(ato_decision_id) = ato_decision_id else {
            if receipt.status == "succeeded" {
                receipt.status = "failed".to_string();
                receipt.failure_reason =
                    Some("ATO work block did not return decision_id".to_string());
            }
            receipt.decision_mappings.push(AtoDecisionMapping {
                fda_decision_id,
                ato_decision_id: "<missing>".to_string(),
                status: "sync_failed".to_string(),
            });
            return;
        };
        receipt.decision_mappings.push(AtoDecisionMapping {
            fda_decision_id,
            ato_decision_id,
            status: if receipt.status == "succeeded" {
                "open".to_string()
            } else {
                "sync_failed".to_string()
            },
        });
        if receipt.status != "succeeded" {
            return;
        }
    }
}

fn sync_decision_answer(
    config: &AtoConfig,
    answer: AtoDecisionAnswer,
    receipt: &mut AtoStateReceipt,
) {
    let fda_decision_id = answer.decision_id.clone();
    let Some(ato_decision_id) = receipt
        .decision_mappings
        .iter()
        .find(|mapping| mapping.fda_decision_id == fda_decision_id && mapping_is_usable(mapping))
        .map(|mapping| mapping.ato_decision_id.clone())
    else {
        if receipt.status == "succeeded" {
            receipt.status = "failed".to_string();
            receipt.failure_reason = Some(format!(
                "ATO decision mapping missing for FDA decision {fda_decision_id}"
            ));
        }
        receipt.decision_sync = Some(AtoDecisionSync {
            fda_decision_id,
            ato_decision_id: "<missing>".to_string(),
            answer_recorded: false,
            applied: false,
        });
        return;
    };

    let answer_result = run_ato_command(
        config,
        "decision_answer",
        &[
            "decisions".to_string(),
            "answer".to_string(),
            ato_decision_id.clone(),
            "--answer".to_string(),
            answer.answer.clone(),
            "--answered-by".to_string(),
            answer.answered_by,
            "--json".to_string(),
        ],
    );
    apply_command_result(receipt, answer_result);
    let answer_recorded = receipt.status == "succeeded";

    if receipt.status == "succeeded" {
        let apply_result = run_ato_command(
            config,
            "decision_apply",
            &[
                "decisions".to_string(),
                "apply".to_string(),
                ato_decision_id.clone(),
                "--json".to_string(),
            ],
        );
        apply_command_result(receipt, apply_result);
    }

    receipt.decision_sync = Some(AtoDecisionSync {
        fda_decision_id: fda_decision_id.clone(),
        ato_decision_id,
        answer_recorded,
        applied: receipt.status == "succeeded",
    });
    if receipt.status == "succeeded" {
        for mapping in &mut receipt.decision_mappings {
            if mapping.fda_decision_id == fda_decision_id {
                mapping.status = "applied".to_string();
            }
        }
    }
}

fn run_ato_command(config: &AtoConfig, kind: &str, args: &[String]) -> AtoCommandResult {
    let command_display = receipt_command_display(&config.cli_command, args);

    let Some(program) = config.cli_command.first() else {
        return AtoCommandResult {
            receipt: AtoCommandReceipt {
                command_kind: kind.to_string(),
                command: command_display,
                status: "adapter_unavailable".to_string(),
                exit_code: None,
                stdout_json_detected: false,
                stdout_top_level_keys: Vec::new(),
                stderr_summary: Some("ATO CLI command is empty".to_string()),
            },
            json: None,
            failure_reason: Some("ATO CLI command is empty".to_string()),
        };
    };

    let mut command = ProcessCommand::new(program);
    if config.cli_command.len() > 1 {
        command.args(&config.cli_command[1..]);
    }
    command.args(args);
    if let Some(backend) = config
        .backend
        .clone()
        .or_else(|| env::var("FDA_ATO_BACKEND").ok())
    {
        command.env("ATO_CLI_BACKEND", backend);
    }
    if let Some(db_path) = config
        .db_path
        .clone()
        .or_else(|| env::var("FDA_ATO_DB_PATH").ok().map(PathBuf::from))
    {
        command.env("ATO_DB_PATH", db_path);
    }

    let output = match run_command_with_timeout(command, ato_command_timeout()) {
        Ok(output) => output,
        Err(error) => {
            return AtoCommandResult {
                receipt: AtoCommandReceipt {
                    command_kind: kind.to_string(),
                    command: command_display,
                    status: error.status,
                    exit_code: None,
                    stdout_json_detected: false,
                    stdout_top_level_keys: Vec::new(),
                    stderr_summary: error.stderr_summary,
                },
                json: None,
                failure_reason: Some(error.failure_reason),
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let json = serde_json::from_str::<Value>(&stdout).ok();
    let status = if output.status.success() {
        "succeeded"
    } else {
        "failed"
    };
    let failure_reason = if output.status.success() {
        None
    } else {
        Some(redacted_failure_reason(output.status.code(), &stderr))
    };

    AtoCommandResult {
        receipt: AtoCommandReceipt {
            command_kind: kind.to_string(),
            command: command_display,
            status: status.to_string(),
            exit_code: output.status.code(),
            stdout_json_detected: json.is_some(),
            stdout_top_level_keys: json
                .as_ref()
                .and_then(Value::as_object)
                .map(|object| object.keys().cloned().collect())
                .unwrap_or_default(),
            stderr_summary: redacted_stderr_summary(&stderr),
        },
        json,
        failure_reason,
    }
}

fn run_command_with_timeout(
    mut command: ProcessCommand,
    timeout: Duration,
) -> Result<Output, AtoProcessFailure> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        let status = if error.kind() == ErrorKind::NotFound {
            "adapter_unavailable"
        } else {
            "failed"
        };
        let redacted_error = redact_process_error(&error.to_string());
        AtoProcessFailure {
            status: status.to_string(),
            stderr_summary: Some(redacted_error.clone()),
            failure_reason: redacted_error,
        }
    })?;

    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return child.wait_with_output().map_err(|error| {
                    let redacted_error = redact_process_error(&error.to_string());
                    AtoProcessFailure {
                        status: "failed".to_string(),
                        stderr_summary: Some(redacted_error.clone()),
                        failure_reason: redacted_error,
                    }
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AtoProcessFailure {
                    status: "failed".to_string(),
                    stderr_summary: Some("ATO CLI timed out".to_string()),
                    failure_reason: format!(
                        "ATO CLI timed out after {} seconds",
                        timeout.as_secs()
                    ),
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let redacted_error = redact_process_error(&error.to_string());
                return Err(AtoProcessFailure {
                    status: "failed".to_string(),
                    stderr_summary: Some(redacted_error.clone()),
                    failure_reason: redacted_error,
                });
            }
        }
    }
}

fn apply_command_result(receipt: &mut AtoStateReceipt, result: AtoCommandResult) {
    if result.receipt.status != "succeeded" && receipt.status == "succeeded" {
        receipt.status = result.receipt.status.clone();
        receipt.failure_reason = result.failure_reason;
    }
    receipt.commands.push(result.receipt);
}

fn finalize_receipt(
    repo_root: &Path,
    artifact_dir: &Path,
    config: &AtoConfig,
    receipt: &mut AtoStateReceipt,
) -> Result<(), String> {
    if receipt.status != "succeeded" {
        let run_arg = receipt.run_id.as_deref();
        let artifact_dir_arg = display_path(repo_root, artifact_dir);
        receipt.resume_command = Some(resume_command(
            &receipt.stage,
            &artifact_dir_arg,
            &receipt.task_key,
            run_arg,
            config,
        ));
    }
    let path = artifact_dir.join("ato_state_receipt.json");
    write_json_file(&path, &json!(&*receipt))?;
    append_inventory_entry(repo_root, artifact_dir, &path)
}

fn append_inventory_entry(
    repo_root: &Path,
    artifact_dir: &Path,
    receipt_path: &Path,
) -> Result<(), String> {
    let inventory_path = artifact_dir.join("artifact_inventory.json");
    if !inventory_path.exists() {
        return Ok(());
    }
    let mut inventory = read_json_value(&inventory_path)?;
    let Some(artifacts) = inventory.get_mut("artifacts").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    let path = display_path(repo_root, receipt_path);
    if artifacts
        .iter()
        .any(|artifact| artifact.get("path_or_url").and_then(Value::as_str) == Some(path.as_str()))
    {
        return Ok(());
    }
    let now = crate::infra::clock::system_unix_seconds();
    artifacts.push(json!({
        "artifact_id": "ART-ATO-STATE-001",
        "artifact_type": "ato_state_receipt",
        "title": "ATO State Receipt",
        "preview_summary": "ATO task/run/checkpoint/decision/evidence sync receipt",
        "path_or_url": path,
        "producer_agent": "forge-delivery-agent",
        "related_program_id": "FDA-V1",
        "related_epic_id": "EPIC-FDA-V1-OPERATIONAL",
        "related_case_ids": ["CASE-FDA-V1-ATO"],
        "related_task_ids": ["PR-V1-016"],
        "latest_version": "v0",
        "evidence_links": [],
        "diff_link": null,
        "open_in_browser_link": null,
        "open_in_editor_link": null,
        "created_at_unix_seconds": now,
        "updated_at_unix_seconds": now
    }));
    write_json_file(&inventory_path, &inventory)
}

fn resolve_task_key(
    config: &AtoConfig,
    stage: &str,
    artifact_dir: &Path,
    previous: &PreviousAtoReceipt,
) -> String {
    config
        .task_key
        .clone()
        .or_else(|| env::var("FDA_ATO_TASK_KEY").ok())
        .or_else(|| previous.task_key.clone())
        .unwrap_or_else(|| {
            format!(
                "fda-{}-{}",
                stage,
                sanitize_id(&artifact_dir.display().to_string())
            )
        })
}

fn resolve_run_id(config: &AtoConfig, previous: &PreviousAtoReceipt) -> Option<String> {
    config
        .run_id
        .clone()
        .or_else(|| env::var("FDA_ATO_RUN_ID").ok())
        .or_else(|| previous.run_id.clone())
}

fn previous_ato_receipt(artifact_dir: &Path) -> PreviousAtoReceipt {
    let path = artifact_dir.join("ato_state_receipt.json");
    let Ok(value) = read_json_value(&path) else {
        return PreviousAtoReceipt {
            task_key: None,
            run_id: None,
            decision_mappings: Vec::new(),
        };
    };
    let decision_mappings = value
        .get("decision_mappings")
        .and_then(Value::as_array)
        .map(|mappings| {
            mappings
                .iter()
                .filter_map(|mapping| {
                    Some(AtoDecisionMapping {
                        fda_decision_id: string_field(mapping, "fda_decision_id")?,
                        ato_decision_id: string_field(mapping, "ato_decision_id")?,
                        status: string_field(mapping, "status")
                            .unwrap_or_else(|| "unknown".to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    PreviousAtoReceipt {
        task_key: string_field(&value, "task_key"),
        run_id: string_field(&value, "run_id"),
        decision_mappings,
    }
}

fn primary_evidence_id(repo_root: &Path, artifact_dir: &Path, result: &Value) -> String {
    result
        .get("validation_report_path")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            result
                .get("receipts_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            result
                .get("artifacts_written")
                .and_then(Value::as_array)
                .and_then(|artifacts| artifacts.first())
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| display_path(repo_root, artifact_dir))
}

fn stage_summary(stage: &str, result: &Value) -> String {
    let verdict = result_verdict(result);
    let phase = string_field(result, "current_phase")
        .or_else(|| string_field(result, "mode"))
        .or_else(|| string_field(result, "design_gate_status"))
        .or_else(|| string_field(result, "merge_gate_status"))
        .unwrap_or_else(|| "artifact sync".to_string());
    format!("FDA {stage} completed with verdict={verdict}; phase={phase}")
}

fn result_verdict(result: &Value) -> String {
    string_field(result, "verdict").unwrap_or_else(|| "pass".to_string())
}

fn validation_status(result: &Value) -> String {
    if result_verdict(result) == "pass" {
        "passed".to_string()
    } else {
        "failed".to_string()
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn mapping_is_usable(mapping: &AtoDecisionMapping) -> bool {
    mapping.status != "sync_failed" && mapping.ato_decision_id != "<missing>"
}

fn should_sync_human_decisions(stage: &str, artifact_dir: &Path) -> bool {
    let _ = artifact_dir;
    stage == "start" || stage == "plan"
}

fn human_decisions_for_sync(result: &Value, artifact_dir: &Path) -> Vec<AtoHumanDecision> {
    if let Some(decisions) = result.get("human_decisions").and_then(Value::as_array) {
        return decisions
            .iter()
            .filter_map(|decision| {
                let required_before = string_field(decision, "required_before").unwrap_or_default();
                Some(AtoHumanDecision {
                    decision_id: string_field(decision, "decision_id")?,
                    summary: string_field(decision, "summary").unwrap_or_default(),
                    recommended_option_id: string_field(decision, "recommended_option_id")
                        .or_else(|| recommended_option_id_from_options(decision))
                        .unwrap_or_default(),
                    option_ids: decision_option_ids_for_sync(decision),
                    reason: ato_reason_for_decision(
                        decision_reason_source(decision).as_deref(),
                        &required_before,
                    ),
                })
            })
            .collect();
    }
    read_json_value(&artifact_dir.join("human_decision_packet.json"))
        .map(|packet| decision_summaries_from_packet_for_sync(&packet))
        .unwrap_or_default()
}

fn decision_summaries_from_packet_for_sync(packet: &Value) -> Vec<AtoHumanDecision> {
    let nested_decisions = packet
        .get("decisions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|decision| {
            let required_before = string_field(decision, "required_before")
                .unwrap_or_else(|| "Design Gate".to_string());
            AtoHumanDecision {
                decision_id: string_field(decision, "decision_id")
                    .unwrap_or_else(|| "HD-FDA-UNKNOWN".to_string()),
                summary: string_field(decision, "summary")
                    .unwrap_or_else(|| "判断内容が未設定です".to_string()),
                recommended_option_id: string_field(decision, "recommended_option_id")
                    .or_else(|| recommended_option_id_from_options(decision))
                    .unwrap_or_default(),
                option_ids: decision_option_ids_for_sync(decision),
                reason: ato_reason_for_decision(
                    decision_reason_source(decision).as_deref(),
                    &required_before,
                ),
            }
        })
        .collect::<Vec<_>>();
    if nested_decisions.is_empty() {
        top_level_decision_summary_for_sync(packet)
            .into_iter()
            .collect()
    } else {
        nested_decisions
    }
}

fn top_level_decision_summary_for_sync(packet: &Value) -> Option<AtoHumanDecision> {
    let decision_needed = string_field(packet, "decision_needed")?;
    let decision_packet_id = string_field(packet, "decision_packet_id");
    let decision_id = packet
        .get("forge_mapping")
        .and_then(|forge_mapping| {
            value_string_array(forge_mapping, "human_decision_points")
                .first()
                .cloned()
        })
        .or_else(|| decision_packet_id.clone())
        .unwrap_or_else(|| "HD-FDA-TOP-LEVEL".to_string());
    let recommended_option_id = string_field(packet, "recommended_option_id")
        .or_else(|| recommended_option_id_from_options(packet))
        .unwrap_or_else(|| "approve".to_string());
    let required_before =
        string_field(packet, "required_before").unwrap_or_else(|| "Design Gate".to_string());

    Some(AtoHumanDecision {
        decision_id,
        summary: decision_needed,
        recommended_option_id,
        option_ids: decision_option_ids_for_sync(packet),
        reason: ato_reason_for_decision(
            decision_reason_source(packet).as_deref(),
            &required_before,
        ),
    })
}

fn decision_option_ids_for_sync(value: &Value) -> Vec<String> {
    value
        .get("option_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .chain(
            value
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|option| {
                    option
                        .as_str()
                        .map(str::to_string)
                        .or_else(|| string_field(option, "id"))
                }),
        )
        .collect()
}

fn recommended_option_id_from_options(value: &Value) -> Option<String> {
    let options = value.get("options").and_then(Value::as_array)?;
    options
        .iter()
        .find(|option| {
            option
                .get("recommended")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .and_then(|option| {
            option
                .as_str()
                .map(str::to_string)
                .or_else(|| string_field(option, "id"))
        })
        .or_else(|| {
            options.first().and_then(|option| {
                option
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| string_field(option, "id"))
            })
        })
}

fn decision_reason_source(value: &Value) -> Option<String> {
    string_field(value, "reason")
        .or_else(|| string_field(value, "reason_type"))
        .or_else(|| string_field(value, "type"))
        .or_else(|| string_field(value, "trigger"))
}

fn ato_reason_for_decision(decision_type: Option<&str>, required_before: &str) -> String {
    let decision_type = decision_type.unwrap_or_default().to_ascii_lowercase();
    if decision_type.contains("risk")
        || decision_type.contains("security")
        || decision_type.contains("privacy")
        || decision_type.contains("legal")
    {
        return "risk_approval".to_string();
    }
    if decision_type.contains("merge") || decision_type.contains("release") {
        return "merge_approval".to_string();
    }
    if decision_type.contains("exception") {
        return "exception_handle".to_string();
    }
    if decision_type.contains("knowledge") {
        return "knowledge_review".to_string();
    }
    if decision_type.contains("blocker") {
        return "blocker_confirmation".to_string();
    }

    let required_before = required_before.to_ascii_lowercase();
    if required_before.contains("merge") || required_before.contains("release") {
        "merge_approval"
    } else if required_before.contains("risk")
        || required_before.contains("security")
        || required_before.contains("privacy")
        || required_before.contains("legal")
    {
        "risk_approval"
    } else if required_before.contains("exception") {
        "exception_handle"
    } else if required_before.contains("knowledge") {
        "knowledge_review"
    } else if required_before.contains("blocker") {
        "blocker_confirmation"
    } else {
        "spec_decision"
    }
    .to_string()
}

fn resume_command(
    stage: &str,
    artifact_dir_arg: &str,
    task_key: &str,
    run_id: Option<&str>,
    config: &AtoConfig,
) -> String {
    let artifact_dir_arg = shell_quote(artifact_dir_arg);
    let prefix = match stage {
        "start" => format!("fda start <goal-or-input> --out {artifact_dir_arg}"),
        "decide" => {
            format!("fda decide <decision-id> --answer <answer> --artifacts {artifact_dir_arg}")
        }
        "design" => format!("fda design --artifacts {artifact_dir_arg}"),
        "plan" => format!("fda plan --requirements <path> --out {artifact_dir_arg} --mode fixture"),
        "implement_dry_run" => {
            format!("fda implement --dry-run --artifacts {artifact_dir_arg} --target-repo <path>")
        }
        "implement_live" => {
            format!("fda implement --live --artifacts {artifact_dir_arg} --target-repo <path>")
        }
        "review" => format!("fda review --artifacts {artifact_dir_arg} --target-repo <path>"),
        "continue" => format!("fda continue --artifacts {artifact_dir_arg} --target-repo <path>"),
        "merge" => format!("fda merge --artifacts {artifact_dir_arg} --target-repo <path>"),
        "open" => format!("fda open --artifacts {artifact_dir_arg}"),
        "status" => format!("fda status --artifacts {artifact_dir_arg}"),
        "notify" => format!("fda notify test --artifacts {artifact_dir_arg}"),
        "validate_artifacts" => format!("fda validate-artifacts --artifacts {artifact_dir_arg}"),
        other => format!("fda {other} --artifacts {artifact_dir_arg}"),
    };
    let run_arg = run_id
        .map(|run_id| format!(" --ato-run-id {}", shell_quote(run_id)))
        .unwrap_or_default();
    format!(
        "{prefix} --ato-sync --ato-task {}{run_arg}{}",
        shell_quote(task_key),
        ato_control_args(config)
    )
}

fn ato_control_args(config: &AtoConfig) -> String {
    let mut args = String::new();
    if let Some(backend) = config
        .backend
        .clone()
        .or_else(|| env::var("FDA_ATO_BACKEND").ok())
    {
        args.push_str(" --ato-backend ");
        args.push_str(&shell_quote(&backend));
    }
    if let Some(db_path) = config
        .db_path
        .clone()
        .or_else(|| env::var("FDA_ATO_DB_PATH").ok().map(PathBuf::from))
    {
        args.push_str(" --ato-db ");
        args.push_str(&shell_quote(&db_path.display().to_string()));
    }
    if config
        .cli_command
        .first()
        .is_some_and(|program| program != "ato")
    {
        args.push_str(" --ato-cli ");
        args.push_str(&shell_quote(&config.cli_command[0]));
    }
    args
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(ch, '-' | '_' | '.' | '/' | ':' | '=' | '@' | '+' | ',')
    }) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn value_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn receipt_command_display(cli_command: &[String], args: &[String]) -> Vec<String> {
    let mut display = Vec::new();
    if let Some(program) = cli_command.first() {
        display.push(program.clone());
        display.extend(
            cli_command
                .iter()
                .skip(1)
                .map(|_| "<redacted-cli-arg>".to_string()),
        );
    }
    display.extend(redacted_receipt_args(args));
    display
}

fn redacted_receipt_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            redacted.push("<redacted>".to_string());
            redact_next = false;
            continue;
        }
        redacted.push(arg.clone());
        if is_sensitive_receipt_flag(arg) {
            redact_next = true;
        }
    }
    redacted
}

fn is_sensitive_receipt_flag(arg: &str) -> bool {
    matches!(
        arg,
        "--answer"
            | "--answered-by"
            | "--evidence-id"
            | "--idempotency-key"
            | "--option"
            | "--question"
            | "--recommended-option"
            | "--summary"
            | "--title"
    )
}

fn ato_command_timeout() -> Duration {
    env::var("FDA_ATO_CLI_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| (1..=3_600).contains(seconds))
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(30))
}

fn find_string_key(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(object) => {
            if let Some(found) = object.get(key).and_then(Value::as_str) {
                return Some(found.to_string());
            }
            object
                .values()
                .find_map(|child| find_string_key(child, key))
        }
        Value::Array(array) => array.iter().find_map(|child| find_string_key(child, key)),
        _ => None,
    }
}

fn redacted_stderr_summary(stderr: &str) -> Option<String> {
    if stderr.trim().is_empty() {
        None
    } else {
        Some("stderr_redacted: ATO CLI wrote stderr; see external logs".to_string())
    }
}

fn redacted_failure_reason(exit_code: Option<i32>, stderr: &str) -> String {
    if stderr.trim().is_empty() {
        format!(
            "ATO CLI exited non-zero with code {}",
            exit_code.unwrap_or_default()
        )
    } else {
        format!(
            "ATO CLI exited non-zero with code {}; stderr redacted",
            exit_code.unwrap_or_default()
        )
    }
}

fn redact_process_error(error: &str) -> String {
    if error.contains("No such file") || error.contains("not found") {
        "ATO CLI adapter unavailable".to_string()
    } else {
        "ATO CLI process error redacted".to_string()
    }
}

fn sanitize_id(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("{name}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn fake_ato_script(dir: &Path) -> PathBuf {
        let path = dir.join("fake-ato");
        fs::write(
            &path,
            r#"#!/usr/bin/env bash
set -eu
if [[ "$1 $2" == "work begin" ]]; then
  printf '{"task":{"task_key":"TASK-FDA"},"run":{"run_id":"RUN-FDA"}}'
elif [[ "$1 $2" == "work checkpoint" ]]; then
  printf '{"checkpoint":{"checkpoint_id":"CP-FDA"}}'
elif [[ "$1 $2" == "work block" ]]; then
  printf '{"decision":{"decision_id":"ATO-DECISION-001"}}'
elif [[ "$1 $2" == "decisions answer" ]]; then
  printf '{"decision":{"decision_id":"ATO-DECISION-001","status":"answered"}}'
elif [[ "$1 $2" == "decisions apply" ]]; then
  printf '{"decision":{"decision_id":"ATO-DECISION-001","status":"applied"}}'
else
  echo "unexpected command: $*" >&2
  exit 7
fi
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn logging_fake_ato_script(dir: &Path) -> (PathBuf, PathBuf) {
        let path = dir.join("logging-fake-ato");
        let log_path = dir.join("fake-ato-args.log");
        fs::write(
            &path,
            format!(
                r#"#!/usr/bin/env bash
set -eu
printf '%s\n' "$*" >> "{}"
if [[ "$1 $2" == "work begin" ]]; then
  printf '{{"task":{{"task_key":"TASK-FDA"}},"run":{{"run_id":"RUN-FDA"}}}}'
elif [[ "$1 $2" == "work checkpoint" ]]; then
  printf '{{"checkpoint":{{"checkpoint_id":"CP-FDA"}}}}'
elif [[ "$1 $2" == "work block" ]]; then
  printf '{{"decision":{{"decision_id":"ATO-DECISION-001"}}}}'
else
  echo "unexpected command: $*" >&2
  exit 7
fi
"#,
                log_path.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        (path, log_path)
    }

    fn failing_ato_script(dir: &Path) -> PathBuf {
        let path = dir.join("failing-ato");
        fs::write(
            &path,
            r#"#!/usr/bin/env bash
set -eu
echo "token=super-secret-value dsn=postgres://secret@example/db" >&2
exit 9
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn sleeping_ato_script(dir: &Path) -> PathBuf {
        let path = dir.join("sleeping-ato");
        fs::write(
            &path,
            r#"#!/usr/bin/env bash
set -eu
sleep 2
printf '{"checkpoint":{"checkpoint_id":"CP-FDA"}}'
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn checkpoint_without_id_ato_script(dir: &Path) -> PathBuf {
        let path = dir.join("checkpoint-no-id-ato");
        fs::write(
            &path,
            r#"#!/usr/bin/env bash
set -eu
if [[ "$1 $2" == "work checkpoint" ]]; then
  printf '{"checkpoint":{}}'
elif [[ "$1 $2" == "work begin" ]]; then
  printf '{"run":{"run_id":"RUN-FDA"}}'
else
  printf '{}'
fi
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[test]
    fn start_sync_creates_receipt_without_raw_output() {
        let dir = temp_dir("fda-ato-start-sync");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("artifact_inventory.json"),
            &json!({"schema_version":"fda.artifact_inventory.v0","artifacts":[]}),
        )
        .unwrap();

        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "out_dir": artifact_dir.display().to_string(),
            "human_decisions": [{
                "decision_id": "HD-FDA-001",
                "summary": "scopeを固定してよいか",
                "option_ids": ["approve_scope", "revise"]
            }],
            "validation_report_path": artifact_dir.join("validation_report.json").display().to_string()
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "start",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert!(!receipt.raw_output_stored);
        assert_eq!(receipt.run_id.as_deref(), Some("RUN-FDA"));
        assert_eq!(
            receipt.decision_mappings[0].ato_decision_id,
            "ATO-DECISION-001"
        );
        assert!(artifact_dir.join("ato_state_receipt.json").exists());
        let inventory = read_json_value(&artifact_dir.join("artifact_inventory.json")).unwrap();
        assert!(inventory
            .get("artifacts")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|entry| entry.get("artifact_type").and_then(Value::as_str)
                == Some("ato_state_receipt")));
    }

    #[test]
    fn start_sync_preserves_recommended_option_and_reason() {
        let dir = temp_dir("fda-ato-start-decision-reason");
        let (fake_ato, log_path) = logging_fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "human_decisions": [{
                "decision_id": "HD-FDA-MERGE-001",
                "type": "merge_policy_decision",
                "summary": "merge policyを固定してよいか",
                "recommended_option_id": "manual_review",
                "option_ids": ["auto_merge", "manual_review"]
            }]
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "start",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        let log = fs::read_to_string(log_path).unwrap();
        let block_line = log
            .lines()
            .find(|line| line.contains("work block"))
            .unwrap();
        assert!(block_line.contains("--reason merge_approval"));
        assert!(block_line.contains("--recommended-option manual_review"));
        assert!(block_line.contains("--option auto_merge"));
        assert!(block_line.contains("--option manual_review"));
    }

    #[test]
    fn plan_sync_preserves_packet_reason_and_recommended_option() {
        let dir = temp_dir("fda-ato-plan-packet-reason");
        let (fake_ato, log_path) = logging_fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "decisions": [{
                    "decision_id": "HD-FDA-RISK-001",
                    "type": "risk_decision",
                    "summary": "privacy riskを承認してよいか",
                    "required_before": "Merge Gate",
                    "options": [{"id": "accept_risk"}, {"id": "revise"}],
                    "recommended_option_id": "accept_risk"
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({"verdict": "pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "plan",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        let log = fs::read_to_string(log_path).unwrap();
        let block_line = log
            .lines()
            .find(|line| line.contains("work block"))
            .unwrap();
        assert!(block_line.contains("--reason risk_approval"));
        assert!(block_line.contains("--recommended-option accept_risk"));
        assert!(block_line.contains("--option accept_risk"));
        assert!(block_line.contains("--option revise"));
    }

    #[test]
    fn unavailable_cli_is_fail_closed() {
        let dir = temp_dir("fda-ato-missing-cli");
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![dir.join("does-not-exist").display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "adapter_unavailable");
        assert!(receipt.resume_command.is_some());
        assert!(!receipt.raw_output_stored);
    }

    #[test]
    fn decide_sync_reuses_previous_task_run_and_receipt_evidence() {
        let dir = temp_dir("fda-ato-decide-sync");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("ato_state_receipt.json"),
            &json!({
                "schema_version": "fda.ato_state_receipt.v0",
                "receipt_id": "ATO-FDA-START-001",
                "stage": "start",
                "status": "succeeded",
                "adapter": "ato_cli",
                "task_key": "TASK-EXISTING",
                "run_id": "RUN-EXISTING",
                "artifact_dir": artifact_dir.display().to_string(),
                "receipt_path": artifact_dir.join("ato_state_receipt.json").display().to_string(),
                "checkpoint_id": "CP-OLD",
                "decision_mappings": [{
                    "fda_decision_id": "HD-FDA-001",
                    "ato_decision_id": "ATO-DECISION-001",
                    "status": "open"
                }],
                "decision_sync": null,
                "evidence_edges": [],
                "commands": [{
                    "command_kind": "work_begin",
                    "command": ["ato", "work", "begin"],
                    "status": "succeeded",
                    "exit_code": 0,
                    "stdout_json_detected": true,
                    "stdout_top_level_keys": ["run"],
                    "stderr_summary": null
                }],
                "failure_reason": null,
                "resume_command": null,
                "raw_output_stored": false
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: None,
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let receipts_path = artifact_dir.join("decision_receipts.json");
        let result = json!({
            "verdict": "pass",
            "artifact_dir": artifact_dir.display().to_string(),
            "receipts_path": receipts_path.display().to_string()
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "decide",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: Some(AtoDecisionAnswer {
                decision_id: "HD-FDA-001".to_string(),
                answer: "approve_scope".to_string(),
                answered_by: "human".to_string(),
            }),
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert_eq!(receipt.task_key, "TASK-EXISTING");
        assert_eq!(receipt.run_id.as_deref(), Some("RUN-EXISTING"));
        assert_eq!(
            receipt.evidence_edges[0].evidence_id,
            receipts_path.display().to_string()
        );
        assert!(receipt
            .commands
            .iter()
            .all(|command| command.command_kind != "work_begin"));
        assert_eq!(
            receipt.decision_sync.as_ref().unwrap().ato_decision_id,
            "ATO-DECISION-001"
        );
        assert_eq!(receipt.decision_mappings[0].status, "applied");
    }

    #[test]
    fn start_retry_skips_existing_decision_mapping() {
        let dir = temp_dir("fda-ato-start-retry");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("ato_state_receipt.json"),
            &json!({
                "task_key": "TASK-EXISTING",
                "run_id": "RUN-EXISTING",
                "decision_mappings": [{
                    "fda_decision_id": "HD-FDA-001",
                    "ato_decision_id": "ATO-DECISION-001",
                    "status": "open"
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: None,
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "human_decisions": [{
                "decision_id": "HD-FDA-001",
                "summary": "scopeを固定してよいか",
                "option_ids": ["approve_scope", "revise"]
            }]
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "start",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert!(receipt
            .commands
            .iter()
            .all(|command| command.command_kind != "work_block"));
        assert_eq!(receipt.decision_mappings.len(), 1);
    }

    #[test]
    fn start_retry_recreates_sync_failed_decision_mapping() {
        let dir = temp_dir("fda-ato-start-retry-sync-failed");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("ato_state_receipt.json"),
            &json!({
                "task_key": "TASK-EXISTING",
                "run_id": "RUN-EXISTING",
                "decision_mappings": [{
                    "fda_decision_id": "HD-FDA-001",
                    "ato_decision_id": "<missing>",
                    "status": "sync_failed"
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: None,
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "human_decisions": [{
                "decision_id": "HD-FDA-001",
                "summary": "scopeを固定してよいか",
                "option_ids": ["approve_scope", "revise"]
            }]
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "start",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert!(receipt
            .commands
            .iter()
            .any(|command| command.command_kind == "work_block"));
        assert_eq!(receipt.decision_mappings.len(), 1);
        assert_eq!(
            receipt.decision_mappings[0].ato_decision_id,
            "ATO-DECISION-001"
        );
        assert_eq!(receipt.decision_mappings[0].status, "open");
    }

    #[test]
    fn explicit_task_override_does_not_reuse_previous_run_or_mappings() {
        let dir = temp_dir("fda-ato-task-override");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("ato_state_receipt.json"),
            &json!({
                "task_key": "TASK-A",
                "run_id": "RUN-A",
                "decision_mappings": [{
                    "fda_decision_id": "HD-FDA-001",
                    "ato_decision_id": "ATO-DECISION-A",
                    "status": "open"
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-B".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert_eq!(receipt.task_key, "TASK-B");
        assert_eq!(receipt.run_id.as_deref(), Some("RUN-FDA"));
        assert!(receipt.decision_mappings.is_empty());
        assert!(receipt
            .commands
            .iter()
            .any(|command| command.command_kind == "work_begin"));
    }

    #[test]
    fn explicit_run_override_does_not_reuse_previous_mappings() {
        let dir = temp_dir("fda-ato-run-override");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("ato_state_receipt.json"),
            &json!({
                "task_key": "TASK-A",
                "run_id": "RUN-A",
                "decision_mappings": [{
                    "fda_decision_id": "HD-FDA-001",
                    "ato_decision_id": "ATO-DECISION-A",
                    "status": "open"
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: None,
            run_id: Some("RUN-B".to_string()),
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "human_decisions": [{
                "decision_id": "HD-FDA-001",
                "summary": "scopeを固定してよいか",
                "option_ids": ["approve_scope", "revise"]
            }]
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "start",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert_eq!(receipt.task_key, "TASK-A");
        assert_eq!(receipt.run_id.as_deref(), Some("RUN-B"));
        assert!(receipt
            .commands
            .iter()
            .any(|command| command.command_kind == "work_block"));
        assert_eq!(receipt.decision_mappings.len(), 1);
        assert_eq!(
            receipt.decision_mappings[0].ato_decision_id,
            "ATO-DECISION-001"
        );
    }

    #[test]
    fn decide_without_mapping_fails_closed() {
        let dir = temp_dir("fda-ato-decide-missing-mapping");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: Some("RUN-FDA".to_string()),
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "receipts_path": artifact_dir.join("decision_receipts.json").display().to_string()
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "decide",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: Some(AtoDecisionAnswer {
                decision_id: "HD-FDA-001".to_string(),
                answer: "approve_scope".to_string(),
                answered_by: "human".to_string(),
            }),
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "failed");
        assert_eq!(
            receipt.decision_sync.as_ref().unwrap().ato_decision_id,
            "<missing>"
        );
        assert!(receipt
            .commands
            .iter()
            .all(|command| command.command_kind != "decision_answer"));
    }

    #[test]
    fn receipt_command_args_redact_decision_answer() {
        let dir = temp_dir("fda-ato-command-redaction");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("ato_state_receipt.json"),
            &json!({
                "task_key": "TASK-EXISTING",
                "run_id": "RUN-EXISTING",
                "decision_mappings": [{
                    "fda_decision_id": "HD-FDA-001",
                    "ato_decision_id": "ATO-DECISION-001",
                    "status": "open"
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: None,
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "receipts_path": artifact_dir.join("decision_receipts.json").display().to_string()
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "decide",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: Some(AtoDecisionAnswer {
                decision_id: "HD-FDA-001".to_string(),
                answer: "token=super-secret-value".to_string(),
                answered_by: "human".to_string(),
            }),
        })
        .unwrap()
        .unwrap();

        let serialized = serde_json::to_string(&receipt).unwrap();
        assert_eq!(receipt.status, "succeeded");
        assert!(!serialized.contains("super-secret-value"));
        assert!(serialized.contains("<redacted>"));
    }

    #[test]
    fn normal_stage_does_not_reopen_human_decision_packet() {
        let dir = temp_dir("fda-ato-no-reopen-design");
        let fake_ato = fake_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        write_json_file(
            &artifact_dir.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "decisions": [{
                    "decision_id": "HD-FDA-001",
                    "summary": "already resolved",
                    "options": [{"option_id": "approve"}]
                }]
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: Some("RUN-FDA".to_string()),
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert!(receipt
            .commands
            .iter()
            .all(|command| command.command_kind != "work_block"));
    }

    #[test]
    fn different_output_dir_can_reuse_previous_artifact_receipt() {
        let dir = temp_dir("fda-ato-previous-artifacts");
        let fake_ato = fake_ato_script(&dir);
        let intake_dir = dir.join("intake");
        let design_dir = dir.join("design");
        fs::create_dir_all(&intake_dir).unwrap();
        fs::create_dir_all(&design_dir).unwrap();
        write_json_file(
            &intake_dir.join("ato_state_receipt.json"),
            &json!({
                "task_key": "TASK-EXISTING",
                "run_id": "RUN-EXISTING",
                "decision_mappings": []
            }),
        )
        .unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: None,
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({
            "verdict": "pass",
            "artifact_dir": intake_dir.display().to_string(),
            "out_dir": design_dir.display().to_string(),
            "artifacts_written": [design_dir.join("basic_design.md").display().to_string()]
        });

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &design_dir,
            previous_artifact_dir: Some(&intake_dir),
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "succeeded");
        assert_eq!(receipt.task_key, "TASK-EXISTING");
        assert_eq!(receipt.run_id.as_deref(), Some("RUN-EXISTING"));
        assert!(receipt
            .commands
            .iter()
            .all(|command| command.command_kind != "work_begin"));
        assert!(design_dir.join("ato_state_receipt.json").exists());
    }

    #[test]
    fn ato_cli_timeout_is_fail_closed() {
        let dir = temp_dir("fda-ato-timeout");
        let sleeping_ato = sleeping_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let previous_timeout = env::var("FDA_ATO_CLI_TIMEOUT_SECONDS").ok();
        env::set_var("FDA_ATO_CLI_TIMEOUT_SECONDS", "1");
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: Some("RUN-FDA".to_string()),
            backend: None,
            db_path: None,
            cli_command: vec![sleeping_ato.display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();
        if let Some(value) = previous_timeout {
            env::set_var("FDA_ATO_CLI_TIMEOUT_SECONDS", value);
        } else {
            env::remove_var("FDA_ATO_CLI_TIMEOUT_SECONDS");
        }

        assert_eq!(receipt.status, "failed");
        assert_eq!(
            receipt.failure_reason.as_deref(),
            Some("ATO CLI timed out after 1 seconds")
        );
        assert!(receipt.resume_command.is_some());
    }

    #[test]
    fn checkpoint_without_id_fails_closed() {
        let dir = temp_dir("fda-ato-checkpoint-no-id");
        let fake_ato = checkpoint_without_id_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: Some("RUN-FDA".to_string()),
            backend: None,
            db_path: None,
            cli_command: vec![fake_ato.display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(receipt.status, "failed");
        assert_eq!(
            receipt.failure_reason.as_deref(),
            Some("ATO work checkpoint succeeded but no checkpoint_id was found in JSON output")
        );
    }

    #[test]
    fn resume_command_preserves_ato_controls() {
        let dir = temp_dir("fda ato resume controls");
        let artifact_dir = dir.join("artifact dir");
        let ato_db = dir.join("ato db.sqlite");
        let ato_cli = dir.join("missing ato");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK FDA".to_string()),
            run_id: Some("RUN FDA".to_string()),
            backend: Some("local".to_string()),
            db_path: Some(ato_db.clone()),
            cli_command: vec![ato_cli.display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        let resume = receipt.resume_command.as_deref().unwrap();
        assert!(resume.contains("--ato-backend local"));
        assert!(resume.contains("--artifacts 'artifact dir'"));
        assert!(resume.contains("--ato-task 'TASK FDA'"));
        assert!(resume.contains("--ato-run-id 'RUN FDA'"));
        assert!(resume.contains(&format!("--ato-db '{}'", ato_db.display())));
        assert!(resume.contains(&format!("--ato-cli '{}'", ato_cli.display())));
    }

    #[test]
    fn stderr_summary_is_redacted_on_failure() {
        let dir = temp_dir("fda-ato-redacted-stderr");
        let failing_ato = failing_ato_script(&dir);
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![failing_ato.display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        let serialized = serde_json::to_string(&receipt).unwrap();
        assert_eq!(receipt.status, "failed");
        assert!(!serialized.contains("super-secret-value"));
        assert!(!serialized.contains("postgres://secret"));
        assert!(serialized.contains("stderr redacted"));
    }

    #[test]
    fn process_error_reason_is_redacted() {
        let dir = temp_dir("fda-ato-redacted-process-error");
        let artifact_dir = dir.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let config = AtoConfig {
            enabled: true,
            task_key: Some("TASK-FDA".to_string()),
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec![dir.join("missing-ato").display().to_string()],
        };
        let result = json!({"verdict":"pass"});

        let receipt = sync_ato_state(AtoSyncRequest {
            config: &config,
            stage: "design",
            repo_root: &dir,
            artifact_dir: &artifact_dir,
            previous_artifact_dir: None,
            result: &result,
            decision_answer: None,
        })
        .unwrap()
        .unwrap();

        let serialized = serde_json::to_string(&receipt).unwrap();
        assert_eq!(receipt.status, "adapter_unavailable");
        assert_eq!(
            receipt.failure_reason.as_deref(),
            Some("ATO CLI adapter unavailable")
        );
        assert!(serialized.contains("ATO CLI adapter unavailable"));
    }
}
