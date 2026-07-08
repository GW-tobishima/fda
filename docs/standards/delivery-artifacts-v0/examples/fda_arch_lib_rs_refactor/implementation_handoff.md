---
artifact: implementation_handoff
program_id: PROGRAM-FDA-ARCH-001
epic_id: EPIC-FDA-ARCH-FU-001
planned_pr_id: PR-FDA-ARCH-FU-001
target_repo: forge-delivery-agent
status: ready_for_implementation
---

# FDA architecture follow-up / 実装 handoff

## 目的

この handoff は `PR-FDA-ARCH-FU-001` の実装入力です。

GitHub Issue #68 を FDA artifact として正式 Epic 化したため、後続 PR は `planned_prs.json` の順序と scope に従います。

## Target Repo

- target repo: `forge-delivery-agent`
- source artifact root: `docs/standards/delivery-artifacts-v0/examples/fda_arch_lib_rs_refactor/`
- target planned PR: `PR-FDA-ARCH-FU-001`
- parent GitHub issue: https://github.com/msamunetogetoge/forge-delivery-agent/issues/68

## 参照順

1. `planned_prs.json`
2. `epic_delivery_plan.json`
3. `case_graph.json`
4. `task_graph.json`
5. `forge_projection.json`
6. `autonomy_contract.json`
7. `requirements_definition.md`

## PR-FDA-ARCH-FU-001 Scope

### Scope In

- `StartResult` / `DecideResult` / `DesignResult` / `PlanResult` / `ValidationReport` の適切な module への移動。
- `display_path` / `resolve_path` など pure helper の support 化。
- `src/application/start.rs`、`decide.rs`、`design.rs`、`plan.rs`、`validate.rs` の crate root import 削減。
- `src/lib.rs` の facade 化に向けた最初の減量。

### Scope Out

- `implement` / `review` / `continue` / `merge` / `open` / `notify` の大移動。
- MCP / Codex process invocation の port 化。
- CLI output、artifact schema、runtime behavior の変更。

## Acceptance Criteria

- 抽出済み application module が crate root helper に強く依存しない。
- `src/lib.rs` の command-independent helper が減っている。
- CLI behavior と generated artifact compatibility が変わらない。
- `python3 scripts/check_architecture_boundaries.py` が pass する。
- `cargo test` が pass する。
- `cargo clippy -- -D warnings` が pass する。

## Review Notes

- 挙動変更なしの function/type move と visibility 調整に限定してください。
- `domain/` に IO を入れないでください。
- `rendering/` に filesystem 書き込みを入れないでください。
- `application/` に direct な `std::fs`、`std::process`、`jsonschema`、`serde_yaml`、`SystemTime`、stdout/stderr を入れないでください。

## Evidence Expected

- `python3 scripts/check_architecture_boundaries.py`
- `cargo test`
- `cargo clippy -- -D warnings`
- PR body の `PR-FDA-ARCH-FU-001` / `CLM-FDA-ARCH-FU-*` mapping
- 変更ファイルと責務境界の short self-review
