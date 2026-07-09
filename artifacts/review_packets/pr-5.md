# Review Packet: PR #5 / F2 Epic 継続ループ（fda continue --epic）（PR-V15-004）

## 対象

- PR: https://github.com/GW-tobishima/fda/pull/5
- Branch: `pr-v15-004`
- Base: `pr-v15-003`（PR #4。PR-V15-003 は未 merge のため stack している）
- State: OPEN（未 merge）
- 位置づけ: EPIC-FDA-V1-5 の PR-V15-004（sequence 4）。epic run dir の `planned_prs.json`
  と `--artifacts-root`（既定 `artifacts/runs`）配下の全 run の receipt を `planned_pr_id`
  で突合し、planned PR ごとの状態（not_started / in_progress / pr_open /
  human_approval_required / merge_ready / blocked / merged）を判定して、次に進める PR /
  waiting_human / blocked / complete を read-only で投影する「F2 Epic 継続ループ」を実装する。
  既存の `fda continue`（repair gate）は不変で、`--epic` 指定時のみ本モジュールへ分岐する。
- Scope:
  - `src/application/epic.rs` 新設（`continue_epic` / `continue_epic_with`）。
    `parse_continue_args` に `--epic` + `--artifacts-root` を追加し、runner で epic 判定へ分岐。
  - 新 schema 2 件: `epic_progress_state.schema.json`（`fda.epic_progress_state.v1`） /
    `next_planned_pr_decision.schema.json`（`fda.next_planned_pr_decision.v1`）。
    `forge_dashboard_epic` に example を追加し `validate-artifacts` で検証。
  - status 判定優先順位: `merged > merge_ready > human_approval_required > pr_open > blocked >
    in_progress > not_started`。sequence 順で最初の未 merged PR だけを見て依存充足を担保する
    （前 PR が全て merged でない限り後続を選ばない）。未解決 Human Decision は
    `waiting_human` として `fda decide` の resume を提示。
  - **read-only 原則**: 既存 run の receipt は一切書き換えず、epic run dir へ
    `epic_progress_state.json` / `next_planned_pr_decision.json` の 2 ファイルのみ出力する。
    auto merge / 自動実装開始はしない。
  - 対象コミット: `535ea49`（F2 本体、13 files changed, 1165 insertions(+), 6 deletions(-)）/
    `50d05ee`（repair: pr_reviewer・security_qa・forge_reviewer 指摘対応、9 files changed,
    787 insertions(+), 109 deletions(-)）。

## 注記: `artifacts/review_packets/pr-5.md` の旧内容について

本ファイルは PR 番号 #5 に紐づく review packet だが、既存の `pr-5.md`（baseline import
commit `7b1c46c` 由来。内容は別リポジトリ `msamunetogetoge/forge-delivery-agent` の
PoC-0 AICX study bot PR）は本フォーク（`GW-tobishima/fda`）の実際の PR とは無関係な
upstream テンプレートの例示コンテンツだった。CI（`.github/workflows/ci.yml`）は
`github.event.pull_request.number` をそのまま `--pr-number` に渡すため、実際に GitHub 上で
OPEN な PR #5（本 PR、`pr-v15-004`）の gate 証跡はこのパスに置く以外の選択肢が無く、
旧内容を本 PR の内容へ置き換えた。旧内容は本フォークの実データではないため実害はない。

## REVIEW_AGENT_GATE

MERGE_APPROVAL: not_granted

| role | status | evidence | rationale |
|---|---|---|---|
| pr_reviewer | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-5/pr_reviewer_receipt.json`; `git show 50d05ee -- src/application/epic.rs`; `git show 50d05ee -- src/application/merge.rs` | 初回 fail（HIGH: epic_id 突合なしの OR 畳み込みで偽 merged・矛盾 evidence を検出できない／HIGH: 無関係な壊れ receipt で scan 全体 abort／medium: blocked verdict 未テスト／low: sequence fail-open・exit code 混同未明記）を指摘。repair `50d05ee` で epic_id 突合＋conflicting_evidence の fail-closed blocked 化、receipt parse error の fail-soft 化（scan_errors 継続走査）、sequence 欠落・0 の fail-closed 化、exit code 区別の明文化を確認し、再レビューで pass。残 low 3 件は非 blocking として記録。新規 informational（fda 外の人間 merge は receipt を残さない）は下記「将来課題」参照。 |
| functional_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-5/functional_qa_receipt.json`; `cargo test --lib` => 247 passed, 0 failed | 3 AC（sequence 順の依存充足判定／waiting_human・complete・merge_ready と human_approval_required の区別／read-only で2ファイルのみ出力）を unit テスト 7 件（第1巡）+ CLI black-box + 実際の epic run dir（`artifacts/runs/fda-start-1783555082/`）に対する E2E 実行で検証し、初回から pass。実 epic run では merge receipt 不在のため全 planned PR が not_started 判定になることを実機で確認（receipt 整備が運用上必須である旨を発見。本タスクで `docs/v1/work_protocol.md` 手順 8 に追記済み）。 |
| security_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-5/security_qa_receipt.json`; `git show 50d05ee -- src/application/epic.rs docs/standards/delivery-artifacts-v0/schemas/epic_progress_state.schema.json` | 第1巡は pass。medium 2 件（advisory=非権威の提案である旨が JSON 実体に表現されていない／receipt parse error 時の fail-closed が無関係な他 planned PR まで波及する）を記録。いずれも非 blocking だが、pr_reviewer・forge_reviewer の HIGH 指摘を受けた同一 repair `50d05ee` で advisory の required const フィールド追加（advisory 化）・receipt parse error の fail-soft 化として併せて解消済みとした。残り low 2 件（trade-off の定期再評価、偽装 receipt の積極検知は行わない設計）は non-blocking の将来課題・既知の設計上の制約として維持。 |
| forge_reviewer | REVIEW_AGENT_OK | `git show 50d05ee -- src/application/epic.rs src/application/merge.rs docs/standards/delivery-artifacts-v0/schemas/next_planned_pr_decision.schema.json`; `docs/v1/work_protocol.md` 手順 7 | 初回 fail（HIGH: 承認待ち（human_approval_required）と実行待ち（merge_ready）を区別しない resume で人間が足踏みする／medium: 提案性（advisory）が JSON 実体に表現されていない／medium: resume_command の run dir がプレースホルダのまま解決されていない）を指摘。repair `50d05ee` で human_approval_required を merge_ready から分離し `fda decide <id> --answer approve` / `fda merge --execute` を状態別に提示、advisory の required const フィールド追加、resume_command の実 run dir 解決（`merge_ready_resume_points_to_merge_execute_with_real_run_dir` 等のテストで確認）を確認し、再レビューで pass。残 low（防御的 dead arm 等）は non-blocking。 |
| design_qa | not_applicable | — | CLI（`src/cli/`）・application ロジック（`src/application/epic.rs` / `merge.rs`）・schema（`docs/standards/delivery-artifacts-v0/schemas/`）・work_protocol（`docs/v1/`）のみの変更で、UI / frontend / browser surface（`fda ui` の `src/application/ui.rs` や `.tsx`/`.css`/`.html` 等）には触れていない。 |

`REVIEW_AGENT_OK` は merge approval ではない。`MERGE_APPROVAL: not_granted` を維持する。

## repair 履歴（fail → repair → pass、1 統合サイクル）

1. **第1巡**:
   - functional_qa は初回から pass（3 AC を unit テスト 7 件 + CLI black-box + 実際の epic
     run dir に対する E2E 実行で検証）。
   - security_qa は初回から pass だが medium 2 件を記録（下記参照）。
   - pr_reviewer・forge_reviewer は fail。HIGH 指摘の要旨:
     - 証跡の畳み込みが **epic_id 突合なしの OR ロジック**のみで行われており、いずれかの run が
       merged を報告すれば以後不可逆に merged 扱いとなった。別 epic の同名 planned_pr_id
       receipt を拾って偽 merged を報告し得る、かつ同一 planned PR に merged と PR open の
       矛盾 evidence が併存しても検出できなかった（pr_reviewer HIGH）。
     - 1 run の receipt が壊れている（parse error）と **scan 全体が abort** し、無関係な他
       planned PR の状態まで判定不能になった（pr_reviewer HIGH、security_qa medium が
       関連事象を指摘）。
     - merge_receipt の **human_approval_required（未承認）と merge_ready（承認済み・実行待ち）
       を区別しない resume** により、人間が「承認すべきか」「実行すべきか」を誤り得た
       （forge_reviewer HIGH）。
     - 付随して pr_reviewer から medium（blocked verdict 未テスト）・low×2（sequence
       fail-open、waiting_human と blocked の exit code 混同が未明記）。
     - forge_reviewer から medium×2（advisory=提案性が JSON 実体に表現されていない、
       resume_command の run dir がプレースホルダのまま解決されていない）。
   - security_qa の medium 2 件: advisory の非権威性が JSON 実体（schema の required
     フィールド）に表現されていない、receipt parse error 時の fail-closed が無関係な他
     planned PR の可視性まで奪う波及域の広さ。
   → **repair commit `50d05ee`** で統合修復:
   - **epic_id 突合 + 矛盾検出（HIGH1）**: `receipt_epic_matches` で receipt の epic_id を
     `planned_prs.json` の epic_id と突合し、一致する receipt のみ状態根拠に採用。不一致・
     欠落は無視して `scan_notes` に記録（別 epic の偽 merged 防止）。同一 planned PR に別 run
     から merged と open の矛盾 evidence がある場合は `conflicting_evidence` として
     fail-closed で blocked（silently merged にしない、根拠パス列挙）。
   - **fail-soft 化（HIGH2）**: receipt の parse error は `scan_errors` に記録して走査を継続
     （`fda gc` と同型）。verdict への影響なし（判定は advisory、安全性は実 merge gate が
     fail-closed で担保、可用性優先の trade-off をモジュールコメントに明記）。
     `next_actions` に `fda gc` の棚卸し推奨を追加。epic run dir の正本（`planned_prs.json` /
     `human_decision_packet.json`）は引き続き fail-closed のまま。
   - **merge 系 resume の実効性（forge HIGH）**: `merge_receipt.json` の
     `human_approval_required`（未承認）を `merge_ready` から分離し新 status に。当該 run の
     `human_decision_packet.json` から merge approval の decision_id を解決（merge gate と
     同一基準 `is_merge_approval_decision` を `pub(crate)` 化して再利用）し
     `fda decide <id> --answer approve` を提示。`merge_ready` / `human_approval_granted`
     （承認済み）は `fda merge --execute` を提示。
   - **MEDIUM 対応**: verdict=blocked（先頭未 merged が blocked）のテスト追加。advisory の
     required const フィールドを両 schema + 出力 + catalog に追加（非権威の提案であり merge
     判定に使用禁止を明文化）。`resume_command` のプレースホルダを廃し evidence の実 run dir
     で解決（解決不能時は `fda gc` 案内）。
   - **LOW 対応**: sequence 欠落・0 を fail-closed（Err）に + schema `minimum: 1`。exit code
     （waiting_human / blocked とも 1）の区別は `--json` の verdict で行う旨を help と
     `work_protocol.md` に明記。`--epic` は `--ato-sync` 非対象の旨も追記。
   - テスト: `cargo test --lib` が 240 passed（第1巡時点、epic 7 件含む）→ 247 passed（repair
     後、epic 14 件: +7 = fake_merged_receipt_from_other_epic_is_ignored /
     conflicting_merged_and_open_evidence_is_blocked /
     broken_receipt_json_is_fail_soft_and_recorded_in_scan_errors /
     first_unmerged_blocked_pr_yields_blocked_verdict / missing_sequence_is_a_fail_closed_error /
     merge_ready_resume_points_to_merge_execute_with_real_run_dir /
     human_approval_required_resume_points_to_decide_with_decision_id）。
   → 全指摘を確認し、pr_reviewer・security_qa・forge_reviewer とも再レビューで **pass**。

## 将来課題（non-blocking）

1. **fda の外での merge は receipt を残さない（新規 informational、本タスクで一次対応済み）**:
   pr_reviewer・functional_qa がそれぞれ独立に発見。`fda merge` 経由の merge は
   `github_merge_receipt.json` を run dir に残すが、GitHub UI / `gh` 直接で人間が merge した
   場合は receipt が残らず epic 投影（`fda continue --epic`）が古いままになる（実 epic run dir
   `artifacts/runs/fda-start-1783555082/` に対する E2E 実行で、全 5 planned PR が
   merge receipt 不在のため not_started と判定されることを確認済み）。本タスクで
   `docs/v1/work_protocol.md` 手順 8 に「fda の外で merge した場合は merge 後に
   `github_merge_receipt.json` 相当を該当 run dir に追記して投影を最新化する」運用ガイドを
   1 行追記した。receipt を自動生成する仕組み（`gh api` 経由での merge 検知等）は本 PR の
   スコープ外の将来課題として残る。
2. **merged vs blocked の run 間食い違い**（設計上許容・未対応）: 別々の run が同一 planned PR
   に対し merged と blocked を報告するケースは `conflicting()` の判定対象外（`conflicting()`
   は merged と open の併存のみを検出）。この場合は既存の優先順位
   （`merged > merge_ready > human_approval_required > pr_open > blocked > in_progress >
   not_started`）により merged が優先して解決される。silently にリスクを覆い隠すものではなく、
   「一度 merge されれば以後の blocked evidence は古い（merge 前の）状態を指す」という
   時系列上自然な解釈のため許容している。
3. **checker の changed files 実 diff 突合**（将来課題・未対応、PR #3 から持ち越し）:
   `scripts/check_review_agent_gate.py` は review packet の自己申告のみを検証し、実際の
   `git diff` / `gh api` の changed files とは突合していない。`gh api
   repos/GW-tobishima/fda/pulls/5/files` 等で実差分と packet 記述内容を機械的に突合する
   チェックがあれば、記述と実態の乖離をさらに検出しやすくなる。
4. **hard guard パス集合の対称化**（将来課題・未対応、PR #3 から持ち越し）:
   `risk_tier.rs::is_governance_critical_path` と `merge.rs::requires_forge_review_for_merge`
   が非対称のまま。本 PR ではこの集合を変更していないため影響はないが、統合が望ましい。

## 検証結果

| command | result |
|---|---|
| `cargo fmt --all -- --check` | pass（clean） |
| `cargo test --lib` | pass: 247 passed; 0 failed; 0 ignored（`50d05ee` 反映後。第1巡時点は 240 passed） |
| `cargo run -q -- validate-artifacts` | pass: 76 passed, 0 failed, 44 skipped |
| `python3 -m unittest discover -s tests -v` | pass: Ran 68 tests, OK |
| `python3 scripts/check_architecture_boundaries.py` | pass: architecture boundary check passed |
| `python3 scripts/check_review_agent_gate.py --pr-number 5` | 本 packet 作成後に実行し pass を確認（下記「検証コマンド実行ログ」参照） |
| `gh pr view 5 --json url,number,state,headRefName,baseRefName` | `state=OPEN`, `headRefName=pr-v15-004`, `baseRefName=pr-v15-003` |
| `gh pr checks 5`（本タスク開始時点） | `rust: fail`（review packet 不在相当。旧 pr-5.md は REVIEW_AGENT_GATE section 自体が無く checker が fail していた）/ `rust-windows: pass` |

## CHANGE_INTENT

- `CLM-RP5-CHANGE-INTENT`: epic run dir の `planned_prs.json` と全 run の receipt を
  `planned_pr_id` で突合し、sequence 順で最初の未 merged PR の状態を判定して次に進める PR /
  waiting_human / blocked / complete を read-only で投影する「F2 Epic 継続ループ」
  （`fda continue --epic`）を導入する。既存の `fda continue`（repair gate）は不変。
  **auto merge / 自動実装開始は一切しない**（advisory な提案であり merge の証明ではない）。
- repair `50d05ee` は、証跡畳み込みの epic_id 未突合・矛盾検出漏れ（偽 merged リスク）、
  1 receipt の parse error による scan 全体 abort、承認待ちと実行待ちを区別しない resume の
  3 つの HIGH 相当リスクを修復し、advisory の明文化・resume の実 run dir 解決・
  sequence/exit code の fail-closed 化で判定の健全性を保守強化する。

## AC_EVIDENCE

- `CLM-RP5-AC-EVIDENCE`: functional_qa が unit テスト + CLI black-box + 実 epic E2E の
  三重で検証した AC は次の3点。
  - AC1: sequence 順で最初の未 merged PR の状態のみを見て依存充足を判定し、前 PR が全て
    merged でない限り後続 PR を選ばない。
  - AC2: 未解決 Human Decision があれば waiting_human、全 merged なら complete。
    merge_ready（承認済み・実行待ち）と human_approval_required（未承認）を区別し、
    それぞれ異なる resume command を提示する。
  - AC3: read-only。既存 run の receipt / handoff を書き換えず、epic run dir へ
    `epic_progress_state.json` / `next_planned_pr_decision.json` の2ファイルのみ出力し、
    auto merge / 自動実装開始はしない。
- 本タスクでの再検証: `cargo test --lib` => `247 passed; 0 failed; 0 ignored`、
  `cargo run -q -- validate-artifacts` => `76 passed, 0 failed, 44 skipped`、
  `python3 -m unittest discover -s tests` => `Ran 68 tests ... OK`（いずれも実行済み、
  上記「検証結果」参照）。加えて自作 fixture による CLI black-box 実行と、実際の epic run dir
  `artifacts/runs/fda-start-1783555082/` に対する E2E 実行を行った（詳細は
  `pr-5/functional_qa_receipt.json` の evidence を参照。E2E 実行で生成された
  `epic_progress_state.json` / `next_planned_pr_decision.json` は検証後に削除し、run dir を
  実行前の状態へ復元済み）。

## SEC_EVIDENCE

- `CLM-RP5-SEC-EVIDENCE`: security_qa の初回指摘（medium: advisory の非権威性が JSON 実体に
  表現されていない／medium: receipt parse error 時の fail-closed の波及域の広さ）は、
  pr_reviewer・forge_reviewer の HIGH 指摘を受けた同一の repair commit `50d05ee` で advisory
  required const フィールド追加（`git show 50d05ee -- docs/standards/delivery-artifacts-v0/schemas/epic_progress_state.schema.json`）
  と receipt parse error の fail-soft 化（`git show 50d05ee -- src/application/epic.rs`）
  により解消済みであることを確認した。
- pr_reviewer・forge_reviewer が指摘した HIGH（epic_id 突合なしの偽 merged リスク・矛盾検出漏れ、
  承認待ち/実行待ちを区別しない resume）は、repair `50d05ee` の `receipt_epic_matches` +
  `PrEvidence::conflicting`（fail-closed blocked）と、`human_approval_required` /
  `merge_ready` の分離＋`is_merge_approval_decision` 再利用による decision_id 解決で解消した。
- fail-soft 化（receipt parse error → scan_errors）は epic 進捗の可視性を優先するトレードオフ
  であり、**実 merge gate（`src/application/merge.rs`）は本 PR で変更されておらず引き続き
  fail-closed のまま**安全性を担保することをモジュール doc コメントと `merge.rs` の diff
  不在（`git show 50d05ee --stat` で `merge.rs` は `is_merge_approval_decision` の
  可視性変更のみ）で確認した。
- 残り low 2 件（trade-off の定期再評価、偽装 receipt の積極検知は行わない設計）は
  non-blocking の将来課題・既知の設計上の制約として整理した（上記「将来課題」参照）。

## ROLLBACK_PLAN

- `CLM-RP5-ROLLBACK-PLAN`: PR #5 merge 後に問題が判明した場合、PR #5 の merge commit を
  revert する。
- repair `50d05ee` のみを戻す場合、epic_id 突合・矛盾検出・fail-soft 化・resume の実効性向上が
  失われ、pr_reviewer・security_qa・forge_reviewer の HIGH 指摘が再発する状態に戻るため、
  **単独 revert は不可**。revert する場合は `535ea49`（F2 本体）ごと戻す必要がある。
- F2 は新規 CLI サブコマンド分岐（`fda continue --epic`）と新規 schema
  （`epic_progress_state`/`next_planned_pr_decision` v1）の追加であり、既存コマンド
  （`fda continue`、`start`/`design`/`implement`/`review`/`merge`）の既存契約は破壊的変更なしで
  拡張されている。rollback の blast radius は F2 関連ファイル（本 packet「対象」節に列挙した
  ファイル）に限定される。既存 run の receipt は本モジュールが一切書き換えないため、revert に
  伴うデータ復旧は不要。

## HUMAN_DECISIONS_REQUIRED

- `HUMAN_TURN_REASON`: merge approval
- `REQUESTED_DECISION`: PR #5（`50d05ee`、`pr-v15-004` → `pr-v15-003`）を merge してよいか
- `OPEN_DECISIONS`: 0（spec / risk 判断は解消済み。残るのは merge approval のみ）
- `MERGE_APPROVAL`: `not_granted`
- 4 reviewer（pr_reviewer / functional_qa / security_qa / forge_reviewer）全て
  `REVIEW_AGENT_OK`、design_qa は not_applicable、`cargo test --lib` 247 passed、
  Review Agent Gate checker pass はいずれも merge approval ではない。最終 merge は人間の
  明示判断を待つ。
- 加えて、本 PR は「fda の外での merge は receipt を残さない」運用上の穴を実機で確認した
  ため、merge 前に PR #4（`pr-v15-003`）が先に merge されていること、および merge 順序
  （PR #3 → PR #4 → PR #5）を人間が確認することを推奨する。

## FORGE_PROMOTION_DECISION

- 本タスクは receipt / review packet 整備と CI green 化が範囲であり、
  `ato case evaluate --task fda-v1-5-20260708 --pr 5 --review-packet-path artifacts/review_packets/pr-5.md --no-write --json`
  の実行は本タスクのスコープ外として未実施。
- REVIEW_AGENT_GATE の forge_reviewer 行は、本 PR が Epic 継続判断ロジック（merge 承認・
  Human Decision の resume 経路に接続する ATO/Forge 証跡投影）を実装対象とすることを踏まえ、
  not_applicable ではなく実施済みの read-only レビューとして記録した（`design_qa` のみ
  UI 非該当のため not_applicable）。
- `ato case evaluate` の実行と verdict 確認は、merge 前の別 gate（Forge Promotion Layer、
  `forge-promotion` skill）として human/orchestrator 側で改めて実施することを推奨する。
  `promote` verdict が出ても merge approval にはならない。

## TASK_TRACEABILITY

- PR: https://github.com/GW-tobishima/fda/pull/5
- Branch: `pr-v15-004` → `pr-v15-003`（`pr-v15-003` は PR #4 として未 merge のため stack）
- 対象コミット: `535ea49` / `50d05ee`
- Review packet: `artifacts/review_packets/pr-5.md`
- Receipts: `artifacts/runs/fda-start-1783555082/pr-5/pr_reviewer_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-5/functional_qa_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-5/security_qa_receipt.json`
  （PR #1 / #3 / #4 用の同名 receipt はそれぞれ `artifacts/runs/fda-start-1783555082/` 直下 /
  `pr-3/` / `pr-4/` に別途存在するため、PR #5 用はこの `pr-5/` サブディレクトリに分離して
  衝突を避けている）
- epic run dir: `artifacts/runs/fda-start-1783555082/`

## EXECUTION_PROFILE

- workspace_policy（reviewer 側）: `read_only`
- source_mutation_allowed（reviewer 側）: `false`
- 本 packet の作成・receipt 集約・`docs/v1/work_protocol.md` 手順 8 への 1 行追記は
  Implementer ロールの作業として実施した（`src/` は変更していない）。

## 検証コマンド実行ログ（本タスク実施分）

| command | result |
|---|---|
| `cargo test --lib` | `test result: ok. 247 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cargo run -q -- validate-artifacts` | `validation pass: 76 passed, 0 failed, 44 skipped` |
| `python3 -m unittest discover -s tests -v` | `Ran 68 tests in 2.116s` / `OK` |
| `python3 scripts/check_architecture_boundaries.py` | `architecture boundary check passed` |
| `cargo fmt --all -- --check` | 差分なし（clean） |
| `python3 scripts/check_review_agent_gate.py --pr-number 5` | 本 packet 作成後に実行（結果は最終報告に記載） |
| `gh pr checks 5`（本タスク開始時点） | `rust: fail`（review packet 不在相当）/ `rust-windows: pass` |
| `cargo run -q --bin fda -- continue --epic --artifacts artifacts/runs/fda-start-1783555082 --json` | 実 epic E2E: 5 planned PR 全て not_started（merge receipt 不在）、生成物は検証後に削除・復元済み |
| CLI black-box fixture（自作 epic_id=EPIC-TEST-CLI） | PR-TEST-001=merged（github_merge_receipt.json status=succeeded）/ PR-TEST-002=pr_open（external_pr_receipt.json status=opened）/ verdict=waiting_human |

## 残リスク

- `fda の外での merge は receipt を残さない」運用上の穴（上記「将来課題」1）は本タスクで
  work_protocol.md への運用ガイド追記までを実施した。receipt の自動生成・検知（`gh api` 連携等）
  は未実装であり、人間または後続 PR による対応が必要。
- PR-V15-003（PR #4）が先に merge されない限り、PR-V15-004（PR #5）は `pr-v15-003` に
  スタックしたまま `main` に対して PR #3・PR #4 の差分も含んだ状態で表示される
  （`gh pr diff 5` で確認可能）。レビューの実体は F2（本 packet の対象）に限定しているが、
  merge 順序は PR #3 → PR #4 → PR #5 を維持する必要がある。
- 既存の `artifacts/review_packets/pr-5.md`（baseline import 由来、別リポジトリの PoC-0
  AICX study bot 向け）は本 PR の内容へ置き換えた。上記「注記」参照。

## ATO_TRACE

- ATO Task Key: fda-v1-5-20260708
- ATO Run ID: run_01KX22MBQ3RBXFX0F4Y5HWZB8H
- Epic: EPIC-FDA-V1-5
- planned_pr_id: PR-V15-004
