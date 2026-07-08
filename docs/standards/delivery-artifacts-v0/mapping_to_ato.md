---
standard_id: delivery-artifacts-v0.mapping-to-ato
version: v0
status: draft
last_reviewed: 2026-06-20
review_cycle_days: 30
owner: forge-delivery-agent
---

# ATO マッピング v0

## ATO の責務

ATO は AI Delivery Organization の状態、要約、次の一手、Human Turn、証跡を保存する Mission Control として扱う。モデル呼び出し、Codex プロセス管理、GitHub PR 作成ロジックは原則 `forge-delivery-agent` 側に置く。

## Task Graph 階層

```text
Program
  Epic
    Case
      Task
        Run
          Evidence
          Summary
          Human Turn
```

## ATO Task metadata

| field | 説明 |
|---|---|
| `parent_type` | `program` / `epic` / `case` |
| `parent_id` | 親 ID |
| `forge.program_id` | Forge Program ID |
| `forge.epic_id` | Forge Epic ID |
| `forge.case_id` | Forge Case ID |
| `forge.claim_ids` | 関連 Claim ID |
| `forge.promotion_state` | `draft` / `repair` / `promote` / `block` |
| `forge.proof_state` | `missing` / `partial` / `complete` |
| `execution.owner_agent` | 担当 role |
| `execution.branch` | 作業ブランチ。実行前の `planned` / `ready_to_work` では未割当として `null` または省略可 |
| `execution.pr_number` | PR 番号 |
| `execution.run_id` | agent run ID |
| `human.decision_required` | 人間判断が必要か |
| `human.decision_packet_id` | Human Decision Packet ID |

## Human Turn

Human Turn は「人間が判断すべきこと」だけに使う。AI が修復できる欠落は Human Turn にしない。

| 種別 | 例 | ATO 表現 |
|---|---|---|
| scope_change | 要件追加、非対象範囲の変更 | Human Decision Packet |
| spec_conflict | 要件間の矛盾 | Human Decision Packet |
| security_exception | High/Critical 例外 | Human Decision Packet |
| release_approval | release target への昇格 | Human Decision Packet |

## AI Repair Lane

| repair reason | 例 | ATO 表現 |
|---|---|---|
| `missing_proof` | AC に対応する証跡がない | repair task |
| `test_not_run` | 必須 test 未実行 | repair task |
| `stale_evidence` | 古い log を参照 | repair task |
| `trace_gap` | Claim と Proof の対応不明 | repair task |
| `review_packet_missing` | PR review packet 欠落 | repair task |

## Summary 更新

各 run は少なくとも次を ATO summary に残す。

- 変更した成果物。
- Scope In/Out の扱い。
- Forge Mapping の判断。
- ATO Task Graph への影響。
- Human Turn の有無。
- AI Repair の有無。
- 検証結果。

## Evidence

| surface | 内容 |
|---|---|
| `file-diff` | 変更ファイル一覧、差分要約 |
| `test-result` | schema validation、Rust test、CI |
| `reference` | 参照した標準、要件、決定 |
| `decision` | 採用した設計判断 |
| `handoff` | 次 agent への引き継ぎ |

## v0 でまだ実装しないもの

- ATO 内でのモデル実行。
- Codex process supervisor。
- GitHub PR 作成。
- UI の完成版。

ただし UI/UX 設計は `uiux_mission_control_design.md` に正本として保持する。
