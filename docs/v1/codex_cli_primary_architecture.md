# FDA V1 Codex CLI Primary Architecture

## 1. 目的

この文書は、FDA V1 の主経路を Codex CLI primary として定義する正本文書である。

FDA V1 は、FDA が内側から Codex MCP server を呼んで実装者にする構成を主経路にしない。V1 では、人間が開いた現在の Codex CLI を実装者およびオーケストレーターとして使い、FDA はその Codex CLI 内で動く Delivery Skill Pack / Work Protocol として振る舞う。

V1 主経路:

```text
Human
  -> Codex CLI
  -> FDA Skill Pack / Work Protocol
  -> current repo / target repo
```

V1.5 以降の optional automation layer:

```text
FDA Orchestrator
  -> Codex MCP / Claude MCP
  -> target repo
```

## 2. 判断済み方針

2026-06-29 の Human Decision `dec_01KW8A04G58XFKREE60PJBESFM` により、次を採用済みとする。

| ID | Decision |
|---|---|
| HDP-001 | FDA V1 の主経路を Codex CLI primary に正式変更する。 |
| HDP-002 | 新方針は `V1-PIVOT-*` または `PR-V1-019+` として追補し、既存PR番号と証跡は壊さない。 |
| HDP-003 | `.fda/` は7ファイル構成をV1必須にする。作業対象repoに `.fda/` が無い場合、FDA作業開始前に `.fda/` folder と必須profileを作成する。 |
| HDP-004 | 同じCodex CLIセッションが、明示的なrole switch、ATO checkpoint、Review Agent Gateを条件にimplementerへ切り替えてよい。 |
| HDP-005 | Codex / Claude MCP direct implementerはV1.5 optional automation layerへ退避し、V1 Done blockerから外す。 |
| HDP-006 | V1 UIはlocal HTMLのOutput Hub、Decision Inbox、status artifactに絞る。 |
| HDP-007 | V1ではauto mergeなし。PR作成・review repairまでは進めるが、merge approvalは人間判断に戻す。 |

## 3. V1 の責務境界

| 領域 | V1 の扱い |
|---|---|
| Codex CLI | 実行主体。要件整理、設計、実装、repair、PR作成、レビュー依頼を現在のセッションで進める。 |
| FDA | Codex CLI内で動く作業OS。要件、設計、判断、handoff、review gate、receipt、Output Hub、statusを標準化する。 |
| ATO | task / run / checkpoint / decision / evidence のcontrol plane。実行runtimeではない。 |
| Forge | Claim / Proof / PromotionDecision のgate projection。merge approvalではない。 |
| `.fda/` | repo-localなFDA profile正本。無いrepoではFDA作業開始前に作成する。 |
| MCP | V1.5以降の外部自動化・並列化・非対話実行のtransport。V1主経路ではない。 |

## 4. `.fda/` Profile Gate

FDA対象repoで作業を始める前に、次の7ファイルが存在することを確認する。

```text
.fda/
  repo.yaml
  delivery_policy.yaml
  skills.lock
  agent_roles.yaml
  gates.yaml
  artifact_map.yaml
  notification.yaml
```

存在しない場合、FDAは実装、設計、レビュー、PR作成へ進む前に `.fda/` を作成する。作成できない場合は `spec_decision` または `blocker_confirmation` として人間判断へ戻す。

V1-PIVOT-005 では、repo root command と target repo command の両方で runtime Profile Gate を実行する。repo root command は `start`、`decide`、`design`、`plan`、`open`、`status`、`notify test` である。target repo command は `implement --dry-run`、`implement --live`、`review`、`continue`、`merge` である。

Profile Gate は不足している必須 profile だけを作成し、既存 `.fda/*` は上書きしない。target repo path 自体が存在しない場合は偽repoを作らず、既存の target repo missing gate に戻す。`validate-artifacts` は自動生成せず、不足 profile を validation failure として検出する。

最小解釈:

- `repo.yaml`: repo識別子、stack、標準commands、docs location。
- `delivery_policy.yaml`: 自律実行上限、人間判断条件、禁止action。
- `skills.lock`: 使用するFDA skill version。
- `agent_roles.yaml`: current Codex CLI、subagent QA、reviewer、merge roleの権限。
- `gates.yaml`: Human Decision、Design、Review Agent、Merge、Profile gate。
- `artifact_map.yaml`: run artifact、review packet、handoff、evidence location。
- `notification.yaml`: CLI、Output Hub、emailなどのHuman Turn通知方針。

## 5. Role Model

V1では current Codex CLI が primary implementer である。ただし、role boundaryを曖昧にしない。

| Role | Executor | Workspace | 主な責務 | 禁止事項 |
|---|---|---|---|---|
| Orchestrator | current Codex CLI | write | intake、設計、判断抽出、ATO checkpoint、gate管理 | human-only decisionの自己承認 |
| Implementer | current Codex CLI | write | approved scope内の実装、test、PR作成 | 未解決Human Decisionを実装で埋める |
| PR Reviewer | Codex subagentなど | read-only | correctness / regression / blast radius review | source mutation、merge approval |
| Functional QA | Codex subagentなど | read-only | AC検証、再現、FAIL分類 | source mutation、security例外承認 |
| Security QA | Codex subagentなど | read-only | security / privacy / auth / secret検証 | Functional QAの代替、risk自己承認 |
| Forge / QAx2 Reviewer | Codex subagentなど | read-only | ATO / Forge / FDA証跡とhuman decision境界確認 | PromotionDecisionやmerge approvalの自己承認 |
| Design QA | Codex subagentなど | read-only | UI / visual / browser surface確認 | Functional QAの代替 |
| Merge Manager | current Codex CLI + human | controlled | merge readiness整理、人間承認handoff | human merge approvalの自己承認 |

OrchestratorからImplementerへ切り替える場合は、最低限次を残す。

- ATO checkpoint
- `current_codex_cli_handoff.json`
- `implementation_handoff.md`
- Scope In / Scope Out
- Human Decision resolved summary
- forbidden changes
- test command
- expected artifacts

## 6. V1 Workflow

```text
1. Profile Gate
   .fda/ 7ファイルを確認する。無ければ作る。

2. Intake
   人間の依頼から requirements_definition と human_decision_packet を作る。

3. Human Decision Gate
   scope / privacy / security / legal / release / merge などの人間判断を分離する。

4. Design
   basic_design、detailed_design、planned_prs、handoff材料を作る。

5. Implementer Role Switch
   current Codex CLI が current_codex_cli_handoff.json と implementation_handoff.md に基づいて approved scope を実装する。

6. Review Agent Gate
   `pr_reviewer_receipt.json`、`functional_qa_receipt.json`、`security_qa_receipt.json`、`review_agent_gate.json`、`review_agent_gate_packet.md` を生成し、pr_reviewer、functional_qa、security_qaをread-onlyで必須実行する。
   ATO / Forge / FDA証跡やhuman decision境界に触れる場合は forge_reviewer または qax2 を実行する。
   UI / frontend / browser surface に触れる場合は design_qa を実行し、該当しない場合も not_applicable 理由を残す。

7. Repair
   missing proof、stale evidence、test not run、trace gap、review packet missing、schema validation failure はAI repairへ戻す。

8. PR / Merge Handoff
   V1はauto mergeしない。merge approvalは人間判断へ戻す。

9. Output Hub / Status
   local HTML、Decision Inbox、status artifactで成果物と判断待ちを見せる。
```

## 7. MCP の位置づけ

MCP direct implementer は V1 の主経路ではない。次の用途は V1.5 以降で扱う。

- Codex CLI外からの非対話batch実行
- Web UI / API主導実行
- 複数repo同時処理
- 並列QA
- 別モデルレビュー
- 人間がCodex CLIを開いていないときのbackground execution

既存の MCP schema、dry-run、tool receipt の資産は破棄しない。ただし、V1 Done blockerではなく optional automation contract として管理する。

## 8. Output Hub と UI

V1 UI は大きな常駐Web UIを主入口にしない。Codex CLIを主入口とし、UIは成果物閲覧と判断Inboxに集中する。

最小生成物:

```text
.fda/runs/<run_id>/output_hub.html
.fda/runs/<run_id>/decision_inbox.html
.fda/runs/<run_id>/status.html
```

または既存artifact rootに合わせて次を使う。

```text
artifacts/runs/<run_id>/output_hub.html
artifacts/runs/<run_id>/decision_inbox.html
artifacts/runs/<run_id>/status.json
```

Output Hub は正本ではない。ATO / Forge / GitHub / FDA artifactsへのprojectionとして扱う。

## 9. V1 Done Definition

FDA V1は次を満たしたときにV1と呼ぶ。

1. `.fda/` 7ファイルprofileが存在し、無いrepoでは作業前に作成される。
2. 自然言語またはrequirementsから要件定義とHuman Decision Packetを作れる。
3. Human Decisionが未解決なら実装やmergeへ進まない。
4. Codex CLI primaryのrole switchとimplementation handoffが残る。
5. current Codex CLIがapproved scope内で実装・test・PR作成まで進められる。
6. Review Agent Gateが `pr_reviewer`、`functional_qa`、`security_qa` のread-only reviewerで通り、`review_agent_gate.json` と `review_agent_gate_packet.md` に残る。
7. missing proof、test not run、trace gapなどはHuman DecisionではなくAI repairへ戻る。
8. V1ではauto mergeせず、merge approvalは人間判断へ戻る。
9. Output Hub / Decision Inbox / status artifactで成果物と判断待ちを確認できる。
10. MCP direct implementerはV1.5 optional automationとして文書化され、V1主経路と矛盾しない。

## 10. 後続PR境界

| Planned PR | 目的 |
|---|---|
| V1-PIVOT-001 | このCodex CLI primary architectureを正本化する。 |
| V1-PIVOT-002 | 既存MCP primary文書、roadmap、PR sequence、operational epicをV1.5 optional automationへ再配置する。 |
| V1-PIVOT-003 | `.fda/` 7ファイル必須化とProfile Gateをschema / docs / examplesに反映する。 |
| V1-PIVOT-004 | FDA run model、Output Hub、Decision Inbox、status artifact contractを固定する。 |
| V1-PIVOT-005 | repo root command と target repo command の runtime Profile Gate を実装する。 |
| V1-PIVOT-006 | Current Codex CLI implementer handoffを実装・文書化する。 |
| V1-PIVOT-007 | Codex subagent / read-only reviewer前提のReview Agent Gateを更新する。 |
| V1-PIVOT-008 | Review Agent Gateの `review_agent_gate_packet.md` projectionを生成し、checkerに通す。 |
| V1-PIVOT-009 | PR番号付きreview packetへの反映方針を固定する。V1では自動反映せず、明示コマンドまたは人間確認後に反映する。 |
| V1-PIVOT-010 | `review_agent_gate_packet.md` があるrunでは、PR番号付きreview packetへ未反映のまま `fda merge` に進めないgateを追加する。 |
