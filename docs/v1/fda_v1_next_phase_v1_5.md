# FDA V1.5 Next Phase Plan

作成日: 2026-07-09
位置づけ: Operational V1 close（`docs/v1/fda_v1_release_note.md`）後の次フェーズ計画。
`docs/v1/fda_v1_status_report` 系の残課題整理（P1/P2）を正本化する。

## 1. V1.5 のゴール

> 要件定義書 / Epic を入力すると、AI 組織が複数 planned PR を判断ポイントまで
> 連続遂行し、人間には Human Decision だけが返る状態を、外部 repo で 2〜3 PR
> 連続の証跡付きで成立させる。

V1 の思想は変えない: Human Decision 自己承認なし / auto merge 常用なし /
SoT 分離（ATO・Forge・FDA・GitHub）維持 / current AI CLI が主経路。

## 2. ワークストリーム

### WS-1: Cross-repo Epic Execution Loop（最優先）

```text
PoC planning artifact → planned_pr_id → implementation_handoff
  → target repo actual PR → external_pr_receipt → QA / review / merge status
  → 次 planned PR 判断
```

必要成果物（新規契約）:

- `external_pr_receipt.json`（既存）の cross-repo hardening
- `evidence_return_packet.json`
- `pr_progress_state.json` / `epic_progress_state.json`
- `next_planned_pr_decision.json`

### WS-2: `fda continue --epic`

Epic state を読み、completed planned PRs / blocked / waiting human / ready を判定し、
次に進める planned PR を選ぶ。Human Decision が必要なら Slack 通知と Decision Inbox
へ戻す。V1 では Claude Code / Codex CLI がこのループを手動で担う
（`docs/v1/claude_code_primary_runbook.md` §5）。

### WS-3: Target repo onboarding 再現性

- 2 つ以上の target repo で `.fda/` onboarding を再現（不足時作成 / 上書きなし /
  missing gate / repo-specific command 検証）
- repo-local policy と FDA 共通 policy の優先順位契約

### WS-4: Human Decision / Notification / Output Hub 磨き込み

- decision document deep link / resume command 明示 / open decision の再通知・期限
- Output Hub の人間体験（artifact preview、Decision only view、AI Repair と
  Human Decision の分離強化、current phase / next action のトップ表示）
- Mission Control Web UI は「成果物・未解決判断・リスク・期限・責任・最終結果・
  失敗時影響」粒度の read-only projection として設計する（Task/Run/Agent 粒度にしない）

### WS-5: MCP optional automation（V1.5 optional）

- Codex MCP live invocation / Claude MCP QA agent / parallel review / background
  implementation。`CodexProcessPort` の多実装化（Claude 対応）を含む。
  Codex CLI / Claude Code primary の体験が自然である限り主経路には戻さない。

### WS-6: Auto merge policy（V1.5 optional / 慎重）

低リスク変更（docs only / tests only / internal tooling / generated proof pack refresh）
限定で検討。必要条件: Forge PromotionDecision=promote、Functional/Security QA pass、
CI green、open Human Decision なし、High/Critical findings なし、rollback plan あり。

## 3. 推奨 PR 順

```text
PR-NEXT-001: Operational V1 final cleanup（本フォークで実施済み: e2e next_actions 更新、release note / residual risks / next phase docs）
PR-NEXT-002: Cross-repo external PR receipt hardening
PR-NEXT-003: Epic progress state / continue --epic design
PR-NEXT-004: 外部 repo での multi-PR execution pilot
PR-NEXT-005: Target repo .fda onboarding hardening
PR-NEXT-006: Output Hub / Decision Inbox UX polish（Mission Control read-only projection を含む）
```

## 4. Done 判定（V1.5）

1. 外部 repo 1 つ以上で planned PR 2〜3 本を連続遂行した証跡（receipt 連鎖）がある。
2. `continue --epic` 相当が次 PR / blocked / waiting human を判定できる。
3. onboarding が 2 repo 以上で再現し、policy 衝突時の優先順位が契約化されている。
4. Human Decision の通知に deep link と resume command が含まれる。
5. 上記すべてで Human Decision 自己承認なし / auto merge 常用なしが維持されている。
