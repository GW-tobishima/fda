---
artifact_type: poc_closeout_summary
version: v0
status: draft
---

# AICX Study Bot PoC Closeout

## 0. Metadata

- Document ID: POC-SUMMARY-AICX-STUDY-BOT-001
- Version: v0
- Owner: forge-delivery-agent
- Status: draft
- Closed Scope: PoC-0 から PoC-4
- Closeout Date: 2026-06-24
- Related Program: PROGRAM-AICX-STUDY-BOT-001
- Related Epic: EPIC-AICX-STUDY-BOT-POC-001
- Next Epic: EPIC-GENERIC-DAILY-AGENT-RUNTIME-001

## 1. 結論

AICX Study Bot PoC は、AICX資格学習という小さい実用ケースを通じて、forge-delivery-agent の本命PoCへ進むための runtime 境界を確認した。

アプリ機能としては、Slackへ10問を送り、Slack thread返信を採点し、正答率と復習ページを返し、`run_state.json` と各種receiptへ証跡を残すところまで成立した。一方で、ローカルPC運用では「Codex CLIが常に動いている」という前提は弱く、PC sleep、WSL停止、WebSocket切断、親shell終了に耐えるには、長時間listenerよりも catch-up 可能な単発command runtime が必要だと分かった。

そのため、AICX Study Bot は PR #23 の `daily-status` までで一旦closeし、次は Generic Daily Agent Runtime のPoCへ進む。

## 2. PoC別成果

| PoC | 成果 | 判定 |
|---|---|---|
| PoC-0 | Human Input Spec、Requirements、Epic Delivery Plan、Environment Readiness、Task Graphを作成した。Slack-first、local-only PDF、安全境界、Human Decisionを分離した。 | PASS |
| PoC-1 | `study_schedule.json`、`topic_map.json`、quiz/grading/recommendation schema、fixture quiz generation、fixture gradingを実装した。PDF本文なしで成立する契約を作った。 | PASS |
| PoC-2A | `question_bank.fixture.json` を導入し、5 topic x 2問の人間品質fixtureから選ぶようにした。 | PASS |
| PoC-2B / 2B.1 | Slack outbound dry-run/live smoke、answer keyなしの `quiz_prompt.json` / `quiz_prompt.md` を追加した。Slack本文は全文10問を送る仕様に寄せた。 | PASS |
| PoC-2E / 2F | Socket Mode single-runでSlack返信を受け、採点artifact生成とSlack threadへの採点返信まで閉じた。 | PASS |
| PoC-2G / 2H / 2I | `run_state.json`、idempotency、daily-dispatch、daily-grade、Socket Modeとrun_stateの接続、運用runbookを整えた。 | PASS |
| PoC-3 / 3A / 3C | grading_reportからadaptive_planを作り、翌日の10問配分へ接続した。Slack内で完結する10問運用に調整した。 | PASS |
| PoC-4 | sleepやlistener断に備え、`daily-grade-poll`、late dispatch、`daily-maintain`、`daily-status` を導入した。 | PASS |

## 3. 失敗と観察

### 3.1 Codex CLIを長時間runnerにする案は不適切

Socket Mode listenerを長時間起動しようとしたが、Codex CLIの一時exec、親shell、ローカルPC sleep、WebSocket再接続の境界が弱かった。

Codex CLIは開発、修正、検証、PR作成の実行者としては有効だが、毎朝動く運用主体にはしない。運用主体はOS scheduler、単発command、または専用daemon/runtimeへ分ける。

### 3.2 local PC sleep前提ではcatch-upが必要

04:30にPCがsleepしていると、定刻dispatchは起きない。WakeToRunのようなOS機能は補助にはなるが、信頼性をそこだけに依存しない。

runtimeは、PC復帰後に `daily-maintain` を何度実行しても安全な形にする必要がある。

- 未送信なら送る
- 送信済みで未採点ならthreadをpollする
- 返信があれば一度だけ採点する
- 採点済みならnoopにする
- 失敗理由はreceiptへ残す

### 3.3 状態が見えないrunnerは信頼されない

run_stateやreceiptがあっても、人間が今何をすべきか読めなければ運用しにくい。

`daily-status` により、raw state、effective status、Slack thread、last_event、next_action、artifact pathを表示できるようにした。これはGeneric Daily Agent Runtimeでも必須にする。

### 3.4 問題品質はruntime設計と別の課題

Slackで10問を送る体験は成立したが、question bankが不足するとfallback問題が重複し、学習価値が下がる。これは問題生成品質の課題であり、runtime closeoutとは分ける。

次Epicでは問題生成ではなく、日次agentを安全に実行、追跡、再開するgeneric runtimeに集中する。

## 4. 設計学習

### 4.1 Trigger分類

AICX Study Botでは、次の起動タイプが分離できた。

| Trigger | AICXでの例 | Generic Runtimeでの扱い |
|---|---|---|
| manual | 今日分を手動再送する | operator command |
| scheduled | 毎朝4:30に問題を送る | dispatch-if-due |
| conditional | Slack返信が来たら採点する | poll/listen then grade |
| catch-up | sleep復帰後に未処理を回収する | maintain command |
| status | 現在状態を確認する | status projection |

### 4.2 Stateはartifactとして永続化する

`run_state.json` は、日次runの最低限の正本になった。

- date
- status
- steps
- slack channel/thread
- artifacts
- processed reply event
- last_event

この状態があることで、dispatch、poll、grade、statusが別commandでも同じrunを扱える。

### 4.3 Receiptは失敗の説明責任を持つ

Slack送信、thread poll、grading reply、maintenanceの各receiptは、成功だけでなくfail-closedの理由を残す必要がある。

token不足、SDK不足、rate limit、thread未発見、invalid reply、duplicate skipは、すべて成功/失敗の判断材料としてartifactに残す。

### 4.4 Long-running listenerは最初の信頼性機構にしない

Socket Modeは有効だが、local-only PoCではprimary reliability mechanismにしない。まずpolling/catch-upで復旧できるようにし、その後に常駐listenerや専用daemonを足す。

## 5. 追加したKnowledge

今回のPoCから、次のKnowledgeを追加した。

| Knowledge ID | Summary | Reuse |
|---|---|---|
| `kn_01KVV3EAKS21MG9Y2Z7JE77Y21` | Codex CLI session should not own long-running local runtime | ローカル常駐、Socket Mode、schedulerを設計するとき |
| `kn_01KVV3EAMJMHK9PS3V41N8EDWF` | Local PC sleep requires catch-up semantics for scheduled agents | 毎朝・毎日などの定時agentをローカルPC前提で運用するとき |
| `kn_01KVV3EANQ21DABVZKPMTSDPKM` | Daily agent runtime needs a status projection from run_state | 単発runner、run_state、status commandを設計するとき |

Repo内のprojection:

- `docs/knowledge/kn_01KVV3EAKS21MG9Y2Z7JE77Y21.md`
- `docs/knowledge/kn_01KVV3EAMJMHK9PS3V41N8EDWF.md`
- `docs/knowledge/kn_01KVV3EANQ21DABVZKPMTSDPKM.md`

## 6. 次PoCへの接続

次Epicは `Generic Daily Agent Runtime` とする。

AICX固有の学習botから、以下を抽出する。

- 定時dispatch
- late/catch-up dispatch
- thread/message polling
- idempotent grading/action execution
- run_state
- receipt
- status projection
- external scheduler前提の運用runbook

AICX固有として次Epicに持ち込まないもの:

- AICX資格のtopic map
- PDF ingest/OCR
- Slack quiz文面品質
- adaptive study loop
- 問題自動生成

## 7. Closeout判定

AICX Study Bot PoCは、学習botとしての改善余地は残るが、forge-delivery-agent の次段階へ必要なruntime学習を得た。

このため、PR #23までをAICX Study Bot PoCのcloseout lineとし、以降はGeneric Daily Agent RuntimeのEpicへ切り替える。
