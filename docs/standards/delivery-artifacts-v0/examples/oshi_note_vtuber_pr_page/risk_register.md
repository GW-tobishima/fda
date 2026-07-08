---
artifact_type: risk_register
version: v0
status: draft
---

# Risk Register: oshi-note VTuber紹介リンク / PRページ

## 0. Metadata

- Risk Register ID: RISK-ON-VPR-6A
- Program ID: PROGRAM-OSHI-VTUBER-PR-001
- Epic ID: EPIC-OSHI-VTUBER-PR-6A
- Status: draft

## 1. Risks

| ID | Risk | Category | Likelihood | Impact | Mitigation | Human Decision / Gate |
|---|---|---|---|---|---|---|
| RISK-001 | 未確認VTuberやなりすましが紹介ページを作る | Trust / Operations | Medium | Critical | 本人確認済みVTuber限定、代理人権限確認、停止受付 | HDP-ON-VPR-001 |
| RISK-002 | 少数母数の集計から個人の視聴・推し活行動が推測される | Privacy | Medium | Critical | unique users / recordsの閾値未満を非表示 | HDP-ON-VPR-002 |
| RISK-003 | 通常メモがVTuberに見えることでプロダクト信頼が毀損する | Privacy / Product Trust | Medium | Critical | 通常メモをVTuber向けAPI、dashboard、card生成から除外 | HDP-ON-VPR-003 |
| RISK-004 | 公開用コメントが通常メモと混同され、同意のない外部表示になる | Privacy / UX | Medium | High | 別入力、明示opt-in、削除導線、利用範囲表示 | HDP-ON-VPR-004 |
| RISK-005 | コメントに個人情報、誹謗中傷、権利侵害表現が混ざる | Moderation | Medium | High | 自動フィルタ、人間レビュー、MVPではコメント非表示 option | HDP-ON-VPR-005 |
| RISK-006 | 案件性がある紹介投稿でPR表記が不足する | Compliance | Medium | Critical | 投稿カード / SNSテンプレへPR / 提供表記を標準化 | HDP-ON-VPR-006 |
| RISK-007 | 公式イラスト、ロゴ、サムネイルの無許諾利用になる | IP / Brand | Medium | Critical | VTuber提供または許諾済み素材だけ使用、許諾範囲を記録 | HDP-ON-VPR-007 |
| RISK-008 | YouTube / Twitch API Dataの保持・表示要件に違反する | Compliance / API Terms | Low | High | MVPはAPI Dataなし、使う場合はrefresh / delete / attribution設計 | HDP-ON-VPR-008 |
| RISK-009 | 推し色がVTuber設定とファン側表示でずれ、紹介体験が壊れる | UX / Data Integrity | Medium | Medium | campaign設定値を初期推し登録へ反映するACと回帰テストを置く | PR-ON-VPR-002 Gate |
| RISK-010 | Stripe / GA4 / Ads metadataに推し名、URL、コメント本文が漏れる | Privacy / Analytics | Low | High | 低カーディナリティmetadataのみ許可しpayload reviewを必須化 | PR-ON-VPR-006 Gate |

## 2. Highest-risk Epic Slices

1. 本人確認、素材許諾、PR表示、API Data方針を未決のまま実装に入ること。
2. 通常メモや低母数集計がVTuber側に出ること。
3. 公開用コメントを通常メモの延長として扱うこと。
4. 推し色反映をUIだけで済ませ、campaign attribution / initial favorite stateの整合性を検証しないこと。

## 3. Risk Posture For PoC-6A

- Planning-onlyのため、リスクは設計GateとHuman Decisionへ分離する。
- 後続実装はHuman Decision Packet解決後に開始する。
- ATO CLI materializationは計画とコマンドdraftのみ残し、実行しない。
