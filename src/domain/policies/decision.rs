use std::collections::HashMap;

use crate::domain::entities::HumanDecisionSummary;

pub(crate) fn decision_blockers(
    decisions: &[HumanDecisionSummary],
    answers: &HashMap<String, String>,
) -> Vec<HumanDecisionSummary> {
    decisions
        .iter()
        .filter(|decision| !decision_has_approval(decision, answers))
        .cloned()
        .collect()
}

fn decision_has_approval(
    decision: &HumanDecisionSummary,
    answers: &HashMap<String, String>,
) -> bool {
    let Some(answer) = decision_receipt_answer(decision, answers) else {
        return false;
    };
    answer_is_approval(decision, &answer)
}

pub(crate) fn decision_receipt_answer(
    decision: &HumanDecisionSummary,
    answers: &HashMap<String, String>,
) -> Option<String> {
    std::iter::once(&decision.decision_id)
        .chain(decision.alias_ids.iter())
        .find_map(|decision_id| answers.get(decision_id).cloned())
}

pub(crate) fn answer_is_approval(decision: &HumanDecisionSummary, answer: &str) -> bool {
    let normalized = normalize_decision_answer(answer);
    if normalized.is_empty() {
        return false;
    }
    if is_explicit_non_approval(&normalized) {
        return false;
    }
    if decision
        .option_ids
        .iter()
        .any(|option_id| normalized == normalize_decision_answer(option_id))
    {
        return true;
    }
    normalized == normalize_decision_answer(&decision.recommended_option_id)
        || matches!(
            normalized.as_str(),
            "yes"
                | "y"
                | "approve"
                | "approved"
                | "accept"
                | "accepted"
                | "ok"
                | "okay"
                | "confirm"
                | "confirmed"
        )
}

fn is_explicit_non_approval(normalized_answer: &str) -> bool {
    matches!(
        normalized_answer,
        "revise"
            | "revise_intake"
            | "revise_top_level"
            | "reject"
            | "rejected"
            | "deny"
            | "denied"
            | "hold"
            | "held"
            | "hold_for_repair"
            | "defer"
            | "deferred"
            | "no"
            | "n"
            | "cancel"
            | "blocked"
            | "block"
    )
}

fn normalize_decision_answer(answer: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;
    for character in answer.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision() -> HumanDecisionSummary {
        HumanDecisionSummary {
            decision_id: "HD-FDA-001".to_string(),
            alias_ids: vec!["HDP-FDA-001".to_string()],
            summary: "scope approval".to_string(),
            recommended_option_id: "approve_scope".to_string(),
            option_ids: vec!["approve_scope".to_string(), "revise".to_string()],
            required_before: "Design Gate".to_string(),
        }
    }

    #[test]
    fn approval_answers_unblock_decision_without_io() {
        let decisions = vec![decision()];
        let answers = HashMap::from([("HD-FDA-001".to_string(), "yes".to_string())]);
        assert!(decision_blockers(&decisions, &answers).is_empty());
    }

    #[test]
    fn explicit_non_approval_blocks_decision_without_io() {
        let decisions = vec![decision()];
        let answers = HashMap::from([("HD-FDA-001".to_string(), "revise".to_string())]);
        assert_eq!(decision_blockers(&decisions, &answers).len(), 1);
    }

    #[test]
    fn aliases_can_resolve_decisions_without_io() {
        let decisions = vec![decision()];
        let answers = HashMap::from([("HDP-FDA-001".to_string(), "approve_scope".to_string())]);
        assert!(decision_blockers(&decisions, &answers).is_empty());
    }
}
