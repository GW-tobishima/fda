# V1-PIVOT-006 Development Handoff

## EPIC全体のゴール

FDA V1を Codex CLI primary の Delivery Skill Pack / Work Protocol として成立させ、`fda implement` の主経路を current Codex CLI handoff に寄せる。MCP direct implementer は V1.5 optional automation として残す。

## 今回のPR境界

- 対象: V1-PIVOT-006 current Codex CLI implementer handoff
- 含む:
  - `current_codex_cli_handoff.json` schema追加
  - `fda implement --dry-run` の出力に `current_codex_cli_handoff.json` を追加
  - `implementation_handoff.md` の目的を current Codex CLI primary に更新
  - dry-run成功時の next action を current Codex CLI 実装へ変更
  - artifact inventory / runner explanation / docs の更新
  - 既存MCP dry-run artifactは V1.5 optional automation互換として維持
- 含まない:
  - current Codex CLIがfixture-freeで実装PRを作るE2E実行
  - Review Agent Gate実起動
  - GitHub merge実行
  - auto merge

## 実装内容

- `docs/standards/delivery-artifacts-v0/schemas/current_codex_cli_handoff.schema.json` を追加した。
- `docs/standards/delivery-artifacts-v0/schemas/artifact_inventory.schema.json` に `current_codex_cli_handoff` artifact type を追加した。
- `src/rendering/implement.rs` に `current_codex_cli_handoff(...)` を追加し、Profile Gate、Human Decision Guard、role switch、required checks、expected evidence、forbidden actionsをJSON化した。
- `src/application/implement.rs` で `current_codex_cli_handoff.json` を生成するようにした。
- `implementation_handoff.md`、runner explanation、artifact inventory、dry-run receipt next actionを current Codex CLI primary に更新した。
- `src/lib.rs` の implement dry-run testで `current_codex_cli_handoff.json` の存在と `status=ready` を確認した。
- product contract、roadmap、PR sequence、Codex CLI primary正本、CLI user journey、MCP architecture文書を更新した。

## Runtime State Hygiene

- destructiveなruntime state削除は行っていない。
- temp directoryを使う既存テストはテスト内で削除している。
- `validate-artifacts` により `artifacts/runs/v1-pivot-006-current-codex-cli-implementer-handoff-20260629/validation_report.json` を生成した。

## Validation Results

- `cargo fmt --all -- --check`: pass
- `git diff --check`: pass
- `python3 scripts/check_architecture_boundaries.py`: pass
- `cargo test`: pass, 137 tests
- `cargo check`: pass
- `cargo run -- validate-artifacts --out artifacts/runs/v1-pivot-006-current-codex-cli-implementer-handoff-20260629/validation_report.json`: pass, 63 passed / 0 failed / 40 skipped

## 残リスク

- fixture-free current Codex CLI実装PR作成はまだ未実行である。Operational V1ではPR-V1-015相当のE2E証跡が必要。
- `fda implement --live` は既存互換としてまだMCP live automationを扱う。V1主経路ではなく、V1.5 optional automationとして文書上は退避した。
- `current_codex_cli_handoff.json` はhandoff契約であり、実装完了receiptそのものではない。実装後の `implementation_receipt.json` / `external_pr_receipt.json` 回収は後続境界で扱う。

## 次にすること

V1-PIVOT-007で、Codex subagent / read-only reviewer前提のReview Agent Gateを更新する。最低限、`pr_reviewer`、`functional_qa`、`security_qa`、条件付き `forge_reviewer` / `design_qa` を review packet とCLI出力で矛盾なく扱う。
