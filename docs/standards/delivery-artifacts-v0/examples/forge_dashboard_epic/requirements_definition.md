---
artifact_type: requirements_definition
version: v0
status: draft
---

# Forge Dashboard Epic 要件定義

## 0. Metadata

- Document ID: REQ-FORGE-DASHBOARD-001
- Version: v0
- Owner: forge-delivery-agent
- Status: draft
- Source: pasted planning memo
- Related Program: PROGRAM-FORGE-DASHBOARD-001
- Related Epic: EPIC-FORGE-DASHBOARD-001

## 1. Business Objective

- Problem: AI Delivery Organization の状態、判断待ち、AI 修復待ち、証跡がチャットログに埋もれる。
- Desired Outcome: Program / Epic / Case / PR の状態を Mission Control として観測できる設計契約を作る。
- Success Metrics:
  - Human Decision と AI Repair が別レーンとして定義されている。
  - Forge Claim と ATO Task Graph の対応が追跡できる。
  - UI 実装なしで UX 合意をレビューできる。
- Non-goals:
  - 今回は UI 実装をしない。
  - 自動マージ実装はしない。

## 2. Scope

### Scope In

- Mission Control の情報設計。
- Human Decision と AI Repair の分離。
- Epic Delivery Plan のサンプル。
- Forge / ATO mapping の確認。

### Scope Out

- React / Tauri / Web UI 実装。
- Codex process supervisor。
- GitHub PR 自動作成。

## 3. Stakeholders / Users

- Primary users: Human reviewer, Program orchestrator
- Operators: ATO operator
- Reviewers: Forge gate reviewer
- Approvers: Human product owner

## 4. Functional Requirements

| ID | Requirement | Rationale | Priority | Acceptance Criteria | Source |
|---|---|---|---|---|---|
| FR-001 | Program / Epic / Case / PR の状態を階層で表現できる | 作業状態の観測に必要 | Must | AC-001 | planning memo |
| FR-002 | Human Decision と AI Repair を別レーンに分ける | 人間判断のノイズを減らす | Must | AC-002 | planning memo |
| FR-003 | Claim、Proof、Gate、PromotionDecision を同一画面概念で辿れる | Forge Gate の説明に必要 | Should | AC-003 | planning memo |

## 5. Non-Functional Requirements

| ID | Quality Attribute | Requirement | Measure | Target | Verification |
|---|---|---|---|---|---|
| NFR-UX-001 | Usability | 状態と詰まりを一覧できる | primary lane count | 7 lanes 以下 | design review |
| NFR-ACC-001 | Accessibility | 状態を色だけに依存しない | WCAG check | WCAG 2.2 AA | accessibility review |

## 6. Constraints

- UI 実装は後続 Phase に回す。
- UI/UX 設計は Phase 0 の標準成果物に含める。
- 実装 CLI が必要になった場合は Rust を使う。

## 7. Assumptions

- ATO は実行状態と Human Turn の SoT である。
- Forge は Gate と PromotionDecision の SoT である。

## 8. Open Questions

| ID | Question | Owner | Blocking? | Due |
|---|---|---|---|---|
| Q-001 | Mission Control UI の最初の実装面は Web か Tauri か | human | no | Phase 5 |

## 9. Human Decision Points

| ID | Trigger | Decision Needed | Options | Required Before |
|---|---|---|---|---|
| HDP-001 | UI 実装に着手する前 | Web / Tauri / ATO cockpit extension のどれで始めるか | Web, Tauri, ATO cockpit | Mission Control UI v0 |

## 10. Forge Mapping

- Claim IDs: CLM-001, CLM-002, CLM-003
- Proof Obligations: PRF-001, PRF-002
- Human Decision Points: HDP-001
- ATO Task Graph: TASK-FD-001, TASK-FD-002
- Planned PRs: PR-FD-001, PR-FD-002
- Gate Requirements: Design Gate, Contract QA Gate
