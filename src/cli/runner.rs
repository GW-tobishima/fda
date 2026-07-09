use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::application::{decide, design, gc, plan, start, status, ui, validate};
use crate::cli::args::{parse_args, AtoConfig, Command};
use crate::cli::output::{
    print_continue_summary, print_decide_summary, print_design_summary, print_gc_summary,
    print_implement_summary, print_merge_summary, print_notify_summary, print_open_summary,
    print_plan_summary, print_review_summary, print_start_summary, print_status_summary,
    print_validation_summary,
};
use crate::infra::ato_state::{
    canonicalize_repo_root_for_sync, sync_ato_state, AtoDecisionAnswer, AtoStateReceipt,
    AtoSyncRequest,
};
use crate::{continue_run, implement, merge_run, notify_test, open_output_hub, review};

pub fn run(args: Vec<String>) -> Result<bool, String> {
    let command = parse_args(args)?;
    match command {
        Command::Help => Ok(true),
        Command::Start(config) => {
            let result = start::start(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt =
                sync_after_command(&config.ato, "start", &config.repo_root, &result_value, None)?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_start_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Decide(config) => {
            let result = decide::decide(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt = sync_after_command(
                &config.ato,
                "decide",
                &config.repo_root,
                &result_value,
                Some(AtoDecisionAnswer {
                    decision_id: result.decision_id.clone(),
                    answer: config.answer.clone(),
                    answered_by: config.decided_by.clone(),
                }),
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_decide_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Design(config) => {
            let result = design::design(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt = sync_after_command(
                &config.ato,
                "design",
                &config.repo_root,
                &result_value,
                None,
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_design_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Plan(config) => {
            let result = plan::plan(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt =
                sync_after_command(&config.ato, "plan", &config.repo_root, &result_value, None)?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_plan_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Implement(config) => {
            let result = implement(&config)?;
            let stage = if config.live {
                "implement_live"
            } else {
                "implement_dry_run"
            };
            let result_value = result_value(&result)?;
            let ato_receipt =
                sync_after_command(&config.ato, stage, &config.repo_root, &result_value, None)?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_implement_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Review(config) => {
            let result = review(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt = sync_after_command(
                &config.ato,
                "review",
                &config.repo_root,
                &result_value,
                None,
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_review_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Continue(config) => {
            let result = continue_run(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt = sync_after_command(
                &config.ato,
                "continue",
                &config.repo_root,
                &result_value,
                None,
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_continue_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Merge(config) => {
            let result = merge_run(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt =
                sync_after_command(&config.ato, "merge", &config.repo_root, &result_value, None)?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_merge_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Open(config) => {
            let result = open_output_hub(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt =
                sync_after_command(&config.ato, "open", &config.repo_root, &result_value, None)?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_open_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Status(config) => {
            let result = status::status(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt = sync_after_command(
                &config.ato,
                "status",
                &config.repo_root,
                &result_value,
                None,
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_status_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::NotifyTest(config) => {
            let result = notify_test(&config)?;
            let result_value = result_value(&result)?;
            let ato_receipt = sync_after_command(
                &config.ato,
                "notify",
                &config.repo_root,
                &result_value,
                None,
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_notify_summary(&result);
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(result.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
        Command::Ui(config) => {
            // read-only projection。ATO 同期は行わない（何も状態変更しないため）。
            if config.print_json {
                let snapshot = ui::mission_control_snapshot(&config)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?
                );
                return Ok(true);
            }
            crate::ui_serve(&config)?;
            Ok(true)
        }
        Command::Gc(config) => {
            // read-only スキャン + docket 出力のみ。ATO 同期は行わない。
            let result = gc::gc(&config)?;
            if config.print_json {
                let value = serde_json::to_value(&result).map_err(|e| e.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
                );
            } else {
                print_gc_summary(&result);
            }
            Ok(result.verdict == "pass")
        }
        Command::ValidateArtifacts(config) => {
            let report = validate::validate(&config)?;
            let mut result_value = result_value(&report)?;
            if let Some(out) = config.out.as_ref() {
                let report_path = actual_output_path(out);
                if let Some(object) = result_value.as_object_mut() {
                    object.insert(
                        "validation_report_path".to_string(),
                        serde_json::Value::String(report_path.display().to_string()),
                    );
                }
            }
            let ato_receipt = sync_after_command(
                &config.ato,
                "validate_artifacts",
                &config.repo_root,
                &result_value,
                None,
            )?;
            if config.print_json {
                print_json_result(result_value, ato_receipt.as_ref())?;
            } else {
                print_validation_summary(&report, config.out.as_deref());
                print_ato_summary(ato_receipt.as_ref());
            }
            Ok(report.verdict == "pass" && ato_ok(ato_receipt.as_ref()))
        }
    }
}

fn result_value(result: &impl Serialize) -> Result<Value, String> {
    serde_json::to_value(result).map_err(|e| e.to_string())
}

fn sync_after_command(
    config: &AtoConfig,
    stage: &str,
    repo_root: &Path,
    result: &Value,
    decision_answer: Option<AtoDecisionAnswer>,
) -> Result<Option<AtoStateReceipt>, String> {
    if !config.enabled {
        return Ok(None);
    }
    let repo_root = canonicalize_repo_root_for_sync(repo_root);
    let artifact_dir = result_artifact_dir(&repo_root, result);
    let previous_artifact_dir = previous_artifact_dir(&repo_root, result);
    sync_ato_state(AtoSyncRequest {
        config,
        stage,
        repo_root: &repo_root,
        artifact_dir: &artifact_dir,
        previous_artifact_dir: previous_artifact_dir.as_deref(),
        result,
        decision_answer,
    })
}

fn result_artifact_dir(repo_root: &Path, result: &Value) -> PathBuf {
    let artifacts_written = result
        .get("artifacts_written")
        .and_then(Value::as_array)
        .map(|artifacts| !artifacts.is_empty())
        .unwrap_or(false);
    let raw = if artifacts_written {
        result
            .get("out_dir")
            .or_else(|| result.get("artifact_dir"))
            .and_then(Value::as_str)
    } else {
        result
            .get("artifact_dir")
            .or_else(|| result.get("out_dir"))
            .and_then(Value::as_str)
    }
    .unwrap_or(".");
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn previous_artifact_dir(repo_root: &Path, result: &Value) -> Option<PathBuf> {
    result
        .get("artifact_dir")
        .and_then(Value::as_str)
        .map(|raw| absolute_path(repo_root, Path::new(raw)))
}

fn absolute_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn actual_output_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn print_json_result(mut result: Value, receipt: Option<&AtoStateReceipt>) -> Result<(), String> {
    if let Some(receipt) = receipt {
        let object = result
            .as_object_mut()
            .ok_or_else(|| "command result JSON must be an object".to_string())?;
        object.insert(
            "ato_state_receipt".to_string(),
            serde_json::to_value(receipt).map_err(|e| e.to_string())?,
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn print_ato_summary(receipt: Option<&AtoStateReceipt>) {
    let Some(receipt) = receipt else {
        return;
    };
    println!();
    println!("ATO sync: {}", receipt.status);
    println!("ATO receipt: {}", receipt.receipt_path);
    if let Some(reason) = &receipt.failure_reason {
        println!("ATO failure: {reason}");
    }
}

fn ato_ok(receipt: Option<&AtoStateReceipt>) -> bool {
    receipt
        .map(|receipt| receipt.status == "succeeded")
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::actual_output_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn actual_output_path_uses_process_cwd_for_relative_paths() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        assert_eq!(
            actual_output_path(Path::new("relative-report.json")),
            cwd.join("relative-report.json")
        );
    }
}
