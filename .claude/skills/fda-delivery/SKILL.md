---
name: fda-delivery
description: >-
  FDA (forge-delivery-agent) の Delivery Work Protocol を Claude Code セッション
  （current AI CLI）として実行するための手順。FDA 対象 repo で「依頼を受けて
  要件定義から PR / merge handoff まで進める」「fda start/design/implement/review/
  merge を回す」「Implementer に role switch する」「Review Agent Gate を通す」
  場面で必ず使う。
---

# FDA Delivery Work Protocol (Claude Code)

Claude Code はこの repo の「current AI CLI」= Orchestrator 兼 Implementer である。
正本: `docs/v1/claude_code_primary_runbook.md` / `docs/v1/codex_cli_primary_architecture.md`。

## 0. 開始前チェック（毎回）

1. ATO task/run を開始する: `ato work begin --new --title "<依頼>" --role implementer --json`
2. Profile Gate: 対象 repo に `.fda/` 7ファイル（repo/delivery_policy/skills.lock/
   agent_roles/gates/artifact_map/notification）があるか確認。無ければ FDA コマンドが
   不足分だけ生成する（既存は上書きしない）。target repo が存在しない場合は偽 repo を
   作らず人間に戻す。
3. 未解決 Human Decision が無いか `fda status --artifacts <run_dir>` で確認。

## 1. ステージの回し方

各コマンドに `--ato-sync --ato-task <key> --ato-run-id <run>` を付けて ATO へ書き戻す。

| ステージ | コマンド | Claude Code の責務 |
|---|---|---|
| Intake | `fda start "<依頼>"` | 生成された Human Decision を人間に提示し、自己承認しない |
| Decision | `fda decide <ID> --answer <回答>` | 回答は人間から得る。`--decided-by human` が既定 |
| Design | `fda design --artifacts <run_dir>` | blocked なら decision へ戻す |
| Handoff | `fda implement --dry-run --target-repo <path>` | target repo を変更しない |
| 実装 | （コマンドなし） | role switch: ATO checkpoint を残し、`current_codex_cli_handoff.json` と `implementation_handoff.md` の Scope In/Out・forbidden changes・test command に従い実装・テスト・PR 作成。`external_pr_receipt.json` を残す |
| Review | `fda review --artifacts <run_dir>` | pr_reviewer / functional_qa / security_qa を **read-only subagent** で分離実行。receipt を混ぜない |
| Repair | `fda continue --artifacts <run_dir>` | `repair_prompt.md` に従い同一スレッドで修正。retry 上限到達時は Human Decision へ |
| Merge | `fda merge --artifacts <run_dir>` | V1 は auto merge しない。merge approval は人間に戻す |
| 閲覧 | `fda open` / `fda status` | 成果物と判断待ちを人間に提示する |

## 2. Review Agent Gate（PR 必須証跡）

1. `fda review` の `review_agent_gate_packet.md` を
   `artifacts/review_packets/pr-<PR番号>.md` へ反映する（自動反映されない）。
2. `python3 scripts/check_review_agent_gate.py --pr-number <PR番号>` を通す
   （python3 が無ければ `python` / `py -3`）。
3. ATO / Forge / FDA 証跡・handoff・human decision 境界に触れる PR は
   `forge_reviewer`（無ければ `qax2` 代替 + 理由記録）、UI に触れる PR は `design_qa`。
   該当しない場合も `design_qa: not_applicable` と理由を残す。
4. `REVIEW_AGENT_OK` は merge approval ではない。

## 3. 禁止事項（fail-closed）

- Human Decision（scope / privacy / legal / security High・Critical / risk / merge /
  release）の自己承認。
- 未解決 Human Decision を実装で埋めること。
- reviewer / QA subagent による source mutation。
- raw stdout/stderr 全文を正本として保存すること（receipt は要約 + verdict）。
- `.fda/` の既存ファイル上書き。

## 4. Forge / ATO gate（merge 前）

- `ato case evaluate --task <key> --no-write --json` で PromotionDecision を試算し、
  verdict / policy_version / missing_proofs を checkpoint に残す。
  `promote` でも merge approval ではない。
- `fda merge` の Forge gate はローカル `forge_projection.json` を評価する。
  hold / blocked のまま merge に進まない。
