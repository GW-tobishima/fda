# AGENTS.md instructions

日本語で応答し、ドキュメントも日本語で書くこと。

## ATO / Forge / FDA 開発の必須ゲート

このリポジトリ、またはユーザーのプロジェクトで ATO、Forge、FDA のいずれかを使って開発・PR・merge 準備を行う場合、Review Agent Gate を必ず実行する。

- ATO task / run を開始してから作業する。
- PR を作る前、または PR を更新した直後に、ATO broker または同等の repo-local policy から必要 reviewer を確認する。
- 少なくとも `pr_reviewer`、`functional_qa`、`security_qa` を read-only reviewer として実行する。
- ATO / Forge / FDA の証跡、handoff、review packet、human decision 境界に触れる場合は `forge_reviewer` を実行する。現在の実行環境で `forge_reviewer` role が broker / transaction policy に無い場合は、`qax2` または orchestrator review-gate run で代替し、その理由を ATO checkpoint と review packet に残す。
- UI / frontend / visual / browser surface に触れる場合は `design_qa` を実行する。該当しない場合も review packet に `design_qa: not_applicable` と理由を残す。
- reviewer は source mutation、merge approval、risk approval、scope approval を行わない。
- `REVIEW_AGENT_OK` は merge approval ではない。`REVIEW_AGENT_HOLD`、FAIL、pending、evidence 不足がある場合は PR ready / merge に進めず、AI repair、QA repair、または typed human decision へ戻す。
- PR ごとに `artifacts/review_packets/pr-<PR番号>.md` を作り、`REVIEW_AGENT_GATE` を記録する。
- `python3 scripts/check_review_agent_gate.py --pr-number <PR番号>` を通す。CI の pull_request でも同じ gate を実行する。

この gate は「任意の丁寧なレビュー」ではなく、ATO / Forge / FDA を使う開発の必須証跡である。
