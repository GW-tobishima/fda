# FDA V1 Release Note (Operational V1)

作成日: 2026-07-09
対象: FDA V1 = Codex CLI primary の AI Delivery Skill Pack / Work Protocol

## 1. リリースサマリ

FDA V1 は **Operational V1 proof complete** として close する。
到達点の定義と証跡は次を正とする。

- 定義: `docs/v1/fda_v1_product_contract.md` / `docs/v1/fda_v1_operational_epic.md`
- 証跡: `docs/standards/delivery-artifacts-v0/examples/fda_v1_operational_e2e/`
  （`end_to_end_receipt.json` status=succeeded、validation 74 passed / 0 failed / 34 skipped）
- 主要 PR: #87 Codex CLI primary rebaseline / #88 Slack Human Decision notifications /
  #89 Operational V1 Slack P0 proof rebaseline（いずれも merge 済み）

## 2. V1 で提供する能力

- CLI 入口: `start / decide / design / plan / implement / review / continue / merge /
  open / status / notify / validate-artifacts`
- Intake → Requirements / NFR / Risk / Human Decision 抽出、未解決 Decision での fail-closed 停止
- Design Gate（basic/detailed design、case graph、task graph、planned PRs、forge projection）
- `.fda/` 7ファイル Repository Profile Gate（不足分のみ生成、既存は上書きしない）
- current AI CLI への implementation handoff / receipt / external PR receipt
- Review Agent Gate（pr_reviewer / functional_qa / security_qa の read-only 分離 receipt）
- Repair Loop（失敗分類、retry 上限、Human Decision への返却）
- Merge Gate（V1 は auto merge しない。human merge approval への handoff。
  `--execute` 時の GitHub merge receipt 証跡）
- Slack Incoming Webhook 通知（fail-closed）、Output Hub / Decision Inbox / Execution Status
- ATO state adapter（`--ato-sync` 明示時のみ書き戻し）、Forge gate adapter
  （ローカル `forge_projection.json` の PromotionDecision 評価）
- 非実装 mode（research / uiux / design-only）

## 3. V1 で意図的にやらないこと

- Human Decision / security High・Critical / merge / release approval の自己承認
- auto merge の常用
- Codex / Claude MCP direct implementer の主経路化（V1.5 optional automation）
- Web UI の主入口化（UI は成果物閲覧・判断確認の projection）
- ATO / Forge を実行ランタイムにすること
- raw stdout / stderr 全文の正本保存

## 4. 既知の残リスク

`docs/v1/fda_v1_residual_risks.md` を参照。

## 5. 次フェーズ

`docs/v1/fda_v1_next_phase_v1_5.md` を参照（Cross-repo Epic Execution Loop、
`continue --epic`、target repo onboarding 再現性、通知/Output Hub 磨き込み、
MCP optional automation、auto merge policy、Web Mission Control）。

## 6. GW-tobishima フォークにおける追補（2026-07-09）

このフォークでは V1 close と同時に、普段使い運用のための次の追補を行った。
上流契約・語彙は変更していない。

- Windows（rustc 1.88+）で `cargo test` / merge gate / gate script が動く
  クロスプラットフォーム修正
- Claude Code を current AI CLI として運用する executor 値の追加
  （`current_claude_code` / `claude_subagent` / `current_ai_cli` / `ai_subagent`）と
  `docs/v1/claude_code_primary_runbook.md`
- CI への Windows job 追加
