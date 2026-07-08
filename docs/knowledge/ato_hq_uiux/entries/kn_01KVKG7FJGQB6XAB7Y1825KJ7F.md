---
id: kn_01KVKG7FJGQB6XAB7Y1825KJ7F
created_at: 2026-06-21T06:50:42.768576286+09:00
origin:
  actor_type: human
  actor_id: kenji
  app: codex
  task_id: forge-delivery-agent-ato-overlap-uiux-review
type: failure
confidence: 0.92
tags:
- ato-runner
- failure
- observability
- runner
- uiux
share_scope: local
visibility: pending
applicability_scope: task_type_specific
last_validated_at: 2026-06-21
---

# Summary
HQ から runner/Codex を再起動しても、何が起きているか直感できなければ信頼されない。

## Context
ユーザーは、ato-runner は準備してもらったつもりだったが良い体験にはならず、ATO-HQ から再度 Codex を動かしたときの挙動が分かりにくかったと振り返った。

## Problem
ATO-HQ と ato-runner の接続が良い感じに見えず、HQ から再度 Codex を動かしたときに、何が起きているか直感的に分からなかった。実行状態が不透明だと、準備や構造があっても人間は安心して使えない。

## Decision / Action
runner 連携 UI は、現在フェーズ、前回実行との差分、実行主体、入力、停止条件、次に起きること、完了/失敗の証跡を 1 画面で説明する。再実行は opaque な起動ボタンではなく、なぜ再実行するのか、何を変えて渡すのか、どこまで自動で進むのかを明示する。

## Outcome
人間は runner の内部ログではなく、実行の因果関係と責任境界を把握できる。再実行や resume が直感的になり、失敗時の追跡もしやすくなる。

## Reuse Hint
ato-runner 後継、worker orchestration、retry/resume UI、Codex 再実行導線を設計するとき、実行状態の見せ方と前回差分説明の必須要件として使う。

## Caveats
