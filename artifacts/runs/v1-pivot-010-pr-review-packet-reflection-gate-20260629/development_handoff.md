# Development Handoff: V1-PIVOT-010 PR Review Packet Reflection Gate

## Goal

HDP-008 A方針に沿い、FDA V1では `fda review` が `review_agent_gate_packet.md` を生成しても、PR番号付きreview packetを暗黙には書き換えない。
ただし、反映漏れのままmergeへ進むことは許可しない。

## Scope

- `review_agent_gate_packet.md` がartifact run内に存在する場合だけ、`fda merge` で `artifacts/review_packets/pr-<PR番号>.md` の存在を確認する。
- PR番号は `external_pr_receipt.json` / `qa_receipt.json` から正規化された `actual_pr_url` から解決する。
- PR番号付きpacketが存在しても、最低限 `## REVIEW_AGENT_GATE` と `MERGE_APPROVAL: not_granted` が無ければblockedにする。

## Explicit Non-goals

- `fda review` から `artifacts/review_packets/pr-<PR番号>.md` を自動更新しない。
- review packetのsection merge、差分適用、conflict解決はV1.5以降の別Decisionに送る。
- merge approvalをFDAが自己承認しない。

## Validation

- `cargo test merge_blocks_unreflected_review_agent_gate_packet -- --nocapture`

## Next

PR番号が確定したら、`review_agent_gate_packet.md` を `artifacts/review_packets/pr-<PR番号>.md` へ明示反映し、`python3 scripts/check_review_agent_gate.py --pr-number <PR番号>` を通してから `fda merge` を再実行する。
