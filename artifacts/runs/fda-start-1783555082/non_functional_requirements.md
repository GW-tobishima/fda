# Non-Functional Requirements

## 1. Traceability

- 入力元 `docs/v1/fda_v1_5_intake.md`、実装可否分類 `implementation_candidate`、Human Decision ID を artifact 間で追跡できること。
- `runner_explanation.json` は stop condition と next action を持つこと。

## 2. Safety

- Intake dry-run は target repo を変更しないこと。
- 未解決 Human Decision がある状態では実装系 command へ進ませないこと。

## 3. Operability

- CLI stdout は未解決判断と再開 command を表示すること。
- Markdown artifact と JSON artifact の両方を生成し、人間確認と機械検証の両方に使えること。

## 4. Input Handling

- 入力要約: # FDA V1.5 Human Input Spec（Epic 要求入力） 作成日: 2026-07-09 依頼者: k_tobishima（AskUserQuestion による機能スコープ承認済み: 2026-07-09） 背景: `docs/v1/fda_v1_next_phase_v1_5.md`（V1.5 計画）と fda+ato 論評 （`artifacts/reports/2026-07-09_fda-ato-philosophy-critique/`）への依頼者所感に基づく。 ## やりたいこと FDA V1.5 を実装する。テーマは「**関所を減らさずに、人間がハンコ係を卒業する**」。 人間が成長しながら、AI のみの開発速度に近づくための 6 機能を、FDA 自身のジャーニー （dogfood）で開発する。 ## 機能要求 - F1 判断の立法化: `fda policy propose` が過去の Human Decision 履歴から委任契約候補 （delegation_contract.yaml）を逆提案し、人間が承認した契約は以後の同型判断を 自動回答する（契約 ID 引用・期限必須・撤回可・自己承認禁止）。 - F2 Epic 継続ループ: `fda continue --epic` が planned_prs / 各 PR の receipt を読み、 epic_progress_state.json を生成し、次に進める planned PR / blocked / waiting_human を判定する。 - F3 道場 UI: `fda ui` に Decision Journal（過去の判断→その後の帰結）ビューと、 Decision Inbox での過去類似判断（precedent）表示を追加する。 - F4 比例ゲート: 変更内容の risk tier を判定し、低リスク run の儀式（受領証・必須成果物）を 軽量化する。ゲートの種類は減らさない（fail-closed 維持）。 - F5 庭師: `fda gc` が artifacts/runs の stale run・不整合 receipt を棚卸しし、 人間には例外だけを docket として提示する（自動削除しない）。 - F6 表層分け: CLI ヘルプと docs を「作業・証跡・判断・知識」の 4 概念カーネルで再編し、 work protocol の正本を単一ファイル化して AGENTS.md / CLAUDE.md / skill からは参照だけにする。 ## 制約・非対象 - FDA V1 の思想を壊さない: Human Decision 自己承認禁止 / auto merge なし / SoT 分離 / fail-closed / 成果物契約の後方互換（`fda.*.v0` は改名しない。追加は v0 追補か新 schema）。 - 「人間の依頼の質」サポート（深掘りインタビュー）は対象外（別プロジェクト DCC の領域）。 - ATO 本体の改修（must_read scope）は本 Epic の外で別途実施する。 - 開発は Windows + Claude Code / Codex CLI 両対応環境で行い、CI（ubuntu + windows）green を維持する。 ## 受け入れの考え方 - 各機能は planned PR に分割し、PR ごとに review agent gate を通し merge-ready にする。 - merge approval は人間（依頼者）に返す。 - V1.5 Done は `docs/v1/fda_v1_next_phase_v1_5.md` §4 に本 Epic の範囲を加味して判定する。
