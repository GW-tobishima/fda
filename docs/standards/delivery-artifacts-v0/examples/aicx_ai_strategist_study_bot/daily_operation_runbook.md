---
artifact_type: daily_operation_runbook
version: v0
status: draft
---

# AICX学習Bot Daily Operation Runbook

## 0. Metadata

- Document ID: RUNBOOK-AICX-STUDY-BOT-DAILY-001
- Version: v0
- Owner: forge-delivery-agent
- Status: draft
- Related Program: PROGRAM-AICX-STUDY-BOT-001
- Related Epic: EPIC-AICX-STUDY-BOT-POC-001
- Scope: PoC-2I / PoC-3A / PoC-4 runtime catch-up

## 1. 目的

毎朝4:30 JSTに10問のAICX朝トレをSlackへ送り、Slack thread返信を1件受けて採点結果を同じthreadへ返す運用手順を定義する。

PoC-2Iでは長時間daemonを作らない。外部schedulerが `daily-dispatch` を起動し、返信採点は `socket-reply-listen --single-run --run-state ...` または後続PoCの `daily-grade-poll` を必要なタイミングで起動する。

PoC-4ではローカルPC運用の前提を修正する。Codex CLIの一時実行セッションは長時間runnerの所有者にしない。PC sleep、WSL停止、WebSocket切断、親shell終了で落ちても後から追いつけるよう、標準復旧経路を `daily-maintain`、`run_state.json`、Slack thread pollingに寄せる。

## 1.1 運用判断

今回のlive運用で次を確認した。

- AICX Botのアプリ機能は成立している。10問送信、Slack thread返信、採点、Slackへのレポート返信、`run_state.json` 更新は動作する。
- 04:30 JSTのローカルcronは、PC/WSLがsleepしていると実行できない。
- Codex CLIの一時 `exec` から `nohup ... &` でSocket Mode listenerを起動しても、この環境では親shell終了後に残らないことがある。
- Socket Modeはpublic HTTP endpointを不要にできるが、WebSocket接続の維持、再接続、切断検知、二重listener防止が必要になる。

したがって、ローカルPC前提の標準運用は次の順にする。

1. 外部schedulerはまず `daily-maintain` を起動する。
2. 04:30前なら `not_due`、未送信ならdispatch、送信済み未採点ならSlack thread polling、採点済みならnoopにする。
3. 返信採点は、長時間Socket Mode listenerより先に `conversations.replies` 相当のthread pollingでcatch-upする。
4. 二重送信と二重採点は `run_state.json` の `steps` と `idempotency.processed_reply_event_ids` で防ぐ。
5. Socket Mode listenerは常時運用の標準経路ではなく、短時間smokeまたは常時起動ホストへ移した後の選択肢とする。

関連Knowledge:

- `kn_01KVRFMDDKT78MF597BAGB98NM`: Codex execからnohupで起動したSlack Socket Mode listenerは、この環境では親shell終了後に残らないことがある。

## 2. 前提

- repoはWSL上の `/root/code/forge-delivery-agent` にある。
- Python仮想環境は `.venv` にあり、`slack_sdk` が入っている。
- `.env.local` はrepo rootに置くが、git管理しない。
- `data/study_bot/` はgit管理しない。`run_state.json`、Slack送信receipt、採点結果などのlive runtime artifactはここに保存する。
- PDF ingestはまだ運用に入れない。出題は `study_schedule.json`、`topic_map.json`、`question_bank.fixture.json` から行う。
- PoC-3A以降は、前回の採点結果から作った `adaptive_plan.json` がある場合だけ `daily-dispatch --adaptive-plan` に渡し、翌日の10問配分を弱点topic寄りにする。

## 3. 必要な環境変数

`.env.local`:

```dotenv
SLACK_BOT_TOKEN=xoxb-...
SLACK_CHANNEL_ID=C...
SLACK_APP_TOKEN=xapp-...
```

用途:

| 変数 | 用途 |
|---|---|
| `SLACK_BOT_TOKEN` | `chat.postMessage` で問題と採点結果を送る |
| `SLACK_CHANNEL_ID` | 朝トレを送るSlack channel |
| `SLACK_APP_TOKEN` | Socket ModeでSlack Events API message eventを受ける。PoC-4標準のpolling運用では必須にしない |

## 4. 保存先規約

live run:

```text
data/study_bot/runs/live/YYYY-MM-DD/
  quiz_set.json
  quiz_prompt.json
  quiz_prompt.md
  slack_outbound_message.json
  slack_delivery_receipt.json
  slack_reply_event.json
  answer_submission.json
  grading_report.json
  study_recommendation.json
  slack_grading_response.json
  slack_grading_delivery_receipt.json
  slack_reply_intake_receipt.json
  slack_thread_poll_receipt.json
  maintenance_receipt.json
  run_state.json
  adaptive_plan.json  # adaptive dispatch時だけコピーされる
```

dry-run:

```text
data/study_bot/runs/dry-run/YYYY-MM-DD/
```

liveとdry-runの `run_state.json` は絶対に同じディレクトリで共有しない。dry-runで `slack_used=false` の状態をlive運用のidempotency判定に混ぜると、実送信していないのに二重送信skip扱いになる可能性がある。

## 5. 標準: daily-maintain

PC復帰後や外部schedulerからは、まず `daily-maintain` を実行する。これは長時間常駐しない単発commandで、同じ日付に何度実行してもよい。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE="$(TZ=Asia/Tokyo date +%F)"

.venv/bin/python scripts/aicx_study_fixture.py daily-maintain \
  --date "${RUN_DATE}" \
  --out-root data/study_bot/runs/live \
  --env-file .env.local \
  --allow-late \
  --late-window-hours 12
```

判定:

- 04:30前なら `maintenance_receipt.json.status=not_due` として送信しない。
- `run_state.json` がない、または `steps.slack_sent=false` なら `daily-dispatch` 相当を実行する。
- 04:30後かつallow-late window内の初回送信は `late_dispatched` になる。
- `steps.slack_sent=true` かつ `steps.graded=false` ならSlack threadをpollして、未処理返信があれば採点してthreadへ返す。
- 返信がなければ `no_reply_found` で正常終了する。
- `steps.graded=true` なら `already_graded` で何もしない。

必ず残る証跡:

```text
maintenance_receipt.json
run_state.json
```

dispatchした場合は `slack_delivery_receipt.json`、thread pollingした場合は `slack_thread_poll_receipt.json` も更新される。

## 6. 分解実行: 04:30 dispatch

日付を明示してlive送信する。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE="$(TZ=Asia/Tokyo date +%F)"
RUN_DIR="data/study_bot/runs/live/${RUN_DATE}"
mkdir -p "${RUN_DIR}"

.venv/bin/python scripts/aicx_study_fixture.py daily-dispatch \
  --study-schedule docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_schedule.json \
  --topic-map docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/topic_map.json \
  --question-bank docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/question_bank.fixture.json \
  --date "${RUN_DATE}" \
  --out-dir "${RUN_DIR}" \
  --run-state "${RUN_DIR}/run_state.json" \
  --mode live \
  --env-file .env.local
```

確認:

```bash
cat "${RUN_DIR}/run_state.json"
cat "${RUN_DIR}/slack_delivery_receipt.json"
```

成功条件:

- `slack_delivery_receipt.json.status` が `sent`
- `run_state.json.status` が `dispatched`
- `run_state.json.slack.thread_ts` にSlack投稿のtsが入る

二重送信確認:

- 同じ `RUN_DIR` / `run_state.json` で再実行した場合、`duplicate_dispatch_skipped=true` になり、Slackへ再投稿しない。

## 7. Slack返信採点

### 7.1 標準: catch-up polling

ローカルPC運用では、返信採点の標準経路をthread pollingにする。これはSlackのthread履歴を取得し、未処理の人間返信だけを採点する方式である。

想定コマンド:

```bash
cd /root/code/forge-delivery-agent
RUN_DATE="$(TZ=Asia/Tokyo date +%F)"
RUN_DIR="data/study_bot/runs/live/${RUN_DATE}"

.venv/bin/python scripts/aicx_study_fixture.py daily-grade-poll \
  --run-state "${RUN_DIR}/run_state.json" \
  --out-dir "${RUN_DIR}" \
  --env-file .env.local
```

期待する挙動:

- `run_state.json.slack.channel_id` と `run_state.json.slack.thread_ts` を読む。
- Slack threadの返信を取得する。
- bot自身の投稿、親メッセージ、subtype付きmessageを除外する。
- `run_state.json.idempotency.processed_reply_event_ids` にない返信だけを対象にする。
- `1:B 2:C ...` 形式なら採点し、同じthreadへ結果を返す。
- 有効返信がなければ `no_reply_found` のreceiptを残して正常終了する。
- 既に採点済みなら `duplicate_reply_skipped` のreceiptを残し、Slackへ再送しない。

想定artifact:

```text
slack_thread_poll_receipt.json
slack_reply_event.json
answer_submission.json
grading_report.json
study_recommendation.json
slack_grading_response.json
slack_grading_delivery_receipt.json
```

`daily-grade-poll` は `daily-maintain` からも呼ばれる。通常運用では `daily-maintain` を使い、原因切り分けが必要なときだけこの分解コマンドを直接実行する。

### 7.2 任意: Socket Mode single-run

返信を待つときは、dispatchで生成された同じ `run_state.json` を渡す。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE="$(TZ=Asia/Tokyo date +%F)"
RUN_DIR="data/study_bot/runs/live/${RUN_DATE}"

.venv/bin/python scripts/aicx_study_fixture.py socket-reply-listen \
  --quiz-set "${RUN_DIR}/quiz_set.json" \
  --out-dir "${RUN_DIR}" \
  --run-state "${RUN_DIR}/run_state.json" \
  --mode live \
  --single-run \
  --timeout-seconds 900 \
  --env-file .env.local
```

返信形式:

```text
1:B 2:C 3:A 4:D 5:B 6:A 7:C 8:D 9:B 10:A
```

成功条件:

- `slack_reply_event.json` が生成される
- `grading_report.json` が生成される
- `slack_grading_delivery_receipt.json.status` が `sent`
- `slack_reply_intake_receipt.json.status` が `received_graded_and_sent`
- `run_state.json.status` が `graded`
- `run_state.json.last_event.status` が `received_graded_and_sent`

二重採点確認:

- 同じ返信eventまたは既に `graded=true` のrunでは `duplicate_reply_skipped` になり、採点返信を再送しない。

注意:

- Codex CLIの一時実行セッションから長時間listenerを所有しない。
- PC sleepやWSL停止があり得るローカルPCでは、Socket Mode listenerだけを信頼性の主軸にしない。
- 長時間待機させる場合は、Windows Task Scheduler、systemd、cronなどOS側の管理下で起動し、PID、ログ、終了コードを残す。
- listenerが落ちた場合は、`daily-grade-poll` または同等のcatch-up pollingで復旧する。

## 8. Windows Task Scheduler実例

Windows Task SchedulerからWSL内のdispatchを起動する例。

Task:

| 項目 | 値 |
|---|---|
| Name | `AICX Study Bot Daily Dispatch` |
| Trigger | Daily, 04:30 |
| Program/script | `wsl.exe` |
| Arguments | `bash -lc 'cd /root/code/forge-delivery-agent && RUN_DATE=$(TZ=Asia/Tokyo date +%F) && .venv/bin/python scripts/aicx_study_fixture.py daily-maintain --date "$RUN_DATE" --out-root data/study_bot/runs/live --mode live --env-file .env.local --allow-late --late-window-hours 12'` |
| Start in | 空欄 |

Task Schedulerには `daily-maintain` を登録する。04:30時点でPCが寝ていた場合も、ログイン時または数分おきに同じcommandを実行すれば、未送信ならcatch-up dispatchし、返信があればthread pollingで採点する。Socket Mode listenerは手動実行または別taskで短いtimeoutを指定して実行する。常駐daemon化、再接続管理、繰り返しlistenは常時起動ホストへ移す場合だけ検討する。

## 9. dry-run手順

Slackへ送らずに状態遷移を確認する。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE=2026-06-21
RUN_DIR="data/study_bot/runs/dry-run/${RUN_DATE}"
mkdir -p "${RUN_DIR}"

.venv/bin/python scripts/aicx_study_fixture.py daily-dispatch \
  --study-schedule docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_schedule.json \
  --topic-map docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/topic_map.json \
  --question-bank docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/question_bank.fixture.json \
  --date "${RUN_DATE}" \
  --out-dir "${RUN_DIR}" \
  --run-state "${RUN_DIR}/run_state.json" \
  --mode dry-run \
  --force
```

dry-runでは `slack_delivery_receipt.json.status=dry_run_ready` になり、Slackには送らない。

## 9.1 adaptive plan付きdispatch

PoC-3Aでは、前回の `grading_report.json` から作った `adaptive_plan.json` を明示した場合だけ、日次dispatchがその配分を使う。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE=2026-06-22
RUN_DIR="data/study_bot/runs/dry-run/${RUN_DATE}"
mkdir -p "${RUN_DIR}"

.venv/bin/python scripts/aicx_study_fixture.py daily-dispatch \
  --study-schedule docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_schedule.json \
  --topic-map docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/topic_map.json \
  --question-bank docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/question_bank.fixture.json \
  --adaptive-plan docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/adaptive_plan.json \
  --date "${RUN_DATE}" \
  --out-dir "${RUN_DIR}" \
  --run-state "${RUN_DIR}/run_state.json" \
  --mode dry-run \
  --force
```

成功条件:

- `quiz_set.json.question_count` が `adaptive_plan.json.question_count` と一致する
- `quiz_set.json` のtopic配分が `adaptive_plan.json.topic_allocations` に従う
- `adaptive_plan.json` が `RUN_DIR` にコピーされ、当日の入力証跡として残る

fail-closed条件:

- `adaptive_plan.next_quiz_date` と `--date` が一致しない
- `adaptive_plan.question_count` とdispatch側の有効問題数が一致しない
- `adaptive_plan.topic_allocations` の合計が `adaptive_plan.question_count` と一致しない
- `adaptive_plan.topic_allocations` に当日のtopic scope外のtopicが含まれる

## 9.2 Slack thread pollingで返信を採点する

PoC-4では、Socket Mode listenerが落ちた場合やPC sleep後に復帰した場合でも、既存のSlack threadから返信を取り直して追いつけるようにする。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE=2026-06-21
RUN_DIR="data/study_bot/runs/live/${RUN_DATE}"

.venv/bin/python scripts/aicx_study_fixture.py daily-grade-poll \
  --run-state "${RUN_DIR}/run_state.json" \
  --out-dir "${RUN_DIR}" \
  --mode live \
  --env-file .env.local
```

`daily-grade-poll` は `run_state.json.slack.channel_id` と `run_state.json.slack.thread_ts` を使い、Slack `conversations.replies` からthread messagesを取得する。親投稿、bot自身の投稿、subtype付きmessage、処理済み `reply_event_id` は除外し、未処理の人間返信だけを既存の採点処理へ渡す。

成功・正常終了のstatus:

| status | 意味 |
|---|---|
| `reply_found_graded_and_sent` | 未処理返信を採点し、threadへ採点結果またはdry-run responseを作成した |
| `invalid_reply_found` | 返信はあったが形式不正で、threadへerror responseまたはdry-run responseを作成した |
| `no_reply_found` | 未処理返信がまだない。失敗ではない |
| `duplicate_reply_skipped` | 既に採点済み、またはthread内の返信が全て処理済み |

失敗status:

| status | 対応 |
|---|---|
| `blocked_missing_env_or_sdk` | `.env.local` の `SLACK_BOT_TOKEN` / `SLACK_CHANNEL_ID` と `.venv` の `slack_sdk` を確認する |
| `thread_not_found` | `run_state.json.slack.channel_id` / `thread_ts`、Botのchannel参加、history scopeを確認する |
| `rate_limited` | 時間を置いて再実行する |
| `poll_failed` | `slack_thread_poll_receipt.json.errors` を確認する |

pollingの証跡は `slack_thread_poll_receipt.json` に残る。採点まで進んだ場合は従来どおり `slack_reply_event.json`、`answer_submission.json`、`grading_report.json`、`study_recommendation.json`、`slack_grading_delivery_receipt.json` も更新される。

## 10. 失敗時の再実行

### dispatch失敗

確認するartifact:

- `run_state.json.failure`
- `slack_delivery_receipt.json.status`
- `slack_delivery_receipt.json.errors`

よくある原因:

| status | 対応 |
|---|---|
| `blocked_missing_env_or_sdk` | `.env.local` の `SLACK_BOT_TOKEN` / `SLACK_CHANNEL_ID` と `.venv` の `slack_sdk` を確認する |
| `send_failed` | Slack app権限、channel参加、token失効を確認する |
| `not_due` | 04:30前に実行している。手動確認なら `--force` を使う |

再実行:

- Slackに投稿されていない失敗なら、同じ `RUN_DIR` で再実行してよい。
- `run_state.steps.slack_sent=true` の場合は二重送信防止が効くため、再投稿したい場合は別途人間判断でrun_stateを退避してから実行する。

### listener timeout

`slack_reply_intake_receipt.json.status=timeout_no_reply` の場合、返信がまだ来ていないか、対象thread/channelが違う。

対応:

- `run_state.json.slack.thread_ts` が朝の問題投稿tsか確認する。
- Slackで回答が問題投稿のthread返信になっているか確認する。
- 同じ `RUN_DIR` で `daily-grade-poll` を実行する。
- Socket Modeの接続確認が必要な場合だけ、短いtimeoutで `socket-reply-listen --single-run --run-state ...` を再実行する。

### missed dispatch

04:30 JSTにPC/WSLがsleepしていた場合、cronやTask Scheduler経由のWSL commandは実行されない可能性がある。

対応:

- PC復帰後に `daily-maintain` を実行する。
- `run_state.json` がない、または `steps.slack_sent=false` なら、当日分のdispatchを自動で実行する。
- 遅延送信した場合も、Slack返信採点はthread pollingで追いつける。
- `steps.slack_sent=true` なら二重送信せず、thread pollingだけ実行する。

catch-up dispatch:

```bash
cd /root/code/forge-delivery-agent
RUN_DATE=2026-06-21
RUN_DIR="data/study_bot/runs/live/${RUN_DATE}"
mkdir -p "${RUN_DIR}"

.venv/bin/python scripts/aicx_study_fixture.py daily-dispatch \
  --study-schedule docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/study_schedule.json \
  --topic-map docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/topic_map.json \
  --question-bank docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/question_bank.fixture.json \
  --date "${RUN_DATE}" \
  --out-dir "${RUN_DIR}" \
  --run-state "${RUN_DIR}/run_state.json" \
  --mode live \
  --env-file .env.local \
  --allow-late \
  --late-window-hours 12
```

- 04:30前なら `run_state.json.status=not_due` で送信しない。
- 04:30後かつ12時間以内なら送信し、`run_state.json.last_event.status=late_dispatched` になる。
- Slack本文には「遅れて配信」が入る。
- 12時間を超えた場合は `run_state.json.status=late_window_expired` で送信しない。
- 既に `steps.slack_sent=true` なら `duplicate_skipped` で二重送信しない。

### invalid reply

`run_state.json.status=invalid_reply` の場合、回答形式が不足または不正。

対応:

- Slack threadにbotがerror responseを返しているか確認する。
- 正しい形式で再返信する。
- PoC-2H/2Iでは1日1採点のidempotencyを優先するため、既に `graded=true` のrunを再採点したい場合は後続PRの運用判断に回す。

## 11. 当日運用チェックリスト

- 04:30後にSlackへ10問が送られている。
- `run_state.json.status=dispatched`。
- `run_state.json.slack.thread_ts` が入っている。
- 回答は問題投稿のthreadに `1:B 2:C ...` 形式で送る。
- `daily-grade-poll` またはlistener実行後に `run_state.json.status=graded`。
- Slack threadに正答率と復習ページが返っている。
- `data/study_bot/runs/live/YYYY-MM-DD/` をgitに入れない。

## 12. 状態確認: daily-status

現在のrun状態を人間がすぐ確認したい場合は `daily-status` を使う。

```bash
cd /root/code/forge-delivery-agent
RUN_DATE="$(TZ=Asia/Tokyo date +%F)"

.venv/bin/python scripts/aicx_study_fixture.py daily-status \
  --run-state "data/study_bot/runs/live/${RUN_DATE}/run_state.json"
```

表示内容:

- `Status`: `steps` と `last_event` から導出した運用状態。例: `waiting_for_reply`、`graded`、`not_due`。
- `raw_run_state`: `run_state.status` と導出状態が異なる場合だけ表示する。
- `Slack`: channel、thread、送信有無。
- `Last Event`: 最後のevent statusと時刻。
- `Next Action`: 次に実行するべき操作。
- `Artifacts`: `quiz_prompt.md`、`slack_delivery_receipt.json`、`maintenance_receipt.json` など主要artifact path。

例:

```text
AICX Study Bot - 2026-06-24

Status:
  waiting_for_reply

Slack:
  sent: true
  thread_ts: 1782243008.159149

Next Action:
  Slack threadへ回答する。返信済みなら daily-maintain を再実行する。
```

`daily-status` はSlack APIへ接続しない。状態を更新したい場合は `daily-maintain` を実行する。

## 13. 次のPoC

PoC-4では、落ちても追いつけるローカルruntimeを先に固める。

優先順:

1. Windows Task Scheduler / cron / launchd への `daily-maintain` 登録例を環境別に増やす。
2. 常時起動ホストが使える場合だけ、Socket Mode listenerの再接続管理を検討する。

Adaptive Study Loopは、PoC-4のcatch-up/idempotency/statusが安定してから進める。
