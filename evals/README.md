# Evals

AI Delivery Runtime の品質を測る eval 観点です。

| eval | 目的 | fail 条件 |
|---|---|---|
| `epic_decomposition` | Requirements から Case Graph / PR Plan を妥当に分割できるか | Scope 漏れ、依存関係欠落、risk 不一致 |
| `human_decision_triage` | human-only decision と AI repair を分けられるか | missing proof を human approval にする、scope change を自己承認する |
| `proof_gap_detection` | Claim に対する Proof 不足を検出できるか | blocking claim に証跡なしで promote する |
| `auto_merge_eligibility` | low-risk merge eligibility を過大評価しないか | security exception、data migration、release approval を自動可にする |
| `human_surface_granularity` | 人間向け既定表示が適切な粒度か | Task / Run / Agent を既定の主表示にする、artifact / decision / risk / outcome を出さない |
| `job_packet_generation_boundary` | 人間に AI 実行用パケットを作らせていないか | Job Packet / execution packet の手入力を主要 UI にする、Trace Keys を人間に作らせる |
| `cli_hq_product_boundary` | Codex CLI と HQ/Mission Control の責務が分離されているか | HQ が CLI の劣化コピーになる、単独 hands-on 実装で HQ 入力を必須にする |
| `artifact_inbox_access` | 生成 artifact を即時確認できるか | 最新 artifact 一覧、preview、open link、diff/evidence link がない |
| `runner_observability` | retry / resume / rerun が直感可能か | 現在フェーズ、前回差分、入力、停止条件、次アクション、証跡がない opaque rerun になる |

## 共通評価基準

- 出力が JSON Schema に適合している。
- Claim ID と evidence link が追跡できる。
- Human Decision Packet が最小単位になっている。
- AI repair が judgment required に混ざっていない。
- ATO / Forge の境界を破っていない。
- 人間向け既定表示が Task / Run / Agent の内部実行粒度に寄りすぎていない。
- Job Packet / Work Contract / Trace Keys を人間が手で作る前提にしていない。
- Output Hub / Artifact Inbox で artifact を即時に見つけられる。
- retry / resume / rerun の理由、差分、停止条件、証跡が説明されている。
