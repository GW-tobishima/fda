---
artifact_type: epic
version: v0
status: draft_decisions_applied
created_at: 2026-06-29
task_key: fda-v1-codex-cli-primary-epic-draft-20260628
run_id: run_01KW89PZTZEMV79QA5BJ7QRJ1K
---

# Epic: FDA V1 Codex CLI Primary Rebaseline

## 0. Metadata

- Epic ID: `FDA-V1-CLI-PRIMARY-REBASIS`
- Program ID: `FDA-V1`
- Status: `draft_decisions_applied`
- Owner: FDA orchestrator / product owner
- Related Requirement: 2026-06-29 user memo "V1ではCodex CLIを実装者そのものとして使い、FDAはCodex CLI内のSkill Pack / Work Protocolにする"
- Primary Artifact:
  - `artifacts/runs/fda-v1-codex-cli-primary-epic-20260629/epic_delivery_plan.md`
  - `artifacts/runs/fda-v1-codex-cli-primary-epic-20260629/human_decision_packet.md`

## 1. Outcome

FDA V1 の主経路を、従来の「FDAがCodex MCP serverを呼ぶ実装者オーケストレーター」から、「人間が開いた現在のCodex CLIを実装者・オーケストレーターとして使い、FDAはCodex CLI内で動くDelivery Skill Pack / Work Protocolになる」方針へ再定義する。

V1 主経路:

```text
Human -> Codex CLI -> FDA Skill Pack / Work Protocol -> current repo / target repo
```

V1.5 以降の optional automation layer:

```text
FDA Orchestrator -> Codex MCP / Claude MCP -> target repo
```

### User / Business Outcome

- 人間がすでに立ち上げているCodex CLIをそのまま実装主体にできる。
- approval、cwd、sandbox、thread、status、PR作成の二重化を避ける。
- FDAは要件定義、設計、判断抽出、handoff、review gate、receipt、Output Hubを担う作業OSとして振る舞う。
- MCPは捨てず、並列QA、非対話batch、Web UI / API主導実行、複数repo同時処理の将来レイヤーへ退避する。

### Success Metrics

- `docs/v1/*` と `docs/standards/fda-v1/*` の V1 定義から、MCP implementer primary と Codex CLI primary の矛盾がなくなる。
- `.fda/` repo profile で、current Codex CLI、subagent QA、human decision、gates、notification、artifact map を表現できる。
- FDA対象repoで作業を始めるとき、`.fda/` が無い場合は作業前に `.fda/` profileを作成することが必須になる。
- `fda start` / `fda design` / `fda open` / `fda status` が Codex CLI primary の成果物と再開導線を返せる。
- `implementation_handoff.md` を生成し、同じCodex CLIセッションが実装へ進める。
- Review Agent Gate は read-only reviewer として分離され、`pr_reviewer`、`functional_qa`、`security_qa`、必要時 `forge_reviewer` または `qax2`、UI該当時 `design_qa` の証跡を残す。
- MCP direct implementer は V1 Done blocker ではなく、V1.5 optional automation として明文化される。

### Non-goals

- V1でWeb UI / background orchestratorを主入口にすること。
- V1でCodex MCP live implementerを主経路または必須Done条件にすること。
- ATO / Forgeを実行ランタイムにすること。
- Human Decision、merge approval、risk approval、scope approvalをFDAが自己承認すること。
- stdout / stderr全文、AI会話全文、target repoのソース複製をFDA成果物やATO DBへ保存すること。

## 2. Scope

### In

- Codex CLI primary architecture の正本化。
- 既存 `mcp_agent_architecture.md`、`fda_v1_product_contract.md`、`fda_v1_roadmap.md`、`fda_v1_pr_sequence.md`、`fda_v1_operational_epic.md` との矛盾解消。
- `.fda/` repo profile の拡張方針:
  - `repo.yaml`
  - `delivery_policy.yaml`
  - `skills.lock`
  - `agent_roles.yaml`
  - `gates.yaml`
  - `artifact_map.yaml`
  - `notification.yaml`
- FDA対象repoに `.fda/` が無い場合、FDA作業開始前に `.fda/` folder と必須profileを作成する強制ルール。
- FDA run model の固定:
  - `requirements_definition.md`
  - `human_decision_packet.md`
  - `basic_design.md`
  - `detailed_design.md`
  - `planned_prs.json`
  - `implementation_handoff.md`
  - `review_receipt.json`
  - `merge_receipt.json`
  - `output_hub.html`
  - `status.json`
- `fda start` / `fda design` / `fda open` / `fda status` のCodex CLI primary contract。
- Current Codex CLI implementer handoff。
- Codex subagent / read-only reviewer を使う Review Agent Gate。
- MCP optional automation layer の位置づけ。

### Out

- 実装者をMCP serverとして直接呼ぶlive executionをV1主経路に戻すこと。
- Claude Code MCP reviewなどの外部agent orchestrationをV1の必須条件にすること。
- Mission Control Web UIの本格実装。
- production deploy、自動release、自動mergeの承認。

## 3. Epic ClaimContract

| Claim ID | Type | Statement | Blocking | Proof |
|---|---|---|---|---|
| CLM-CLI-001 | product | FDA V1の主経路は `Human -> Codex CLI -> FDA Skill Pack -> repo` である。 | yes | `docs/v1/codex_cli_primary_architecture.md` と更新済み product contract |
| CLM-CLI-002 | architecture | Current Codex CLI は V1 の primary implementer として扱える。 | yes | implementer handoff contract / role policy |
| CLM-CLI-003 | architecture | MCP direct implementer は V1.5 optional automation layer であり、V1 Done blockerではない。 | yes | roadmap / PR sequence の更新 |
| CLM-CLI-004 | config | repo-local `.fda/` profile はCodex CLI primary、subagent QA、gates、notificationを表現できる。 | yes | schema / examples / validation |
| CLM-CLI-005 | artifact | FDA run model は判断、設計、planned PR、handoff、review、merge、Output Hub、statusを再開可能なartifactとして残す。 | yes | run model schema / example run |
| CLM-CLI-006 | governance | Human Decision と AI repair は混ぜず、approval / scope / security / privacy / release は人間判断に戻す。 | yes | human decision packet / delivery policy |
| CLM-CLI-007 | review | ATO / Forge / FDA開発ではReview Agent Gateを必須証跡にする。 | yes | review packet template / check script pass |

## 4. Case Graph

| Case ID | Purpose | Depends On | Claims | Risk |
|---|---|---|---|---|
| CASE-CLI-001 | Codex CLI primary architecture を正本化する | human decision HDP-001 | CLM-CLI-001, CLM-CLI-003 | high |
| CASE-CLI-002 | 既存MCP primary文書とPR sequenceを再整理する | CASE-CLI-001, HDP-002 | CLM-CLI-003 | high |
| CASE-CLI-003 | `.fda/` repo profile spec を拡張する | HDP-003 | CLM-CLI-004 | medium |
| CASE-CLI-004 | FDA run model と Output Hub / Decision Inbox を固定する | CASE-CLI-001 | CLM-CLI-005 | medium |
| CASE-CLI-005 | `fda start` / `design` / `open` / `status` のCLI contractをCodex CLI primaryへ寄せる | CASE-CLI-004 | CLM-CLI-001, CLM-CLI-005 | medium |
| CASE-CLI-006 | Current Codex CLI implementer handoffを定義する | HDP-004, CASE-CLI-005 | CLM-CLI-002, CLM-CLI-006 | high |
| CASE-CLI-007 | Review Agent GateをCodex subagent / read-only reviewer方針へ合わせる | CASE-CLI-006 | CLM-CLI-007 | medium |
| CASE-CLI-008 | MCP optional automation layer のV1.5境界を明文化する | HDP-005, CASE-CLI-002 | CLM-CLI-003 | medium |

## 5. Planned PRs

既存の `PR-V1-001` から `PR-V1-018` と衝突しないよう、draftでは `V1-PIVOT-*` IDを使う。正式なPR番号体系は HDP-002 で決める。

| Planned PR | Case | Purpose | Risk | Auto-merge Allowed |
|---|---|---|---|---|
| V1-PIVOT-001 | CASE-CLI-001 | `docs/v1/codex_cli_primary_architecture.md` を追加し、V1主経路をCodex CLI primaryとして正本化する | high | no |
| V1-PIVOT-002 | CASE-CLI-002, CASE-CLI-008 | 既存MCP primary文書、roadmap、PR sequence、operational epicをV1.5 optional automationへ再配置する | high | no |
| V1-PIVOT-003 | CASE-CLI-003 | `.fda/` repo profile spec / schema / examples をCodex CLI primary対応へ拡張する | medium | no |
| V1-PIVOT-004 | CASE-CLI-004 | FDA run model、Output Hub、Decision Inbox、status artifact contractを固定する | medium | no |
| V1-PIVOT-005 | CASE-CLI-005 | `fda start` / `fda design` / `fda open` / `fda status` のCLI contractを更新する | medium | no |
| V1-PIVOT-006 | CASE-CLI-006 | Current Codex CLI implementer handoffを実装・文書化する | high | no |
| V1-PIVOT-007 | CASE-CLI-007 | Codex subagent / read-only reviewer前提のReview Agent Gateを更新する | medium | no |
| V1-PIVOT-008 | CASE-CLI-007 | `review_agent_gate.json` から `check_review_agent_gate.py --packet-path` に通る `review_agent_gate_packet.md` を生成する | medium | no |
| V1-PIVOT-009 | CASE-CLI-007 | 実PRの `artifacts/review_packets/pr-<PR番号>.md` への反映方針を固定する。V1では自動反映せず、projection生成までを標準にする | medium | no |
| V1-PIVOT-010 | CASE-CLI-007 | `review_agent_gate_packet.md` が存在するrunでは、実PRの `artifacts/review_packets/pr-<PR番号>.md` へ未反映のまま `fda merge` に進めないgateを追加する | medium | no |

## 6. Human Decision Points

詳細は `human_decision_packet.md` を正とする。

| ID | Trigger | Decision | Impact |
|---|---|---|---|
| HDP-001 | V1主経路の再定義 | A: Codex CLI primaryへ正式変更 | `Human -> Codex CLI -> FDA Skill Pack -> repo` をV1主経路にする |
| HDP-002 | 既存PR-V1-001..018との番号・履歴衝突 | A: `V1-PIVOT-*` として追補 | 既存PR番号と証跡は壊さず、新方針を追補する |
| HDP-003 | `.fda/` profileのV1必須範囲 | A: 7ファイル構成を必須化し、`.fda/` が無いrepoでは作業前に作成を強制 | repository profile gateをV1-PIVOT-003の必須要件にする |
| HDP-004 | Current Codex CLIのrole boundary | A: 同一Codex CLIがorchestratorからimplementerへrole switch可能 | role switch、checkpoint、review gateの証跡を必須にする |
| HDP-005 | MCPのV1扱い | A: V1.5 optional automationへ退避 | MCP direct implementerをV1 Done blockerから外す |
| HDP-006 | UI scope | A: local HTML / Decision Inbox / statusをV1範囲にする | Mission Control本格Web UIはV1後へ送る |
| HDP-007 | merge/autonomy policy | A: V1ではauto mergeなし | merge approvalは人間判断に戻す |
| HDP-008 | Review Agent Gate packetのPR packet反映 | A: V1では `review_agent_gate_packet.md` projection生成まで。実PR packetへの反映は明示コマンドまたは人間確認後にする | PR review packetをFDAが暗黙に書き換えない。`--pr-number` 自動更新はV1.5以降または別Decisionに送る |

## 7. Release Strategy

- Release target: FDA V1 rebaseline docs + schema + CLI contract PRs
- Feature flag: none for docs; future CLI behavior may use explicit `--codex-cli-primary` during transition
- Smoke test:
  - schema validation
  - `python3 scripts/check_review_agent_gate.py --packet-path <review_agent_gate_packet.md>` for FDA-generated projection
  - `python3 scripts/check_review_agent_gate.py --pr-number <PR番号>` after explicit PR review packet reflection
  - `fda merge` blocks when `review_agent_gate_packet.md` exists but `artifacts/review_packets/pr-<PR番号>.md` is missing or lacks `REVIEW_AGENT_GATE`
  - architecture boundary checks when Rust code changes
  - Output Hub / status artifact generation when CLI changes
- Rollback:
  - keep existing MCP docs as deprecated / V1.5 candidate instead of deleting in the first PR
  - avoid renumbering existing PR history unless HDP-002 explicitly chooses it

## 8. Forge Mapping

- Claim IDs:
  - `CLM-CLI-001` through `CLM-CLI-007`
- Proof Obligations:
  - architecture doc exists
  - conflicting docs updated
  - `.fda/` profile schema validates
  - run model example validates
  - Review Agent Gate packet exists
  - Human Decisions resolved before implementation beyond docs draft
- Human Decision Points:
  - `HDP-001` through `HDP-008`
- ATO Task Graph:
  - current task: `fda-v1-codex-cli-primary-epic-draft-20260628`
  - future implementation tasks should be created after HDP resolution
- Planned PRs:
  - `V1-PIVOT-001` through `V1-PIVOT-010`
- Gate Requirements:
  - Human Decision Gate
  - Documentation Consistency Gate
  - Schema Gate
  - Review Agent Gate
  - Output Hub Gate

## 9. Acceptance Criteria

- [x] Human Decision Packet が作成され、HDP-001からHDP-007が分離されている。
- [x] HDP-001からHDP-007の人間回答がATO decision `dec_01KW8A04G58XFKREE60PJBESFM` に適用されている。
- [ ] V1主経路がCodex CLI primaryであることを示すarchitecture doc案が作れる。
- [ ] MCP direct implementerの扱いがV1 / V1.5で明確に分かれる。
- [ ] 既存 `docs/v1/mcp_agent_architecture.md` の主張と新方針の衝突を解消するPR境界がある。
- [x] `.fda/` repo profileの必須・optional範囲が人間判断として切り出され、7ファイル必須化と未作成repoでの作成強制が決定されている。
- [x] planned PRが既存PR番号と衝突しないdraft IDで定義されている。
- [x] Review Agent Gateの必須reviewerとnot-applicable規則が維持されている。
- [x] `review_agent_gate_packet.md` は生成されるが、実PR packetへの自動反映はV1標準にしないことが明文化されている。
- [x] `review_agent_gate_packet.md` があるrunでは、PR番号付きreview packetへ未反映のまま `fda merge` に進めない。

## 10. Recommended Next Action

1. `V1-PIVOT-010` の検証を完了し、Review Agent Gate packet反映漏れがmerge前にblockedになる証跡を残す。
2. Operational V1の残作業は `PR-V1-012` 以降のemail live、status、merge execution、fixture-free current Codex CLI evidence、ATO adapter、Forge adapter、E2E proofへ進める。
3. 実PRを作る場合は、PR番号確定後に `review_agent_gate_packet.md` を `artifacts/review_packets/pr-<PR番号>.md` へ明示反映し、`python3 scripts/check_review_agent_gate.py --pr-number <PR番号>` を通す。
