use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::{CodexLiveInvocationResult, RuntimeContext, ToolProbeResult};
use crate::rendering::inventory::{artifact_inventory_entry, ArtifactInventorySpec};
use crate::support::paths::display_path;
use crate::{
    marker_value, now_unix_seconds, parse_pr_number_from_url, DryRunGateStatus, HumanDecisionGuard,
};

pub(crate) fn implementation_live_handoff_markdown(
    artifact_dir: &Path,
    target_repo: &Path,
    context: &RuntimeContext,
    dry_run_gate: &DryRunGateStatus,
) -> String {
    format!(
        "# Implementation Handoff\n\n\
artifact: implementation_handoff\n\
schema_version: fda.implementation_handoff.v0\n\
status: live\n\n\
## 1. Purpose\n\n\
PR-V1-006 は Codex MCP live implementer を実行し、planned PR と actual PR の対応、test evidence、implementation receipt、external PR receipt を残す。\n\n\
## 2. Context\n\n\
- Program: {}\n\
- Epic: {}\n\
- Source artifact dir: {}\n\
- Target repo cwd: {}\n\
- Dry-run gate: {}\n\n\
## 3. Scope In\n\n\
- dry_run_receipt.json が succeeded であることを確認する。\n\
- `codex mcp-server` の `codex` tool を `workspace-write` / `on-request` で呼び出す。\n\
- target repo の実装、test、PR作成を Implementer に依頼する。\n\
- 実装結果を `implementation_receipt.json` と `external_pr_receipt.json` に正規化する。\n\n\
## 4. Scope Out\n\n\
- Functional QA / Security QA は PR-V1-007 に分離する。\n\
- merge / release / human-only approval は行わない。\n\
- Human Decision 未解決または dry-run 未通過のまま Codex tool を呼ばない。\n\n\
## 5. Expected Evidence\n\n\
- `mcp_tool_call_receipt.json`\n\
- `implementation_receipt.json`\n\
- `external_pr_receipt.json`\n\
- `coding_agent_thread_state.json`\n",
        context.program_id,
        context.epic_id,
        artifact_dir.display(),
        target_repo.display(),
        dry_run_gate.status
    )
}

pub(crate) fn codex_live_prompt_markdown(target_repo: &Path, context: &RuntimeContext) -> String {
    format!(
        "# Codex MCP Live Implementer Prompt\n\n\
あなたは FDA V1 の Implementer です。target repo で planned PR を実装し、test を実行し、GitHub PR を作成してください。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- planned PR: `PR-V1-006`\n\
- workspace policy: `write`\n\
- approval policy: `on-request`\n\n\
## 必須事項\n\n\
- Human Decision 未解決のscope変更をしない。\n\
- 実装後に関連testまたはreadiness checkを実行する。\n\
- PRを作成し、planned PR ID と actual PR URL の対応を返す。\n\
- merge / release は行わない。\n\n\
## 最終応答フォーマット\n\n\
最後に以下のmarkerを必ず出力してください。\n\n\
```text\n\
FDA_ACTUAL_PR_URL: https://github.com/<owner>/<repo>/pull/<number>\n\
FDA_TEST_STATUS: passed|failed|not_run\n\
FDA_TESTS_RUN: <command summary>\n\
FDA_CHANGED_FILES: <comma separated paths or NONE>\n\
FDA_SCOPE_DRIFT: none|<summary>\n\
```\n",
        target_repo.display(),
        context.program_id,
        context.epic_id
    )
}

pub(crate) fn implementation_handoff_markdown(
    artifact_dir: &Path,
    target_repo: &Path,
    context: &RuntimeContext,
) -> String {
    format!(
        "# Implementation Handoff\n\n\
artifact: implementation_handoff\n\
schema_version: fda.implementation_handoff.v0\n\
status: current_codex_cli_handoff\n\n\
## 1. Purpose\n\n\
PR-V1-005 / V1-PIVOT-006 は current Codex CLI primary の実装handoffを作る。現在のCodex CLIが明示的なrole switchとATO checkpointの後、approved scope内の実装へ進むための材料を固定する。\n\n\
## 2. Context\n\n\
- Program: {}\n\
- Epic: {}\n\
- Source artifact dir: {}\n\
- Target repo cwd: {}\n\n\
## 3. Scope In\n\n\
- `.fda/` Profile Gate、Human Decision Guard、target repo cwdを確認する。\n\
- current Codex CLIがimplementerへ切り替えるための `current_codex_cli_handoff.json` を生成する。\n\
- approved scope、forbidden actions、required checks、expected evidenceをhandoffに残す。\n\
- MCP invocation artifactsはV1.5 optional automation互換として残す。\n\n\
## 4. Scope Out\n\n\
- このdry-run自体ではtarget repoのsource file、branch、commit、PR、merge、releaseを変更しない。\n\
- Human Decision 未解決または非承認回答がある場合はimplementer roleへ切り替えない。\n\
- merge / release / deploy / human-only approvalは行わない。\n\n\
## 5. Expected Evidence\n\n\
- `current_codex_cli_handoff.json`\n\
- `mcp_agent_invocation_plan.json`\n\
- `codex_prompt.md`\n\
- `mcp_tool_call_receipt.json`\n\
- `dry_run_receipt.json`\n",
        context.program_id,
        context.epic_id,
        artifact_dir.display(),
        target_repo.display()
    )
}

pub(crate) fn codex_dry_run_prompt_markdown(
    target_repo: &Path,
    context: &RuntimeContext,
) -> String {
    format!(
        "# Codex MCP Dry-run Prompt\n\n\
あなたは FDA V1 の Implementer 候補です。この prompt は dry-run 用であり、実装は行いません。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- allowed tool: `codex`\n\
- continuation tool: `codex-reply`\n\
- approval policy: `on-request`\n\n\
## 禁止事項\n\n\
- source file mutation\n\
- branch 作成\n\
- commit / push / PR作成\n\
- merge / release\n\
- Human Decision なしの scope 変更\n\n\
この prompt は `tools/list` と dry-run receipt の検証材料であり、MCP `codex` tool へ送信しない。\n",
        target_repo.display(),
        context.program_id,
        context.epic_id
    )
}

pub(crate) fn qa_dry_run_prompt_markdown(
    role: &str,
    target_repo: &Path,
    context: &RuntimeContext,
) -> String {
    let (title, focus, forbidden_extra) = match role {
        "security_qa" => (
            "Security QA",
            "security/privacy/legal risk and policy compliance",
            "functional QA の出力をコピーしてSecurity QA判定の代替にしない。",
        ),
        _ => (
            "Functional QA",
            "acceptance criteria coverage and regression risk",
            "Security exception approvalやmerge approvalを行わない。",
        ),
    };
    format!(
        "# {title} MCP Dry-run Prompt\n\n\
あなたは FDA V1 の {title} 候補です。この prompt は dry-run 用であり、レビュー実行は行いません。\n\n\
## 境界\n\n\
- target repo cwd: `{}`\n\
- program: `{}`\n\
- epic: `{}`\n\
- role: `{}`\n\
- focus: `{}`\n\
- workspace policy: `read_only`\n\
- approval policy: `read-only`\n\n\
## 禁止事項\n\n\
- source file mutation\n\
- branch 作成\n\
- commit / push / PR作成\n\
- merge / release\n\
- risk self-approval\n\
- {forbidden_extra}\n\n\
この prompt は `mcp_agent_invocation_plan.json` の参照整合性を満たすdry-run artifactであり、MCP `codex` toolへ送信しない。\n",
        target_repo.display(),
        context.program_id,
        context.epic_id,
        role,
        focus
    )
}

pub(crate) fn implement_agent_role_policy() -> Value {
    json!({
        "schema_version": "fda.agent_role_policy.v0",
        "policy_id": "ARP-FDA-V1-MCP-DRY-RUN-001",
        "status": "approved",
        "roles": [
            {
                "role": "implementer",
                "workspace_policy": "write",
                "source_mutation_allowed": true,
                "allowed_actions": ["prepare implementation handoff", "run tests in later live phase"],
                "forbidden_actions": ["merge", "release", "scope_change_without_human_decision"],
                "allowed_mcp_tools": ["codex", "codex-reply"],
                "approval_policy": "on-request",
                "receipt_required": true
            },
            {
                "role": "functional_qa",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "allowed_actions": ["read PR evidence", "verify acceptance criteria"],
                "forbidden_actions": ["source_mutation", "merge", "security_exception_approval"],
                "allowed_mcp_tools": ["codex", "claude"],
                "approval_policy": "read-only",
                "receipt_required": true
            },
            {
                "role": "security_qa",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "allowed_actions": ["read PR evidence", "verify security and privacy risks"],
                "forbidden_actions": ["source_mutation", "functional_qa_copy_paste", "risk_self_approval"],
                "allowed_mcp_tools": ["codex", "claude"],
                "approval_policy": "read-only",
                "receipt_required": true
            }
        ],
        "human_decision_guard": {
            "unresolved_decisions_block_invocation": true,
            "non_approval_answers_block_invocation": true
        }
    })
}

pub(crate) fn current_codex_cli_handoff(
    repo_root: &Path,
    artifact_dir: &Path,
    target_repo: &Path,
    context: &RuntimeContext,
    guard: &HumanDecisionGuard,
) -> Value {
    let target_exists = target_repo.is_dir();
    let human_clear =
        guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty();
    let ready = target_exists && human_clear;
    json!({
        "schema_version": "fda.current_codex_cli_handoff.v0",
        "handoff_id": "CCLI-HANDOFF-FDA-V1-006-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-006",
        "executor": "current_codex_cli",
        "target_repo": {
            "cwd": target_repo.to_string_lossy(),
            "exists": target_exists
        },
        "status": if ready { "ready" } else { "blocked" },
        "role_switch": {
            "from_role": "orchestrator",
            "to_role": "implementer",
            "requires_checkpoint": true,
            "requires_handoff": true
        },
        "profile_gate": {
            "required": true,
            "created_or_verified": target_exists,
            "required_files": [
                ".fda/repo.yaml",
                ".fda/delivery_policy.yaml",
                ".fda/skills.lock",
                ".fda/agent_roles.yaml",
                ".fda/gates.yaml",
                ".fda/artifact_map.yaml",
                ".fda/notification.yaml"
            ]
        },
        "human_decision_guard": {
            "unresolved_decision_ids": guard.unresolved_decision_ids,
            "non_approval_decision_ids": guard.non_approval_decision_ids
        },
        "source_artifacts": [
            display_path(repo_root, artifact_dir),
            "implementation_handoff.md",
            "planned_pr_execution_packet.json",
            "agent_role_policy.json"
        ],
        "scope_in": [
            "Read implementation_handoff.md and planned_pr_execution_packet.json",
            "Implement approved scope only in the current Codex CLI session",
            "Run the repository checks defined in .fda/repo.yaml where applicable",
            "Create or update the target PR and return actual PR URL evidence"
        ],
        "scope_out": [
            "Unapproved scope change",
            "Merge, release, deploy",
            "Security high or critical risk approval",
            "Privacy, legal, terms, or public API breaking approval"
        ],
        "required_checks": [
            "ATO checkpoint before role switch",
            "Human Decision guard clear",
            ".fda profile present",
            "Relevant test or readiness command executed",
            "Scope drift recorded if present"
        ],
        "expected_evidence": [
            "implementation_receipt.json",
            "external_pr_receipt.json",
            "test command summary",
            "actual PR URL",
            "changed files list",
            "scope drift summary"
        ],
        "forbidden_actions": [
            "merge",
            "release",
            "deploy",
            "scope_change_without_human_decision",
            "raw_user_data_exposure",
            "destructive_migration_without_human_decision"
        ],
        "next_action": if ready {
            "current Codex CLI may switch to implementer after ATO checkpoint and execute approved scope"
        } else {
            "resolve Human Decision guard or target repo Profile Gate before implementation"
        }
    })
}

pub(crate) fn planned_pr_execution_packet(
    repo_root: &Path,
    artifact_dir: &Path,
    target_repo: &Path,
    context: &RuntimeContext,
    guard: &HumanDecisionGuard,
) -> Value {
    let human_resolved =
        guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty();
    json!({
        "schema_version": "planned_pr_execution_packet.v0",
        "packet_id": "PPEXEC-FDA-V1-005-001",
        "status": if human_resolved { "ready_for_external_implementation" } else { "blocked" },
        "created_at": format!("unix:{}", now_unix_seconds()),
        "source_repo": display_path(repo_root, repo_root),
        "target_repo": {
            "name": target_repo
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("target-repo"),
            "local_path": target_repo.to_string_lossy()
        },
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-005",
        "planned_pr_title": "Current Codex CLI implementation gate",
        "handoff_kind": "external_implementation",
        "agent_invocation_plan_id": Value::Null,
        "source_artifacts": [
            display_path(repo_root, artifact_dir),
            "implementation_handoff.md",
            "current_codex_cli_handoff.json",
            "codex_prompt.md"
        ],
        "resolved_human_decisions": [],
        "human_decision_dependencies": guard
            .unresolved_decision_ids
            .iter()
            .chain(guard.non_approval_decision_ids.iter())
            .cloned()
            .collect::<Vec<_>>(),
        "scope_in": [
            "Current Codex CLI implementer handoff generation",
            ".fda Profile Gate and Human Decision guard check",
            "cwd, approved scope, required checks, forbidden actions receipt generation",
            "V1.5 optional MCP automation compatibility artifacts"
        ],
        "scope_out": [
            "target repo source mutation during dry-run",
            "branch creation during dry-run",
            "commit, push, PR creation, merge, release during dry-run",
            "Human Decision self-approval"
        ],
        "files_to_change_candidates": [],
        "acceptance_criteria": [
            "Given fda implement --dry-run, When Codex MCP adapter is available, Then tools/list verifies codex and codex-reply.",
            "Given dry-run execution, When artifacts are generated, Then target repo source is not modified.",
            "Given unresolved Human Decision, When dry-run runs, Then MCP server is not invoked and receipt is blocked."
        ],
        "security_privacy_legal_checks": [
            "No target repo mutation in dry-run",
            "No secret values are written to receipt",
            "No live implementation prompt is sent to Codex tool"
        ],
        "rollback_noop_plan": [
            "Dry-run writes only output artifacts outside target repo; delete output dir if rollback is needed."
        ],
        "expected_evidence_from_target_pr": [
            "current_codex_cli_handoff.json",
            "mcp_agent_invocation_plan.json",
            "mcp_tool_call_receipt.json",
            "dry_run_receipt.json"
        ],
        "implementation_start_gate": {
            "human_decisions_resolved": human_resolved,
            "target_repo_changes_allowed_by_this_repo": false,
            "target_repo_pr_creation_allowed_by_this_packet": false,
            "notes": "Dry-run creates current Codex CLI handoff. Actual implementation requires explicit role switch and checkpoint."
        },
        "forge_mapping": {
            "claim_ids": ["CLM-FDA-V1-CODEX-CLI-HANDOFF-001"],
            "proof_obligations": ["PROOF-FDA-V1-CODEX-CLI-HANDOFF-001"],
            "gate_requirements": ["Profile Gate", "Human Decision guard", "Current Codex CLI handoff", "No target repo mutation during dry-run"]
        }
    })
}

pub(crate) fn mcp_agent_invocation_plan(
    target_repo: &Path,
    context: &RuntimeContext,
    guard: &HumanDecisionGuard,
    status: &str,
) -> Value {
    let gate_verdict = if status == "ready" {
        "clear"
    } else {
        "blocked"
    };
    json!({
        "schema_version": "fda.mcp_agent_invocation_plan.v0",
        "plan_id": "MCP-FDA-V1-DRY-RUN-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-005",
        "status": status,
        "human_decision_guard": {
            "unresolved_decision_ids": guard.unresolved_decision_ids,
            "non_approval_decision_ids": guard.non_approval_decision_ids,
            "gate_verdict": gate_verdict
        },
        "role_policy_ref": "agent_role_policy.json",
        "invocations": [
            {
                "invocation_id": "INV-FDA-V1-IMPLEMENTER-DRY-RUN-001",
                "role": "implementer",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "write",
                "source_mutation_allowed": true,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "codex_prompt.md",
                "input_artifacts": ["planned_pr_execution_packet.json", "implementation_handoff.md"],
                "allowed_tools": ["codex", "codex-reply"],
                "forbidden_actions": ["merge", "release", "scope_change_without_human_decision"],
                "expected_receipts": ["mcp_tool_call_receipt.json", "dry_run_receipt.json"]
            },
            {
                "invocation_id": "INV-FDA-V1-FQA-DRY-RUN-001",
                "role": "functional_qa",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "functional_qa_prompt.md",
                "input_artifacts": ["planned_pr_execution_packet.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex"],
                "forbidden_actions": ["source_mutation", "security_exception_approval", "merge"],
                "expected_receipts": ["functional_qa_receipt.json"]
            },
            {
                "invocation_id": "INV-FDA-V1-SQA-DRY-RUN-001",
                "role": "security_qa",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "security_qa_prompt.md",
                "input_artifacts": ["planned_pr_execution_packet.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex"],
                "forbidden_actions": ["source_mutation", "functional_qa_copy_paste", "risk_self_approval"],
                "expected_receipts": ["security_qa_receipt.json"]
            }
        ],
        "expected_global_receipts": ["mcp_tool_call_receipt.json", "dry_run_receipt.json"]
    })
}

pub(crate) fn mcp_tool_call_receipt(
    target_repo: &Path,
    probe: &ToolProbeResult,
    status: &str,
    semantic_verdict: &str,
    gate_effect: &str,
    started_at: &str,
    completed_at: &str,
) -> Value {
    json!({
        "schema_version": "fda.mcp_tool_call_receipt.v0",
        "receipt_id": "MCPR-FDA-V1-DRY-RUN-001",
        "plan_id": "MCP-FDA-V1-DRY-RUN-001",
        "invocation_id": "INV-FDA-V1-IMPLEMENTER-DRY-RUN-001",
        "role": "implementer",
        "provider": "codex",
        "mcp_server": "codex mcp-server",
        "tool_name": "tools/list",
        "thread_id": Value::Null,
        "cwd": target_repo.to_string_lossy(),
        "status": status,
        "started_at": started_at,
        "completed_at": completed_at,
        "input_artifacts": ["mcp_agent_invocation_plan.json", "codex_prompt.md"],
        "output_artifacts": ["dry_run_receipt.json"],
        "tool_result_digest": {
            "raw_result_stored": false,
            "summary": probe.summary,
            "exit_code": probe.exit_code
        },
        "semantic_result": {
            "semantic_verdict": semantic_verdict,
            "summary": probe.summary,
            "scope_drift": [],
            "tests": [
                {
                    "command": "tools/list",
                    "verdict": if status == "succeeded" { "pass" } else { "fail" },
                    "evidence": format!("detected tools: {}", probe.detected_tools.join(", "))
                }
            ],
            "gate_effect": gate_effect
        },
        "evidence_links": ["mcp_agent_invocation_plan.json", "dry_run_receipt.json"],
        "next_action": if status == "succeeded" { "current Codex CLIで current_codex_cli_handoff.json に従って実装する。fda implement --live はV1.5 optional automation。" } else { "fix dry-run gate failure and rerun" }
    })
}

pub(crate) fn dry_run_receipt(
    target_repo: &Path,
    expected_tools: &[String],
    missing_tools: &[String],
    probe: &ToolProbeResult,
    status: &str,
    guard: &HumanDecisionGuard,
    time_window: (&str, &str),
) -> Value {
    json!({
        "schema_version": "fda.mcp_dry_run_receipt.v0",
        "receipt_id": "MCPDRY-FDA-V1-005-001",
        "plan_id": "MCP-FDA-V1-DRY-RUN-001",
        "invocation_id": "INV-FDA-V1-IMPLEMENTER-DRY-RUN-001",
        "provider": "codex",
        "mcp_server_command": ["codex", "mcp-server"],
        "cwd": target_repo.to_string_lossy(),
        "status": status,
        "started_at": time_window.0,
        "completed_at": time_window.1,
        "target_repo_mutated": false,
        "expected_tools": expected_tools,
        "detected_tools": probe.detected_tools,
        "missing_tools": missing_tools,
        "checks": [
            {
                "check_id": "human_decision_guard",
                "status": if guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty() { "pass" } else { "fail" },
                "summary": "未解決または非承認 Human Decision がないこと"
            },
            {
                "check_id": "cwd",
                "status": if target_repo.is_dir() { "pass" } else { "fail" },
                "summary": target_repo.to_string_lossy()
            },
            {
                "check_id": "prompt_artifact",
                "status": "pass",
                "summary": "codex_prompt.md generated"
            },
            {
                "check_id": "approval_policy",
                "status": "pass",
                "summary": "on-request"
            },
            {
                "check_id": "forbidden_actions",
                "status": "pass",
                "summary": "merge/release/scope_change_without_human_decision/source mutation during dry-run are forbidden"
            },
            {
                "check_id": "tools_list",
                "status": if status == "succeeded" { "pass" } else { "fail" },
                "summary": probe.summary
            },
            {
                "check_id": "target_repo_mutation",
                "status": "pass",
                "summary": "FDA dry-run wrote artifacts only to output dir outside target repo"
            }
        ],
        "evidence_links": ["mcp_tool_call_receipt.json", "mcp_agent_invocation_plan.json"]
    })
}

pub(crate) fn planned_pr_execution_packet_live(
    repo_root: &Path,
    artifact_dir: &Path,
    target_repo: &Path,
    context: &RuntimeContext,
    guard: &HumanDecisionGuard,
    dry_run_gate: &DryRunGateStatus,
    status: &str,
) -> Value {
    let human_resolved =
        guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty();
    let start_allowed = status == "ready" && human_resolved && dry_run_gate.is_pass();
    json!({
        "schema_version": "planned_pr_execution_packet.v0",
        "packet_id": "PPEXEC-FDA-V1-006-001",
        "status": if start_allowed { "ready_for_external_implementation" } else { "blocked" },
        "created_at": format!("unix:{}", now_unix_seconds()),
        "source_repo": display_path(repo_root, repo_root),
        "target_repo": {
            "name": target_repo
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("target-repo"),
            "local_path": target_repo.to_string_lossy()
        },
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-006",
        "planned_pr_title": "Codex MCP live implementer",
        "handoff_kind": "mcp_implementation",
        "agent_invocation_plan_id": "MCP-FDA-V1-LIVE-001",
        "source_artifacts": [
            display_path(repo_root, artifact_dir),
            "implementation_handoff.md",
            "codex_live_prompt.md",
            "dry_run_receipt.json"
        ],
        "resolved_human_decisions": [],
        "human_decision_dependencies": guard
            .unresolved_decision_ids
            .iter()
            .chain(guard.non_approval_decision_ids.iter())
            .cloned()
            .collect::<Vec<_>>(),
        "scope_in": [
            "Codex MCP codex tool live invocation",
            "target repo implementation by MCP implementer",
            "test evidence collection",
            "actual PR URL collection",
            "implementation and external PR receipts"
        ],
        "scope_out": [
            "Functional QA and Security QA execution",
            "repair loop",
            "merge, release, deployment",
            "human-only approval self-approval"
        ],
        "files_to_change_candidates": [],
        "acceptance_criteria": [
            "Given fda implement --live and dry-run pass, When Codex MCP returns an implementation result, Then FDA records implementation_receipt.json.",
            "Given Codex creates a target PR, When FDA parses the result, Then planned_pr_id and actual_pr_url are recorded in external_pr_receipt.json.",
            "Given tests were run by the implementer, When FDA writes receipts, Then test status and command evidence are recorded."
        ],
        "security_privacy_legal_checks": [
            "Do not invoke live Codex tool with unresolved or non-approval Human Decision",
            "Do not merge or release from implementer phase",
            "Record scope drift instead of silently closing planned PR"
        ],
        "rollback_noop_plan": [
            "Close the target PR or revert its branch if implementation output must be discarded.",
            "Delete FDA output artifacts if they were generated for a failed live attempt."
        ],
        "expected_evidence_from_target_pr": [
            "implementation_receipt.json",
            "external_pr_receipt.json",
            "mcp_tool_call_receipt.json"
        ],
        "implementation_start_gate": {
            "human_decisions_resolved": human_resolved,
            "target_repo_changes_allowed_by_this_repo": start_allowed,
            "target_repo_pr_creation_allowed_by_this_packet": start_allowed,
            "notes": if start_allowed {
                "dry-run gate passed and live implementer may mutate target repo through Codex MCP."
            } else {
                "live implementer is blocked until Human Decision and dry-run gate pass."
            }
        },
        "forge_mapping": {
            "claim_ids": ["CLM-FDA-V1-MCP-LIVE-001"],
            "proof_obligations": ["PROOF-FDA-V1-MCP-LIVE-001"],
            "gate_requirements": ["Development Gate", "MCP Dry-run Gate", "Human Decision guard", "External PR receipt"]
        }
    })
}

pub(crate) fn mcp_agent_invocation_plan_live(
    target_repo: &Path,
    context: &RuntimeContext,
    guard: &HumanDecisionGuard,
    status: &str,
) -> Value {
    let gate_verdict =
        if guard.unresolved_decision_ids.is_empty() && guard.non_approval_decision_ids.is_empty() {
            "clear"
        } else {
            "blocked"
        };
    json!({
        "schema_version": "fda.mcp_agent_invocation_plan.v0",
        "plan_id": "MCP-FDA-V1-LIVE-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-006",
        "status": status,
        "human_decision_guard": {
            "unresolved_decision_ids": guard.unresolved_decision_ids,
            "non_approval_decision_ids": guard.non_approval_decision_ids,
            "gate_verdict": gate_verdict
        },
        "role_policy_ref": "agent_role_policy.json",
        "invocations": [
            {
                "invocation_id": "INV-FDA-V1-IMPLEMENTER-LIVE-001",
                "role": "implementer",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "write",
                "source_mutation_allowed": true,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "codex_live_prompt.md",
                "input_artifacts": ["planned_pr_execution_packet.json", "implementation_handoff.md", "dry_run_receipt.json"],
                "allowed_tools": ["codex", "codex-reply"],
                "forbidden_actions": ["merge", "release", "scope_change_without_human_decision"],
                "expected_receipts": ["mcp_tool_call_receipt.json", "implementation_receipt.json", "external_pr_receipt.json"]
            },
            {
                "invocation_id": "INV-FDA-V1-FQA-LIVE-PLACEHOLDER-001",
                "role": "functional_qa",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "PR-V1-007-functional-qa-prompt",
                "input_artifacts": ["planned_pr_execution_packet.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex"],
                "forbidden_actions": ["source_mutation", "security_exception_approval", "merge"],
                "expected_receipts": ["functional_qa_receipt.json"]
            },
            {
                "invocation_id": "INV-FDA-V1-SQA-LIVE-PLACEHOLDER-001",
                "role": "security_qa",
                "agent_provider": "codex",
                "mcp_server": {
                    "command": ["codex", "mcp-server"],
                    "transport": "stdio",
                    "tools_list_required": true
                },
                "tool_name": "codex",
                "thread_policy": "new_thread",
                "workspace_policy": "read_only",
                "source_mutation_allowed": false,
                "cwd": target_repo.to_string_lossy(),
                "prompt_artifact": "PR-V1-007-security-qa-prompt",
                "input_artifacts": ["planned_pr_execution_packet.json", "external_pr_receipt.json"],
                "allowed_tools": ["codex"],
                "forbidden_actions": ["source_mutation", "functional_qa_copy_paste", "risk_self_approval"],
                "expected_receipts": ["security_qa_receipt.json"]
            }
        ],
        "expected_global_receipts": ["mcp_tool_call_receipt.json", "implementation_receipt.json", "external_pr_receipt.json", "coding_agent_thread_state.json"]
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn mcp_tool_call_receipt_live(
    target_repo: &Path,
    result: &CodexLiveInvocationResult,
    status: &str,
    semantic_verdict: &str,
    gate_effect: &str,
    test_status: &str,
    scope_drift: &[String],
    time_window: (&str, &str),
) -> Value {
    let test_verdict = match test_status {
        "passed" => "pass",
        "failed" => "fail",
        _ => "not_run",
    };
    json!({
        "schema_version": "fda.mcp_tool_call_receipt.v0",
        "receipt_id": "MCPR-FDA-V1-LIVE-001",
        "plan_id": "MCP-FDA-V1-LIVE-001",
        "invocation_id": "INV-FDA-V1-IMPLEMENTER-LIVE-001",
        "role": "implementer",
        "provider": "codex",
        "mcp_server": "codex mcp-server",
        "tool_name": "codex",
        "thread_id": result.thread_id,
        "cwd": target_repo.to_string_lossy(),
        "status": status,
        "started_at": time_window.0,
        "completed_at": time_window.1,
        "input_artifacts": ["mcp_agent_invocation_plan.json", "codex_live_prompt.md", "planned_pr_execution_packet.json"],
        "output_artifacts": ["implementation_receipt.json", "external_pr_receipt.json", "coding_agent_thread_state.json"],
        "tool_result_digest": {
            "raw_result_stored": false,
            "summary": result.summary,
            "exit_code": result.exit_code
        },
        "semantic_result": {
            "semantic_verdict": semantic_verdict,
            "summary": result.summary,
            "scope_drift": scope_drift,
            "tests": [
                {
                    "command": marker_value(&result.content, "FDA_TESTS_RUN:").unwrap_or_else(|| "FDA_TESTS_RUN marker not returned".to_string()),
                    "verdict": test_verdict,
                    "evidence": test_status
                }
            ],
            "gate_effect": gate_effect
        },
        "evidence_links": ["implementation_receipt.json", "external_pr_receipt.json"],
        "next_action": if status == "succeeded" { "fda review" } else { "resolve live implementation failure and rerun fda implement --live" }
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn implementation_receipt(
    target_repo: &Path,
    context: &RuntimeContext,
    dry_run_gate: &DryRunGateStatus,
    result: &CodexLiveInvocationResult,
    status: &str,
    actual_pr_url: Option<&str>,
    test_status: &str,
    changed_files: &[String],
    scope_drift: &[String],
    time_window: (&str, &str),
) -> Value {
    json!({
        "schema_version": "fda.implementation_receipt.v0",
        "receipt_id": "IMPL-FDA-V1-006-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "planned_pr_id": "PR-V1-006",
        "provider": "codex",
        "mcp_server_command": ["codex", "mcp-server"],
        "cwd": target_repo.to_string_lossy(),
        "status": status,
        "started_at": time_window.0,
        "completed_at": time_window.1,
        "thread_id": result.thread_id,
        "dry_run_gate": {
            "status": dry_run_gate.status,
            "issues": dry_run_gate.issues,
            "evidence_links": dry_run_gate.evidence_links
        },
        "actual_pr_url": actual_pr_url,
        "changed_files": changed_files,
        "tests": [
            {
                "command": marker_value(&result.content, "FDA_TESTS_RUN:").unwrap_or_else(|| "FDA_TESTS_RUN marker not returned".to_string()),
                "status": test_status,
                "summary": if test_status == "passed" { "Implementer reported passing tests." } else { "Implementer did not report passing tests." }
            }
        ],
        "scope_drift": scope_drift,
        "input_artifacts": ["implementation_handoff.md", "codex_live_prompt.md", "planned_pr_execution_packet.json", "dry_run_receipt.json"],
        "output_artifacts": ["external_pr_receipt.json", "coding_agent_thread_state.json"],
        "evidence_links": ["mcp_tool_call_receipt.json", "external_pr_receipt.json"],
        "next_action": if status == "succeeded" { "fda review" } else { "repair live implementer output and rerun fda implement --live" },
        "summary": result.summary
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn external_pr_receipt(
    target_repo: &Path,
    context: &RuntimeContext,
    implementation_status: &str,
    actual_pr_url: Option<&str>,
    test_status: &str,
    test_command: &str,
    changed_files: &[String],
    scope_drift: &[String],
    summary: &str,
    target_head_sha: &str,
) -> Value {
    let url = actual_pr_url.unwrap_or("unavailable:codex_live_no_actual_pr");
    let external_status = if actual_pr_url.is_some() {
        "opened"
    } else {
        "blocked"
    };
    let target_pr_state = if actual_pr_url.is_some() {
        "open"
    } else {
        "closed"
    };
    let test_check = match test_status {
        "passed" => "passed",
        "failed" => "failed",
        _ => "not_run",
    };
    let ac_status = if implementation_status == "succeeded" {
        "pass"
    } else {
        "fail"
    };
    json!({
        "schema_version": "external_pr_receipt.v0",
        "receipt_id": "EXTPR-FDA-V1-006-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "source_packet_id": "PPEXEC-FDA-V1-006-001",
        "target_repo": target_repo.to_string_lossy(),
        "planned_pr_id": "PR-V1-006",
        "actual_pr_url": url,
        "status": external_status,
        "checks": {
            "tests": test_check,
            "lint": "not_run",
            "security": "not_run"
        },
        "evidence": ["implementation_receipt.json", "mcp_tool_call_receipt.json"],
        "human_decisions_resolved": [],
        "open_issues": if implementation_status == "succeeded" {
            Vec::<String>::new()
        } else {
            vec![summary.to_string()]
        },
        "scope_disposition": {
            "kind": if scope_drift.is_empty() { "within_scope" } else { "scope_deviation" },
            "closure_recommendation": if scope_drift.is_empty() && implementation_status == "succeeded" { "close_planned_pr" } else { "manual_review_required" },
            "affected_planned_pr_ids": ["PR-V1-006"],
            "summary": if scope_drift.is_empty() { "No scope drift was reported by the implementer." } else { "Scope drift was reported by the implementer." }
        },
        "target_pr": {
            "url": url,
            "number": actual_pr_url.and_then(parse_pr_number_from_url),
            "head_sha": target_head_sha,
            "state": target_pr_state
        },
        "changed_files": changed_files,
        "acceptance_criteria_status": [
            {
                "criterion": "planned PR と actual PR が対応し、test 結果が receipt に残る",
                "status": ac_status,
                "evidence": "implementation_receipt.json"
            }
        ],
        "validation": [
            {
                "command": test_command,
                "status": if test_status == "passed" { "pass" } else if test_status == "failed" { "fail" } else { "not_run" },
                "summary": test_status
            }
        ],
        "security_privacy_legal_review": "PR-V1-007でFunctional QA / Security QAを分離実行する。",
        "rollback_plan": "必要に応じてtarget PRをcloseし、target branchをrevertまたは削除する。",
        "notes": summary
    })
}

pub(crate) fn coding_agent_thread_state(
    target_repo: &Path,
    result: &CodexLiveInvocationResult,
    status: &str,
    actual_pr_url: Option<&str>,
) -> Value {
    let thread_status = match status {
        "succeeded" => "completed",
        "blocked" | "adapter_unavailable" => "blocked",
        _ => "waiting_for_reply",
    };
    let failure_classification = match status {
        "succeeded" => Value::Null,
        "adapter_unavailable" => json!("adapter_unavailable"),
        "blocked" if actual_pr_url.is_none() => json!("missing_actual_pr_url_or_preflight_blocked"),
        _ => json!("implementation_failed"),
    };
    json!({
        "schema_version": "fda.coding_agent_thread_state.v0",
        "thread_state_id": "THREAD-FDA-V1-006-001",
        "provider": "codex",
        "role": "implementer",
        "status": thread_status,
        "started_from_invocation_id": "INV-FDA-V1-IMPLEMENTER-LIVE-001",
        "planned_pr_id": "PR-V1-006",
        "actual_pr_url": actual_pr_url,
        "cwd": target_repo.to_string_lossy(),
        "thread_id": result.thread_id,
        "continuation_tool_name": "codex-reply",
        "current_turn": if result.thread_id.is_some() { 1 } else { 0 },
        "repair_attempt_count": 0,
        "failure_classification": failure_classification,
        "last_prompt_artifact": "codex_live_prompt.md",
        "last_receipt_id": "IMPL-FDA-V1-006-001",
        "summary": result.summary,
        "open_items": if status == "succeeded" { Vec::<String>::new() } else { vec![result.summary.clone()] },
        "evidence_links": ["implementation_receipt.json", "external_pr_receipt.json", "mcp_tool_call_receipt.json"]
    })
}

pub(crate) struct LiveExecutionEvidence<'a> {
    pub(crate) target_repo: &'a Path,
    pub(crate) context: &'a RuntimeContext,
    pub(crate) dry_run_gate: &'a DryRunGateStatus,
    pub(crate) result: &'a CodexLiveInvocationResult,
    pub(crate) implementation_status: &'a str,
    pub(crate) actual_pr_url: Option<&'a str>,
    pub(crate) test_status: &'a str,
    pub(crate) changed_files: &'a [String],
    pub(crate) scope_drift: &'a [String],
    pub(crate) detected_tools: &'a [String],
    pub(crate) missing_tools: &'a [String],
    pub(crate) tools_list_source: &'a str,
    pub(crate) codex_tool_source: &'a str,
    pub(crate) fixture_used: bool,
    pub(crate) time_window: (&'a str, &'a str),
}

pub(crate) fn live_execution_evidence(input: LiveExecutionEvidence<'_>) -> Value {
    let fixture_free_status = if input.implementation_status == "succeeded" && !input.fixture_used {
        "pass"
    } else if input.fixture_used {
        "blocked"
    } else {
        "fail"
    };
    let status = if input.fixture_used {
        "fixture_mode"
    } else {
        input.implementation_status
    };
    json!({
        "schema_version": "fda.live_execution_evidence.v0",
        "evidence_id": "LIVE-EVIDENCE-FDA-V1-015-001",
        "program_id": input.context.program_id,
        "epic_id": input.context.epic_id,
        "planned_pr_id": "PR-V1-006",
        "operational_completion_pr_id": "PR-V1-015",
        "status": status,
        "implementation_status": input.implementation_status,
        "fixture_free_required_for_operational_v1": true,
        "fixture_used": input.fixture_used,
        "fixture_free_gate": {
            "status": fixture_free_status,
            "summary": if input.fixture_used {
                "Live execution used a fixture; this is valid for unit tests but not sufficient for Operational V1 proof."
            } else if input.implementation_status == "succeeded" {
                "Codex MCP live execution completed without fixture input."
            } else {
                "Codex MCP live execution did not complete successfully without fixtures."
            }
        },
        "mcp": {
            "server_command": ["codex", "mcp-server"],
            "tools_list_source": input.tools_list_source,
            "codex_tool_source": input.codex_tool_source,
            "codex_tool_call_sent": input.result.tool_call_sent,
            "expected_tools": ["codex", "codex-reply"],
            "detected_tools": input.detected_tools,
            "missing_tools": input.missing_tools,
            "thread_id": input.result.thread_id,
            "exit_code": input.result.exit_code
        },
        "target_repo": input.target_repo.to_string_lossy(),
        "dry_run_gate": {
            "status": input.dry_run_gate.status,
            "issues": input.dry_run_gate.issues,
            "evidence_links": input.dry_run_gate.evidence_links
        },
        "implementation_result": {
            "actual_pr_url": input.actual_pr_url,
            "test_status": input.test_status,
            "changed_files": input.changed_files,
            "scope_drift": input.scope_drift,
            "summary": input.result.summary
        },
        "started_at": input.time_window.0,
        "completed_at": input.time_window.1,
        "evidence_links": [
            "mcp_tool_call_receipt.json",
            "implementation_receipt.json",
            "external_pr_receipt.json",
            "coding_agent_thread_state.json"
        ],
        "next_action": if input.implementation_status == "succeeded" && !input.fixture_used {
            "fda review"
        } else if input.fixture_used {
            "fixtureなしの Codex MCP live 実行証跡を取得する"
        } else {
            "Codex MCP live 実行の失敗理由を修正して fda implement --live を再実行する"
        }
    })
}

pub(crate) fn implement_live_runner_explanation(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    status: &str,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "external_implementation_live",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda implement --live from {}", display_path(repo_root, artifact_dir)),
            "changed_input_summary": Value::Null,
            "stop_condition": format!("development_gate_{status}"),
            "next_action": if status == "succeeded" { "fda review" } else { "fix live implementation gate and rerun fda implement --live" },
            "automation_boundary": "PR-V1-006 runs the Implementer only; Functional QA, Security QA, repair loop, merge, release, notification, and Output Hub are later gates",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("implementation_receipt.json")),
                display_path(repo_root, &out_dir.join("external_pr_receipt.json")),
                display_path(repo_root, &out_dir.join("mcp_tool_call_receipt.json")),
                display_path(repo_root, &out_dir.join("live_execution_evidence.json"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}

pub(crate) fn implement_live_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
) -> Value {
    let now = now_unix_seconds();
    let specs = [
        (
            "ART-LIVE-001",
            "development_handoff",
            "Implementation Handoff",
            "Live implementer handoff",
            "implementation_handoff.md",
        ),
        (
            "ART-LIVE-002",
            "generic_receipt",
            "Codex Live Prompt",
            "Codex MCP live implementer prompt",
            "codex_live_prompt.md",
        ),
        (
            "ART-LIVE-003",
            "agent_role_policy",
            "Agent Role Policy",
            "Implementer / QA role policy",
            "agent_role_policy.json",
        ),
        (
            "ART-LIVE-004",
            "planned_pr_execution_packet",
            "Planned PR Execution Packet",
            "Live execution packet",
            "planned_pr_execution_packet.json",
        ),
        (
            "ART-LIVE-005",
            "mcp_agent_invocation_plan",
            "MCP Agent Invocation Plan",
            "Codex MCP live invocation plan",
            "mcp_agent_invocation_plan.json",
        ),
        (
            "ART-LIVE-006",
            "mcp_tool_call_receipt",
            "MCP Tool Call Receipt",
            "Codex tool semantic receipt",
            "mcp_tool_call_receipt.json",
        ),
        (
            "ART-LIVE-007",
            "implementation_receipt",
            "Implementation Receipt",
            "Live implementer semantic receipt",
            "implementation_receipt.json",
        ),
        (
            "ART-LIVE-008",
            "external_pr_receipt",
            "External PR Receipt",
            "Actual PR and test evidence receipt",
            "external_pr_receipt.json",
        ),
        (
            "ART-LIVE-009",
            "coding_agent_thread_state",
            "Coding Agent Thread State",
            "Codex thread continuation state",
            "coding_agent_thread_state.json",
        ),
        (
            "ART-LIVE-010",
            "live_execution_evidence",
            "Live Execution Evidence",
            "Fixture-free Codex MCP live evidence gate",
            "live_execution_evidence.json",
        ),
        (
            "ART-LIVE-011",
            "runner_explanation",
            "Runner Explanation",
            "Live gate stop condition and next action",
            "runner_explanation.json",
        ),
        (
            "ART-LIVE-012",
            "validation_report",
            "Validation Report",
            "Live artifact schema validation",
            "validation_report.json",
        ),
    ];
    let artifacts = specs
        .iter()
        .map(
            |(artifact_id, artifact_type, title, preview_summary, file_name)| {
                artifact_inventory_entry(
                    repo_root,
                    out_dir,
                    context,
                    ArtifactInventorySpec {
                        artifact_id,
                        artifact_type,
                        title,
                        preview_summary,
                        file_name,
                        timestamp: now,
                    },
                )
            },
        )
        .collect::<Vec<_>>();
    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": artifacts,
        "groups": [
            {
                "group_id": "mcp_live",
                "title": "MCP Live Implementer Artifacts",
                "artifact_ids": ["ART-LIVE-001", "ART-LIVE-002", "ART-LIVE-003", "ART-LIVE-004", "ART-LIVE-005", "ART-LIVE-006", "ART-LIVE-007", "ART-LIVE-008", "ART-LIVE-009"]
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-LIVE-010", "ART-LIVE-011", "ART-LIVE-012"]
            }
        ]
    })
}

pub(crate) fn implement_runner_explanation(
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    receipt_status: &str,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "external_implementation_handoff",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda implement --dry-run from {}", display_path(repo_root, artifact_dir)),
            "changed_input_summary": Value::Null,
            "stop_condition": format!("current_codex_cli_handoff_{receipt_status}"),
            "next_action": if receipt_status == "succeeded" { "current Codex CLIでhandoffに従って実装し、PR作成後 fda review" } else { "fix dry-run gate failure and rerun" },
            "automation_boundary": "V1 primary is current Codex CLI handoff. This dry-run does not mutate target repo, create branch, create PR, merge, release, deploy, notify, or generate Output Hub. MCP artifacts are V1.5 optional automation compatibility.",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("current_codex_cli_handoff.json")),
                display_path(repo_root, &out_dir.join("mcp_agent_invocation_plan.json")),
                display_path(repo_root, &out_dir.join("mcp_tool_call_receipt.json")),
                display_path(repo_root, &out_dir.join("dry_run_receipt.json"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}

pub(crate) fn implement_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
) -> Value {
    let now = now_unix_seconds();
    let specs = [
        (
            "ART-IMPL-001",
            "development_handoff",
            "Implementation Handoff",
            "Dry-run用実装handoff",
            "implementation_handoff.md",
        ),
        (
            "ART-IMPL-002",
            "generic_receipt",
            "Codex Prompt",
            "Codex MCP dry-run prompt",
            "codex_prompt.md",
        ),
        (
            "ART-IMPL-003",
            "generic_receipt",
            "Functional QA Prompt",
            "Functional QA MCP dry-run prompt",
            "functional_qa_prompt.md",
        ),
        (
            "ART-IMPL-004",
            "generic_receipt",
            "Security QA Prompt",
            "Security QA MCP dry-run prompt",
            "security_qa_prompt.md",
        ),
        (
            "ART-IMPL-005",
            "agent_role_policy",
            "Agent Role Policy",
            "Implementer / QA role policy",
            "agent_role_policy.json",
        ),
        (
            "ART-IMPL-012",
            "current_codex_cli_handoff",
            "Current Codex CLI Handoff",
            "Current Codex CLI primary implementer handoff",
            "current_codex_cli_handoff.json",
        ),
        (
            "ART-IMPL-006",
            "planned_pr_execution_packet",
            "Planned PR Execution Packet",
            "Current Codex CLI implementation gate packet",
            "planned_pr_execution_packet.json",
        ),
        (
            "ART-IMPL-007",
            "mcp_agent_invocation_plan",
            "MCP Agent Invocation Plan",
            "Codex MCP dry-run invocation plan",
            "mcp_agent_invocation_plan.json",
        ),
        (
            "ART-IMPL-008",
            "mcp_tool_call_receipt",
            "MCP Tool Call Receipt",
            "tools/list semantic receipt",
            "mcp_tool_call_receipt.json",
        ),
        (
            "ART-IMPL-009",
            "dry_run_receipt",
            "MCP Dry-run Receipt",
            "tools/list / cwd / prompt / policy dry-run receipt",
            "dry_run_receipt.json",
        ),
        (
            "ART-IMPL-010",
            "runner_explanation",
            "Runner Explanation",
            "Dry-run停止条件と次action",
            "runner_explanation.json",
        ),
        (
            "ART-IMPL-011",
            "validation_report",
            "Validation Report",
            "Dry-run artifact schema validation",
            "validation_report.json",
        ),
    ];
    let artifacts = specs
        .iter()
        .map(
            |(artifact_id, artifact_type, title, preview_summary, file_name)| {
                artifact_inventory_entry(
                    repo_root,
                    out_dir,
                    context,
                    ArtifactInventorySpec {
                        artifact_id,
                        artifact_type,
                        title,
                        preview_summary,
                        file_name,
                        timestamp: now,
                    },
                )
            },
        )
        .collect::<Vec<_>>();
    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": artifacts,
        "groups": [
            {
                "group_id": "current_codex_cli",
                "title": "Current Codex CLI Primary Handoff",
                "artifact_ids": ["ART-IMPL-001", "ART-IMPL-005", "ART-IMPL-006", "ART-IMPL-012"]
            },
            {
                "group_id": "mcp_dry_run",
                "title": "V1.5 Optional MCP Dry-run Artifacts",
                "artifact_ids": ["ART-IMPL-002", "ART-IMPL-003", "ART-IMPL-004", "ART-IMPL-007", "ART-IMPL-008", "ART-IMPL-009"]
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-IMPL-010", "ART-IMPL-011"]
            }
        ]
    })
}
