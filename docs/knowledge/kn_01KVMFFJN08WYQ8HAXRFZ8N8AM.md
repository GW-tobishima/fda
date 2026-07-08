---
id: kn_01KVMFFJN08WYQ8HAXRFZ8N8AM
created_at: 2026-06-21T15:56:53.920303409+09:00
origin:
  actor_type: ai
  actor_id: codex-cli
  app: ato-knowledge-cli
  task_id: knowledge-cli
type: pattern
confidence: 0.82
tags:
- local-runtime
- messaging
- poc
share_scope: local
visibility: pending
applicability_scope: task_type_specific
applies_to_agent_roles:
- orchestrator
- solution_architect
- implementer
applies_to_paths:
- docs/standards/delivery-artifacts-v0/examples/**/*
- adapters/messaging/**/*
required_checks:
- Slack-first と LINE-first のどちらを初期チャネルにするか human decision として分離する。
- LINE-first の場合は public HTTPS webhook、tunnel、または relay の有無を確認する。
blocked_actions:
- 完全ローカル運用を前提にしたまま LINE 返信取り込みを実装可能と断定しない。
last_validated_at: 2026-06-21
---

# Summary
完全ローカル前提の messaging Bot PoC で返信取り込みまで必要な場合、初期実装は Slack-first を優先する。LINE-first は inbound reply に public HTTPS webhook、tunnel、または relay が必要になる。

## Context
AI ストラテジスト学習 Bot の PoC-0 で、毎朝 4:30 に問題を送り、ユーザー回答を受けて採点と推奨 PDF ページを返す要件を整理した。

## Problem
Slack と LINE はどちらも送信先候補だが、完全ローカルの Codex CLI 実行を前提にすると、返信取り込みの実装難度が違う。Slack は Bot token と channel/thread polling で local-only PoC を組みやすい。LINE Messaging API は通常 webhook 受信が必要で、local-only では tunnel/relay が blocker になりやすい。

## Decision / Action
通知チャネルは human decision として分離する。短期 PoC では Slack-first を推奨し、LINE-first を選ぶ場合は public HTTPS webhook、Cloudflare Tunnel、ngrok、または小さな relay を environment readiness の blocker として扱う。

## Outcome
PoC の最初の実装範囲を、通知 adapter 抽象、Slack-first polling、mock delivery に分けやすくなる。LINE-first を選んだ場合も、返信取り込みの追加環境が明示されるため、実装開始後に local-only 前提が崩れることを避けられる。

## Reuse Hint
Codex-local の scheduler Bot、Slack/LINE 通知、ユーザー返信取り込み、学習 Bot、daily quiz Bot を計画するときに再利用する。

## Caveats
Slack でも権限 scope、チャンネル種別、履歴取得ポリシーによって polling が使えない場合がある。その場合は Slack Events API と webhook/tunnel の検討が必要になる。
