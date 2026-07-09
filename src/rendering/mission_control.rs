use serde_json::Value;

/// Mission Control ページ（read-only projection）を描画する純関数。
/// 入力は application::ui::mission_control_snapshot の JSON スナップショット。
pub(crate) fn mission_control_page(snapshot: &Value) -> String {
    let summary = &snapshot["summary"];
    let repo_root = text(&snapshot["repo_root"]);
    let runs_root = text(&snapshot["runs_root"]);
    let generated_at = snapshot["generated_at_unix"].as_u64().unwrap_or(0);

    let mut html = String::with_capacity(32 * 1024);
    html.push_str("<!DOCTYPE html>\n<html lang=\"ja\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<meta http-equiv=\"refresh\" content=\"15\">\n");
    html.push_str("<title>FDA Mission Control</title>\n");
    html.push_str(STYLE);
    html.push_str("</head>\n<body>\n");

    // ヘッダ
    html.push_str("<header>\n<div class=\"title-row\">");
    html.push_str("<h1>FDA Mission Control</h1>");
    html.push_str("<span class=\"badge badge-readonly\" title=\"正本は artifacts / ATO / GitHub。ここからは何も変更できません\">read-only projection</span>");
    html.push_str("</div>\n<p class=\"meta\">repo: <code>");
    html.push_str(&escape_html(&repo_root));
    html.push_str("</code> / runs: <code>");
    html.push_str(&escape_html(&runs_root));
    html.push_str("</code> / 更新: <span id=\"generated-at\" data-unix=\"");
    html.push_str(&generated_at.to_string());
    html.push_str("\"></span>（15秒ごとに自動更新）</p>\n</header>\n");

    // サマリ
    html.push_str("<section class=\"summary\">\n");
    summary_card(
        &mut html,
        "run",
        summary["run_count"].as_u64().unwrap_or(0),
        "count-neutral",
    );
    summary_card(
        &mut html,
        "未解決の人間判断",
        summary["open_decisions"].as_u64().unwrap_or(0),
        "count-human",
    );
    summary_card(
        &mut html,
        "AI repair 中",
        summary["repair_count"].as_u64().unwrap_or(0),
        "count-repair",
    );
    summary_card(
        &mut html,
        "merge 待ち(人間承認)",
        summary["merge_ready_count"].as_u64().unwrap_or(0),
        "count-ready",
    );
    html.push_str("</section>\n");

    // セクション順: サマリ → Decision Inbox → AI Repair Lane → Epic 進捗 → Runs → 道場 → 庭師 → フッタ。
    render_decision_inbox(&mut html, snapshot["decision_inbox"].as_array());
    render_repair_lane(&mut html, snapshot["repair_lane"].as_array());
    render_epic_progress(&mut html, &snapshot["epic_progress"]);
    render_runs(&mut html, snapshot["runs"].as_array());
    render_dojo(&mut html, snapshot["decision_journal"].as_array());
    render_gc_docket(&mut html, &snapshot["gc_docket"]);

    html.push_str("<footer><p>正本: FDA artifacts（このリポジトリ）/ ATO（状態・証跡）/ GitHub（コード・PR）。");
    html.push_str("この画面は投影であり、判断・実行は CLI（<code>fda decide</code> / <code>fda merge</code> など）で行います。</p></footer>\n");
    html.push_str(SCRIPT);
    html.push_str("</body>\n</html>\n");
    html
}

fn summary_card(html: &mut String, label: &str, count: u64, class: &str) {
    html.push_str("<div class=\"card ");
    html.push_str(class);
    html.push_str("\"><div class=\"count\">");
    html.push_str(&count.to_string());
    html.push_str("</div><div class=\"label\">");
    html.push_str(&escape_html(label));
    html.push_str("</div></div>\n");
}

fn render_decision_inbox(html: &mut String, decisions: Option<&Vec<Value>>) {
    html.push_str("<section>\n<h2><span class=\"lane-mark lane-human\"></span>Decision Inbox（人間の判断待ち）</h2>\n");
    let Some(decisions) = decisions.filter(|rows| !rows.is_empty()) else {
        html.push_str(
            "<p class=\"empty\">未解決の Human Decision はありません。</p>\n</section>\n",
        );
        return;
    };
    html.push_str("<table>\n<thead><tr><th>run</th><th>判断</th><th>必要になる前</th><th>推奨</th><th>再開コマンド</th></tr></thead>\n<tbody>\n");
    for decision in decisions {
        html.push_str("<tr><td><code>");
        html.push_str(&escape_html(&text(&decision["run"])));
        html.push_str("</code></td><td><strong>");
        html.push_str(&escape_html(&text(&decision["decision_id"])));
        html.push_str("</strong><br>");
        html.push_str(&escape_html(&text(&decision["summary"])));
        render_applicable_contract(html, &decision["applicable_contract"]);
        render_precedents(html, decision["precedents"].as_array());
        html.push_str("</td><td>");
        html.push_str(&escape_html(&text(&decision["required_before"])));
        html.push_str("</td><td>");
        html.push_str(&escape_html(&text(&decision["recommended_option_id"])));
        html.push_str("</td><td><code class=\"cmd\">");
        html.push_str(&escape_html(&text(&decision["resume_command"])));
        html.push_str("</code></td></tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</section>\n");
}

/// 適用可能な delegation contract があれば「DC-xxx 適用可」バッジ + resume command を出す。
/// 自動適用ではない（人間が `--by-contract` を明示実行するためのヒント）。
fn render_applicable_contract(html: &mut String, contract: &Value) {
    if contract.is_null() {
        return;
    }
    let rule_id = text(&contract["rule_id"]);
    if rule_id == "-" {
        return;
    }
    html.push_str("<div class=\"contract\"><span class=\"badge badge-contract\" title=\"適用可能な委任契約。自動適用はしません（人間が明示実行）\">");
    html.push_str(&escape_html(&rule_id));
    html.push_str(" 適用可</span> <code class=\"cmd\">");
    html.push_str(&escape_html(&text(&contract["resume_command"])));
    html.push_str("</code></div>");
}

/// 過去の類似判断（precedent）を小さく折りたたんで出す。答え・誰が・その後の帰結を並べる。
fn render_precedents(html: &mut String, precedents: Option<&Vec<Value>>) {
    let Some(precedents) = precedents.filter(|rows| !rows.is_empty()) else {
        return;
    };
    html.push_str("<details class=\"precedent\"><summary>過去の類似判断 (");
    html.push_str(&precedents.len().to_string());
    html.push_str(")</summary><ul class=\"precedent-list\">");
    for precedent in precedents {
        html.push_str("<li><code>");
        html.push_str(&escape_html(&text(&precedent["run"])));
        html.push_str("</code> <strong>");
        html.push_str(&escape_html(&text(&precedent["decision_id"])));
        html.push_str("</strong> → 答え: ");
        html.push_str(&escape_html(&text(&precedent["answer"])));
        html.push_str("（誰: ");
        html.push_str(&escape_html(&text(&precedent["decided_by"])));
        html.push_str("）");
        html.push_str(&status_badge(&text(&precedent["outcome"])));
        html.push_str("</li>");
    }
    html.push_str("</ul></details>");
}

fn render_repair_lane(html: &mut String, repairs: Option<&Vec<Value>>) {
    html.push_str("<section>\n<h2><span class=\"lane-mark lane-repair\"></span>AI Repair Lane（AI 側で修正中）</h2>\n");
    let Some(repairs) = repairs.filter(|rows| !rows.is_empty()) else {
        html.push_str("<p class=\"empty\">repair 待ちの run はありません。</p>\n</section>\n");
        return;
    };
    html.push_str("<table>\n<thead><tr><th>run</th><th>状態</th><th>失敗分類</th><th>retry</th><th>次アクション</th></tr></thead>\n<tbody>\n");
    for repair in repairs {
        let retry = match (
            repair["retry_attempt_count"].as_u64(),
            repair["retry_limit"].as_u64(),
        ) {
            (Some(count), Some(limit)) => format!("{count}/{limit}"),
            _ => "-".to_string(),
        };
        html.push_str("<tr><td><code>");
        html.push_str(&escape_html(&text(&repair["run"])));
        html.push_str("</code></td><td>");
        html.push_str(&status_badge(&text(&repair["repair_loop_status"])));
        html.push_str("</td><td>");
        html.push_str(&escape_html(&text(&repair["failure_classification"])));
        html.push_str("</td><td>");
        html.push_str(&escape_html(&retry));
        html.push_str("</td><td><code class=\"cmd\">");
        html.push_str(&escape_html(&text(&repair["next_action"])));
        html.push_str("</code></td></tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</section>\n");
}

fn render_runs(html: &mut String, runs: Option<&Vec<Value>>) {
    html.push_str("<section>\n<h2>Runs（判断待ち → repair → 進行中 → 完了の順）</h2>\n");
    let Some(runs) = runs.filter(|rows| !rows.is_empty()) else {
        html.push_str(
            "<p class=\"empty\">run がまだありません。<code>fda start \"やりたいこと\"</code> から始めます。</p>\n</section>\n",
        );
        return;
    };
    for run in runs {
        let name = text(&run["run"]);
        html.push_str("<article class=\"run\">\n<div class=\"run-head\"><h3>");
        html.push_str(&escape_html(&name));
        html.push_str("</h3>");
        if let Some(error) = run["error"].as_str() {
            html.push_str(&status_badge("error"));
            html.push_str("</div>\n<p class=\"reason\">");
            html.push_str(&escape_html(error));
            html.push_str("</p>\n</article>\n");
            continue;
        }
        let status = &run["status"];
        html.push_str(&status_badge(&text(&status["current_phase"])));
        html.push_str("</div>\n<p class=\"reason\">");
        html.push_str(&escape_html(&text(&status["phase_reason"])));
        html.push_str("</p>\n<dl class=\"facts\">");
        fact(html, "QA", &text(&status["qa"]["qa_status"]));
        fact(
            html,
            "repair",
            &text(&status["repair"]["repair_loop_status"]),
        );
        fact(html, "merge", &text(&status["merge"]["merge_gate_status"]));
        if let Some(pr_url) = status["merge"]["actual_pr_url"].as_str() {
            html.push_str("<div><dt>PR</dt><dd><a href=\"");
            html.push_str(&escape_html(pr_url));
            html.push_str("\" target=\"_blank\" rel=\"noopener\">");
            html.push_str(&escape_html(pr_url));
            html.push_str("</a></dd></div>");
        }
        html.push_str("</dl>\n");
        if let Some(actions) = status["next_actions"].as_array() {
            if !actions.is_empty() {
                html.push_str("<div class=\"next\"><span>次:</span>");
                for action in actions {
                    html.push_str("<code class=\"cmd\">");
                    html.push_str(&escape_html(&text(action)));
                    html.push_str("</code>");
                }
                html.push_str("</div>\n");
            }
        }
        if let Some(artifacts) = run["artifacts"].as_array() {
            if !artifacts.is_empty() {
                html.push_str("<details><summary>成果物 (");
                html.push_str(&artifacts.len().to_string());
                html.push_str(")</summary><ul class=\"artifacts\">");
                for artifact in artifacts {
                    let file = text(artifact);
                    html.push_str("<li><a href=\"/artifact/");
                    html.push_str(&escape_html(&name));
                    html.push('/');
                    html.push_str(&escape_html(&file));
                    html.push_str("\" target=\"_blank\" rel=\"noopener\">");
                    html.push_str(&escape_html(&file));
                    html.push_str("</a></li>");
                }
                html.push_str("</ul></details>\n");
            }
        }
        html.push_str("</article>\n");
    }
    html.push_str("</section>\n");
}

/// Epic 進捗（PR ごとの status バッジ + summary）。epic_progress が無ければ何も出さない。
fn render_epic_progress(html: &mut String, epic: &Value) {
    if epic.is_null() {
        return;
    }
    html.push_str("<section>\n<h2>Epic 進捗</h2>\n");
    html.push_str("<p class=\"reason\">Epic <code>");
    html.push_str(&escape_html(&text(&epic["epic_id"])));
    html.push_str("</code>");
    if epic["summary"].is_object() {
        let summary = &epic["summary"];
        html.push_str("： ");
        html.push_str(&format!(
            "merged {} / open {} / merge待ち {} / blocked {} / 未着手 {}",
            summary_count(summary, "merged"),
            summary_count(summary, "open"),
            summary_count(summary, "waiting_human"),
            summary_count(summary, "blocked"),
            summary_count(summary, "not_started"),
        ));
    }
    html.push_str("</p>\n");
    let Some(prs) = epic["prs"].as_array().filter(|rows| !rows.is_empty()) else {
        html.push_str("<p class=\"empty\">planned PR がまだありません。</p>\n</section>\n");
        return;
    };
    html.push_str("<table>\n<thead><tr><th>PR</th><th>seq</th><th>状態</th><th>タイトル</th></tr></thead>\n<tbody>\n");
    for pr in prs {
        html.push_str("<tr><td><code>");
        html.push_str(&escape_html(&text(&pr["planned_pr_id"])));
        html.push_str("</code></td><td>");
        html.push_str(&escape_html(&text(&pr["sequence"])));
        html.push_str("</td><td>");
        html.push_str(&status_badge(&text(&pr["status"])));
        html.push_str("</td><td>");
        html.push_str(&escape_html(&text(&pr["title"])));
        html.push_str("</td></tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</section>\n");
}

fn summary_count(summary: &Value, key: &str) -> u64 {
    summary[key].as_u64().unwrap_or(0)
}

/// 道場（判断の振り返り）: 判断 → 帰結の時系列テーブル。良い判断も痛い判断も同列に見せる。
fn render_dojo(html: &mut String, journal: Option<&Vec<Value>>) {
    html.push_str("<section>\n<h2>道場（判断の振り返り）</h2>\n");
    html.push_str("<p class=\"reason\">あなたの過去の判断がその後どうなったかの鏡です。</p>\n");
    let Some(journal) = journal.filter(|rows| !rows.is_empty()) else {
        html.push_str("<p class=\"empty\">まだ回答済みの判断がありません。</p>\n</section>\n");
        return;
    };
    html.push_str("<table>\n<thead><tr><th>判断</th><th>要約</th><th>答え</th><th>誰が</th><th>その後</th></tr></thead>\n<tbody>\n");
    for entry in journal {
        html.push_str("<tr><td><code>");
        html.push_str(&escape_html(&text(&entry["run"])));
        html.push_str("</code><br><strong>");
        html.push_str(&escape_html(&text(&entry["decision_id"])));
        html.push_str("</strong></td><td>");
        html.push_str(&escape_html(&text(&entry["summary"])));
        html.push_str("</td><td>");
        html.push_str(&escape_html(&text(&entry["answer"])));
        html.push_str("</td><td>");
        render_decided_by(html, entry);
        html.push_str("</td><td>");
        html.push_str(&outcome_badge(&text(&entry["outcome"]["label"])));
        html.push_str("</td></tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</section>\n");
}

/// 「誰が」判断したか（人間 or 委任契約）。契約適用時は rule_id バッジを添える。
fn render_decided_by(html: &mut String, entry: &Value) {
    html.push_str(&escape_html(&text(&entry["decided_by"])));
    if !entry["contract_rule_id"].is_null() {
        let rule = text(&entry["contract_rule_id"]);
        if rule != "-" {
            html.push_str(" <span class=\"badge badge-contract\">契約 ");
            html.push_str(&escape_html(&rule));
            html.push_str("</span>");
        }
    }
}

/// 庭師（棚卸し docket）: gc_docket があれば候補テーブル。無ければ生成コマンドを案内する。
fn render_gc_docket(html: &mut String, gc: &Value) {
    html.push_str("<section>\n<h2>庭師（棚卸し docket）</h2>\n");
    if gc.is_null() {
        html.push_str(
            "<p class=\"empty\">docket なし（<code>fda gc</code> で生成）。</p>\n</section>\n",
        );
        return;
    }
    html.push_str("<p class=\"reason\">候補 ");
    html.push_str(&summary_count(&gc["summary"], "candidate_count").to_string());
    html.push_str(" 件 / 要人間判断 ");
    html.push_str(&summary_count(&gc["summary"], "needs_human_count").to_string());
    html.push_str(" 件。<code>fda gc</code> は削除・変更を一切しません。</p>\n");
    let Some(candidates) = gc["candidates"].as_array().filter(|rows| !rows.is_empty()) else {
        html.push_str("<p class=\"empty\">棚卸し候補はありません。</p>\n</section>\n");
        return;
    };
    html.push_str("<table>\n<thead><tr><th>run</th><th>理由</th><th>推奨</th><th>needs_human</th></tr></thead>\n<tbody>\n");
    for candidate in candidates {
        html.push_str("<tr><td><code>");
        html.push_str(&escape_html(&text(&candidate["run"])));
        html.push_str("</code></td><td>");
        if let Some(reasons) = candidate["reasons"].as_array() {
            for (index, reason) in reasons.iter().enumerate() {
                if index > 0 {
                    html.push_str("<br>");
                }
                html.push_str(&escape_html(&text(reason)));
            }
        }
        html.push_str("</td><td>");
        html.push_str(&escape_html(&text(&candidate["recommendation"])));
        html.push_str("</td><td>");
        if candidate["needs_human"].as_bool() == Some(true) {
            html.push_str("<span class=\"badge badge-human\">要人間</span>");
        } else {
            html.push_str("<span class=\"badge badge-neutral\">no</span>");
        }
        html.push_str("</td></tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</section>\n");
}

/// 判断の帰結ラベル用バッジ（status_badge とは別: repair を痛い判断として可視化する）。
fn outcome_badge(label: &str) -> String {
    let class = match label {
        "merged" => "badge-done",
        "merge_ready" => "badge-ready",
        "blocked" => "badge-blocked",
        "repair" => "badge-repair",
        _ => "badge-neutral",
    };
    format!(
        "<span class=\"badge {class}\">{}</span>",
        escape_html(label)
    )
}

fn fact(html: &mut String, label: &str, value: &str) {
    html.push_str("<div><dt>");
    html.push_str(&escape_html(label));
    html.push_str("</dt><dd>");
    html.push_str(&escape_html(value));
    html.push_str("</dd></div>");
}

fn status_badge(phase: &str) -> String {
    let class = match phase {
        "human_turn" | "waiting_for_decision" => "badge-human",
        "repair_planned" | "qa_failed" | "error" | "failed" | "blocked" => "badge-blocked",
        phase if phase.starts_with("ready_for_") || phase == "merge_ready" => "badge-ready",
        "merged" | "operational_v1_complete" | "no_repair_needed" | "passed" | "pass" => {
            "badge-done"
        }
        _ => "badge-neutral",
    };
    format!(
        "<span class=\"badge {class}\">{}</span>",
        escape_html(phase)
    )
}

fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "-".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn escape_html(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for character in raw.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

const STYLE: &str = r#"<style>
:root { color-scheme: light dark;
  --bg: #f6f7f9; --panel: #ffffff; --ink: #1c2330; --sub: #5b6472; --line: #e3e6ea;
  --human: #b45309; --human-bg: #fef3c7; --blocked: #b91c1c; --blocked-bg: #fee2e2;
  --ready: #1d4ed8; --ready-bg: #dbeafe; --done: #15803d; --done-bg: #dcfce7;
  --neutral: #475569; --neutral-bg: #e2e8f0; }
@media (prefers-color-scheme: dark) { :root {
  --bg: #11151c; --panel: #1a212b; --ink: #e8ecf1; --sub: #9aa4b2; --line: #2a3340;
  --human-bg: #453206; --blocked-bg: #4c1414; --ready-bg: #172a54; --done-bg: #123a20; --neutral-bg: #2b3646; } }
* { box-sizing: border-box; }
body { margin: 0 auto; padding: 24px; max-width: 1080px; background: var(--bg); color: var(--ink);
  font-family: "Hiragino Sans", "Yu Gothic UI", "Meiryo", system-ui, sans-serif; line-height: 1.6; }
h1 { font-size: 1.4rem; margin: 0; } h2 { font-size: 1.05rem; margin: 28px 0 10px; }
h3 { font-size: .95rem; margin: 0; }
.title-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.meta { color: var(--sub); font-size: .8rem; margin: 6px 0 0; }
code { background: var(--neutral-bg); padding: 1px 6px; border-radius: 4px; font-size: .82em; word-break: break-all; }
.cmd { display: inline-block; margin: 2px 4px 2px 0; user-select: all; }
.summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; margin-top: 18px; }
.card { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; padding: 12px 16px; }
.card .count { font-size: 1.7rem; font-weight: 700; }
.card .label { color: var(--sub); font-size: .78rem; }
.count-human .count { color: var(--human); } .count-repair .count { color: var(--blocked); }
.count-ready .count { color: var(--ready); }
table { width: 100%; border-collapse: collapse; background: var(--panel); border: 1px solid var(--line); border-radius: 10px; overflow: hidden; }
th, td { text-align: left; padding: 8px 12px; border-top: 1px solid var(--line); font-size: .85rem; vertical-align: top; }
thead th { border-top: none; background: var(--neutral-bg); color: var(--sub); font-size: .75rem; }
.badge { display: inline-block; padding: 1px 10px; border-radius: 999px; font-size: .72rem; font-weight: 600; white-space: nowrap; }
.badge-human { color: var(--human); background: var(--human-bg); }
.badge-blocked { color: var(--blocked); background: var(--blocked-bg); }
.badge-ready { color: var(--ready); background: var(--ready-bg); }
.badge-done { color: var(--done); background: var(--done-bg); }
.badge-neutral, .badge-readonly { color: var(--neutral); background: var(--neutral-bg); }
.badge-contract { color: var(--ready); background: var(--ready-bg); }
.badge-repair { color: var(--human); background: var(--human-bg); }
.contract { margin: 6px 0 2px; font-size: .8rem; }
.precedent { margin-top: 4px; }
.precedent-list { margin: 4px 0 0; padding-left: 16px; font-size: .8rem; }
.precedent-list li { margin: 2px 0; }
.lane-mark { display: inline-block; width: 10px; height: 10px; border-radius: 3px; margin-right: 8px; }
.lane-human { background: var(--human); } .lane-repair { background: var(--blocked); }
.run { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; padding: 14px 16px; margin-bottom: 12px; }
.run-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; }
.reason { color: var(--sub); font-size: .82rem; margin: 6px 0; }
.facts { display: flex; gap: 18px; flex-wrap: wrap; margin: 6px 0; }
.facts div { display: flex; gap: 6px; align-items: baseline; }
.facts dt { color: var(--sub); font-size: .72rem; } .facts dd { margin: 0; font-size: .82rem; }
.next { font-size: .82rem; } .next span { color: var(--sub); margin-right: 6px; }
.artifacts { columns: 2; margin: 8px 0 0; padding-left: 18px; font-size: .82rem; }
.empty { color: var(--sub); font-size: .85rem; background: var(--panel); border: 1px dashed var(--line); border-radius: 10px; padding: 10px 14px; }
footer { margin-top: 32px; color: var(--sub); font-size: .75rem; border-top: 1px solid var(--line); padding-top: 12px; }
a { color: var(--ready); }
details summary { cursor: pointer; font-size: .8rem; color: var(--sub); }
</style>
"#;

const SCRIPT: &str = r#"<script>
(function () {
  var node = document.getElementById("generated-at");
  if (!node) { return; }
  var unix = Number(node.getAttribute("data-unix"));
  if (unix > 0) { node.textContent = new Date(unix * 1000).toLocaleString("ja-JP"); }
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_snapshot() -> Value {
        json!({
            "schema_version": "fda.mission_control_snapshot.v0",
            "generated_at_unix": 1783546909,
            "repo_root": "C:/repo",
            "runs_root": "artifacts/runs",
            "summary": {"run_count": 1, "open_decisions": 1, "repair_count": 0, "merge_ready_count": 0},
            "decision_inbox": [{
                "run": "fda-start-100",
                "decision_id": "HD-FDA-001",
                "summary": "<scope> を固定してよいか",
                "required_before": "Design Gate",
                "recommended_option_id": "yes",
                "resume_command": "fda decide HD-FDA-001 --answer <answer> --artifacts artifacts/runs/fda-start-100"
            }],
            "repair_lane": [],
            "runs": [{
                "run": "fda-start-100",
                "run_dir": "artifacts/runs/fda-start-100",
                "status": {
                    "current_phase": "human_turn",
                    "phase_reason": "未解決 Human Decision があります。",
                    "qa": {"qa_status": "missing"},
                    "repair": {"repair_loop_status": "missing"},
                    "merge": {"merge_gate_status": "missing", "actual_pr_url": null},
                    "next_actions": ["fda decide HD-FDA-001 --answer <answer>"]
                },
                "artifacts": ["requirements_definition.md"]
            }]
        })
    }

    #[test]
    fn page_renders_decision_inbox_and_runs() {
        let html = mission_control_page(&sample_snapshot());
        assert!(html.contains("FDA Mission Control"));
        assert!(html.contains("HD-FDA-001"));
        assert!(html.contains("badge-human"));
        assert!(html.contains("/artifact/fda-start-100/requirements_definition.md"));
        assert!(html.contains("read-only projection"));
    }

    #[test]
    fn page_escapes_html_in_snapshot_values() {
        let html = mission_control_page(&sample_snapshot());
        assert!(html.contains("&lt;scope&gt; を固定してよいか"));
        assert!(!html.contains("<scope>"));
    }

    #[test]
    fn escape_html_covers_special_characters() {
        assert_eq!(
            escape_html("<a href=\"x\">&'</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;"
        );
    }

    /// 道場 / 庭師 / Epic / precedent / 契約バッジを含む豊富なスナップショット。
    fn rich_snapshot() -> Value {
        json!({
            "schema_version": "fda.mission_control_snapshot.v0",
            "generated_at_unix": 1783546909,
            "repo_root": "C:/repo",
            "runs_root": "artifacts/runs",
            "summary": {"run_count": 2, "open_decisions": 1, "repair_count": 0, "merge_ready_count": 0},
            "decision_inbox": [{
                "run": "fda-start-999",
                "decision_id": "HD-NOW-001",
                "type": "spec_decision",
                "summary": "scope を固定してよいか",
                "required_before": "Design Gate",
                "recommended_option_id": "yes",
                "resume_command": "fda decide HD-NOW-001 --answer <answer> --artifacts artifacts/runs/fda-start-999",
                "precedents": [{
                    "run": "run-p1",
                    "decision_id": "HD-PAST-001",
                    "answer": "approve_scope",
                    "decided_by": "human",
                    "outcome": "merged"
                }],
                "applicable_contract": {
                    "rule_id": "DC-001",
                    "resume_command": "fda decide HD-NOW-001 --by-contract DC-001 --artifacts artifacts/runs/fda-start-999"
                }
            }],
            "repair_lane": [],
            "runs": [],
            "decision_journal": [{
                "run": "fda-start-200",
                "run_dir": "artifacts/runs/fda-start-200",
                "decision_id": "HD-A-001",
                "type": "spec_decision",
                "summary": "Scope を固定してよいか",
                "answer": "approve_scope",
                "decided_by": "delegation_contract:DC-001:k_tobishima",
                "contract_rule_id": "DC-001",
                "decided_at_unix": 1000,
                "outcome": {"label": "repair", "merge_gate_status": "missing", "merge_verdict": "-", "repair_occurred": true, "qa_status": "failed"}
            }],
            "gc_docket": {
                "generated_at_unix": 1783555200u64,
                "scanned_runs": 3,
                "summary": {"candidate_count": 1, "needs_human_count": 1},
                "candidates": [{
                    "run": "run-stale",
                    "reasons": ["stale 未完了", "validation_report.json 欠落"],
                    "recommendation": "archive",
                    "needs_human": true
                }]
            },
            "epic_progress": {
                "epic_id": "EPIC-FDA-V1-5",
                "generated_at_unix": 1783555200u64,
                "prs": [
                    {"planned_pr_id": "PR-V15-001", "sequence": 1, "title": "F6 表層分け", "status": "merged"},
                    {"planned_pr_id": "PR-V15-005", "sequence": 5, "title": "F3 道場 UI", "status": "pr_open"}
                ],
                "summary": {"merged": 4, "open": 1, "blocked": 0, "waiting_human": 0, "not_started": 0}
            }
        })
    }

    #[test]
    fn page_renders_dojo_gc_epic_sections() {
        let html = mission_control_page(&rich_snapshot());
        // 道場。
        assert!(html.contains("道場（判断の振り返り）"));
        assert!(html.contains("あなたの過去の判断がその後どうなったかの鏡です"));
        assert!(html.contains("HD-A-001"));
        assert!(html.contains("approve_scope"));
        assert!(html.contains("badge-repair")); // 痛い判断も同列に可視化。
        assert!(html.contains("契約 DC-001")); // 誰が = 契約。
                                               // 庭師。
        assert!(html.contains("庭師（棚卸し docket）"));
        assert!(html.contains("run-stale"));
        assert!(html.contains("archive"));
        assert!(html.contains("要人間"));
        // Epic 進捗。
        assert!(html.contains("Epic 進捗"));
        assert!(html.contains("PR-V15-005"));
        assert!(html.contains("badge-done")); // merged PR。
        assert!(html.contains("merged 4 / open 1"));
    }

    #[test]
    fn page_renders_precedent_and_contract_in_inbox() {
        let html = mission_control_page(&rich_snapshot());
        assert!(html.contains("過去の類似判断 (1)"));
        assert!(html.contains("HD-PAST-001"));
        assert!(html.contains("DC-001 適用可"));
        assert!(html.contains("--by-contract DC-001"));
    }

    #[test]
    fn page_escapes_html_in_new_sections() {
        let mut snapshot = rich_snapshot();
        snapshot["decision_journal"][0]["summary"] =
            json!("<script>alert('x')</script> を固定してよいか");
        snapshot["gc_docket"]["candidates"][0]["reasons"][0] = json!("<img src=x>");
        snapshot["decision_inbox"][0]["precedents"][0]["answer"] = json!("<b>yes</b>");
        let html = mission_control_page(&snapshot);
        assert!(html.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;img src=x&gt;"));
        assert!(html.contains("&lt;b&gt;yes&lt;/b&gt;"));
    }

    #[test]
    fn page_shows_gc_empty_note_and_skips_absent_epic() {
        // decision_journal / gc_docket / epic_progress を欠いた最小スナップショット。
        let html = mission_control_page(&sample_snapshot());
        assert!(html.contains("docket なし"));
        assert!(html.contains("まだ回答済みの判断がありません"));
        // epic_progress が無いときは Epic セクション自体を出さない。
        assert!(!html.contains("Epic 進捗"));
    }
}
