# CLAUDE.md instructions

日本語で応答し、ドキュメントも日本語で書くこと。

このリポジトリは FDA (forge-delivery-agent) の GW-tobishima フォークであり、
**Claude Code を「current AI CLI」（人間が開いた現在の AI CLI セッション）として運用する**。
V1 正本（`docs/v1/codex_cli_primary_architecture.md`）の "current Codex CLI" は、
このフォークでは Claude Code セッションが等価に担う。役割境界・gate・成果物契約は一切変えない。

正本ドキュメント:

- 普段使い運用: `docs/v1/claude_code_primary_runbook.md`（このフォークの運用正本）
- V1 アーキテクチャ: `docs/standards/fda-v1/architecture.md` / `docs/v1/codex_cli_primary_architecture.md`
- 作業プロトコル skill: `.claude/skills/fda-delivery/SKILL.md`

## Claude Code セッションの役割

- Claude Code は Orchestrator 兼 Implementer（`.fda/agent_roles.yaml` の
  `current_claude_code`）。role switch 時は ATO checkpoint と
  `current_codex_cli_handoff.json` / `implementation_handoff.md` を残す。
  ※ `current_codex_cli_*` という成果物名・schema_version は V1 契約語彙であり、
  「現在の AI CLI への handoff」として読む。改名しない。
- PR Reviewer / Functional QA / Security QA は read-only の subagent
  （`claude_subagent`）として分離実行する。reviewer に source mutation をさせない。
- Human Decision（scope / privacy / legal / security High・Critical / merge / release）を
  自己承認しない。未解決なら実装・merge に進まず `fda decide` へ戻す。
- V1 では auto merge しない。`REVIEW_AGENT_OK` は merge approval ではない。

## ATO / Forge / FDA 開発の必須ゲート（AGENTS.md と同一契約）

- ATO task / run を開始してから作業する（`ato work begin`）。FDA コマンドは
  `--ato-sync --ato-task <key> --ato-run-id <run>` で ATO へ書き戻せる。
- PR ごとに `artifacts/review_packets/pr-<PR番号>.md` を作り、`REVIEW_AGENT_GATE` を記録する。
- 少なくとも `pr_reviewer` / `functional_qa` / `security_qa` を read-only で実行する。
  ATO / Forge / FDA の証跡・handoff・human decision 境界に触れる場合は `forge_reviewer`
  （無ければ `qax2` 代替 + 理由記録）、UI に触れる場合は `design_qa`（該当しない場合も
  `not_applicable` と理由）。
- gate checker: `python3 scripts/check_review_agent_gate.py --pr-number <PR番号>`
  （Windows で `python3` が無い場合は `python` / `py -3`。Rust 側は `FDA_PYTHON` →
  `python3` → `python` → `py -3` の順で自動解決する）。
- architecture gate: `python3 scripts/check_architecture_boundaries.py`。
  `src/` の module 境界（main.rs ≤100行 / lib.rs 非テスト ≤520行 / application は
  allowlist 済み infra import のみ）を壊さない。

## Windows での検証コマンド

```powershell
cargo fmt --all -- --check
cargo test                 # rustc 1.88+ 必須（Cargo.lock の time crate 制約）
python3 scripts/check_architecture_boundaries.py
python3 -m unittest discover -s tests   # 要 pip install tzdata
cargo run -- validate-artifacts --out artifacts/runs/<run_id>/validation_report.json
```
