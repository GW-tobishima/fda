---
standard_id: delivery-artifacts-v0
version: v0
status: draft
last_reviewed: 2026-06-20
review_cycle_days: 30
owner: forge-delivery-agent
---

# 標準成果物カタログ v0

## 目的

この標準は、AI Delivery Runtime が要件から Epic Delivery Plan、ATO Task Graph、Forge Gate、PR、Proof を生成するための最小契約を定める。

人間が読む文書だけでなく、AI と CI が検査できる機械可読な構造を持たせる。v0 では完璧な社内標準を作らず、PoC で使える最小セットに限定する。

ATO リポジトリでは、このパックを AI Delivery Runtime との共有 contract として保持する。
実行ランタイム本体は `forge-delivery-agent` 側で扱い、ATO は Program / Epic / Case / Task / Run / Evidence / Human Decision / AI Repair の state と projection を持つ。

## 適用方針

- すべての成果物は Forge Mapping を持つ。
- すべての判断停止は Human Decision Point として明示する。
- テスト不足、証跡不足、古いログ、追跡漏れは原則 AI Repair に分類する。
- UI 実装は Phase 5 以降に回すが、UI/UX 設計成果物は Phase 0 から作る。
- validator、CLI、実行ランタイムを実装する場合は Rust を既定にする。

## ディレクトリ構成

```text
docs/standards/delivery-artifacts-v0/
  artifact_catalog.md
  mapping_to_forge.md
  mapping_to_ato.md
  uiux_mission_control_design.md
  model_contracts/
    planner.contract.yaml
  templates/
    01_requirements_definition.md
    02_basic_design.md
    03_detailed_design.md
    04_epic.md
    05_pbi.md
    06_sbi.md
    07_non_functional_requirements.md
    08_design_agreement.md
    09_issue.md
    10_pull_request.md
    11_autonomy_contract.md
    12_human_decision_packet.md
  schemas/
    requirements_definition.schema.json
    epic_delivery_plan.schema.json
    autonomy_contract.schema.json
    human_decision_packet.schema.json
    issue.schema.json
    pull_request.schema.json
    case_graph.schema.json
    task_graph.schema.json
    planned_prs.schema.json
    ato_cli_materialization_plan.schema.json
    forge_projection.schema.json
    generic_run_state.schema.json
    generic_receipt.schema.json
    artifact_inventory.schema.json
    runner_explanation.schema.json
    planned_pr_execution_packet.schema.json
    mcp_agent_invocation_plan.schema.json
    mcp_tool_call_receipt.schema.json
    dry_run_receipt.schema.json
    coding_agent_thread_state.schema.json
    agent_role_policy.schema.json
    question_bank.fixture.schema.json
    quiz_set.schema.json
    quiz_prompt.schema.json
    answer_submission.schema.json
    grading_report.schema.json
    study_recommendation.schema.json
    adaptive_plan.schema.json
    slack_outbound_message.schema.json
    slack_delivery_receipt.schema.json
    slack_reply_event.schema.json
    slack_grading_response.schema.json
    slack_grading_delivery_receipt.schema.json
    slack_socket_mode_payload.schema.json
    slack_reply_intake_receipt.schema.json
    slack_thread_poll_receipt.schema.json
    maintenance_receipt.schema.json
    run_state.schema.json
    # daily_operation_runbook は Markdown artifact のため専用 schema なし
  validators/
    validate_epic_delivery_plan.py
  examples/
    forge_dashboard_epic/
```

## 成果物一覧

| 成果物 | 主用途 | 入力 | 出力 | 必須マッピング |
|---|---|---|---|---|
| Requirements Definition | 要件の契約化 | 事業目的、制約、ユーザー課題 | FR/NFR、Scope、Open Questions | Claims, Human Decision Points |
| Basic Design | アーキテクチャ合意 | 要件、制約、NFR | Solution Strategy、境界、Runtime Scenario | Claims, Proof Obligations |
| Detailed Design | PR 実装前の作業契約 | Case、Planned PR | 変更対象、実装手順、検証 | Case, Planned PR, Proof |
| Epic | Program 内の成果単位 | 要件、基本設計 | Case Graph、PR Plan、Release Strategy | Epic ClaimContract |
| PBI | ユーザー価値の単位 | Epic、要求 | AC、NFR、依存関係 | Claim IDs |
| SBI | Sprint または実行単位 | PBI、Case | Done 条件、検証、Handoff | ATO Task |
| NFR | 品質要求 | 要件、運用制約 | 測定可能な品質要求 | NFR Proof |
| Design Agreement | UI/UX 合意 | Epic、ユーザー操作 | IA、状態、アクセシビリティ、判断 UX | Human Decision UX |
| Issue | Intake または判断依頼 | 目的、Scope、Autonomy | 実行要求、制約 | Program/Epic seed |
| Daily Operation Runbook | 日次運用手順 | scheduler方針、env、run_state、Slack運用境界 | 起動手順、保存規約、再実行手順、チェックリスト | Program/Epic/Run State |
| Pull Request | 変更の昇格候補 | Case、Proof、CI | Gate 判定材料 | PromotionDecision |
| Planned PRs | Epic Planning時点の実装slice候補 | Case Graph、Claim、Proof方針 | planned_pr_id、scope、risk、claims、proof strategy | Case, Claim IDs, Proof Obligations |
| ATO CLI Materialization Plan | ATO CLI登録前のdry-run投影 | Epic Plan、Case Graph、Task Graph、Human Decision Packet、Forge Projection | Program/Epic/Case/Task/Human Decision/AI Repair lane payload preview、予定CLIコマンド | ATO CLI task/run/checkpoint/block/evidence |
| ATO State Receipt | ATO CLI書き戻しのsemantic receipt | FDA stage result、Human Decision Packet、decision receipt、主要artifact | ATO task/run/checkpoint、typed decision、decision answer/apply、evidence edge、adapter failure | ATO Task/Run/Decision/Evidence |
| Autonomy Contract | AI の権限境界 | Scope、risk、policy | allowed/forbidden/escalation | Gate Requirements |
| Human Decision Packet | 人間判断の最小単位 | 判断トリガー、選択肢 | 決定、根拠、影響 | ATO Human Turn |
| ATO CLI Commands | ATO CLI実行前の予定コマンド列 | ATO CLI Materialization Plan | work begin/checkpoint/block/evidence/readinessのdry-runコマンド | ATO CLI |
| ATO Summary Preview | ATO CLI materialization dry-runの人間向け要約 | ATO CLI Materialization Plan | 投影順序、Human Decision、Task lane、CLI dry-run境界 | ATO Summary |
| Forge Projection | Forge Gateの前段投影 | Epic、Claim、Planned PR、Proof方針 | ClaimContract候補、Proof Obligation候補、readiness | ClaimContract, Proof Obligation, PromotionReadiness |
| Planned PR Execution Packet | Planned PRを実装者へ渡す実行単位 | Design Gate、Planned PRs、Human Decision結果 | Scope、AC、検証、target repo、開始gate | Planned PR, Human Decision, Proof |
| MCP Agent Invocation Plan | MCP経由のagent起動計画 | Execution Packet、Role Policy、adapter候補 | Implementer / Functional QA / Security QA invocation、cwd、tool、禁止事項 | Task, Role Policy, Receipt |
| MCP Tool Call Receipt | MCP tool結果のsemantic receipt化 | MCP tool result、Invocation Plan | semantic verdict、scope drift、test evidence、next action | Evidence, Gate Verdict |
| MCP Dry-run Receipt | MCP adapter dry-runのgate証跡 | Invocation Plan、prompt、target repo cwd | tools/list、cwd、prompt、approval policy、禁止事項、target repo mutationなし | MCP Dry-run Gate |
| Coding Agent Thread State | repair loop用のthread継続状態 | MCP receipt、provider thread id | continuation tool、last receipt、repair count、open items | Run State, Repair |
| Agent Role Policy | agent roleごとの権限境界 | Autonomy Contract、QA policy | write/read-only、source mutation可否、禁止事項 | Autonomy, QA Gate |
| Risk Tier | F4 比例ゲートの risk tier 判定 | Scope In（planned_prs expected_files）、delivery_policy low_risk_paths / human_required_for、risk_register | tier (low/standard/high)、理由、一致した low_risk_paths、policy source。merge 時に live 再計算 + governance hard guard で再検証 | Merge Gate, Review Agent Gate |
| GC Docket | F5 庭師の棚卸し docket（read-only、削除しない） | artifacts/runs の run 群、mtime、receipt/validation/ato_state/decision 状態 | stale 未完了 / validation 欠落 / ato 失敗 / 未解決判断 / parse_error 候補と recommendation、needs_human | Human-facing Summary |
| Epic Progress State | F2 Epic 継続ループの planned PR ごとの進捗投影（read-only）。非権威の提案であり merge の証明ではない（自動化は merge 判定に使ってはならない） | epic run dir の planned_prs.json、全 run の epic_id 一致 external_pr / github_merge / merge receipt、handoff artifact | planned_pr_id ごとの status (not_started/in_progress/pr_open/human_approval_required/merge_ready/merged/blocked)、evidence、reasons、summary、scan_notes、scan_errors、advisory | Epic ClaimContract, Runtime Evidence |
| Next Planned PR Decision | F2 Epic 継続ループの次 PR 判定（read-only、auto merge しない）。非権威の提案であり実装開始許可・merge 承認ではない（自動化は merge 判定に使ってはならない） | Epic Progress State、依存 (sequence) 充足、epic run dir の未解決 Human Decision | verdict (proceed/waiting_human/blocked/complete)、next_planned_pr_id、reasons、resume_commands（実 run dir で解決）、advisory | Human Decision Gate, Merge Gate |
| Generic Run State | 汎用Daily Agent Runtimeの状態契約 | schedule、command state、idempotency、artifact refs | dispatch/event/action/maintain/statusの現在状態 | Run State, Claim IDs, Proof Obligations |
| Generic Receipt | 汎用Daily Agent Runtimeのreceipt契約 | command実行結果、idempotency、artifact refs | dispatch/event_intake/action/maintenance/status receipt examples | Runtime Evidence |
| Runtime Artifact Contracts | PR-GDAR-001のschema/fixture契約説明 | Planned PRs、Forge Projection | 汎用化対象、AICX artifactとの関係、Claim/Proof対応 | Case, Claim IDs, Proof Obligations |
| Study Schedule | 学習範囲 fixture | 学習計画、日付、ページ範囲 | 指定日の出題対象範囲 | Task input |
| Topic Map | Topic とページの対応 fixture | 教材構造、手動ページ範囲 | topic_id、page_ranges、出題方針 | Evidence source |
| Question Bank Fixture | 人間が見て意味のある問題fixture | Topic Map、手動作問 | topic_id に紐づく問題、選択肢、正答、根拠、scenario_tags | Evidence source |
| Quiz Set | 出題セット | Study Schedule、Topic Map、指定日 | A-D 選択式問題、正答キー | Proof / Runtime Evidence |
| Quiz Prompt | 受験者向け出題artifact | Quiz Set | 正答キー・解説を除いた出題全文、回答形式、学習範囲。日次Slack運用は10問を標準とし、必要なら5問×2へ分ける | Human-facing Summary |
| Answer Submission | 回答提出 | Quiz Set、回答 | question_id ごとの選択肢 | Runtime Evidence |
| Grading Report | 採点結果 | Quiz Set、Answer Submission | 全体正答率、topic 別正答率 | Proof |
| Study Recommendation | 復習推奨 | Grading Report、Topic Map | 推奨ページ、次アクション | Human-facing Summary |
| Adaptive Plan | 弱点topic反映の翌日配分 | Grading Report履歴、Study Schedule、Topic Map、Question Bank Fixture | topic別集計、80%未満topic、翌日10問配分、question bank優先の選択計画、daily-dispatch optional入力 | Runtime Evidence |
| Slack Outbound Message | Slack送信 payload smoke | Quiz Set、Quiz Prompt | summary、Slack本文に含める全10問、user-facing artifact path、required env | Runtime Evidence |
| Slack Delivery Receipt | Slack送信結果またはdry-run証跡 | Slack Outbound Message、env readiness | dry-run/live status、secret非含有 readiness、Slack response要約 | Runtime Evidence |
| Slack Reply Event | Slack返信fixture | Slack Delivery Receipt、回答text | thread_ts、event_ts、user_id、回答本文 | Runtime Evidence |
| Slack Grading Response | Slack返信への採点返信 | Slack Reply Event、Quiz Set、Grading Report、Study Recommendation | Slack thread向け採点文、topic別正答率、推奨ページ | Human-facing Summary |
| Slack Grading Delivery Receipt | 採点返信のSlack送信証跡 | Slack Grading Response、env readiness | dry-run/live status、thread_ts、Slack response要約、失敗理由 | Runtime Evidence |
| Slack Socket Mode Payload | Socket Mode受信payload fixture | Slack Events API message event | envelope_id、events_api payload、thread返信本文 | Runtime Evidence |
| Slack Reply Intake Receipt | Slack返信受信・採点・返信の実行証跡 | Socket Mode payload、env readiness、Grading Response | 受信/timeout/送信status、secret非含有readiness、Slack response要約 | Runtime Evidence |
| Slack Thread Poll Receipt | Slack thread pollingの実行証跡 | Run State、Slack thread、env readiness | no_reply_found、reply_found_graded_and_sent、invalid_reply_found、失敗理由 | Runtime Evidence |
| Daily Run State | 日次実行状態 | Study Schedule、Quiz Prompt、Slack Receipt、Slack Reply Event | 04:30 JST dispatch、Slack送信、返信受信、採点、採点返信、重複skipの状態 | Runtime Evidence |
| Maintenance Receipt | 日次catch-up統合コマンドの実行証跡 | Run State、Slack Delivery Receipt、Slack Thread Poll Receipt | daily-maintainが選んだaction/status、失敗理由、関連artifact path | Runtime Evidence |

## v0 の完了条件

- テンプレートが PoC の入力として使える。
- JSON Schema が構文として有効である。
- ready / running / done の Epic は `validators/validate_epic_delivery_plan.py` で `case_graph[].planned_pr` と `pr_plan[]` の対応も検査できる。
- サンプル Epic が Requirements、Epic Plan、Case Graph、Autonomy Contract を通して追跡できる。
- UI は未実装でも、Mission Control の情報設計、レーン、状態、判断 UX が設計済みである。

## 後続実装候補

1. Rust 製 schema validator CLI。
2. Requirements から Epic Delivery Plan を生成する planning adapter。
3. Epic Delivery Plan から ATO Task Graph を materialize する adapter。
4. Forge Gate の評価結果を ATO Evidence として保存する adapter。
5. Mission Control UI v0。
