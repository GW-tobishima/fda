---
artifact_type: requirements_definition
version: v0
status: draft
---

# Generic Daily Agent Runtime 要件定義

## 0. Metadata

- Document ID: REQ-GENERIC-DAILY-AGENT-RUNTIME-001
- Version: v0
- Owner: forge-delivery-agent
- Status: draft
- Source: HIS-GENERIC-DAILY-AGENT-RUNTIME-001
- Related Epic: EPIC-GENERIC-DAILY-AGENT-RUNTIME-001

## 1. Business Objective

- Problem: ローカルPCや一時実行セッションを前提にした日次agentは、sleep、ネットワーク断、listener停止、二重実行、状態不明に弱い。
- Desired Outcome: 任意の日次agentが、定時実行に失敗しても復帰後に追いつけ、二重実行せず、人間が状態と次actionをすぐ確認できるruntime契約を持つ。
- Success Metrics:
  - 同じ日付・同じtargetへ二重dispatchしない。
  - 未送信ならcatch-up dispatchできる。
  - 送信済みで未処理イベントがあればpoll/actionできる。
  - 完了済みならnoopになる。
  - 失敗理由がreceiptへ残る。
  - `daily-status` 相当のprojectionで現在状態と次actionが読める。

## 2. Scope

### Scope In

- Generic Daily Agent RuntimeのHuman Input SpecとRequirements Definition。
- scheduled dispatch、poll/catch-up、action execution、status projectionの責務整理。
- run_state、receipt、idempotency、failure classificationの要件定義。
- Codex CLI、OS scheduler、専用runtimeの責務境界。

### Scope Out

- AICX Study Botの新機能追加。
- Slack SDKやSocket Modeの新規実装。
- PDF/OCR/LLM問題生成。
- 本格daemon、supervisor、再接続制御。
- ATO/Forge正本stateの置換。

## 3. Functional Requirements

| ID | Requirement | Rationale | Priority | Acceptance Criteria |
|---|---|---|---|---|
| FR-GDAR-001 | 指定日とdue timeからdispatch可否を判定できる | sleep復帰後のcatch-up判断に必要 | Must | due前はnot_due、due後未送信ならdispatch candidate、window超過ならblockedになる |
| FR-GDAR-002 | dispatch結果をrun_stateとdelivery receiptに保存できる | 二重送信防止と監査に必要 | Must | sent/dry_run/blocked/failedがartifactに残る |
| FR-GDAR-003 | 既存run_stateを読んで二重dispatchをskipできる | 再実行安全性に必要 | Must | 同一date/targetで送信済みならduplicate skipになる |
| FR-GDAR-004 | inbound eventをpollingまたはfixtureから取得できる | long-running listenerなしでcatch-upするため | Must | 未処理eventがなければno_reply_found相当で正常終了する |
| FR-GDAR-005 | 未処理eventがあれば一度だけactionへ渡せる | 二重採点・二重処理を防ぐため | Must | processed event idがrun_stateへ残る |
| FR-GDAR-006 | action responseをtargetへ返し、receiptに保存できる | 条件付き処理の完了証跡に必要 | Should | response sent/failed/blockedがartifactに残る |
| FR-GDAR-007 | maintenance commandでdispatch/poll/action/noopを統合判断できる | 人間の運用判断を減らすため | Must | 何度実行しても現在状態に応じた安全な1 actionまたはnoopになる |
| FR-GDAR-008 | status commandで現在状態、last_event、next_action、artifact pathを表示できる | runner状態の可視化に必要 | Must | JSONを直接読まずに次actionを判断できる |
| FR-GDAR-009 | Codex CLIを長時間runnerの所有者にしない境界を明示できる | local session依存を避けるため | Must | docs/runbookでCodex CLI、OS scheduler、runtime ownerの責務が分かれている |
| FR-GDAR-010 | failure reasonを分類してreceiptへ残せる | 復旧判断に必要 | Must | missing env、SDK不足、network、rate limit、thread not found、invalid inputが区別される |

## 4. Non-Functional Requirements

| ID | Quality Attribute | Requirement | Target |
|---|---|---|---|
| NFR-GDAR-REL-001 | Reliability | command再実行はidempotentである | 二重dispatch/actionなし |
| NFR-GDAR-OBS-001 | Observability | run_stateとreceiptから実行因果を追える | last_event、status、artifact pathが残る |
| NFR-GDAR-OPS-001 | Operability | 人間がstatus projectionから次actionを判断できる | 1 commandで確認可能 |
| NFR-GDAR-SEC-001 | Security | tokenやsecretをrepoに保存しない | `.env.local` 等のlocal-only運用 |
| NFR-GDAR-PORT-001 | Portability | OS schedulerに載せられる単発commandを基本にする | cron、Task Scheduler、launchdへ接続可能 |
| NFR-GDAR-REC-001 | Recoverability | PC sleepやlistener停止後にcatch-upできる | due後のmaintenanceで復旧可能 |

## 5. State Contract

Generic run_stateは最低限次を持つ。

- run_id
- date
- target
- status
- steps
- last_event
- schedule
- dispatch
- inbound
- action
- artifacts
- processed_event_ids
- failure

statusは、実装時に次のような分類へ正規化する。

- `not_due`
- `waiting_for_dispatch`
- `dispatched`
- `waiting_for_inbound`
- `action_completed`
- `failed`
- `noop`

## 6. Receipt Contract

各commandは、stdoutだけでなくreceipt artifactを残す。

| Receipt | Purpose |
|---|---|
| dispatch_receipt | 送信/dry-run/skip/failureの証跡 |
| poll_receipt | inbound event取得/no event/failureの証跡 |
| action_receipt | action実行/response送信/failureの証跡 |
| maintenance_receipt | 統合commandが何を選んだかの証跡 |
| status_report | 人間向け状態projection |

## 7. Runtime Boundary

| Actor | Responsibility | Non-responsibility |
|---|---|---|
| Codex CLI | 開発、修正、手動実行、PR作成、検証 | 長時間runnerの所有 |
| OS scheduler | due timeに単発commandを起動 | business logicやidempotency判断 |
| Generic Daily Runtime | run_stateを読んでdispatch/poll/action/statusを判断 | ATO/Forge正本stateの置換 |
| ATO | task/run/decision/evidenceのcontrol plane | WebSocket接続やscheduler実行 |
| Human | scope、risk、merge、運用方針の判断 | JSON artifactを毎回手で読むこと |

## 8. Open Questions

| ID | Question | Owner | Blocking? |
|---|---|---|---|
| Q-GDAR-001 | state保存はfile artifactで十分か、SQLiteへ寄せるか | human/runtime architect | no |
| Q-GDAR-002 | inboundはpolling-firstでよいか、Socket Mode/webhookを先に抽象化するか | human/runtime architect | no |
| Q-GDAR-003 | strict deadlineが必要なagentをlocal PC対象から外すか | human/runtime architect | no |
| Q-GDAR-004 | ATO task/runとgeneric run_stateの紐づけ粒度 | runtime architect | no |

## 9. Initial Epic Slices

| Slice | Goal | Non-goal |
|---|---|---|
| GDAR-0 | Generic Daily Agent RuntimeのEpic Delivery Planを作る | 実装しない |
| GDAR-1 | generic run_state / receipt schemaを定義する | adapter実装しない |
| GDAR-2 | dispatch-if-due contractを作る | Slack専用にしない |
| GDAR-3 | poll/action contractを作る | long-running daemonにしない |
| GDAR-4 | maintenance/status projectionを作る | UIを作らない |

## 10. Closeout Input

このRequirementsはAICX Study Bot PoCのcloseoutを入力にする。AICX固有のquestion bank、PDF、資格学習範囲は次Epicへ持ち込まない。
