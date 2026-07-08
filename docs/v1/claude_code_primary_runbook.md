# FDA Current AI CLI Runbook（普段使い運用正本 / Claude Code・Codex CLI 両対応）

作成日: 2026-07-09（2026-07-09 Codex CLI 両対応に改訂）
対象: GW-tobishima/fda フォーク（Windows 11 + Claude Code / Codex CLI + ローカル ATO CLI 環境）

## 1. 位置づけ

FDA V1 の主経路は「人間が開いた現在の AI CLI の中で FDA を Delivery Skill Pack /
Work Protocol として動かす」ことである（`docs/v1/codex_cli_primary_architecture.md`）。
上流の正本は Codex CLI をその実行主体としているが、この設計の本質は
**特定ベンダーの CLI ではなく「current AI CLI」という役割**にある。

このフォークでは **Claude Code と Codex CLI のどちらを開いていても current AI CLI**
として同じジャーニーを回す。AI CLI への指示ファイルは Claude Code = `CLAUDE.md`、
Codex CLI = `AGENTS.md`（内容は同契約）。

```text
Human
  -> current AI CLI（Claude Code または Codex CLI）
  -> FDA Skill Pack / Work Protocol（fda CLI + CLAUDE.md / AGENTS.md + skills）
  -> current repo / target repo
```

変えないもの（思想の維持）:

- Stage Gate（Profile / Human Decision / Design / Review Agent / Merge）
- Human Decision の自己承認禁止、V1 auto merge なし
- receipt / handoff / review packet の成果物契約と `fda.*.v0` schema 語彙
  （`current_codex_cli_handoff.json` 等の名称は V1 契約語彙のまま「現在の AI CLI への
  handoff」として読む）
- SoT 分離: ATO=状態/証跡、Forge=Claim/Proof/Gate、FDA=runtime/skills、GitHub=実コード

## 2. Executor 対応表

`.fda/agent_roles.yaml` の executor は次の等価クラスで読む
（schema: `docs/standards/fda-v1/schemas/repository-profile/agent_roles_yaml.schema.json`）。
**このフォーク（この repo 自身の profile）は汎用値 `current_ai_cli` / `ai_subagent` を使う**。
Codex 専用 repo / Claude 専用 repo ではベンダー明示値を選んでもよい。

| 等価クラス | 汎用値（この repo） | Codex 明示値 | Claude 明示値 | 意味 |
|---|---|---|---|---|
| current CLI | `current_ai_cli` | `current_codex_cli` | `current_claude_code` | 人間が開いた現在の AI CLI セッション。Orchestrator / Implementer / Merge Manager |
| subagent | `ai_subagent` | `codex_subagent` | `claude_subagent` | current CLI が起動する read-only レビュー/QA サブエージェント |
| 外部 | `external_adapter` | 同左 | 同左 | MCP 等の外部実行層（V1.5 optional automation） |

## 3. セットアップ（Windows）

```powershell
# 1. Rust toolchain（Cargo.lock の time crate が rustc 1.88+ を要求）
rustup update stable

# 2. fda のビルドとインストール（%USERPROFILE%\.cargo\bin\fda.exe）
cargo install --path .

# 3. Python（gate script 用）。python3 / python / py -3 のどれかが見えればよい。
#    zoneinfo を使う PoC テスト用に tzdata を入れる。
python3 -m pip install tzdata

# 4. 環境変数（必要なもののみ）
#    Slack 通知（Human Decision 通知の P0 経路）
$env:FDA_SLACK_WEBHOOK_URL = "https://hooks.slack.com/services/..."
#    Python launcher を固定したい場合
$env:FDA_PYTHON = "py -3"
#    ATO 接続の既定（fda の --ato-backend / --ato-db でも指定可能）
$env:FDA_ATO_BACKEND = "local"
$env:FDA_ATO_DB_PATH = "C:\Tools\ATO\data\ato.db"
```

Codex CLI を current AI CLI / `implement --live` に使う場合は、`codex` が PATH に
あること（npm グローバル install の `.cmd` shim でよい。FDA が spawn 時に自動解決する）。

動作確認:

```powershell
fda status --artifacts artifacts/runs/<run_id>
cargo test        # Windows green
python3 scripts/check_architecture_boundaries.py
codex --version   # Codex CLI 併用時
```

## 4. 普段使いジャーニー（1 依頼 = 1 run ディレクトリ）

ATO task/run を先に開始し、`--ato-sync` で FDA の各 stage を ATO checkpoint /
typed decision に書き戻す。

```powershell
# 0) ATO task/run を開始（Claude Code が実行）
ato work begin --new --title "<依頼名>" --role implementer --json
# → task_key / run_id を控える

# 1) Intake: 要件定義・NFR・リスク・Human Decision 抽出
fda start "やりたいこと" --ato-sync --ato-task <task_key> --ato-run-id <run_id>
# 出力: artifacts/runs/fda-start-<ts>/requirements_definition.md ほか

# 2) Human Decision: 未解決判断に回答（人間）
fda decide HD-FDA-001 --answer yes --artifacts <run_dir> --ato-sync ...

# 3) Design Gate: 設計・Case/Task Graph・Planned PRs・Forge Projection
fda design --artifacts <run_dir> --ato-sync ...

# 4) Implementation handoff（target repo は変更されない）
fda implement --dry-run --target-repo <path> --artifacts <run_dir>

# 5) current AI CLI（Claude Code / Codex CLI）が Implementer に role switch して実装
#    - current_codex_cli_handoff.json / implementation_handoff.md を読む
#    - ATO checkpoint（role switch）を残す
#    - approved scope 内で実装・テスト・PR 作成
#    - external_pr_receipt.json を run ディレクトリに残す
#    ※ Codex CLI 環境では `fda implement --live`（codex mcp-server 経由の自動実装、
#      V1.5 optional automation）も選択可。Windows の npm 版 codex（.cmd shim）は
#      FDA が自動解決する

# 6) Review Agent Gate: read-only subagent 3役（+条件役）でレビュー
fda review --artifacts <run_dir> --target-repo <path>
#    review_agent_gate_packet.md を artifacts/review_packets/pr-<PR番号>.md に反映し
python3 scripts/check_review_agent_gate.py --pr-number <PR番号>

# 7) Repair loop（QA FAIL 時）
fda continue --artifacts <run_dir>    # repair_prompt.md を生成 → Claude Code が修正

# 8) Merge Gate（V1 は auto merge しない。human approval に戻す）
fda merge --artifacts <run_dir> --target-repo <path>
#    人間が承認したら（gh pr merge 実行を含めたい場合）
fda merge --execute --merge-method squash ...

# 9) 状態確認 / 成果物閲覧
fda status --artifacts <run_dir>
fda open --artifacts <run_dir>        # output_hub.html / decision_inbox.html（run 単体）

# 10) Mission Control（全 run 横断の read-only ダッシュボード）
fda ui --open                          # http://127.0.0.1:4870/ を開く
fda ui --json                          # スナップショット JSON を一度出力（機械可読）
```

`fda ui` は判断待ち（Decision Inbox）・AI Repair・run 状態を 1 画面に集約する
read-only projection で、UI から状態変更は一切できない
（設計: `docs/v1/mission_control_uiux.md`）。

## 5. Epic 継続ループ（current AI CLI がオーケストレーター）

`fda continue` は単発の Repair Loop Gate であり、Epic 全体の「次 PR 選択」は
V1 ではランタイム化されていない（V1.5 スコープ。`docs/v1/fda_v1_next_phase_v1_5.md`）。
V1 思想では current AI CLI がオーケストレーターなので、Claude Code / Codex CLI が
次の手順で上位ループを回す。

1. `planned_prs.json` と各 PR の `external_pr_receipt.json` / merge receipt を読む。
2. 未解決 Human Decision があれば止まり、`fda decide` の resume command を人間に提示する。
3. 依存関係を満たした次の planned PR を 1 つ選び、`fda implement --dry-run` から
   ジャーニー 4〜8 を繰り返す。
4. 各 PR 完了時に ATO checkpoint を残し、`ato case evaluate --task <key> --no-write --json`
   で Forge gate を試算してから merge handoff する。

## 6. トラブルシュート

| 症状 | 原因と対処 |
|---|---|
| `cargo test` がコンパイルエラー（time crate） | rustc < 1.88。`rustup update stable` |
| `ato_state_receipt.json` が `adapter_unavailable` | `ato` が PATH に無い。`--ato-cli <path>` か PATH 追加。fail-closed 動作なので成果物生成自体は継続する |
| gate script が `python3 not found` | `FDA_PYTHON` を設定するか `python` / `py -3` を導入（Rust 側は自動フォールバックする） |
| PoC テスト（aicx）で zoneinfo エラー | `python3 -m pip install tzdata` |
| Slack 通知が failed | `FDA_SLACK_WEBHOOK_URL` 未設定。fail-closed（成功扱いにしない）仕様 |
| `fda implement --live` が codex を要求 | `--live` は Codex MCP 依存の V1.5 optional automation。Codex CLI 導入済みなら利用可（Windows の npm 版 .cmd shim は自動解決）。Claude Code のみの環境では `--dry-run` + role switch 実装が主経路 |

## 7. 変更履歴

- 2026-07-09: 初版（GW-tobishima/fda フォーク。Windows 対応修正・Claude executor 追加と同時に作成）
