# Requirements Definition

## 1. 入力

- 入力元: `cli_goal`
- 入力要約: FDAの普段使い検証: READMEにClaude Code運用の注意書きを1行追加したい

## 2. 実装可否分類

- 分類: `implementation_candidate`
- mode: `implement`
- 理由: Human Decision 解決後に Design Gate へ進める実装候補として扱う。
- 次 gate: `Design Gate`

## 3. Scope In

- 入力された目的を FDA V1 の Intake artifact に変換する。
- 要件定義、非機能要件、リスク、Human Decision を生成する。
- Human Decision を CLI stdout とこの要件定義書の両方に記録する。

## 4. Scope Out

- この dry-run では target repo の実装、PR 作成、merge は行わない。
- Human Decision の回答適用、Design Gate、MCP agent invocation は後続 PR の責務とする。

## 5. 受入条件

- `requirements_definition.md` に Human Decision が記録されている。
- `human_decision_packet.md` と `human_decision_packet.json` が生成されている。
- `artifact_inventory.json` と `runner_explanation.json` が生成されている。
- `validate-artifacts` で生成 JSON artifact を検証できる。

## 6. Human Decision

1. HD-FDA-001: 入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか  
   - recommended: `approve_scope`  
   - required_before: `Design Gate`
2. HD-FDA-002: 実装可否分類 `implementation_candidate` と次 gate `Design Gate` を採用してよいか  
   - recommended: `accept_classification`  
   - required_before: `Design Gate`
3. HD-FDA-003: 外部API、個人情報、法務制約の未記載項目を Design Gate で明示確認する前提で進めてよいか  
   - recommended: `confirm_before_design`  
   - required_before: `Design Gate`

## 7. 次 action

1. `fda decide HD-FDA-001 --answer <answer> --artifacts <this-output-dir>`
2. `fda decide HD-FDA-002 --answer <answer> --artifacts <this-output-dir>`
3. `fda decide HD-FDA-003 --answer <answer> --artifacts <this-output-dir>`
4. 未解決判断がなくなったら `fda design --artifacts <this-output-dir>`
