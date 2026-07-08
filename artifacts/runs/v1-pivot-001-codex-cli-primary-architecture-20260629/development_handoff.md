# Development Handoff

## Task

- Title: V1-PIVOT-001 Codex CLI primary architecture
- Source: `artifacts/runs/fda-v1-codex-cli-primary-epic-20260629/epic_delivery_plan.md`

## Goal

FDA V1 の主経路を Codex CLI primary として正本化し、`.fda/` 7ファイルprofileが無いrepoでは作業前に作成する方針を、このrepo自身に適用する。

## Delivery Boundary

- Target PBI: `FDA-V1-CLI-PRIMARY-REBASIS`
- Covered SBI: `V1-PIVOT-001` と、HDP-003適用のためのrepo-local `.fda/` bootstrap
- PR Boundary: docs / profile / handoff artifact まで。Rust behavior変更は含めない。
- Explicit Non-goals for this PR:
  - CLI runtimeの挙動変更
  - MCP adapter実装の削除
  - `.fda/agent_roles.yaml` など未スキーマ4ファイルのschema実装
  - GitHub PR作成、merge、release

## Implementation Plan

1. `.fda/` 7ファイルprofileをrepo-localに追加する。
2. `docs/v1/codex_cli_primary_architecture.md` を追加し、既存V1 docsをCodex CLI primary方針へ寄せる。
3. 標準gate、テスト、YAML構文確認を実行し、handoffを残す。

## Runtime State Hygiene

- State inspected: ATO DB / generated artifacts / `.fda/` profile
- Existing state found: ATO canonical DBは `/root/code/forge-delivery-agent/.ato/ato.db`。既存 `.fda/` は存在しなかった。
- Action taken: `.fda/` 7ファイルを新規作成。既存 `.gitignore`、`artifacts/janitor/`、`artifacts/runs/mcp-invocation-audit/` は未変更。
- Human approval needed: no。HDP-001..007は `dec_01KW8A04G58XFKREE60PJBESFM` で回答済み。
- Effect on validation: docs/profile中心のため、YAML構文、architecture gate、Rust test/checkで確認した。

## Changed Files

- `.fda/repo.yaml`
- `.fda/delivery_policy.yaml`
- `.fda/skills.lock`
- `.fda/agent_roles.yaml`
- `.fda/gates.yaml`
- `.fda/artifact_map.yaml`
- `.fda/notification.yaml`
- `docs/v1/codex_cli_primary_architecture.md`
- `docs/v1/fda_v1_product_contract.md`
- `docs/v1/fda_v1_roadmap.md`
- `docs/v1/fda_v1_pr_sequence.md`
- `docs/v1/fda_v1_operational_epic.md`
- `docs/v1/mcp_agent_architecture.md`
- `docs/standards/fda-v1/architecture.md`
- `docs/standards/fda-v1/repository_profile.md`
- `README.md`
- `artifacts/runs/v1-pivot-001-codex-cli-primary-architecture-20260629/development_handoff.md`

## Acceptance Criteria Evidence

- [x] AC: V1主経路がCodex CLI primaryであることを示すarchitecture doc案が作れる。
  - Evidence: `docs/v1/codex_cli_primary_architecture.md`
- [x] AC: MCP direct implementerの扱いがV1 / V1.5で明確に分かれる。
  - Evidence: `docs/v1/mcp_agent_architecture.md` 冒頭と `docs/v1/codex_cli_primary_architecture.md`
- [x] AC: 既存 `docs/v1/mcp_agent_architecture.md` の主張と新方針の衝突を解消するPR境界がある。
  - Evidence: `docs/v1/fda_v1_roadmap.md` と `docs/v1/fda_v1_pr_sequence.md`
- [x] AC: `.fda/` repo profileの7ファイル必須化と未作成repoでの作成強制が反映されている。
  - Evidence: `.fda/*`、`docs/standards/fda-v1/repository_profile.md`
- [x] AC: Review Agent Gateの必須reviewerとnot-applicable規則が維持されている。
  - Evidence: `.fda/gates.yaml`、`docs/v1/codex_cli_primary_architecture.md`

## Validation Results

- Validation Level: unit only
- Command or check: `git diff --check`
- Result: pass
- Notes: whitespace errorなし。

- Validation Level: unit only
- Command or check: `python3 scripts/check_architecture_boundaries.py`
- Result: pass
- Notes: `architecture boundary check passed`

- Validation Level: unit only
- Command or check: `cargo test`
- Result: pass
- Notes: 134 tests passed。

- Validation Level: unit only
- Command or check: `cargo check`
- Result: pass
- Notes: Rust compile check passed。

- Validation Level: unit only
- Command or check: `python3 - <<'PY' ... yaml.safe_load(.fda/*) ... PY`
- Result: pass
- Notes: `.fda/` 7ファイルのYAML構文を確認。

## Open Risks

- `.fda/agent_roles.yaml`、`.fda/gates.yaml`、`.fda/artifact_map.yaml`、`.fda/notification.yaml` は今回schema未実装。`V1-PIVOT-003` でschema / examples / validationに反映する。
- 既存CLI testsにはまだ `implement --dry-run` / `implement --live` の互換名が残る。V1主経路のCLI挙動変更は `V1-PIVOT-005` 以降で扱う。
- MCP関連のexample artifactはV1.5 optional automationとして残した。削除や大規模移動は今回のPR境界外。

## Human-only Decisions

なし。理由: HDP-001..007 はユーザー回答済みで、今回の変更はその範囲内に収まる。

## Recommended Next Pack

- `dev.self-review`
