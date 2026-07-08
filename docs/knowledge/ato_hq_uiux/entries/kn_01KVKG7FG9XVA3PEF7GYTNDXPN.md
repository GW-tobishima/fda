---
id: kn_01KVKG7FG9XVA3PEF7GYTNDXPN
created_at: 2026-06-21T06:50:42.697506184+09:00
origin:
  actor_type: human
  actor_id: kenji
  app: codex
  task_id: forge-delivery-agent-ato-overlap-uiux-review
type: pattern
confidence: 0.95
tags:
- ato-hq
- human-interface
- traceability
- uiux
share_scope: local
visibility: pending
applicability_scope: task_type_specific
last_validated_at: 2026-06-21
---

# Summary
通常の人間向け UI で ATO の Task/Run/Agent 粒度を主表示にしない。

## Context
ユーザーは ATO-HQ の UI について、ATO の作業単位で人が見るのは普段は無理、個々のエージェント作業を見るのも無理だと振り返った。

## Problem
ATO の作業単位や 1 個 1 個のエージェント作業は、人間が日常的に追うには細かすぎる。ATO-HQ の UI がこの粒度を前面に出すと、状態を見て判断するだけで負荷が高くなり、普段使いの入口にならない。

## Decision / Action
人間向け UI の既定表示は、成果物、未解決判断、リスク、期限、責任、最終結果、失敗時の影響に寄せる。Task/Run/Agent の詳細は監査・デバッグ用の drill-down とし、通常の操作面には出さない。

## Outcome
人間は AI の内部実行ログではなく、自分が判断すべき例外と受け取る成果を中心に見られる。ATO/Forge は traceability の基盤として残しつつ、日常 UI の認知負荷を下げられる。

## Reuse Hint
ATO-HQ 後継、Mission Control、Forge/ATO UI、AI delivery dashboard を設計するとき、Task/Run/Agent を主画面に出す案を評価する基準として使う。

## Caveats
