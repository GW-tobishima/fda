# Basic Design

## 1. 目的

Intake で定義された目的を、実装前に検証可能な Design Gate artifact へ落とす。

## 2. 入力要約

# FDA V1.5 Human Input Spec（Epic 要求入力） 作成日: 2026-07-09 依頼者: k_tobishima（AskUserQuestion による機能スコープ承認済み: 2026-07-09） 背景: `docs/v1/fda_v1_next_phase_v1_5.md`（V1.5 計画）と fda+ato 論評 （`artifacts/reports/2026-07-09_fda-ato-philosophy-critique/`）への依頼者所感に基づく。 ## やりたいこと FDA V1.5 を実装する。テーマは「**関所を減らさずに、人間がハンコ係を卒業する**」。 人間が成長しながら、AI のみの開発速度に近づくための 6 機能を、FDA 自身のジャーニー （dogfood）で開発する。 ## 機能要求 - F1 判断の立法化: `fda policy propose` が過去の Human Decision 履歴から委任契約候補 （delegation_contract.yaml）を逆提案し、人間が承認した契約は以後の同型判断を 自動回答する（契約 ID 引用・期限必須・撤回可・自己承認禁止）。 - F2 Epic 継続ループ: `fda continue --epic` が planned_prs / 各 PR の receipt を読み、 epic_progress_state.json を生成し、次に進める planned PR / blocked / waiting_human を判定する。 - F3 道場 UI: `fda ui` に Decision Journal（過去の判断→その後の帰結）ビューと、 Decision Inbox での過去類似判断（precedent）表示を追加する。 - F4 比例ゲート: 変更内容の risk tier を判定し、低リスク run の儀式（受領証・必須成果物）を 軽量化する。ゲートの種類は減らさない（fail-closed 維持）。 - F5 庭師: `fda gc` が artifacts/runs の stale run・不整合 receipt を棚卸しし、 人間には例外だけを docket として提示する（自動削除しない）。 - F6 表層分け: CLI ヘルプと docs を「作業・証跡・判断・知識」の 4 概念カーネルで再編し、 work protocol の正本を単一ファイル化して AGENTS.md / CLAUDE.md / skill からは参照だけにする。 ## 制約・非対象 - FDA V1 の思想を壊さない: Human Decision 自己承認禁止 / auto merge なし / SoT 分離 / fail-closed / 成果物契約の後方互換（`fda.*.v0` は改名しない。追加は v0 追補か新 schema）。 - 「人間の依頼の質」サポート（深掘りインタビュー）は対象外（別プロジェクト DCC の領域）。 - ATO 本体の改修（must_read scope）は本 Epic の外で別途実施する。 - 開発は Windows + Claude Code / Codex CLI 両対応環境で行い、CI（ubuntu + windows）green を維持する。 ## 受け入れの考え方 - 各機能は planned PR に分割し、PR ごとに review agent gate を通し merge-ready にする。 - merge approval は人間（依頼者）に返す。 - V1.5 Done は `docs/v1/fda_v1_next_phase_v1_5.md` §4 に本 Epic の範囲を加味して判定する。

## 3. Scope In

- Basic Design と Detailed Design を作る。
- Case Graph、Task Graph、Planned PRs、Autonomy Contract、Forge Projection を作る。
- Functional QA brief と Security QA brief を作る。

## 4. Scope Out

- この command は target repo の実装、MCP agent invocation、PR 作成、merge を行わない。
- 実装者、Functional QA、Security QA の実起動は後続 PR の責務とする。

## 5. Acceptance Criteria

- Given Intake の Human Decision が解決済み、When `fda design` を実行する、Then Design Gate artifact が生成される。
- Given Human Decision が未解決、When `fda design` を実行する、Then Design Gate は停止し、判断 ID と再開 command を表示する。
- Given 生成 JSON artifact、When `validate-artifacts` を実行する、Then schema validation が pass する。

## 6. OPEN_QUESTIONS

- 実装PRの分割数は後続 Planned PR refinement で確定する。
- MCP 実装 agent の実 tool capability は PR-V1-005 の dry-run で確認する。

## 7. Risk And Mitigation

- Risk: 設計 artifact が入力意図を過剰に具体化する。Mitigation: Scope Out と Human Decision dependency を Planned PRs に残す。
- Risk: Security QA が Functional QA と混ざる。Mitigation: brief を分離し、Task Graph 上も role を分ける。
