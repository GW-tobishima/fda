# Cross-repo handoff v1

Cross-repo handoff は、fda が計画した Planned PR を対象 repo の実装者へ渡し、actual PR の結果を receipt と evidence packet として回収するための contract である。

fda は対象 repo の実コードを正本として持たない。対象 repo のコード、PR、CI は GitHub repo 側の正本である。

## 最小artifact

- `target_repo_profile.json`
- `implementation_handoff.json`
- `actual_pr_handle.json`
- `external_pr_receipt.json`
- `evidence_return_packet.json`

## schema

- `schemas/cross-repo/target_repo_profile.schema.json`
- `schemas/cross-repo/implementation_handoff.schema.json`
- `schemas/cross-repo/actual_pr_handle.schema.json`
- `schemas/cross-repo/external_pr_receipt.schema.json`
- `schemas/cross-repo/evidence_return_packet.schema.json`

## oshi-note example

- `examples/cross_repo/oshi_note/target_repo_profile.json`
- `examples/cross_repo/oshi_note/implementation_handoff.json`
- `examples/cross_repo/oshi_note/actual_pr_handle.json`
- `examples/cross_repo/oshi_note/external_pr_receipt.json`
- `examples/cross_repo/oshi_note/evidence_return_packet.json`

## 境界

- fda は対象 repo の実コードを変更しない。
- handoff は対象 repo 実装者への入力であり、merge approval ではない。
- actual PR handle は receipt collector への入力であり、actual PR の URL、番号、head/base、最新 commit、観測時刻を固定する。
- receipt は対象 repo PR の実結果であり、planned PR を自動で成功扱いにしない。
- missing proof、stale evidence、trace gap は Human Decision ではなく AI Repair に戻す。
