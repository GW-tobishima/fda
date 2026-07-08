---
artifact_type: ato_cli_commands
version: v0
status: draft
---

# ATO CLI Commands Draft

このファイルは後続実行用のdry-run draftであり、このPRでは実行しない。機械可読な正本は `ato_cli_materialization_plan.json`。

## 0. Execution Boundary

- `actual_cli_execution=false`
- `ato_db_change=false`
- `mcp_usage=false`
- 人間が明示承認するまで、以下のコマンドはコピー実行しない。
- `<RUN_ID:...>` は対応する `ato work begin` の返却値で置換する。

## 1. Begin Program / Epic / Decision Tasks

```bash
ato work begin --task PROGRAM-ON-VPR-001 --title "oshi-note VTuber Referral / PR Page" --agent-id codex --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key on-vpr-program-001 --json
ato work begin --task EPIC-ON-VPR-6A --title "PoC-6A oshi-note VTuber PR page high-risk epic planning" --agent-id codex --role orchestrator --capability-profile planning --workspace-policy read_only --idempotency-key on-vpr-epic-6a --json
ato work begin --task TASK-ON-VPR-001 --title "Resolve verified creator and material permission decisions" --agent-id codex --role human_liaison --capability-profile planning --workspace-policy read_only --idempotency-key on-vpr-task-001 --json
ato work begin --task TASK-ON-VPR-003 --title "Resolve normal memo privacy and public comment decisions" --agent-id codex --role human_liaison --capability-profile planning --workspace-policy read_only --idempotency-key on-vpr-task-003 --json
ato work begin --task TASK-ON-VPR-004 --title "Resolve low-count aggregation threshold decision" --agent-id codex --role human_liaison --capability-profile planning --workspace-policy read_only --idempotency-key on-vpr-task-004 --json
ato work begin --task TASK-ON-VPR-005 --title "Resolve PR disclosure material permission and API Data decisions" --agent-id codex --role human_liaison --capability-profile planning --workspace-policy read_only --idempotency-key on-vpr-task-005 --json
```

## 2. Individual Human Decision Work Blocks

### HDP-ON-VPR-001 本人確認済みVTuber限定

```bash
ato work block --task TASK-ON-VPR-001 --run-id "<RUN_ID:TASK-ON-VPR-001>" --reason spec_decision --title "HDP-ON-VPR-001 本人確認済みVTuber限定" --question "紹介ページ / 宣伝URLを本人確認済みVTuberまたは事務所アカウントだけに発行するか。" --recommended-option "本人確認済みVTuber / 事務所アカウントに限定する" --option "本人確認済みVTuber / 事務所アカウントに限定する" --option "申請後審査中の限定公開を許す" --json
```

### HDP-ON-VPR-002 少数母数非表示閾値

```bash
ato work block --task TASK-ON-VPR-004 --run-id "<RUN_ID:TASK-ON-VPR-004>" --reason risk_approval --title "HDP-ON-VPR-002 少数母数非表示閾値" --question "VTuber dashboardやランキングで少数母数を非表示にする閾値を決める。" --recommended-option "unique users 10未満またはrecords 10未満は非表示" --option "unique users 10未満またはrecords 10未満は非表示" --option "unique users 5未満を非表示" --option "項目ごとに別閾値を設定する" --json
```

### HDP-ON-VPR-003 通常メモ非公開

```bash
ato work block --task TASK-ON-VPR-003 --run-id "<RUN_ID:TASK-ON-VPR-003>" --reason risk_approval --title "HDP-ON-VPR-003 通常メモ非公開" --question "通常メモ本文をVTuber側、投稿カード、集計dashboardへ出さない方針を固定するか。" --recommended-option "通常メモは常に非公開" --option "通常メモは常に非公開" --option "個別明示同意がある場合だけ一部公開" --json
```

### HDP-ON-VPR-004 公開用コメントのopt-in

```bash
ato work block --task TASK-ON-VPR-003 --run-id "<RUN_ID:TASK-ON-VPR-003>" --reason spec_decision --title "HDP-ON-VPR-004 公開用コメントのopt-in" --question "公開用コメントを通常メモと分け、どのopt-in方式で収集するか。" --recommended-option "通常メモとは別入力 + 明示チェック" --option "通常メモとは別入力 + 明示チェック" --option "通常メモから公開用コメントへ転記確認" --json
```

### HDP-ON-VPR-005 moderation要否

```bash
ato work block --task TASK-ON-VPR-003 --run-id "<RUN_ID:TASK-ON-VPR-003>" --reason risk_approval --title "HDP-ON-VPR-005 moderation要否" --question "公開用コメントをVTuber側や投稿カードに表示する前に必要なmoderation方式を決める。" --recommended-option "自動フィルタ + 人間レビュー" --option "自動フィルタ + 人間レビュー" --option "自動フィルタのみ" --option "MVPではコメント表示しない" --json
```

### HDP-ON-VPR-006 PR表示

```bash
ato work block --task TASK-ON-VPR-005 --run-id "<RUN_ID:TASK-ON-VPR-005>" --reason risk_approval --title "HDP-ON-VPR-006 PR表示" --question "投稿カードとSNS投稿テンプレにおけるPR / 提供表記の範囲と削除可否を決める。" --recommended-option "案件性がある場合は削除不可のPR表示" --option "案件性がある場合は削除不可のPR表示" --option "SNSテンプレで警告するが削除可能" --option "全紹介投稿にPR表示" --json
```

### HDP-ON-VPR-007 素材許諾

```bash
ato work block --task TASK-ON-VPR-005 --run-id "<RUN_ID:TASK-ON-VPR-005>" --reason risk_approval --title "HDP-ON-VPR-007 素材許諾" --question "PRページや投稿カードで使う公式素材、ロゴ、サムネイルの許諾範囲を決める。" --recommended-option "VTuber提供または許諾済み素材のみ" --option "VTuber提供または許諾済み素材のみ" --option "公式ガイドライン許可範囲の素材も許す" --option "MVPは素材なし" --json
```

### HDP-ON-VPR-008 YouTube/Twitch API Data利用有無

```bash
ato work block --task TASK-ON-VPR-005 --run-id "<RUN_ID:TASK-ON-VPR-005>" --reason risk_approval --title "HDP-ON-VPR-008 YouTube/Twitch API Data利用有無" --question "動画名、サムネイル、ランキング、投稿カードでYouTube / Twitch API Dataを使うか。" --recommended-option "MVPではAPI Dataを使わない" --option "MVPではAPI Dataを使わない" --option "refresh / delete方針込みで使う" --option "VTuber提供素材だけ使う" --json
```

## 3. Evidence Checkpoint Draft

```bash
ato work checkpoint --task EPIC-ON-VPR-6A --run-id "<RUN_ID:EPIC-ON-VPR-6A>" --kind progress --summary "Prepared ATO CLI materialization dry-run plan with eight individual Human Decision work block commands. No ATO CLI command was executed." --evidence-surface file --evidence-id docs/standards/delivery-artifacts-v0/examples/oshi_note_vtuber_pr_page/ato_cli_materialization_plan.json --evidence-verdict pass --validation-status passed --freshness current --durability-class artifact --trust-level self_reported --json
```

## 4. Still Not Executed

- `ato work begin`
- `ato work block`
- `ato work checkpoint`
- `ato decisions answer`
- `ato decisions apply`
