# V1-PIVOT-009 Development Handoff

## EPIC全体のゴール

FDA V1を Codex CLI primary の Delivery Skill Pack / Work Protocol として成立させ、`.fda/` Profile Gate、current Codex CLI implementer handoff、Review Agent Gateを必須運用にする。MCP direct implementerはV1.5 optional automationとして残す。

## 今回のPR境界

- 対象: V1-PIVOT-009 PR review packet update contract
- 含む:
  - Human Decision `dec_01KW8F0K245Q95BH2698NDDZVG` へのA回答の記録と適用
  - `review_agent_gate_packet.md` を実PRの `artifacts/review_packets/pr-<PR番号>.md` へ自動反映しないV1方針の明文化
  - PIVOT-008 / PIVOT-009 を `epic_delivery_plan.md`、roadmap、PR sequence、Codex CLI primary architectureへ追記
  - `human_decision_packet.md` に HDP-008 を追加
- 含まない:
  - `fda review --pr-number <n>` の自動更新実装
  - `fda review-packet apply` の実装
  - GitHub PR body / review packetの自動編集
  - merge approval

## 採用した判断

HDP-008はAを採用した。

> V1では `review_agent_gate_packet.md` projection生成まで。実PR packetへの反映は明示コマンドまたは人間確認後にする。

このため、V1の標準挙動では `fda review` は `review_agent_gate_packet.md` を生成するが、PR番号付きreview packetを暗黙に書き換えない。実PR packetへ反映した後に `scripts/check_review_agent_gate.py --pr-number <PR番号>` を通す。

## Validation Results

- `git diff --check`: pass
- `cargo run -- validate-artifacts --out artifacts/runs/v1-pivot-009-pr-review-packet-update-contract-20260629/validation_report.json`: pass, 64 passed / 0 failed / 41 skipped

## 残リスク

- 実PR packetへの反映は手動または将来の明示コマンドに残るため、反映忘れをmerge gateで検出する必要がある。
- `fda review-packet apply` のような半自動反映はV1.5候補であり、別Decisionが必要である。

## 次にすること

次の実装では、merge / PR ready gate側で「`review_agent_gate_packet.md` はあるがPR packetへ未反映」の状態を検出し、Human DecisionではなくAI repairまたは明示actionとして返す。
