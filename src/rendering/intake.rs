use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::{
    HumanDecisionSummary, IntakeClassification, IntakeInput, RuntimeContext,
};
use crate::single_line;
use crate::support::paths::display_path;

pub(crate) fn requirements_definition_markdown(
    input: &IntakeInput,
    classification: &IntakeClassification,
    decisions: &[HumanDecisionSummary],
) -> String {
    format!(
        "# Requirements Definition\n\n\
## 1. 入力\n\n\
- 入力元: `{}`\n\
- 入力要約: {}\n\n\
## 2. 実装可否分類\n\n\
- 分類: `{}`\n\
- mode: `{}`\n\
- 理由: {}\n\
- 次 gate: `{}`\n\n\
## 3. Scope In\n\n\
- 入力された目的を FDA V1 の Intake artifact に変換する。\n\
- 要件定義、非機能要件、リスク、Human Decision を生成する。\n\
- Human Decision を CLI stdout とこの要件定義書の両方に記録する。\n\n\
## 4. Scope Out\n\n\
- この dry-run では target repo の実装、PR 作成、merge は行わない。\n\
- Human Decision の回答適用、Design Gate、MCP agent invocation は後続 PR の責務とする。\n\n\
## 5. 受入条件\n\n\
- `requirements_definition.md` に Human Decision が記録されている。\n\
- `human_decision_packet.md` と `human_decision_packet.json` が生成されている。\n\
- `artifact_inventory.json` と `runner_explanation.json` が生成されている。\n\
- `validate-artifacts` で生成 JSON artifact を検証できる。\n\n\
## 6. Human Decision\n\n\
{}\n\n\
## 7. 次 action\n\n\
1. `fda decide HD-FDA-001 --answer <answer> --artifacts <this-output-dir>`\n\
2. `fda decide HD-FDA-002 --answer <answer> --artifacts <this-output-dir>`\n\
3. `fda decide HD-FDA-003 --answer <answer> --artifacts <this-output-dir>`\n\
4. 未解決判断がなくなったら `fda design --artifacts <this-output-dir>`\n",
        input.source,
        single_line(&input.body),
        classification.name,
        classification.mode,
        classification.summary,
        classification.next_gate,
        decisions_markdown(decisions)
    )
}

pub(crate) fn non_functional_requirements_markdown(
    input: &IntakeInput,
    classification: &IntakeClassification,
) -> String {
    format!(
        "# Non-Functional Requirements\n\n\
## 1. Traceability\n\n\
- 入力元 `{}`、実装可否分類 `{}`、Human Decision ID を artifact 間で追跡できること。\n\
- `runner_explanation.json` は stop condition と next action を持つこと。\n\n\
## 2. Safety\n\n\
- Intake dry-run は target repo を変更しないこと。\n\
- 未解決 Human Decision がある状態では実装系 command へ進ませないこと。\n\n\
## 3. Operability\n\n\
- CLI stdout は未解決判断と再開 command を表示すること。\n\
- Markdown artifact と JSON artifact の両方を生成し、人間確認と機械検証の両方に使えること。\n\n\
## 4. Input Handling\n\n\
- 入力要約: {}\n",
        input.source,
        classification.name,
        single_line(&input.body)
    )
}

pub(crate) fn risk_register_markdown(
    classification: &IntakeClassification,
    decisions: &[HumanDecisionSummary],
) -> String {
    format!(
        "# Risk Register\n\n\
| ID | Risk | Impact | Mitigation | Human Decision |\n\
|---|---|---|---|---|\n\
| R-FDA-001 | 入力解釈が人間の意図とずれる | Design / implementation の手戻り | Scope In / Out を Human Decision として固定する | {} |\n\
| R-FDA-002 | 実装可否分類が誤る | research / uiux / design / implementation の分岐を誤る | 分類 `{}` を Design Gate 前に承認対象にする | {} |\n\
| R-FDA-003 | 外部API、個人情報、法務制約が未確認 | Security / legal gate で停止する | 未記載制約を Design Gate の確認事項へ送る | {} |\n",
        decisions[0].decision_id,
        classification.name,
        decisions[1].decision_id,
        decisions[2].decision_id
    )
}

pub(crate) fn non_implementation_artifacts(
    input: &IntakeInput,
    classification: &IntakeClassification,
    decisions: &[HumanDecisionSummary],
) -> Vec<(&'static str, String)> {
    match classification.mode {
        "research" => vec![
            (
                "research_report.md",
                research_report_markdown(input, decisions),
            ),
            ("source_refs.md", source_refs_markdown(input)),
        ],
        "uiux" => vec![
            ("uiux_brief.md", uiux_brief_markdown(input, decisions)),
            ("user_flow.md", user_flow_markdown(input)),
            ("mock.html", mock_html(input)),
            ("mock.excalidraw", mock_excalidraw(input)),
        ],
        "design-only" => vec![
            (
                "basic_design.md",
                design_only_basic_design_markdown(input, decisions),
            ),
            (
                "detailed_design.md",
                design_only_detailed_design_markdown(input),
            ),
            (
                "implementation_readiness_report.md",
                implementation_readiness_report_markdown(input, decisions),
            ),
        ],
        _ => Vec::new(),
    }
}

fn research_report_markdown(input: &IntakeInput, decisions: &[HumanDecisionSummary]) -> String {
    format!(
        "# Research Report\n\n\
## 調査目的\n\n\
{}\n\n\
## 調査観点\n\n\
- 事実確認が必要な論点を分解する。\n\
- 法務、privacy、security、外部API、運用リスクを分けて扱う。\n\
- source refs の信頼度を明示し、未確認事項を Human Decision に戻す。\n\n\
## 暫定結論\n\n\
- この成果物は PR-V1-010 の offline draft であり、外部検索や法務判断を確定しない。\n\
- 実装へ進む前に source_refs.md の不足と Human Decision を確認する。\n\n\
## Human Decision\n\n\
{}\n",
        single_line(&input.body),
        decisions_markdown(decisions)
    )
}

fn source_refs_markdown(input: &IntakeInput) -> String {
    format!(
        "# Source References\n\n\
## 入力\n\n\
- `{}`\n\n\
## 参照方針\n\n\
| ID | Source | Trust | Status | Notes |\n\
|---|---|---|---|---|\n\
| SRC-FDA-001 | User input | high | captured | CLI inputを一次情報として扱う |\n\
| SRC-FDA-002 | External sources | unknown | pending | 実調査時に公式情報または一次情報で補完する |\n\n\
## 注意\n\n\
- PR-V1-010 は非実装modeのartifact生成を扱うため、外部source取得は行わない。\n\
- source refs が必要な結論は `pending` のまま残す。\n",
        single_line(&input.body)
    )
}

fn uiux_brief_markdown(input: &IntakeInput, decisions: &[HumanDecisionSummary]) -> String {
    format!(
        "# UIUX Brief\n\n\
## 目的\n\n\
{}\n\n\
## 対象ユーザー\n\n\
- 目的を短時間で確認し、未解決判断と成果物を見たい人間レビュアー。\n\n\
## 主要体験\n\n\
- 入力内容から主要workflowを抽出する。\n\
- user_flow.md で状態遷移を確認する。\n\
- mock.html / mock.excalidraw で画面案を確認する。\n\n\
## 状態\n\n\
- 通常状態: 主要情報、判断、次actionが見える。\n\
- 空状態: 入力不足をHuman Decisionとして表示する。\n\
- 失敗状態: 実装readyにせず、修正すべき論点を表示する。\n\n\
## Human Decision\n\n\
{}\n",
        single_line(&input.body),
        decisions_markdown(decisions)
    )
}

fn user_flow_markdown(input: &IntakeInput) -> String {
    format!(
        "# User Flow\n\n\
## Flow\n\n\
1. ユーザーが目的を入力する。\n\
2. FDA が mode と成果物候補を提示する。\n\
3. ユーザーが mock / brief / decision を確認する。\n\
4. 実装へ進める場合は Design Gate へ昇格する。\n\n\
## 入力要約\n\n\
- {}\n",
        single_line(&input.body)
    )
}

fn mock_html(input: &IntakeInput) -> String {
    format!(
        "<!doctype html>\n\
<html lang=\"ja\">\n\
<head>\n\
  <meta charset=\"utf-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
  <title>FDA UIUX Mock</title>\n\
  <style>\n\
    body {{ font-family: system-ui, sans-serif; margin: 0; background: #f7f7f8; color: #1f2328; }}\n\
    main {{ max-width: 920px; margin: 0 auto; padding: 32px 20px; }}\n\
    section {{ border: 1px solid #d8dee4; background: #fff; border-radius: 8px; padding: 20px; margin-bottom: 16px; }}\n\
    .label {{ color: #57606a; font-size: 13px; }}\n\
    .actions {{ display: flex; gap: 8px; flex-wrap: wrap; }}\n\
    button {{ border: 1px solid #1f6feb; background: #1f6feb; color: #fff; border-radius: 6px; padding: 8px 12px; }}\n\
  </style>\n\
</head>\n\
<body>\n\
  <main>\n\
    <section>\n\
      <div class=\"label\">Goal</div>\n\
      <h1>UIUX Mode Mock</h1>\n\
      <p>{}</p>\n\
    </section>\n\
    <section>\n\
      <div class=\"label\">Decision Inbox</div>\n\
      <p>未解決判断を確認し、実装へ進める前にscopeを固定する。</p>\n\
      <div class=\"actions\"><button>Approve</button><button>Revise</button></div>\n\
    </section>\n\
  </main>\n\
</body>\n\
</html>\n",
        html_escape(&single_line(&input.body))
    )
}

fn mock_excalidraw(input: &IntakeInput) -> String {
    json!({
        "type": "excalidraw",
        "version": 2,
        "source": "forge-delivery-agent",
        "elements": [
            {
                "id": "goal",
                "type": "text",
                "x": 80,
                "y": 80,
                "width": 520,
                "height": 80,
                "angle": 0,
                "strokeColor": "#1f2328",
                "backgroundColor": "transparent",
                "fillStyle": "solid",
                "strokeWidth": 1,
                "strokeStyle": "solid",
                "roughness": 0,
                "opacity": 100,
                "text": format!("Goal: {}", single_line(&input.body)),
                "fontSize": 20,
                "fontFamily": 1,
                "textAlign": "left",
                "verticalAlign": "top",
                "baseline": 26,
                "containerId": null,
                "originalText": format!("Goal: {}", single_line(&input.body)),
                "lineHeight": 1.25
            }
        ],
        "appState": {},
        "files": {}
    })
    .to_string()
}

fn design_only_basic_design_markdown(
    input: &IntakeInput,
    decisions: &[HumanDecisionSummary],
) -> String {
    format!(
        "# Basic Design\n\n\
## 目的\n\n\
{}\n\n\
## Scope In\n\n\
- 要件を設計artifactへ落とし込む。\n\
- 未解決判断を実装前の停止条件として残す。\n\n\
## Scope Out\n\n\
- target repo の実装、PR作成、mergeは行わない。\n\n\
## Human Decision\n\n\
{}\n",
        single_line(&input.body),
        decisions_markdown(decisions)
    )
}

fn design_only_detailed_design_markdown(input: &IntakeInput) -> String {
    format!(
        "# Detailed Design\n\n\
## Input Trace\n\n\
- {}\n\n\
## Components\n\n\
- Intake artifact reader\n\
- Decision packet projector\n\
- Implementation readiness evaluator\n\n\
## Acceptance Criteria\n\n\
- Scope In / Out が明示されている。\n\
- 実装へ進むための不足条件が分かる。\n\
- 未解決 Human Decision が残っている場合、実装readyにしない。\n",
        single_line(&input.body)
    )
}

fn implementation_readiness_report_markdown(
    input: &IntakeInput,
    decisions: &[HumanDecisionSummary],
) -> String {
    format!(
        "# Implementation Readiness Report\n\n\
## Verdict\n\n\
`not_ready_until_human_decisions_resolved`\n\n\
## Input\n\n\
- {}\n\n\
## Blocking Conditions\n\n\
{}\n\n\
## Next Actions\n\n\
- Human Decision を記録する。\n\
- 実装へ進める場合は `fda design` または後続 Design Gate へ渡す。\n",
        single_line(&input.body),
        decisions_markdown(decisions)
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn human_decision_packet_markdown(decisions: &[HumanDecisionSummary]) -> String {
    format!(
        "# Human Decision Packet\n\n\
Status: waiting_human\n\n\
## 判断が必要です\n\n\
{}\n\n\
## 続行するには\n\n\
- `fda decide HD-FDA-001 --answer yes`\n\
- `fda decide HD-FDA-002 --answer \"accept\"`\n\
- `fda decide HD-FDA-003 --answer \"confirm before design\"`\n",
        decisions_markdown(decisions)
    )
}

fn decisions_markdown(decisions: &[HumanDecisionSummary]) -> String {
    decisions
        .iter()
        .enumerate()
        .map(|(index, decision)| {
            format!(
                "{}. {}: {}  \n   - recommended: `{}`  \n   - required_before: `{}`",
                index + 1,
                decision.decision_id,
                decision.summary,
                decision.recommended_option_id,
                decision.required_before
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn human_decision_packet_json(
    input: &IntakeInput,
    classification: &IntakeClassification,
    decisions: &[HumanDecisionSummary],
    context: &RuntimeContext,
) -> Value {
    json!({
        "decision_packet_id": "HDP-FDA-V1-INTAKE-001",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "case_id": Value::Null,
        "status": "waiting_human",
        "required_before": classification.next_gate,
        "decision_needed": "Intake artifact を Design Gate または非実装 mode へ進める前に、人間が scope、分類、未記載制約の扱いを確認する必要がある。",
        "trigger": "fda start dry-run generated intake artifacts",
        "context": {
            "current_state": format!("Input source {} was classified as {}", input.source, classification.name),
            "relevant_requirement": "PR-V1-002: fda start は要件定義、Human Decision、runner explanation を生成し、判断事項を stdout と requirements_definition.md の両方へ出す。",
            "relevant_evidence": [
                "requirements_definition.md",
                "non_functional_requirements.md",
                "risk_register.md",
                "runner_explanation.json"
            ]
        },
        "options": [
            {
                "id": "approve_intake",
                "description": "生成された Intake artifact を正本として採用し、判断回答後に次 gate へ進める。",
                "pros": ["PR-V1-002 の dry-run から Design Gate へ進める", "Human Decision の stop 条件が明確になる"],
                "cons": ["入力解釈に誤りがある場合は後続 gate で手戻りする"],
                "recommended": true
            },
            {
                "id": "revise_intake",
                "description": "Scope、分類、制約を修正してから再度 fda start を実行する。",
                "pros": ["早い段階で入力解釈を修正できる"],
                "cons": ["Design Gate への着手が遅れる"],
                "recommended": false
            }
        ],
        "decisions": decisions.iter().map(|decision| json!({
            "decision_id": decision.decision_id,
            "type": "spec_decision",
            "summary": decision.summary,
            "required_before": decision.required_before,
            "options": [
                {
                    "id": decision.recommended_option_id,
                    "description": "推奨案として採用する"
                },
                {
                    "id": "revise",
                    "description": "修正して再生成する"
                }
            ],
            "recommended_option_id": decision.recommended_option_id
        })).collect::<Vec<_>>(),
        "impact": {
            "scope": "Scope In / Out が未確定のまま Design Gate へ進むことを防ぐ。",
            "security": "外部API、個人情報、法務制約の未記載項目を後続 gate の確認対象にできる。",
            "schedule": "判断待ちでは Design Gate へ進めず、手戻りを Intake で閉じる。",
            "ux": "CLI stdout と Markdown の両方で判断事項を確認できる。",
            "operations": "runner_explanation と artifact_inventory で再開位置を追跡できる。"
        },
        "default_if_no_decision": "waiting_human のまま停止し、実装系 command には進まない。",
        "forge_mapping": {
            "claim_ids": ["FDA-V1-INTAKE-HUMAN-DECISION"],
            "proof_obligations": ["CLI stdout includes Human Decision", "requirements_definition.md includes same Human Decision", "JSON artifacts validate"],
            "human_decision_points": decisions.iter().map(|decision| decision.decision_id.clone()).collect::<Vec<_>>(),
            "ato_task_graph": ["pr-v1-002-intake-command-contract"],
            "planned_prs": ["PR-V1-002"],
            "gate_requirements": ["Intake Gate", classification.next_gate]
        }
    })
}

pub(crate) fn start_runner_explanation(
    repo_root: &Path,
    input: &IntakeInput,
    out_dir: &Path,
    classification: &IntakeClassification,
    context: &RuntimeContext,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "planning",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("fda start dry-run input from {}: {}", input.source, single_line(&input.body)),
            "changed_input_summary": Value::Null,
            "stop_condition": "intake_artifacts_generated_with_waiting_human_decisions",
            "next_action": format!("Resolve Human Decision with fda decide, then continue to {}", classification.next_gate),
            "automation_boundary": "PR-V1-002 dry-run only; no target repo implementation, model provider call, MCP agent invocation, GitHub PR execution, merge, release, deploy, notification, or Output Hub generation",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("requirements_definition.md")),
                display_path(repo_root, &out_dir.join("human_decision_packet.json")),
                display_path(repo_root, &out_dir.join("artifact_inventory.json"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}
