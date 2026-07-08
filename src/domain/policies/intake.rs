use crate::domain::entities::{HumanDecisionSummary, IntakeClassification};
use crate::domain::value_objects::IntakeMode;

pub(crate) fn classify_intake(mode: IntakeMode, body: &str) -> IntakeClassification {
    let inferred_mode = if mode == IntakeMode::Auto {
        infer_intake_mode(body)
    } else {
        mode
    };

    match inferred_mode {
        IntakeMode::Research => IntakeClassification {
            name: "research_ready",
            mode: "research",
            summary: "実装前に調査成果物へ分岐する入力として扱う。",
            next_gate: "Research Mode Gate",
        },
        IntakeMode::Uiux => IntakeClassification {
            name: "uiux_ready",
            mode: "uiux",
            summary: "UIUX brief と mock 生成へ分岐する入力として扱う。",
            next_gate: "UIUX Mode Gate",
        },
        IntakeMode::DesignOnly => IntakeClassification {
            name: "design_only_ready",
            mode: "design-only",
            summary: "基本設計と詳細設計までを成果物化する入力として扱う。",
            next_gate: "Design Gate",
        },
        IntakeMode::Implement | IntakeMode::Auto => IntakeClassification {
            name: "implementation_candidate",
            mode: "implement",
            summary: "Human Decision 解決後に Design Gate へ進める実装候補として扱う。",
            next_gate: "Design Gate",
        },
    }
}

fn infer_intake_mode(body: &str) -> IntakeMode {
    let lower = body.to_lowercase();
    if ["調査", "research", "法務", "リスク"]
        .iter()
        .any(|token| lower.contains(token))
    {
        return IntakeMode::Research;
    }
    if ["ui", "ux", "モック", "mock", "画面"]
        .iter()
        .any(|token| lower.contains(token))
    {
        return IntakeMode::Uiux;
    }
    if ["設計", "design-only", "基本設計", "詳細設計"]
        .iter()
        .any(|token| lower.contains(token))
    {
        return IntakeMode::DesignOnly;
    }
    IntakeMode::Implement
}

pub(crate) fn intake_mode_name(mode: IntakeMode) -> &'static str {
    match mode {
        IntakeMode::Auto => "auto",
        IntakeMode::Implement => "implement",
        IntakeMode::Research => "research",
        IntakeMode::Uiux => "uiux",
        IntakeMode::DesignOnly => "design-only",
    }
}

pub(crate) fn intake_decisions(classification: &IntakeClassification) -> Vec<HumanDecisionSummary> {
    vec![
        HumanDecisionSummary {
            decision_id: "HD-FDA-001".to_string(),
            alias_ids: Vec::new(),
            summary: "入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか".to_string(),
            recommended_option_id: "approve_scope".to_string(),
            option_ids: vec!["approve_scope".to_string(), "revise".to_string()],
            required_before: classification.next_gate.to_string(),
        },
        HumanDecisionSummary {
            decision_id: "HD-FDA-002".to_string(),
            alias_ids: Vec::new(),
            summary: format!(
                "実装可否分類 `{}` と次 gate `{}` を採用してよいか",
                classification.name, classification.next_gate
            ),
            recommended_option_id: "accept_classification".to_string(),
            option_ids: vec!["accept_classification".to_string(), "revise".to_string()],
            required_before: classification.next_gate.to_string(),
        },
        HumanDecisionSummary {
            decision_id: "HD-FDA-003".to_string(),
            alias_ids: Vec::new(),
            summary: "外部API、個人情報、法務制約の未記載項目を Design Gate で明示確認する前提で進めてよいか".to_string(),
            recommended_option_id: "confirm_before_design".to_string(),
            option_ids: vec!["confirm_before_design".to_string(), "revise".to_string()],
            required_before: classification.next_gate.to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_research_inputs_without_io() {
        let classification = classify_intake(IntakeMode::Auto, "法務リスクを調査したい");
        assert_eq!(classification.name, "research_ready");
        assert_eq!(classification.next_gate, "Research Mode Gate");
    }

    #[test]
    fn creates_intake_decisions_from_classification_without_io() {
        let classification = classify_intake(IntakeMode::Implement, "実装したい");
        let decisions = intake_decisions(&classification);
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].decision_id, "HD-FDA-001");
        assert_eq!(decisions[0].required_before, "Design Gate");
    }
}
