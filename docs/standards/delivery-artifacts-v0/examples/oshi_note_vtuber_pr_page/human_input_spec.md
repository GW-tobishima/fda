---
artifact_type: human_input_spec
version: v0
status: draft
---

# PoC-6A Human Input Spec: oshi-note VTuber紹介リンク / PRページ

## 0. Metadata

- Document ID: HIS-ON-VPR-6A
- Program ID: PROGRAM-OSHI-VTUBER-PR-001
- Epic ID: EPIC-OSHI-VTUBER-PR-6A
- Owner: forge-delivery-agent
- Status: draft
- Created At: 2026-06-25 JST
- Planning Mode: Planning-only

## 1. Input Sources

- `/root/code/oshi-note/artifacts/reports/vtuber-pr-page-feasibility-20260620.md`
- `/root/code/oshi-note/artifacts/designs/vtuber-referral-monetization-flow-20260624.excalidraw`
- 追加ユーザー入力: VTuber が宣伝用登録時に推しの色を選択でき、宣伝URLから登録したファンの初期推しがその色になること。

## 2. User Request Summary

oshi-note の次PoCとして、VTuber紹介リンク / PRページ機能を High-risk Epic Planning として分解する。実装、GitHub PR作成、ATO CLI実行、Forge実評価は行わず、標準成果物 example を追加する。

## 3. Required Product Behaviors

- 本人確認済み VTuber / 事務所アカウントだけが紹介URL / PRページを発行できる。
- VTuber は宣伝用登録またはキャンペーン設定で推し色を選択できる。
- 紹介URL経由で登録したファンは、対象VTuberが初期推しとして登録され、VTuberが選択した推し色が初期表示に反映される。
- VTuber側に表示する成果は匿名集計に限定する。
- 少数母数の集計は非表示にする。
- 通常メモ本文、個人ID、メールアドレス、自由入力URL、raw log は VTuber に出さない。
- 公開用コメントは通常メモと分離し、明示 opt-in で収集する。
- コメントを投稿カードやVTuber向け画面に出す場合は moderation 方針を決める。
- 案件性がある投稿カード、SNS投稿テンプレ、PRページには PR / 提供表記を入れる。
- 公式素材、ロゴ、サムネイル等は許諾範囲を記録してから使用する。
- YouTube / Twitch API Data を使う場合は、保持、refresh、表示要件、公式統計との混同防止を設計する。

## 4. Hard Constraints For This PoC

- oshi-note repo の実装コードは変更しない。
- GitHub PR は作成しない。
- ATO CLI は実行しない。
- Forge の実評価は実行しない。
- 成果物は `docs/standards/delivery-artifacts-v0/examples/oshi_note_vtuber_pr_page/` に追加する。

## 5. Human Decisions Required

| ID | Decision | Why Human-owned | Required Before | Recommended Default For Planning |
|---|---|---|---|---|
| HDP-ON-VPR-001 | 本人確認済みVTuber限定にするか | なりすまし、代理人権限、事務所確認を含む運用判断 | 申請 / URL発行実装 | 限定する |
| HDP-ON-VPR-002 | 少数母数非表示閾値 | プライバシーと事業KPIのバランス判断 | 集計 / dashboard実装 | unique users 10未満またはrecords 10未満は非表示 |
| HDP-ON-VPR-003 | 通常メモ非公開 | プロダクト信頼とプライバシー判断 | VTuber dashboard / card実装 | 非公開を固定 |
| HDP-ON-VPR-004 | 公開用コメントのopt-in | ユーザー同意と表示範囲の判断 | first log / comment UI実装 | 明示opt-inのみ |
| HDP-ON-VPR-005 | moderation要否 | 運用負荷、炎上リスク、投稿前責任の判断 | 公開コメント表示前 | 自動フィルタ + 人間レビュー |
| HDP-ON-VPR-006 | PR表示 | ステルスマーケティング対応方針 | SNSテンプレ / card生成前 | 案件性がある場合は削除不可のPR表示 |
| HDP-ON-VPR-007 | 素材許諾 | 著作権、商標、パブリシティ、事務所規約の判断 | PRページ / cardで素材使用前 | VTuber提供または許諾済み素材のみ |
| HDP-ON-VPR-008 | YouTube/Twitch API Data利用有無 | API規約、保持期限、公式統計との誤認防止判断 | 動画名 / サムネイル / ranking表示前 | MVPはAPI Dataなし、必要なら後段Decision |

## 6. Planning Assumptions

- MVPはファン側Premium転換を優先し、Creator Pro / 紹介報酬は後段に分離する。
- 申請 / URL発行は初期PoCでは手動運用でもよい。
- 計測metadataは低カーディナリティに限定し、推し名、メモ本文、URL、コメント本文を外部広告 / 決済metadataへ送らない。
- 推し色はVTuber / campaign側の設定値として扱い、ファン登録時の初期推し表示へコピーまたは参照される。

## 7. Non-goals

- oshi-note 実装コード変更。
- DB migration作成。
- GitHub Issue / PR作成。
- ATO stateへの書き込み。
- Forge promotion判定。
- 法務最終判断。
