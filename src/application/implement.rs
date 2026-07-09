use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::application::decisions::value_string;
use crate::application::ports::{ArtifactStore, CodexProcessPort};
use crate::application::profile::{
    ensure_repository_profile, ensure_target_repository_profile_if_present,
};
use crate::application::validate::{validate_artifacts, write_report};
use crate::cli::args::{AtoConfig, ImplementConfig, ValidateConfig};
use crate::domain::entities::{
    CodexLiveInvocationResult, CodexLiveStatus, RuntimeContext, ToolProbeResult, ToolProbeStatus,
};
use crate::domain::policies::decision::answer_is_approval;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::json_file::{read_json_value, write_json_file};
use crate::infra::paths::{canonicalize_existing, canonicalize_existing_or_parent};
use crate::now_unix_seconds;
use crate::rendering::implement::*;
use crate::support::paths::{display_path, resolve_path};
use crate::{
    carry_forward_implement_artifacts, human_decision_guard_with, implementation_gate_effect,
    implementation_semantic_verdict, implementation_status, marker_value, parse_actual_pr_url,
    parse_codex_test_status, parse_marker_list, parse_scope_drift, write_text_file,
    DryRunGateStatus, HumanDecisionGuard, ImplementResult, DEFAULT_MODEL_CONTRACT_DIRS,
    DEFAULT_SCHEMA_DIR,
};

fn canonicalize_for_config(path: &Path, label: &str) -> Result<PathBuf, String> {
    canonicalize_existing(path)
        .map_err(|e| format!("failed to resolve {label} {}: {e}", path.display()))
}

fn create_output_dir(path: &Path) -> Result<(), String> {
    FsArtifactStore
        .create_dir_all(path)
        .map_err(|e| format!("failed to create output dir {}: {e}", path.display()))
}

pub(crate) fn implement(
    config: &ImplementConfig,
    process: &impl CodexProcessPort,
) -> Result<ImplementResult, String> {
    if config.dry_run == config.live {
        return Err("implement requires exactly one of --dry-run or --live".to_string());
    }
    if config.dry_run {
        return implement_dry_run(config, process);
    }
    implement_live(config, process)
}

fn implement_dry_run(
    config: &ImplementConfig,
    process: &impl CodexProcessPort,
) -> Result<ImplementResult, String> {
    let store = FsArtifactStore;
    let repo_root = canonicalize_for_config(&config.repo_root, "repo root")?;
    ensure_repository_profile(&store, &repo_root)?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let target_repo = resolve_path(&repo_root, &config.target_repo);
    let target_repo = canonicalize_existing(&target_repo).unwrap_or(target_repo);
    let out_dir = match &config.out {
        Some(out) => resolve_path(&repo_root, out),
        None => env::temp_dir().join(format!("fda-implement-dry-run-{}", now_unix_seconds())),
    };
    let out_dir_for_safety = canonicalize_existing_or_parent(&out_dir).map_err(|e| {
        format!(
            "failed to resolve dry-run output dir {} for containment check: {e}",
            out_dir.display()
        )
    })?;

    if out_dir_for_safety.starts_with(&target_repo) {
        return Err(format!(
            "implement --dry-run output dir {} must not be inside target repo {}",
            out_dir.display(),
            target_repo.display()
        ));
    }

    create_output_dir(&out_dir)?;
    let out_dir = canonicalize_existing(&out_dir).unwrap_or(out_dir_for_safety);
    if out_dir.starts_with(&target_repo) {
        return Err(format!(
            "implement --dry-run output dir {} must not be inside target repo {}",
            out_dir.display(),
            target_repo.display()
        ));
    }
    ensure_target_repository_profile_if_present(&store, &target_repo, &repo_root)?;

    let context = implement_runtime_context(&artifact_dir);
    let guard = implement_human_decision_guard(&artifact_dir)?;
    let expected_tools = vec!["codex".to_string(), "codex-reply".to_string()];
    let plan_status =
        if guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty() {
            "ready"
        } else {
            "blocked"
        };
    let dry_run_started_at = format!("unix:{}", now_unix_seconds());

    let mut artifacts_written = Vec::new();
    write_text_file(
        &out_dir.join("implementation_handoff.md"),
        &implementation_handoff_markdown(&artifact_dir, &target_repo, &context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("implementation_handoff.md"),
    ));

    write_text_file(
        &out_dir.join("codex_prompt.md"),
        &codex_dry_run_prompt_markdown(&target_repo, &context),
    )?;
    artifacts_written.push(display_path(&repo_root, &out_dir.join("codex_prompt.md")));

    write_text_file(
        &out_dir.join("functional_qa_prompt.md"),
        &qa_dry_run_prompt_markdown("functional_qa", &target_repo, &context),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("functional_qa_prompt.md"),
    ));

    write_text_file(
        &out_dir.join("security_qa_prompt.md"),
        &qa_dry_run_prompt_markdown("security_qa", &target_repo, &context),
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
        &out_dir.join("current_codex_cli_handoff.json"),
        &current_codex_cli_handoff(&repo_root, &artifact_dir, &target_repo, &context, &guard),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("current_codex_cli_handoff.json"),
    ));

    write_json_file(
        &out_dir.join("planned_pr_execution_packet.json"),
        &planned_pr_execution_packet(&repo_root, &artifact_dir, &target_repo, &context, &guard),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("planned_pr_execution_packet.json"),
    ));

    write_json_file(
        &out_dir.join("mcp_agent_invocation_plan.json"),
        &mcp_agent_invocation_plan(&target_repo, &context, &guard, plan_status),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("mcp_agent_invocation_plan.json"),
    ));

    let probe = if plan_status == "blocked" {
        ToolProbeResult {
            status: ToolProbeStatus::Failed,
            detected_tools: Vec::new(),
            summary: "Human Decision guard blocked MCP tools/list; no adapter was invoked."
                .to_string(),
            exit_code: None,
        }
    } else if !target_repo.is_dir() {
        ToolProbeResult {
            status: ToolProbeStatus::Failed,
            detected_tools: Vec::new(),
            summary: format!("target repo cwd does not exist: {}", target_repo.display()),
            exit_code: None,
        }
    } else if let Some(tools) = config.tools_list_fixture.clone() {
        ToolProbeResult {
            status: ToolProbeStatus::Succeeded,
            detected_tools: tools,
            summary: "tools/list fixture supplied by test config.".to_string(),
            exit_code: Some(0),
        }
    } else {
        process.query_mcp_tools_list(
            &["codex".to_string(), "mcp-server".to_string()],
            &target_repo,
        )
    };

    let missing_tools = expected_tools
        .iter()
        .filter(|expected| !probe.detected_tools.iter().any(|tool| tool == *expected))
        .cloned()
        .collect::<Vec<_>>();

    let receipt_status = if plan_status == "blocked" {
        "blocked"
    } else {
        match probe.status {
            ToolProbeStatus::Succeeded if missing_tools.is_empty() => "succeeded",
            ToolProbeStatus::AdapterUnavailable => "adapter_unavailable",
            ToolProbeStatus::Succeeded | ToolProbeStatus::Failed => "failed",
        }
    };
    let semantic_verdict = match receipt_status {
        "succeeded" => "pass",
        "adapter_unavailable" => "adapter_unavailable",
        "blocked" => "blocked",
        _ => "fail",
    };
    let gate_effect = if receipt_status == "succeeded" {
        "advance"
    } else {
        "hold"
    };
    let dry_run_completed_at = format!("unix:{}", now_unix_seconds());

    let mcp_receipt = mcp_tool_call_receipt(
        &target_repo,
        &probe,
        receipt_status,
        semantic_verdict,
        gate_effect,
        &dry_run_started_at,
        &dry_run_completed_at,
    );
    write_json_file(&out_dir.join("mcp_tool_call_receipt.json"), &mcp_receipt)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("mcp_tool_call_receipt.json"),
    ));

    write_json_file(
        &out_dir.join("dry_run_receipt.json"),
        &dry_run_receipt(
            &target_repo,
            &expected_tools,
            &missing_tools,
            &probe,
            receipt_status,
            &guard,
            (&dry_run_started_at, &dry_run_completed_at),
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("dry_run_receipt.json"),
    ));

    carry_forward_implement_artifacts(&repo_root, &artifact_dir, &out_dir, &mut artifacts_written)?;

    // F4 比例ゲート: Scope In (planned_prs.json の expected_files) と delivery_policy から
    // risk tier を判定し risk_tier.json を出力する。carry_forward の後に生成することで、
    // 前回 run の stale な tier に上書きされない（常に最新の判定が残る）。
    artifacts_written.push(crate::application::risk_tier::write_risk_tier_artifact(
        &store,
        &repo_root,
        &artifact_dir,
        &out_dir,
    )?);

    write_json_file(
        &out_dir.join("runner_explanation.json"),
        &implement_runner_explanation(
            &repo_root,
            &artifact_dir,
            &out_dir,
            &context,
            receipt_status,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    write_json_file(
        &out_dir.join("artifact_inventory.json"),
        &implement_artifact_inventory(&repo_root, &out_dir, &context),
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

    let verdict = if validation_report.verdict == "pass" && receipt_status == "succeeded" {
        "pass"
    } else if receipt_status == "blocked" {
        "blocked"
    } else {
        "fail"
    }
    .to_string();

    let next_actions = match receipt_status {
        "succeeded" => vec![
            "current Codex CLIで current_codex_cli_handoff.json と implementation_handoff.md に従って実装し、PR作成後 fda review".to_string(),
            "V1.5 optional automation として fda implement --live を使う".to_string(),
        ],
        "blocked" => guard
            .unresolved_decision_ids
            .iter()
            .chain(guard.non_approval_decision_ids.iter())
            .map(|decision_id| {
                format!(
                    "fda decide {} --answer <answer> --artifacts {}",
                    decision_id,
                    display_path(&repo_root, &artifact_dir)
                )
            })
            .collect(),
        "adapter_unavailable" => {
            vec![
                "Codex MCP adapterを利用できる環境で fda implement --dry-run を再実行する"
                    .to_string(),
            ]
        }
        _ => vec!["dry_run_receipt.json の failed check を修正して再実行する".to_string()],
    };

    Ok(ImplementResult {
        schema_version: "fda.implement_result.v0",
        mode: "dry_run".to_string(),
        verdict,
        dry_run_gate_status: receipt_status.to_string(),
        development_gate_status: None,
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        target_repo: display_path(&repo_root, &target_repo),
        artifacts_written,
        detected_tools: probe.detected_tools,
        missing_tools,
        actual_pr_url: None,
        thread_id: None,
        validation_report_path: Some(display_path(&repo_root, &validation_report_path)),
        next_actions,
    })
}

fn implement_live(
    config: &ImplementConfig,
    process: &impl CodexProcessPort,
) -> Result<ImplementResult, String> {
    let store = FsArtifactStore;
    let repo_root = canonicalize_for_config(&config.repo_root, "repo root")?;
    ensure_repository_profile(&store, &repo_root)?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let target_repo_input = resolve_path(&repo_root, &config.target_repo);
    let target_repo_exists = target_repo_input.is_dir();
    let target_repo = if target_repo_exists {
        canonicalize_for_config(&target_repo_input, "target repo")?
    } else {
        target_repo_input.clone()
    };
    ensure_target_repository_profile_if_present(&store, &target_repo, &repo_root)?;
    let out_dir = match &config.out {
        Some(out) => resolve_path(&repo_root, out),
        None => env::temp_dir().join(format!("fda-implement-live-{}", now_unix_seconds())),
    };
    let out_dir_for_safety = canonicalize_existing_or_parent(&out_dir).map_err(|e| {
        format!(
            "failed to resolve live output dir {} for containment check: {e}",
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
            "implement --live output dir {} must not be inside target repo {}",
            out_dir.display(),
            target_repo.display()
        ));
    }
    create_output_dir(&out_dir)?;
    let out_dir = canonicalize_existing(&out_dir).unwrap_or(out_dir);

    let mut context = implement_runtime_context(&artifact_dir);
    context.task_ids = vec!["PR-V1-006".to_string()];
    if context.case_ids.is_empty() {
        context.case_ids = vec!["CASE-FDA-V1-MCP-LIVE-001".to_string()];
    }

    let guard = implement_human_decision_guard(&artifact_dir)?;
    let dry_run_gate = dry_run_gate_status_from_artifacts(&artifact_dir, &target_repo)?;
    let expected_tools = vec!["codex".to_string(), "codex-reply".to_string()];
    let human_clear =
        guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty();
    let live_gate_ready = human_clear && dry_run_gate.is_pass() && target_repo_exists;
    let plan_status = if live_gate_ready { "ready" } else { "blocked" };
    let live_started_at = format!("unix:{}", now_unix_seconds());
    let tools_list_fixture_used = config.tools_list_fixture.is_some();
    let codex_live_fixture_used = config.codex_live_fixture.is_some();
    let fixture_used = tools_list_fixture_used || codex_live_fixture_used;
    let mut tools_list_source = "not_invoked".to_string();
    let mut codex_tool_source = "not_invoked".to_string();

    let mut artifacts_written = Vec::new();
    write_text_file(
        &out_dir.join("implementation_handoff.md"),
        &implementation_live_handoff_markdown(&artifact_dir, &target_repo, &context, &dry_run_gate),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("implementation_handoff.md"),
    ));

    let live_prompt = codex_live_prompt_markdown(&target_repo, &context);
    write_text_file(&out_dir.join("codex_live_prompt.md"), &live_prompt)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("codex_live_prompt.md"),
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
        &out_dir.join("planned_pr_execution_packet.json"),
        &planned_pr_execution_packet_live(
            &repo_root,
            &artifact_dir,
            &target_repo,
            &context,
            &guard,
            &dry_run_gate,
            plan_status,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("planned_pr_execution_packet.json"),
    ));

    write_json_file(
        &out_dir.join("mcp_agent_invocation_plan.json"),
        &mcp_agent_invocation_plan_live(&target_repo, &context, &guard, plan_status),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("mcp_agent_invocation_plan.json"),
    ));

    let mut detected_tools = Vec::new();
    let mut missing_tools = expected_tools.clone();
    let live_result = if !live_gate_ready {
        let mut issues = Vec::new();
        if !human_clear {
            issues.extend(
                guard
                    .unresolved_decision_ids
                    .iter()
                    .map(|decision_id| format!("unresolved decision: {decision_id}")),
            );
            issues.extend(
                guard
                    .non_approval_decision_ids
                    .iter()
                    .map(|decision_id| format!("non-approval decision: {decision_id}")),
            );
        }
        if !dry_run_gate.is_pass() {
            issues.extend(dry_run_gate.issues.clone());
        }
        if !target_repo_exists {
            issues.push(format!(
                "target repo cwd does not exist: {}",
                target_repo.display()
            ));
        }
        CodexLiveInvocationResult {
            status: CodexLiveStatus::Blocked,
            thread_id: None,
            content: String::new(),
            summary: format!("Live implementer preflight blocked: {}", issues.join("; ")),
            exit_code: None,
            tool_call_sent: false,
        }
    } else {
        let probe = if let Some(tools) = config.tools_list_fixture.clone() {
            tools_list_source = "fixture".to_string();
            ToolProbeResult {
                status: ToolProbeStatus::Succeeded,
                detected_tools: tools,
                summary: "tools/list fixture supplied by test config.".to_string(),
                exit_code: Some(0),
            }
        } else {
            tools_list_source = "codex_mcp_server".to_string();
            process.query_mcp_tools_list(
                &["codex".to_string(), "mcp-server".to_string()],
                &target_repo,
            )
        };
        detected_tools = probe.detected_tools.clone();
        missing_tools = expected_tools
            .iter()
            .filter(|expected| !detected_tools.iter().any(|tool| tool == *expected))
            .cloned()
            .collect::<Vec<_>>();

        match probe.status {
            ToolProbeStatus::Succeeded if missing_tools.is_empty() => {
                if let Some(fixture) = config.codex_live_fixture.clone() {
                    codex_tool_source = "fixture".to_string();
                    CodexLiveInvocationResult {
                        status: fixture.status,
                        thread_id: fixture.thread_id,
                        content: fixture.content,
                        summary: "codex live fixture supplied by test config.".to_string(),
                        exit_code: Some(0),
                        tool_call_sent: false,
                    }
                } else {
                    let result = process.query_codex_live_tool(
                        &target_repo,
                        &live_prompt,
                        Duration::from_secs(config.live_timeout_seconds),
                    );
                    codex_tool_source = if result.tool_call_sent {
                        "codex_mcp_tool_call".to_string()
                    } else {
                        "codex_mcp_server_before_tool_call".to_string()
                    };
                    result
                }
            }
            ToolProbeStatus::AdapterUnavailable => CodexLiveInvocationResult {
                status: CodexLiveStatus::AdapterUnavailable,
                thread_id: None,
                content: String::new(),
                summary: probe.summary,
                exit_code: probe.exit_code,
                tool_call_sent: false,
            },
            ToolProbeStatus::Succeeded | ToolProbeStatus::Failed => CodexLiveInvocationResult {
                status: CodexLiveStatus::Failed,
                thread_id: None,
                content: String::new(),
                summary: if missing_tools.is_empty() {
                    probe.summary
                } else {
                    format!("missing expected MCP tools: {}", missing_tools.join(", "))
                },
                exit_code: probe.exit_code,
                tool_call_sent: false,
            },
        }
    };

    let actual_pr_url = parse_actual_pr_url(&live_result.content);
    let test_status = parse_codex_test_status(&live_result.content);
    let test_command = marker_value(&live_result.content, "FDA_TESTS_RUN:")
        .unwrap_or_else(|| "FDA_TESTS_RUN marker not returned".to_string());
    let changed_files = parse_marker_list(&live_result.content, "FDA_CHANGED_FILES:");
    let scope_drift = parse_scope_drift(&live_result.content);
    let implementation_status =
        implementation_status(live_result.status, actual_pr_url.as_deref(), &test_status);
    let semantic_verdict = implementation_semantic_verdict(implementation_status);
    let gate_effect = implementation_gate_effect(implementation_status);
    let live_completed_at = format!("unix:{}", now_unix_seconds());

    write_json_file(
        &out_dir.join("mcp_tool_call_receipt.json"),
        &mcp_tool_call_receipt_live(
            &target_repo,
            &live_result,
            implementation_status,
            semantic_verdict,
            gate_effect,
            &test_status,
            &scope_drift,
            (&live_started_at, &live_completed_at),
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("mcp_tool_call_receipt.json"),
    ));

    write_json_file(
        &out_dir.join("implementation_receipt.json"),
        &implementation_receipt(
            &target_repo,
            &context,
            &dry_run_gate,
            &live_result,
            implementation_status,
            actual_pr_url.as_deref(),
            &test_status,
            &changed_files,
            &scope_drift,
            (&live_started_at, &live_completed_at),
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("implementation_receipt.json"),
    ));

    let target_head_sha = process.git_head_sha(&target_repo);
    write_json_file(
        &out_dir.join("external_pr_receipt.json"),
        &external_pr_receipt(
            &target_repo,
            &context,
            implementation_status,
            actual_pr_url.as_deref(),
            &test_status,
            &test_command,
            &changed_files,
            &scope_drift,
            &live_result.summary,
            &target_head_sha,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("external_pr_receipt.json"),
    ));

    carry_forward_implement_artifacts(&repo_root, &artifact_dir, &out_dir, &mut artifacts_written)?;

    write_json_file(
        &out_dir.join("coding_agent_thread_state.json"),
        &coding_agent_thread_state(
            &target_repo,
            &live_result,
            implementation_status,
            actual_pr_url.as_deref(),
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("coding_agent_thread_state.json"),
    ));

    write_json_file(
        &out_dir.join("live_execution_evidence.json"),
        &live_execution_evidence(LiveExecutionEvidence {
            target_repo: &target_repo,
            context: &context,
            dry_run_gate: &dry_run_gate,
            result: &live_result,
            implementation_status,
            actual_pr_url: actual_pr_url.as_deref(),
            test_status: &test_status,
            changed_files: &changed_files,
            scope_drift: &scope_drift,
            detected_tools: &detected_tools,
            missing_tools: &missing_tools,
            tools_list_source: &tools_list_source,
            codex_tool_source: &codex_tool_source,
            fixture_used,
            time_window: (&live_started_at, &live_completed_at),
        }),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("live_execution_evidence.json"),
    ));

    write_json_file(
        &out_dir.join("runner_explanation.json"),
        &implement_live_runner_explanation(
            &repo_root,
            &artifact_dir,
            &out_dir,
            &context,
            implementation_status,
        ),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("runner_explanation.json"),
    ));

    write_json_file(
        &out_dir.join("artifact_inventory.json"),
        &implement_live_artifact_inventory(&repo_root, &out_dir, &context),
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

    let verdict = if validation_report.verdict == "pass" && implementation_status == "succeeded" {
        "pass"
    } else if implementation_status == "blocked" {
        "blocked"
    } else {
        "fail"
    }
    .to_string();

    let next_actions = match implementation_status {
        "succeeded" => vec!["fda review".to_string()],
        "adapter_unavailable" => {
            vec![
                "Codex MCP adapterを利用できる環境で fda implement --live を再実行する".to_string(),
            ]
        }
        "blocked" => {
            let mut actions = Vec::new();
            if !dry_run_gate.is_pass() {
                actions.push(
                    "fda implement --dry-run をpassさせてから --live を再実行する".to_string(),
                );
            }
            if actual_pr_url.is_none() && dry_run_gate.is_pass() {
                actions.push("Codex live実行結果に actual PR URL を含めて再実行する".to_string());
            }
            if actions.is_empty() {
                actions
                    .push("blocked reasonを解消して fda implement --live を再実行する".to_string());
            }
            actions
        }
        _ => vec!["implementation_receipt.json の failed check を修正して再実行する".to_string()],
    };

    Ok(ImplementResult {
        schema_version: "fda.implement_result.v0",
        mode: "live".to_string(),
        verdict,
        dry_run_gate_status: dry_run_gate.status,
        development_gate_status: Some(implementation_status.to_string()),
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        target_repo: display_path(&repo_root, &target_repo),
        artifacts_written,
        detected_tools,
        missing_tools,
        actual_pr_url,
        thread_id: live_result.thread_id,
        validation_report_path: Some(display_path(&repo_root, &validation_report_path)),
        next_actions,
    })
}

fn dry_run_gate_status_from_artifacts(
    artifact_dir: &Path,
    target_repo: &Path,
) -> Result<DryRunGateStatus, String> {
    let dry_run_receipt_path = artifact_dir.join("dry_run_receipt.json");
    if !dry_run_receipt_path.exists() {
        return Ok(DryRunGateStatus {
            status: "missing".to_string(),
            issues: vec![format!(
                "dry_run_receipt.json is required before --live: {}",
                dry_run_receipt_path.display()
            )],
            evidence_links: vec![],
        });
    }
    let receipt = read_json_value(&dry_run_receipt_path)?;
    let status = value_string(&receipt, "status").unwrap_or_else(|| "unknown".to_string());
    let mut issues = Vec::new();
    if status != "succeeded" {
        issues.push(format!("dry_run_receipt status is {status}"));
    }
    match receipt.get("target_repo_mutated").and_then(Value::as_bool) {
        Some(false) => {}
        Some(true) => issues.push("dry_run_receipt indicates target_repo_mutated=true".to_string()),
        None => issues.push("dry_run_receipt target_repo_mutated must be false".to_string()),
    }
    match value_string(&receipt, "cwd") {
        Some(cwd) if dry_run_cwd_matches_target(&cwd, target_repo) => {}
        Some(cwd) => issues.push(format!(
            "dry_run_receipt cwd `{cwd}` does not match live target repo `{}`",
            target_repo.display()
        )),
        None => issues.push("dry_run_receipt cwd is missing or malformed".to_string()),
    }

    let expected_tools = string_array_field(&receipt, "expected_tools", &mut issues);
    for required_tool in ["codex", "codex-reply"] {
        if !expected_tools.iter().any(|tool| tool == required_tool) {
            issues.push(format!(
                "dry_run_receipt expected_tools is missing required tool `{required_tool}`"
            ));
        }
    }
    let detected_tools = string_array_field(&receipt, "detected_tools", &mut issues);
    for required_tool in ["codex", "codex-reply"] {
        if !detected_tools.iter().any(|tool| tool == required_tool) {
            issues.push(format!(
                "dry_run_receipt detected_tools is missing required tool `{required_tool}`"
            ));
        }
    }
    let missing_tools = string_array_field(&receipt, "missing_tools", &mut issues);
    if !missing_tools.is_empty() {
        issues.push(format!(
            "dry_run_receipt missing tools: {}",
            missing_tools.join(", ")
        ));
    }
    let required_check_ids = [
        "human_decision_guard",
        "cwd",
        "prompt_artifact",
        "approval_policy",
        "forbidden_actions",
        "tools_list",
        "target_repo_mutation",
    ];
    match receipt.get("checks").and_then(Value::as_array) {
        Some(checks) if !checks.is_empty() => {
            let mut seen_check_ids = HashSet::new();
            for check in checks {
                let check_id =
                    value_string(check, "check_id").unwrap_or_else(|| "unknown".to_string());
                seen_check_ids.insert(check_id.clone());
                match check.get("status").and_then(Value::as_str) {
                    Some("pass") => {}
                    Some(status) => {
                        let summary = value_string(check, "summary").unwrap_or_default();
                        issues.push(format!(
                            "dry-run check failed: {check_id}: {status}: {summary}"
                        ));
                    }
                    None => issues.push(format!(
                        "dry-run check `{check_id}` is missing a valid status"
                    )),
                }
            }
            for required_check_id in required_check_ids {
                if !seen_check_ids.contains(required_check_id) {
                    issues.push(format!(
                        "dry_run_receipt checks is missing required check `{required_check_id}`"
                    ));
                }
            }
        }
        _ => issues.push("dry_run_receipt checks must be a non-empty array".to_string()),
    }
    let evidence_links = receipt
        .get("evidence_links")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .chain(std::iter::once("dry_run_receipt.json".to_string()))
        .collect::<Vec<_>>();
    Ok(DryRunGateStatus {
        status,
        issues,
        evidence_links,
    })
}

fn dry_run_cwd_matches_target(receipt_cwd: &str, target_repo: &Path) -> bool {
    let receipt_path = PathBuf::from(receipt_cwd);
    let receipt_resolved = canonicalize_existing(&receipt_path).unwrap_or(receipt_path);
    let target_resolved =
        canonicalize_existing(target_repo).unwrap_or_else(|_| target_repo.to_path_buf());
    receipt_resolved == target_resolved
}

fn string_array_field(value: &Value, field: &str, issues: &mut Vec<String>) -> Vec<String> {
    match value.get(field).and_then(Value::as_array) {
        Some(items) => {
            let mut values = Vec::new();
            for item in items {
                if let Some(item) = item.as_str() {
                    values.push(item.to_string());
                } else {
                    issues.push(format!(
                        "dry_run_receipt {field} contains a non-string item"
                    ));
                }
            }
            values
        }
        None => {
            issues.push(format!("dry_run_receipt {field} must be an array"));
            Vec::new()
        }
    }
}

pub(crate) fn implement_runtime_context(artifact_dir: &Path) -> RuntimeContext {
    let mut context = RuntimeContext {
        program_id: "FDA-V1".to_string(),
        epic_id: "EPIC-FDA-V1-MCP".to_string(),
        case_ids: vec!["CASE-FDA-V1-MCP-DRY-RUN-001".to_string()],
        task_ids: vec!["PR-V1-005".to_string()],
    };

    for artifact in [
        "planned_prs.json",
        "forge_projection.json",
        "human_decision_packet.json",
    ] {
        let path = artifact_dir.join(artifact);
        let Some(value) = path.exists().then(|| read_json_value(&path).ok()).flatten() else {
            continue;
        };
        if let Some(program_id) = value_string(&value, "program_id") {
            context.program_id = program_id;
        }
        if let Some(epic_id) = value_string(&value, "epic_id") {
            context.epic_id = epic_id;
        }
    }

    context
}

pub(crate) fn implement_human_decision_guard(
    artifact_dir: &Path,
) -> Result<HumanDecisionGuard, String> {
    human_decision_guard_with(artifact_dir, answer_is_approval)
}
