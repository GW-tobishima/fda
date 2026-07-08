---
artifact_type: design_agreement
version: v0
status: draft
---

# Forge Dashboard Epic デザイン合意

## 0. Metadata

- Design Agreement ID: DA-FORGE-DASHBOARD-001
- Related Epic: EPIC-FORGE-DASHBOARD-001
- Related UI: Mission Control
- Owner: forge-delivery-agent
- Status: draft

## 1. User Experience Goal

AI Delivery Organization の状態、詰まり、判断待ち、証跡を、チャットログではなく作業管制画面として把握できるようにする。

## 2. Interaction Principles

- What should feel fast: Program Tree から Case / PR / Evidence を辿る操作。
- What should be explicit: Human Decision、Autonomy Contract、Gate failure。
- What should be hidden: AI が自己修復できる一時的な内部試行。
- What must never be hidden: scope change、security exception、release approval。

## 3. Layout / IA

- Primary screen: Program Header + Program Tree + Main Work Surface + Decision Box。
- Navigation: Program / Epic / Case / PR / Evidence を同じ階層で移動する。
- Empty states: 計画未作成、証跡未登録、判断待ちなし、AI Repair なし。
- Error states: schema 不整合、ATO 書き込み失敗、Forge Gate 判定不能。

## 4. Accessibility Agreement

| WCAG Ref | Requirement | Verification |
|---|---|---|
| WCAG-2.2-AA | 状態を色だけで表現しない | design review |
| WCAG-2.2-AA | Decision と Repair をキーボードで辿れる | keyboard walkthrough |

## 5. Visual Tokens

- Color: status ごとに色とラベルを併用する。
- Typography: dashboard 向けの高密度表示。
- Spacing: pane 間の分離を明確にする。
- Density: 一覧性を優先する。
- Status colors: `human_turn` と `ai_repair` を明確に分ける。

## 6. Human Decision UX

- Decision types shown: scope_change, security_exception, release_approval。
- AI repair types hidden: test_not_run, stale_evidence, trace_gap。
- Escalation presentation: option、impact、default_if_no_decision を同時に表示する。

## 7. Acceptance Criteria

- AC-UI-001: Human Decision と AI Repair が別レーンとして説明できる。
- AC-UI-002: Epic から Claim、Case、Planned PR、Proof へ辿れる。

## 8. Implementation Timing

- UI implementation phase: Phase 5 以降。
- Required design artifacts before implementation: Design Agreement、Epic Delivery Plan、ATO mapping。
- Deferred implementation notes: v0 では画面設計だけを進める。

## 9. Forge Mapping

- Claim IDs: CLM-002
- Proof Obligations: PRF-002
- Human Decision Points: HDP-001
- ATO Task Graph: TASK-FD-002
- Planned PRs: PR-FD-002
- Gate Requirements: Design Gate
