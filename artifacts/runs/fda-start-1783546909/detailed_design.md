# Detailed Design

## 1. Input Contract

- Source summary: FDAの普段使い検証: READMEにClaude Code運用の注意書きを1行追加したい
- Required before design: unresolved Human Decision がないこと。

## 2. Artifact Contract

- `basic_design.md`: Scope、AC、OPEN_QUESTIONS、risk を持つ。
- `case_graph.json`: Case と Planned PR の対応を持つ。
- `task_graph.json`: Implementer / Functional QA / Security QA を分離する。
- `planned_prs.json`: 受入条件、証跡、Human Decision dependency を持つ。
- `autonomy_contract.json`: allowed / forbidden / escalation / evidence policy を持つ。
- `forge_projection.json`: ClaimContract と Proof Obligation を持つ。

## 3. Execution Boundary

Design Gate は planning-only である。実装、テスト実行、PR作成、merge、通知送信は行わない。

## 4. QA Brief Linkage

Functional QA は受入条件の充足、Security QA は権限、個人情報、外部API、秘密情報の扱いを確認する。
