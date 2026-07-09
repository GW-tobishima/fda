# v15-merge-receipts-pr-v15-004

## これは何か

`EPIC-FDA-V1-5` の planned PR `PR-V15-004`（実 PR: [GW-tobishima/fda#5](https://github.com/GW-tobishima/fda/pull/5)）は、
`fda` の外（GitHub UI / `gh pr merge` 相当）で merge されたため、`fda merge` が生成する
`github_merge_receipt.json` が対応する run dir に残らず、`fda continue --epic` の epic 投影が
古いまま（当該 PR を `not_started` と誤判定）になっていた。

`docs/v1/work_protocol.md` §3「標準ジャーニー」手順 8 に定義された運用:

> fda の外で merge した場合（GitHub UI / gh 直接）は merge receipt が残らず epic 投影が
> 古くなるため、merge 後に github_merge_receipt.json 相当を該当 run dir に追記して投影を
> 最新化する

に従い、`gh pr view 5 --json number,mergedAt,mergeCommit,url,title,headRefOid` 等で取得した
実データから `github_merge_receipt.json` を事後的に作成し、この専用 run dir に格納した。

## 出所データ（2026-07-09 取得、`gh` CLI 実行結果に基づく）

| フィールド | 値 |
|---|---|
| PR | https://github.com/GW-tobishima/fda/pull/5 |
| merge commit (`merge_sha`) | `f4b72af41314f97a811774a4c07ff0b2a2c3bc63` |
| head SHA (`expected_head_sha`) | `ad617087d45706f7de066428307523506b17653f` |
| merged_at | `2026-07-09T04:42:06Z` |
| merged by (GitHub) | `GW-tobishima` |

## 注記

- この receipt は `fda merge --execute` による生成ではなく、`work_protocol.md` 手順 8 の
  事後追記運用の初適用である（merge 実行そのものへの人間承認は GitHub 上で完結済み）。
- 既存 run（`artifacts/runs/fda-start-1783555082/pr-5/` 等）の既存ファイルは一切変更していない。
  本 run dir はこの 1 receipt のためだけの新設 dir。
- `fda continue --epic` は `github_merge_receipt.json` の `epic_id` / `planned_pr_id` /
  `status` / `merge_executed` を artifacts-root 配下の全 run から突合するため、この専用
  run dir に置くだけで epic 投影に反映される（read-only スキャン、既存 receipt 書き換え無し）。
