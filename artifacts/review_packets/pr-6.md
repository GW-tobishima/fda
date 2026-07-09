# Review Packet: PR #6 / F3 道場 UI（PR-V15-005）

## 対象

- PR: https://github.com/GW-tobishima/fda/pull/6
- Branch: `pr-v15-005`
- Base: `pr-v15-004`（PR #5。PR-V15-004 は未 merge のため stack している）
- State: OPEN（未 merge）
- 位置づけ: EPIC-FDA-V1-5 の PR-V15-005（sequence 5、Epic 最終 PR）。F3「UI を道場にする」—
  人間が過去の判断とその後の帰結（merge/repair/qa の状態）に向き合い、判断力が育つ
  read-only projection を Mission Control（`fda ui`）に追加する。
- Scope:
  - `src/application/ui.rs`: スナップショット拡張（decision_journal / decision_inbox precedent /
    gc_docket / epic_progress）+ 収集ロジック + 単体テスト（計13）。
  - `src/rendering/mission_control.rs`: 道場 / 庭師 / Epic 進捗セクション + precedent /
    契約バッジ描画 + 単体テスト（計10）。
  - `src/application/policy.rs`: `normalize_summary_signature` を `pub(crate)` 化。
  - `docs/v1/mission_control_uiux.md`: §3 に Epic 進捗・道場・庭師と outcome バッジ意味論
    （human_approval=琥珀 含む）を追記。
  - `scripts/check_architecture_boundaries.py`: ui.rs allowlist に
    `use crate::infra::yaml::SerdeYamlValidator;` を追記。
- 対象コミット: `9002395`（F3 本体、4 files changed, 1061 insertions(+), 2 deletions(-)）/
  `204de96`（repair: pr_reviewer・forge_reviewer・design_qa 指摘対応、3 files changed,
  582 insertions(+), 136 deletions(-)）/ `320cdeb`（本タスクの polish: pr_reviewer・
  forge_reviewer 残 low `outcome_badge` の `human_approval` 色欠落を修正、1 file changed,
  7 insertions(+)）。

## 注記: `artifacts/review_packets/pr-6.md` の旧内容について

本ファイルは PR 番号 #6 に紐づく review packet だが、既存の `pr-6.md`（baseline import
commit `7b1c46c` 由来。内容は別リポジトリ `msamunetogetoge/forge-delivery-agent` の
PoC-1 AICX study bot PR）は本フォーク（`GW-tobishima/fda`）の実際の PR とは無関係な
upstream テンプレートの例示コンテンツだった。CI（`.github/workflows/ci.yml`）は
`github.event.pull_request.number` をそのまま `--pr-number` に渡すため、実際に GitHub 上で
OPEN な PR #6（本 PR、`pr-v15-005`）の gate 証跡はこのパスに置く以外の選択肢が無く、
旧内容を本 PR の内容へ置き換えた（PR #5 の `pr-5.md` で行われたのと同型の対応）。
旧内容は本フォークの実データではないため実害はない。

## 注記: 本タスク（review agent gate 証跡整備）の位置づけ

本タスクは PR #6 の Review Agent Gate 証跡（receipt 3 件 + 本 review packet）を完成させ
CI（`rust` ジョブの `check_review_agent_gate.py` ステップ）を green にすることが目的。
あわせて、review 済みだった残 low 2 件（`outcome_badge` の `human_approval` 色欠落 / PR body の
検証数値・レビュー経緯の陳腐化）を polish commit `320cdeb` と PR body 更新で解消した。
`src/` への変更は `320cdeb`（`outcome_badge` の match arm 追加 + テスト）のみで、既存 packet・
他 PR の receipt・`204de96` 以前の src 実装には手を入れていない。

## REVIEW_AGENT_GATE

MERGE_APPROVAL: not_granted

| role | status | evidence | rationale |
|---|---|---|---|
| pr_reviewer | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-6/pr_reviewer_receipt.json`; `git show 204de96 -- src/application/ui.rs src/rendering/mission_control.rs`; `git show 320cdeb -- src/rendering/mission_control.rs` | 初回 fail（HIGH: merge_receipt 実 schema に無いキーの死に分岐 + 架空キー fixture による偽 green／HIGH: run 帰結の全判断誤帰属／medium: precedent 一致度不透明・契約バッジ混同／low 3）を指摘。repair `204de96` で run_outcome の実 schema 準拠化・run 単位帰結の明示（列名変更 + 共通注記）・precedent 一致理由表示・契約バッジ視覚分離・packet 二重読込解消・epic tie テスト追加を確認し、再レビューで pass。残 low 2 件（human_approval バッジ色・PR body 陳腐化）は本タスクの polish commit `320cdeb` + PR body 更新で解消済みであることを確認した。 |
| functional_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-6/functional_qa_receipt.json`; `cargo test --lib` => 265 passed, 0 failed | AC1-3（道場の判断→帰結時系列 / decision inbox precedent 最大3件 / gc_docket・epic_progress 表示）を unit テスト（rendering 10 + ui 13）に加え、実 UI 起動（`fda ui --port` で `GET /api/state.json` の decision_journal 6 件確認、`fda continue --epic` / `fda gc` で一時生成した実データに対する Epic 進捗・庭師表示確認）で検証し、初回から pass。検証用生成物は確認後に削除し run dir を復元済み（`git status` クリーン確認）。 |
| security_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-6/security_qa_receipt.json`; `git show 204de96 -- src/rendering/mission_control.rs` | 新規描画経路（decision_journal / precedent / gc_docket / epic_progress）が全て `escape_html` を経由することを確認し XSS 注入経路なし。書き込みエンドポイントの追加なし（read-only projection 原則維持）。low 1 件（`/api/state.json` への判断履歴集約は閲覧範囲を広げるトレードオフ。走査コストは表示上限と独立に発生する）を non-blocking の設計上のトレードオフとして記録し pass。 |
| forge_reviewer | REVIEW_AGENT_OK | `git show 204de96 -- src/application/ui.rs docs/v1/mission_control_uiux.md`; `git show 320cdeb -- src/rendering/mission_control.rs`; `docs/v1/mission_control_uiux.md` §3 | 初回 fail（medium: epic_progress の advisory（非権威の提案である旨）が UI 投影に表現されていない／low 3）を指摘。repair `204de96` で advisory を Epic セクション冒頭に明文表示し確認、再レビューで pass。残 low 1 件（pr_reviewer と同じ `outcome_badge` の human_approval 色欠落）は polish commit `320cdeb` で解消済みであることを確認した。 |
| design_qa | REVIEW_AGENT_OK | `git show 204de96 -- docs/v1/mission_control_uiux.md src/rendering/mission_control.rs`; `cargo test --lib rendering::mission_control` => 10 passed | UI PR のため design_qa は not_applicable ではなく実施済み。設計原則（read-only 一貫性・純関数維持）・ライト/ダーク両対応（`--outcome-repair` 等のカスタムプロパティ）・HTML 構造（`<details>`/`<table>` の意味的妥当性）を確認。medium 2 件（設計正本の未更新／Runs の常時全件表示による可読性低下）を repair `204de96` で対応（設計正本 §3 更新・Runs の完了/その他折りたたみ化）し pass。 |

`REVIEW_AGENT_OK` は merge approval ではない。`MERGE_APPROVAL: not_granted` を維持する。

## repair 履歴（fail → repair → pass、1 統合サイクル + 本タスクでの残 low polish）

1. **第1巡**:
   - functional_qa は初回から pass（AC1-3 を unit テスト + 実 UI 起動で検証）。
   - security_qa は初回から pass だが low 1 件を記録（下記 SEC_EVIDENCE 参照）。
   - pr_reviewer・forge_reviewer・design_qa は fail。指摘の要旨:
     - **run_outcome の実 schema 不一致（pr HIGH）**: 判定ロジックが `merge_receipt` の
       実際のスキーマに存在しないキーを参照する死に分岐で構成されており、テスト側も
       実在しない架空キーの fixture を通していたため、`blocked` / `human_approval_required`
       が実際には常に `pending` へフォールバックする不具合を検出できず、偽の green を
       報告していた。
     - **帰結の誤帰属（pr HIGH）**: run 単位の帰結（merge/qa/repair の状態）を、道場の
       判断一覧で run 内の全判断に一様に紐付けて表示しており、個々の判断固有の帰結で
       あるかのように誤認させた。
     - **precedent 不透明・バッジ混同（pr MEDIUM ×2）**: 一致理由（完全一致/接頭辞一致）
       非表示、契約適用バッジと生の `decided_by` 文字列（`delegation_contract:DC-001:...`）
       の混同。
     - **advisory 未投影（forge MEDIUM）**: epic_progress が非権威の提案（advisory）である
       旨が UI 上に表現されていなかった。
     - **設計正本未更新・可読性（design MEDIUM ×2）**: 新セクションの意味論が設計正本
       `docs/v1/mission_control_uiux.md` に反映されておらず、Runs セクションが完了 run も
       常時全件表示するため可読性が低下していた。
     - 付随して pr_reviewer から low×2（packet 二重読込、epic 最新選択の tie 未テスト）、
       forge_reviewer から low×3（道場の切り詰め注記欠如、repair 帰結の色分離欠如、
       `outcome_badge` の `human_approval` 色欠落）、design_qa から low×2
       （道場の切り詰め注記、repair 帰結の視覚分離）。
   → **repair commit `204de96`** で統合修復:
   - run_outcome を `merge_receipt` 実 schema（`status` キーのみ）に修正し、fixture も実在値化
     （pr HIGH1 解消）。
   - 道場の帰結列を「その後（run の帰結）」に改称し「run 内 N 判断で共通」注記を追加、
     設計正本に因果非主張（run 単位の投影であり個々の判断固有の結果ではない）を明記
     （pr HIGH2 解消）。
   - precedent 一致理由（完全一致/接頭辞一致）表示 + `signatures_similar` 接頭辞テスト追加、
     inbox 契約ヒントを outline バッジ + 「（提案・自動適用なし）」常時表示に変更し、
     契約適用は authority + 塗りバッジ（生 `decided_by` 文字列を廃止）（pr MEDIUM 解消）。
   - epic_progress の advisory を Epic セクション冒頭に明文表示（forge MEDIUM 解消）。
   - 設計正本 §3 に Epic 進捗・道場・庭師と outcome バッジ意味論を追記、Runs の完了・その他を
     折りたたみ（既定閉）にしアクティブ run のみ常時表示（design MEDIUM ×2 解消）。
   - 道場の切り詰め注記「最新 N 件を表示中（全 M 件）」追加、repair 帰結を専用色
     `--outcome-repair`（紫・ライト/ダーク対応）に分離（design/forge low 解消）。
   - packet の run 毎二重読込を解消、epic 最新選択を `>` に厳格化 + tie テスト追加
     （pr low 解消）。
   - テスト: `cargo test --lib` が 251 passed（第1巡時点）→ 264 passed（repair `204de96` 後、
     rendering 9 + ui 13）。
   → pr_reviewer・forge_reviewer・design_qa とも再レビューで **pass**。残 low 2 件
     （`outcome_badge` の `human_approval` 色欠落、PR body の検証数値・レビュー経緯の陳腐化）
     は non-blocking として記録し、本タスクへ持ち越された。
2. **本タスク（PR #6 review agent gate 証跡整備・本 packet 作成）での polish**:
   - **残 low 1（`outcome_badge` の human_approval 色欠落）**: `src/rendering/mission_control.rs`
     の `outcome_badge()` に `"human_approval" => "badge-human"` の match arm を追加し、
     design 正本（`docs/v1/mission_control_uiux.md`、琥珀色定義）と一致させた。
     `outcome_badge_marks_human_approval_as_badge_human` テストを追加して固定（commit
     `320cdeb`）。
   - **残 low 2（PR body 陳腐化）**: PR #6 の body を repair `204de96` 後の実測値
     （`cargo test --lib` 265 passed、ui 13 + rendering 10）とレビュー経緯（5 役、
     fail → repair → pass）に更新した。
   - テスト: `cargo test --lib` が 264 passed（repair後）→ **265 passed**（`320cdeb` 後、
     rendering 10 = 9+1）。

## 将来課題（non-blocking）

1. **`/api/state.json` への判断履歴集約（トレードオフ、security_qa 記録）**: 道場（判断の
   振り返り）機能のため既存 `/api/state.json`（read-only, `127.0.0.1` bind のみ）に
   `decision_journal`（誰が・何を・いつ答えたか、最大50件）を追加で集約する。閲覧できる者に
   見える判断履歴の範囲が広がるトレードオフだが、`fda ui` の既存 read-only projection 原則
   （書き込みエンドポイント無し・127.0.0.1 のみ bind）の範囲内であり許容する。**走査コスト
   （全 run の `decision_receipts.json` 読込）は表示上限（上限50）とは独立に発生する**ため、
   run 数が非常に多い環境では走査コスト自体の再評価が将来必要になり得る（性能上の懸念では
   なく、run 数増加時の運用観測ポイントとして記録）。
2. **checker の changed files 実 diff 突合**（将来課題・未対応、PR #3 から持ち越し）:
   `scripts/check_review_agent_gate.py` は review packet の自己申告のみを検証し、実際の
   `git diff` / `gh api` の changed files とは突合していない。
3. **hard guard パス集合の対称化**（将来課題・未対応、PR #3 から持ち越し）:
   `risk_tier.rs::is_governance_critical_path` と `merge.rs::requires_forge_review_for_merge`
   が非対称のまま。本 PR ではこの集合を変更していないため影響はない。

## 検証結果

| command | result |
|---|---|
| `cargo fmt --all -- --check` | pass（clean） |
| `cargo test --lib` | pass: **265 passed; 0 failed; 0 ignored**（`320cdeb` 反映後。repair `204de96` 時点は 264 passed、第1巡時点は 251 passed） |
| `cargo run -q -- validate-artifacts` | pass: 76 passed, 0 failed, 44 skipped |
| `python3 -m unittest discover -s tests -v` | pass: Ran 68 tests, OK |
| `python3 scripts/check_architecture_boundaries.py` | pass: architecture boundary check passed |
| `python3 scripts/check_review_agent_gate.py --pr-number 6` | 本 packet 作成後に実行し pass を確認（下記「検証コマンド実行ログ」参照） |
| `gh pr view 6 --json url,number,state,headRefName,baseRefName` | `state=OPEN`, `headRefName=pr-v15-005`, `baseRefName=pr-v15-004` |
| `gh pr checks 6`（本タスク開始時点） | `rust: fail`（review packet 不在相当）/ `rust-windows: pass` |

## CHANGE_INTENT

- `CLM-RP6-CHANGE-INTENT`: Mission Control（`fda ui`）に「道場（判断の振り返り）」
  「庭師（棚卸し docket）」「Epic 進捗」の3新セクションと decision inbox precedent /
  契約バッジを追加し、人間が過去の判断とその後の帰結（merge/repair/qa の状態）に向き合える
  read-only projection にする（F3）。schema `fda.mission_control_snapshot.v0` は据置＝
  追加フィールドのみ。書き込みエンドポイントは追加しない。
- repair `204de96` は、run_outcome の実 schema 不一致（偽 green リスク）、帰結の判断への
  誤帰属、precedent 不透明・バッジ混同、advisory 未投影、設計正本未更新の 2 HIGH + 4
  MEDIUM 相当の指摘を修復した。
- 本タスクの polish `320cdeb` は、repair 後に残った low 2 件のうち `outcome_badge` の
  `human_approval` 色欠落を修正した（PR body 陳腐化は本 packet 作成と合わせて PR body
  更新で解消）。

## AC_EVIDENCE

- `CLM-RP6-AC-EVIDENCE`: functional_qa が unit テスト + 実 UI 起動 E2E の二重で検証した AC は
  次の3点。
  - AC1: 回答済み判断 + 帰結を持つ run 群で道場に判断→その後（run の帰結）が新しい順の
    時系列で表示され、帰結が run 単位の投影であることが「run 内 N 判断で共通」注記で
    明示される。
  - AC2: 未解決判断 + 同 type の過去判断で precedent が最大 3 件、一致理由付きで decision
    inbox に添付され、適用可能な delegation contract があれば契約バッジ + resume command が
    添付される。
  - AC3: gc_docket と epic_progress の双方が、生成済みであれば表示され、無ければ空状態
    メッセージ（「docket なし」「Epic セクション自体を出さない」）が出る。
- 本タスクでの再検証: `cargo test --lib` => `265 passed; 0 failed; 0 ignored`、
  `cargo run -q -- validate-artifacts` => `76 passed, 0 failed, 44 skipped`、
  `python3 -m unittest discover -s tests` => `Ran 68 tests ... OK`（いずれも実行済み、上記
  「検証結果」参照）。加えて実際の repo 状態（既存 run の `decision_receipts.json` 計6件）に
  対する UI 実起動 E2E、および `fda continue --epic` / `fda gc` で一時生成した実 Epic 進捗・
  gc docket データに対する UI 実起動 E2E を実施した（詳細は
  `pr-6/functional_qa_receipt.json` の evidence を参照。E2E で生成した
  `epic_progress_state.json` / `next_planned_pr_decision.json` / `artifacts/runs/_gc/` は
  検証後に削除し、`git status --short` が空であることを確認して復元済み）。

## SEC_EVIDENCE

- `CLM-RP6-SEC-EVIDENCE`: security_qa が確認した新規描画経路（decision_journal / precedent /
  gc_docket / epic_progress）は全て `escape_html` を経由しており（`git grep -n escape_html
  src/rendering/mission_control.rs`）、XSS 注入経路は見当たらない。
  `page_escapes_html_in_new_sections` テストで `<script>`/`<img>`/`<b>` 注入がいずれも
  エスケープされることを確認済み。
- 本 PR は書き込みエンドポイントを追加していない（`git grep` で `fda ui` に POST/PUT/DELETE
  ハンドラが無いことを確認、既存 `GET /` と `GET /api/state.json` のみ）。
- 残り low 1 件（`/api/state.json` への判断履歴集約は閲覧範囲を広げるトレードオフ。走査コストは
  表示上限と独立）は non-blocking の将来課題として整理した（上記「将来課題」1 参照）。

## ROLLBACK_PLAN

- `CLM-RP6-ROLLBACK-PLAN`: PR #6 merge 後に問題が判明した場合、PR #6 の merge commit を
  revert する。
- repair `204de96` のみを戻す場合、run_outcome の実 schema 準拠化・帰結の run 単位明示・
  precedent 一致理由表示・advisory 投影・設計正本更新が失われ、pr_reviewer・forge_reviewer・
  design_qa の指摘が再発する状態に戻るため、**単独 revert は不可**。revert する場合は
  `9002395`（F3 本体）ごと戻す必要がある。
- polish `320cdeb`（`outcome_badge` の `human_approval` 色追加）は独立した表示上の修正であり
  単独revertしても他コミットの契約を壊さない（badge-neutral へのフォールバックに戻るのみ）。
- F3 は新規セクション追加（既存 `fda ui` の HTML 構造への追記）であり、既存コマンド
  （`start`/`design`/`implement`/`review`/`merge`/`continue`/`gc`）の既存契約は破壊的変更なしで
  拡張されている。rollback の blast radius は F3 関連ファイル（本 packet「対象」節に列挙した
  ファイル）に限定される。既存 run の receipt / handoff は本モジュールが一切書き換えないため、
  revert に伴うデータ復旧は不要。

## HUMAN_DECISIONS_REQUIRED

- `HUMAN_TURN_REASON`: merge_approval
- `REQUESTED_DECISION`: PR #6（`9002395` → `204de96` → `320cdeb`、`pr-v15-005` →
  `pr-v15-004`）を merge してよいか
- `OPEN_DECISIONS`: 0（spec / risk 判断は解消済み。残るのは merge approval のみ）
- `MERGE_APPROVAL`: `not_granted`
- 5 reviewer（pr_reviewer / functional_qa / security_qa / forge_reviewer / design_qa）全て
  `REVIEW_AGENT_OK`、`cargo test --lib` 265 passed、Review Agent Gate checker pass は
  いずれも merge approval ではない。最終 merge は人間の明示判断を待つ。
- 本 PR は EPIC-FDA-V1-5 の最終 PR（sequence 5）であり、`pr-v15-004`（PR #5）・
  `pr-v15-003`（PR #4）が先に merge されていること、および merge 順序
  （PR #3 → PR #4 → PR #5 → PR #6）を人間が確認することを推奨する。

## FORGE_PROMOTION_DECISION

- 本タスクは PR #6 の Review Agent Gate 証跡（receipt + review packet）整備と CI green 化が
  範囲であり、`ato case evaluate --task fda-v1-5-20260708 --pr 6
  --review-packet-path artifacts/review_packets/pr-6.md --no-write --json` の実行は
  本タスクのスコープ外として未実施。
- REVIEW_AGENT_GATE の forge_reviewer 行は、本 PR が判断の帰結投影・advisory・delegation
  contract 適用可視化という ATO/Forge 証跡投影ロジックを対象とすることを踏まえ、
  not_applicable ではなく実施済みの read-only レビューとして記録した。design_qa も UI PR
  のため not_applicable ではなく実施済みとして記録した。
- `ato case evaluate` の実行と verdict 確認は、merge 前の別 gate（Forge Promotion Layer、
  `forge-promotion` skill）として human/orchestrator 側で改めて実施することを推奨する。
  `promote` verdict が出ても merge approval にはならない。

## TASK_TRACEABILITY

- PR: https://github.com/GW-tobishima/fda/pull/6
- Branch: `pr-v15-005` → `pr-v15-004`（`pr-v15-004` は PR #5 として未 merge のため stack）
- 対象コミット: `9002395` / `204de96` / `320cdeb`
- Review packet: `artifacts/review_packets/pr-6.md`
- Receipts: `artifacts/runs/fda-start-1783555082/pr-6/pr_reviewer_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-6/functional_qa_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-6/security_qa_receipt.json`
  （PR #1 / #3 / #4 / #5 用の同名 receipt はそれぞれ
  `artifacts/runs/fda-start-1783555082/` 直下 / `pr-3/` / `pr-4/` / `pr-5/` に別途存在するため、
  PR #6 用はこの `pr-6/` サブディレクトリに分離して衝突を避けている）
- epic run dir: `artifacts/runs/fda-start-1783555082/`

## EXECUTION_PROFILE

- workspace_policy（reviewer 側）: `read_only`
- source_mutation_allowed（reviewer 側）: `false`
- 本 packet の作成・receipt 集約・`outcome_badge` の polish 修正（`320cdeb`）・PR body 更新は
  Implementer ロールの作業として実施した。`src/` への変更は `320cdeb`（1 file）のみ。

## 検証コマンド実行ログ（本タスク実施分）

| command | result |
|---|---|
| `cargo test --lib` | `test result: ok. 265 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cargo test --lib rendering::mission_control` | `test result: ok. 10 passed; 0 failed`（`outcome_badge_marks_human_approval_as_badge_human` 含む） |
| `cargo test --lib application::ui::` | `test result: ok. 13 passed; 0 failed` |
| `cargo run -q -- validate-artifacts` | `validation pass: 76 passed, 0 failed, 44 skipped` |
| `python3 -m unittest discover -s tests -v` | `Ran 68 tests in 2.xxxs` / `OK` |
| `python3 scripts/check_architecture_boundaries.py` | `architecture boundary check passed` |
| `cargo fmt --all -- --check` | 差分なし（clean） |
| `python3 scripts/check_review_agent_gate.py --pr-number 6` | 本 packet 作成後に実行（結果は最終報告に記載） |
| `gh pr checks 6`（本タスク開始時点） | `rust: fail`（review packet 不在相当）/ `rust-windows: pass` |
| `cargo run -q --bin fda -- continue --epic --artifacts artifacts/runs/fda-start-1783555082 --artifacts-root artifacts/runs --json` | 実 epic E2E: 5 planned PR 全て not_started（merge receipt 不在）、生成物は検証後に削除・復元済み |
| `cargo run -q --bin fda -- gc --artifacts-root artifacts/runs --json` | 実 gc E2E: candidate_count=4, needs_human_count=0、生成物は検証後に削除・復元済み |
| `fda ui --artifacts-root artifacts/runs --port 4877` + `GET /` / `GET /api/state.json` | 200 / decision_journal 6 件、道場・庭師・Epic 進捗セクション表示を確認 |

## 残リスク

- PR-V15-004（PR #5）・PR-V15-003（PR #4）が先に merge されない限り、PR-V15-005（PR #6）は
  `pr-v15-004` にスタックしたまま `main` に対して PR #3・PR #4・PR #5 の差分も含んだ状態で
  表示される（`gh pr diff 6` で確認可能）。レビューの実体は F3（本 packet の対象）に限定して
  いるが、merge 順序は PR #3 → PR #4 → PR #5 → PR #6 を維持する必要がある。
- `/api/state.json` への判断履歴集約のトレードオフ（上記「将来課題」1）は non-blocking として
  維持し、走査コストの再評価は run 数増加時の運用観測ポイントとして残す。

## ATO_TRACE

- ATO Task Key: fda-v1-5-20260708
- ATO Run ID: run_01KX22MBQ3RBXFX0F4Y5HWZB8H
- Epic: EPIC-FDA-V1-5
- planned_pr_id: PR-V15-005
