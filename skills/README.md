# Workflow Skills

このディレクトリは、AI Delivery Runtime の workflow skeleton を置く場所です。
Codex skill として配布する場合は、各 workflow ごとに `SKILL.md` を追加します。

## v0 workflow

| workflow | 入力 | 出力 |
|---|---|---|
| `requirement_to_epic_plan` | Requirements Definition | Epic Delivery Plan |
| `epic_to_task_graph` | Epic Delivery Plan | ATO Task Graph |
| `forge_gate_evaluate` | Case / PR / Proof | PromotionDecision |
| `github_pr_prepare` | SBI / implementation handoff | PR candidate |
| `qa_repair_loop` | QA finding / missing proof | repair task |

## v1 registry

v1 では `skills/registry.yaml` を skill pack registry として扱います。
対象 repo 側の `.fda/skills.lock` は、この registry の `name` と `version` に固定します。

各 workflow は `skill.yaml` を持ち、次を明示します。

- `name`
- `version`
- `input_schema`
- `output_schema`
- `allowed_actions`
- `forbidden_actions`
- `required_artifacts`
- `validation_commands`
- `escalation_conditions`

初期 registry:

| skill | version | 目的 |
|---|---:|---|
| `requirement_to_epic_plan` | `v0.3.0` | Requirements Definition から Epic Delivery Plan を作る |
| `epic_to_task_graph` | `v0.1.0` | Epic / Case から Task Graph を作る |
| `epic_to_planned_prs` | `v0.2.0` | Epic / Case / Task から Planned PRs を作る |
| `forge_projection` | `v0.1.0` | Case / Task / Planned PR を Forge Claim / Proof mapping に投影する |
| `external_implementation_handoff` | `v0.1.0` | 対象 repo 実装 agent への handoff を作る |
| `external_pr_receipt_collect` | `v0.1.0` | actual PR / CI / deviation / evidence を回収する |
| `ato_cli_materialization` | `v0.1.0` | ATO CLI write plan と command preview を作る |
| `human_decision_triage` | `v0.2.0` | Human Decision と AI Repair を分離する |
| `proof_gap_detection` | `v0.1.0` | Claim / proof obligation と evidence の不足を検出する |

## 境界

- workflow は human-only decision を自己承認しない。
- workflow は ATO / Forge の state schema 正本を fork しない。
- workflow は schema validation failure を AI repair として扱う。
