use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CodexLiveStatus {
    Succeeded,
    Failed,
    Blocked,
    AdapterUnavailable,
}

pub(crate) struct CodexLiveInvocationResult {
    pub(crate) status: CodexLiveStatus,
    pub(crate) thread_id: Option<String>,
    pub(crate) content: String,
    pub(crate) summary: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) tool_call_sent: bool,
}

pub(crate) struct ToolProbeResult {
    pub(crate) status: ToolProbeStatus,
    pub(crate) detected_tools: Vec<String>,
    pub(crate) summary: String,
    pub(crate) exit_code: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolProbeStatus {
    Succeeded,
    Failed,
    AdapterUnavailable,
}

#[derive(Clone)]
pub(crate) struct IntakeClassification {
    pub(crate) name: &'static str,
    pub(crate) mode: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) next_gate: &'static str,
}

pub(crate) struct IntakeInput {
    pub(crate) source: String,
    pub(crate) body: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct HumanDecisionSummary {
    pub(crate) decision_id: String,
    pub(crate) alias_ids: Vec<String>,
    pub(crate) summary: String,
    pub(crate) recommended_option_id: String,
    pub(crate) option_ids: Vec<String>,
    pub(crate) required_before: String,
}

impl HumanDecisionSummary {
    pub(crate) fn matches_id(&self, decision_id: &str) -> bool {
        self.decision_id == decision_id || self.alias_ids.iter().any(|alias| alias == decision_id)
    }
}

#[derive(Clone)]
pub(crate) struct TracePlan {
    pub(crate) status: String,
    pub(crate) cases: Vec<TraceCase>,
    pub(crate) claims: Vec<TraceClaim>,
    pub(crate) planned_prs: Vec<TracePlannedPr>,
    pub(crate) proofs: Vec<TraceProof>,
}

#[derive(Clone)]
pub(crate) struct TraceCase {
    pub(crate) case_id: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) planned_pr: Option<String>,
}

#[derive(Clone)]
pub(crate) struct TraceClaim {
    pub(crate) claim_id: String,
}

#[derive(Clone)]
pub(crate) struct TracePlannedPr {
    pub(crate) planned_pr_id: String,
    pub(crate) case_id: Option<String>,
}

#[derive(Clone)]
pub(crate) struct TraceProof {
    pub(crate) claim_id: String,
}

pub(crate) struct RuntimeContext {
    pub(crate) program_id: String,
    pub(crate) epic_id: String,
    pub(crate) case_ids: Vec<String>,
    pub(crate) task_ids: Vec<String>,
}

impl RuntimeContext {
    pub(crate) fn for_v1_intake() -> Self {
        Self {
            program_id: "FDA-V1".to_string(),
            epic_id: "EPIC-FDA-V1-INTAKE".to_string(),
            case_ids: vec!["CASE-FDA-V1-INTAKE".to_string()],
            task_ids: vec!["PR-V1-002".to_string()],
        }
    }

    pub(crate) fn for_v1_design() -> Self {
        Self {
            program_id: "FDA-V1".to_string(),
            epic_id: "EPIC-FDA-V1-INTAKE".to_string(),
            case_ids: vec!["CASE-FDA-V1-DESIGN-001".to_string()],
            task_ids: vec![
                "TASK-FDA-V1-IMPLEMENT-001".to_string(),
                "TASK-FDA-V1-FQA-001".to_string(),
                "TASK-FDA-V1-SQA-001".to_string(),
            ],
        }
    }
}
