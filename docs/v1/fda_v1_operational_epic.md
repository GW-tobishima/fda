# FDA V1 Operational Epic

## 1. 目的

このEpicは、`docs/v1/fda_v1_product_contract.md` に書かれたFDA V1を「文書上の契約」ではなく、実運用でV1と呼べる状態にするための要件定義である。

`PR-V1-011` までで、CLI、artifact、receipt、gate、Output Hubの契約カバレッジは揃う。ただし、次の外部実行やSoT連携が dry-run / fixture / receipt生成止まりの場合、Operational V1 とは呼ばない。

- Slack Incoming Webhook実通知
- `fda status`
- GitHub merge実行
- fixtureなしのcurrent Codex CLI primary実行
- ATO task / run / decision / evidence への書き戻し
- Forge Claim / Proof / PromotionDecision gate との接続

このEpicが閉じた時点で、FDA V1は「人間の目的入力から、判断停止、設計、実装PR、QA、repair、merge、非実装成果物、Output HubまでをCLI-firstで運用できる」状態になる。

## 1.1 2026-06-28 差分再調査の結論

2026-06-28時点の `origin/main`、CLI help、`src/cli/args.rs`、`src/lib.rs`、`docs/v1/fda_v1_product_contract.md` を読み直した結果、Operational V1との差分は次である。

| Gap ID | 差分 | 現状 | V1で必要な状態 | 対応PR |
|---|---|---|---|---|
| GAP-V1-001 | 通知実送信 | `fda notify test` はrequest / receipt生成のみで `status=skipped` | Slack Incoming Webhook方式で実送信し、成功/失敗/fail-closed receiptを残す | PR-V1-012 / PR #88 |
| GAP-V1-002 | `fda status` | `Command` enum / CLI helpに存在しない | phase、open decision、notification、QA/repair/merge、next commandを表示する | PR-V1-013 |
| GAP-V1-003 | CLI discoverability | parserは `fda implement --live` を受け付けるがhelpに出ていない | helpと実装済みcommandが一致する | PR-V1-013 |
| GAP-V1-004 | GitHub merge実行 | `fda merge` はgate artifactを作るがGitHub mergeを実行しない | policy許可時にGitHub mergeし、結果receiptを残す | PR-V1-014 |
| GAP-V1-005 | Current Codex CLI primary execution evidence | `fda implement` のcurrent Codex CLI primary実行がfixtureなしE2E証跡として不足している | fixtureなしでcurrent Codex CLIがimplementation handoffに基づいて実装し、PR URL/test/scope evidenceを回収する | PR-V1-015 |
| GAP-V1-006 | ATO SoT連携 | ATOへの記録は外側のCodex運用で行っており、FDA CLI runtimeはATO adapterを持たない | FDA CLIがtask/run/checkpoint/decision/evidenceをATOへ書き戻せる | PR-V1-016 |
| GAP-V1-007 | Forge Gate連携 | `fda merge` はlocal artifactsを読むが、Forge Claim / Proof / PromotionDecisionを評価しない | Forge gateを評価し、PromotionDecisionをmerge判断へ反映する | PR-V1-017 |
| GAP-V1-008 | Operational E2E proof | 代表runのend-to-end receiptがない | 実装run、repairなし/あり、非実装mode、Output Hub、statusを通したE2E証跡がある | PR-V1-018 |

人間判断は1件解決済みである。

- Slack live adapterのcredential方式: `slack_incoming_webhook`

現時点で追加の人間判断は不要である。Slack実送信確認では、実credentialを `FDA_SLACK_WEBHOOK_URL` として環境変数で渡す。credential値を文書やPR本文に書かない。

## 1.2 2026-07-04 Slack P0再ベースライン

PR #88で、FDA V1の通知P0はSlack Incoming Webhookへ切り替え済みである。Operational V1 proof pack は、旧email SMTPのblocked receiptではなく、Slack live送信成功receipt、webhook未設定時のfail-closed receipt、repo/project名とdecision文書フルパスを含むSlack通知requestを参照する。

email SMTPはV1の主経路ではなく、deprecated/docs-only互換として扱う。追加の人間判断は不要である。

## 2. 現在の到達点

現在の到達点は **Operational V1 proof complete** である。

できていること:

- `fda start` で要件定義、NFR、risk、Human Decisionを生成する。
- `fda decide` でHuman Decisionを記録する。
- `fda design` で設計成果物とPR計画を生成する。
- `fda implement` でcurrent Codex CLI primary向けのimplementation handoff、実装receipt、external PR receiptを生成する契約を持つ。
- Codex / Claude MCP呼び出し契約はV1.5 optional automationとして保持する。
- `fda review` でPR Reviewer、Functional QA、Security QAのreceiptを分け、`review_agent_gate.json` と `review_agent_gate_packet.md` に集約する。
- V1では `review_agent_gate_packet.md` を実PRの `artifacts/review_packets/pr-<PR番号>.md` へ自動反映しない。反映は明示コマンドまたは人間確認後に行う。
- `review_agent_gate_packet.md` があるrunでは、実PRの `artifacts/review_packets/pr-<PR番号>.md` へ未反映のまま `fda merge` に進めない。
- `fda continue` でrepair loopのreceiptを生成する。
- `fda merge` でQA、CI、risk、scope、Human Decisionを確認する。
- research / uiux / design-only modeで非実装成果物を生成する。
- `fda notify test` と `fda open` で通知requestとOutput Hubを生成する。

Operational V1化で解消した差分:

- Slack Incoming Webhookの実送信adapterはPR #88で接続済みで、live送信成功receiptとfail-closed receiptが残っている。
- `fda status` はPR #74で現在phase、未解決Decision、通知、次commandを表示できる。
- GitHub merge adapterはPR #75でmerge readiness / receipt生成を実装済みで、HDP-007によりV1主経路ではauto mergeせずHuman merge approvalへ戻す。
- current Codex CLI primary実行はPR #87でfixtureなしの実装PR URL、test status、scope evidenceを回収した。
- FDA CLI runtimeのATO state adapterはPR #79で証跡化済みである。
- FDA CLI runtimeのForge gate adapterはPR #80でPromotionDecisionをmerge判断へ反映できる。

## 3. V1要件

### FR-001: Slack live notification

Human Decisionで停止したとき、FDAは設定済みのSlack Incoming Webhookを使ってSlackへ通知できる。

採用方式:

- Slack Incoming Webhook方式を採用する。
- webhook URLは環境変数またはローカルsecret providerから読む。
- credential値はartifact、ATO、PR本文、stdoutに出さない。

受入条件:

- `fda notify test --channel slack --live` がSlack Incoming WebhookへHTTPS POSTする。
- 宛先labelはCLI引数または `FDA_SLACK_CHANNEL_LABEL` から解決できる。webhook URL自体は表示しない。
- webhook URLが未設定の場合は送信成功扱いにせず、fail-closedでreceiptに残す。
- receiptにはrecipient、webhook_source、adapter、http_statusまたはfailure_reason、sent_atが残る。
- webhook URLの必須設定は `FDA_SLACK_WEBHOOK_URL` とする。
- HDS-007=Bにより、V1では `hooks.slack.com` の通常Slack webhookだけを許可する。

人間判断が必要な事項:

- credential方式は `slack_incoming_webhook` で解決済み。
- 実送信確認時のwebhook URLは人間が環境に設定する。値そのものはFDA artifactへ保存しない。

### FR-002: Status command

FDAは `fda status` で現在位置を人間に返す。

受入条件:

- artifact dirから現在phaseを推定する。
- 未解決Human Decisionを一覧表示する。
- notification_request / notification_receipt の状態を表示する。
- QA / repair / merge gateの状態を表示する。
- 次に実行すべきcommandを表示する。
- JSON出力と人間向けstdoutの両方を持つ。

### FR-003: GitHub merge execution adapter

Merge Gateが通過した場合、FDAはmerge可能状態とHuman merge approval handoffをreceiptに残す。HDP-007により、V1主経路ではauto mergeしない。

受入条件:

- external PR receipt自体に `actual_pr_url` と `planned_pr_id` があることを必須にする。
- `checks.tests` が存在し、すべての必須checkがpassしていることを必須にする。
- `scope_disposition.kind=within_scope` がない場合はmergeしない。
- Human Decisionがhold / defer / reject / no の場合はmergeしない。
- `review_agent_gate_packet.md` が存在するrunでは、`artifacts/review_packets/pr-<PR番号>.md` に `REVIEW_AGENT_GATE` と `MERGE_APPROVAL: not_granted` が無い場合はmergeしない。
- GitHub merge成功時にmerge sha、method、merged_at、actor、PR URLをreceiptに残す。
- GitHub merge失敗時にfailure_reasonと再開commandをreceiptに残す。

### FR-004: Fixture-free current Codex CLI primary execution

FDAはfixtureなしでcurrent Codex CLI primary実行を行い、実装PR作成の証跡を回収できる。
PR-V1-015は、既存のPR-V1-006 implementer契約をOperational V1向けに検証する証跡PRである。

受入条件:

- `.fda/` 7ファイルprofileを確認する。無い場合は作業前に作成する。
- Human Decision未解決時は実装へ進まない。
- target repoを明示し、実装agentのcwd、prompt、approval policyをreceiptに残す。
- 実装agentからactual PR URL、test status、changed files、scope driftを回収する。
- current Codex CLI role switch、handoff、test、PR URLが揃わない場合、成功扱いにしない。

### FR-005: End-to-end V1 evidence

Operational V1は単体機能の寄せ集めではなく、end-to-endの証跡で完了扱いにする。

受入条件:

- `profile gate -> start -> decide -> design -> implement -> review -> continue if needed -> merge handoff -> open -> status` の代表runを残す。
- research / uiux / design-only の各modeで成果物完了を確認する。
- Human Decisionで停止し、Slack通知またはfail-closed notification receiptが残る。
- Output Hubから要件、設計、PR計画、QA、merge結果、未解決Decisionを見られる。

### FR-006: ATO state adapter

FDA CLIは、作業状態と判断をATOへ書き戻せる。

PR-V1-016では、各 command に共通の `--ato-sync` optionを追加する。ATO書き戻しは明示 opt-in とし、既定では外部stateを変更しない。`--ato-task`、`--ato-run-id`、`--ato-backend`、`--ato-db` で対象backendを固定し、成功/失敗のいずれも `ato_state_receipt.json` に残す。

受入条件:

- `fda start` が必要に応じてATO task / run / checkpointを作れる。
- Human Decision生成時にATO typed decisionを作れる。
- `fda decide` がartifactだけでなくATO decision answer / applyへ反映できる。
- 各stageの主要artifactをATO evidence edgeとして参照できる。
- ATO CLI / backendが利用できない場合は成功扱いにせず、adapter unavailable receiptと再開commandを残す。
- raw stdout / stderr全文やsecret値をATO DBに保存しない。

### FR-007: Forge gate adapter

FDA CLIは、merge判断時にForge Claim / Proof / PromotionDecisionを評価できる。

受入条件:

- `fda design` または `fda merge` がClaim / Proof / planned PR対応を確認できる。
- `fda merge` がPromotionDecisionの `promote` / `hold` / `reject` をmerge判断へ反映する。
- missing proof、stale evidence、trace gapをhuman approvalで通さずAI repairへ戻す。
- security / privacy / legal例外はHuman Decisionへ戻す。
- Forge adapter unavailableの場合は成功扱いにせず、receiptに残す。

## 4. Planned PRs

| PR | 内容 | 対応要件 | Done |
|---|---|---|---|
| PR-V1-012 | Slack live notification adapter | FR-001 | Slack webhook URLありならSlack送信、credentialなしならfail-closed |
| PR-V1-013 | Status command and CLI discoverability | FR-002 | 現在phase、Decision、通知、次commandが表示され、helpが実装済みcommandと一致する |
| PR-V1-014 | GitHub merge execution adapter | FR-003 | merge実行結果receiptを生成する |
| PR-V1-015 | Fixture-free current Codex CLI primary execution evidence | FR-004 | fixtureなしcurrent Codex CLI実行証跡が残る |
| PR-V1-016 | ATO state adapter | FR-006 | `--ato-sync` でATO task/run/checkpoint/decision/evidenceへ書き戻し、`ato_state_receipt.json` を残す |
| PR-V1-017 | Forge gate adapter | FR-007 | Forge PromotionDecisionがmerge判断へ反映される |
| PR-V1-018 | Operational V1 E2E proof pack | FR-005 | 実装run、非実装mode、status、Output Hubを含むE2E証跡が残る |

このEpicの順序は `docs/v1/fda_v1_pr_sequence.md` のOperational V1 completionを上書きする。2026-06-28の差分再調査により、PR-V1-016からPR-V1-018を追加した。

PR-V1-018 の proof pack は `docs/standards/delivery-artifacts-v0/examples/fda_v1_operational_e2e/` に置く。2026-07-04時点の proof pack は、PR-V1-012 から PR-V1-018 の merge 済み事実と、notification / status / merge adapter / ATO state / Forge gate / Output Hub の証跡に加え、PR #87のfixture-free current Codex CLI primary実装証跡とPR #88のSlack P0通知証跡をまとめる。MCP live未達はV1.5 optional automationへ退避し、V1 Done blockerにはしない。

## 5. Stage Gate

| Gate | 必須証跡 | 通過条件 |
|---|---|---|
| Notification Gate | notification_request.json, notification_receipt.json | Slack実送信成功、またはcredential未設定をfail-closedで記録 |
| Status Gate | status_summary.json | phase、open decision、next commandが欠落しない |
| Merge Execution Gate | merge_gate_summary.json, external_pr_receipt.json, github_merge_receipt.json | policyが許可し、GitHub merge結果が回収される |
| Live Execution Gate | implementation_handoff.md, implementation_receipt.json, external_pr_receipt.json, live_execution_evidence.json | fixtureなしでPR URL、test、scope evidenceが回収される |
| ATO State Gate | ato_state_receipt.json | task/run/checkpoint/decision/evidenceがATOへ書き戻される |
| Forge Gate | forge_promotion_receipt.json | PromotionDecisionがmerge判断へ反映される |
| V1 Evidence Gate | output_hub.html, execution_status.html, status_summary.json, end_to_end_receipt.json | 実装runと非実装modeの代表証跡が揃う |

V1 Evidence Gate は schema validation が pass するだけでは通過しない。`end_to_end_receipt.json` の `status` が `succeeded` で、`blocking_issues` が空であり、fixtureなし実装runのactual PR URL、test status、scope evidenceが揃っていることを Operational V1 完了条件に含める。

## 6. Human Decision Policy

このEpicで実装前に必要だった人間判断は、通知credential方式である。これは `slack_incoming_webhook` として解決済みである。

未解決のまま進めない事項:

- GitHub mergeをauto実行するrepository policyの例外
- security / privacy / legal riskの例外承認

AI側で実装してよい事項:

- Slack Incoming Webhook adapter
- webhook URL未設定時にfail-closedする実装
- `fda status` の表示項目
- merge gateの必須証跡強化
- fixture-free current Codex CLI primary実装の証跡回収
- Output Hub / statusへのreceipt表示
- ATO adapter unavailable時のfail-closed receipt
- Forge adapter unavailable時のfail-closed receipt

## 7. V1 Done Definition

このEpicは、AI側では次をすべて満たしたらOperational V1 proof completeとする。最終mergeはHDP-007によりHuman merge approvalへ戻す。

- Operational V1 completionのPR-V1-012からPR-V1-018に対応する証跡が揃い、PR #87とPR #88のrebaseline差分がreview可能である。
- Codex自動レビューの未対応threadがない。
- CIがpassしている。
- ATOに作業開始、checkpoint、検証、merge approval handoffが残っている。
- FDA CLI自身がATO state adapterを通して主要stateを書き戻せる。
- FDA CLI自身がForge gate adapterを通してPromotionDecisionをmerge判断へ反映できる。
- Output HubでV1 E2E成果物をローカル確認できる。
- `end_to_end_receipt.json` が `status=succeeded` で、fixtureなしcurrent Codex CLI primary実装のactual PR URL、test status、scope evidenceを参照している。
- `docs/v1/fda_v1_product_contract.md` と実装状態の差分が解消されている。
