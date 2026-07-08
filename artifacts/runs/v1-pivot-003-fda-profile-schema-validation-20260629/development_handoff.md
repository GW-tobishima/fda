# Development Handoff

## Task

- Title: V1-PIVOT-003 FDA profile schema validation
- Source: `artifacts/runs/fda-v1-codex-cli-primary-epic-20260629/epic_delivery_plan.md`

## Goal

`.fda/` 7ファイルprofileをV1必須contractとしてschema化し、`fda validate-artifacts` で欠落やschema不一致をfailとして検出できるようにする。

## Delivery Boundary

- Target PBI: `FDA-V1-CLI-PRIMARY-REBASIS`
- Covered SBI: `V1-PIVOT-003`
- PR Boundary: repository profile schema、validate integration、profile validation tests、docs更新。
- Explicit Non-goals for this PR:
  - `fda start` / `implement` のProfile Gate runtime enforcement
  - MCP実装の削除
  - auto merge policy変更
  - Mission Control Web UI

## Implementation Plan

1. `.fda/agent_roles.yaml`、`.fda/gates.yaml`、`.fda/artifact_map.yaml`、`.fda/notification.yaml` のschemaを追加する。
2. `validate-artifacts` に `.fda/` 7ファイルschema validationを追加する。
3. pass / missing profile のテストとvalidation reportを残す。

## Runtime State Hygiene

- State inspected: ATO DB / generated validation report / `.fda/` profile
- Existing state found: `.fda/` 7ファイルはV1-PIVOT-001で作成済み。
- Action taken: `artifacts/runs/v1-pivot-003-fda-profile-schema-validation-20260629/validation_report.json` を生成。
- Human approval needed: no。HDP-003で7ファイル必須化と未作成repoでの作成強制は承認済み。
- Effect on validation: `validate-artifacts` のpass数にrepository profile validation 7件が含まれる。

## Changed Files

- `docs/standards/fda-v1/schemas/repository-profile/agent_roles_yaml.schema.json`
- `docs/standards/fda-v1/schemas/repository-profile/gates_yaml.schema.json`
- `docs/standards/fda-v1/schemas/repository-profile/artifact_map_yaml.schema.json`
- `docs/standards/fda-v1/schemas/repository-profile/notification_yaml.schema.json`
- `src/application/ports.rs`
- `src/infra/yaml.rs`
- `src/application/validate.rs`
- `docs/standards/fda-v1/architecture.md`
- `docs/standards/fda-v1/repository_profile.md`
- `README.md`
- `artifacts/runs/v1-pivot-003-fda-profile-schema-validation-20260629/validation_report.json`
- `artifacts/runs/v1-pivot-003-fda-profile-schema-validation-20260629/development_handoff.md`

## Acceptance Criteria Evidence

- [x] AC: `.fda/` 7ファイルprofileがschema validation対象になる。
  - Evidence: `src/application/validate.rs`
- [x] AC: 追加4ファイルのschemaが存在する。
  - Evidence: `docs/standards/fda-v1/schemas/repository-profile/*_yaml.schema.json`
- [x] AC: `.fda/` 欠落はfailになる。
  - Evidence: `repository_profile_validation_fails_when_profile_is_missing`
- [x] AC: repo-local `.fda/` はpassする。
  - Evidence: `repository_profile_validation_passes_for_repo_profile`
- [x] AC: `fda validate-artifacts` の実行結果にrepository profile validationが含まれる。
  - Evidence: `artifacts/runs/v1-pivot-003-fda-profile-schema-validation-20260629/validation_report.json`

## Validation Results

- Validation Level: unit only
- Command or check: `git diff --check`
- Result: pass
- Notes: whitespace errorなし。

- Validation Level: unit only
- Command or check: `python3 scripts/check_architecture_boundaries.py`
- Result: pass
- Notes: applicationから具体YAML実装をinfraへ戻し、architecture boundaryに適合。

- Validation Level: unit only
- Command or check: `cargo test`
- Result: pass
- Notes: 136 tests passed。

- Validation Level: unit only
- Command or check: `cargo check`
- Result: pass
- Notes: Rust compile check passed。

- Validation Level: unit only
- Command or check: `cargo run -- validate-artifacts --out artifacts/runs/v1-pivot-003-fda-profile-schema-validation-20260629/validation_report.json`
- Result: pass
- Notes: `validation pass: 62 passed, 0 failed, 39 skipped`

## Open Risks

- `validate-artifacts` でprofile validationは実行されるが、`fda start` / `fda implement` の入口でprofile作成を自動強制するruntime behaviorは未実装。これは `V1-PIVOT-005` 以降で扱う。
- 既存のMCP dry-run / live command名は互換として残っている。Codex CLI primaryへのCLI contract更新は `V1-PIVOT-005` / `V1-PIVOT-006` で扱う。

## Human-only Decisions

なし。理由: HDP-003で `.fda/` 7ファイル必須化と、無いrepoでの作業前作成強制は承認済み。

## Recommended Next Pack

- `dev.self-review`
