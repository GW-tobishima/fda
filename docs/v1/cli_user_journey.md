# FDA V1 CLI User Journey

## 1. 基本方針

FDA V1 の入口は CLI である。Web UI は最初の依頼入口ではなく、成果物閲覧と Human Decision 確認の補助面として扱う。

CLI は、内部 task / run / agent の詳細ではなく、人間が次に判断または確認すべきことを返す。

## 2. Journey 1: Intake

Command:

```bash
fda start "oshi-noteでVTuber紹介リンク/PRページを作りたい"
```

または:

```bash
fda start --input docs/ideas/vtuber_pr_page.md
```

FDA が行うこと:

- 入力を Human Input Spec として保持する。
- 要件定義書を作る。
- 非機能要件を作る。
- リスクを整理する。
- 実装可否を分類する。
- Human Decision を抽出する。
- 判断事項を CLI stdout と Requirements Definition の両方へ出す。
- 次 action を提示する。

想定 stdout:

```text
Requirements Definition を作成しました。

判断が必要です:
1. HD-FDA-001: 通常メモ本文を外部表示しない方針で固定してよいか
2. HD-FDA-002: 集計表示閾値を unique_user_count >= 10 にしてよいか
3. HD-FDA-003: MVPでは YouTube / Twitch API Data を使わない方針でよいか

続行するには:
fda decide HD-FDA-001 --answer yes
fda decide HD-FDA-002 --answer "10 users / 10 records"

成果物:
- requirements_definition.md
- non_functional_requirements.md
- risk_register.md
- human_decision_packet.md
```

## 3. Journey 2: Human Decision

Command:

```bash
fda decide HD-FDA-001 --answer yes
```

FDA が行うこと:

- 回答を decision receipt として記録する。
- ATO decision state へ反映する。
- 関連 artifact の resolved / unresolved を更新する。
- `fda status` と `fda continue` の次 action を更新する。

想定 stdout:

```text
HD-FDA-001 を記録しました。

未解決判断:
- HD-FDA-002
- HD-FDA-003

次:
fda decide HD-FDA-002 --answer <answer>
fda decide HD-FDA-003 --answer <answer>
```

Human Decision は実装の前提条件である。未解決判断がある場合、`fda implement` は実装へ進まず停止理由を返す。

## 4. Journey 3: Status

Command:

```bash
fda status
```

表示するもの:

- 現在 phase
- 未解決 Human Decision
- 次に実行できる command
- gate 状態
- 主要 artifact
- 通知状態
- PR / CI / QA 状態, if available

表示しないもの:

- 通常表示での raw stdout / stderr 全文
- 内部 agent prompt 全文
- Task / Run / Agent の細かい一覧

想定 stdout:

```text
Current phase: Design Gate
State: waiting_for_decision

未解決判断:
- HD-FDA-002: 集計表示閾値

次:
fda decide HD-FDA-002 --answer "10 users / 10 records"

主要成果物:
- requirements_definition.md
- human_decision_packet.md
- output_hub.html
```

## 5. Journey 4: Design

Command:

```bash
fda design
```

FDA が行うこと:

- Human Decision 未解決なら止める。
- Basic Design を作る。
- Detailed Design を作る。
- Case Graph を作る。
- Task Graph を作る。
- Planned PRs を作る。
- Autonomy Contract を作る。
- Forge Projection を作る。
- Functional QA brief と Security QA brief を作る。

想定 stdout:

```text
Design Gate を通過しました。

成果物:
- basic_design.md
- detailed_design.md
- case_graph.json
- task_graph.json
- planned_prs.json
- autonomy_contract.json
- forge_projection.json

次:
fda implement --dry-run
```

判断が必要な場合:

```text
Design Gate で停止しました。

判断が必要です:
- HD-FDA-004: MVPで検索インデックスを作るか

次:
fda decide HD-FDA-004 --answer <answer>
```

## 6. Journey 5: MCP Dry-run

Command:

```bash
fda implement --dry-run
```

FDA が行うこと:

- target repo を確認する。
- `.fda/` Profile Gate を実行する。
- Human Decision Guard を確認する。
- current Codex CLI が implementer へ切り替えるための handoff を作る。
- cwd、prompt、approval policy、禁止事項、timeout を確認する。
- target repo を変更しない。
- MCP dry-run artifact は V1.5 optional automation 互換として残す。

想定 stdout:

```text
Current Codex CLI handoff を生成しました。

確認済み:
- .fda profile: present
- Human Decision guard: clear
- target repo mutation: none

成果物:
- current_codex_cli_handoff.json
- implementation_handoff.md
- mcp_agent_invocation_plan.json
- dry_run_receipt.json

次:
current Codex CLIで current_codex_cli_handoff.json と implementation_handoff.md に従って実装し、PR作成後 fda review
```

## 7. Journey 6: Implement With Current Codex CLI

Command:

```bash
current Codex CLIで current_codex_cli_handoff.json と implementation_handoff.md を読む
```

FDA が行うこと:

- Human Decision と Design Gate を確認する。
- ATO checkpoint を残して Implementer role へ切り替える。
- worktree / branch policy を確認する。
- implementation handoff を配置する。
- test を実行する。
- PR を作る。
- PR URL が回収できない場合は blocked receipt として止める。
- external PR receipt を生成する。

想定 stdout:

```text
実装 PR を作成しました。

PR:
- https://github.com/example/repo/pull/123

検証:
- unit tests: pass
- scope drift: none

成果物:
- current_codex_cli_handoff.json
- implementation_receipt.json
- external_pr_receipt.json

次:
fda review
```

## 8. Journey 7: Review

Command:

```bash
fda review
```

FDA が行うこと:

- PR Reviewer を read-only で起動する。
- Functional QA agent を read-only で起動する。
- Security QA agent を read-only で起動する。
- PR reviewer / QA receipt を分離し、`review_agent_gate.json` と `review_agent_gate_packet.md` に集約する。
- FAIL なら repair loop に戻す。
- security High / Critical なら Human Decision へ戻す。

想定 stdout:

```text
Review を完了しました。

PR Reviewer: PASS
Functional QA: PASS
Security QA: PASS

成果物:
- pr_reviewer_receipt.json
- functional_qa_receipt.json
- security_qa_receipt.json
- review_agent_gate.json
- review_agent_gate_packet.md

次:
fda merge
```

FAIL の場合:

```text
Review は FAIL です。

戻し先:
- implementer

理由:
- AC-003 の検証が未実装

次:
fda continue
```

## 9. Journey 8: Continue

Command:

```bash
fda continue
```

FDA が行うこと:

- current truth を読む。
- decision 待ちなら止まる。
- design 未完なら `design` 相当を進める。
- implementation 待ちなら `implement` 相当を進める。
- QA 待ちなら `review` 相当を進める。
- repair 必要なら同じ thread へ修正依頼する。
- merge 可能なら `merge` へ進める。

`continue` は状態遷移の sugar であり、gate を bypass しない。

## 10. Journey 9: Merge

Command:

```bash
fda merge
```

FDA が行うこと:

- Forge Gate を確認する。
- CI green を確認する。
- QA receipts を確認する。
- open Human Decision がないことを確認する。
- risk policy を確認する。
- V1ではauto mergeせず、merge可能状態をreceiptに残す。
- final merge approval は Human Decision に戻す。

想定 stdout:

```text
Merge Gate を通過しました。V1ではauto mergeせず、Human merge approvalへ戻します。

結果:
- merged: false
- merge_approval_required: true

成果物:
- merge_receipt.json

次:
fda continue
```

## 11. Journey 10: Output Hub

Command:

```bash
fda open
```

FDA が行うこと:

- `output_hub.html` を生成または更新する。
- `decision_inbox.html` を生成または更新する。
- `execution_status.html` を生成または更新する。
- local browser open が使えない環境では path を返す。

想定 stdout:

```text
Output Hub を更新しました。

- output_hub.html
- decision_inbox.html
- execution_status.html
```

## 12. CLI Error Policy

CLI は失敗時に raw stack trace だけを返さない。

必須表示:

- 何が止まったか
- なぜ止まったか
- 人間判断が必要か
- AI repair で進められるか
- 次に実行する command
- 関連 artifact

例:

```text
Implement Gate で停止しました。

理由:
- Human Decision HD-FDA-003 が未解決です。

次:
fda decide HD-FDA-003 --answer <answer>

関連:
- human_decision_packet.md
- requirements_definition.md
```
