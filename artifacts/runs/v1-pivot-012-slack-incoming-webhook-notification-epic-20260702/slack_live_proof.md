# Slack Live Notification Proof

## Summary

HDS-005=B により、Slack Incoming Webhook の実送信成功を V1 Slack通知の必須proofとして確認した。

Webhook URL は `/root/code/forge-delivery-agent/.env.slack` の `FDA_SLACK_WEBHOOK_URL` から読み、artifact、stdout、ATO checkpoint、PR本文には保存しない。

## Evidence

- Live receipt: `slack_live_notification_receipt.json`
- Dry-run receipt: `slack_dry_run_notification_receipt.json`
- Missing env fail-closed receipt: `slack_missing_env_notification_receipt.json`
- Context path request: `slack_context_path_notification_request.json`
- Context path live receipt: `slack_context_path_notification_receipt.json`
- No-open-decision status receipt: `slack_no_open_decision_status_receipt.json`

## Commands

```bash
cargo run -- start "Slack live notification smoke" --out /tmp/fda-slack-live-smoke/artifacts --json
bash -lc 'set -a; . /root/code/forge-delivery-agent/.env.slack; set +a; cargo run -- notify test --artifacts /tmp/fda-slack-live-smoke/artifacts --out /tmp/fda-slack-live-smoke/notify --channel slack --live --json'
cargo run -- notify test --artifacts /tmp/fda-slack-live-smoke/artifacts --out /tmp/fda-slack-live-smoke/dry-run --channel slack --json
cargo run -- notify test --artifacts /tmp/fda-slack-live-smoke/artifacts --out /tmp/fda-slack-live-smoke/missing-env --channel slack --live --json
bash -lc 'set -a; . /root/code/forge-delivery-agent/.env.slack; set +a; cargo run -- notify test --artifacts /tmp/fda-slack-context-smoke/artifacts --out /tmp/fda-slack-context-smoke/notify-context-path --channel slack --live --json'
```

No-open-decision status update は同じ `FDA_SLACK_WEBHOOK_URL` を使い、Node fetchで送信した。payloadは `slack_no_open_decision_status_receipt.json` にsecretなしで要約した。

## Result

- Live: `status=sent`, `adapter=slack_incoming_webhook`, `http_status=200`, `provider_response_digest=fnv1a64:08b05d07b5566bef`
- Dry-run: `status=skipped`, `sent=false`
- Missing env: `status=blocked`, `adapter=slack_incoming_webhook`, `sent=false`
- Context path live: `status=sent`, `adapter=slack_incoming_webhook`, `http_status=200`, `repo_name=forge-delivery-agent`, `project=forge-delivery-agent`, `decision_document_path=/tmp/fda-slack-context-smoke/artifacts/human_decision_packet.md`
- No-open-decision status: `status=sent`, `http_status=200`, `repo_name=forge-delivery-agent`, `decision_document_full_path=not_applicable_no_open_decision`
- Secret scan: artifact、src、docs、`.fda/` に実際の webhook URL 本体は保存されていない。URL prefixの一般例示はdocs/testsにだけ残る。
