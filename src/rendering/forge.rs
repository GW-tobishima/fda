use serde_json::{json, Value};

use crate::domain::entities::RuntimeContext;

pub(crate) fn design_forge_projection(context: &RuntimeContext) -> Value {
    json!({
        "schema_version": "forge_projection.v0",
        "program_id": context.program_id,
        "epic_id": context.epic_id,
        "source_artifacts": ["basic_design.md", "detailed_design.md", "planned_prs.json", "autonomy_contract.json"],
        "claim_contracts": [
            {
                "claim_id": "CLM-FDA-V1-DESIGN-001",
                "type": "contract",
                "statement": "Design Gate は Scope、AC、risk、QA brief、Planned PR、Forge proof を実装前に固定する。",
                "blocking": true,
                "case_ids": ["CASE-FDA-V1-DESIGN-001"],
                "planned_pr_ids": ["PPR-FDA-V1-IMPLEMENT-001"],
                "proof_obligations": ["PROOF-FDA-V1-DESIGN-001"]
            }
        ],
        "proof_obligations": [
            {
                "proof_id": "PROOF-FDA-V1-DESIGN-001",
                "claim_id": "CLM-FDA-V1-DESIGN-001",
                "type": "schema_validation",
                "required_evidence": ["validation_report.json"],
                "blocking": true,
                "owner_agent": "forge-delivery-agent",
                "validation_method": "cargo run -- validate-artifacts --artifacts <design-output>"
            }
        ],
        "promotion_readiness": {
            "verdict": "hold",
            "reason": "Design Gate artifact は生成済みだが implementation proof は後続 PR で回収する。",
            "evaluated_at": Value::Null,
            "gate_inputs_ready": true
        },
        "gate_requirements": ["Scope In / Out", "Given / When / Then AC", "OPEN_QUESTIONS", "Risk mitigation", "Functional QA brief", "Security QA brief"]
    })
}
