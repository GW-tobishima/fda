use serde::Serialize;
use serde_json::{json, Value};
use std::env;
use std::path::Path;

use crate::application::decisions::value_string;
use crate::application::output_hub::{
    decision_rows_from_artifacts, execution_status_rows_from_artifacts,
};
use crate::application::ports::ArtifactStore;
use crate::application::profile::ensure_repository_profile;
use crate::cli::args::NotifyConfig;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::git::repo_project_name;
use crate::infra::json_file::write_json_file;
use crate::infra::slack::{
    send_slack_notification, slack_config_from_env, slack_webhook_url, SlackSendResponse,
};
use crate::infra::smtp::{
    send_smtp_notification, smtp_config_from_env, smtp_envelope_address, smtp_message_id,
};
use crate::rendering::notify::{
    human_turn_notice_markdown, notification_receipt,
    notification_request as render_notification_request, slack_message_payload, smtp_message_body,
    NotificationRequestInput,
};
use crate::rendering::output_hub::DecisionView;
use crate::support::paths::{display_path, resolve_path};
use crate::{ensure_artifact_dir_exists, now_unix_seconds, write_text_file};

#[derive(Debug, Serialize)]
pub(crate) struct NotifyResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: String,
    pub(crate) notification_status: String,
    pub(crate) channel: String,
    pub(crate) recipient: String,
    pub(crate) artifact_dir: String,
    pub(crate) out_dir: String,
    pub(crate) artifacts_written: Vec<String>,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) struct NotificationRecipient {
    pub(crate) recipient: String,
    pub(crate) recipient_source: String,
    pub(crate) sendable: bool,
}

pub(crate) fn notify_test(config: &NotifyConfig) -> Result<NotifyResult, String> {
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

    let channel = config.channel.clone();
    if channel != "slack" && channel != "email" && channel != "codex-app" {
        return Err(format!(
            "unsupported notification channel `{channel}`; expected slack, email, or codex-app"
        ));
    }
    let env_email = env::var("FDA_NOTIFY_EMAIL").ok();
    let env_slack_channel = env::var("FDA_SLACK_CHANNEL_LABEL").ok();
    let slack_sendable = env::var("FDA_SLACK_WEBHOOK_URL")
        .ok()
        .is_some_and(|value| slack_webhook_url(&value).is_ok());
    let recipient = resolve_notification_recipient(
        &channel,
        config.recipient.as_deref(),
        env_email.as_deref(),
        env_slack_channel.as_deref(),
        slack_sendable,
    );
    let decisions = decision_rows_from_artifacts(&artifact_dir)?;
    let status_rows = execution_status_rows_from_artifacts(&artifact_dir)?;

    let mut artifacts_written = Vec::new();
    let request = notification_request(
        &repo_root,
        &artifact_dir,
        &channel,
        &recipient,
        &decisions,
        config.live,
    );
    write_json_file(&out_dir.join("notification_request.json"), &request)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("notification_request.json"),
    ));

    let receipt = if config.live {
        live_notification_receipt(&request)
    } else {
        notification_receipt(&request)
    };
    let notification_status =
        value_string(&receipt, "status").unwrap_or_else(|| "unknown".to_string());
    write_json_file(&out_dir.join("notification_receipt.json"), &receipt)?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("notification_receipt.json"),
    ));

    write_text_file(
        &out_dir.join("human_turn_notice.md"),
        &human_turn_notice_markdown(&decisions, &status_rows),
    )?;
    artifacts_written.push(display_path(
        &repo_root,
        &out_dir.join("human_turn_notice.md"),
    ));

    Ok(NotifyResult {
        schema_version: "fda.notify_result.v0",
        verdict: "pass".to_string(),
        notification_status,
        channel,
        recipient: recipient.recipient,
        artifact_dir: display_path(&repo_root, &artifact_dir),
        out_dir: display_path(&repo_root, &out_dir),
        artifacts_written,
        next_actions: vec![
            "HUMAN_TURN時は notification_request.json を実通知adapterへ渡す".to_string(),
        ],
    })
}

pub(crate) fn resolve_notification_recipient(
    channel: &str,
    explicit_recipient: Option<&str>,
    env_email: Option<&str>,
    env_slack_channel: Option<&str>,
    slack_sendable: bool,
) -> NotificationRecipient {
    if let Some(recipient) = explicit_recipient
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if channel == "slack" {
            return NotificationRecipient {
                recipient: recipient.to_string(),
                recipient_source: "cli".to_string(),
                sendable: slack_sendable,
            };
        }
        return NotificationRecipient {
            recipient: recipient.to_string(),
            recipient_source: "cli".to_string(),
            sendable: true,
        };
    }
    if channel == "email" {
        if let Some(recipient) = env_email.map(str::trim).filter(|value| !value.is_empty()) {
            return NotificationRecipient {
                recipient: recipient.to_string(),
                recipient_source: "env:FDA_NOTIFY_EMAIL".to_string(),
                sendable: true,
            };
        }
        return NotificationRecipient {
            recipient: "kenjiii534@gmail.com".to_string(),
            recipient_source: "default:candidate".to_string(),
            sendable: false,
        };
    }
    if channel == "slack" {
        if let Some(recipient) = env_slack_channel
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return NotificationRecipient {
                recipient: recipient.to_string(),
                recipient_source: "env:FDA_SLACK_CHANNEL_LABEL".to_string(),
                sendable: slack_sendable,
            };
        }
        return NotificationRecipient {
            recipient: "slack:webhook".to_string(),
            recipient_source: "channel-default".to_string(),
            sendable: slack_sendable,
        };
    }
    NotificationRecipient {
        recipient: channel.to_string(),
        recipient_source: "channel-default".to_string(),
        sendable: false,
    }
}

pub(crate) fn notification_request(
    repo_root: &Path,
    artifact_dir: &Path,
    channel: &str,
    recipient: &NotificationRecipient,
    decisions: &[DecisionView],
    live: bool,
) -> Value {
    let project = repo_project_name(repo_root);
    render_notification_request(NotificationRequestInput {
        repo_root,
        artifact_dir,
        project: &project,
        channel,
        recipient: &recipient.recipient,
        recipient_source: &recipient.recipient_source,
        sendable: recipient.sendable,
        decisions,
        live,
    })
}

pub(crate) fn live_notification_receipt(request: &Value) -> Value {
    let sent_at = format!("unix:{}", now_unix_seconds());
    let channel = value_string(request, "channel").unwrap_or_default();
    if channel == "slack" {
        return live_slack_notification_receipt(request, &sent_at);
    }
    if channel != "email" {
        return json!({
            "schema_version": "fda.notification_receipt.v0",
            "receipt_id": "NOTIFY-RECEIPT-FDA-V1-012-001",
            "notification_id": value_string(request, "notification_id").unwrap_or_default(),
            "status": "skipped",
            "dry_run": false,
            "sent": false,
            "channel": channel,
            "recipient": value_string(request, "recipient").unwrap_or_default(),
            "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
            "sendable": request.get("sendable").and_then(Value::as_bool).unwrap_or(false),
            "adapter": "codex-app",
            "fail_closed": true,
            "failure_reason": "live_delivery_not_supported_for_codex_app",
            "sent_at": sent_at
        });
    }

    let recipient = value_string(request, "recipient").unwrap_or_default();
    let recipient_source = value_string(request, "recipient_source").unwrap_or_default();
    if recipient_source == "default:candidate" {
        return json!({
            "schema_version": "fda.notification_receipt.v0",
            "receipt_id": "NOTIFY-RECEIPT-FDA-V1-012-001",
            "notification_id": value_string(request, "notification_id").unwrap_or_default(),
            "status": "blocked",
            "dry_run": false,
            "sent": false,
            "channel": "email",
            "recipient": recipient,
            "recipient_source": recipient_source,
            "sendable": false,
            "adapter": "smtp",
            "fail_closed": true,
            "failure_reason": "live_email_requires_explicit_recipient",
            "sender_source": "env:FDA_SMTP_FROM",
            "sent_at": sent_at
        });
    }

    let sender_source = "env:FDA_SMTP_FROM";
    let smtp_config = match smtp_config_from_env() {
        Ok(config) => config,
        Err(reason) => {
            return json!({
                "schema_version": "fda.notification_receipt.v0",
                "receipt_id": "NOTIFY-RECEIPT-FDA-V1-012-001",
                "notification_id": value_string(request, "notification_id").unwrap_or_default(),
                "status": "blocked",
                "dry_run": false,
                "sent": false,
                "channel": "email",
                "recipient": recipient,
                "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
                "sendable": request.get("sendable").and_then(Value::as_bool).unwrap_or(false),
                "adapter": "smtp",
                "fail_closed": true,
                "failure_reason": reason,
                "sender_source": sender_source,
                "sent_at": sent_at
            });
        }
    };

    let response = (|| {
        let sender = smtp_envelope_address(&smtp_config.from, "FDA_SMTP_FROM")?;
        let recipient_address = smtp_envelope_address(&recipient, "recipient")?;
        let message_id = smtp_message_id();
        let body = smtp_message_body(&sender, &recipient_address, request, &message_id);
        send_smtp_notification(
            &smtp_config,
            &sender,
            &recipient_address,
            &body,
            &message_id,
        )
    })();

    match response {
        Ok(response) => json!({
            "schema_version": "fda.notification_receipt.v0",
            "receipt_id": "NOTIFY-RECEIPT-FDA-V1-012-001",
            "notification_id": value_string(request, "notification_id").unwrap_or_default(),
            "status": "sent",
            "dry_run": false,
            "sent": true,
            "channel": "email",
            "recipient": recipient,
            "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
            "sendable": request.get("sendable").and_then(Value::as_bool).unwrap_or(false),
            "adapter": "smtp",
            "message_id": response.message_id,
            "provider_response_digest": response.provider_response_digest,
            "sender_source": sender_source,
            "sent_at": sent_at
        }),
        Err(reason) => json!({
            "schema_version": "fda.notification_receipt.v0",
            "receipt_id": "NOTIFY-RECEIPT-FDA-V1-012-001",
            "notification_id": value_string(request, "notification_id").unwrap_or_default(),
            "status": "failed",
            "dry_run": false,
            "sent": false,
            "channel": "email",
            "recipient": recipient,
            "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
            "sendable": request.get("sendable").and_then(Value::as_bool).unwrap_or(false),
            "adapter": "smtp",
            "fail_closed": true,
            "failure_reason": reason,
            "sender_source": sender_source,
            "sent_at": sent_at
        }),
    }
}

fn live_slack_notification_receipt(request: &Value, sent_at: &str) -> Value {
    if request
        .get("decision_ids")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        return json!({
            "schema_version": "fda.notification_receipt.v0",
            "receipt_id": "NOTIFY-RECEIPT-FDA-V1-SLACK-001",
            "notification_id": value_string(request, "notification_id").unwrap_or_default(),
            "status": "skipped",
            "dry_run": false,
            "sent": false,
            "channel": "slack",
            "recipient": value_string(request, "recipient").unwrap_or_default(),
            "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
            "sendable": request.get("sendable").and_then(Value::as_bool).unwrap_or(false),
            "adapter": "slack_incoming_webhook",
            "skip_reason": "no_open_human_decision",
            "webhook_source": "env:FDA_SLACK_WEBHOOK_URL",
            "sent_at": sent_at
        });
    }
    let config = match slack_config_from_env() {
        Ok(config) => config,
        Err(reason) => {
            return json!({
                "schema_version": "fda.notification_receipt.v0",
                "receipt_id": "NOTIFY-RECEIPT-FDA-V1-SLACK-001",
                "notification_id": value_string(request, "notification_id").unwrap_or_default(),
                "status": "blocked",
                "dry_run": false,
                "sent": false,
                "channel": "slack",
                "recipient": value_string(request, "recipient").unwrap_or_default(),
                "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
                "sendable": false,
                "adapter": "slack_incoming_webhook",
                "fail_closed": true,
                "failure_reason": reason,
                "webhook_source": "env:FDA_SLACK_WEBHOOK_URL",
                "sent_at": sent_at
            });
        }
    };
    let payload = slack_message_payload(request);
    match send_slack_notification(&config, &payload) {
        Ok(response) => successful_slack_notification_receipt(request, response, sent_at),
        Err(error) => json!({
            "schema_version": "fda.notification_receipt.v0",
            "receipt_id": "NOTIFY-RECEIPT-FDA-V1-SLACK-001",
            "notification_id": value_string(request, "notification_id").unwrap_or_default(),
            "status": "failed",
            "dry_run": false,
            "sent": false,
            "channel": "slack",
            "recipient": value_string(request, "recipient").unwrap_or_default(),
            "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
            "sendable": true,
            "adapter": "slack_incoming_webhook",
            "fail_closed": true,
            "failure_reason": error.reason,
            "retryable": error.retryable,
            "http_status": error.http_status,
            "provider_response_digest": error.provider_response_digest,
            "webhook_source": "env:FDA_SLACK_WEBHOOK_URL",
            "sent_at": sent_at
        }),
    }
}

pub(crate) fn successful_slack_notification_receipt(
    request: &Value,
    response: SlackSendResponse,
    sent_at: &str,
) -> Value {
    json!({
        "schema_version": "fda.notification_receipt.v0",
        "receipt_id": "NOTIFY-RECEIPT-FDA-V1-SLACK-001",
        "notification_id": value_string(request, "notification_id").unwrap_or_default(),
        "status": "sent",
        "dry_run": false,
        "sent": true,
        "channel": "slack",
        "recipient": value_string(request, "recipient").unwrap_or_else(|| "slack:webhook".to_string()),
        "recipient_source": value_string(request, "recipient_source").unwrap_or_default(),
        "sendable": true,
        "adapter": "slack_incoming_webhook",
        "http_status": response.http_status,
        "provider_response_digest": response.provider_response_digest,
        "webhook_source": "env:FDA_SLACK_WEBHOOK_URL",
        "sent_at": sent_at
    })
}
