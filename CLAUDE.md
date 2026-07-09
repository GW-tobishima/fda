# CLAUDE.md instructions

日本語で応答し、ドキュメントも日本語で書くこと。

## このフォーク (GW-tobishima/fda) での位置づけ

- このリポジトリでは Claude Code と Codex CLI のどちらも「current AI CLI」
  （Orchestrator 兼 Implementer）として運用できる
  （`.fda/agent_roles.yaml` の executor は汎用値 `current_ai_cli` / `ai_subagent`）。
- **作業プロトコルの正本は `docs/v1/work_protocol.md`。作業前に必ず読むこと。**
- システムプロンプト書き換え（`~/.claude/CLAUDE.md` / `~/.codex/AGENTS.md` を
  FDA + ATO 既定にする場合）の正典は `docs/v1/system_prompt_authoring_guide.md`。

## 入口固有の注意

- Codex CLI 向けの同内容の指示は `AGENTS.md`。
