---
artifact_type: human_decision_packet
version: v0
status: applied
created_at: 2026-06-29
task_key: fda-v1-codex-cli-primary-epic-draft-20260628
run_id: run_01KW89PZTZEMV79QA5BJ7QRJ1K
---

# Human Decision Packet: FDA V1 Codex CLI Primary Rebaseline

## 0. Metadata

- Decision Packet ID: `HDP-FDA-V1-CLI-PRIMARY-REBASIS`
- Program ID: `FDA-V1`
- Epic ID: `FDA-V1-CLI-PRIMARY-REBASIS`
- Status: `applied`
- Required Before: `V1-PIVOT-001` 実装着手前
- Owner Role: product owner

## 1. Decision Needed

FDA V1 の主経路を、Codex MCP serverを呼ぶ外部orchestrator型から、現在のCodex CLIを実装者として使うSkill Pack / Work Protocol型へ正式に切り替えるかを決める。

この判断に伴い、既存PR sequence、`.fda/` profile範囲、同一Codex CLIのrole boundary、MCPのV1扱い、UI scope、merge/autonomy policyも決める必要がある。

## 2. Trigger

実運用では人間がすでにCodex CLIを開き、そこでFDAに仕事を頼む。FDAがさらに内側で別のCodex MCP serverを呼ぶと、approval、権限、cwd、thread、sandbox、PR作成、statusが二重化しやすい。

既存文書には `docs/v1/mcp_agent_architecture.md` のように「V1ではCodex MCPをImplementerの第一候補にする」と明記された箇所があり、今回の方針と矛盾している。

## 3. Context

- Current state:
  - 既存V1 docsはCLI-firstを掲げつつ、実装者呼び出しはCodex MCP primaryとして設計されている。
  - Operational V1 Epicは `PR-V1-012` から `PR-V1-018` までを定義済み。
  - `.fda/` repository profileの現行必須は `repo.yaml`、`delivery_policy.yaml`、`skills.lock` の3ファイル。
- Relevant requirement:
  - V1では `Human -> Codex CLI -> FDA Skill Pack -> repo` を主経路にする。
  - MCPはV1.5以降のoptional automation layerへ退避する。
- Relevant evidence:
  - `docs/v1/mcp_agent_architecture.md`
  - `docs/v1/fda_v1_product_contract.md`
  - `docs/v1/fda_v1_roadmap.md`
  - `docs/v1/fda_v1_pr_sequence.md`
  - `docs/v1/fda_v1_operational_epic.md`
  - `docs/standards/fda-v1/repository_profile.md`

## 4. Decisions

### HDP-001: V1主経路をCodex CLI primaryへ正式変更するか

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | V1主経路を `Human -> Codex CLI -> FDA Skill Pack -> repo` に正式変更する | 実運用に合う。approval / cwd / sandbox / threadの二重化を避けられる。V1の学習・運用コストが下がる | 既存MCP primary文書、roadmap、PR sequenceの修正が必要 | recommended |
| B | MCP primaryを維持し、Codex CLI primaryは追加modeにする | 既存docsとの差分が小さい。外部orchestrator構想を早く残せる | V1主経路が二重化し、今回の課題が残る。人間がCodex CLIを開いている運用と噛み合いにくい | not recommended |

Required before: `V1-PIVOT-001`

Default if no decision: 現在のMCP primary文書を正本のまま維持し、このEpicはdraftに留める。

Recorded decision: A. V1主経路を `Human -> Codex CLI -> FDA Skill Pack -> repo` に正式変更する。

### HDP-002: 既存PR-V1-001..018との番号・履歴をどう扱うか

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | 新方針は `V1-PIVOT-*` または `PR-V1-019+` として追補し、既存番号は履歴として残す | 既存PRやreview packetとの参照が壊れにくい。移行差分を追いやすい | PR番号体系が少し長くなる | recommended |
| B | 既存 `PR-V1-004..018` をrenumber / supersedeし、V1計画を完全に再編する | 最終文書がきれいになる | 過去の証跡、review packet、handoff、ATO taskとの対応が崩れやすい | not recommended unless history cleanup is explicitly desired |

Required before: planned PRをGitHub Issue化またはPR化する前。

Default if no decision: `V1-PIVOT-*` のdraft IDを使い、既存番号は変更しない。

Recorded decision: A. 新方針は `V1-PIVOT-*` または `PR-V1-019+` として追補し、既存番号と証跡は維持する。

### HDP-003: `.fda/` repo profileのV1必須範囲

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | `repo.yaml`、`delivery_policy.yaml`、`skills.lock` に加え、`agent_roles.yaml`、`gates.yaml`、`artifact_map.yaml`、`notification.yaml` もV1のdelivery-ready profileに含める。FDA対象repoに `.fda/` が無い場合は、作業開始前に `.fda/` folder と必須profile作成を強制する | Codex CLI primary、subagent QA、Output Hub、通知までrepo-localに表現できる。repoごとの暗黙運用を減らせる | target repo導入時の初期設定が重くなる | selected |
| B | 既存どおり3ファイルだけをV1必須にし、残り4ファイルはV1.5 optionalにする | 導入が軽い。既存docsとの差分が小さい | agent role / gate / notificationが暗黙設定になりやすく、repoごとの運用差が残る | acceptable for minimal V1 |

Required before: repository profile schema更新前。

Default if no decision: 3ファイル必須を維持し、残り4ファイルはoptional proposalとして扱う。

Recorded decision: A. 7ファイル構成をV1必須にする。作業対象repoに `.fda/` が無い場合、FDA作業開始前に `.fda/` folder と必須profileを作成することを強制する。

### HDP-004: Current Codex CLIのrole boundary

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | 同じCodex CLIセッションが、FDA orchestratorとして設計・判断抽出を行った後、`implementation_handoff.md` に基づいてimplementer roleへ切り替えて実装してよい | ユーザーの実運用に最も合う。handoffがそのまま実装プロンプトになる。実装者MCPの二重化を避けられる | role switchの明示、checkpoint、review gateがないと責務が混ざる | recommended |
| B | FDA orchestratorはsource mutationせず、実装は別subagent / 別worktree / 別Codex sessionへ渡す | orchestrator / implementer境界が厳密。reviewしやすい | V1でまた実行主体が増え、今回避けたい二重化に近づく | not recommended for V1 primary |

Required before: implementer handoff contract実装前。

Default if no decision: source mutationを伴う実装は行わず、docs draftまでに止める。

Recorded decision: A. 同じCodex CLIセッションが、明示的なrole switch、ATO checkpoint、Review Agent Gateを条件にimplementerへ切り替えてよい。

### HDP-005: MCPのV1扱い

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | Codex / Claude MCP direct implementerはV1.5 optional automation layerへ退避し、V1 Done blockerから外す | V1主経路が単純になる。今の実運用と合う | 既存のMCP live / dry-run PR計画を再整理する必要がある | recommended |
| B | MCP dry-runだけをV1 gateに残し、MCP live implementerはV1.5へ送る | adapter capability検出の資産を残せる | V1主経路に不要なgateが残り、V1の完了条件がやや重くなる | acceptable transitional option |

Required before: roadmap / operational epic更新前。

Default if no decision: MCP primaryの既存docsは変更せず、新方針はdraft扱い。

Recorded decision: A. Codex / Claude MCP direct implementerはV1.5 optional automation layerへ退避し、V1 Done blockerから外す。

### HDP-006: UI scope

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | V1 UIはlocal HTMLのOutput Hub、Decision Inbox、status artifactに絞る | Codex CLI主入口と矛盾しない。成果物閲覧・判断俯瞰に集中できる | 常駐Web UIやAPI操作は後回しになる | recommended |
| B | Mission Control Web UIをV1に含める | 将来像に近い。非CLIユーザーにも見せやすい | V1の実装範囲が膨らみ、Codex CLI primaryの価値検証が遅れる | not recommended for V1 |

Required before: Output Hub / status contract更新前。

Default if no decision: local HTML / status artifactまでをV1として扱う。

Recorded decision: A. V1 UIはlocal HTMLのOutput Hub、Decision Inbox、status artifactに絞る。

### HDP-007: merge / autonomy policy

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | V1ではauto mergeなし。Current Codex CLIはPR作成・review repairまで進め、merge approvalは人間判断に戻す | risk境界が明確。V1導入時に安全 | 完全自動deliveryにはならない | recommended |
| B | low-risk auto mergeをV1に残す | 自動化の価値が出やすい | policy / evidence / Forge gate / human approval境界が重くなり、V1主経路変更と同時に扱うにはリスクが高い | not recommended for this pivot |

Required before: `delivery_policy.yaml` と merge gate更新前。

Default if no decision: `auto_merge_allowed: false` をV1既定にする。

Recorded decision: A. V1ではauto mergeなし。PR作成・review repairまでは進めるが、merge approvalは人間判断に戻す。

### HDP-008: Review Agent Gate packet update contract

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A | V1では `review_agent_gate_packet.md` projection生成まで。実PR packetへの反映は明示コマンドまたは人間確認後にする | PR証跡をFDAが暗黙に書き換えず、review packetの文脈や既存手書きevidenceを守れる | 反映忘れが起き得る。`--pr-number` gateは別途実行が必要 | recommended |
| B | `fda review --pr-number <n>` が `artifacts/review_packets/pr-<n>.md` の `REVIEW_AGENT_GATE` section を自動更新する | 手作業の反映漏れが減り、PR ready前gateを素早く通せる | PR番号誤指定や既存section上書きのリスクが高い。人間のreview packet編集と衝突し得る | not recommended for V1 |
| C | `fda review` は更新案diffだけを生成し、別コマンド `fda review-packet apply` で反映する | Aより反映漏れが少なく、Bより安全。diff確認後に適用できる | コマンドとconflict処理の仕様が増える | good V1.5 candidate |

Required before: PR番号付きreview packetへの自動または半自動反映実装前。

Default if no decision: A。V1ではprojection生成までに留める。

Recorded decision: A. V1では `review_agent_gate_packet.md` のprojection生成までを標準にし、実PR packetへの反映は明示コマンドまたは人間確認後にする。

Operational enforcement: `review_agent_gate_packet.md` が存在するrunでは、FDAは自動反映はしないが、`artifacts/review_packets/pr-<PR番号>.md` が未反映または `REVIEW_AGENT_GATE` 不足のまま `fda merge` に進むことをblockedにする。これは新しい人間判断ではなく、HDP-008 A方針の反映漏れ防止gateである。

## 5. Impact

- Scope:
  - 既存MCP primary計画の一部はV1.5へ移動する。
  - `.fda/` profile、run model、CLI contract、review gateに変更が入る。
- Security:
  - source mutation権限をCurrent Codex CLIに持たせる場合、role switchとReview Agent Gateの証跡が必須になる。
  - secret、privacy、security High/Criticalは引き続きhuman-only decision。
- Schedule:
  - V1-PIVOT PRが増えるが、MCP live実装の不確実性をV1 blockerから外せる。
- UX:
  - 人間はCodex CLIを主入口にし、Output Hub / Decision Inbox / emailで俯瞰する。
- Operations:
  - ATO checkpoint、review packet、handoff、receiptの整合が重要になる。

## 6. Requested Reply Format

回答済み。記録された回答は次の通り。

```text
HDP-001: A
HDP-002: A
HDP-003: A
HDP-004: A
HDP-005: A
HDP-006: A
HDP-007: A
HDP-008: A
補足: HDP-003 は必須化する。作業するときに、そのリポジトリに .fda が無い場合は .fda folder を作るように強制する。
```

## 7. Recorded Decision

- Decision: HDP-001からHDP-007はすべてAを採用する。HDP-003は、7ファイル構成の必須化に加えて、FDA対象repoに `.fda/` が無い場合は作業開始前に `.fda/` folder と必須profileを作成することを強制する。
- Additional Decision: HDP-008はAを採用する。V1では `review_agent_gate_packet.md` をPR番号付きreview packetへ自動反映せず、明示コマンドまたは人間確認後に反映する。
- Decided By: user
- Decided At: 2026-06-29
- ATO Decision ID: `dec_01KW8A04G58XFKREE60PJBESFM`
- ATO Decision ID for HDP-008: `dec_01KW8F0K245Q95BH2698NDDZVG`
- Rationale: V1の実運用では人間がすでにCodex CLIを開いているため、Codex MCP serverを内側で呼ぶ二重構造を避け、FDAをCodex CLI内のSkill Pack / Work Protocolとして正本化する。

## 8. Forge Mapping

- Claim IDs:
  - `CLM-CLI-001`
  - `CLM-CLI-002`
  - `CLM-CLI-003`
  - `CLM-CLI-004`
  - `CLM-CLI-005`
  - `CLM-CLI-006`
  - `CLM-CLI-007`
- Proof Obligations:
  - architecture doc
  - updated roadmap / PR sequence
  - `.fda/` schema and examples
  - run model artifacts
  - review packet / Review Agent Gate evidence
- Human Decision Points:
  - `HDP-001` through `HDP-008`
- ATO Task Graph:
  - `fda-v1-codex-cli-primary-epic-draft-20260628`
- Planned PRs:
  - `V1-PIVOT-001` through `V1-PIVOT-010`
- Gate Requirements:
  - Human Decision Gate
  - Documentation Consistency Gate
  - Schema Gate
  - Review Agent Gate
