# FDA Work Protocol（単一正本）

作成日: 2026-07-09（PR-V15-001 / F6 表層分け）
位置づけ: **このファイルが FDA 対象 repo における作業プロトコルの唯一の正本である。**
`AGENTS.md`（Codex CLI 向け）・`CLAUDE.md`（Claude Code 向け）・
`.claude/skills/fda-delivery/SKILL.md` は「入口ごとの位置づけ + 本ファイルへの参照」だけを持ち、
プロトコル本文をコピーしない（4 重記述による drift の根絶）。

## 1. 4 概念カーネル

FDA / ATO の語彙は多いが、すべて次の 4 概念のどれかの射影である。
迷ったら「これは 4 つのどれか」を考える。

| カーネル | 意味 | fda の語彙 | ato の語彙 |
|---|---|---|---|
| **作業 (Work)** | 誰が何を進めているか | run dir、stage gate、handoff、epic / planned PR | task、run、checkpoint、readiness |
| **証跡 (Evidence)** | 何が検証されたか | receipt、validation_report、review packet、risk tier | evidence edge、trace、proof、Case |
| **判断 (Decision)** | 人間の権限は何か | Human Decision、merge approval、delegation contract | typed decision（answer / apply）、HUMAN_TURN |
| **知識 (Knowledge)** | 何を再利用するか | precedent（判断の先例） | knowledge、product memory、context bootstrap |

原則: **作業は証跡を生み、証跡が判断を支え、判断は知識になる。**
逆流（知識が判断を自動で置き換える、証跡なしで作業が進む）は fail-closed で止める。

## 2. 役割（current AI CLI モデル）

- 人間が開いた AI CLI セッション（Claude Code / Codex CLI）が **current AI CLI** =
  Orchestrator 兼 Implementer（`.fda/agent_roles.yaml` の `current_ai_cli`）。
- PR Reviewer / Functional QA / Security QA は **read-only subagent**（`ai_subagent`）で分離実行。
  reviewer は source mutation / merge approval / risk approval / scope approval /
  Forge PromotionDecision の自己承認をしない。
- Orchestrator → Implementer の role switch 時は ATO checkpoint と
  `current_codex_cli_handoff.json` / `implementation_handoff.md` を残す
  （`current_codex_cli_*` は V1 契約語彙。「現在の AI CLI への handoff」と読む）。

## 3. 標準ジャーニー

ATO task/run を先に開き、各 fda コマンドに
`--ato-sync --ato-task <key> --ato-run-id <run>` を付けて書き戻す。

```text
0) ato work begin --new --title "<依頼>" --role implementer --json
1) fda start "<依頼>" | --input <md>     … 要件・NFR・リスク・Human Decision 抽出
2) fda decide <ID> --answer <答え>       … 人間が回答（V1.5: --by-contract で委任契約適用可。PR-V15-003 で実装予定・現時点は未実装）
3) fda design                            … 設計・case/task graph・planned PRs・forge projection
4) fda implement --dry-run --target-repo <path> … handoff 生成（repo は変更しない）
5) （current AI CLI が Implementer に role switch して実装・テスト・PR 作成、
     external_pr_receipt.json を残す。Codex 環境は fda implement --live も可）
6) fda review                            … 3 read-only reviewer の receipt を集約
   → review_agent_gate_packet.md を artifacts/review_packets/pr-<PR番号>.md に反映
     （自動反映されない。手動反映が必須。未反映のまま merge に進むと blocked）
   → python3 scripts/check_review_agent_gate.py --pr-number <PR番号>
7) fda continue                          … QA FAIL 時の repair loop（V1.5: --epic で次 PR 判定。PR-V15-004 で実装予定・現時点は未実装）
8) fda merge                             … merge gate。V1.5 でも auto merge しない。
     merge 前に ato case evaluate --task <key> --no-write --json で Forge gate を試算
     （verdict=promote でも merge approval ではない。fda merge 自体の Forge gate は
       ローカル forge_projection.json を評価し、hold / blocked のまま merge に進まない）
9) fda status / fda ui / fda open        … 状態確認・Mission Control・run 単体 hub
```

## 4. Review Agent Gate（PR 必須証跡）

- PR を作る前、または PR を更新した直後に、`ato agent broker --task <key> [--role <role>] --json`
  または repo-local policy（`.fda/agent_roles.yaml`）から必要 reviewer を確認する。
- PR ごとに `artifacts/review_packets/pr-<PR番号>.md` を作り `REVIEW_AGENT_GATE` を記録する
  （`review_agent_gate_packet.md` からの反映は自動ではない）。
- 必須 read-only reviewer: `pr_reviewer` / `functional_qa` / `security_qa`。
- ATO / Forge / FDA の証跡・handoff・review packet・human decision 境界に触れる場合は
  `forge_reviewer`（無ければ `qax2` 代替 + 理由記録）。UI / frontend / browser に触れる場合は
  `design_qa`。該当しない場合も `design_qa: not_applicable` と理由を残す。
- checker: `python3 scripts/check_review_agent_gate.py --pr-number <PR番号>`
  （python3 が無ければ `python` / `py -3`。Rust 側は `FDA_PYTHON` → python3 → python → py -3）。
- `REVIEW_AGENT_OK` は merge approval ではない。`REVIEW_AGENT_HOLD` / FAIL / pending /
  evidence 不足は PR ready / merge に進めず、AI repair・QA repair・typed human decision へ戻す。

## 5. 禁止事項（fail-closed）

- Human Decision（scope / privacy / legal / security High・Critical / risk / merge / release）の
  自己承認。委任契約（delegation contract）の**制定**も常に人間（AI は提案まで）。
- 未解決 Human Decision を実装で埋めること。
- reviewer / QA subagent による source mutation。
- auto merge（単発・例外を含め禁止。V1.5 でも不変。merge approval は常に人間へ）。
- raw stdout / stderr 全文を正本として保存すること（receipt は要約 + verdict）。
- `.fda/` の既存ファイル上書き。target repo が無い場合に偽 repo を作ること。

## 6. ゲート一覧（種類は減らさない。重さは risk tier で比例）

| ゲート | 実体 | 参照 |
|---|---|---|
| Profile Gate | `.fda/` 7 ファイル（不足分のみ生成・上書きなし） | `docs/v1/codex_cli_primary_architecture.md` §4 |
| Human Decision Gate | 未解決判断があると実装・merge に進めない | `.fda/gates.yaml` |
| Design Gate | 設計成果物一式が実装の前提 | 同上 |
| Review Agent Gate | §4 | `scripts/check_review_agent_gate.py` |
| Merge Gate | QA / CI / risk / Forge / 反映済み packet を検証。人間承認へ handoff | `src/application/merge.rs` |
| Architecture Gate | `src/` module 境界 | `python3 scripts/check_architecture_boundaries.py` |

## 7. Windows / 環境注意

- rustc 1.88+（`rustup update stable`）。`cargo test` は Windows green（bash fixture 依存
  テストのみ unix 限定。ubuntu CI + 実機 E2E で補完）。
- Python は `python3` / `python` / `py -3` のどれかがあればよい。PoC テストは `pip install tzdata`。
- npm 版 CLI（codex 等）の `.cmd` shim は FDA が spawn 時に自動解決する。
- 検証コマンド一式: `cargo fmt --all -- --check` / `cargo test` /
  `python3 scripts/check_architecture_boundaries.py` / `python3 -m unittest discover -s tests` /
  `cargo run -- validate-artifacts --out <path>`。

## 8. 参照

- 普段使い運用: `docs/v1/claude_code_primary_runbook.md`（Claude Code / Codex CLI 両対応）
- V1 アーキテクチャ正本: `docs/standards/fda-v1/architecture.md` /
  `docs/v1/codex_cli_primary_architecture.md`
- V1.5 計画: `docs/v1/fda_v1_next_phase_v1_5.md` / Epic: `docs/v1/fda_v1_5_intake.md`
- Mission Control UI 設計: `docs/v1/mission_control_uiux.md`
