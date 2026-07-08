# AGENTS.md instructions

日本語で応答し、ドキュメントも日本語で書くこと。

## このフォーク (GW-tobishima/fda) での位置づけ

- Codex CLI セッションは V1 正本どおり「current Codex CLI」= Orchestrator 兼
  Implementer として振る舞う（`.fda/agent_roles.yaml` の executor は汎用値
  `current_ai_cli` / `ai_subagent`。Claude Code セッションも同じ役割を担える。
  Claude Code 向けの同内容の指示は `CLAUDE.md`）。
- 普段使いの運用正本は `docs/v1/claude_code_primary_runbook.md`
  （Claude Code / Codex CLI 両対応の current AI CLI ランブック）。
  ジャーニーは `fda start → decide → design → implement --dry-run →
  （current AI CLI が role switch して実装・PR 作成）→ review → merge` で、
  各コマンドに `--ato-sync --ato-task <key> --ato-run-id <run>` を付けて ATO へ書き戻す。
- Codex CLI では `fda implement --dry-run / --live` の MCP probe（`codex mcp-server`）が
  そのまま使える。Windows の npm 版 codex（.cmd shim）は FDA 側で自動解決される。
- Windows での検証: `cargo test`（rustc 1.88+）/
  `python3 scripts/check_architecture_boundaries.py`（python3 が無ければ `python` / `py -3`。
  Rust 側は `FDA_PYTHON` で明示可能）/ `python3 -m unittest discover -s tests`（要 tzdata）。
- 全 run 横断の状態確認は `fda ui`（read-only Mission Control、127.0.0.1）。

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
