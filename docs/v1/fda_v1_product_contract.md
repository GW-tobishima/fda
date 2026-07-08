# FDA V1 Product Contract

## 1. 目的

FDA V1 は、CLI から入力された「やりたいこと」を、要件定義、Human Decision、設計、現在の Codex CLI による実装、レビュー、PR、merge handoff、または非実装成果物生成まで通しで制御する AI Delivery Organization Runtime である。

FDA 自身は ATO や Forge の正本を置き換えない。FDA は外部実行層として、ATO の task / run / decision / evidence と Forge の Claim / Proof / PromotionDecision を読み書きし、Codex / Claude / GitHub / CI / email などの adapter を制御する。

V1 の一文定義:

> FDA V1 は Codex CLI primary の AI Delivery Skill Pack / Work Protocol であり、人間が開いた現在の Codex CLI を実装者・オーケストレーターとして使いながら、Stage Gate と Human Decision で止まり、成果物または PR / merge handoff まで進める。

V1 主経路:

```text
Human -> Codex CLI -> FDA Skill Pack / Work Protocol -> current repo / target repo
```

Codex / Claude MCP direct implementer は V1.5 以降の optional automation layer とし、V1の主経路またはDone blockerにはしない。

## 2. V1 の利用者体験

人間は Job Packet、Work Contract、Trace Key、agent 実行パケットを手で作らない。

人間が入力するもの:

- 目的
- 制約
- 成功条件
- 避けたいこと
- 必要なら対象 repo / branch / mode

FDA が生成するもの:

- Requirements Definition
- Non-functional Requirements
- Risk Register
- Human Decision Packet
- Basic Design
- Detailed Design
- Case Graph
- Task Graph
- Planned PRs
- Autonomy Contract
- Forge Projection
- Implementation Handoff
- QA Receipts
- External PR Receipt
- Output Hub

## 3. SoT 境界

| 領域 | 正本 | FDA V1 の責務 |
|---|---|---|
| 作業状態 | ATO | task / run / checkpoint / decision / evidence を作成、更新、参照する |
| Gate / proof | Forge | Claim / Proof / PromotionDecision / ReleasePromotionDecision を参照し、gate 結果を実行判断へ反映する |
| 実行 | Execution Adapter | Codex / Claude / GitHub / CI / notification を呼び出す |
| 成果物 | repository artifact | Markdown / JSON / HTML / Excalidraw / receipt を生成し、Output Hub へ集約する |

FDA は model runtime でも DB 正本でもない。ATO にない判断、Forge にない gate 判定、artifact に残らない検証結果を暗黙の成功扱いにしない。

## 4. V1 Scope

### Scope In

- `fda start` で自然言語または Markdown 入力を受け付ける。
- 要件定義書と Human Decision を生成し、CLI stdout と artifact の両方へ表示する。
- 未解決 Human Decision がある場合は実装へ進めない。
- `fda decide` で判断を記録し、再開可能にする。
- `fda design` で実装前設計を生成する。
- `implementation_handoff.md` を生成し、現在のCodex CLIがapproved scope内で実装へ進める。
- `fda implement` で current Codex CLI primary の実装handoff、receipt、PR receiptを回収する。
- `fda review` で PR Reviewer、Functional QA、Security QA を必須 read-only reviewer として別 receipt にし、`review_agent_gate.json` と `review_agent_gate_packet.md` に集約する。
- `fda continue` で現在状態から次 action を選ぶ。
- QA fail、missing proof、stale evidence、trace gap は AI repair へ戻す。
- High / Critical security、scope 変更、merge / release approval は Human Decision へ戻す。
- research / uiux / design-only の非実装 mode でも完了成果物を出す。
- `fda open` で Output Hub を開く、または生成する。
- `fda status` で現在の phase、未解決判断、次 command、主要 artifact を表示する。
- FDA対象repoに `.fda/` が無い場合、作業開始前に `.fda/` folder と必須profileを作成する。

### Scope Out

- ATO を実行ランタイムにすること。
- Forge を GitHub / CI / Codex 実行ランタイムにすること。
- Human Decision を FDA が自己承認すること。
- security / privacy / legal / release approval を low-risk として自動 merge すること。
- 実装者と QA の権限を混ぜること。
- Task / Run / Agent 粒度を人間向け既定 UI の主表示にすること。
- Codex / Claude MCP direct implementerをV1主経路またはV1 Done blockerにすること。
- Codex / Claude MCP の tool 名や通知 API の存在を hardcode して成功扱いにすること。

## 5. CLI Contract

V1 の最小 command:

```bash
fda start <goal-or-file>
fda status
fda open
fda decide <decision-id> --answer <answer>
fda design
fda implement
fda review
fda merge
fda continue
fda notify test
```

Mode を明示する場合:

```bash
fda start "調査して" --mode research
fda start "UI案を作って" --mode uiux
fda start "設計まで" --mode design-only
fda implement --dry-run
fda implement --live
```

`--dry-run` / `--live` は既存互換またはV1.5 automation向けの表現であり、V1主経路では current Codex CLI による `current_codex_cli_handoff.json`、`implementation_handoff.md`、receipt を正本にする。

CLI stdout は、次の順序で人間に返す。

1. 現在の結果
2. 未解決 Human Decision
3. 次に実行できる command
4. 主要 artifact path
5. 通知 / gate / blocker の状態

## 6. Stage Gates

| Gate | 入口 | 必須確認 | 通過条件 | fail 時 |
|---|---|---|---|---|
| Intake Gate | `fda start` | 要件、NFR、risk、実装可否分類、Human Decision | artifact と stdout に判断点が揃う | Human Decision または requirements repair |
| Design Gate | `fda design` | Scope In/Out、Given/When/Then AC、OPEN_QUESTIONS、risk、Functional QA brief、Security QA brief | 未解決 Human Decision がない | Human Decision または design repair |
| Profile Gate | FDA作業開始前 | `.fda/` 7ファイルprofile | 無ければ作業前に作成する | profile作成または Human Decision |
| Implementation Handoff Gate | `fda implement` | `.fda/` Profile Gate、`current_codex_cli_handoff.json`、approved scope、test command、forbidden changes | current Codex CLI が実装へ進める材料が揃う | repair または Human Decision |
| Development Gate | `fda implement` | handoff、test、scope drift、PR receipt | planned PR と actual PR が対応する | repair または Human Decision |
| Review Agent Gate | `fda review` | `pr_reviewer_receipt.json`、Functional QA、Security QA、`review_agent_gate.json`、`review_agent_gate_packet.md`、conditional reviewer の not_applicable 理由 | `pr_reviewer`、`functional_qa`、`security_qa` が read-only で通り、packet projectionが `check_review_agent_gate.py --packet-path` に通る。実PR packetへの反映はV1では自動実行しない | repair loop または Human Decision |
| Functional QA Gate | `fda review` | AC_TEST_MAPPING、再現手順、FAIL 分類 | AC と検証結果が 1:1 対応 | repair loop |
| Security QA Gate | `fda review` | secret / auth / privacy / injection / data handling | High / Critical 未解決なし | Human Decision または block |
| Merge Gate | `fda merge` | CI、QA receipts、Forge Gate、risk、open decisions、必要時 `review_agent_gate_packet.md` のPR番号付きreview packet反映 | V1ではauto mergeしない。`review_agent_gate_packet.md` が存在するrunでは `artifacts/review_packets/pr-<PR番号>.md` に `REVIEW_AGENT_GATE` と `MERGE_APPROVAL: not_granted` があり、最終merge approvalは人間判断へ戻す | merge approval または repair |

## 7. Human Decision Contract

Human Decision は人間だけが確定できる判断である。

Human Decision にするもの:

- Scope In / Scope Out の変更
- Acceptance Criteria の意味変更
- security High / Critical の例外
- privacy / legal / compliance 判断
- public API breaking change
- data migration
- merge approval
- release approval
- Autonomy Contract の権限拡張

Human Decision にしないもの:

- test 未実行
- missing proof
- stale evidence
- trace gap
- review packet missing
- schema validation failure
- format repair

後者は AI repair に戻す。

Human Decision registry record は `docs/standards/fda-v1/schemas/delivery-registry/human_decision.schema.json` と互換にする。

registry record の必須 field:

- schema_version
- decision_id
- reason
- status
- question
- owner_role
- created_at
- updated_at

registry record の任意 field:

- options
- recommended_option
- why_now
- impact_if_delayed
- trigger
- evidence_ids
- related_knowledge_ids

人間向けの `human_decision_packet.md` や通知文面は、schema record への projection として `title`、`required_before`、`resume_command`、`related_artifacts` を追加表示してよい。ただし registry へ登録する JSON は schema-compatible field のみにする。

## 8. Agent Role Contract

| Role | 権限 | 禁止事項 |
|---|---|---|
| FDA Orchestrator | 計画、gate 判定、ATO / Forge projection、implementation handoff作成 | Human Decisionの自己承認、role switchなしの実装 |
| Implementer | current Codex CLI による target repo の実装、test、PR 作成 | Human Decision 未解決の scope を実装すること |
| PR Reviewer | correctness / regression / blast radius / artifact整合性確認 | source mutation、merge approval、risk approval |
| Functional QA | AC 検証、再現、FAIL 分類 | source mutation、security 判定の代替 |
| Security QA | security / privacy / auth / secret / injection 検証 | Functional QA のコピペで代替すること |
| Forge / QAx2 Reviewer | ATO / Forge / FDA 証跡、handoff、review packet、human decision境界確認 | PromotionDecisionやmerge approvalの自己承認 |
| Design QA | UI / visual / browser surface確認 | Functional QAの代替 |
| Merge Role | merge gate 確認、policy に基づく merge / approval handoff | merge / release approval の自己承認 |

QA は read-only policy を持つ。実装者だけが write 可能な workspace を持つ。

同じCodex CLIセッションが Orchestrator から Implementer へ切り替わる場合は、ATO checkpoint、`implementation_handoff.md`、Review Agent Gate を必須にする。

## 9. Notification Contract

V1 の P0 通知先は Slack Incoming Webhook とする。email SMTPはdeprecated/docs-only互換として残すが、V1のHuman Turn主経路にはしない。Codex app notification / automation、GitHub issue、ATO UI、Slack interactivity は P2 とする。

通知は Human Decision が必要な場合に送る。通知失敗は task 失敗ではないが、`fda status` で visible にする。

通知 payload は最低限次を含む。

- decision_id
- summary
- options
- recommended_option
- due_at, if available
- resume_command
- artifact_links
- notification_receipt

## 10. Output Hub Contract

CLI-first でも Output Hub は必要である。

Output Hub は依頼作成 UI ではなく、成果物閲覧の面である。人間は Task / Run / Agent ではなく、成果物、未解決判断、リスク、期限、責任、最終結果を見る。

最小生成物:

- `output_hub.html`
- `decision_inbox.html`
- `execution_status.html`

Output Hub は次を grouped view で表示する。

- Requirements
- Designs
- Planned PRs
- Human Decisions
- QA Receipts
- Risk Register
- PR / CI / Merge receipts
- Research reports
- UIUX mock

## 11. V1 Done

FDA V1 は次を満たすと V1 と呼ぶ。

この一覧は「最終的に V1 と呼ぶための到達条件」である。`PR-V1-011` までで CLI command、artifact、gate、receipt の骨格は揃うが、外部 adapter が dry-run / receipt 生成止まりの場合は Operational V1 完了とは扱わない。

特に次は V1 complete の blocker である。

- FDA対象repoに `.fda/` 7ファイルprofileがなく、作業前に作成されていない。
- `slack` channel は `fda notify test --live --channel slack` で Slack Incoming Webhook への実送信を試行できる。`FDA_SLACK_WEBHOOK_URL` 未設定時は success 扱いにせず、fail-closed receipt を残す。
- `email` channel は deprecated/docs-only互換として残し、SMTP app password 方式の実送信を試行できる。
- `fda merge` が merge gate receipt 生成だけで、人間のmerge approval handoffと結果receiptを回収しない。
- `fda status` が現在 phase、未解決判断、通知状態、主要 artifact を実行 state から表示できない。
- current Codex CLI primary の `implementation_handoff.md`、test、PR 作成、PR URL 回収を実運用で検証できていない。

したがって、V1 の呼称は次の 2 段階に分ける。

- **V1 contract coverage:** command と artifact contract が揃い、各 gate の dry-run / fixture / receipt レベルの検証が通る状態。
- **V1 operational:** `.fda/` Profile Gate、email、current Codex CLI primary実装、GitHub PR / merge handoff、status が fixture なしで動き、失敗時に Human Decision または repair loop へ戻せる状態。

1. CLI からやりたいことを入力できる。
2. 要件定義書と Human Decision が生成される。
3. 判断事項が CLI stdout と要件定義書の両方に出る。
4. `fda decide` で判断を記録できる。
5. `fda design` で設計成果物を作れる。
6. 判断が必要なら止まり、通知できる。
7. 実装可能な場合、current Codex CLI が `implementation_handoff.md` に基づき approved scope 内で実装できる。
8. PR Reviewer / Functional QA / Security QA を別 read-only reviewerとして実行できる。
9. QA FAIL なら repair loop に入れる。
10. PR を作成できる。
11. Forge Gate を確認できる。
12. policy に応じて merge または Human approval に回せる。
13. 要件が満たされるまで AI 側で続行できる。
14. 調査依頼なら research report を出せる。
15. UIUX 依頼なら HTML または Excalidraw mock を出せる。
16. Output Hub で成果物を見られる。
17. `status` で今どこか分かる。
18. Codex / Claude MCP direct implementerはV1.5 optional automationとして文書化され、V1主経路と矛盾しない。

## 12. V1 以降へ送る未確定事項

次は V1 contract 上の blocker ではなく、後続 phase で確認する adapter 詳細である。

- Codex app notification / automation の公式 API route。
- Claude Code MCP server の local availability と version policy。
- V1.5 MCP agent invocation における secret / credential injection の標準形式。
- auto merge を許す low-risk policy の実 repository への適用条件。V1ではauto mergeしない。
- Output Hub の web UI 化範囲。
