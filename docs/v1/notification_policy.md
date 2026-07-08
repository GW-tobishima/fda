# FDA V1 Notification Policy

## 1. 目的

FDA V1 の通知は、Human Decision が必要な場面で人間に戻すための補助導線である。通知は state の正本ではない。正本は ATO の decision / checkpoint / evidence と repository artifact に置く。

## 2. Priority

| Priority | Channel | V1 扱い | 理由 |
|---|---|---|---|
| P0 | Slack Incoming Webhook | 実装対象 | Human Turnに気づきやすく、SMTP credential / 送信元検証 / 迷惑メール判定を主経路から外せる |
| P1 | email SMTP | deprecated/docs-only互換 | 既存実装は残すが、V1のHuman Turn主経路にはしない |
| P2 | Codex app notification / GitHub issue / ATO UI / Slack interactivity | 後続 | 返信取り込み、button、公式通知route、UI集約はV1.5以降に分ける |

V1 は P0 Slack Incoming Webhook を基準にする。Slackは一方向通知のみとし、判断の正本は `fda decide` と ATO decision / repository artifact に置く。

## 3. 通知を送る条件

通知を送る:

- Human Decision が新規作成された。
- Human Decision の期限が近い。
- Human Decision が stale になった。
- retry 上限超過で人間判断が必要になった。
- merge / release / risk approval が必要になった。

通知を送らない:

- test 未実行。
- missing proof。
- stale evidence。
- trace gap。
- schema validation failure。
- format repair。

通知を送らない項目は AI repair として扱う。ただし `fda status` では visible にする。

## 4. Notification Request

`notification_request.json` は最低限次を含む。

```json
{
  "notification_id": "ntf-001",
  "task_key": "TASK-FDA-001",
  "decision_id": "HD-FDA-001",
  "channel": "slack",
  "recipient": "#fda-human-turn",
  "recipient_source": "env:FDA_SLACK_CHANNEL_LABEL",
  "sendable": true,
  "reason": "human_decision_required",
  "summary": "集計表示閾値の判断が必要です。",
  "decisions": [
    {
      "decision_id": "HD-FDA-001",
      "summary": "集計表示閾値の判断が必要です。",
      "options": ["10 users / 10 records", "20 users / 20 records"],
      "recommended_option_id": "10 users / 10 records",
      "resume_command": "fda decide HD-FDA-001 --answer \"10 users / 10 records\""
    }
  ],
  "options": ["10 users / 10 records", "20 users / 20 records"],
  "recommended_option": "10 users / 10 records",
  "due_at": "2026-07-01T00:00:00Z",
  "required_before": "Design Gate",
  "impact_if_delayed": "Design Gate 以降へ進めません。",
  "resume_command": "fda decide HD-FDA-001 --answer \"10 users / 10 records\"",
  "artifact_links": ["human_decision_packet.md", "requirements_definition.md"],
  "created_at": "2026-06-27T00:00:00Z"
}
```

`slack` channel の recipient は通知先labelであり、credentialではない。次の順で解決する。

1. `fda notify test --to <channel-label>`
2. `FDA_SLACK_CHANNEL_LABEL`
3. どちらも無い場合は `slack:webhook` を入れる。

Slack live送信のcredentialは `FDA_SLACK_WEBHOOK_URL` だけから読む。この値はsecretなので artifact、ATO、PR本文、stdout、stderrに保存しない。V1ではHDS-007により通常Slackのみを対象にし、webhook URLは `https://hooks.slack.com/services/` で始まるものだけを許可する。

`email` channel の recipient は互換adapterでのみ次の順で解決する。

1. `fda notify test --to <email>`
2. `FDA_NOTIFY_EMAIL`
3. どちらも無い場合は既定候補 `kenjiii534@gmail.com` を入れ、`recipient_source=default:candidate`、`sendable=false` とする。

`fda notify test` は既定では通知 request / receipt / notice を作る dry-run であり、実送信しない。dry-run receipt は `status=skipped`、`dry_run=true`、`sent=false` として記録する。

`fda notify test --live --channel slack` は Slack Incoming Webhook へHTTPS POSTする。必須 env は `FDA_SLACK_WEBHOOK_URL` とする。任意の表示labelとして `FDA_SLACK_CHANNEL_LABEL` を使える。credential未設定、URL不正、非2xx応答、response bodyが `ok` ではない場合はsuccess扱いにせず、fail-closed receiptを残す。実Slack送信成功はHDS-005=BによりV1 Slack通知の必須proofとする。

`fda notify test --live --channel email` は deprecated/docs-only互換でSMTP app password方式の実メール送信を試行できる。必須 env は `FDA_SMTP_HOST`、`FDA_SMTP_PORT`、`FDA_SMTP_USERNAME`、`FDA_SMTP_PASSWORD`、`FDA_SMTP_FROM` とする。`FDA_SMTP_TLS_MODE` は `starttls`、`tls`、`none` を受け付け、既定は `starttls` とする。live送信では、privacy leakを避けるため `--to` または `FDA_NOTIFY_EMAIL` による明示recipientを必須にし、既定候補 `kenjiii534@gmail.com` だけでは送信しない。

credential または必須 env が未設定の場合は success 扱いにせず、`notification_receipt.json` に `status=blocked`、`adapter=smtp`、`failure_reason`、`sent_at`、`sent=false` を残す。secret 値は artifact、stdout、test failure に出さない。

live 送信時は `recipient` と `FDA_SMTP_FROM` を SMTP envelope address として検証し、CR/LF、control character、angle bracket、`@` 欠落を拒否する。SMTP本文は `Content-Transfer-Encoding: base64` でASCII化し、UTF-8のHuman Decision本文がSMTP commandやDATA terminatorとして解釈されないようにする。

`starttls` または `tls` では SMTP host に対する SNI と hostname 検証を行う。証明書検証やTLS起動に失敗した場合もsuccess扱いにせず、`status=failed` の fail-closed receipt を残す。

SMTP接続と応答待ちは30秒を上限にする。`none` mode のDNS解決とTCP接続も bounded worker と `connect_timeout` を使い、送信不能時に長時間停止しないようにする。

live Slack本文には、decision ID、summary、options要約、recommended option、resume commandを含める。live 送信成功時は `status=sent`、`adapter=slack_incoming_webhook`、`http_status`、`provider_response_digest`、`sent_at`、`recipient`、`webhook_source` を receipt に残す。`codex-app` channel は実送信対象外であり、live 指定時も fail-closed 相当の skipped/blocked receipt にする。

`message_id` は、同秒内の複数送信でも重複しないよう、高精度時刻、process id、process内counterから生成する。

## 5. Notification Receipt

`notification_receipt.json` は最低限次を含む。

```json
{
  "notification_id": "ntf-001",
  "request_id": "req-001",
  "channel": "slack",
  "status": "skipped",
  "dry_run": true,
  "sent": false,
  "sent_at": null,
  "provider_message_id": null,
  "failure_reason": null
}
```

`status` は次のいずれかにする。

- `queued`
- `sent`
- `failed`
- `skipped`
- `blocked`

通知失敗は FDA run 全体の失敗ではない。ただし Human Decision 自体は未解決のままなので、`fda continue` は gate で止まる。

## 6. Human Turn Notice

`human_turn_notice.md` は、人間が読む最小文面である。

必須項目:

- 判断ID
- 質問
- 推奨 option
- option 一覧
- なぜ今必要か
- 遅延時影響
- 証跡リンク
- 再開 command
- owner role

例:

```markdown
# Human Decision Required: HD-FDA-001

## Question

集計表示閾値を `unique_user_count >= 10` にしてよいか。

## Recommended Option

10 users / 10 records

## Why Now

Design Gate で privacy 表示要件を固定する必要があるため。

## Resume

`fda decide HD-FDA-001 --answer "10 users / 10 records"`
```

## 7. Slack Adapter Contract

Slack adapter は次を受け取る。

- webhook_url from `FDA_SLACK_WEBHOOK_URL`
- channel_label from `FDA_SLACK_CHANNEL_LABEL` or CLI `--to`
- JSON payload with `text` and optional `blocks`
- artifact_links
- decision_id
- task_key

Slack adapter は次を返す。

- http_status
- provider_response_digest
- sent_at
- status
- failure_reason
- retryable

Slack本文に secret、raw prompt、raw stdout / stderr 全文を入れない。

## 8. Email Adapter Contract

Email adapter は次を受け取る。

- recipient
- subject
- markdown_body
- text_body
- artifact_links
- decision_id
- task_key

Email adapter は次を返す。

- provider_message_id
- sent_at
- status
- failure_reason

Email 本文に secret、raw prompt、raw stdout / stderr 全文を入れない。

## 9. Codex App Notification Candidate

Codex app notification / automation は P2 候補である。

採用前に確認すること:

- 公式 route があるか。
- user / workspace 宛の通知 API があるか。
- notification payload に decision_id と resume command を載せられるか。
- FDA 側が message delivery receipt を得られるか。
- delivery failure を `fda status` に表示できるか。

確認できない場合、V1 は Slack Incoming Webhook のみをP0として実装する。

## 10. Status 表示

`fda status` は通知状態を次のように表示する。

```text
Notifications:
- HD-FDA-001: slack sent at 2026-07-02T00:01:00Z
- HD-FDA-002: slack failed, check notification_receipt.json
```

通知状態は Human Decision の解決状態を置き換えない。
