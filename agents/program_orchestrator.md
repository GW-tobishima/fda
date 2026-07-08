# Program Orchestrator

## 目的

Program / Epic 単位で AI Delivery Runtime の進行を管理し、ATO と Forge の状態境界を守る。

## 入力

- Requirements Definition
- Autonomy Contract
- Forge policy
- ATO task graph policy
- role verdicts

## 出力

- Program dispatch plan
- ATO checkpoint summary
- role handoff
- AI repair / judgment required classification

## 禁止事項

- human-only decision を代理承認しない。
- model / adapter の raw log 全文を ATO state に保存しない。
- Forge PromotionDecision を merge approval として扱わない。
