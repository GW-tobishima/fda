# FDA V1 Roadmap

## 1. Roadmap 方針

FDA V1 は「最初から全自動実装」を目指さない。まず Codex CLI primary の契約と成果物を固定し、Profile Gate、Intake、Design、Implementation Handoff、current Codex CLI 実装、QA、repair、merge handoff、非実装 mode、通知、Output Hub の順に gate を増やす。

2026-06-29 のV1 pivotにより、Codex / Claude MCP direct implementerはV1.5 optional automation layerへ退避する。既存のMCP schema / dry-run資産は破棄しないが、V1主経路またはV1 Done blockerにはしない。

各 phase は、artifact、schema、receipt、gate 条件を残す。実装できない場合も途中停止ではなく、research / uiux / design-only の成果物へ分岐する。

## 2. Phase 一覧

| Phase | 名前 | 目的 | 主 command | 主成果物 |
|---|---|---|---|---|
| 0 | V1 要件・用語・CLI体験固定 | V1 の言葉をぶれない正本にする | なし | `docs/v1/*` |
| 0.5 | Repository Profile Gate | `.fda/` 7ファイルprofileを作業入口条件にする | なし | `.fda/*` |
| 1 | Intake -> Requirements -> Human Decision | やりたいことから要件と判断点を作る | `fda start` | requirements / NFR / risk / decision |
| 2 | Design Gate | 実装前に設計へ落とす | `fda design` | design / case graph / PR plan |
| 3 | Implementation Handoff Contract | current Codex CLIが実装へ進めるhandoffを固める | なし | implementation handoff / role policy |
| 4 | Current Codex CLI Implementation Gate | current Codex CLIがapproved scope内で実装できる状態にする | `fda implement` | implementation receipt / PR receipt |
| 5 | MCP Optional Automation Contract | V1.5向けにCodex / Claude MCP呼び出し契約を保持する | なし | MCP schemas / dry-run receipt |
| 6 | Review Agents | PR Reviewer、Functional QA、Security QA を分離する | `fda review` | reviewer receipts / review agent gate |
| 7 | Repair Loop | 要件充足まで AI 側で修正する | `fda continue` | repair receipt |
| 8 | PR / Merge Gate | merge 可能性を policy で判定する | `fda merge` | merge receipt / approval packet |
| 9 | Non-implementation Modes | 調査、UIUX、設計のみを完了成果物化する | `fda start --mode ...` | report / mock / design |
| 10 | Notifications | Human Decision で止まり通知する | `fda notify test` | notification request / receipt |
| 11 | Output Hub v0 | 成果物閲覧導線を作る | `fda open` | HTML hub |

## 3. Phase 0: V1 要件・用語・CLI体験固定

目的:

- FDA V1 を CLI-first の AI Delivery Organization Runtime として定義する。
- V1 の Human Decision、Stage Gate、Codex CLI primary、非実装 mode、Output Hub の契約を固定する。

成果物:

- `docs/v1/fda_v1_product_contract.md`
- `docs/v1/fda_v1_roadmap.md`
- `docs/v1/cli_user_journey.md`
- `docs/v1/codex_cli_primary_architecture.md`
- `docs/v1/mcp_agent_architecture.md`
- `docs/v1/notification_policy.md`
- `docs/v1/non_implementation_modes.md`

Done:

- CLI-first が明記されている。
- 入力から Requirements Definition と Human Decision を返す流れが明記されている。
- 実装 / 調査 / UIUX / 設計のみの分岐が定義されている。
- Codex CLI primary 利用方針が明記されている。
- Codex / Claude MCP は V1.5 optional automation として位置づけられている。
- Human Decision と AI Repair の境界が明記されている。

## 4. Phase 1: Intake -> Requirements -> Human Decision

目的:

- `fda start` で自然言語または Markdown を受け取り、要件定義と判断事項を生成する。

Command:

```bash
fda start "oshi-noteでVTuber紹介リンク/PRページを作りたい"
fda start --input docs/ideas/vtuber_pr_page.md
```

成果物:

- `requirements_definition.md`
- `non_functional_requirements.md`
- `risk_register.md`
- `human_decision_packet.md`
- `artifact_inventory.json`
- `runner_explanation.json`

受入条件:

- Human Decision が CLI stdout に表示される。
- 同じ Human Decision が `requirements_definition.md` にも記録される。
- 実装可否分類が出る。
- 次 action として `fda decide` または `fda design` が提示される。
- 未解決 Human Decision がある場合、実装系 command は gate で止まる。

## 5. Phase 2: Design Gate

目的:

- 実装前に基本設計、詳細設計、Case Graph、Task Graph、PR計画へ落とす。

Command:

```bash
fda design
```

成果物:

- `basic_design.md`
- `detailed_design.md`
- `case_graph.json`
- `task_graph.json`
- `planned_prs.json`
- `autonomy_contract.json`
- `forge_projection.json`

受入条件:

- Scope In / Scope Out がある。
- Given / When / Then の受け入れ基準がある。
- OPEN_QUESTIONS がある。
- risk と mitigation がある。
- Functional QA brief と Security QA brief がある。
- 判断が必要な場合は Human Decision として停止する。
- 通知が必要な Human Decision には通知必要性と通知候補 channel を記録する。実際の `notification_request.json` 生成は Phase 10 で扱う。

## 6. Phase 3: Implementation Handoff Contract

目的:

- current Codex CLI が approved scope 内で実装へ進めるための handoff、role switch、receipt 契約を固定する。

成果物:

- `implementation_handoff.md`
- `current_codex_cli_handoff.json`
- `implementation_receipt.json`
- `external_pr_receipt.json`
- `coding_agent_thread_state.schema.json`
- `planned_pr_execution_packet.schema.json`
- `agent_role_policy.schema.json`

受入条件:

- Orchestrator / Implementer / Functional QA / Security QA の role boundary が分かれている。
- current Codex CLI が implementer へ role switch する条件が明記されている。
- QA は read-only policy を持つ。
- Human Decision 未解決なら実装不可。
- Scope In / Scope Out、forbidden changes、test command、expected artifacts が handoff に残る。
- 実装結果を semantic receipt へ変換できる。

## 7. Phase 4: Current Codex CLI Implementation Gate

目的:

- current Codex CLI が implementation handoff に基づいて target repo で実装 PR を作れる状態にする。

Command:

```bash
fda implement
```

成果物:

- `current_codex_cli_handoff.json`
- `implementation_handoff.md`, optional prompt view
- `implementation_receipt.json`
- `external_pr_receipt.json`
- `coding_agent_thread_state.json`

受入条件:

- `.fda/` profileが存在する。無い場合は作業前に作成されている。
- Human Decision 未解決なし。
- Design Gate pass。
- worktree / branch policy が明記されている。
- current Codex CLI のrole switch checkpointがある。
- test 結果が receipt に入る。
- scope 逸脱が記録される。
- actual PR URL が回収される。

## 8. Phase 5: MCP Optional Automation Contract

目的:

- V1.5以降に、Codex / Claude MCP direct implementerや外部orchestratorを使うための契約をoptional automationとして保持する。

Command:

```bash
fda implement --dry-run
fda implement --live
```

成果物:

- `mcp_agent_invocation_plan.schema.json`
- `mcp_tool_call_receipt.schema.json`
- `dry_run_receipt.json`
- `mcp_agent_invocation_plan.json`

受入条件:

- V1主経路ではないことが明記されている。
- V1 Done blockerではない。
- Human Decision 未解決なら MCP invocation 不可。
- target repo mutationなしのdry-runと、semantic receipt変換ができる。
- tool 名、server 名、transport、cwd、approval policy、timeout、allowed paths が記録される。

## 9. Phase 6: Review Agents

目的:

- 実装 PR を PR Reviewer、Functional QA、Security QA で分離レビューする。
- ATO / Forge / FDA を使う開発では、Review Agent Gate を PR ready / merge 前の必須証跡にする。

Command:

```bash
fda review
```

処理:

- PR Reviewer read-only review
- Functional QA read-only review
- Security QA read-only review
- 必要に応じて Forge / QAx2 reviewer、Design QA を read-only で起動する。
- `pr_reviewer_receipt.json`、QA receipt、`review_agent_gate.json`、`review_agent_gate_packet.md` 生成
- V1では `review_agent_gate_packet.md` projection生成までを標準にし、実PRの `artifacts/review_packets/pr-<PR番号>.md` への反映は明示コマンドまたは人間確認後にする。
- FAIL なら repair へ戻す
- High / Critical なら Human Decision または block

受入条件:

- PR Reviewer、Functional QA、Security QA の出力が分離される。
- Security QA は Functional QA のコピペで代替されない。
- `review_agent_gate.json` があり、`pr_reviewer`、`functional_qa`、`security_qa` が必須 reviewer として記録される。
- `review_agent_gate_packet.md` が `scripts/check_review_agent_gate.py --packet-path` に通る。
- `REVIEW_AGENT_OK` は merge approval、risk approval、scope approval として扱われない。
- `AC_TEST_MAPPING` が埋まる。
- FAIL 時の戻し先 role が決まる。
- source mutation は QA role から行われない。

## 10. Phase 7: Repair Loop

目的:

- 要件が満たされるまで AI 側で修正する。

Command:

```bash
fda continue
```

処理:

- QA FAIL を読む。
- thread continuation が使える場合は同一 agent thread へ修正依頼する。
- 再テストする。
- 再レビューする。
- retry 上限超過で Human Decision に戻す。

受入条件:

- 同じ失敗分類の自動修正は上限がある。
- 修正履歴が receipt に残る。
- 失敗分類が残る。
- Human Decision へ戻す条件が明確である。

## 11. Phase 8: PR / Merge Gate

目的:

- 条件を満たした PR を merge する、または人間承認に回す。

Command:

```bash
fda merge
```

処理:

- Forge Gate 確認
- CI 確認
- QA receipts 確認
- risk 確認
- V1ではauto mergeせず、merge可能状態なら Human Decision

受入条件:

- low-risk PR でもV1ではauto mergeしない。
- privacy / security / legal / release 系は Human approval 必須。
- merge readiness と Human approval handoff が receipt に残る。
- 次 Planned PR へ進める。

## 12. Phase 9: Non-implementation Modes

目的:

- 実装できない依頼でも完了成果物を作る。

Mode:

- research
- uiux
- design-only

受入条件:

- 実装不可でも中途半端に止まらない。
- report / mock / design artifact が出る。
- Human Decision が必要な場合は明示される。
- Output Hub v0 が取り込める artifact inventory / hub-feed metadata が出る。実際の Output Hub 表示は Phase 11 で扱う。

## 13. Phase 10: Notifications

目的:

- 判断が必要なところで止まり、人間に知らせる。

通知優先度:

- P0: Slack Incoming Webhook
- P1: email SMTP deprecated/docs-only互換
- P2: Codex app notification / GitHub issue / ATO UI / Slack interactivity

成果物:

- `notification_request.json`
- `notification_receipt.json`
- `human_turn_notice.md`

受入条件:

- Human Decision が開いたら通知 request が出る。
- 通知には判断ID、要約、選択肢、期限、再開 command が含まれる。
- 通知失敗しても `fda status` で分かる。

## 14. Phase 11: Output Hub v0

目的:

- Markdown / JSON 成果物を見る手間を減らす。

Command:

```bash
fda open
```

出力:

- `output_hub.html`
- `decision_inbox.html`
- `execution_status.html`

受入条件:

- 要件定義書、設計書、PR 計画、レビュー結果がリンクで見える。
- Decision だけ一覧で見える。
- Task / Run / Agent 詳細は主表示しない。

## 15. PR 分割

| PR | 名前 | 範囲 |
|---|---|---|
| PR-V1-001 | FDA V1 CLI Roadmap & Product Contract | V1 docs 正本化 |
| PR-V1-002 | Intake command contract | `fda start` dry-run、requirements、decision |
| PR-V1-003 | Design command contract | `fda design`、design、planned PRs |
| PR-V1-004 | Implementation handoff and optional automation schemas | current Codex CLI handoff / receipt / thread state schemas。MCP schemaはV1.5 optional automationとして保持 |
| PR-V1-005 | Current Codex CLI implementation gate | `.fda/` Profile Gate、role switch、implementation handoff、target repo policy |
| PR-V1-006 | Current Codex CLI implementer receipt | target repo worktree、current Codex CLI実装、test、PR receipt |
| PR-V1-007 | Review agents | PR Reviewer / Functional QA / Security QA と Review Agent Gate receipt |
| PR-V1-008 | Repair loop | QA FAIL、missing proof、retry上限、repair receipt |
| PR-V1-009 | PR / Merge gate | Forge Gate、CI、QA、risk、merge / Human approval |
| PR-V1-010 | Non-implementation modes | research / uiux / design-only artifacts |
| PR-V1-011 | Notification and Output Hub v0 | Slack通知、通知receipt、Output Hub / Decision Inbox |

2026-06-29 のV1 pivot以降、既存PR番号と証跡は壊さず、新方針は `V1-PIVOT-*` または `PR-V1-019+` として追補する。

| Pivot PR | 範囲 |
|---|---|
| V1-PIVOT-001 | Codex CLI primary architecture 正本化 |
| V1-PIVOT-002 | 既存MCP primary文書、roadmap、PR sequence、operational epicをV1.5 optional automationへ再配置 |
| V1-PIVOT-003 | `.fda/` 7ファイル必須化とProfile Gateをschema / docs / examplesへ反映 |
| V1-PIVOT-004 | FDA run model、Output Hub、Decision Inbox、status artifact contract固定 |
| V1-PIVOT-005 | repo root command と target repo command の runtime Profile Gate 実装 |
| V1-PIVOT-006 | Current Codex CLI implementer handoff実装・文書化 |
| V1-PIVOT-007 | Codex subagent / read-only reviewer前提のReview Agent Gate更新 |
| V1-PIVOT-008 | Review Agent Gateのpacket projection生成 |
| V1-PIVOT-009 | PR review packetへの反映方針固定。V1では自動反映しない |
| V1-PIVOT-010 | `review_agent_gate_packet.md` があるrunではPR番号付きreview packetへ未反映のままmerge前gateを通さない |

詳細な順序、分割理由、V1到達条件との対応は `docs/v1/fda_v1_pr_sequence.md` を正本とする。
