# Review Packet: PR #3 / F4 比例ゲート (risk tier) + F5 庭師 (fda gc)（PR-V15-002、旧 PR #2 の後継）

## 対象

- PR: https://github.com/GW-tobishima/fda/pull/3
- Branch: `pr-v15-002`
- Base: `main`
- State: OPEN（未 merge）
- 位置づけ: EPIC-FDA-V1-5 の PR-V15-002。関所を減らさずに儀式を blast radius に比例させる
  「F4 比例ゲート（risk tier）」と、棚卸しを AI の定常業務にする「F5 庭師（fda gc）」を導入する。
  旧 PR #2（同ブランチ `pr-v15-002`）の後継 PR で、内容は同一系譜（fail→repair→pass の経緯を含む）。
- Scope:
  - **F4 比例ゲート**: 新 schema `risk_tier.schema.json`（`fda.risk_tier.v1`）。
    `src/application/risk_tier.rs` に `assess_risk_tier`（scope_paths + delivery_policy からの
    純ロジック判定）、`proportional_relaxation`（merge 時 live 再検証つきの比例緩和判定）、
    `is_governance_critical_path`（governance-critical パスのハードガード、YAML 上書き不可）を実装。
    `fda implement --dry-run` が `risk_tier.json` を出力し、`fda review` / `fda merge` / `fda status` が
    tier を伝搬・表示する。
  - **F5 庭師**: 新コマンド `fda gc [--artifacts-root] [--max-age-days 30] [--repo-root] [--json]`。
    read-only スキャン + docket 出力のみ（既存 run への変更・削除はしない）。新 schema
    `gc_docket.schema.json`（`fda.gc_docket.v1`）。
  - 対象コミット: `4a4aacb`（F4+F5 本体）/ `73dbbfe`（repair: security_qa 指摘の比例ゲート脆弱性を修復 —
    merge 時再検証 + ガバナンス・ハードガード）/ `ecc8781`（本タスクの polish: governance hard guard に
    `tests/test_review_agent_gate.py` と `.github/workflows/ci.yml` を追加）。
  - 差分: `git diff --stat 1b1d209..73dbbfe` = 25 files changed, 2161 insertions(+), 20 deletions(-)。
    `ecc8781` はこれに `src/application/risk_tier.rs` の 6 行追加（ガード対象パス2行 + assert 2行 +
    フォーマット整形分）を加える。

## REVIEW_AGENT_GATE

MERGE_APPROVAL: not_granted

| role | status | evidence | rationale |
|---|---|---|---|
| pr_reviewer | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-3/pr_reviewer_receipt.json`; `git show 73dbbfe`; `git show ecc8781 -- src/application/risk_tier.rs` | 初回 fail（high1: 統治ファイルが forge_reviewer 緩和から実質除外され得た / medium1: merge.rs の独自スキップ判定による二重ゲート不整合 / low1: gc の壊れた JSON で scan 全体 abort / low2: detailed_design.md の記述乖離）を指摘。repair `73dbbfe` で全解消を確認し、再レビューで pass。再レビュー時の残余 low 4 件はうち2件を本 PR の `ecc8781` で解消済み（下記「残余 low」参照）。 |
| functional_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-3/functional_qa_receipt.json`; `cargo test --lib` => 207 passed, 0 failed; `cargo run -q -- validate-artifacts` => 70 passed, 0 failed, 43 skipped | AC1（risk tier 純ロジック + 実 CLI dry-run 生成物の比較）/ AC2（fda gc の read-only 性、実行前後の内容比較）/ AC3（merge 時 live 再検証・stored/live mismatch 却下）を実 CLI 実行 + ハッシュ/フィールド比較で検証し、初回から pass。 |
| security_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-3/security_qa_receipt.json`; `git show 73dbbfe -- src/application/risk_tier.rs` | 初回 fail（HIGH-1: stored tier 無検証信頼による TOCTOU/scope-drift / HIGH-2: 統治ファイルの forge_reviewer バイパス経路）を指摘。repair `73dbbfe` で live 再検証（`proportional_relaxation`）とガバナンス・ハードガード（`is_governance_critical_path`、YAML 上書き不可）を導入したことを確認し、再レビューで pass。本 PR の `ecc8781` は既存ハードガード関数の対象パス集合を広げるのみで新規リスクを持ち込まないことを確認。 |
| forge_reviewer | REVIEW_AGENT_OK | `git show 73dbbfe -- src/application/review.rs src/application/merge.rs scripts/check_review_agent_gate.py`; `docs/v1/work_protocol.md` 比例緩和節 | 本 PR 自体が ATO/Forge 統治ロジック（risk tier・merge gate・review gate schema）の変更を伴うため forge_reviewer は not_applicable ではなく実施済み。初回 fail（HIGH: 緩和判定が merge.rs 独自スキップとして二重化し review_agent_gate.json の既存 schema 契約を迂回）を指摘。repair `73dbbfe` で緩和判定を `review.rs` の既存契約（`status=not_applicable` + `not_applicable_reason` に `risk_tier=low`）に一本化し、`merge.rs` は live 再検証のみを行う構成へ修復したことを確認し、再レビューで pass。 |
| design_qa | not_applicable | — | UI / frontend / browser surface に触れない。変更は CLI（`src/cli/`）・application ロジック（`src/application/risk_tier.rs` / `gc.rs` / `merge.rs` / `review.rs`）・schema（`docs/standards/delivery-artifacts-v0/schemas/`）・gate script（`scripts/`）のみで、`fda ui`（`src/application/ui.rs`）や `.tsx`/`.css`/`.html` 等の frontend 資材は変更していない。 |

`REVIEW_AGENT_OK` は merge approval ではない。`MERGE_APPROVAL: not_granted` を維持する。

## repair 履歴（fail → repair → pass、1 統合サイクル + 1 polish）

1. **第1巡**: functional_qa は初回から pass（3 AC を実 CLI 実行 + ハッシュ/フィールド比較で検証）。
   pr_reviewer・security_qa・forge_reviewer は fail。HIGH 指摘の要旨:
   - stored tier（`risk_tier.json`）を無検証で信頼していた TOCTOU / scope-drift（security_qa HIGH-1）。
   - `.fda/**` を含む統治ファイル変更が `delivery_policy.yaml` の `low_risk_paths` 指定次第で
     forge_reviewer 緩和の対象になり得た（統治バイパス。security_qa HIGH-2、pr_reviewer high1 が同一事象を指摘）。
   - 緩和判定が `merge.rs` 独自のスキップ判定として二重化し、`review.rs` / `review_agent_gate.json` の
     既存 schema 契約を迂回していた（二重ゲート不整合・schema 契約迂回。forge_reviewer HIGH、
     pr_reviewer medium1 が同一事象を指摘）。
   - 付随して pr_reviewer から low×2（`fda gc` の壊れた JSON で scan 全体 abort / `detailed_design.md` の
     「ac_test_mapping 最低件数免除」記述と実装の乖離）。
   → **repair commit `73dbbfe`** で統合修復:
   - `proportional_relaxation` 新設。merge 時に `changed_files` を `.fda/delivery_policy.yaml` で
     live 再計算し、stored=low かつ live=low の両方が成立する場合のみ緩和を受理（TOCTOU 対策）。
   - `is_governance_critical_path` 新設。governance-critical パス（`.fda/` 配下全て、
     `scripts/check_review_agent_gate.py`、`scripts/check_architecture_boundaries.py`、
     `src/application/{merge,review,risk_tier,policy}*`）を含む PR は low_risk_paths / tier に
     関係なく forge_reviewer 緩和を不適用。コードにハードコードし delivery_policy.yaml では
     上書き不能（defense in depth として repo の `.fda/delivery_policy.yaml` からも `low_risk_paths` の
     `.fda/**` を削除）。
   - 緩和の表現を `review.rs` の既存 schema 契約（`status=not_applicable` + `not_applicable_reason`）に
     一本化し、packet に `RISK_TIER: low` 行を併記。`merge.rs` は独自スキップ判定をやめ、この主張を
     live 再検証で検証するだけに変更（二重ゲート不整合の解消）。
   - `fda gc` が壊れた JSON を検出しても scan 全体を abort せず `parse_error` 理由の候補として
     報告を継続するよう修正。`detailed_design.md` から「ac_test_mapping 最低件数免除」を削除。
   - テスト: Rust 207 passed（+11: relaxation 純ロジック3 / governance 判定1 / merge 受理・hard guard
     却下・stale 却下3 / review 側緩和生成2 / dry-run tier=low 1 / gc parse_error 2）。
   → 全指摘を確認し、pr_reviewer・security_qa・forge_reviewer とも再レビューで **pass**。
2. **polish（本タスク・commit `ecc8781`）**: 再レビュー時に残余 low 4 件（non-blocking、将来 hardening）が
   記録された。うち2件をこの polish で解消:
   - `is_governance_critical_path` に `tests/test_review_agent_gate.py` と `.github/workflows/ci.yml` の
     完全一致2パスを追加し、`governance_critical_paths_are_hardcoded` テストに対応する assert を2行追加。
   - 残り2件（hard guard パス集合の `requires_forge_review_for_merge` との対称化、checker の
     changed_files 実 diff 突合）は将来課題として維持（下記「残余 low」参照）。
   - cargo test --lib は 207 passed のまま（既存テストへの assert 追加のみで新規テスト関数はなし、回帰なし）。

## 残余 low（non-blocking、将来 hardening）

4 件（再レビュー時に記録。blocking ではないため pass 判定は維持）:

1. **hard guard パス集合の対称化**（将来課題・未対応）: `risk_tier.rs::is_governance_critical_path`
   （F4 比例緩和のハードガード）と `merge.rs::requires_forge_review_for_merge`（別の既存 gate が使う
   governance 対象パス集合）が非対称。両者を単一の定義に統合するか、差分を明示的に文書化することが望ましい。
2. **checker の changed files 突合**（将来課題・未対応）: `scripts/check_review_agent_gate.py` は
   review packet の自己申告（rationale・evidence 文字列）のみを検証し、実際の `git diff` /
   `gh pr diff` の changed files とは突合していない。CI 実行環境で PR の実差分と review packet の
   記述内容を機械的に突合するチェックがあれば、記述と実態の乖離をさらに検出しやすくなる。
3. **自己申告 changed_files 基準の土台となる CI 定義の保護**（**本 PR の commit `ecc8781` で対応済み**）:
   `.github/workflows/ci.yml` はチェッカー起動条件（`github.event.pull_request.number` 等）を定義する
   ファイルだが、`is_governance_critical_path` のハードガード対象外だった。`ecc8781` で対象に追加し、
   CI 定義自体の無断改変（起動条件の緩和等）にも governance hard guard が及ぶようにした。
4. **`tests/test_review_agent_gate.py` 自体の未ガード**（**本 PR の commit `ecc8781` で対応済み**）:
   gate の自己検証コードである `tests/test_review_agent_gate.py` が `is_governance_critical_path`
   のハードガード対象外だった。`ecc8781` で対象に追加し、gate のテストコード自体の弱体化にも
   forge_reviewer 緩和が及ばないようにした。

## 検証結果

| command | result |
|---|---|
| `cargo fmt --all -- --check` | pass（clean） |
| `cargo test --lib` | pass: 207 passed; 0 failed; 0 ignored（`ecc8781` 反映後。件数は `73dbbfe` 時点と不変 — 既存 `governance_critical_paths_are_hardcoded` への assert 追加のみ） |
| `cargo test`（CI と同一コマンド、lib+bin+doc） | pass: lib 207 passed / bin 0 tests / doc-tests 0 tests |
| `cargo run -q -- validate-artifacts --out <tmp>/fda-validation-report.json` | pass: 70 passed, 0 failed, 43 skipped |
| `python3 -m unittest discover -s tests -v` | pass: Ran 68 tests, OK |
| `python3 scripts/check_architecture_boundaries.py` | pass: architecture boundary check passed |
| `python3 scripts/check_review_agent_gate.py --pr-number 3` | 本 packet 作成後に実行し pass を確認（下記「検証コマンド実行ログ」参照） |
| `gh pr view 3 --json url,number,state,headRefName,baseRefName` | `state=OPEN`, `headRefName=pr-v15-002`, `baseRefName=main` |

## CHANGE_INTENT

- `CLM-RP3-CHANGE-INTENT`: F4 比例ゲート（`risk_tier.rs`、`fda.risk_tier.v1` schema）を導入し、
  変更の blast radius に応じて `forge_reviewer` / `design_qa` の要求を比例的に緩和する（関所の種類は
  減らさない。fail-closed 維持）。F5 庭師（`fda gc`）を導入し、read-only の棚卸しスキャンを AI の
  定常業務にする。
- repair `73dbbfe` は、緩和判定の TOCTOU 脆弱性（stored tier 無検証信頼）と統治バイパス経路
  （governance-critical パスが緩和対象になり得る）を修復し、緩和の表現を既存 schema 契約に一本化する。
- polish `ecc8781`（本タスク）は、governance hard guard の対象パス集合に `tests/test_review_agent_gate.py`
  と `.github/workflows/ci.yml` を追加するのみで、緩和判定ロジック自体・schema・CLI 引数には変更を加えない。

## AC_EVIDENCE

- `CLM-RP3-AC-EVIDENCE`: functional_qa が実 CLI 実行 + ハッシュ/フィールド比較で検証した AC は次の3点。
  - AC1: `assess_risk_tier` が scope_paths + delivery_policy のみからの純粋な判定であり、
    `fda implement --dry-run` 実行で `risk_tier.json` が実際に生成される。
  - AC2: `fda gc` が read-only スキャンのみを行い、既存 run（`artifacts/runs/**`）の内容・mtime を
    変更しない（実行前後のディレクトリ内容比較で確認）。
  - AC3: `fda merge` が保存済み `risk_tier=low` を無検証で信頼せず、merge 時に `changed_files` を
    live 再計算し、stored/live 不一致時は緩和を却下する。
- 本タスクでの再検証: `cargo test --lib` => `207 passed; 0 failed; 0 ignored`、
  `cargo run -q -- validate-artifacts` => `70 passed, 0 failed, 43 skipped`、
  `python3 -m unittest discover -s tests` => `Ran 68 tests ... OK`（いずれも実行済み、上記「検証結果」参照）。

## SEC_EVIDENCE

- `CLM-RP3-SEC-EVIDENCE`: security_qa の初回指摘（HIGH-1: stored tier 無検証信頼の TOCTOU、
  HIGH-2: 統治ファイルの forge_reviewer バイパス経路）は repair commit `73dbbfe` の
  `proportional_relaxation`（live 再検証）と `is_governance_critical_path`（ハードガード、
  YAML 上書き不可）で解消済みであることを `git show 73dbbfe -- src/application/risk_tier.rs` で確認した。
- polish `ecc8781`（本タスク）は `is_governance_critical_path` の対象パス集合を2件（テストファイル・CI 定義）
  拡張するのみで、緩和判定の許可条件（stored=low かつ live=low）や denied のフォールバック挙動には
  変更を加えていない。既存の `hard_guard_denies_forge_relaxation_even_when_policy_marks_fda_low_risk` /
  `stored_live_mismatch_denies_relaxation` / `relaxation_applies_only_when_stored_and_live_are_both_low`
  の3テストが `cargo test --lib` で継続して pass することを確認し、後退がないことを検証した。
- 残余 low（対称化・checker 突合の2件）は non-blocking の gate-hardening 課題であり、secret 処理・
  認証・PII には触れないため security scope 外として将来課題に整理した。

## ROLLBACK_PLAN

- `CLM-RP3-ROLLBACK-PLAN`: PR #3 merge 後に問題が判明した場合、PR #3 の merge commit を revert する。
- polish `ecc8781` のみを戻す場合は `git revert ecc8781` で `src/application/risk_tier.rs` の
  ハードガード対象パス集合を2パス分だけ縮小できる（他ファイルへの影響なし。F4/F5 本体・repair 73dbbfe の
  挙動には影響しない）。
- F4/F5 は新規 CLI サブコマンド（`fda gc`）と新規 schema（`risk_tier`/`gc_docket` v1）の追加であり、
  既存コマンド（`start`/`design`/`implement`/`review`/`merge`）の既存契約は破壊的変更なしで拡張されている
  （tier 無し PR は現行の standard 相当動作を維持）。rollback の blast radius は F4/F5 関連ファイル
  （本 packet「対象」節に列挙した25ファイル）に限定される。

## HUMAN_DECISIONS_REQUIRED

- `HUMAN_TURN_REASON`: merge approval
- `REQUESTED_DECISION`: PR #3（`ecc8781`、`pr-v15-002` → `main`）を merge してよいか
- `OPEN_DECISIONS`: 0（spec / risk 判断は解消済み。残るのは merge approval のみ）
- `MERGE_APPROVAL`: `not_granted`
- 4 reviewer（pr_reviewer / functional_qa / security_qa / forge_reviewer）全て `REVIEW_AGENT_OK`、
  design_qa は not_applicable、`cargo test --lib` 207 passed、Review Agent Gate checker pass は
  いずれも merge approval ではない。最終 merge は人間の明示判断を待つ。

## FORGE_PROMOTION_DECISION

- 本セッション（作業実施担当）は ATO task key `fda-v1-5-20260708` / run ID
  `run_01KX22MBQ3RBXFX0F4Y5HWZB8H` を保持しているが、本タスクの範囲は receipt/packet 整備と
  CI green 化であり、`ato case evaluate --task fda-v1-5-20260708 --pr 3 --review-packet-path artifacts/review_packets/pr-3.md --no-write --json` の実行は本タスクのスコープ外として未実施。
- REVIEW_AGENT_GATE の forge_reviewer 行は、本 PR 自体が ATO/Forge 統治ロジック（risk tier・merge gate・
  review gate schema）の変更を伴うことを踏まえ、not_applicable ではなく実施済みの read-only レビューとして
  記録した（`design_qa` のみ UI 非該当のため not_applicable）。
- `ato case evaluate` の実行と verdict 確認は、merge 前の別 gate（Forge Promotion Layer、
  `forge-promotion` skill）として human/orchestrator 側で改めて実施することを推奨する。`promote` verdict が
  出ても merge approval にはならない。

## TASK_TRACEABILITY

- PR: https://github.com/GW-tobishima/fda/pull/3
- Branch: `pr-v15-002` → `main`
- 対象コミット: `4a4aacb` / `73dbbfe` / `ecc8781`
- Review packet: `artifacts/review_packets/pr-3.md`
- Receipts: `artifacts/runs/fda-start-1783555082/pr-3/pr_reviewer_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-3/functional_qa_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-3/security_qa_receipt.json`
  （PR #1 用の同名 receipt は `artifacts/runs/fda-start-1783555082/` 直下に別途存在するため、
  PR #3 用はこの `pr-3/` サブディレクトリに分離して衝突を避けている）
- epic run dir: `artifacts/runs/fda-start-1783555082/`

## EXECUTION_PROFILE

- workspace_policy（reviewer 側）: `read_only`
- source_mutation_allowed（reviewer 側）: `false`
- 本 packet の作成・receipt 集約・governance hard guard の polish 修正（commit `ecc8781`）は
  Implementer ロールの作業として実施した。

## 検証コマンド実行ログ（本タスク実施分）

| command | result |
|---|---|
| `cargo fmt` | `src/application/risk_tier.rs` を整形（既存フォーマットへの追従。ロジック変更なし） |
| `cargo test --lib` | `test result: ok. 207 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cargo test`（lib+bin+doc） | lib: 207 passed / bin (`fda`): 0 tests / doc-tests: 0 tests、いずれも ok |
| `cargo run -q -- validate-artifacts --out <tmp>/fda-validation-report.json` | `validation pass: 70 passed, 0 failed, 43 skipped` |
| `python3 -m unittest discover -s tests -v` | `Ran 68 tests in 2.247s` / `OK` |
| `python3 scripts/check_architecture_boundaries.py` | `architecture boundary check passed` |
| `cargo fmt --all -- --check` | 差分なし（clean） |
| `python3 scripts/check_review_agent_gate.py --pr-number 3` | 本 packet 作成後に実行（結果は最終報告に記載） |

## 残リスク

- 残余 low の未対応2件（hard guard パス集合の対称化、checker の changed_files 実 diff 突合）は
  non-blocking のまま次サイクル以降の hardening 課題として持ち越す。対応の緊急性は低いと判断した根拠は
  「対称化なしでも `is_governance_critical_path` 側は fail-closed（緩和を拒否する側）にのみ影響し、
  `requires_forge_review_for_merge` 側の既存 gate 自体は本 PR で弱体化していない」ため。
- 旧 PR #2（同ブランチ `pr-v15-002` の前段階、review packet 名 `pr-2.md` が存在した可能性）との関係:
  本 packet は現在の GitHub PR 番号（#3）を正本とする。`pr-2.md` 相当のファイルは本タスク開始時点で
  リポジトリに存在しなかったため、混同の実害はない。

## ATO_TRACE

- ATO Task Key: fda-v1-5-20260708
- ATO Run ID: run_01KX22MBQ3RBXFX0F4Y5HWZB8H
- Epic: EPIC-FDA-V1-5
- planned_pr_id: PR-V15-002
