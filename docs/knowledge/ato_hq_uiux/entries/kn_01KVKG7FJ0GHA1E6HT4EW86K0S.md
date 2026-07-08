---
id: kn_01KVKG7FJ0GHA1E6HT4EW86K0S
created_at: 2026-06-21T06:50:42.752243744+09:00
origin:
  actor_type: human
  actor_id: kenji
  app: codex
  task_id: forge-delivery-agent-ato-overlap-uiux-review
type: insight
confidence: 0.93
tags:
- ato-hq
- codex-cli
- product-boundary
- uiux
share_scope: local
visibility: pending
applicability_scope: repo_wide
last_validated_at: 2026-06-21
---

# Summary
Codex CLI が自然な入口なら、HQ 型 UI は CLI にない価値だけを担う。

## Context
ユーザーは、タスク化済みまたは頭にある作業なら Codex CLI へ直接頼めるため、ATO-HQ を使う意味が見いだせなかったと振り返った。

## Problem
タスク化されたものやユーザーの頭にあるものは、Codex CLI に直接頼めば ATO/Forge を使って適切に処理できる。この場合、別の HQ UI に依頼を入れ直す意味が見えにくい。

## Decision / Action
単独ユーザーの hands-on 実装依頼では Codex CLI を主入口として尊重する。HQ 型 UI は backlog 管理、複数案件の俯瞰、会社間・顧客向け責任、非同期レビュー、成果物提出、監査、Human Turn 集約など、CLI にない価値へ限定する。

## Outcome
HQ が CLI の劣化コピーになるのを防ぎ、必要な場面だけ使われる補助面として設計できる。ATO/Forge の利用は CLI の裏側でも成立するため、UI の存在理由を別に置ける。

## Reuse Hint
AI delivery の新しい human front door を設計するとき、Codex CLI と競合する機能を作るか、CLI では難しい運用面を担うかを判断する材料として使う。

## Caveats
