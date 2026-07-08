use serde_json::{json, Value};

#[derive(Clone)]
pub(crate) struct HubArtifactView {
    pub(crate) title: String,
    pub(crate) artifact_type: String,
    pub(crate) path_or_url: String,
    pub(crate) preview_summary: String,
}

#[derive(Clone)]
pub(crate) struct DecisionView {
    pub(crate) decision_id: String,
    pub(crate) summary: String,
    pub(crate) required_before: String,
    pub(crate) status: String,
    pub(crate) options: Vec<String>,
    pub(crate) recommended_option_id: String,
    pub(crate) resume_command: String,
}

#[derive(Clone)]
pub(crate) struct StatusView {
    pub(crate) label: String,
    pub(crate) value: String,
}

pub(crate) fn output_hub_html(
    artifact_dir: &str,
    artifacts: &[HubArtifactView],
    decisions: &[DecisionView],
    statuses: &[StatusView],
) -> String {
    let artifact_rows = artifacts
        .iter()
        .map(|artifact| {
            format!(
                "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                escape_html(&artifact.title),
                escape_html(&artifact.artifact_type),
                escape_html(&artifact.path_or_url),
                escape_html(&artifact.preview_summary)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let decision_rows = decisions
        .iter()
        .map(|decision| {
            let options = decision.options.join(", ");
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td></tr>",
                escape_html(&decision.decision_id),
                escape_html(&decision.summary),
                escape_html(&decision.status),
                escape_html(&options),
                escape_html(&decision.resume_command)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let status_rows = statuses
        .iter()
        .map(|status| {
            format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                escape_html(&status.label),
                escape_html(&status.value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    html_shell(
        "FDA Output Hub",
        &format!(
            "<section><h1>FDA Output Hub</h1><p><code>{}</code></p></section>\
<section><h2>Artifacts</h2><table><thead><tr><th>Title</th><th>Type</th><th>Path</th><th>Summary</th></tr></thead><tbody>{}</tbody></table></section>\
<section><h2>Decisions</h2><table><thead><tr><th>ID</th><th>Summary</th><th>Status</th><th>Options</th><th>Resume</th></tr></thead><tbody>{}</tbody></table></section>\
<section><h2>Status</h2><table><thead><tr><th>Field</th><th>Value</th></tr></thead><tbody>{}</tbody></table></section>",
            escape_html(artifact_dir),
            artifact_rows,
            decision_rows,
            status_rows
        ),
    )
}

pub(crate) fn decision_inbox_html(decisions: &[DecisionView]) -> String {
    let rows = if decisions.is_empty() {
        "<tr><td colspan=\"6\">未解決Decisionはありません</td></tr>".to_string()
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
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td></tr>",
                    escape_html(&decision.decision_id),
                    escape_html(&decision.summary),
                    escape_html(&decision.required_before),
                    escape_html(&decision.status),
                    escape_html(&options),
                    escape_html(&decision.resume_command)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    html_shell(
        "FDA Decision Inbox",
        &format!(
            "<section><h1>Decision Inbox</h1><table><thead><tr><th>ID</th><th>Summary</th><th>Required Before</th><th>Status</th><th>Options</th><th>Resume</th></tr></thead><tbody>{}</tbody></table></section>",
            rows
        ),
    )
}

pub(crate) fn execution_status_html(statuses: &[StatusView]) -> String {
    let rows = statuses
        .iter()
        .map(|status| {
            format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                escape_html(&status.label),
                escape_html(&status.value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    html_shell(
        "FDA Execution Status",
        &format!(
            "<section><h1>Execution Status</h1><table><thead><tr><th>Field</th><th>Value</th></tr></thead><tbody>{}</tbody></table></section>",
            rows
        ),
    )
}

pub(crate) fn output_hub_receipt(
    artifact_dir: &str,
    artifacts: &[HubArtifactView],
    decisions: &[DecisionView],
    statuses: &[StatusView],
) -> Value {
    json!({
        "schema_version": "fda.output_hub_receipt.v0",
        "receipt_id": "OUTPUT-HUB-FDA-V1-011-001",
        "artifact_dir": artifact_dir,
        "artifact_count": artifacts.len(),
        "decision_count": decisions.len(),
        "status_count": statuses.len(),
        "outputs": ["output_hub.html", "decision_inbox.html", "execution_status.html"]
    })
}

fn html_shell(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"ja\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>body{{font-family:system-ui,sans-serif;margin:0;background:#f6f8fa;color:#1f2328}}main{{max-width:1120px;margin:0 auto;padding:28px 20px}}section{{background:#fff;border:1px solid #d8dee4;border-radius:8px;padding:18px;margin-bottom:16px}}table{{width:100%;border-collapse:collapse}}th,td{{border-bottom:1px solid #d8dee4;text-align:left;padding:8px;vertical-align:top}}code{{font-family:ui-monospace,monospace}}</style>\n</head>\n<body><main>{}</main></body>\n</html>\n",
        escape_html(title),
        body
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
