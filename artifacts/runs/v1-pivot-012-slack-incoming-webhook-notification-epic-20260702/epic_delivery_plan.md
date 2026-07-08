---
artifact_type: epic
version: v0
status: decisions_applied
created_at: 2026-07-02
task_key: v1-pivot-012-slack-incoming-webhook-notification-epic-20260702
run_id: run_01KWGDWS4H1H1PNW3Z5ND5A9B4
---

# Epic: FDA V1 Slack Incoming Webhook Notification

## 0. Metadata

- Epic ID: `FDA-V1-SLACK-INCOMING-WEBHOOK`
- Program ID: `FDA-V1`
- Parent Epic: `FDA-V1-CLI-PRIMARY-REBASIS`
- Status: `decisions_applied`
- Owner: FDA orchestrator / product owner
- Related request: 2026-07-02 user request "P0: Slack incoming webhook について、Epicを新たに作ってください"
- Primary Artifacts:
  - `artifacts/runs/v1-pivot-012-slack-incoming-webhook-notification-epic-20260702/epic_delivery_plan.md`
  - `artifacts/runs/v1-pivot-012-slack-incoming-webhook-notification-epic-20260702/human_decision_packet.md`
- External reference:
  - Slack Developer Docs: `https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks/`

## 1. Outcome

FDA V1 の Human Turn 通知の P0 を email SMTP から Slack Incoming Webhook へ変更する。

V1 の通知主経路:

```text
Human Decision generated
  -> notification_request.json
  -> Slack incoming webhook adapter
  -> Slack channel
  -> human runs fda decide / fda continue in Codex CLI
```

Email実装は互換のため残すが、HDS-006=CによりV1のHuman Turn主経路からは外し、deprecated/docs-only互換として扱う。

### User / Business Outcome

- SMTP、Gmail app password、送信元検証、迷惑メール判定の複雑さを V1 主経路から外せる。
- Human Turn が Slack channel に流れ、Codex CLI を見ていない時間帯でも判断依頼に気づきやすくなる。
- 通知本文に `decision_id`、推奨 option、再開 command、Output Hub / Decision Packet へのリンクを載せ、Slack から Codex CLI へ戻れる。
- 通知は state の正本ではない。正本は ATO decision / checkpoint / evidence と repository artifact に置く。

### Success Metrics

- `docs/v1/notification_policy.md` で P0 が Slack Incoming Webhook、email SMTP が deprecated/docs-only互換として定義されている。
- `.fda/notification.yaml` と schema が `slack` channel、credential source、required env、secret storage policy を表現できる。
- `fda notify test --channel slack` は dry-run で `notification_request.json`、`notification_receipt.json`、`human_turn_notice.md` を生成する。
- `fda notify test --channel slack --live` は `FDA_SLACK_WEBHOOK_URL` が設定されている場合だけ Slack へ送信し、未設定なら fail-closed receipt を残す。
- Slack webhook URL は artifact、stdout、stderr、ATO checkpoint、PR body、review packet に保存しない。
- HDS-005=Bにより、Slack live 送信成功時のreceiptをV1 Slack通知の必須proofとして残す。
- Slack live 送信成功時、receipt に `status=sent`、`adapter=slack_incoming_webhook`、`sent_at`、`http_status`、`provider_response_digest` を残す。
- Slack live 送信失敗時、receipt に `status=failed` または `blocked`、`failure_reason`、`retryable` を残す。
- `fda status` が open Human Decision ごとの Slack通知状態を表示する。
- Review Agent Gate と artifact validation で、secret漏洩、payload不足、resume command不足、receipt不足を検出できる。

### Non-goals

- Slack上のボタン押下で `fda decide` を完結させること。
- Slack replies / Events API / Socket Mode で人間回答を取り込むこと。
- Slack App OAuth install flow を FDA が自動実行すること。
- Slack message の削除、更新、thread `ts` 取得を V1 必須にすること。
- Slack通知を ATO / artifact の正本状態の代替にすること。
- raw prompt、raw stdout / stderr、secret、個人情報、未レビューの機密差分をSlack本文へ載せること。

## 2. Slack Incoming Webhook Constraints

Slack Incoming Webhook は、Slack App に紐づく一意の URL へ JSON payload を POST して message を投稿する仕組みである。

V1で前提にする制約:

- Webhook URL は secret として扱う。repository、artifact、ATO、PR、ログに保存しない。
- webhook は基本的に選択済みchannelへ投稿する。Incoming Webhook単体では投稿先channel、username、iconを実行時に任意変更しない。
- 投稿後のmessage削除は Incoming Webhook では扱わない。
- 成功は HTTP 200 と `ok` response を期待する。
- `invalid_payload`、`invalid_token`、`no_active_hooks`、`channel_is_archived`、`action_prohibited` などは fail-closed receipt にする。
- GovSlack の場合は `slack-gov.com` domain を許可する。

## 3. Scope

### In

- Notification priority の変更:
  - P0: Slack Incoming Webhook
  - P1: email SMTP
  - P2: Codex app notification / GitHub issue / ATO UI / Slack interactivity
- `.fda/notification.yaml` の `slack` section 追加。
- Repository profile schema の `slack` section 追加。
- CLI contract:
  - `fda notify test --channel slack`
  - `fda notify test --channel slack --live`
  - `fda notify test --channel email` は fallback として維持
- Slack notification request / receipt の artifact contract。
- Slack本文 rendering:
  - run / task
  - decision ID
  - summary
  - required_before
  - options
  - recommended option
  - resume command
  - artifact links
- Secret handling:
  - `FDA_SLACK_WEBHOOK_URL` は env から読む
  - secret値は永続化しない
  - URL validation は HDS-007=B に従い、`https://hooks.slack.com/services/` だけを許可候補にする
- Fail-closed behavior:
  - env missing
  - invalid URL
  - malformed payload
  - non-2xx response
  - timeout
  - response body not `ok`
- Status / Output Hub integration。
- Dry-run、credential-missing、invalid payload、successful live smoke の検証計画。

### Out

- Slack OAuth install automation。
- Slack interactive components を使った承認ボタン。
- Slack reply / thread polling による回答取り込み。
- 複数workspace / 複数channel routing。
- Slack file upload。
- Slack通知失敗時のHuman Decision自動解決。

## 4. Epic ClaimContract

| Claim ID | Type | Statement | Blocking | Proof |
|---|---|---|---|---|
| CLM-SLACK-001 | product | FDA V1の通知P0はSlack Incoming Webhookである。 | yes | notification policy / roadmap update |
| CLM-SLACK-002 | security | Slack webhook URLはsecretとして扱われ、artifact、ATO、stdout、PRに保存されない。 | yes | tests / review packet / secret scan |
| CLM-SLACK-003 | adapter | credentialありならSlackへlive送信し、credentialなしならfail-closed receiptを残す。 | yes | `fda notify test --channel slack --live` evidence |
| CLM-SLACK-004 | artifact | notification request / receipt はSlack channelでもemail channelでも共通contractを維持する。 | yes | schema / fixture / validation |
| CLM-SLACK-005 | UX | Slack本文だけで人間が判断内容と再開commandを理解できる。 | yes | rendered message fixture / Human Turn Notice |
| CLM-SLACK-006 | governance | Slack通知はHuman Decisionの解決状態を置き換えない。 | yes | status / continue gate / docs |
| CLM-SLACK-007 | migration | email SMTPはP1 fallbackとして残り、既存email実装を壊さない。 | yes | regression tests |

## 5. Case Graph

| Case ID | Purpose | Depends On | Claims | Risk |
|---|---|---|---|---|
| CASE-SLACK-001 | Slack P0 notification policyを正本化する | human decision HDS-001 | CLM-SLACK-001, CLM-SLACK-007 | medium |
| CASE-SLACK-002 | `.fda/notification.yaml` とschemaをSlack対応にする | CASE-SLACK-001 | CLM-SLACK-002, CLM-SLACK-004 | medium |
| CASE-SLACK-003 | Slack message / receipt contractを実装する | CASE-SLACK-002 | CLM-SLACK-004, CLM-SLACK-005 | medium |
| CASE-SLACK-004 | Slack live adapterを実装する | HDS-002, CASE-SLACK-003 | CLM-SLACK-002, CLM-SLACK-003 | high |
| CASE-SLACK-005 | status / Output HubにSlack通知状態を出す | CASE-SLACK-003 | CLM-SLACK-006 | medium |
| CASE-SLACK-006 | Review Agent Gate / E2E proofを整える | CASE-SLACK-004, CASE-SLACK-005 | all | high |

## 6. Planned PRs

既存のemail live notification計画履歴と衝突させないため、Slack pivot は `V1-SLACK-*` として分ける。

| Planned PR | Case | Purpose | Risk | Auto-merge Allowed |
|---|---|---|---|---|
| V1-SLACK-001 | CASE-SLACK-001 | notification policy / roadmap / operational epicでP0 Slack, P1 emailに変更する | medium | no |
| V1-SLACK-002 | CASE-SLACK-002 | `.fda/notification.yaml`、schema、profile defaultsにSlack sectionを追加する | medium | no |
| V1-SLACK-003 | CASE-SLACK-003 | Slack notification request / receipt / message rendering contractを追加する | medium | no |
| V1-SLACK-004 | CASE-SLACK-004 | `fda notify test --channel slack --live` のSlack adapterを実装する | high | no |
| V1-SLACK-005 | CASE-SLACK-005 | `fda status` / Output Hub / Decision InboxにSlack通知状態を表示する | medium | no |
| V1-SLACK-006 | CASE-SLACK-006 | Slack通知E2E proof、secret漏洩検査、Review Agent Gate packetを揃える | high | no |

## 7. Required Inputs From Human

実装計画とdry-runは、追加情報なしで進められる。

live送信までに人間から必要なもの:

| Required | Item | How to provide | Secret | Needed By |
|---|---|---|---|---|
| yes | Slack Incoming Webhook URL | ローカル環境変数 `FDA_SLACK_WEBHOOK_URL` に設定する。chat / PR / artifactには貼らない | yes | V1-SLACK-004 live smoke |
| optional | 投稿先channelの運用名 | `#fda-human-turn` のような非secret labelを会話、`.fda/notification.yaml`、または `FDA_SLACK_CHANNEL_LABEL` に記録。webhook自体が投稿先を持つため必須ではない | no | V1-SLACK-002 |
| yes | Slack workspaceがGovSlackか | HDS-007=Bにより通常Slackのみ | no | V1-SLACK-002 |
| yes | メッセージ粒度 | HDS-004=Aで決定済み | no | V1-SLACK-003 |
| yes | live smokeを必須にするか | HDS-005=Bで決定済み | no | V1-SLACK-006 |
| optional | Slack App表示名 / icon | Slack App側で設定。FDAは実行時に上書きしない | no | 運用前 |
| yes | email fallbackを残すか | HDS-006=Cによりdeprecated/docs-only互換で決定済み | no | V1-SLACK-001 |

## 8. Human Decision Points

詳細は `human_decision_packet.md` を正とする。

| ID | Trigger | Decision | Recommended |
|---|---|---|---|
| HDS-001 | 通知P0の変更 | Slack Incoming WebhookをP0、emailをP1 fallbackにするか | A |
| HDS-002 | HTTP adapter方式 | native Rust HTTP clientを追加するか、external command adapterにするか | A |
| HDS-003 | Slack interaction scope | V1は一方向通知のみか、Slack reply / buttonまで含めるか | A |
| HDS-004 | Slack本文の情報量 | summary + resume command中心か、decision詳細全文を載せるか | A |
| HDS-005 | live proofの扱い | 実Slack送信成功をV1 Done必須にする | B |
| HDS-006 | email fallback | email SMTPをdeprecated/docs-only互換にする | C |
| HDS-007 | webhook URL domain | `hooks.slack.com` の通常Slackのみ許可する | B |

## 9. Acceptance Criteria

- [ ] Slack P0 / email P1 の方針がdocsに反映されている。
- [ ] `.fda/notification.yaml` に `slack` sectionがあり、schema validationが通る。
- [ ] Slack webhook URLを環境変数から読むが、secret値をartifactやstdoutへ出さない。
- [ ] `fda notify test --channel slack` dry-runでrequest / receipt / noticeが生成される。
- [ ] `fda notify test --channel slack --live` はenv未設定で `status=blocked` のreceiptを残す。
- [ ] `fda notify test --channel slack --live` はenv設定時にHTTP POSTを実行し、HTTP responseをdigest化してreceiptに残す。
- [ ] malformed payload / invalid URL / non-2xx / timeout のnegative testsがある。
- [ ] `fda status` がSlack通知の状態を表示する。
- [ ] Review Agent Gate packetにSlack通知のsecret handling、negative tests、not-applicable reviewer理由が記録される。
- [ ] email channelの既存testが壊れていない。

## 10. Recommended Next Action

1. `V1-SLACK-001` で policy / roadmap / operational epic を P0 Slack へ更新する。
2. `V1-SLACK-002` で `.fda/notification.yaml` schema と default profile を更新する。
3. `V1-SLACK-003` から `V1-SLACK-004` で Slack rendering / live adapterを実装する。
4. HDS-005=Bに従い、ローカル `.env.slack` の `FDA_SLACK_WEBHOOK_URL` を使ってlive送信receiptを残す。
