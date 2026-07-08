# V1-PIVOT-008 Development Handoff

## EPIC全体のゴール

FDA V1を Codex CLI primary の Delivery Skill Pack / Work Protocol として成立させ、`.fda/` Profile Gate、current Codex CLI implementer handoff、Review Agent Gateを必須運用にする。MCP direct implementerはV1.5 optional automationとして残す。

## 今回のPR境界

- 対象: V1-PIVOT-008 Review Agent Gate packet projection
- 含む:
  - `review_agent_gate.json` から既存 Review Agent Gate checker が読めるMarkdown projectionを生成する
  - `fda review` の出力に `review_agent_gate_packet.md` を追加する
  - `review_agent_gate_packet.md` を `review_packet` artifactとして inventory / runner evidence に登録する
  - passing review testで `scripts/check_review_agent_gate.py --packet-path <review_agent_gate_packet.md>` が通ることを確認する
  - README / V1 docsへpacket projectionの運用を追記する
- 含まない:
  - `artifacts/review_packets/pr-<PR番号>.md` への自動書き込み
  - GitHub PR body / review packet更新
  - 実際のCodex subagent起動
  - merge approval

## 実装内容

- `src/rendering/review.rs` に `review_agent_gate_packet_markdown(...)` を追加した。
- packet projectionは `MERGE_APPROVAL: not_granted`、必須 reviewer 3種、orchestrator equivalent、conditional `forge_reviewer` / `design_qa` の `not_applicable` rationaleを含む。
- `src/application/review.rs` で `review_agent_gate_packet.md` を出力するようにした。
- `review_artifact_inventory(...)` に `review_agent_gate_packet.md` を `review_packet` として追加した。
- `src/lib.rs` の passing review testで生成packetを `scripts/check_review_agent_gate.py --packet-path` に渡してpassを確認した。
- README、Product Contract、Roadmap、PR Sequence、CLI User Journey、Codex CLI Primary Architecture、Operational Epicを更新した。

## Runtime State Hygiene

- destructiveなruntime state削除は行っていない。
- テストは `/tmp` 配下の一時directoryを使い、成功時は削除する。

## Validation Results

- `cargo fmt --all -- --check`: pass
- `git diff --check`: pass
- `python3 scripts/check_architecture_boundaries.py`: pass
- `cargo test review_writes_separate_functional_and_security_receipts`: pass
- `python3 -m pytest tests/test_review_agent_gate.py -q`: pass, 15 tests
- `cargo test`: pass, 137 tests
- `cargo check`: pass
- `cargo run -- validate-artifacts --out artifacts/runs/v1-pivot-008-review-agent-gate-packet-projection-20260629/validation_report.json`: pass, 64 passed / 0 failed / 41 skipped

## 残リスク

- `review_agent_gate_packet.md` はPR packetへ貼れるprojectionであり、まだ `artifacts/review_packets/pr-<PR番号>.md` へ自動反映しない。
- PR番号がある実PR運用では、`review_agent_gate_packet.md` の内容を review packet の `## REVIEW_AGENT_GATE` sectionへ反映し、`python3 scripts/check_review_agent_gate.py --pr-number <PR番号>` を別途通す必要がある。
- `orchestrator` row は既存checkerの Forge/QAx2 equivalent として使っている。実PRで ATO / Forge / FDA証跡境界に触れる場合は、repo policyに従い `forge_reviewer` または `qax2` の実review evidenceを追加する。

## 次にすること

V1-PIVOT-009相当で、PR番号を持つ実review packetへの自動投影、または `fda review --pr-number <n>` 形式のpacket update contractを定義する。PR更新やmerge approvalは引き続き人間判断境界として扱う。
