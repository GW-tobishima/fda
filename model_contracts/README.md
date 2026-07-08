# Model Contracts

role 別の技術契約です。model 名ではなく、入力、出力、権限、禁止事項、監査 key で管理します。

| contract | role | input | output |
|---|---|---|---|
| `planner.contract.yaml` | work_breakdown_planner | Requirements Definition | Epic Delivery Plan |
| `implementer.contract.yaml` | implementer | SBI / Detailed Design | implementation handoff |
| `qa.contract.yaml` | functional_qa / security_qa / contract_qa | PR / evidence | QA verdict |
| `merge.contract.yaml` | merge_manager | PR / PromotionDecision / QA verdict | merge handoff |

標準成果物 v0 側の planner contract は `docs/standards/delivery-artifacts-v0/model_contracts/planner.contract.yaml` にも保持します。
