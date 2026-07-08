use std::collections::{HashMap, HashSet};

use crate::domain::entities::TracePlan;

pub(crate) fn validate_case_pr_links(plan: &TracePlan) -> Vec<String> {
    if !matches!(plan.status.as_str(), "ready" | "running" | "done") {
        return Vec::new();
    }

    let cases_by_id: HashMap<String, _> = plan
        .cases
        .iter()
        .map(|case| (case.case_id.clone(), case))
        .collect();
    let claim_ids: HashSet<String> = plan
        .claims
        .iter()
        .map(|claim| claim.claim_id.clone())
        .collect();
    let mut pr_by_id = HashMap::new();
    let mut errors = Vec::new();

    for pr in &plan.planned_prs {
        if pr_by_id.contains_key(&pr.planned_pr_id) {
            errors.push(format!("duplicate planned_pr_id {}", pr.planned_pr_id));
            continue;
        }
        pr_by_id.insert(pr.planned_pr_id.clone(), pr);
    }

    for case in &plan.cases {
        for dependency in &case.depends_on {
            if !cases_by_id.contains_key(dependency) {
                errors.push(format!(
                    "case {} depends on missing case {dependency}",
                    case.case_id
                ));
            }
        }
        for claim_id in &case.claim_ids {
            if !claim_ids.contains(claim_id) {
                errors.push(format!(
                    "case {} references missing claim_id {claim_id}",
                    case.case_id
                ));
            }
        }
        let Some(planned_pr_id) = &case.planned_pr else {
            continue;
        };
        match pr_by_id.get(planned_pr_id) {
            Some(pr) if pr.case_id.as_deref() == Some(case.case_id.as_str()) => {}
            Some(pr) => errors.push(format!(
                "case {} references planned_pr {planned_pr_id}, but that PR maps to case {}",
                case.case_id,
                pr.case_id.as_deref().unwrap_or("<missing>")
            )),
            None => errors.push(format!(
                "case {} references missing planned_pr {planned_pr_id}",
                case.case_id
            )),
        }
    }

    for (planned_pr_id, pr) in &pr_by_id {
        let Some(case_id) = &pr.case_id else {
            continue;
        };
        let Some(case) = cases_by_id.get(case_id) else {
            errors.push(format!(
                "planned_pr {planned_pr_id} references missing case {case_id}"
            ));
            continue;
        };
        if case.planned_pr.as_deref() != Some(planned_pr_id.as_str()) {
            errors.push(format!(
                "planned_pr {planned_pr_id} maps to case {case_id}, but that case references planned_pr {}",
                case.planned_pr.as_deref().unwrap_or("<missing>")
            ));
        }
    }

    for proof in &plan.proofs {
        if !claim_ids.contains(&proof.claim_id) {
            errors.push(format!(
                "proof_strategy references missing claim_id {}",
                proof.claim_id
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{TraceCase, TraceClaim, TracePlannedPr, TraceProof};

    #[test]
    fn detects_missing_case_reference_without_io() {
        let plan = TracePlan {
            status: "ready".to_string(),
            cases: vec![TraceCase {
                case_id: "CASE-001".to_string(),
                depends_on: vec!["CASE-MISSING".to_string()],
                claim_ids: vec!["CLM-001".to_string()],
                planned_pr: Some("PR-001".to_string()),
            }],
            claims: vec![TraceClaim {
                claim_id: "CLM-001".to_string(),
            }],
            planned_prs: vec![TracePlannedPr {
                planned_pr_id: "PR-001".to_string(),
                case_id: Some("CASE-001".to_string()),
            }],
            proofs: vec![TraceProof {
                claim_id: "CLM-001".to_string(),
            }],
        };

        assert_eq!(
            validate_case_pr_links(&plan),
            vec!["case CASE-001 depends on missing case CASE-MISSING"]
        );
    }

    #[test]
    fn accepts_consistent_ready_links_without_io() {
        let plan = TracePlan {
            status: "ready".to_string(),
            cases: vec![TraceCase {
                case_id: "CASE-001".to_string(),
                depends_on: Vec::new(),
                claim_ids: vec!["CLM-001".to_string()],
                planned_pr: Some("PR-001".to_string()),
            }],
            claims: vec![TraceClaim {
                claim_id: "CLM-001".to_string(),
            }],
            planned_prs: vec![TracePlannedPr {
                planned_pr_id: "PR-001".to_string(),
                case_id: Some("CASE-001".to_string()),
            }],
            proofs: vec![TraceProof {
                claim_id: "CLM-001".to_string(),
            }],
        };

        assert!(validate_case_pr_links(&plan).is_empty());
    }
}
