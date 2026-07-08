---
artifact_type: runtime_artifact_contracts
version: v0
status: draft
planned_pr_id: PR-GDAR-001
---

# Generic Daily Agent Runtime Artifact Contracts

## 目的

PR-GDAR-001では、AICX固有の `run_state` / receipt を削除せず、Generic Daily Agent Runtimeとして再利用できる最小の汎用contractを追加する。

このPRではruntime command実装、Slack adapter移行、ATO materialization writeは行わない。後続のPR-GDAR-002/003が実装へ進む前に、状態と証跡の語彙を固定する。

## 追加する汎用artifact

| Artifact | Schema | Fixture | 役割 |
|---|---|---|---|
| Generic Run State | `schemas/generic_run_state.schema.json` | `generic_run_state.json` | dispatch / event intake / action / maintain / status の現在状態、idempotency、artifact refs、human decisionsを保持する |
| Generic Receipt | `schemas/generic_receipt.schema.json` | `generic_receipt.json` | dispatch / event_intake / action / maintenance / status のreceipt例を同一contractで検証する |

## AICX artifactとの関係

- 既存の `run_state.schema.json`、`maintenance_receipt.schema.json`、Slack系receipt schemaは残す。
- Generic schemaはAICX固有フィールドを置き換えず、後続migrationでadapter境界へ写すための追加contractとして扱う。
- AICX互換性の破壊やdeprecated判断はPR-GDAR-003および `HD-GDAR-002` の範囲に残す。

## Claim / Proof対応

| Claim | Proof | Evidence |
|---|---|---|
| `CLM-GDAR-001` | `PRF-GDAR-001` | `generic_run_state.json` / `generic_receipt.json` schema validation |
| `CLM-GDAR-001` | `PRF-GDAR-002` | `artifact_inventory.json` / `runner_explanation.json` traceability |
| `CLM-GDAR-002` | `PRF-GDAR-001` | dispatch、event_intake、action、maintenance、status receipt examples |
| `CLM-GDAR-005` | `PRF-GDAR-002` | runtime code、ATO write、Forge実評価を行わないfailure evidence |

## 次に進む条件

- `validate-artifacts` が `generic_daily_agent_runtime` exampleでPASSする。
- `artifact_inventory.json` からPR-GDAR-001のschema、fixture、validation report、QA evidenceを辿れる。
- `runner_explanation.json` が次actionをPR-GDAR-002またはHuman Decision解決へ向ける。
