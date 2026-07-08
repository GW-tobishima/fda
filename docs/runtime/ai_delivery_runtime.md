# AI Delivery Runtime

AI Delivery Runtime は、標準成果物 v0 を入力として Program / Epic / Case / Task / PR / Proof の実行計画を作る外部実行層です。

ATO と Forge の責務を取り込まず、以下の境界を守ります。

| 領域 | 正本 |
|---|---|
| ATO | task/run/summary/human decision/evidence/handoff/AI repair lane |
| Forge | ClaimContract/Proof Obligation/PromotionDecision/ReleasePromotionDecision/Human Exception Firewall |
| forge-delivery-agent | role orchestration/model contract/adapter/eval |
| Execution Adapter | GitHub/Codex/sandbox/CI/model provider の具体実行 |

## 入力

- `docs/standards/delivery-artifacts-v0/templates/01_requirements_definition.md`
- `docs/standards/delivery-artifacts-v0/schemas/requirements_definition.schema.json`
- Autonomy Contract
- repository profile
- Forge policy
- ATO task graph policy

## 出力

- Epic Delivery Plan
- Case Graph
- Task Graph
- Human Decision Plan
- role handoff
- evidence link plan
- artifact inventory / Output Hub feed
- runner explanation packet

## 状態 lane

Runtime lane は Mission Control の分類であり、ATO へ戻すときは canonical state に変換します。
`merge_ready` / `release_ready` は表示 lane であり、その文字列を Task Graph や Epic Delivery Plan の state として直書きしません。

| runtime lane | ATO task.status | Epic state | 説明 |
|---|---|---|---|
| `planning` | `planned` | `draft` | Requirements から Epic Delivery Plan を作る |
| `ready_to_work` | `ready_to_work` | `ready` | scope / decision point / PR plan が揃った |
| `running` | `in_progress` | `running` | 実装または QA 実行中 |
| `ai_repair` | `ai_repair` | `running` | test 未実行、missing proof、stale evidence、trace gap |
| `human_turn` | `human_turn` | `human_turn` | human-only decision が必要。Mission Control 上では Judgment Required と表示してよい |
| `merge_ready` | `done` | `running` | PromotionDecision、QA、CI green、open human decision なしが揃った merge candidate |
| `release_ready` | `done` | `running` | ReleasePromotionDecision と rollback/smoke proof が揃った release candidate |

## Human-facing surface policy

ATO-HQ UIUX Knowledge Pack に基づき、human-facing surface は AI の内部実行単位ではなく、人間の判断と成果物受領を中心にします。

参照:

- `docs/knowledge/ato_hq_uiux/README.md`
- `docs/knowledge/ato_hq_uiux/entries/kn_01KVKG7FG9XVA3PEF7GYTNDXPN.md`
- `docs/knowledge/ato_hq_uiux/entries/kn_01KVKG7FH49649SBZEBX9CB4SR.md`
- `docs/knowledge/ato_hq_uiux/entries/kn_01KVKG7FJ0GHA1E6HT4EW86K0S.md`
- `docs/knowledge/ato_hq_uiux/entries/kn_01KVKG7FJGQB6XAB7Y1825KJ7F.md`
- `docs/knowledge/ato_hq_uiux/entries/kn_01KVKGFHGCEVG0MS3VJ6C02NNX.md`

Runtime は次を守ります。

- Task / Run / Agent は既定の人間向け表示単位にしない。
- 既定表示は Program / Epic / Case / Artifact / Human Decision / Risk / Outcome / Evidence に寄せる。
- Task / Run / Agent の詳細は監査・デバッグ用 drill-down として保持する。
- 人間に Job Packet / execution packet を手で作らせない。
- 人間は自然言語の目的、制約、成功条件、避けたいことを渡し、runtime が Job Packet / Work Contract / Trace Keys を生成する。
- Codex CLI が自然な hands-on entry point の場合、HQ/Mission Control は CLI の代替ではなく、backlog、複数案件俯瞰、成果物閲覧、非同期レビュー、監査、Human Turn 集約を担う。

## Output Hub / Artifact Inbox contract

Runtime は、依頼作成 UI が主機能でない場合でも、生成 artifact の即時閲覧導線を出力します。

各 artifact には最低限次を持たせます。

- artifact_id
- artifact_type
- title
- producer_adapter
- producer_agent, if available
- related_program_id / epic_id / case_id / task_id
- latest_version
- preview_summary
- path_or_url
- open_in_editor_link, if available
- open_in_browser_link, if available
- diff_link, if available
- evidence_links
- created_at / updated_at

Output Hub は、設計文書、レポート、handoff、QA verdict、PromotionDecision、Human Decision Packet を種類別に grouped view できる前提で設計します。

## Runner explanation packet

retry / resume / rerun の導線は opaque な起動ボタンにしません。

runner 連携 adapter は、少なくとも次を返します。

- current_phase
- previous_run_id
- diff_from_previous_run
- execution_actor
- input_summary
- changed_input_summary
- stop_condition
- next_action
- automation_boundary
- completion_evidence
- failure_evidence

人間が見る画面では、stdout / stderr の全文ではなく、実行の因果関係、責任境界、停止条件、次に起きること、完了/失敗の証跡を説明します。

## Human Exception Firewall

Runtime は次を自己承認しません。

- Scope In/Out 変更
- security High/Critical 例外
- public API breaking change
- data migration
- release approval
- Autonomy Contract の権限拡張

次は human decision ではなく AI repair に戻します。

- missing proof
- test not run
- stale evidence
- trace gap
- review packet missing
- schema repair

## v0 の非目標

- ATO 内で model を呼ぶこと
- Forge 内で PR を作ること
- runtime が ATO task state を置換すること
- runtime が merge / release approval を自己承認すること
- HQ/Mission Control が Codex CLI の劣化コピーになること
