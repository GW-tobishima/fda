# Agents

AI Delivery Organization の role contract です。

各 role は model 名ではなく、入力 schema、出力 schema、許可 action、禁止 action、handoff key、evidence policy で管理します。

| role | 主責務 | 主な出力 |
|---|---|---|
| Program Orchestrator | Program 全体の進行、ATO/Forge への記録 | dispatch plan、checkpoint summary |
| Requirements Analyst | 要件から scope / FR / NFR / open question を抽出 | requirements normalization |
| Solution Architect | 基本設計、境界、NFR response を作る | basic design、architecture claim |
| Work Breakdown Planner | Epic Plan、Case Graph、Task Graph を作る | epic_delivery_plan、task_graph |
| Implementer | SBI / Detailed Design に基づき変更する | PR diff、handoff、evidence |
| Functional QA | AC と user flow を検証する | functional QA verdict |
| Security QA | SEC / NFR security を検証する | security QA verdict |
| Contract QA | schema、mapping、handoff、trace を検査する | contract QA verdict |
| Merge Manager | merge candidate を整理する | merge handoff、open risk |
| Human Liaison | Human Decision Packet を作る | decision packet |

## 共通禁止事項

- Scope In/Out を人間承認なしに変更しない。
- security High/Critical 例外を自己承認しない。
- merge / release approval を自己承認しない。
- ATO state を Markdown summary だけで代替しない。
- Forge `promote` を merge approval として扱わない。
