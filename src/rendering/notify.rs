use serde_json::{json, Value};
use std::path::Path;

use crate::rendering::output_hub::{DecisionView, StatusView};

const SLACK_SECTION_TEXT_LIMIT: usize = 3000;

pub(crate) struct NotificationRequestInput<'a> {
    pub(crate) repo_root: &'a Path,
    pub(crate) artifact_dir: &'a Path,
    pub(crate) project: &'a str,
    pub(crate) channel: &'a str,
    pub(crate) recipient: &'a str,
    pub(crate) recipient_source: &'a str,
    pub(crate) sendable: bool,
    pub(crate) decisions: &'a [DecisionView],
    pub(crate) live: bool,
}

pub(crate) fn notification_request(input: NotificationRequestInput<'_>) -> Value {
    let repo_root = input.repo_root;
    let artifact_dir = input.artifact_dir;
    let repo_name = input.project;
    let decision_document_path = artifact_dir.join("human_decision_packet.md");
    let primary_decision = input.decisions.first();
    let top_level_options = primary_decision
        .map(|decision| decision.options.clone())
        .unwrap_or_default();
    let top_level_recommended_option = primary_decision
        .map(|decision| decision.recommended_option_id.clone())
        .unwrap_or_default();
    let top_level_resume_command = primary_decision
        .map(|decision| decision.resume_command.clone())
        .unwrap_or_default();
    let decision_requests = input
        .decisions
        .iter()
        .map(|decision| {
            json!({
                "decision_id": decision.decision_id.clone(),
                "summary": decision.summary.clone(),
                "required_before": decision.required_before.clone(),
                "status": decision.status.clone(),
                "options": decision.options.clone(),
                "recommended_option_id": decision.recommended_option_id.clone(),
                "resume_command": decision.resume_command.clone()
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": "fda.notification_request.v0",
        "notification_id": "NOTIFY-FDA-V1-011-001",
        "mode": if input.live { "live" } else { "dry_run" },
        "channel": input.channel,
        "recipient": input.recipient,
        "recipient_source": input.recipient_source,
        "sendable": input.sendable && !input.decisions.is_empty(),
        "reason": "human_turn_notice",
        "repo_name": repo_name,
        "repo_root": repo_root.to_string_lossy(),
        "project": repo_name,
        "artifact_dir": artifact_dir.to_string_lossy(),
        "decision_document_path": decision_document_path.to_string_lossy(),
        "decision_ids": input.decisions.iter().map(|decision| decision.decision_id.clone()).collect::<Vec<_>>(),
        "decisions": decision_requests,
        "options": top_level_options,
        "recommended_option": top_level_recommended_option,
        "resume_command": top_level_resume_command,
        "summary": if input.decisions.is_empty() { "No open Human Decision found in artifact dir" } else { "Human Decision requires attention" },
        "resume_commands": input.decisions.iter().map(|decision| decision.resume_command.clone()).collect::<Vec<_>>()
    })
}

pub(crate) fn notification_receipt(request: &Value) -> Value {
    json!({
        "schema_version": "fda.notification_receipt.v0",
        "receipt_id": "NOTIFY-RECEIPT-FDA-V1-011-001",
        "notification_id": value_string(request, "notification_id").unwrap_or_default(),
        "status": "skipped",
        "dry_run": true,
        "skip_reason": "dry_run_no_delivery_attempted",
        "sent": false,
        "channel": value_string(request, "channel").unwrap_or_default(),
        "recipient": value_string(request, "recipient").unwrap_or_default(),
        "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
        "sendable": request.get("sendable").and_then(Value::as_bool).unwrap_or(false),
        "adapter_boundary": "PR-V1-011 writes notification artifacts only; real email/Codex app delivery is handled by a later adapter."
    })
}

pub(crate) fn human_turn_notice_markdown(
    decisions: &[DecisionView],
    statuses: &[StatusView],
) -> String {
    let decision_lines = if decisions.is_empty() {
        "- 未解決Decisionはありません。".to_string()
    } else {
        decisions
            .iter()
            .map(|decision| {
                let options = if decision.options.is_empty() {
                    "<none>".to_string()
                } else {
                    decision.options.join(", ")
                };
                format!(
                    "- {}: {} (`{}`)\n  - options: {}\n  - recommended: {}\n  - resume: `{}`",
                    decision.decision_id,
                    decision.summary,
                    decision.required_before,
                    options,
                    decision.recommended_option_id,
                    decision.resume_command
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let status_lines = statuses
        .iter()
        .map(|status| format!("- {}: {}", status.label, status.value))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# Human Turn Notice\n\n## Decisions\n\n{}\n\n## Status\n\n{}\n\n## Resume\n\n`fda decide <decision-id> --answer <answer>`\n",
        decision_lines, status_lines
    )
}

pub(crate) fn smtp_message_body(
    sender: &str,
    recipient: &str,
    request: &Value,
    message_id: &str,
) -> String {
    let encoded_body = smtp_base64_lines(smtp_plain_message_text(request).as_bytes());
    format!(
        "From: {}\r\nTo: {}\r\nSubject: [FDA] Human Decision requires attention\r\nMessage-ID: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n.\r\n",
        sender, recipient, message_id, encoded_body
    )
}

pub(crate) fn slack_message_payload(request: &Value) -> Value {
    let text = slack_plain_message_text(request);
    json!({
        "text": text,
        "blocks": [
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": truncate_slack_text(&slack_mrkdwn_message_text(request), SLACK_SECTION_TEXT_LIMIT)
                }
            }
        ]
    })
}

pub(crate) fn slack_plain_message_text(request: &Value) -> String {
    let decision_ids = request
        .get("decision_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "<none>".to_string());
    let repo_name =
        value_string(request, "repo_name").unwrap_or_else(|| "unknown-repo".to_string());
    let decision_document_path = value_string(request, "decision_document_path")
        .unwrap_or_else(|| "<unknown decision document path>".to_string());
    let decision_summaries = plain_decision_summaries(request);
    let decision_actions = plain_decision_actions(request);
    format!(
        "[FDA] Human Decision requires attention in {}: {}. Decision actions: {}. Decision summary: {}. Decision document: {}",
        slack_escape(&repo_name),
        slack_escape(&decision_ids),
        decision_actions,
        decision_summaries,
        slack_escape(&decision_document_path)
    )
}

fn slack_mrkdwn_message_text(request: &Value) -> String {
    let decision_ids = request
        .get("decision_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(slack_escape)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "&lt;none&gt;".to_string());
    let summary = slack_escape(
        &value_string(request, "summary")
            .unwrap_or_else(|| "Human Decision requires attention".to_string()),
    );
    let repo_name = slack_escape(
        &value_string(request, "repo_name").unwrap_or_else(|| "unknown-repo".to_string()),
    );
    let repo_root = slack_escape(
        &value_string(request, "repo_root").unwrap_or_else(|| "<unknown repo root>".to_string()),
    );
    let artifact_dir = slack_escape(
        &value_string(request, "artifact_dir")
            .unwrap_or_else(|| "<unknown artifact dir>".to_string()),
    );
    let decision_document_path = slack_escape(
        &value_string(request, "decision_document_path")
            .unwrap_or_else(|| "<unknown decision document path>".to_string()),
    );
    let options = request
        .get("options")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .take(5)
                .map(|option| format!("• {}", slack_escape(option)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "• &lt;no options recorded&gt;".to_string());
    let decision_summaries = mrkdwn_decision_summaries(request);
    let decision_actions = mrkdwn_decision_actions(request);
    format!(
        "*FDA Human Decision Required*\n*Repository / Project:* {repo_name}\n*Repo root:* `{repo_root}`\n*Artifact dir:* `{artifact_dir}`\n*Decision document:* `{decision_document_path}`\n*Summary:* {summary}\n*Decision:* {decision_ids}\n*Decision actions:*\n{decision_actions}\n*Options:*\n{options}\n*Decision summaries:*\n{decision_summaries}"
    )
}

fn plain_decision_summaries(request: &Value) -> String {
    decision_summary_entries(request)
        .into_iter()
        .map(|entry| slack_escape(&entry))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn mrkdwn_decision_summaries(request: &Value) -> String {
    let summaries = decision_summary_entries(request);
    if summaries.is_empty() {
        return "• &lt;no decision summary recorded&gt;".to_string();
    }
    summaries
        .into_iter()
        .map(|entry| format!("• {}", slack_escape(&entry)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn plain_decision_actions(request: &Value) -> String {
    decision_action_entries(request)
        .into_iter()
        .map(|entry| slack_escape(&entry))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn mrkdwn_decision_actions(request: &Value) -> String {
    let actions = decision_action_entries(request);
    if actions.is_empty() {
        return "• &lt;no decision action recorded&gt;".to_string();
    }
    actions
        .into_iter()
        .map(|entry| format!("• {}", slack_escape(&entry)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn decision_action_entries(request: &Value) -> Vec<String> {
    request
        .get("decisions")
        .and_then(Value::as_array)
        .map(|decisions| {
            decisions
                .iter()
                .take(5)
                .filter_map(|decision| {
                    let decision_id = value_string(decision, "decision_id").unwrap_or_default();
                    let recommended =
                        value_string(decision, "recommended_option_id").unwrap_or_default();
                    let resume = value_string(decision, "resume_command").unwrap_or_default();
                    if decision_id.is_empty() && recommended.is_empty() && resume.is_empty() {
                        None
                    } else {
                        Some(format!(
                            "{}: recommended={}, resume={}",
                            if decision_id.is_empty() {
                                "<unknown>"
                            } else {
                                &decision_id
                            },
                            if recommended.is_empty() {
                                "<none>"
                            } else {
                                &recommended
                            },
                            if resume.is_empty() {
                                "fda status"
                            } else {
                                &resume
                            }
                        ))
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|entries| !entries.is_empty())
        .unwrap_or_else(|| {
            vec![format!(
                "<top-level>: recommended={}, resume={}",
                value_string(request, "recommended_option").unwrap_or_else(|| "<none>".to_string()),
                value_string(request, "resume_command").unwrap_or_else(|| "fda status".to_string())
            )]
        })
}

fn decision_summary_entries(request: &Value) -> Vec<String> {
    request
        .get("decisions")
        .and_then(Value::as_array)
        .map(|decisions| {
            decisions
                .iter()
                .take(5)
                .filter_map(|decision| {
                    let decision_id = value_string(decision, "decision_id").unwrap_or_default();
                    let summary = value_string(decision, "summary").unwrap_or_default();
                    if decision_id.is_empty() && summary.is_empty() {
                        None
                    } else if summary.is_empty() {
                        Some(decision_id)
                    } else if decision_id.is_empty() {
                        Some(summary)
                    } else {
                        Some(format!("{decision_id}: {summary}"))
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|entries| !entries.is_empty())
        .unwrap_or_else(|| {
            vec![value_string(request, "summary")
                .unwrap_or_else(|| "Human Decision requires attention".to_string())]
        })
}

fn truncate_slack_text(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    let suffix = "...";
    let max_body_len = limit.saturating_sub(suffix.len());
    let cut = value
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= max_body_len)
        .last()
        .unwrap_or(0);
    format!("{}{}", &value[..cut], suffix)
}

fn slack_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn smtp_plain_message_text(request: &Value) -> String {
    let summary =
        value_string(request, "summary").unwrap_or_else(|| "FDA notification".to_string());
    let resume_commands = request
        .get("resume_commands")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\r\n")
        })
        .unwrap_or_default();
    let decision_ids = request
        .get("decision_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let decision_details = smtp_decision_details(request);
    format!(
        "{}\n\nDecision IDs: {}\n\nDecisions:\n{}\n\nResume commands:\n{}",
        smtp_normalized_text(&summary),
        smtp_normalized_text(&decision_ids),
        smtp_normalized_text(&decision_details),
        smtp_normalized_text(&resume_commands)
    )
}

pub(crate) fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn smtp_base64_lines(input: &[u8]) -> String {
    let encoded = base64_encode(input);
    encoded
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\r\n")
}

fn smtp_normalized_text(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn smtp_decision_details(request: &Value) -> String {
    let Some(decisions) = request.get("decisions").and_then(Value::as_array) else {
        return "No decision details found in notification request.".to_string();
    };
    if decisions.is_empty() {
        return "No open Human Decision found.".to_string();
    }

    decisions
        .iter()
        .map(|decision| {
            let decision_id = value_string(decision, "decision_id").unwrap_or_default();
            let summary = value_string(decision, "summary").unwrap_or_default();
            let required_before = value_string(decision, "required_before").unwrap_or_default();
            let recommended =
                value_string(decision, "recommended_option_id").unwrap_or_default();
            let resume = value_string(decision, "resume_command").unwrap_or_default();
            let options = decision
                .get("options")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|option| format!("- {option}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "- <no options recorded>".to_string());
            format!(
                "Decision: {decision_id}\nSummary: {summary}\nRequired before: {required_before}\nRecommended option: {recommended}\nOptions:\n{options}\nResume command: {resume}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}
