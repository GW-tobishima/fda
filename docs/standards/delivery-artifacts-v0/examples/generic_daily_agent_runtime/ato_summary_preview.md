---
artifact_type: ato_summary_preview
version: v0
status: draft
---

# PoC-5B ATO CLI Materialization Summary Preview

## 目的

Generic Daily Agent Runtime の planning artifact を、ATO CLI で実登録するとした場合の Program / Epic / Case / Task / Human Decision / AI Repair 投影を確認する。

このpreviewは dry-run であり、`ato_cli_materialization_plan.json` と `ato_cli_commands.md` に予定コマンド列だけを残す。対象Program/Epic/Case/TaskのATO CLI実行、ATO MCP write、ATO DB直接変更、Forge Gate実評価、runtime実装PR作成、自動マージは行わない。

## Materialization Order

1. Program: `PROGRAM-GDAR-001`
2. Epic: `EPIC-GDAR-001`
3. Cases: `CASE-GDAR-001` / `CASE-GDAR-002` / `CASE-GDAR-003`
4. Tasks: `TASK-GDAR-001` から `TASK-GDAR-006`
5. Human Decisions: `HD-GDAR-001` から `HD-GDAR-003` を `ato work block` のhuman turn payloadへ変換する
6. AI Repair: 現時点では空。構造だけを `ato_cli_materialization_plan.json` に保持する

## CLI Dry-run Boundary

- 予定コマンドは `ato work begin` / `ato work checkpoint` / `ato work block` / `ato evidence attach` / `ato task readiness` を前提にする。
- `run_id` はCLI実行時に決まるため、dry-runでは `<RUN_ID_FROM_...>` のplaceholderで保持する。
- `ato_cli_commands.md` は実行手順ではなく、次PRで実CLIに合わせて確定するためのplan artifactである。
- 今回のPRで実行したATO CLIは、Codex作業記録用のtask/run/checkpointだけであり、このmaterialization対象のProgram/Epic/Case/Taskには書き込まない。

## Human Turn Preview

| Human Turn | Source Decision | Reason | Recommended Option | Target Task |
|---|---|---|---|---|
| HT-GDAR-001 | HD-GDAR-001 | spec_decision | command boundaryまで汎用化する | TASK-GDAR-003 |
| HT-GDAR-002 | HD-GDAR-002 | spec_decision | 既存CLI完全互換 | TASK-GDAR-005 |
| HT-GDAR-003 | HD-GDAR-003 | merge_approval | runtime codeは人間確認 | TASK-GDAR-006 |

## Task Lane Preview

| Task | Case | Planned PR | Current Status | Decision |
|---|---|---|---|---|
| TASK-GDAR-001 | CASE-GDAR-001 | PR-GDAR-001 | planned | none |
| TASK-GDAR-002 | CASE-GDAR-001 | PR-GDAR-001 | planned | none |
| TASK-GDAR-003 | CASE-GDAR-002 | PR-GDAR-002 | needs_human_decision | HD-GDAR-001 |
| TASK-GDAR-004 | CASE-GDAR-002 | PR-GDAR-002 | planned | none |
| TASK-GDAR-005 | CASE-GDAR-003 | PR-GDAR-003 | needs_human_decision | HD-GDAR-002 |
| TASK-GDAR-006 | CASE-GDAR-003 | PR-GDAR-003 | needs_human_decision | HD-GDAR-003 |

## Evidence

- `docs/standards/delivery-artifacts-v0/examples/generic_daily_agent_runtime/ato_cli_materialization_plan.json`
- `docs/standards/delivery-artifacts-v0/examples/generic_daily_agent_runtime/ato_cli_commands.md`
- `docs/standards/delivery-artifacts-v0/examples/generic_daily_agent_runtime/human_decision_packet.json`
- `docs/standards/delivery-artifacts-v0/examples/generic_daily_agent_runtime/forge_projection.json`
- `artifacts/runs/poc-5b-generic-daily-agent-runtime/validation_report.json`
