use serde_json::{json, Value};
use std::path::Path;

use crate::domain::entities::RuntimeContext;
use crate::now_unix_seconds;
use crate::support::paths::display_path;

pub(crate) struct ArtifactInventorySpec<'a> {
    pub(crate) artifact_id: &'a str,
    pub(crate) artifact_type: &'a str,
    pub(crate) title: &'a str,
    pub(crate) preview_summary: &'a str,
    pub(crate) file_name: &'a str,
    pub(crate) timestamp: u64,
}

pub(crate) fn artifact_inventory_entry(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    spec: ArtifactInventorySpec<'_>,
) -> Value {
    let path = display_path(repo_root, &out_dir.join(spec.file_name));
    json!({
        "artifact_id": spec.artifact_id,
        "artifact_type": spec.artifact_type,
        "title": spec.title,
        "producer_agent": "forge-delivery-agent",
        "related_program_id": context.program_id,
        "related_epic_id": context.epic_id,
        "related_case_ids": context.case_ids,
        "related_task_ids": context.task_ids,
        "latest_version": "v0",
        "preview_summary": spec.preview_summary,
        "path_or_url": path,
        "open_in_editor_link": Value::Null,
        "open_in_browser_link": Value::Null,
        "diff_link": Value::Null,
        "evidence_links": [],
        "created_at_unix_seconds": spec.timestamp,
        "updated_at_unix_seconds": spec.timestamp
    })
}

pub(crate) fn start_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
    mode: &str,
) -> Value {
    let now = now_unix_seconds();
    let mut entries = vec![
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-001",
                artifact_type: "generic_receipt",
                title: "Requirements Definition",
                preview_summary: "Intake 要件定義と Human Decision の正本 Markdown",
                file_name: "requirements_definition.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-002",
                artifact_type: "generic_receipt",
                title: "Non-Functional Requirements",
                preview_summary: "Intake dry-run の非機能要件",
                file_name: "non_functional_requirements.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-003",
                artifact_type: "generic_receipt",
                title: "Risk Register",
                preview_summary: "Intake Gate の初期リスクと Human Decision 対応",
                file_name: "risk_register.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-004",
                artifact_type: "human_decision_packet",
                title: "Human Decision Packet Markdown",
                preview_summary: "人間向けの判断事項一覧",
                file_name: "human_decision_packet.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-005",
                artifact_type: "human_decision_packet",
                title: "Human Decision Packet JSON",
                preview_summary: "Schema 検証可能な Human Decision Packet",
                file_name: "human_decision_packet.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-006",
                artifact_type: "runner_explanation",
                title: "Runner Explanation",
                preview_summary: "Intake dry-run の停止条件、境界、次 action",
                file_name: "runner_explanation.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-INTAKE-007",
                artifact_type: "validation_report",
                title: "Validation Report",
                preview_summary: "生成 JSON artifact の schema validation 結果",
                file_name: "validation_report.json",
                timestamp: now,
            },
        ),
    ];
    let extra_specs = start_mode_artifact_specs(mode);
    for (artifact_id, file_name, title, preview_summary) in &extra_specs {
        entries.push(artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id,
                artifact_type: "generic_receipt",
                title,
                preview_summary,
                file_name,
                timestamp: now,
            },
        ));
    }

    let extra_artifact_ids = extra_specs
        .iter()
        .map(|(artifact_id, _, _, _)| artifact_id.to_string())
        .collect::<Vec<_>>();
    let mut groups = vec![
        json!({
            "group_id": "intake_artifacts",
            "title": "Intake Artifacts",
            "artifact_ids": ["ART-INTAKE-001", "ART-INTAKE-002", "ART-INTAKE-003", "ART-INTAKE-004", "ART-INTAKE-005"]
        }),
        json!({
            "group_id": "run_evidence",
            "title": "Run Evidence",
            "artifact_ids": ["ART-INTAKE-006", "ART-INTAKE-007"]
        }),
    ];
    if !extra_artifact_ids.is_empty() {
        groups.push(json!({
            "group_id": "non_implementation_outputs",
            "title": "Non-implementation Outputs",
            "artifact_ids": extra_artifact_ids
        }));
    }

    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": entries,
        "groups": groups
    })
}

fn start_mode_artifact_specs(
    mode: &str,
) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    match mode {
        "research" => vec![
            (
                "ART-NONIMPL-RESEARCH-001",
                "research_report.md",
                "Research Report",
                "Research mode report and unresolved decision summary",
            ),
            (
                "ART-NONIMPL-RESEARCH-002",
                "source_refs.md",
                "Source References",
                "Source reference policy and pending external sources",
            ),
        ],
        "uiux" => vec![
            (
                "ART-NONIMPL-UIUX-001",
                "uiux_brief.md",
                "UIUX Brief",
                "UIUX mode brief with state and decision coverage",
            ),
            (
                "ART-NONIMPL-UIUX-002",
                "user_flow.md",
                "User Flow",
                "User journey and workflow outline",
            ),
            (
                "ART-NONIMPL-UIUX-003",
                "mock.html",
                "HTML Mock",
                "Reviewable static HTML mock",
            ),
            (
                "ART-NONIMPL-UIUX-004",
                "mock.excalidraw",
                "Excalidraw Mock",
                "Excalidraw-compatible mock source",
            ),
        ],
        "design-only" => vec![
            (
                "ART-NONIMPL-DESIGN-001",
                "basic_design.md",
                "Basic Design",
                "Design-only basic design artifact",
            ),
            (
                "ART-NONIMPL-DESIGN-002",
                "detailed_design.md",
                "Detailed Design",
                "Design-only detailed design artifact",
            ),
            (
                "ART-NONIMPL-DESIGN-003",
                "implementation_readiness_report.md",
                "Implementation Readiness Report",
                "Readiness verdict and blockers before implementation",
            ),
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn design_artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
) -> Value {
    let now = now_unix_seconds();
    let entries = vec![
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-001",
                artifact_type: "generic_receipt",
                title: "Basic Design",
                preview_summary: "Design Gate の基本設計",
                file_name: "basic_design.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-002",
                artifact_type: "generic_receipt",
                title: "Detailed Design",
                preview_summary: "Design Gate の詳細設計",
                file_name: "detailed_design.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-003",
                artifact_type: "generic_receipt",
                title: "Functional QA Brief",
                preview_summary: "Functional QA の確認契約",
                file_name: "functional_qa_brief.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-004",
                artifact_type: "generic_receipt",
                title: "Security QA Brief",
                preview_summary: "Security QA の確認契約",
                file_name: "security_qa_brief.md",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-005",
                artifact_type: "case_graph",
                title: "Case Graph",
                preview_summary: "Design Gate の Case Graph",
                file_name: "case_graph.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-006",
                artifact_type: "task_graph",
                title: "Task Graph",
                preview_summary: "Implementer / QA role を分離した Task Graph",
                file_name: "task_graph.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-007",
                artifact_type: "planned_prs",
                title: "Planned PRs",
                preview_summary: "Design Gate 後の Planned PR 契約",
                file_name: "planned_prs.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-008",
                artifact_type: "autonomy_contract",
                title: "Autonomy Contract",
                preview_summary: "Design Gate の自律境界",
                file_name: "autonomy_contract.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-009",
                artifact_type: "forge_projection",
                title: "Forge Projection",
                preview_summary: "ClaimContract と Proof Obligation",
                file_name: "forge_projection.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-010",
                artifact_type: "runner_explanation",
                title: "Runner Explanation",
                preview_summary: "Design Gate の停止条件と次 action",
                file_name: "runner_explanation.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-DESIGN-011",
                artifact_type: "validation_report",
                title: "Validation Report",
                preview_summary: "Design Gate JSON artifact の schema validation",
                file_name: "validation_report.json",
                timestamp: now,
            },
        ),
    ];

    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": entries,
        "groups": [
            {
                "group_id": "design_artifacts",
                "title": "Design Gate Artifacts",
                "artifact_ids": ["ART-DESIGN-001", "ART-DESIGN-002", "ART-DESIGN-003", "ART-DESIGN-004", "ART-DESIGN-005", "ART-DESIGN-006", "ART-DESIGN-007", "ART-DESIGN-008", "ART-DESIGN-009"]
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-DESIGN-010", "ART-DESIGN-011"]
            }
        ]
    })
}

pub(crate) fn runner_explanation(
    repo_root: &Path,
    requirements_path: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
) -> Value {
    json!({
        "runner_explanation": {
            "current_phase": "planning",
            "previous_run_id": Value::Null,
            "diff_from_previous_run": Value::Null,
            "execution_actor": "forge-delivery-agent",
            "input_summary": format!("Fixture planning input from {}", display_path(repo_root, requirements_path)),
            "changed_input_summary": Value::Null,
            "stop_condition": "planning_artifacts_generated_and_validated",
            "next_action": "review generated artifacts before running the Planning-only PoC",
            "automation_boundary": "fixture mode only; no model provider, ATO write, Forge write, GitHub PR execution, merge, release, deploy, or UI execution",
            "completion_evidence": [
                display_path(repo_root, &out_dir.join("validation_report.json")),
                display_path(repo_root, &out_dir.join("artifact_inventory.json"))
            ],
            "failure_evidence": [],
            "related_program_id": context.program_id,
            "related_epic_id": context.epic_id
        }
    })
}

pub(crate) fn artifact_inventory(
    repo_root: &Path,
    out_dir: &Path,
    context: &RuntimeContext,
) -> Value {
    let now = now_unix_seconds();
    let entries = vec![
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-001",
                artifact_type: "epic_delivery_plan",
                title: "Epic Delivery Plan",
                preview_summary: "Program / Epic / Case / PR / Proof planning contract",
                file_name: "epic_delivery_plan.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-002",
                artifact_type: "case_graph",
                title: "Case Graph",
                preview_summary: "Forge Case graph derived by fixture mode",
                file_name: "case_graph.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-003",
                artifact_type: "task_graph",
                title: "ATO Task Graph",
                preview_summary: "ATO-compatible task graph draft produced by fixture mode",
                file_name: "task_graph.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-004",
                artifact_type: "autonomy_contract",
                title: "Autonomy Contract",
                preview_summary:
                    "Allowed actions, forbidden actions, escalation rules, and evidence policy",
                file_name: "autonomy_contract.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-005",
                artifact_type: "human_decision_packet",
                title: "Human Decision Packet",
                preview_summary: "Human-only decision point separated from AI repair work",
                file_name: "human_decision_packet.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-006",
                artifact_type: "runner_explanation",
                title: "Runner Explanation",
                preview_summary: "Planning phase, stop condition, boundary, and next action",
                file_name: "runner_explanation.json",
                timestamp: now,
            },
        ),
        artifact_inventory_entry(
            repo_root,
            out_dir,
            context,
            ArtifactInventorySpec {
                artifact_id: "ART-007",
                artifact_type: "validation_report",
                title: "Validation Report",
                preview_summary:
                    "Schema validation and trace-link validation result for generated artifacts",
                file_name: "validation_report.json",
                timestamp: now,
            },
        ),
    ];

    json!({
        "schema_version": "fda.artifact_inventory.v0",
        "generated_at_unix_seconds": now,
        "artifacts": entries,
        "groups": [
            {
                "group_id": "planning",
                "title": "Planning Artifacts",
                "artifact_ids": ["ART-001", "ART-002", "ART-003", "ART-004", "ART-005"]
            },
            {
                "group_id": "run_evidence",
                "title": "Run Evidence",
                "artifact_ids": ["ART-006", "ART-007"]
            }
        ]
    })
}
