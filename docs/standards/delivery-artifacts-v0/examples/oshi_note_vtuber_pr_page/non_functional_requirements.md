---
artifact_type: non_functional_requirements
version: v0
status: draft
---

# 非機能要件: oshi-note VTuber紹介リンク / PRページ

## 0. Metadata

- NFR Set ID: NFR-ON-VPR-6A
- Related Requirement: REQ-ON-VPR-6A
- Related Epic: EPIC-OSHI-VTUBER-PR-6A
- Owner: forge-delivery-agent
- Status: draft

## 1. Quality Attribute Table

| ID | Quality Attribute | Requirement | Metric | Target | Verification |
|---|---|---|---|---|---|
| NFR-PRIV-001 | Privacy | VTuber側画面、投稿カード、画像生成に個別ファンID、メールアドレス、通常メモ本文、自由入力URL、raw logを含めない | prohibited field exposure | 0 | privacy review + API contract test |
| NFR-PRIV-002 | Privacy | 少数母数の集計bucketを表示しない | suppressed low-count buckets | 100% | aggregation unit test |
| NFR-SEC-001 | Security | VTuber dashboardは認可済みcreator/campaignの匿名集計だけ読める | cross-creator data access | 0 | authorization test |
| NFR-COMP-001 | Compliance | 案件性があるPRページ、投稿カード、SNSテンプレはPR / 提供表記を含む | required disclosure coverage | 100% | compliance review |
| NFR-COMP-002 | Compliance | YouTube / Twitch API Dataを使う場合は保持期限、refresh、表示要件を満たす | stale API Data in card | 0 | API data retention review |
| NFR-OPS-001 | Operations | 本人確認、代理人権限、素材許諾、停止受付、表示名変更の運用状態を追跡できる | required operational state coverage | 100% | operations checklist |
| NFR-OBS-001 | Observability | funnel eventは低カーディナリティmetadataに限定する | forbidden metadata values | 0 | analytics payload review |
| NFR-UX-001 | Usability | 紹介URL経由登録では推し登録済み、推し色反映済みでfirst logへ進める | first-run setup completion | 1 path | UX review |
| NFR-ACC-001 | Accessibility | 推し色は色だけに依存せずラベルや名前でも判別できる | color-only state | 0 | accessibility review |
| NFR-MAINT-001 | Maintainability | creator, campaign, referral attribution, comment opt-in, aggregationの境界を分ける | module boundary review findings | High/Critical=0 | architecture review |

## 2. Risk and Trade-offs

- 少数母数閾値を高くするとVTuber側の成果確認価値は下がるが、個人推測リスクは下がる。
- 公開用コメントに人間レビューを必須にすると安全性は上がるが、運用負荷と表示遅延が増える。
- YouTube / Twitch API Dataを使わないMVPは表現力が下がるが、保持期限と表示要件のリスクを後段へ分離できる。
- 推し色をVTuber側設定に寄せると紹介体験は強くなるが、ファン側で後から変更できる設計が必要になる。

## 3. Proof Obligations

| NFR ID | Proof Type | Owner | Blocking |
|---|---|---|---|
| NFR-PRIV-001 | privacy data-flow review | privacy reviewer | yes |
| NFR-PRIV-002 | aggregation threshold unit test | functional_qa | yes |
| NFR-SEC-001 | authorization test | security_qa | yes |
| NFR-COMP-001 | disclosure template review | legal / compliance reviewer | yes |
| NFR-COMP-002 | API data retention review | compliance reviewer | yes if API Data is used |
| NFR-OPS-001 | operations runbook review | operations owner | yes |
| NFR-OBS-001 | analytics payload review | analytics reviewer | yes |
| NFR-UX-001 | first-run UX review | product owner | yes |

## 4. Forge Mapping

- Claim IDs: CLM-ON-VPR-003, CLM-ON-VPR-004, CLM-ON-VPR-005, CLM-ON-VPR-006, CLM-ON-VPR-007
- Proof Obligations: PRF-ON-VPR-002, PRF-ON-VPR-003, PRF-ON-VPR-004, PRF-ON-VPR-005
- Human Decision Points: HDP-ON-VPR-002, HDP-ON-VPR-003, HDP-ON-VPR-004, HDP-ON-VPR-005, HDP-ON-VPR-006, HDP-ON-VPR-007, HDP-ON-VPR-008
- ATO Task Graph: TASK-ON-VPR-003, TASK-ON-VPR-004, TASK-ON-VPR-005
- Planned PRs: PR-ON-VPR-003, PR-ON-VPR-004, PR-ON-VPR-005
- Gate Requirements: Privacy Gate, Security Gate, Compliance Gate, Operations Gate
