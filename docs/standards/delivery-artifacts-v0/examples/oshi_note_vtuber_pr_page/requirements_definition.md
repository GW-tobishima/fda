---
artifact_type: requirements_definition
version: v0
status: draft
---

# PoC-6A 要件定義: oshi-note VTuber紹介リンク / PRページ

## 0. Metadata

- Document ID: REQ-ON-VPR-6A
- Version: v0
- Owner: forge-delivery-agent
- Status: draft
- Source: VTuber紹介リンク / PRページ 実現性検討レポート 2026-06-20, referral monetization flow 2026-06-24, user-added 推し色 requirement
- Related Program: PROGRAM-OSHI-VTUBER-PR-001
- Related Epic: EPIC-OSHI-VTUBER-PR-6A

## 1. Business Objective

- Problem: VTuber紹介ページからファン登録、初回記録、Premium転換へつなげたいが、ファンの記録やコメントをVTuber側へ見せる設計にはプライバシー、本人確認、権利、PR表記、API規約の高リスク判断がある。
- Desired Outcome: 実装前に高リスクEpicをCase、Claim、Task、Planned PR、人間判断、証跡義務へ分解し、AIが安全に後続実装へ進める計画を作る。
- Success Metrics:
  - 必須Human Decisions 8件が成果物に明示されている。
  - 推し色選択から紹介URL経由初期推し登録までの要求がFR/Case/Task/PRに追跡できる。
  - 通常メモ非公開、少数母数非表示、opt-inコメント、PR表示、素材許諾、API Data利用可否がGate化されている。
  - Planning-only制約により oshi-note 実装コード、GitHub PR、ATO CLI、Forge実評価が変更されない。
- Non-goals:
  - 実装コード、DB migration、UI実装、外部API連携の作成。
  - GitHub Issue / PR作成。
  - ATO CLI実行またはForge実評価。
  - 法務判断の最終確定。

## 2. Scope

### Scope In

- VTuber紹介リンク / PRページのEpic分解。
- VTuber本人確認、素材許諾、PR表示、API Data利用可否の判断点定義。
- 宣伝用登録時の推し色選択と、紹介URL経由登録時の初期推し色反映要求。
- ファン側登録、初回記録、公開用コメント、Premium CTA、匿名集計dashboardの境界設計。
- Case Graph、Task Graph、Planned PR、ATO CLI materialization draft、validation artifact。

### Scope Out

- oshi-note repoの実装。
- GitHub PR作成。
- ATO CLI実行。
- Forge実評価。
- VTuber本人確認の実運用実施。
- YouTube / Twitch APIの実呼び出し。

## 3. Stakeholders / Users

- Primary users: ファン, 本人確認済みVTuber / 事務所担当者
- Operators: oshi-note運営, moderation担当, campaign運用担当
- Reviewers: product owner, privacy reviewer, legal / compliance reviewer, security QA
- Approvers: human product owner, legal / compliance owner

## 4. Functional Requirements

| ID | Requirement | Rationale | Priority | Acceptance Criteria | Source |
|---|---|---|---|---|---|
| FR-001 | VTuber / 事務所は紹介ページ発行前に本人確認済み状態になっている | なりすましと代理人権限リスクを下げる | Must | 未確認VTuberでは紹介URL発行CaseがGateで止まる | feasibility report |
| FR-002 | VTuberは宣伝用登録またはcampaign設定で推し色を選択できる | 紹介URL経由登録時の初期体験をVTuberらしくする | Must | campaignに推し色が保存され、紹介ページと初期推し登録に同じ色が反映される | user-added requirement |
| FR-003 | 紹介URL経由で登録したファンは対象VTuberが初期推しとして登録される | 紹介ページの価値をfirst logへ接続する | Must | referral_campaign_idからcreator_idが解決され、初期推し登録済みで開始できる | feasibility report / design |
| FR-004 | 紹介ページでは匿名集計利用を登録前に説明する | ファンの同意期待と信頼を守る | Must | 登録CTA前に匿名集計利用説明が表示される | feasibility report |
| FR-005 | 通常メモ本文はVTuber側、投稿カード、集計dashboardへ出さない | 自分用メモの心理的安全性を守る | Must | VTuber向けquery / API / card生成に通常メモ本文が含まれない | feasibility report |
| FR-006 | 公開用コメントは通常メモと分離し、明示opt-inとして任意入力にする | 通常メモの意図外利用を避ける | Must | opt-inなしのコメントはVTuber側表示候補にならない | feasibility report / design |
| FR-007 | 公開用コメントは表示前にmoderation方針を通す | 個人情報、誹謗中傷、炎上リスクを下げる | Should | moderation未完了コメントは投稿カードへ入らない | feasibility report |
| FR-008 | VTuber dashboardは匿名集計だけを表示し、少数母数を非表示にする | 個人推測リスクを下げる | Must | unique_user_countやrecord_countが閾値未満のbucketは表示されない | feasibility report |
| FR-009 | 投稿カードとSNS投稿テンプレは対象期間、独自集計注記、公式統計ではない注記、必要なPR表記を含む | 誤認とステマリスクを下げる | Must | card / templateの必須表示要素がGateで検証される | feasibility report |
| FR-010 | 公式素材、ロゴ、配信サムネイル等は許諾範囲が記録されたものだけ使う | 著作権、商標、パブリシティリスクを下げる | Must | 素材許諾statusなしではPRページ / card素材として使えない | feasibility report |
| FR-011 | YouTube / Twitch API Dataを使う場合は保持、refresh、表示要件、公式統計との分離を設計する | API規約違反と誤認表示を避ける | Should | API Data使用有無がHuman Decisionとして解決されるまで該当Caseは実装不可 | feasibility report |
| FR-012 | first log後にPremium CTAを出し、推し追加枠、月次レポート、共有カードの価値へ接続する | 無料紹介ページからPremium転換を検証する | Should | creator_page_viewからsubscription_createdまでの低カーディナリティKPIが定義される | design |

## 5. Non-Functional Requirements

| ID | Quality Attribute | Requirement | Measure | Target | Verification |
|---|---|---|---|---|---|
| NFR-PRIV-001 | Privacy | VTuber側への個人対応データ提供を避ける | exposed direct identifiers | 0 | privacy review |
| NFR-SEC-001 | Security | VTuber dashboardからraw logや個人IDを取得できない | High/Critical finding | 0 | security QA |
| NFR-COMP-001 | Compliance | PR / 提供表記と独自集計注記を標準化する | card/template required label coverage | 100% | compliance review |
| NFR-OPS-001 | Operations | 本人確認、素材許諾、停止受付が運用できる | required manual workflow present | 100% | operations review |
| NFR-OBS-001 | Observability | funnel KPIを低カーディナリティで追跡する | forbidden metadata leak | 0 | analytics review |

## 6. Constraints

- Technical: Planning-onlyのため実装コード、DB migration、API連携は作らない。
- Legal / compliance: 法務最終判断はHuman Decisionに残す。
- Operational: 本人確認、素材許諾、moderationは運用負荷を含めて人間判断に戻す。
- Schedule: このPoCでは成果物追加と検証のみ。
- Dependencies: oshi-note側既存仕様、privacy/terms、YouTube API Data retention方針。

## 7. Assumptions

- 申請 / URL発行はMVP初期では手動運用を許容する。
- Creator Pro / 紹介報酬はtraction後の別設計に分離する。
- 推し色はcampaignまたはcreator profileに紐づく設定で、紹介URL経由登録時の初期推しへ反映する。
- VTuberに出す成果は匿名集計であり、個別ファン単位の提供はしない。

## 8. Open Questions

| ID | Question | Owner | Blocking? | Due |
|---|---|---|---|---|
| Q-001 | 少数母数非表示閾値を unique users 10未満 / records 10未満で固定するか | human product owner / privacy reviewer | yes | aggregation implementation |
| Q-002 | 公開用コメントのmoderationを自動フィルタのみ、または人間レビュー必須にするか | operations owner | yes | comment display implementation |
| Q-003 | MVPでYouTube / Twitch API Dataを使うか | product owner / compliance reviewer | yes | video ranking / thumbnail implementation |
| Q-004 | PR表示を削除不可にする範囲を有償案件だけにするか、全紹介投稿にするか | legal / compliance owner | yes | card/template implementation |

## 9. Human Decision Points

| ID | Trigger | Decision Needed | Options | Required Before |
|---|---|---|---|---|
| HDP-ON-VPR-001 | 紹介ページ発行に入る前 | 本人確認済みVTuber限定にするか | 限定する, 申請後審査中も限定公開を許す | PR-ON-VPR-001 |
| HDP-ON-VPR-002 | 集計dashboard設計前 | 少数母数非表示閾値 | users 10未満, records 10未満, 両方満たすまで非表示 | PR-ON-VPR-004 |
| HDP-ON-VPR-003 | VTuber dashboard API設計前 | 通常メモ非公開を固定するか | 固定非公開, 個別同意で一部公開 | PR-ON-VPR-003 |
| HDP-ON-VPR-004 | first log UI設計前 | 公開用コメントのopt-in方法 | 明示チェック + 別入力, 通常メモから転記確認 | PR-ON-VPR-003 |
| HDP-ON-VPR-005 | コメント表示前 | moderation要否 | 自動 + 人間レビュー, 自動のみ, MVPではコメント非表示 | PR-ON-VPR-003 |
| HDP-ON-VPR-006 | 投稿カード生成前 | PR表示 | 案件は削除不可, テンプレ警告のみ, 全紹介投稿にPR | PR-ON-VPR-005 |
| HDP-ON-VPR-007 | 素材アップロード前 | 素材許諾 | 許諾済み素材のみ, VTuber提供素材のみ, APIサムネイル許可 | PR-ON-VPR-005 |
| HDP-ON-VPR-008 | 動画名 / サムネイル表示前 | YouTube/Twitch API Data利用有無 | MVPでは使わない, refresh前提で使う, VTuber提供素材に限定 | PR-ON-VPR-005 |

### 2026-06-25 MVP Decision Record

| ID | Decision |
|---|---|
| HDP-ON-VPR-001 | 本人確認済みVTuberのみ |
| HDP-ON-VPR-002 | `unique users < 10` または `records < 10` は非表示 |
| HDP-ON-VPR-003 | メモは常に非公開 |
| HDP-ON-VPR-004 | コメントはMVPでは作成しない |
| HDP-ON-VPR-005 | moderationは後で考える。MVPでは公開コメントを作らず、コメント機能開始前に再判断する |
| HDP-ON-VPR-006 | PR表示する |
| HDP-ON-VPR-007 | VTuber提供または許諾済み素材のみ使用する |
| HDP-ON-VPR-008 | MVPではYouTube / Twitch API Dataを使わない |

## 10. Forge Mapping

- Claim IDs: CLM-ON-VPR-001, CLM-ON-VPR-002, CLM-ON-VPR-003, CLM-ON-VPR-004, CLM-ON-VPR-005, CLM-ON-VPR-006, CLM-ON-VPR-007
- Proof Obligations: PRF-ON-VPR-001, PRF-ON-VPR-002, PRF-ON-VPR-003, PRF-ON-VPR-004, PRF-ON-VPR-005
- Human Decision Points: HDP-ON-VPR-001, HDP-ON-VPR-002, HDP-ON-VPR-003, HDP-ON-VPR-004, HDP-ON-VPR-005, HDP-ON-VPR-006, HDP-ON-VPR-007, HDP-ON-VPR-008
- ATO Task Graph: TASK-ON-VPR-001, TASK-ON-VPR-002, TASK-ON-VPR-003, TASK-ON-VPR-004, TASK-ON-VPR-005, TASK-ON-VPR-006
- Planned PRs: PR-ON-VPR-001, PR-ON-VPR-002, PR-ON-VPR-003, PR-ON-VPR-004, PR-ON-VPR-005, PR-ON-VPR-006
- Gate Requirements: Human Decision Gate, Privacy Gate, Compliance Gate, Material Permission Gate, Planning-only Gate
