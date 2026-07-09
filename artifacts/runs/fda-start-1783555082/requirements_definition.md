# Requirements Definition

## 1. 入力

- 入力元: `docs/v1/fda_v1_5_intake.md`
- 入力要約: # FDA V1.5 Human Input Spec（Epic 要求入力） 作成日: 2026-07-09 依頼者: k_tobishima（AskUserQuestion による機能スコープ承認済み: 2026-07-09） 背景: `docs/v1/fda_v1_next_phase_v1_5.md`（V1.5 計画）と fda+ato 論評 （`artifacts/reports/2026-07-09_fda-ato-philosophy-critique/`）への依頼者所感に基づく。 ## やりたいこと FDA V1.5 を実装する。テーマは「**関所を減らさずに、人間がハンコ係を卒業する**」。 人間が成長しながら、AI のみの開発速度に近づくための 6 機能を、FDA 自身のジャーニー （dogfood）で開発する。 ## 機能要求 - F1 判断の立法化: `fda policy propose` が過去の Human Decision 履歴から委任契約候補 （delegation_contract.yaml）を逆提案し、人間が承認した契約は以後の同型判断を 自動回答する（契約 ID 引用・期限必須・撤回可・自己承認禁止）。 - F2 Epic 継続ループ: `fda continue --epic` が planned_prs / 各 PR の receipt を読み、 epic_progress_state.json を生成し、次に進める planned PR / blocked / waiting_human を判定する。 - F3 道場 UI: `fda ui` に Decision Journal（過去の判断→その後の帰結）ビューと、 Decision Inbox での過去類似判断（precedent）表示を追加する。 - F4 比例ゲート: 変更内容の risk tier を判定し、低リスク run の儀式（受領証・必須成果物）を 軽量化する。ゲートの種類は減らさない（fail-closed 維持）。 - F5 庭師: `fda gc` が artifacts/runs の stale run・不整合 receipt を棚卸しし、 人間には例外だけを docket として提示する（自動削除しない）。 - F6 表層分け: CLI ヘルプと docs を「作業・証跡・判断・知識」の 4 概念カーネルで再編し、 work protocol の正本を単一ファイル化して AGENTS.md / CLAUDE.md / skill からは参照だけにする。 ## 制約・非対象 - FDA V1 の思想を壊さない: Human Decision 自己承認禁止 / auto merge なし / SoT 分離 / fail-closed / 成果物契約の後方互換（`fda.*.v0` は改名しない。追加は v0 追補か新 schema）。 - 「人間の依頼の質」サポート（深掘りインタビュー）は対象外（別プロジェクト DCC の領域）。 - ATO 本体の改修（must_read scope）は本 Epic の外で別途実施する。 - 開発は Windows + Claude Code / Codex CLI 両対応環境で行い、CI（ubuntu + windows）green を維持する。 ## 受け入れの考え方 - 各機能は planned PR に分割し、PR ごとに review agent gate を通し merge-ready にする。 - merge approval は人間（依頼者）に返す。 - V1.5 Done は `docs/v1/fda_v1_next_phase_v1_5.md` §4 に本 Epic の範囲を加味して判定する。

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
