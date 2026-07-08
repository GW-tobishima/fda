# FDA V1.5 MCP Agent Architecture

> 2026-06-29 update:
> この文書は FDA V1 の主経路ではなく、V1.5 以降の optional automation layer を定義する。
> V1 の正本は `docs/v1/codex_cli_primary_architecture.md` であり、V1主経路は `Human -> Codex CLI -> FDA Skill Pack / Work Protocol -> repo` である。
> Codex / Claude MCP direct implementer はV1 Done blockerではない。

## 1. 目的

FDA V1.5 optional automation は、設計完了後に MCP 経由で実装エージェントと QA エージェントを起動する。FDA はコードを直接書く主体ではなく、agent invocation、権限、handoff、receipt、gate を制御する orchestrator である。

V1では、人間が開いた現在のCodex CLIがprimary implementerである。MCPは、非対話batch、Web UI / API主導実行、複数repo同時処理、並列QA、別モデルレビューなどで使う。

## 2. 構成

```text
FDA Orchestrator
  |
  | mcp_agent_invocation_plan
  v
Implementer MCP Agent
  |
  | implementation_receipt / external_pr_receipt
  v
Functional QA MCP Agent
  |
  | functional_qa_receipt
  v
Security QA MCP Agent
  |
  | security_qa_receipt
  v
Forge Gate / GitHub PR / Merge Gate
```

FDA は、agent の raw output をそのまま成功扱いにしない。各 tool result は semantic receipt へ変換し、ATO / Forge / artifact に戻す。

## 3. MCP Server 方針

### Codex

V1.5 optional automation では Codex MCP を Implementer の候補にする。V1では current Codex CLI primary を使う。

確認済み前提:

- ローカル CLI は `codex mcp-server` を持つ。
- `codex mcp-server --help` は stdio MCP server として起動する command surface を示す。

V1 の実装では、tool 名を hardcode して成功扱いにしない。Phase 4 の dry-run で `tools/list` を実行し、expected tools が存在することを確認する。

### Claude Code

V1.5 optional automation では Claude Code MCP を Implementer または QA の候補にする。

Claude Code docs は、Claude Code が MCP server に接続できること、MCP server scope、plugin MCP tool naming、tool search、resources、channels などの運用要素を説明している。FDA はこの前提を、adapter capability として検出する。

ローカル環境で `claude` CLI が見つからない場合、Claude adapter は unavailable として扱い、Codex または他の adapter に fallback する。

## 4. Invocation Plan

`mcp_agent_invocation_plan.json` は最低限次を含む。

```json
{
  "plan_id": "mcp-plan-001",
  "task_key": "TASK-FDA-001",
  "planned_pr_id": "PR-FDA-001",
  "role": "implementer",
  "provider": "codex",
  "server_command": ["codex", "mcp-server"],
  "transport": "stdio",
  "cwd": "/path/to/target-repo",
  "handoff_path": "implementation_handoff.json",
  "prompt_path": "codex_prompt.md",
  "expected_tools": ["codex", "codex-reply"],
  "approval_policy": "on-request",
  "workspace_policy": "write",
  "allowed_paths": ["/path/to/target-repo"],
  "denied_actions": ["merge", "release", "scope_change_without_decision"],
  "timeout_seconds": 3600,
  "requires_unresolved_human_decisions": false
}
```

`expected_tools` は plan 上の期待値であり、実行時は `tools/list` の結果で検証する。存在しない場合は invocation failed として止める。

PR-V1-004 では、単一 role の例だけでなく、実行前の正本として次を schema 化する。

- `mcp_agent_invocation_plan.schema.json`
- `mcp_tool_call_receipt.schema.json`
- `coding_agent_thread_state.schema.json`
- `planned_pr_execution_packet.schema.json`
- `agent_role_policy.schema.json`

Schema Gate では、`mcp_agent_invocation_plan.json` に Implementer / Functional QA / Security QA の invocation がそれぞれ存在することを必須にする。Functional QA と Security QA は `workspace_policy=read_only`、`source_mutation_allowed=false` でなければならない。

`human_decision_guard` は invocation plan の先頭で評価する。未解決 Human Decision または `revise` のような非承認回答が残る場合、plan は `status=blocked` でなければならず、MCP agent を呼び出さない。

## 5. Role Policy

| Role | Provider 候補 | Workspace | できること | できないこと |
|---|---|---|---|---|
| Implementer | Codex / Claude | write | 実装、test、PR作成 | Human Decision 未解決の scope 変更 |
| PR Reviewer | Codex / Claude | read-only | correctness / regression / blast radius review | source mutation、Functional QA / Security QA の代替 |
| Functional QA | Claude / Codex | read-only | AC検証、再現、FAIL分類 | source mutation、security例外承認 |
| Security QA | Claude / Codex | read-only | security / privacy / auth / secret 検証 | Functional QA の代替、risk自己承認 |
| Forge / QAx2 Reviewer | Codex / Claude | read-only | ATO / Forge / FDA 証跡、handoff、review packet、human decision 境界の確認 | merge approval、PromotionDecision の自己承認 |
| Design QA | Codex / Claude | read-only | UI / visual / browser evidence の確認 | Functional QA の代替 |
| Merge Role | FDA / GitHub adapter | controlled write | policyに基づくmerge、approval handoff | human-only approval の自己承認 |

ATO / Forge / FDA を使う開発では、PR ready / merge 前に Review Agent Gate を必ず通す。
review packet の `REVIEW_AGENT_GATE` には、`pr_reviewer`、`functional_qa`、`security_qa`、`forge_reviewer` または `qax2` の evidence を残す。
UI / visual 変更がない場合でも、`design_qa` は `not_applicable` と理由を残す。
`REVIEW_AGENT_OK` は merge approval ではなく、HOLD / FAIL / pending / evidence 不足は AI repair、QA repair、または typed human decision に戻す。

## 6. Handoff

Implementer handoff は `implementation_handoff.json` を contract artifact とし、最低限次を含む。

- 目的
- target repo
- branch / worktree policy
- Scope In / Scope Out
- Acceptance Criteria
- Non-functional Requirements
- Human Decision resolved summary
- forbidden changes
- test command
- expected artifacts
- receipt output path
- PR body requirements

Markdown handoff は `implementation_handoff.json` から生成される人間向け view または prompt 補助として扱い、schema validation の正本にしない。

QA handoff は最低限次を含む。

- PR URL
- planned PR ID
- AC list
- expected test evidence
- changed files
- risk focus
- read-only policy
- receipt output path

## 7. Tool Call Receipt

`mcp_tool_call_receipt.json` は、tool 実行結果を gate 判定できる形へ変換する。

必須 field:

- `receipt_id`
- `provider`
- `server`
- `tool_name`
- `thread_id`, if available
- `role`
- `cwd`
- `input_artifacts`
- `output_artifacts`
- `started_at`
- `completed_at`
- `exit_status`
- `semantic_verdict`
- `scope_drift`
- `tests_run`
- `errors`
- `evidence_links`

`semantic_verdict` は `pass`, `fail`, `blocked`, `needs_human`, `adapter_unavailable` のいずれかにする。

raw tool result はそのまま gate 成功にしない。V1 では `tool_result_digest` と `semantic_result` を分け、stdout / stderr 全文ではなく、要約、scope drift、test evidence、gate effect、next action を receipt に残す。

## 8. Thread State

Repair loop では、可能な場合は同じ coding agent thread を継続する。

`coding_agent_thread_state.json` は次を持つ。

- `thread_id`
- `provider`
- `role`
- `planned_pr_id`
- `actual_pr_url`
- `last_prompt_id`
- `last_receipt_id`
- `repair_attempt_count`
- `failure_classification`
- `next_allowed_action`

thread continuation が使えない provider では、新規 invocation に過去 receipt と failure summary を渡す。

## 9. Dry-run Gate

`fda implement --dry-run` は target repo を変更しない。

必須確認:

- server command が存在する。
- server が起動できる。
- `tools/list` が成功する。
- expected tools が存在する。
- cwd が target repo である。
- prompt が artifact として存在する。
- approval policy が plan と一致する。
- denied actions が plan に含まれる。

失敗時は `dry_run_receipt.json` に `adapter_unavailable` または `fail` として残す。

Codex CLI `0.142.0` の `codex mcp-server` は stdio 上で JSON Lines の JSON-RPC message を返す。FDA V1 の dry-run adapter は、`initialize` と `tools/list` だけを送り、`codex` tool 自体は呼ばない。`CODEX_HOME` が未指定で既定 state DB が読み取り専用の場合は、tools/list 検出用に `/tmp` 配下の一時 `CODEX_HOME` を使ってよい。この一時 home は adapter capability 検出のためだけに使い、target repo には書き込まない。

## 10. Live Gate

`fda implement --live` は dry-run の pass を前提にする。

必須確認:

- Human Decision 未解決なし。
- Design Gate pass。
- worktree / branch 作成済み。
- implementation handoff 配置済み。
- Implementer receipt あり。
- test evidence あり。
- PR receipt あり。

V1.5 optional automation としてMCP live実行を使う場合は、`codex mcp-server` の `tools/list` を再確認したうえで `tools/call` の `codex` tool を呼ぶ。Codex tool には `prompt`、`cwd`、`approval-policy=on-request`、`sandbox=workspace-write` を渡す。V1主経路では `current_codex_cli_handoff.json` と `implementation_handoff.md` を使い、現在のCodex CLIが実装者へrole switchする。

Implementer の最終応答は次の marker を含む必要がある。

```text
FDA_ACTUAL_PR_URL: https://github.com/<owner>/<repo>/pull/<number>
FDA_TEST_STATUS: passed|failed|not_run
FDA_TESTS_RUN: <command summary>
FDA_CHANGED_FILES: <comma separated paths or NONE>
FDA_SCOPE_DRIFT: none|<summary>
```

FDA は raw output を成功扱いにせず、marker と tool result を次の artifact へ正規化する。

- `mcp_tool_call_receipt.json`
- `implementation_receipt.json`
- `external_pr_receipt.json`
- `coding_agent_thread_state.json`

`FDA_ACTUAL_PR_URL` が回収できない、test が `passed` でない、dry-run が `succeeded` でない、または Human Decision が未解決の場合は Development Gate を `blocked` または `failed` として止める。

## 11. QA Separation

Functional QA と Security QA は別 invocation と別 receipt にする。

Functional QA の focus:

- Acceptance Criteria
- Given / When / Then
- regression
- UX / workflow
- test evidence

Security QA の focus:

- secret exposure
- auth / permission
- privacy
- injection
- unsafe external content
- data retention
- legal / compliance flags

同じ provider または同じ人が両方を担当しても、出力 artifact は統合しない。

PR-V1-004 の `agent_role_policy.json` はこの境界を機械検証可能にする。QA role は read-only であり、source mutation、merge approval、risk self-approval は禁止事項として残す。

## 12. Failure Classification

| Failure | 戻し先 | Human Decision |
|---|---|---|
| test not run | AI repair | 不要 |
| missing proof | AI repair | 不要 |
| stale evidence | AI repair | 不要 |
| trace gap | AI repair | 不要 |
| AC未達 | Implementer repair | 不要 |
| scope conflict | Human Decision | 必要 |
| High / Critical security | Human Decision / block | 必要 |
| adapter unavailable | Orchestrator | 状況次第 |
| retry overrun | Human Decision | 必要 |

## 13. 参照

- OpenAI Codex docs: https://developers.openai.com/codex/
- Claude Code MCP docs: https://code.claude.com/docs/en/mcp
- 既存 runtime 境界: `docs/runtime/ai_delivery_runtime.md`
