---
artifact_type: human_input_spec
version: v0
status: draft
---

# Generic Daily Agent Runtime Human Input Spec

## 0. Metadata

- Document ID: HIS-GENERIC-DAILY-AGENT-RUNTIME-001
- Version: v0
- Owner: forge-delivery-agent
- Status: draft
- Source: AICX Study Bot PoC closeout on 2026-06-24
- Related Epic: EPIC-GENERIC-DAILY-AGENT-RUNTIME-001

## 1. 入力メモの要約

AICX Study Bot PoCでは、毎朝Slackへ問題を送り、返信を採点し、run_stateとreceiptへ証跡を残す日次agentを小さく作った。機能自体は成立したが、ローカルPC sleep、Socket Mode listenerの長時間維持、Codex CLI一時execの親子プロセス境界により、運用runtimeの弱点が明らかになった。

次は、AICX固有の学習botではなく、任意の日次agentに使えるGeneric Daily Agent Runtimeを定義する。

## 2. 正規化した要求

- 日次agentを、定時dispatch、条件付き処理、catch-up、status projectionの組み合わせとして扱う。
- Codex CLIを長時間runnerの所有者にしない。
- local PC sleepやネットワーク断が起きても、復帰後に単発commandで追いつけるようにする。
- `run_state.json` または同等のstate artifactで、日付、target、dispatch、reply/poll、action、receipt、last_eventを永続化する。
- 同じ日付、同じtarget、同じinputに対して二重dispatchや二重actionが起きないようにする。
- 人間が現在状態と次actionを読めるstatus commandを持つ。
- 実装ではなく、次EpicのHuman Input SpecとRequirements Definitionまでを作る。

## 3. AICX PoCから抽出した汎用概念

| AICX固有の概念 | 汎用runtime概念 |
|---|---|
| 毎朝4:30に問題を送る | scheduled dispatch |
| Slack thread返信 | inbound event or polled message |
| 採点 | conditional action |
| 推奨ページ返信 | action response |
| `run_state.json` | durable run state |
| Slack delivery receipt | delivery receipt |
| thread poll receipt | poll receipt |
| maintenance receipt | orchestrated catch-up receipt |
| `daily-status` | status projection |

## 4. Human Decisions

| ID | Decision | Default | Options | Required Before |
|---|---|---|---|---|
| HDP-GDAR-001 | 最初の実行環境 | local single-machine | local single-machine, always-on server, cloud scheduler | runtime adapter design |
| HDP-GDAR-002 | inbound処理方式 | polling-first | polling-first, Socket Mode/listener-first, webhook-first | inbound adapter design |
| HDP-GDAR-003 | state保存方式 | file artifact | file artifact, SQLite, ATO-backed state | state adapter design |
| HDP-GDAR-004 | strict scheduling要件 | catch-up acceptable | catch-up acceptable, strict deadline required | scheduler policy |

## 5. Non-goals

- AICX Study Botの問題品質改善は扱わない。
- Slack専用botを新しく実装しない。
- PDF ingest、OCR、LLM問題生成は扱わない。
- 長時間daemonや再接続制御をこのsliceでは実装しない。
- ATOやForgeの正本stateを置き換えない。

## 6. PoC入力として期待するもの

- 日次agent名
- target date または period
- dispatch due time
- delivery target
- inbound event source
- action command
- state root
- idempotency key policy
- status projectionに表示する人間向け項目

## 7. 次の出力

- `requirements_definition.md`
- future: runtime epic delivery plan
- future: generic run_state schema
- future: dispatch/poll/action/status command contract
