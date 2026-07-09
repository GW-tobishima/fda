# Review Packet: PR #1 / F6 表層分け（work protocol 単一正本化 + 4 概念カーネル help）

## 対象

- PR: https://github.com/GW-tobishima/fda/pull/1
- Branch: `pr-v15-001`
- Base: `main`
- State: OPEN（未 merge）
- Scope: `docs/v1/work_protocol.md` を FDA 作業プロトコルの単一正本にし、
  `AGENTS.md` / `CLAUDE.md` / `.claude/skills/fda-delivery/SKILL.md` を
  「入口ごとの位置づけ + 正本への参照」だけに削減する（4 重記述による drift の根絶）。
  `docs/v1/system_prompt_authoring_guide.md` を新規追加し、`src/cli/output.rs` の
  `print_help()` を 4 概念カーネル（作業 Work / 証跡 Evidence / 判断 Decision / 知識 Knowledge）で
  グルーピングする。
- 対象コミット: `43a3dc0`（F6 表層分け本体）/ `39612bc`（repair: security_qa 指摘の安全境界後退を修復）/
  `1b1d209`（repair 2: pr_reviewer 指摘の情報ロストを修復）/ `f589691`（repair 3: forge_reviewer low 指摘 3 件の表現改善）。
- 差分: `git diff --stat a03e30c..f589691` = 6 files changed, 518 insertions(+), 136 deletions(-)
  （`.claude/skills/fda-delivery/SKILL.md` / `AGENTS.md` / `CLAUDE.md` / `docs/v1/system_prompt_authoring_guide.md`
  / `docs/v1/work_protocol.md` / `src/cli/output.rs`）。

## REVIEW_AGENT_GATE

MERGE_APPROVAL: not_granted

| role | status | evidence | rationale |
|---|---|---|---|
| pr_reviewer | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/pr_reviewer_receipt.json`; `git show 43a3dc0 -- docs/v1/work_protocol.md`; `git diff 43a3dc0..1b1d209 -- docs/v1/work_protocol.md` | 初回 fail（high1: `REVIEW_AGENT_HOLD` 列挙脱落 / medium3: scope approval 禁止列挙・手動反映注記・`ato agent broker` 手順の脱落 / low1: Forge gate 詳細の簡略化）を指摘。repair `39612bc` + `1b1d209` で全指摘を docs/v1/work_protocol.md へ復元・反映したことを確認し、再レビューで pass。 |
| functional_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/functional_qa_receipt.json`; `cargo test --lib` => 183 passed, 0 failed; `python3 scripts/check_architecture_boundaries.py` => pass | AC1（重複記述なし・正本参照関係が実在）/ AC2（4 概念カーネル groupingの help 表現）/ AC3（`cargo test --lib` 183 pass、architecture gate pass）を全て実コマンドで検証し、初回から pass。 |
| security_qa | REVIEW_AGENT_OK | `artifacts/runs/fda-start-1783555082/security_qa_receipt.json`; `git show 39612bc -- docs/v1/work_protocol.md` | 初回 fail（high1: auto merge 禁止が「常用」へ縮退し単発・例外の抜け道を許容 / medium1: scope approval 禁止列挙の脱落）を指摘。repair `39612bc` で auto merge 禁止を「単発・例外を含め禁止」へ復元し scope approval 禁止列挙を復元したことを確認し、再レビューで pass。 |
| forge_reviewer | REVIEW_AGENT_OK | `git show f589691 -- docs/v1/work_protocol.md`; `docs/v1/work_protocol.md` §2/§3/§4 | 初回から pass。low 指摘 3 件（1: §3 手順2/7 の V1.5 `--by-contract`/`--epic` が未実装である旨の明記不足、2: §4 `ato agent broker` の `--role` が実 CLI では任意なのに必須表記、3: §2 reviewer 禁止列挙に Forge PromotionDecision の自己承認禁止が欠落）を repair 3（commit `f589691`）で解消済み。 |
| design_qa | not_applicable | — | UI / frontend / browser surface に触れない。変更は `docs/`（Markdown）と `src/cli/output.rs` の `println!` によるヘルプテキスト、`AGENTS.md` / `CLAUDE.md` / `SKILL.md` のポインタ化のみで、`fda ui` の Web surface（`src/application/ui.rs`）や `.tsx`/`.css`/`.html` 等の frontend 資材は変更していない。 |

`REVIEW_AGENT_OK` は merge approval ではない。`MERGE_APPROVAL: not_granted` を維持する。

## 検証結果

| command | result |
|---|---|
| `cargo test --lib` | pass: 183 passed; 0 failed; 0 ignored |
| `python3 scripts/check_architecture_boundaries.py` | pass |
| `git diff --stat a03e30c..f589691` | 6 files changed, 518 insertions(+), 136 deletions(-) |
| `cargo run -q -- review --artifacts artifacts/runs/fda-start-1783555082 --target-repo .` | 実行成功だが verdict=blocked（`implementation_receipt.json` / `external_pr_receipt.json` が epic run dir に無いため）。epic run dir はまだ設計段階の成果物のみを持ち、実装完了後の receipt を含まないため、`fda review` の自動集約経路は使わず本 packet を手動で作成した（fail-soft フォールバック）。 |
| `python3 scripts/check_review_agent_gate.py --pr-number 1` | 本 packet 作成後に実行し pass を確認（下記「検証コマンド実行ログ」参照）。 |
| `gh pr view 1 --json url,number,state,headRefName,baseRefName` | `state=OPEN`, `headRefName=pr-v15-001`, `baseRefName=main` |

## repair 履歴（fail → repair → pass、2 サイクル + 1 polish）

1. **サイクル 1（pr_reviewer）**: 初回 fail — high1: `REVIEW_AGENT_HOLD` 列挙脱落 / medium3: scope approval 禁止列挙・手動反映注記・`ato agent broker` 手順の脱落 / low1: Forge gate 詳細の簡略化。
   → repair commit `39612bc` + repair 2 commit `1b1d209` で `docs/v1/work_protocol.md` へ全項目を復元・反映 → 再レビューで pass。
2. **サイクル 2（security_qa）**: 初回 fail — high1: auto merge 禁止が「常用」に縮退（単発・例外の抜け道を許容する表現） / medium1: scope approval 禁止列挙の脱落。
   → repair commit `39612bc` で auto merge 禁止を「単発・例外を含め禁止」へ、scope approval を禁止列挙へ復元 → 再レビューで pass。
3. **polish（forge_reviewer）**: forge_reviewer は初回から pass。low 指摘 3 件（V1.5 未実装明記・`--role` 任意化・Forge PromotionDecision 自己承認禁止の明記）を repair 3 commit `f589691`（本タスクで作成、コミットメッセージ 1 行目
   `docs: forge_reviewer low 指摘 3 件の表現改善 (PR-V15-001 repair 3)`）で反映。
4. functional_qa は初回から pass。fail サイクルなし。

## CHANGE_INTENT

- `CLM-RP1-CHANGE-INTENT`: FDA 作業プロトコルの正本を `docs/v1/work_protocol.md` に一本化し、
  `AGENTS.md` / `CLAUDE.md` / `.claude/skills/fda-delivery/SKILL.md` の重複本文を「入口ごとの位置づけ + 正本参照」へ縮退する。
- `docs/v1/system_prompt_authoring_guide.md` を新規追加し、グローバルシステムプロンプト書き換えの正典とする。
- `src/cli/output.rs` の `print_help()` を 4 概念カーネル（Work / Evidence / Decision / Knowledge）でグルーピングする
  （出力文字列のみの変更で、CLI 引数解析・実行経路・成果物契約には影響しない）。
- repair 3（本タスク）は forge_reviewer の low 指摘 3 件の表現改善のみを `docs/v1/work_protocol.md` に適用し、
  それ以外のドキュメント・`src/` への変更は行っていない。

## AC_EVIDENCE

- `CLM-RP1-AC-EVIDENCE`: functional_qa が検証した AC は次の3点。
  - AC1: `docs/v1/work_protocol.md` 本文に重複記述がなく、`AGENTS.md` / `CLAUDE.md` / `SKILL.md` が正本を参照する構造になっている。
  - AC2: help 出力が 4 概念カーネル（作業/証跡/判断/知識）でグルーピングされている。
  - AC3: `cargo test --lib` が 183 passed / 0 failed、`python3 scripts/check_architecture_boundaries.py` が pass。
- 本タスクでの再検証: `cargo test --lib` => `183 passed; 0 failed; 0 ignored`（実行済み、下記ログ参照）。

## SEC_EVIDENCE

- `CLM-RP1-SEC-EVIDENCE`: security_qa の初回指摘（auto merge 禁止の安全境界後退、scope approval 脱落）は
  repair commit `39612bc` で解消済みであることを `git show 39612bc -- docs/v1/work_protocol.md` で確認した。
- repair 3（本タスク）は `docs/v1/work_protocol.md` の §2 reviewer 禁止列挙に
  「Forge PromotionDecision の自己承認をしない」を追加し、security 境界をさらに明確化した（後退なし）。
- 本タスクでの変更は `docs/v1/work_protocol.md` の文言改善のみで、`src/` のコード・secret 処理・
  adapter runtime には触れていない。

## ROLLBACK_PLAN

- `CLM-RP1-ROLLBACK-PLAN`: PR #1 merge 後に問題が判明した場合、PR #1 の merge commit を revert する。
- repair 3（`f589691`）のみを戻す場合は `git revert f589691` で `docs/v1/work_protocol.md` の該当 3 箇所を復元前の表現へ戻せる
  （他ファイルへの影響なし）。
- runtime code（`src/cli/output.rs` の help 文字列を除く）には変更がないため、rollback の blast radius は
  docs + help テキストに限定される。

## HUMAN_DECISIONS_REQUIRED

- `HUMAN_TURN_REASON`: merge approval
- `REQUESTED_DECISION`: PR #1（`f589691`、`pr-v15-001` → `main`）を merge してよいか
- `OPEN_DECISIONS`: 0（spec / risk 判断は解消済み。残るのは merge approval のみ）
- `MERGE_APPROVAL`: `not_granted`
- 4 reviewer（pr_reviewer / functional_qa / security_qa / forge_reviewer）全て `REVIEW_AGENT_OK`、
  design_qa は not_applicable、`cargo test --lib` 183 passed、Review Agent Gate checker pass は
  いずれも merge approval ではない。最終 merge は人間の明示判断を待つ。

## FORGE_PROMOTION_DECISION

- 本セッション（Implementer サブエージェント・集約担当）では ATO task key を保持していないため
  `ato case evaluate --task <key> --pr 1 --review-packet-path artifacts/review_packets/pr-1.md --no-write --json`
  は実行していない。
- REVIEW_AGENT_GATE の forge_reviewer 行がすでに Forge/ATO 証跡・handoff・human decision 境界の
  read-only reviewer 確認と、PromotionDecision 自己承認禁止の明記（repair 3）をカバーしている。
- `ato case evaluate` の実行と verdict 確認は、merge 前の別 gate（Forge Promotion Layer、`forge-promotion` skill）として
  human/orchestrator 側で改めて実施することを推奨する。`promote` verdict が出ても merge approval にはならない。

## TASK_TRACEABILITY

- PR: https://github.com/GW-tobishima/fda/pull/1
- Branch: `pr-v15-001` → `main`
- 対象コミット: `43a3dc0` / `39612bc` / `1b1d209` / `f589691`
- Review packet: `artifacts/review_packets/pr-1.md`
- Receipts: `artifacts/runs/fda-start-1783555082/pr_reviewer_receipt.json` /
  `artifacts/runs/fda-start-1783555082/functional_qa_receipt.json` /
  `artifacts/runs/fda-start-1783555082/security_qa_receipt.json`
- epic run dir: `artifacts/runs/fda-start-1783555082/`

## EXECUTION_PROFILE

- workspace_policy（reviewer 側）: `read_only`
- source_mutation_allowed（reviewer 側）: `false`
- 本 packet の作成・receipt 集約は Implementer サブエージェント（集約・記帳担当）が行った。

## 検証コマンド実行ログ（本タスク実施分）

| command | result |
|---|---|
| `cargo run -q -- review --artifacts artifacts/runs/fda-start-1783555082 --target-repo .` | 実行成功。出力: `Functional QA Gate: blocked` / `Security QA Gate: blocked` / `QA verdict: blocked`。理由: `implementation_receipt.json` と `external_pr_receipt.json` が epic run dir に無いため。fallback として本 packet を手動作成した。 |
| `cargo test --lib` | `test result: ok. 183 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `python3 scripts/check_review_agent_gate.py --pr-number 1` | 本 packet 作成後に実行（結果は最終報告に記載）。 |

## 残リスク

- epic run dir（`artifacts/runs/fda-start-1783555082/`）に `implementation_receipt.json` /
  `external_pr_receipt.json` が無いため、`fda review` の自動集約（`review_agent_gate_packet.md` の生成）は
  今回使えなかった。本 packet は fallback として手動作成したものであり、将来これらの receipt が
  epic run dir に揃った時点で `cargo run -- review` を再実行し、自動生成物と本 packet の内容を
  突合することを推奨する。
- `artifacts/review_packets/pr-v15-001.md`（ブランチ名ベース、未 push のローカルマージを前提にした別内容の
  review packet）が同ディレクトリに未追跡ファイルとして存在する。本タスクのスコープ外のため変更していないが、
  PR #1 の GitHub merge フローとは矛盾する記述（`HUMAN_DECISIONS_REQUIRED` が「承認済み」となっている）を
  含むため、混同しないよう注意。本 packet（`pr-1.md`）が PR #1 の正本である。
