---
id: kn_01KVV3EANQ21DABVZKPMTSDPKM
created_at: 2026-06-24T05:41:11.095742224+09:00
origin:
  actor_type: ai
  actor_id: codex-cli
  app: ato-knowledge-cli
  task_id: poc-aicx-closeout-generic-runtime-20260624
type: pattern
confidence: 0.82
tags:
- aicx-study-bot
- observability
- run-state
share_scope: local
visibility: pending
applicability_scope: task_type_specific
applies_to_agent_roles:
- orchestrator
- implementer
applies_to_paths:
- docs/standards/delivery-artifacts-v0/examples/**/*daily*runtime*
- scripts/**/*daily*
required_checks:
- daily runtimeにはrun_state、last_event、artifact path、next_actionを表示するstatus projectionを用意する。
- stdout/stderr全文ではなく、人間が次に何をすべきか読めるsummaryを出す。
blocked_actions:
- run_stateやreceiptだけを残し、人間が現在状態を即確認できるcommandやprojectionを用意しないまま運用開始しない。
last_validated_at: 2026-06-24
---

# Summary

Daily agent runtime needs a status projection from run_state.

## Context

AICX Study Botでは、dispatch、poll、grade、maintenanceが別commandになったため、現在の状態を人間が即確認できないと運用判断が難しくなった。

## Problem

`run_state.json` とreceiptがあっても、人間が「送信済みか」「返信待ちか」「採点済みか」「次に何を実行すべきか」を毎回JSONから読むのは運用負荷が高い。状態が見えないrunnerは、再実行や復旧の判断を誤らせる。

## Decision / Action

daily runtimeには `daily-status` 相当のprojectionを用意する。表示する内容は、effective status、raw status、last_event、Slack thread、主要artifact、next_actionを最低限とする。

## Outcome

PoC-4では `daily-status` を追加し、Slack APIへ接続せずに `run_state.json` から現在状態と次actionを読めるようにした。

## Reuse Hint

daily-dispatch、daily-grade、daily-maintain、run_state、receiptを持つ単発runnerを設計するときに再利用する。

## Caveats

status projectionは状態を更新しない。状態を更新する場合は、maintenanceやpolling commandを別に実行する。
