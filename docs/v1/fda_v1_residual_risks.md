# FDA V1 Residual Risks

作成日: 2026-07-09
対象: Operational V1 close 時点の既知残リスク。V1.5 planning の入力にする。

| ID | リスク | 影響 | 現状の緩和 | 恒久対応（フェーズ） |
|---|---|---|---|---|
| RR-001 | 複数 PR の Epic 連続遂行が代表 run 1 本しかなく、外部 repo で 2〜3 PR 連続の証跡が無い | Epic 単位の自律遂行で未知の gap が出る | current AI CLI がオーケストレーターとして手動ループ（runbook §5） | V1.5: Cross-repo Epic Execution Loop / `continue --epic` |
| RR-002 | `fda continue` は repair gate 単発で、次 PR 選択・依存解決を行わない | 上位ループが AI セッションの規律に依存する | `.claude/skills/fda-delivery` / AGENTS.md で手順を固定 | V1.5: `epic_progress_state.json` / `next_planned_pr_decision.json` |
| RR-003 | `.fda/` profile の実 repo 適用が forge-delivery-agent 自身以外で未再現 | target repo onboarding の再現性が未証明 | Profile Gate は不足分のみ生成・上書きなしで安全側 | V1.5: 2 つ以上の target repo での onboarding 証跡 |
| RR-004 | repo-local policy と FDA 共通 policy の衝突優先順位が未定義 | 外部 repo の既存 CI/branch rule と衝突し得る | 人間判断（spec_decision）へ戻す運用 | V1.5: 優先順位契約の明文化 |
| RR-005 | Slack 通知は P0 のみで、decision deep link・再通知・期限が無い | 判断待ちの見落とし | `fda status` / Decision Inbox で確認 | V1.5: 通知磨き込み |
| RR-006 | Forge gate はローカル `forge_projection.json` 評価であり、ATO の `ato case evaluate`（正本 gate）とは別物 | 二重評価の乖離があり得る | merge 前に `ato case evaluate --no-write` を併走（runbook / skill に明記） | V1.5: forge_cli adapter の実装検討 |
| RR-007 | `plan --mode model` が未実装（fixture のみ） | plan 経路の実運用は fixture 前提 | `start`/`design` 経路を使う | V1.5 で要否判断 |
| RR-008 | `implement --live` は Codex MCP 依存で、Claude Code 運用では自動実装経路が無い | live 自動化は Codex 環境限定 | `--dry-run` + role switch 実装が主経路（V1 思想通り） | V1.5: `CodexProcessPort` の Claude 実装追加 |
| RR-009 | Windows では bash fixture 依存の ato_state ユニットテスト 15 件が skip される | Windows 単体では ATO adapter の回帰検出が弱い | ubuntu CI がフルスイートを実行。実機 ato.exe での E2E 証跡を別途取得 | 必要なら fixture の cmd 移植 |
| RR-010 | email 通知（SMTP）は deprecated/docs-only 互換で openssl 依存 | 誤って主経路と読まれる恐れ | Slack P0 を正とし、docs 上 deprecated 明記 | V2 で削除判断 |
