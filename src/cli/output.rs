use std::path::Path;

use crate::application::decide::DecideResult;
use crate::application::design::DesignResult;
use crate::application::gc::GcResult;
use crate::application::plan::PlanResult;
use crate::application::start::StartResult;
use crate::application::status::StatusResult;
use crate::application::validate::ValidationReport;
use crate::{ContinueResult, ImplementResult, MergeResult, NotifyResult, OpenResult, ReviewResult};

pub(crate) fn print_help() {
    println!("fda — AI Delivery Runtime (work protocol: docs/v1/work_protocol.md)");
    println!();
    println!("[作業 Work]");
    println!("fda start <goal> [--out <dir>] [--mode auto|implement|research|uiux|design-only] [--repo-root <path>] [--json]");
    println!("fda start --input <path> [--out <dir>] [--mode auto|implement|research|uiux|design-only] [--repo-root <path>] [--json]");
    println!("fda design [--artifacts <dir>] [--out <dir>] [--repo-root <path>] [--json]");
    println!("fda plan --requirements <path> --out <dir> --mode fixture [--repo-root <path>] [--fixture-dir <dir>] [--json]");
    println!("fda implement --dry-run [--artifacts <dir>] [--out <dir>] [--target-repo <path>] [--repo-root <path>] [--json]");
    println!("fda implement --live [--artifacts <dir>] [--out <dir>] [--target-repo <path>] [--live-timeout-seconds <n>] [--repo-root <path>] [--json]");
    println!("fda continue [--artifacts <dir>] [--out <dir>] [--target-repo <path>] [--max-retries <n>] [--repo-root <path>] [--json]");
    println!("fda merge [--artifacts <dir>] [--out <dir>] [--target-repo <path>] [--repo-root <path>] [--execute] [--merge-method merge|squash|rebase] [--json]");
    println!();
    println!("[判断 Decision]");
    println!("fda decide <decision-id> --answer <answer> [--artifacts <dir>] [--decided-by <actor>] [--repo-root <path>] [--json]");
    println!("fda notify test [--artifacts <dir>] [--out <dir>] [--channel slack|email|codex-app] [--to <recipient>] [--live] [--repo-root <path>] [--json]");
    println!();
    println!("[証跡 Evidence]");
    println!("fda review [--artifacts <dir>] [--out <dir>] [--target-repo <path>] [--repo-root <path>] [--json]");
    println!("fda status [--artifacts <dir>] [--repo-root <path>] [--json]");
    println!("fda open [--artifacts <dir>] [--out <dir>] [--repo-root <path>] [--json]");
    println!("fda ui [--artifacts-root <dir>] [--port <n>] [--open] [--repo-root <path>]  # read-only Mission Control (127.0.0.1)");
    println!("fda gc [--artifacts-root <dir>] [--max-age-days <n>] [--repo-root <path>] [--json]  # stale run 棚卸し docket (read-only, 削除しない)");
    println!("fda validate-artifacts [--repo-root <path>] [--schemas <dir>] [--artifacts <dir>] [--model-contracts <dir>] [--out <path>] [--json]");
    println!();
    println!("共通ATO連携: [--ato-sync] [--ato-task <key>] [--ato-run-id <run>] [--ato-backend <backend>] [--ato-db <path>] [--ato-cli <path>]");
    println!("[知識 Knowledge] は ato knowledge / ato search を使う (正本: ATO)");
}

pub(crate) fn print_start_summary(result: &StartResult) {
    println!("Requirements Definition を作成しました。");
    println!();
    println!("実装可否分類: {}", result.implementation_classification);
    println!();
    println!("判断が必要です:");
    for (index, decision) in result.human_decisions.iter().enumerate() {
        println!(
            "{}. {}: {}",
            index + 1,
            decision.decision_id,
            decision.summary
        );
    }
    println!();
    println!("続行するには:");
    for action in &result.next_actions {
        println!("{action}");
    }
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
}

pub(crate) fn print_decide_summary(result: &DecideResult) {
    println!("{} を記録しました。", result.decision_id);
    println!();
    if result.unresolved_decisions.is_empty() {
        println!("Human Decision はすべて解決済みです。");
    } else {
        println!("未解決判断:");
        for decision in &result.unresolved_decisions {
            println!("- {}: {}", decision.decision_id, decision.summary);
        }
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_design_summary(result: &DesignResult) {
    if result.verdict == "blocked" {
        println!("Design Gate で停止しました。");
        println!();
        println!("判断が必要です:");
        for decision in &result.blocked_decisions {
            println!("- {}: {}", decision.decision_id, decision.summary);
        }
        println!();
        println!("次:");
        for action in &result.next_actions {
            println!("{action}");
        }
        return;
    }

    println!("Design Gate を通過しました。");
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_implement_summary(result: &ImplementResult) {
    if result.mode == "live" {
        println!(
            "Development gate: {}",
            result
                .development_gate_status
                .as_deref()
                .unwrap_or("<unknown>")
        );
        println!("MCP dry-run gate: {}", result.dry_run_gate_status);
        if let Some(actual_pr_url) = &result.actual_pr_url {
            println!("Actual PR: {actual_pr_url}");
        }
        if let Some(thread_id) = &result.thread_id {
            println!("Codex thread: {thread_id}");
        }
    } else {
        println!("MCP dry-run gate: {}", result.dry_run_gate_status);
    }
    println!();
    println!("検出 tool:");
    if result.detected_tools.is_empty() {
        println!("- <none>");
    } else {
        for tool in &result.detected_tools {
            println!("- {tool}");
        }
    }
    if !result.missing_tools.is_empty() {
        println!();
        println!("不足 tool:");
        for tool in &result.missing_tools {
            println!("- {tool}");
        }
    }
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_review_summary(result: &ReviewResult) {
    println!("Functional QA Gate: {}", result.functional_qa_status);
    println!("Security QA Gate: {}", result.security_qa_status);
    println!("QA verdict: {}", result.verdict);
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_continue_summary(result: &ContinueResult) {
    println!("Repair Loop Gate: {}", result.repair_loop_status);
    println!("Failure classification: {}", result.failure_classification);
    println!(
        "Retry: {}/{}",
        result.retry_attempt_count, result.retry_limit
    );
    println!("Verdict: {}", result.verdict);
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_merge_summary(result: &MergeResult) {
    println!("Merge Gate: {}", result.merge_gate_status);
    println!("Policy disposition: {}", result.policy_disposition);
    println!("CI status: {}", result.ci_status);
    println!("Risk classification: {}", result.risk_classification);
    if let Some(risk_tier) = &result.risk_tier {
        println!("Risk tier: {risk_tier}");
    }
    if !result.proportional_gate_notes.is_empty() {
        println!("比例ゲート:");
        for note in &result.proportional_gate_notes {
            println!("- {note}");
        }
    }
    if let Some(actual_pr_url) = &result.actual_pr_url {
        println!("Actual PR: {actual_pr_url}");
    }
    println!("Merge execution: {}", result.merge_execution_status);
    println!("Merge method: {}", result.merge_method);
    if let Some(path) = &result.github_merge_receipt_path {
        println!("GitHub merge receipt: {path}");
    }
    if let Some(reason) = &result.merge_failure_reason {
        println!("Merge failure: {reason}");
    }
    println!("Verdict: {}", result.verdict);
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_open_summary(result: &OpenResult) {
    println!("Output Hub: {}", result.output_hub_path);
    println!("Decision Inbox: {}", result.decision_inbox_path);
    println!("Execution Status: {}", result.execution_status_path);
    println!("Verdict: {}", result.verdict);
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_notify_summary(result: &NotifyResult) {
    println!("Notification: {}", result.notification_status);
    println!("Channel: {}", result.channel);
    println!("Recipient: {}", result.recipient);
    println!("Verdict: {}", result.verdict);
    println!();
    println!("成果物:");
    for artifact in &result.artifacts_written {
        println!("- {artifact}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_status_summary(result: &StatusResult) {
    println!("Current phase: {}", result.current_phase);
    println!("Reason: {}", result.phase_reason);
    println!("Artifact dir: {}", result.artifact_dir);
    println!();
    if result.unresolved_decisions.is_empty() {
        println!("未解決 Human Decision: なし");
    } else {
        println!("未解決 Human Decision:");
        for decision in &result.unresolved_decisions {
            println!(
                "- {}: {} (required before: {})",
                decision.decision_id, decision.summary, decision.required_before
            );
        }
    }
    println!();
    println!("Notification:");
    println!("- request: {}", result.notification.request_status);
    println!("- receipt: {}", result.notification.receipt_status);
    if let Some(channel) = &result.notification.channel {
        println!("- channel: {channel}");
    }
    if let Some(recipient) = &result.notification.recipient {
        println!("- recipient: {recipient}");
    }
    if let Some(sent) = result.notification.sent {
        println!("- sent: {sent}");
    }
    println!();
    println!("QA / Repair / Merge:");
    println!("- Functional QA Gate: {}", result.qa.functional_qa_status);
    println!("- Security QA Gate: {}", result.qa.security_qa_status);
    println!("- QA Gate: {}", result.qa.qa_status);
    if let Some(return_to_role) = &result.qa.return_to_role {
        println!("- Return to role: {return_to_role}");
    }
    println!("- Repair Loop Gate: {}", result.repair.repair_loop_status);
    if let Some(failure_classification) = &result.repair.failure_classification {
        println!("- Failure classification: {failure_classification}");
    }
    println!("- Merge Gate: {}", result.merge.merge_gate_status);
    if let Some(policy_disposition) = &result.merge.policy_disposition {
        println!("- Policy disposition: {policy_disposition}");
    }
    if let Some(ci_status) = &result.merge.ci_status {
        println!("- CI status: {ci_status}");
    }
    if let Some(risk_classification) = &result.merge.risk_classification {
        println!("- Risk classification: {risk_classification}");
    }
    if let Some(risk_tier) = &result.merge.risk_tier {
        println!("- Risk tier: {risk_tier}");
    }
    if let Some(actual_pr_url) = &result.merge.actual_pr_url {
        println!("- Actual PR: {actual_pr_url}");
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_gc_summary(result: &GcResult) {
    println!("GC docket: {}", result.docket_path);
    println!("Artifacts root: {}", result.artifacts_root);
    println!("Scanned runs: {}", result.scanned_runs);
    println!("Candidates: {}", result.candidate_count);
    println!();
    if result.candidates.is_empty() {
        println!("棚卸し候補はありません。");
    } else {
        println!("棚卸し候補 (削除・変更はしません):");
        for candidate in &result.candidates {
            println!(
                "- {} [{}]{}",
                candidate.run,
                candidate.recommendation,
                if candidate.needs_human {
                    " (要人間判断)"
                } else {
                    ""
                }
            );
            for reason in &candidate.reasons {
                println!("    - {reason}");
            }
        }
    }
    println!();
    println!("次:");
    for action in &result.next_actions {
        println!("{action}");
    }
}

pub(crate) fn print_plan_summary(result: &PlanResult) {
    println!(
        "fixture plan {}: {} artifacts written to {}",
        result.verdict,
        result.artifacts_written.len(),
        result.out_dir
    );
}

pub(crate) fn print_validation_summary(report: &ValidationReport, out: Option<&Path>) {
    println!(
        "validation {}: {} passed, {} failed, {} skipped",
        report.verdict, report.summary.passed, report.summary.failed, report.summary.skipped
    );
    if let Some(out) = out {
        println!("report: {}", out.display());
    }
}
