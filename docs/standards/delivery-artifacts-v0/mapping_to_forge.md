---
standard_id: delivery-artifacts-v0.mapping-to-forge
version: v0
status: draft
last_reviewed: 2026-06-20
review_cycle_days: 30
owner: forge-delivery-agent
---

# Forge マッピング v0

## Forge の責務

Forge は「進めてよいか」を判定する制度として扱う。AI Delivery Runtime は成果物を作るが、昇格可否は Forge の Claim、Proof、Gate、PromotionDecision で決める。

## 共通 ID

| ID | 形式 | 説明 |
|---|---|---|
| Program ID | `PROGRAM-001` | 複数 Epic を束ねる単位 |
| Epic ID | `EPIC-001` | 成果単位 |
| Case ID | `CASE-001` | Forge が評価する作業単位 |
| Claim ID | `CLM-001` | 証明対象の主張 |
| Proof ID | `PRF-001` | Claim を支える証跡 |
| Gate ID | `GATE-001` | 昇格判定のルール |
| PromotionDecision ID | `PROMO-001` | Case または PR の昇格判断 |

## 成果物別マッピング

| 成果物 | Forge 対応 | 必須項目 |
|---|---|---|
| Requirements Definition | Claim 候補、Scope 境界、Human Decision 候補 | `scope_in`, `scope_out`, `functional_requirements`, `human_decision_points` |
| Basic Design | Design Claim、Proof Obligation | `solution_strategy`, `interfaces`, `quality_mapping` |
| Detailed Design | Case 実装契約 | `case_id`, `planned_pr`, `target_claims`, `test_plan` |
| Epic | Epic ClaimContract、Case Graph、PR Plan | `claim_tree`, `case_graph`, `pr_plan` |
| PBI | Value Claim、Acceptance Criteria | `acceptance_criteria`, `forge_mapping.claim_ids` |
| SBI | 実行 Task と Proof 計画 | `done_conditions`, `validation`, `handoff` |
| NFR | Quality Claim、Proof Obligation | `quality_attribute`, `metric`, `target`, `verification` |
| Design Agreement | UI/UX Claim、Accessibility Proof | `interaction_principles`, `accessibility_agreement`, `acceptance_criteria` |
| Issue | Intake Claim seed | `objective`, `scope`, `autonomy_level` |
| Pull Request | PromotionDecision input | `claim_ids`, `qa_verdict`, `validation_evidence`, `rollback_plan` |
| Autonomy Contract | Gate policy input | `allowed_actions`, `forbidden_actions`, `escalation_rules` |
| Human Decision Packet | Human Exception Firewall input | `decision_needed`, `options`, `impact`, `required_before` |

## Gate 分類

| Gate | 入力 | PASS 条件 | FAIL 時の扱い |
|---|---|---|---|
| Design Gate | Requirements, Basic Design, Design Agreement | Scope、Claim、NFR、UX 方針が追跡可能 | AI Repair または Human Decision |
| Development Gate | Detailed Design, SBI | 変更範囲、検証、Rollback が明確 | AI Repair |
| Functional QA Gate | PR, test evidence | AC が検証済み | AI Repair |
| Security QA Gate | PR, security evidence | High/Critical 未解決なし | Human Decision または block |
| Contract QA Gate | schemas, handoff, mappings | schema と trace が整合 | AI Repair |
| Merge Gate | PR, PromotionDecision | CI green、Proof complete、判断未解決なし | Human Decision または block |

## Human Exception Firewall

次の条件は AI が勝手に確定しない。

- Scope In/Out の変更。
- Security High/Critical の例外承認。
- public API breaking change。
- data migration。
- release approval。
- Autonomy Contract の権限拡張。

## AI Repair に留めるもの

- テスト未実行。
- 証跡不足。
- stale evidence。
- handoff key 不足。
- PR description 欠落。
- lint または format failure。
- 低リスクな schema repair。
