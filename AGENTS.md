# AGENTS.md instructions

日本語で応答し、ドキュメントも日本語で書くこと。

## このフォーク (GW-tobishima/fda) での位置づけ

- このリポジトリでは Claude Code と Codex CLI のどちらも「current AI CLI」
  （Orchestrator 兼 Implementer）として運用できる
  （`.fda/agent_roles.yaml` の executor は汎用値 `current_ai_cli` / `ai_subagent`）。
- **作業プロトコルの正本は `docs/v1/work_protocol.md`。作業前に必ず読むこと。**
- グローバルのシステムプロンプト（`~/.codex/AGENTS.md` / `~/.claude/CLAUDE.md`）を
  FDA + ATO 既定にする場合の正典は `docs/v1/system_prompt_authoring_guide.md`。

## 入口固有の注意

- Claude Code 向けの同内容の指示は `CLAUDE.md`。
- Codex CLI では `fda implement --dry-run` に加え `--live`（MCP probe: `codex mcp-server`）も利用できる。
