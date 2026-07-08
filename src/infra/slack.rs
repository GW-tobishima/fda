use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;
use std::env;
use std::time::Duration;

const SLACK_WEBHOOK_PREFIX: &str = "https://hooks.slack.com/services/";

#[derive(Clone)]
pub(crate) struct SlackConfig {
    pub(crate) webhook_url: String,
}

pub(crate) struct SlackSendResponse {
    pub(crate) http_status: u16,
    pub(crate) provider_response_digest: String,
}

pub(crate) struct SlackSendError {
    pub(crate) reason: String,
    pub(crate) retryable: bool,
    pub(crate) http_status: Option<u16>,
    pub(crate) provider_response_digest: Option<String>,
}

pub(crate) fn slack_config_from_env() -> Result<SlackConfig, String> {
    let webhook_url = env::var("FDA_SLACK_WEBHOOK_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "missing required Slack env: FDA_SLACK_WEBHOOK_URL".to_string())?;
    let webhook_url = slack_webhook_url(&webhook_url)?;
    Ok(SlackConfig { webhook_url })
}

pub(crate) fn slack_webhook_url(value: &str) -> Result<String, String> {
    let url = value.trim();
    if url.is_empty() {
        return Err("invalid FDA_SLACK_WEBHOOK_URL: empty".to_string());
    }
    if url
        .chars()
        .any(|character| character == '\r' || character == '\n')
    {
        return Err("invalid FDA_SLACK_WEBHOOK_URL: contains CR/LF".to_string());
    }
    if url.chars().any(|character| character.is_control()) {
        return Err("invalid FDA_SLACK_WEBHOOK_URL: contains control character".to_string());
    }
    if url.chars().any(char::is_whitespace) {
        return Err("invalid FDA_SLACK_WEBHOOK_URL: contains whitespace".to_string());
    }
    if !url.starts_with(SLACK_WEBHOOK_PREFIX) {
        return Err(
            "invalid FDA_SLACK_WEBHOOK_URL: expected hooks.slack.com incoming webhook URL"
                .to_string(),
        );
    }
    Ok(url.to_string())
}

pub(crate) fn send_slack_notification(
    config: &SlackConfig,
    payload: &Value,
) -> Result<SlackSendResponse, SlackSendError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("forge-delivery-agent/0.1")
        .build()
        .map_err(|_| SlackSendError {
            reason: "failed_to_build_slack_http_client".to_string(),
            retryable: false,
            http_status: None,
            provider_response_digest: None,
        })?;
    let response = client
        .post(&config.webhook_url)
        .json(payload)
        .send()
        .map_err(|_| SlackSendError {
            reason: "slack_webhook_request_failed_or_timed_out".to_string(),
            retryable: true,
            http_status: None,
            provider_response_digest: None,
        })?;
    let status = response.status();
    let http_status = status.as_u16();
    let body = response.text().unwrap_or_default();
    let response_digest = slack_response_digest(&body);
    if status == StatusCode::OK && body.trim() == "ok" {
        return Ok(SlackSendResponse {
            http_status,
            provider_response_digest: response_digest,
        });
    }

    Err(SlackSendError {
        reason: slack_failure_reason(status, &body),
        retryable: slack_failure_retryable(status, &body),
        http_status: Some(http_status),
        provider_response_digest: Some(response_digest),
    })
}

fn slack_failure_reason(status: StatusCode, body: &str) -> String {
    let error = body.trim();
    if !error.is_empty()
        && error
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return format!("slack_webhook_error:{error}");
    }
    format!("slack_webhook_http_status:{}", status.as_u16())
}

fn slack_failure_retryable(status: StatusCode, body: &str) -> bool {
    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
        return true;
    }
    matches!(body.trim(), "rollup_error")
}

pub(crate) fn slack_response_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}
