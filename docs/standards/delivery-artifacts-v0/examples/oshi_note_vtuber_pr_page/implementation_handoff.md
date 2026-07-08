---
artifact: implementation_handoff
program_id: PROGRAM-OSHI-VTUBER-PR-001
epic_id: EPIC-OSHI-VTUBER-PR-6A
planned_pr_id: PR-ON-VPR-001
target_repo: oshi-note
status: ready_for_external_implementation
---

# oshi-note VTuber紹介リンク / PRページ 実装handoff

## 目的

このhandoffは、`oshi-note` repoで最初に実装する `PR-ON-VPR-001: VTuber partner intake, verification, and permission policy` の実装入力です。

`forge-delivery-agent` 側ではplanning artifactだけを管理し、`oshi-note` の実装コードは変更しません。

## Target Repo

- target repo: `oshi-note`
- local path: `/root/code/oshi-note`
- source artifact root: `/root/code/forge-delivery-agent/docs/standards/delivery-artifacts-v0/examples/oshi_note_vtuber_pr_page/`
- target planned PR: `PR-ON-VPR-001`

## 参照順

1. `implementation_handoff.md`
2. `planned_pr_execution_packet.json`
3. `requirements_definition.md`
4. `planned_prs.json`
5. `human_decision_packet.json`
6. `risk_register.md`
7. `human_decisions/*.json`

元入力として必要な場合は以下も参照します。

- `/root/code/oshi-note/artifacts/reports/vtuber-pr-page-feasibility-20260620.md`
- `/root/code/oshi-note/artifacts/designs/vtuber-referral-monetization-flow-20260624.excalidraw`

## 解決済みHuman Decisions

| ID | 決定 | 実装指示 |
|---|---|---|
| HDP-ON-VPR-001 | 本人確認済みVTuberのみ | 未確認、申請中、代理権限未確認のアカウントには紹介URLを発行しない。 |
| HDP-ON-VPR-002 | `unique users < 10` または `records < 10` は非表示 | 集計dashboardやランキングを作る場合のblocking requirementとして残す。 |
| HDP-ON-VPR-003 | メモは常に非公開 | 通常メモ本文をVTuber、公開ページ、dashboard、投稿カードへ出さない。 |
| HDP-ON-VPR-004 | コメントはMVPでは作成しない | 公開用コメント入力、保存、表示をMVP scope outにする。 |
| HDP-ON-VPR-005 | 後で考える | MVPでは公開コメントを作らないためmoderation実装は不要。コメント機能開始前に再判断する。 |
| HDP-ON-VPR-006 | PR表示する | 案件性がある表示面ではPR表示を消せない前提にする。 |
| HDP-ON-VPR-007 | VTuber提供または許諾済み素材のみ使用する | 素材許諾記録がない素材をPRページ、カード、テンプレートに出さない。 |
| HDP-ON-VPR-008 | MVPではYouTube / Twitch API Dataを使わない | API Dataを動画名、サムネイル、ランキング、投稿カードに使わない。 |

## Scope In

- VTuber partner application / intake policy
- 本人確認済みVTuber限定の紹介URL発行policy
- 代理人権限確認policy
- 素材許諾記録policy
- PR表示policy
- 停止、削除、表示名変更の受付runbook
- terms / privacy / ops docsの必要最小更新

## Scope Out

- 紹介ページ実装
- 宣伝用登録時の推し色選択UI実装
- 紹介URL経由登録後の初期推し登録実装
- creator dashboard実装
- 公開コメント作成、保存、表示、moderation実装
- YouTube / Twitch API Data連携
- 自動本人確認provider連携
- 決済、成果報酬、Premium CTA実装

## Files To Change Candidates

oshi-note側の実際の構成に合わせて調整してください。

- `docs/spec/vtuber-referral-pr-page.md`
- `docs/spec/vtuber-partner-verification.md`
- `docs/spec/vtuber-material-permission.md`
- `docs/spec/vtuber-pr-disclosure.md`
- `docs/runbooks/vtuber-partner-intake.md`
- `templates/terms.html`
- `templates/privacy.html`

## Acceptance Criteria

- 未確認VTuberは紹介URLを発行できない方針が明記されている。
- 素材許諾範囲が未記録の素材はPRページやカードで使えない方針が明記されている。
- 停止、削除、表示名変更の受付導線が定義されている。
- 公開コメントはMVP scope outであり、moderationは将来判断として明記されている。
- YouTube / Twitch API DataはMVP scope outであることが明記されている。
- 案件性がある表示面ではPR表示する方針が明記されている。

## Security / Privacy / Legal Checks

- 本人確認済みVTuber以外への発行禁止がpolicyに入っていること。
- 通常メモ本文の非公開がpolicyに入っていること。
- 少数母数非表示閾値が将来dashboard scopeのblocking requirementとして残っていること。
- 素材許諾記録なしの素材利用禁止がpolicyに入っていること。
- PR表示方針がterms/privacyまたは関連specへ反映されていること。
- YouTube / Twitch API DataをMVPで使わないことが明記されていること。

## Rollback / No-op Plan

- docs/specやrunbookのみの変更なら、該当PRをrevertしてもruntime影響はない。
- terms/privacy templateを変更した場合は、表示文言の差し戻しPRで元に戻す。
- DB migrationやproduction config変更はこのplanned PRでは行わない。

## Evidence Expected From oshi-note PR

oshi-note側PR完了時は、以下を返してください。

- `external_pr_receipt.json`
- `evidence_return_packet.json`
- target PR URL
- target commit SHA
- changed file list
- acceptance criteria mapping
- validation commands and results
- security/privacy/legal self-review
- rollback/no-op statement

返却schemaは以下を参照します。

- `/root/code/forge-delivery-agent/docs/standards/delivery-artifacts-v0/schemas/external_pr_receipt.schema.json`
- `/root/code/forge-delivery-agent/docs/standards/delivery-artifacts-v0/schemas/evidence_return_packet.schema.json`
