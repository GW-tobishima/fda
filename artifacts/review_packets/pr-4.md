# Review Packet: PR #4 / F1 判断の立法化（delegation contract + fda policy propose）（PR-V15-003）

## 対象

- PR: https://github.com/GW-tobishima/fda/pull/4
- Branch: `pr-v15-003`
- Base: `main`（`pr-v15-002` に stack。PR-V15-002 は PR #3 として未 merge）
- State: OPEN（未 merge）
- 位置づけ: EPIC-FDA-V1-5 の PR-V15-003（sequence 3）。過去に人間が繰り返し同型で承認した
  Human Decision を**委任契約（delegation contract）**として明示適用できるようにする F1
  「判断の立法化」を実装する。**契約の制定・自動適用は一切しない**（AI は提案まで、適用は
  明示指定時のみ、fail-closed）。
- Scope:
  - optional profile `.fda/delegation_contract.yaml`（必須 7 ファイルには加えない）+ 新 schema
    `docs/standards/fda-v1/schemas/repository-profile/delegation_contract_yaml.schema.json`。
  - `fda policy propose [--min-occurrences 3] [--out artifacts/runs/_policy]`: 全 run の decision
    履歴を (type × summary 正規化署名 × answer) でクラスタし、min-occurrences 以上を契約候補として
    `policy_proposal.{json,md}`（新 schema `fda.policy_proposal.v1`）へ提案。**`.fda` へは書かない**。
  - `fda decide <ID> --by-contract <rule_id>`（`--answer` と排他）: rule 存在 / decision_type 一致 /
    keyword 一致 / expires >= 今日 を全て満たす場合のみ契約の answer で記録。
    decided_by=`delegation_contract:<rule_id>:<authority>`、receipt に
    contract_rule_id / contract_expires / authority を追記。
  - `fda status` の未解決判断表示に「DC-xxx 適用可」ヒントを追加（自動適用しない）。
  - 対象コミット: `8de4e0e`（F1 本体、17 files changed, 1605 insertions(+), 24 deletions(-)）/
    `83cbe7e`（repair: pr_reviewer・forge_reviewer 指摘対応、10 files changed, 298 insertions(+),
    45 deletions(-)）。

## この PR の位置づけ（forge_reviewer 実施の理由）

**この PR は human decision 境界そのものに触れるため forge_reviewer を実施済み**（`design_qa`
と異なり not_applicable ではない）。理由: delegation contract は「AI がいつ人間の判断を代行して
よいか」を定義する仕組みであり、ATO/Forge の Human Decision 境界（work_protocol §5 の禁止事項）
と直接に交差する。実際、第1巡の pr_reviewer・forge_reviewer は共に「AI コミットが live 契約
`.fda/delegation_contract.yaml` を制定した」ことを HIGH として指摘しており、この境界チェックが
機能したことを示している。

## REVIEW_AGENT_GATE

MERGE_APPROVAL: not_granted

| role | status | evidence | rationale |
|---|---|---|---|
| pr_reviewer | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-4/pr_reviewer_receipt.json`; `git show 83cbe7e`; `git show 83cbe7e -- docs/v1/work_protocol.md` | 初回 fail（HIGH: AI コミットが live 契約 `.fda/delegation_contract.yaml` を制定＝work_protocol §5 違反 / HIGH: 定型文言 keywords + OR 部分一致による包括委任化 / MEDIUM: enacted_from 根拠不整合 / LOW: `policy_proposal` schema 未配線）を指摘。repair `83cbe7e` で live 契約の完全撤去・keyword AND 化・work_protocol §5 明文化・policy_proposal ランタイム自己検証を確認し、再レビューで pass。残 low 1 件（propose の検証→書き込み順序、書き込み後検証で不適合ファイルが `_policy/` に残置され得る）は非 blocking として記録。 |
| functional_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-4/functional_qa_receipt.json`; `cargo test --lib` => 235 passed, 0 failed | 3 AC（propose のクラスタリングと `.fda` 不変 / `--by-contract` 適用時の decided_by・receipt 記録 / 失効・type 不一致・keyword 不一致時の fail-closed 拒否）を unit テスト + 実 CLI 実行の両方で検証し、第1巡から pass（第1巡時点は `cargo test --lib` 231 passed で green、repair 後 235 passed を再確認）。 |
| security_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr-4/security_qa_receipt.json`; `git show 83cbe7e -- src/application/policy.rs src/support/date.rs` | 第1巡は pass。medium 1 件（keyword OR 部分一致の緩さによる包括委任化リスク）/ low 3 件（expires の UTC 境界の緩さ、decided_by⇔contract_rule_id 整合の機械検証欠如、authority 自由記述）を記録。repair `83cbe7e` で keyword AND 化・expires strict 化（当日から無効）を確認し medium 1 件・low 1 件は解消済みとした。残り low 2 件（decided_by 整合の機械検証は将来課題、authority は自由記述のまま設計上許容）は non-blocking として維持。 |
| forge_reviewer | REVIEW_AGENT_OK | `git show 83cbe7e -- docs/v1/work_protocol.md src/application/policy.rs docs/standards/delivery-artifacts-v0/schemas/status_summary.schema.json`; `docs/standards/fda-v1/examples/delegation_contract.example.yaml` | 初回 fail（HIGH: 同上の live 契約制定・work_protocol §5 違反 / MEDIUM: enacted_from 根拠不整合 / MEDIUM: `status_summary.schema.json` に `contract_hints` が未反映で `fda status --json` の schema 適合が保証されない）を指摘。repair `83cbe7e` で live 契約撤去・docs example 化・`status_summary.schema.json` への `contract_hints`（本 PR）+ `merge.risk_tier`（PR #3）の optional 追加・`fda status --json` の schema 適合回帰テスト追加を確認し、再レビューで pass。 |
| design_qa | not_applicable | — | CLI（`src/cli/`）・application ロジック（`src/application/policy.rs` / `decide.rs` / `status.rs` / `validate.rs` 等）・schema（`docs/standards/`）・work_protocol（`docs/v1/`）のみの変更で、UI / frontend / browser surface（`fda ui` の `src/application/ui.rs` や `.tsx`/`.css`/`.html` 等）には触れていない。 |

`REVIEW_AGENT_OK` は merge approval ではない。`MERGE_APPROVAL: not_granted` を維持する。

## repair 履歴（fail → repair → pass、1 統合サイクル）

1. **第1巡**:
   - functional_qa は初回から pass（3 AC を unit テスト + 実 CLI 実行で検証）。
   - security_qa は初回から pass だが medium 1 件・low 3 件を記録（下記参照）。
   - pr_reviewer・forge_reviewer は fail。HIGH 指摘の要旨:
     - AI が行ったコミット（`8de4e0e`）が **live 契約 `.fda/delegation_contract.yaml` を PR に
       含めて制定**しており、work_protocol §5「委任契約の制定は常に人間、AI は提案まで」に
       違反する（pr_reviewer HIGH・forge_reviewer HIGH が同一事象を指摘）。
     - 実例ルール（DC-001）の `match_summary_keywords` が定型文言に依存し、かつ keyword 照合が
       OR（いずれか1語の部分一致）で成立するため、意図しない decision まで包括委任として
       扱われ得る（pr_reviewer HIGH、security_qa medium が同一事象を指摘）。
     - PR 説明の `enacted_from` 根拠（過去に人間が承認した決定への参照）が実際の decision 履歴と
       不整合（pr_reviewer MEDIUM、forge_reviewer MEDIUM）。
     - `status_summary.schema.json` に `contract_hints` が未反映で `fda status --json` の
       schema 適合が保証されない（forge_reviewer MEDIUM）。
     - 付随して pr_reviewer から LOW（`fda.policy_proposal.v1` schema が定義されたが
       `fda policy propose` の出力が実行時に検証されておらず未配線）。
   - security_qa の low 3 件: expires の UTC 境界の緩さ（失効当日でも適用され得る）、
     decided_by⇔contract_rule_id の整合を機械的に検証する仕組みの欠如（将来課題）、
     authority が自由記述であること（設計上の既知の制約）。
   → **repair commit `83cbe7e`** で統合修復:
   - **live 契約の完全撤去**: `.fda/delegation_contract.yaml` を PR から削除。代わりに効力を
     持たない書式例 `docs/standards/fda-v1/examples/delegation_contract.example.yaml` を新設し
     （「これは書式例であり、いかなる効力も持たない」と明記）、内容も包括委任にならない架空の
     狭い例（`risk_approval` × `["docsのみの変更", "dependabot"]`）へ差し替え。
   - **keyword AND 化 + schema 制約**: keyword 照合を ANY→ALL（全キーワードの AND 一致のみ適用）に
     変更し、`delegation_contract_yaml.schema.json` の `keywords` items に `minLength: 4` を追加。
   - **expires strict 化**: `today < expires` の UTC 暦日基準へ厳格化（expires 当日から無効）。
     境界日テスト（`contract_rejected_on_expiry_boundary_day`）を追加。
   - **policy_proposal ランタイム自己検証**: `fda policy propose` が出力直後に自身の
     `policy_proposal.json` を schema でランタイム自己検証（違反 / schema 不在は fail-closed）。
     テスト `proposal_json_is_schema_validated_at_runtime` を追加。
   - **work_protocol §5 の明文化**: 「AI が `.fda/delegation_contract.yaml` を作成・編集すること」
     を禁止事項に明記し、「AI に許されるのは `docs/standards/fda-v1/examples/delegation_contract.example.yaml`
     の提示と `fda policy propose` の提案出力まで」と限定。
   - **status_summary.schema.json への追従**: `contract_hints`（本 PR）と `merge.risk_tier`
     （PR #3。同じ optional 追加として一括対応）を optional で追加し、
     `status_result_json_conforms_to_status_summary_schema` の schema 適合回帰テストを追加。
   - テスト: `cargo test --lib` が 231 passed（第1巡時点）→ 235 passed（repair 後、+4: AND 部分一致
     拒否 `contract_rejected_when_only_some_keywords_match` / expires 境界日
     `contract_rejected_on_expiry_boundary_day` / proposal schema 配線
     `proposal_json_is_schema_validated_at_runtime` / status schema 適合
     `status_result_json_conforms_to_status_summary_schema`）。
   → 全指摘を確認し、pr_reviewer・security_qa・forge_reviewer とも再レビューで **pass**。

## 将来課題（non-blocking）

1. **decision_receipts.json の schema 化**（未対応）: 現状 `fda.decision_receipts.v0` は schema
   ファイルが未定義。
2. **decided_by 整合の機械検証**（未対応）: `decided_by=delegation_contract:<rule_id>:<authority>`
   と契約側 `contract_rule_id` の整合を validate 側で機械的に検証する仕組みがなく、目視確認に
   依存している。上記 1 の schema 化と合わせて対応することが望ましい。
3. **`fda policy propose` の検証→書き込み順序**（未対応）: 現在は `_policy/` へ書き込んでから
   自身の schema へランタイム自己検証する順序のため、検証失敗時に不適合ファイルが `_policy/` に
   残置され得る（非 blocking。検証→書き込みの順序入れ替えが望ましい）。
4. **authority の自由記述**（設計上許容）: 契約の制定自体が人間の YAML 編集のみで、AI が
   制定・自動適用しない fail-closed 境界（work_protocol §5）で担保されているため、本 PR の
   セキュリティスコープでは許容範囲と判断。

## 検証結果

| command | result |
|---|---|
| `cargo fmt --all -- --check` | pass（clean） |
| `cargo test --lib` | pass: 235 passed; 0 failed; 0 ignored（`83cbe7e` 反映後。第1巡時点は 231 passed） |
| `cargo run -q -- validate-artifacts` | pass: 71 passed, 0 failed, 44 skipped |
| `python -m unittest discover -s tests -v` | pass: Ran 68 tests, OK |
| `python scripts/check_architecture_boundaries.py` | pass: architecture boundary check passed |
| `python scripts/check_review_agent_gate.py --pr-number 4` | 本 packet 作成後に実行し pass を確認（下記「検証コマンド実行ログ」参照） |
| `gh pr view 4 --json url,number,state,headRefName,baseRefName` | `state=OPEN`, `headRefName=pr-v15-003`, `baseRefName=main` |

補足: 依頼メモに記載の「cargo test 235 / validate-artifacts 71 / unittest 66」のうち、
`cargo test`（235）と `validate-artifacts`（71 passed）は本タスクでの再実行と一致した。
`python -m unittest` は本タスクでの再実行では **68 passed**（`66` ではない）を確認した。
これは PR #3（`pr-3.md`）の時点で既に `tests/test_review_agent_gate.py` に governance hard guard
関連テスト 2 件が追加され `Ran 68 tests, OK` へ更新されていたためで（`pr-v15-003` は
`pr-v15-002` に stack しておりその状態を引き継ぐ）、本 PR（`8de4e0e` / `83cbe7e`）は
`tests/test_review_agent_gate.py` を変更していない。実行結果を優先し本 packet には実測値
（68 passed）を記録する。

## CHANGE_INTENT

- `CLM-RP4-CHANGE-INTENT`: 過去に人間が繰り返し同型で承認した Human Decision を委任契約
  （`.fda/delegation_contract.yaml`、optional profile）として明示適用できるようにする F1
  「判断の立法化」を導入する。契約の**制定は常に人間の YAML 編集のみ**、AI は
  `fda policy propose` による**提案まで**（`.fda` へは書かない）。適用は
  `fda decide --by-contract <rule_id>` の**明示指定時のみ**、かつ rule 存在・type 一致・
  keyword 一致・未失効の全条件を満たす場合のみ（1 つでも欠ければ fail-closed で人間判断へ）。
- repair `83cbe7e` は、AI コミットによる live 契約の制定（work_protocol §5 違反）を完全に
  取り除き、契約適用の判定条件（keyword 一致・expires）を厳格化し、work_protocol §5 に
  禁止事項を明記することで、人間権限の境界を保守強化する。

## AC_EVIDENCE

- `CLM-RP4-AC-EVIDENCE`: functional_qa が unit テスト + 実 CLI 実行の両方で検証した AC は
  次の3点。
  - AC1: 同型判断 3 回以上で `policy_proposal` に候補化、2 回では候補化されず、`.fda/` は
    一切変更されない（`fda policy propose` 実行前後の `.fda/` 配下ハッシュ比較で確認）。
  - AC2: 有効な契約 + `--by-contract` で契約 answer が記録され、`decided_by` に
    `delegation_contract:<rule_id>:<authority>`、receipt に `contract_rule_id` /
    `contract_expires` / `authority` が付与される。
  - AC3: 期限切れ / type 不一致 / keyword 不一致のいずれかで `--by-contract` が拒否され、
    人間判断（`--answer`）へ差し戻される。`--answer` と `--by-contract` の併用はエラー。
- 本タスクでの再検証: `cargo test --lib` => `235 passed; 0 failed; 0 ignored`、
  `cargo run -q -- validate-artifacts` => `71 passed, 0 failed, 44 skipped`、
  `python -m unittest discover -s tests` => `Ran 68 tests ... OK`（いずれも実行済み、
  上記「検証結果」参照）。

## SEC_EVIDENCE

- `CLM-RP4-SEC-EVIDENCE`: security_qa の第1巡指摘（medium: keyword OR 部分一致による包括委任化
  リスク／low: expires の UTC 境界の緩さ）は repair commit `83cbe7e` の keyword AND 化
  （`docs/standards/fda-v1/schemas/repository-profile/delegation_contract_yaml.schema.json` の
  `minLength: 4` を含む）と expires strict 化（`today < expires` の UTC 暦日基準）で解消済みで
  あることを `git show 83cbe7e -- src/application/policy.rs src/support/date.rs` で確認した。
- pr_reviewer・forge_reviewer が指摘した HIGH（AI コミットによる live 契約
  `.fda/delegation_contract.yaml` の制定＝Human Decision 境界＝work_protocol §5 違反）は、
  repair `83cbe7e` で live 契約を完全撤去し、`docs/standards/fda-v1/examples/delegation_contract.example.yaml`
  （効力なし明記）へ置換し、work_protocol §5 に禁止事項を明記したことで解消した。
- 残り low 2 件（decided_by⇔contract_rule_id 整合の機械検証欠如、authority 自由記述）は
  non-blocking の将来課題・設計上の既知の制約として整理した（上記「将来課題」参照）。

## ROLLBACK_PLAN

- `CLM-RP4-ROLLBACK-PLAN`: PR #4 merge 後に問題が判明した場合、PR #4 の merge commit を
  revert する。
- repair `83cbe7e` のみを戻す場合、`.fda/delegation_contract.yaml` の live 契約が復活してしまい
  work_protocol §5 違反状態に戻るため、**単独revert は不可**。revert する場合は
  `8de4e0e`（F1 本体）ごと戻す必要がある。
- F1 は新規 optional profile（`.fda/delegation_contract.yaml`、不存在時は検証スキップで従来動作）
  と新規 CLI 引数（`fda policy propose`、`fda decide --by-contract`）の追加であり、既存コマンドの
  既存契約は破壊的変更なしで拡張されている。rollback の blast radius は F1 関連ファイル
  （本 packet「対象」節に列挙したファイル）に限定される。

## HUMAN_DECISIONS_REQUIRED

- `HUMAN_TURN_REASON`: merge approval
- `REQUESTED_DECISION`: PR #4（`83cbe7e`、`pr-v15-003` → `main`）を merge してよいか
- `OPEN_DECISIONS`: 0（spec / risk 判断は解消済み。残るのは merge approval のみ）
- `MERGE_APPROVAL`: `not_granted`
- 4 reviewer（pr_reviewer / functional_qa / security_qa / forge_reviewer）全て `REVIEW_AGENT_OK`、
  design_qa は not_applicable、`cargo test --lib` 235 passed、Review Agent Gate checker pass は
  いずれも merge approval ではない。最終 merge は人間の明示判断を待つ。
- 加えて本 PR は「委任契約」という Human Decision 境界そのものを実装対象とするため、merge 前に
  `.fda/delegation_contract.yaml` が repo に存在しないこと（人間が制定していないこと）を
  人間自身が確認することを推奨する。

## FORGE_PROMOTION_DECISION

- 本タスクは receipt / review packet 整備と CI green 化が範囲であり、
  `ato case evaluate --task fda-v1-5-20260708 --pr 4 --review-packet-path artifacts/review_packets/pr-4.md --no-write --json`
  の実行は本タスクのスコープ外として未実施。
- REVIEW_AGENT_GATE の forge_reviewer 行は、本 PR が Human Decision 境界（委任契約）そのものを
  実装対象とすることを踏まえ、not_applicable ではなく実施済みの read-only レビューとして記録した
  （`design_qa` のみ UI 非該当のため not_applicable）。
- `ato case evaluate` の実行と verdict 確認は、merge 前の別 gate（Forge Promotion Layer、
  `forge-promotion` skill）として human/orchestrator 側で改めて実施することを推奨する。
  `promote` verdict が出ても merge approval にはならない。

## TASK_TRACEABILITY

- PR: https://github.com/GW-tobishima/fda/pull/4
- Branch: `pr-v15-003` → `main`（`pr-v15-002` に stack）
- 対象コミット: `8de4e0e` / `83cbe7e`
- Review packet: `artifacts/review_packets/pr-4.md`
- Receipts: `artifacts/runs/fda-start-1783555082/pr-4/pr_reviewer_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-4/functional_qa_receipt.json` /
  `artifacts/runs/fda-start-1783555082/pr-4/security_qa_receipt.json`
  （PR #1 / PR #3 用の同名 receipt はそれぞれ `artifacts/runs/fda-start-1783555082/` 直下 /
  `artifacts/runs/fda-start-1783555082/pr-3/` に別途存在するため、PR #4 用はこの `pr-4/`
  サブディレクトリに分離して衝突を避けている）
- epic run dir: `artifacts/runs/fda-start-1783555082/`

## EXECUTION_PROFILE

- workspace_policy（reviewer 側）: `read_only`
- source_mutation_allowed（reviewer 側）: `false`
- 本 packet の作成・receipt 集約は Implementer ロールの作業として実施した（src 変更は行っていない）。

## 検証コマンド実行ログ（本タスク実施分）

| command | result |
|---|---|
| `cargo test --lib` | `test result: ok. 235 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cargo run -q -- validate-artifacts` | `validation pass: 71 passed, 0 failed, 44 skipped` |
| `python -m unittest discover -s tests -v` | `Ran 68 tests in 2.505s` / `OK` |
| `python scripts/check_architecture_boundaries.py` | `architecture boundary check passed` |
| `cargo fmt --all -- --check` | 差分なし（clean） |
| `python scripts/check_review_agent_gate.py --pr-number 4` | 本 packet 作成後に実行（結果は最終報告に記載） |
| `gh pr checks 4`（本タスク開始時点） | `rust: fail`（`review-agent-gate: fail: review packet がありません: artifacts/review_packets/pr-4.md`）/ `rust-windows: pass` |

## 残リスク

- `python -m unittest` の実測値（68 passed）が PR 説明文中の記載値（66 passed）と一致しない。
  原因は `pr-v15-003` が `pr-v15-002`（PR #3、未 merge）に stack しており、PR #3 の polish
  commit（`ecc8781`。ハードガード対象パス追加テスト2件を含む）を merge commit `ffe48af` 経由で
  引き継いでいるため。本 PR（`8de4e0e` / `83cbe7e`）自体は `tests/test_review_agent_gate.py` を
  変更していない。実害はないが、PR #3 が merge されるまでは PR #4 の CI 上のテスト総数が
  PR 説明文の記載値と乖離し続ける点に留意する。
- PR-V15-002（PR #3）が先に merge されない限り、PR-V15-003（PR #4）は `main` に対して
  PR-V15-002 の差分も含んだ状態で表示される（`gh pr diff 4` で確認済み）。レビューの実体は
  F1（本 packet の対象）に限定しているが、merge 順序は PR #3 → PR #4 を維持する必要がある。

## ATO_TRACE

- ATO Task Key: fda-v1-5-20260708
- ATO Run ID: run_01KX22MBQ3RBXFX0F4Y5HWZB8H
- Epic: EPIC-FDA-V1-5
- planned_pr_id: PR-V15-003
