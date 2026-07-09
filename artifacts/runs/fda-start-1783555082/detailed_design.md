# Detailed Design

## 1. Input Contract

- Source summary: # FDA V1.5 Human Input Spec（Epic 要求入力） 作成日: 2026-07-09 依頼者: k_tobishima（AskUserQuestion による機能スコープ承認済み: 2026-07-09） 背景: `docs/v1/fda_v1_next_phase_v1_5.md`（V1.5 計画）と fda+ato 論評 （`artifacts/reports/2026-07-09_fda-ato-philosophy-critique/`）への依頼者所感に基づく。 ## やりたいこと FDA V1.5 を実装する。テーマは「**関所を減らさずに、人間がハンコ係を卒業する**」。 人間が成長しながら、AI のみの開発速度に近づくための 6 機能を、FDA 自身のジャーニー （dogfood）で開発する。 ## 機能要求 - F1 判断の立法化: `fda policy propose` が過去の Human Decision 履歴から委任契約候補 （delegation_contract.yaml）を逆提案し、人間が承認した契約は以後の同型判断を 自動回答する（契約 ID 引用・期限必須・撤回可・自己承認禁止）。 - F2 Epic 継続ループ: `fda continue --epic` が planned_prs / 各 PR の receipt を読み、 epic_progress_state.json を生成し、次に進める planned PR / blocked / waiting_human を判定する。 - F3 道場 UI: `fda ui` に Decision Journal（過去の判断→その後の帰結）ビューと、 Decision Inbox での過去類似判断（precedent）表示を追加する。 - F4 比例ゲート: 変更内容の risk tier を判定し、低リスク run の儀式（受領証・必須成果物）を 軽量化する。ゲートの種類は減らさない（fail-closed 維持）。 - F5 庭師: `fda gc` が artifacts/runs の stale run・不整合 receipt を棚卸しし、 人間には例外だけを docket として提示する（自動削除しない）。 - F6 表層分け: CLI ヘルプと docs を「作業・証跡・判断・知識」の 4 概念カーネルで再編し、 work protocol の正本を単一ファイル化して AGENTS.md / CLAUDE.md / skill からは参照だけにする。 ## 制約・非対象 - FDA V1 の思想を壊さない: Human Decision 自己承認禁止 / auto merge なし / SoT 分離 / fail-closed / 成果物契約の後方互換（`fda.*.v0` は改名しない。追加は v0 追補か新 schema）。 - 「人間の依頼の質」サポート（深掘りインタビュー）は対象外（別プロジェクト DCC の領域）。 - ATO 本体の改修（must_read scope）は本 Epic の外で別途実施する。 - 開発は Windows + Claude Code / Codex CLI 両対応環境で行い、CI（ubuntu + windows）green を維持する。 ## 受け入れの考え方 - 各機能は planned PR に分割し、PR ごとに review agent gate を通し merge-ready にする。 - merge approval は人間（依頼者）に返す。 - V1.5 Done は `docs/v1/fda_v1_next_phase_v1_5.md` §4 に本 Epic の範囲を加味して判定する。
- Required before design: unresolved Human Decision がないこと。

## 2. Artifact Contract

- `basic_design.md`: Scope、AC、OPEN_QUESTIONS、risk を持つ。
- `case_graph.json`: Case と Planned PR の対応を持つ。
- `task_graph.json`: Implementer / Functional QA / Security QA を分離する。
- `planned_prs.json`: 受入条件、証跡、Human Decision dependency を持つ。
- `autonomy_contract.json`: allowed / forbidden / escalation / evidence policy を持つ。
- `forge_projection.json`: ClaimContract と Proof Obligation を持つ。

## 3. Execution Boundary

Design Gate は planning-only である。実装、テスト実行、PR作成、merge、通知送信は行わない。

## 4. QA Brief Linkage

Functional QA は受入条件の充足、Security QA は権限、個人情報、外部API、秘密情報の扱いを確認する。

---

## V1.5 実設計（orchestrator = current AI CLI 記述）

テーマ: **関所を減らさずに、人間がハンコ係を卒業する。**

不変条件（全 PR 共通）: Human Decision の自己承認をしない（委任契約の制定は常に人間）/
V1 の auto merge なし方針を維持 / `fda.*.v0` schema は改名しない（新規は新 version）/
fail-closed 維持（ゲートの種類は減らさず重さだけ比例）/ architecture gate 維持 / CI green。

### PR-V15-001: F6 表層分け + work protocol 単一ソース化（docs / low risk）

- 新規 `docs/v1/work_protocol.md` = 作業プロトコルの単一正本（4 概念カーネル:
  作業 Work / 証跡 Evidence / 判断 Decision / 知識 Knowledge と fda/ato 語彙の対応表、
  標準ジャーニー、Review Agent Gate 契約、禁止事項、ato-sync、Windows 注意点）。
- AGENTS.md / CLAUDE.md / .claude/skills/fda-delivery は「位置づけ + 参照」に縮退（drift 根絶）。
- `fda` help を 4 概念グループ表示に再編（コマンド変更なし）。

### PR-V15-002: F4 比例ゲート + F5 庭師

- F4: 新 artifact `risk_tier.json`（schema `fda.risk_tier.v1`）。implement --dry-run 時に
  Scope In ファイル群を delivery_policy の low_risk_paths / human_required_for と突合し
  low / standard / high を判定。low は merge gate の forge_reviewer / design_qa を自動
  not_applicable（理由記録）+ ac_test_mapping 最低件数免除。high は現行フル + 明示。
- F5: 新コマンド `fda gc [--max-age-days 30] [--json]`。stale 未完了 run / validation 欠落 /
  ato sync 失敗放置 / 長期未解決判断を検出し `artifacts/runs/_gc/gc_docket.{json,md}` を生成。
  削除は一切しない（人間には例外だけ）。

### PR-V15-003: F1 判断の立法化（delegation contract）

- optional profile `.fda/delegation_contract.yaml`（7 ファイル必須は不変）+ 新 schema
  `delegation_contract_yaml.schema.json`。rule: {rule_id, decision_type,
  match_summary_keywords, answer, authority, enacted_from, expires(必須), note}。
- `fda policy propose [--min-occurrences 3]`: 全 run の decision_receipts を (type ×
  summary 署名 × answer) でクラスタし、契約候補を policy_proposal.{json,md} に出力。
  .fda へは書かない（人間の YAML 編集 = 署名 = 制定）。
- `fda decide <ID> --by-contract <rule_id>`: 合致 + 未失効の場合のみ契約の answer で記録。
  decided_by = `delegation_contract:<rule_id>:<authority>`。失効/不一致は拒否し人間へ。
- status / inbox に「DC-xxx 適用可」ヒント表示（自動適用しない）。

### PR-V15-004: F2 Epic 継続ループ

- `fda continue --epic --artifacts <epic run dir>`: planned_prs.json と全 run の
  external_pr_receipt / merge receipt を planned_pr_id で突合し
  `epic_progress_state.json`（`fda.epic_progress_state.v1`）を生成。sequence 順で
  依存充足済みの次 PR を選定し `next_planned_pr_decision.json` を出力。
  未解決判断 / merge 待ちは waiting_human として resume command 提示。
  既存 continue（repair gate）は不変。

### PR-V15-005: F3 道場 UI

- ui snapshot に decision_journal（回答済み判断とその後の帰結）、inbox precedent
  （同 type の過去判断最大 3 件）、gc docket / epic progress 表示を追加。
- HTML に「道場（判断の振り返り）」セクション。read-only 原則は不変。

### 順序と依存

001 → 002 → 003 → 004 → 005（stacked branch、merge 承認は人間へ）。
テスト: 各 PR で cargo test + validate-artifacts（新 schema + example 同梱）+
architecture gate + CI (ubuntu/windows)。
