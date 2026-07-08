---
id: kn_01KVKG7FH49649SBZEBX9CB4SR
created_at: 2026-06-21T06:50:42.724262443+09:00
origin:
  actor_type: human
  actor_id: kenji
  app: codex
  task_id: forge-delivery-agent-ato-overlap-uiux-review
type: pattern
confidence: 0.96
tags:
- ato-hq
- intake
- job-packet
- uiux
share_scope: local
visibility: pending
applicability_scope: task_type_specific
last_validated_at: 2026-06-21
---

# Summary
人間に AI 実行用パケットを作らせる UIUX は避ける。

## Context
ユーザーは、ATO-HQ から再度 Codex を動かしたときに何が起きているか直感的に分からず、人間がパケットを作る思想そのものが難しいと振り返った。

## Problem
人間に Job Packet や実行パケットを準備してもらう思想は、責任や痛みが強い high stakes な仕事でない限り負荷が高い。通常の実装依頼では、人間は頭の中にある目的やタスクを話し、AI に思考整理と実装を任せたい。

## Decision / Action
依頼入口は自然言語の目的、制約、成功条件、避けたいことを受け取り、システム側が Job Packet / Work Contract / Trace Key を生成する。人間には不足判断、リスク承認、Scope 変更、最終提出可否だけを聞く。

## Outcome
Codex CLI に直接頼む感覚に近いまま、裏側で ATO/Forge の構造化・証跡・Human Turn を生成できる。パケットは人間が作るものではなく、AI が生成し、人間が必要箇所だけ確認するものになる。

## Reuse Hint
Job Packet、Human Input Spec、Work Contract、AI delivery intake UI を設計するとき、入力フォームやパケット編集 UI を人間に直接見せる前に再利用する。

## Caveats
