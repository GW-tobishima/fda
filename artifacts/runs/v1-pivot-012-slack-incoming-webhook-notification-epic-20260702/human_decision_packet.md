---
artifact_type: human_decision_packet
version: v0
status: decisions_applied
created_at: 2026-07-02
task_key: v1-pivot-012-slack-incoming-webhook-notification-epic-20260702
run_id: run_01KWGDWS4H1H1PNW3Z5ND5A9B4
---

# Human Decision Packet: FDA V1 Slack Incoming Webhook Notification

## 1. Summary

FDA V1 のHuman Turn通知は、email SMTPよりSlack Incoming WebhookをP0にしたほうが運用が軽い。

この判断パケットは、Slack通知Epicを実装PRへ進める前に人間が確認すべき事項を分離する。

人間回答は HDS-001=A、HDS-002=A、HDS-003=A、HDS-004=A、HDS-005=B、HDS-006=C、HDS-007=B。

## 2. 今あなたから必要なもの

受領済み:

- HDS-001=A
- HDS-002=A
- HDS-003=A
- HDS-004=A
- HDS-005=B
- HDS-006=C
- HDS-007=B
- `/root/code/forge-delivery-agent/.env.slack` に `FDA_SLACK_WEBHOOK_URL`

任意で追加できるもの:

- 投稿先channelの運用名。例: `#fda-human-turn`、`#dev-fda-alerts`。webhook URLはSlack側で投稿先channelを保持するため、FDAでは表示labelとしてだけ使う。

Webhook URLはsecretなので、chat、PR、Markdown artifactには貼らない。ローカルで次のように設定する。

```bash
export FDA_SLACK_WEBHOOK_URL='<slack-webhook-url>'
```

任意で、artifactやstatusに出してよい非secret labelも設定できる。

```bash
export FDA_SLACK_CHANNEL_LABEL='#fda-human-turn'
```

## 3. Decisions

### HDS-001: Notification Priority

Question: FDA V1 の通知P0をSlack Incoming Webhookへ変更し、email SMTPをP1 fallbackにするか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | Slack Incoming WebhookをP0、email SMTPをP1 fallbackにする | SMTP credential、迷惑メール、送信元検証の問題を主経路から外せる。Human Turnに気づきやすい | Slack workspace / app / webhook URLが必要 | yes |
| B | email SMTPをP0のまま維持し、SlackはP1にする | 既存実装との差分が小さい | Gmail/SendGrid/SMTPの設定負荷が残る | no |
| C | emailを無効化しSlackのみ実装する | 設計が単純 | Slack障害やworkspace制約時のfallbackがない | no |

Recommended Option: A

### HDS-002: Slack HTTP Adapter Implementation

Question: SlackへのHTTPS POSTをどう実装するか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | native Rust HTTP client dependencyを追加し、process argvにsecretを出さない | webhook URLをprocess listに出さずに済む。timeout / TLS / response handlingをコードで制御しやすい | 依存crateが増える | yes |
| B | `curl` external command adapterで送る | 実装が薄い。依存crate追加が不要 | secretをprocess argvに出さない設計が難しい。curl availabilityに依存する | no |
| C | Slack SDK / OAuth前提にする | 将来のinteractive運用へ拡張しやすい | V1には重い。OAuth / scopes / install flowが増える | no |

Recommended Option: A

### HDS-003: Slack Interaction Scope

Question: V1でSlackからの返信やボタン押下まで扱うか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | V1は一方向通知のみ。判断はCodex CLIで `fda decide` する | 早く、安全に実装できる。stateの正本がぶれない | Slack上だけで完結しない | yes |
| B | Slack buttonでoption選択まで実装する | 人間の操作は楽になる | Slack interactivity、署名検証、public endpoint、ATO decision applyが必要 | no |
| C | Slack repliesをpollingして回答として取り込む | Slack threadで議論しやすい | Events API / history API / token scope / ambiguity handlingが必要 | no |

Recommended Option: A

### HDS-004: Slack Message Detail Level

Question: Slack本文にどこまで判断内容を載せるか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | summary、decision ID、推奨option、選択肢要約、resume command、artifact linksだけ載せる | 情報漏洩リスクが低い。Slackで見やすい | 詳細確認にはOutput Hub / artifactを開く必要がある | yes |
| B | Human Decision Packetの該当項目を詳しく載せる | Slackだけで判断しやすい | 機密情報や長文がSlackに流れやすい | no |
| C | decision IDとリンクだけ載せる | 最小漏洩 | Slackだけでは緊急度や判断内容が分かりにくい | no |

Recommended Option: A

### HDS-005: Live Proof Requirement

Question: V1 Done条件に実Slack送信のlive proofを必須にするか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | credential未設定のfail-closed proofを必須にし、実Slack送信はwebhook URLが提供された場合の追加evidenceにする | secretなしでもCI/PRで検証可能。人間がURLを渡すまでblockしない | 実到達性は任意evidenceになる | no |
| B | 実Slack送信成功をV1 Done必須にする | 本番運用に近い証跡になる | secret提供とnetwork accessが必須になり、PRが止まりやすい | selected |

Selected Option: B

### HDS-006: Email Fallback

Question: 既存email SMTP adapterをどう扱うか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | email SMTPはP1 fallbackとして残す | 既存成果を捨てず、Slackが使えないrepoでfallbackできる | notification policyが2 channelになる | no |
| B | email SMTPをV1から無効化する | 設定が単純 | Slackを使えない環境で通知経路がなくなる | no |
| C | email SMTPをdeprecated扱いにしてdocsだけ残す | 実装面は軽い | 既存CLI behaviorとの整合が崩れやすい | selected |

Selected Option: C

### HDS-007: Webhook Domain Allowlist

Question: Slack webhook URLとしてどのdomainを許可するか。

| Option | 内容 | メリット | デメリット | 推奨 |
|---|---|---|---|---|
| A | `hooks.slack.com` と `hooks.slack-gov.com` を許可する | 通常SlackとGovSlackの両方に対応できる | domain validationが少し増える | no |
| B | `hooks.slack.com` のみ許可する | 実装が単純 | GovSlack workspaceで使えない | selected |
| C | 任意HTTPS URLを許可する | Slack互換proxyに使える | secret exfiltration / SSRF riskが増える | no |

Selected Option: B

## 4. Decision Summary

推奨セット:

```text
HDS-001: A
HDS-002: A
HDS-003: A
HDS-004: A
HDS-005: B
HDS-006: C
HDS-007: B
```

このセットなら、V1は次の形になる。

```text
P0: Slack incoming webhook
email: deprecated/docs-only互換
Slack: one-way Human Turn notification only
Decision: Codex CLIで fda decide
Secret: FDA_SLACK_WEBHOOK_URL env only
Live proof: 実Slack送信成功をV1 Done必須にする
Webhook domain: hooks.slack.com only
```

## 5. Impact If Delayed

- email SMTPをP0にしたまま実装が進み、Gmail / SendGrid / SMTP credentialの運用負荷がV1の主経路に残る。
- `notification.yaml`、notification policy、status表示、E2E proofのchannel前提が後から再変更になり、PR分割が増える。

## 6. Evidence Links

- Epic: `artifacts/runs/v1-pivot-012-slack-incoming-webhook-notification-epic-20260702/epic_delivery_plan.md`
- Current policy: `docs/v1/notification_policy.md`
- Current repo profile: `.fda/notification.yaml`
- Slack docs: `https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks/`

## 7. Owner Role

- Human owner: product owner
- AI owner: FDA orchestrator / current Codex CLI
- Implementation role: current Codex CLI implementer
- Review roles: `pr_reviewer`, `functional_qa`, `security_qa`, and `forge_reviewer` or `qax2` when ATO / Forge / FDA gate behavior changes
