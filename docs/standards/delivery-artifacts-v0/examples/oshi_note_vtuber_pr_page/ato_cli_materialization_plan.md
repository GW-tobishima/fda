---
artifact_type: ato_cli_materialization_plan
version: v0
status: draft
---

# ATO CLI Materialization Plan: oshi-note VTuber紹介リンク / PRページ

## 0. Boundary

このPoC-6Aでは ATO CLI を実行しない。ここに書く内容は、後続で人間が実行を承認した場合の materialization draft であり、今回の証跡はPlanning-only artifactに限定する。

## 1. Intended ATO Task / Run Shape

- Task Key候補: `ON-VPR-6A`
- Title候補: `oshi-note VTuber referral PR page high-risk epic planning`
- Role: `orchestrator`
- Workspace Policy: `read_only` または implementation開始時は `dedicated_worktree`
- Capability Profile: `planning`
- Current State: `HUMAN_TURN`相当の未解決Human Decisionあり。ただし今回のPoCではATOへ書き込まない。

## 2. Materialization Order

1. `ato work begin` でPlanning task/runを開始する。
2. `human_input_spec.md`、`requirements_definition.md`、`risk_register.md` をevidenceとしてcheckpointする。
3. `human_decision_packet.json` をtyped decision packetとしてblock登録する。
4. `epic_delivery_plan.json`、`case_graph.json`、`task_graph.json`、`planned_prs.json` をcheckpointする。
5. validation結果をcheckpointする。
6. Human Decisionが未解決のため、後続実装taskは `ready_for_ai` にせず `needs_human` として扱う。

## 3. Human Decision Mapping

| Human Decision | ATO Reason Type | Decision Packet |
|---|---|---|
| 本人確認済みVTuber限定 | spec_decision | HDPACKET-ON-VPR-6A |
| 少数母数非表示閾値 | risk_approval | HDPACKET-ON-VPR-6A |
| 通常メモ非公開 | risk_approval | HDPACKET-ON-VPR-6A |
| 公開用コメントのopt-in | spec_decision | HDPACKET-ON-VPR-6A |
| moderation要否 | risk_approval | HDPACKET-ON-VPR-6A |
| PR表示 | risk_approval | HDPACKET-ON-VPR-6A |
| 素材許諾 | risk_approval | HDPACKET-ON-VPR-6A |
| YouTube/Twitch API Data利用有無 | risk_approval | HDPACKET-ON-VPR-6A |

## 4. Evidence Edges To Attach Later

- `human_input_spec.md`: user input and source summary.
- `requirements_definition.md`: functional/non-functional requirements.
- `risk_register.md`: high-risk planning register.
- `human_decision_packet.json`: typed human decision packet.
- `epic_delivery_plan.json`: claim/case/pr/proof plan.
- `case_graph.json`: Forge case projection.
- `task_graph.json`: ATO task graph projection.
- `planned_prs.json`: downstream implementation PR boundaries.
- `validation_report.json`: local artifact validation result.

## 5. Stop Conditions

- Human Decision Packet is unresolved.
- Any downstream implementation would expose normal memos, low-count aggregates, direct identifiers, free-form URLs, or raw logs.
- ATO canonical backend is unknown.
- ATO CLI execution is still forbidden by the current PoC constraints.
