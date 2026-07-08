---
id: kn_01KVV3EAMJMHK9PS3V41N8EDWF
created_at: 2026-06-24T05:41:11.058850032+09:00
origin:
  actor_type: ai
  actor_id: codex-cli
  app: ato-knowledge-cli
  task_id: poc-aicx-closeout-generic-runtime-20260624
type: pattern
confidence: 0.82
tags:
- aicx-study-bot
- catch-up-runtime
- local-runtime
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
- 定時agentをlocal PCで動かす場合、sleep復帰後のdispatch-if-dueとpolling catch-upを設計する。
- 同じ日付・同じtargetへの二重dispatch、二重採点、二重replyを防ぐrun_stateを用意する。
blocked_actions:
- PCがsleepしない前提だけで04:30などの定時agentを成功扱いにしない。
last_validated_at: 2026-06-24
---

# Summary

Local PC sleep requires catch-up semantics for scheduled agents.

## Context

AICX Study Botでは毎朝4:30にSlackへ問題を送る要件があったが、ローカルPCがsleepしていると定刻実行できないことが分かった。

## Problem

Windows Task Schedulerのwake設定やcron/systemd timerは補助になるが、local PC、WSL、ネットワーク、ユーザーログイン状態に依存する。定刻に動かなかったとき、手動再実行で二重送信や二重採点が起きると運用できない。

## Decision / Action

定時agentは「定刻に必ず動く」ではなく「復帰後に安全に追いつく」をprimary reliability modelにする。`daily-maintain` のような単発commandを何度実行しても、未送信なら送信し、返信があれば採点し、完了済みならnoopにする。

## Outcome

PoC-4では late dispatch、thread polling、maintenance receipt、run_state idempotencyを導入し、PC sleep後もcatch-up可能なruntime境界を確認した。

## Reuse Hint

毎朝・毎日・毎週などの定時agentをローカルPC前提で運用するときに再利用する。

## Caveats

厳密な定刻性が必要な場合は、local PCではなく常時起動ホストやクラウドschedulerを使う。それでもduplicate preventionとstatus projectionは必要である。
