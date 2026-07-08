# Superseded Codex MCP Live Implementer Prompt

このartifactはPR #86時点のCodex MCP live implementer試行を残すための履歴証跡である。PR #87以降のV1主経路では、current Codex CLI session が実装者であり、Codex MCP live implementerはV1.5 optional automationで扱う。

## 境界

- target repo cwd: `transient-worktree:fda-v1-017-forge-gate-adapter`
- program: `FDA-V1`
- epic: `EPIC-FDA-V1-MCP`
- planned PR: `PR-V1-006`
- workspace policy: `write`
- approval policy: `on-request`

## 必須事項

- Human Decision 未解決のscope変更をしない。
- 実装後に関連testまたはreadiness checkを実行する。
- PRを作成し、planned PR ID と actual PR URL の対応を返す。
- merge / release は行わない。

## 最終応答フォーマット

最後に以下のmarkerを必ず出力してください。

```text
FDA_ACTUAL_PR_URL: https://github.com/<owner>/<repo>/pull/<number>
FDA_TEST_STATUS: passed|failed|not_run
FDA_TESTS_RUN: <command summary>
FDA_CHANGED_FILES: <comma separated paths or NONE>
FDA_SCOPE_DRIFT: none|<summary>
```
