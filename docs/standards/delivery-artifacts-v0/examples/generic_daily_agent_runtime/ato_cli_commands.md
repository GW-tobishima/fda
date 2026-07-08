---
artifact_type: ato_cli_commands
version: v0
status: dry-run
---

# PoC-5B ATO CLI予定コマンド

この文書は、Generic Daily Agent Runtime のPlanning成果物をATO CLIへmaterializeする場合の予定コマンド列である。今回のPRでは実行しない。実行する場合は、人間確認後にrun id placeholderを直前のCLI出力で置換する。

## 前提

- transportはMCPではなく `ato_cli`。
- Program/Epic/Case/Taskの登録は `ato work begin` を予定する。
- summaryとartifact evidenceは `ato work checkpoint` または `ato evidence attach` を予定する。
- Human Decision Packetは `ato work block` のtyped human turn payloadへ変換する。
- AI Repair laneは空。将来、validation failureや不足証跡が出た場合だけrepair payloadを作る。

## コマンド列

### 1. Program

```bash
ato work begin --task PROGRAM-GDAR-001 --agent-id ato-cli-materializer --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key PROGRAM-GDAR-001 --json
```

```bash
ato work checkpoint --task PROGRAM-GDAR-001 --run-id <RUN_ID_FROM_PROGRAM_GDAR_001> --kind summary --summary "Generic Daily Agent Runtime program materialization preview. See ato_summary_preview.md" --evidence-surface ato_summary_preview --evidence-id docs/standards/delivery-artifacts-v0/examples/generic_daily_agent_runtime/ato_summary_preview.md --evidence-verdict pass --validation-status passed --freshness current --durability-class artifact --trust-level self_reported --json
```

### 2. Epic

```bash
ato work begin --task EPIC-GDAR-001 --agent-id ato-cli-materializer --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key EPIC-GDAR-001 --json
```

```bash
ato evidence attach --task EPIC-GDAR-001 --run-id <RUN_ID_FROM_EPIC_GDAR_001> --surface artifact --path docs/standards/delivery-artifacts-v0/examples/generic_daily_agent_runtime/ato_cli_materialization_plan.json --relation planning_projection --verdict pass --json
```

### 3. Cases

```bash
ato work begin --task CASE-GDAR-001 --agent-id ato-cli-materializer --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key CASE-GDAR-001 --json
ato work begin --task CASE-GDAR-002 --agent-id ato-cli-materializer --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key CASE-GDAR-002 --json
ato work begin --task CASE-GDAR-003 --agent-id ato-cli-materializer --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key CASE-GDAR-003 --json
```

### 4. Tasks

```bash
ato work begin --task TASK-GDAR-001 --agent-id ato-cli-materializer --role implementer --capability-profile planning --workspace-policy read_only --idempotency-key TASK-GDAR-001 --json
ato work begin --task TASK-GDAR-002 --agent-id ato-cli-materializer --role implementer --capability-profile planning --workspace-policy read_only --idempotency-key TASK-GDAR-002 --json
ato work begin --task TASK-GDAR-003 --agent-id ato-cli-materializer --role implementer --capability-profile planning --workspace-policy read_only --idempotency-key TASK-GDAR-003 --json
ato work begin --task TASK-GDAR-004 --agent-id ato-cli-materializer --role implementer --capability-profile planning --workspace-policy read_only --idempotency-key TASK-GDAR-004 --json
ato work begin --task TASK-GDAR-005 --agent-id ato-cli-materializer --role implementer --capability-profile planning --workspace-policy read_only --idempotency-key TASK-GDAR-005 --json
ato work begin --task TASK-GDAR-006 --agent-id ato-cli-materializer --role implementer --capability-profile planning --workspace-policy read_only --idempotency-key TASK-GDAR-006 --json
```

### 5. Human Decisions

```bash
ato work block --task TASK-GDAR-003 --run-id <RUN_ID_FROM_TASK_GDAR_003> --reason spec_decision --title "HD-GDAR-001: 汎用runtime化の切り出し粒度" --question "Generic runtimeをAICX runnerからどこまで切り出すか" --option "A: run_state/receiptだけ汎用化する" --option "B: command boundaryまで汎用化する" --option "C: Slack adapterまで汎用化する" --json
```

```bash
ato work block --task TASK-GDAR-005 --run-id <RUN_ID_FROM_TASK_GDAR_005> --reason spec_decision --title "HD-GDAR-002: AICX Study Bot互換性の優先度" --question "AICX Study Bot互換性の優先度をどうするか" --option "A: 既存CLI完全互換" --option "B: breaking change許容" --option "C: adapter移行後に旧コマンドdeprecated" --json
```

```bash
ato work block --task TASK-GDAR-006 --run-id <RUN_ID_FROM_TASK_GDAR_006> --reason merge_approval --title "HD-GDAR-003: 自動マージ許可範囲" --question "runtime codeをどこまで自動マージ候補にするか" --option "A: docs/schema/testのみ自動候補" --option "B: runtime codeは人間確認" --option "C: すべて人間確認" --json
```

### 6. Readiness Preview

```bash
ato task readiness --task EPIC-GDAR-001 --json
```

```bash
ato trace task EPIC-GDAR-001 --json
```

## 実行しないこと

- このPRで上記コマンドを実行しない。
- ATO MCP writeを使わない。
- ATO DBを直接変更しない。
- runtime codeを変更しない。
- Forge Gateを実評価しない。
- 自動マージしない。
