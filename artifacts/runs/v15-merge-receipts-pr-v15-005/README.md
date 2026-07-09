# v15-merge-receipts-pr-v15-005

## これは何か

`EPIC-FDA-V1-5` の planned PR `PR-V15-005`（実 PR: [GW-tobishima/fda#6](https://github.com/GW-tobishima/fda/pull/6)）は、
`fda` の外（GitHub UI / `gh pr merge` 相当）で merge されたため、`fda merge` が生成する
`github_merge_receipt.json` が対応する run dir に残らず、`fda continue --epic` の epic 投影が
古いまま（当該 PR を `not_started` と誤判定）になっていた。

`docs/v1/work_protocol.md` §3「標準ジャーニー」手順 8 に定義された運用:

> fda の外で merge した場合（GitHub UI / gh 直接）は merge receipt が残らず epic 投影が
> 古くなるため、merge 後に github_merge_receipt.json 相当を該当 run dir に追記して投影を
> 最新化する

に従い、`gh pr view 6 --json number,mergedAt,mergeCommit,url,title,headRefOid` 等で取得した
実データから `github_merge_receipt.json` を事後的に作成し、この専用 run dir に格納した。

## 出所データ（2026-07-09 取得、`gh` CLI 実行結果に基づく）

| フィールド | 値 |
|---|---|
| PR | https://github.com/GW-tobishima/fda/pull/6 |
| merge commit (`merge_sha`) | `845a051c4081ad617cfd69c5f5ed4fbfe529258b` |
| head SHA (`expected_head_sha`) | `5ccb810267d58ff68ca026fb66af21f898ee49b4` |
| merged_at | `2026-07-09T04:42:46Z` |
| merged by (GitHub) | `GW-tobishima` |

## 注記

- この receipt は `fda merge --execute` による生成ではなく、`work_protocol.md` 手順 8 の
  事後追記運用の初適用である（merge 実行そのものへの人間承認は GitHub 上で完結済み）。
- 既存 run（`artifacts/runs/fda-start-1783555082/pr-6/` 等）の既存ファイルは一切変更していない。
  本 run dir はこの 1 receipt のためだけの新設 dir。
- `fda continue --epic` は `github_merge_receipt.json` の `epic_id` / `planned_pr_id` /
  `status` / `merge_executed` を artifacts-root 配下の全 run から突合するため、この専用
  run dir に置くだけで epic 投影に反映される（read-only スキャン、既存 receipt 書き換え無し）。

## schema ファイルについての注記

`docs/standards/delivery-artifacts-v0/schemas/github_merge_receipt.schema.json` は本リポジトリの
現時点（main / このタスク時点）には**存在しない**（`git log --all --diff-filter=A` でも追加履歴なし。
存在するのは `docs/standards/delivery-artifacts-v0/examples/fda_v1_operational_e2e/github_merge_receipt.json`
という example のみ）。そのため 5 receipt のフィールド構成は下記を正典として採用した:

- 生成ロジック側の正本: `src/rendering/merge.rs` の `github_merge_receipt_success` 等が
  実際に生成する field set（`schema_version` / `receipt_id` / `program_id` / `epic_id` /
  `planned_pr_id` / `actual_pr_url` / `status` / `merge_executed` / `merge_method` /
  `expected_head_sha` / `merge_sha` / `merged_at` / `actor` / `started_at` / `completed_at` /
  `failure_reason` / `resume_command` / `receipt_collection_command` / `evidence_links` /
  `rollback_plan` / `collected_at_unix_seconds`）。
- 消費ロジック側の正本: `src/application/epic.rs` の `read_receipt_with_pr_id` /
  `receipt_epic_matches`（`planned_pr_id` 必須・`epic_id` 一致必須・
  `status == "succeeded" || merge_executed == true` で merged 判定）。

`cargo run -q -- validate-artifacts` は `--schemas` 配下の `*.schema.json` を起点に対応する
artifact を探す実装（`validate_artifacts_with_ports`）のため、`github_merge_receipt.schema.json`
が存在しない現状では本 receipt に対する schema チェックは一切実行されない（pass/fail/skipped
のいずれのレコードも出ない）。この事実確認と、jsonschema 相当の手動照合結果は
`artifacts/runs/v15-merge-receipts-pr-v15-001/` 直下ではなくタスク最終報告に記載する。
