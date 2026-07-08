# V1-PIVOT-007 Development Handoff

## EPIC全体のゴール

FDA V1を Codex CLI primary の Delivery Skill Pack / Work Protocol として成立させ、`.fda/` Profile Gate、current Codex CLI implementer handoff、Review Agent Gateを必須運用にする。MCP direct implementerはV1.5 optional automationとして残す。

## 今回のPR境界

- 対象: V1-PIVOT-007 Codex subagent / read-only reviewer前提のReview Agent Gate更新
- 含む:
  - `review_agent_gate.schema.json` の追加
  - `artifact_inventory` の `review_agent_gate` artifact type追加
  - `fda review` の出力に `pr_reviewer_prompt.md`、`pr_reviewer_receipt.json`、`review_agent_gate.json` を追加
  - `review_agent_gate.json` に必須 reviewer として `pr_reviewer`、`functional_qa`、`security_qa` を記録
  - `forge_reviewer`、`design_qa` は条件未発火時に `not_applicable` と理由を記録
  - reviewer は read-only、source mutation不可、merge approval不可として記録
  - V1 docs / README の Review Agent Gate 表現更新
- 含まない:
  - 実際のCodex subagent起動
  - GitHub review comment収集
  - `artifacts/review_packets/pr-<PR番号>.md` の自動生成
  - merge approval
  - auto merge

## 実装内容

- `src/rendering/review.rs` に PR reviewer receipt、PR reviewer prompt、Review Agent Gate aggregate JSONの生成関数を追加した。
- `src/application/review.rs` で `fda review` 実行時に新artifactを出力するようにした。
- `docs/standards/delivery-artifacts-v0/schemas/review_agent_gate.schema.json` を追加し、必須 reviewer、conditional reviewer、read-only/source mutation不可/merge approval不可をschemaで固定した。
- `docs/standards/delivery-artifacts-v0/schemas/artifact_inventory.schema.json` に `review_agent_gate` typeを追加した。
- `src/lib.rs` の passing review testで、新artifactの存在、`review_agent_gate.status=passed`、`source_mutation_allowed=false`、`merge_approval_granted=false`、必須 reviewer 3種を確認した。
- README、Product Contract、Roadmap、PR Sequence、CLI User Journey、Codex CLI Primary Architecture、Operational Epicを更新した。

## Runtime State Hygiene

- destructiveなruntime state削除は行っていない。
- 関連テストは `/tmp` 配下の一時directoryを使う。
- 途中で失敗した一時テスト出力は `/tmp/fda-review-pass-test-*` に残る可能性があるが、repo正本には含めない。

## Validation Results

- `cargo fmt --all -- --check`: pass
- `git diff --check`: pass
- `python3 scripts/check_architecture_boundaries.py`: pass
- `cargo test review_`: pass, 9 tests
- `cargo test`: pass, 137 tests
- `cargo check`: pass
- `cargo run -- validate-artifacts --out artifacts/runs/v1-pivot-007-review-agent-gate-codex-subagents-20260629/validation_report.json`: pass, 64 passed / 0 failed / 41 skipped

## 残リスク

- `fda review` は今回、必須 reviewer artifactを生成するが、実際の外部subagent実行はまだfixture / artifact生成レベルである。
- `review_agent_gate.json` はPRごとの `artifacts/review_packets/pr-<PR番号>.md` 生成とは別artifactである。PR review packetへの自動投影は後続で扱う。
- `forge_reviewer` と `design_qa` の条件発火判定は今回のartifact setでは未実装で、未発火時の `not_applicable` 理由を固定するところまでに留めた。

## 次にすること

V1-PIVOT-008相当で、`review_agent_gate.json` を review packet の `REVIEW_AGENT_GATE` へ投影し、`scripts/check_review_agent_gate.py --pr-number <PR番号>` と `fda review` のartifact contractを接続する。
