---
id: kn_01KVV3EAKS21MG9Y2Z7JE77Y21
created_at: 2026-06-24T05:41:11.033528788+09:00
origin:
  actor_type: ai
  actor_id: codex-cli
  app: ato-knowledge-cli
  task_id: poc-aicx-closeout-generic-runtime-20260624
type: pattern
confidence: 0.82
tags:
- aicx-study-bot
- codex-cli
- local-runtime
share_scope: local
visibility: pending
applicability_scope: task_type_specific
applies_to_agent_roles:
- orchestrator
- implementer
applies_to_paths:
- docs/pocs/**/*
- docs/standards/delivery-artifacts-v0/examples/**/*daily*runtime*
- scripts/**/*daily*
required_checks:
- 長時間listenerやschedulerを追加する前に、実行主体がCodex CLIセッションなのかOS/専用runtimeなのかを確認する。
- local-only運用であれば、単発commandとrun_stateで再開できる経路を先に用意する。
blocked_actions:
- Codex CLIの一時実行セッションを、毎日動く本番runnerの所有者として設計しない。
last_validated_at: 2026-06-24
---

# Summary

Codex CLI session should not own long-running local runtime.

## Context

AICX Study Bot PoCでは、Slack Socket Mode listenerをCodex CLIの作業セッションから長時間起動して、返信を受けて採点する運用を試した。

## Problem

Codex CLIの一時exec、親shell、WSL、ローカルPC sleep、ネットワーク断、WebSocket refreshの境界が絡むと、listenerが落ちたのか待機中なのかが見えにくくなる。Codex CLIは開発や検証には向いているが、毎朝動き続ける運用主体としては弱い。

## Decision / Action

Codex CLIは、開発、修正、検証、PR作成の実行者として扱う。長時間runnerの所有者は、OS scheduler、単発catch-up command、または専用daemon/runtimeへ分ける。

## Outcome

PoC-4では長時間listenerをprimary pathにせず、`daily-maintain`、`daily-grade-poll`、`run_state.json`、`daily-status` による再開可能なruntimeへ寄せた。

## Reuse Hint

ローカル常駐、Socket Mode、scheduler、Codex CLI連携を設計するときに再利用する。

## Caveats

専用daemonや常時起動ホストを使える場合はSocket Mode listenerをprimary pathにしてよい。ただし、その場合もrun_state、receipt、status projectionは必要である。
