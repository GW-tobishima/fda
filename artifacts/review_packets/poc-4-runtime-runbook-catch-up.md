# PoC-4 Review Packet: Resumable Local Runtime Runbook

## 目的

AICX学習Botのlive運用で見えたローカルruntimeの弱点をrunbookへ反映する。アプリ機能ではなく、PC sleep、WSL停止、Socket Mode listenerの長時間維持、Codex CLI一時execの親子プロセス境界を運用リスクとして整理し、標準復旧経路をcatch-up pollingへ寄せる。

## 変更範囲

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/daily_operation_runbook.md`
  - PoC-4 runtime catch-upをScopeへ追加
  - Codex CLIの一時実行セッションを長時間runner所有者にしない判断を追加
  - 04:30 dispatch失敗時のmissed dispatch復旧手順を追加
  - Slack返信採点の標準経路をcatch-up pollingへ寄せる方針を追加
  - `daily-grade-poll` の想定コマンド、期待挙動、想定artifactを追加
  - Socket Mode single-runを任意経路として位置づけ直し
  - Windows Task Scheduler運用でdispatchとpollingを分ける方針を追加
  - listener timeout時の復旧を `daily-grade-poll` 優先に変更
  - 次PoCの優先順を `daily-grade-poll` / late dispatch / `daily-maintain` / `daily-status` に更新
- `artifacts/review_packets/poc-4-runtime-runbook-catch-up.md`
  - このPRのreview packetを追加

## 非対象

- `daily-grade-poll` の実装
- Slack Web API pollingのschema追加
- `daily-dispatch` のlate dispatch対応
- `daily-maintain`
- `daily-status`
- Socket Mode常駐daemon
- Windows Task Scheduler XMLやsystemd unitの追加
- Slack live smokeの再実行

## 受入条件対応

- ローカルPC sleep時に04:30送信できない制約を明記
  - `missed dispatch` として、PC復帰後に当日 `run_state.json` を確認する手順を追加。
- Codex CLI一時execを長時間runnerにしない判断を明記
  - `運用判断` とSocket Mode注意事項に追加。
- Socket Modeを標準信頼性機構にしない判断を明記
  - Slack返信採点の標準をcatch-up pollingへ寄せ、Socket Modeは任意経路に変更。
- 次PRで実装する `daily-grade-poll` の契約を固定
  - 想定コマンド、期待挙動、想定artifact、重複防止の前提を追加。
- run_state/idempotencyを復旧の中心に置く
  - `steps.slack_sent` と `idempotency.processed_reply_event_ids` を復旧判断に使うことを明記。

## 検証

- `git diff --check`
  - 結果: pass
- `rg -n "daily-grade-poll|PoC-4|Codex CLI|Socket Mode|missed dispatch|Knowledge|catch-up" docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/daily_operation_runbook.md`
  - 結果: 期待する新規記述が存在

## 主要成果物

- `docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/daily_operation_runbook.md`
- `artifacts/review_packets/poc-4-runtime-runbook-catch-up.md`

## 残リスク

- `daily-grade-poll` はまだ実装していない。次PRで実装し、`no_reply_found`、`reply_found_graded_and_sent`、`duplicate_reply_skipped`、`invalid_reply_found`、`poll_failed`、`thread_not_found`、`rate_limited` をreceiptで表現する。
- runbookは運用方針を固定するだけで、PC sleepからの自動復旧そのものはまだ提供しない。
- ATO Knowledge `kn_01KVRFMDDKT78MF597BAGB98NM` はlocal Knowledgeであり、repo内の正本ではない。runbookはその判断を人間が読める形に投影している。
