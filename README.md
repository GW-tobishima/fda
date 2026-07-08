# forge-delivery-agent

Forge / ATO / AI Delivery Organization の実験用リポジトリです。
v1 では `forge-delivery-agent` の略称として `fda` を使います。

現在の正本は `docs/standards/delivery-artifacts-v0/` の標準成果物パック v0 です。
v1 の設計方針は `docs/standards/fda-v1/` に集約します。

## 方針

- UI 実装は後回しにする。
- UI/UX 設計は先行して進め、Human Decision と AI Repair を混ぜない Mission Control として設計する。
- コーディングが必要になった場合は Rust を既定にする。
- ATO は状態、要約、Human Turn、証跡の SoT として扱う。
- Forge は Claim、Proof、Gate、PromotionDecision の SoT として扱う。
- fda は実行 runtime、skills、repo profile schema / adapter contract、handoff、artifact contract の SoT として扱う。
- GitHub repo は実コード、PR、CI、repo-local config の SoT として扱う。

## fda v1

fda v1 は、任意の対象リポジトリに対して、自然言語または要件定義書から Epic Planning、Task/PR 分解、Human Decision 抽出、実装 handoff、外部 PR 証跡回収、ATO / Forge への状態反映までを、同じ skills と設定で再現できる AI Delivery Runtime です。

2026-06-29 以降の V1 主経路は Codex CLI primary です。人間が開いた現在の Codex CLI を実装者・オーケストレーターとして使い、fda はその中で動く Delivery Skill Pack / Work Protocol として扱います。Codex / Claude MCP direct implementer は V1.5 以降の optional automation layer です。

FDA 対象 repo で作業する場合は `.fda/` 7ファイル profile が必須です。対象 repo に `.fda/` が無い場合、設計、実装、レビュー、PR作成へ進む前に `.fda/` folder と必須 profile を作成します。既存 `.fda/*` は上書きせず、不足分だけを作成します。target repo path 自体が存在しない場合は偽repoを作らず、target repo missing gate に戻します。

v1 では、全部を 1 つの DB に押し込まず、Source of Truth を分けた中央 Control Plane として扱います。

- ATO: 状態、Summary、Human Turn、Knowledge、Context Snapshot
- Forge: Claim、Proof、Precedent、PromotionDecision、ReleaseDecision
- fda: 実行 runtime、skills、repo profile schema / adapter contract、handoff、artifact contract
- GitHub repos: 実コード、PR、CI、repo-local config、repo-local `.fda/` values

詳細は `docs/standards/fda-v1/architecture.md` を参照します。
Codex CLI primary の正本は `docs/v1/codex_cli_primary_architecture.md` を参照します。
source tree の最終形と各 module の責務は `docs/standards/fda-v1/source_architecture.md` を参照します。
Rust module 境界と lightweight architecture gate は `docs/standards/fda-v1/architecture.md` の `Rust module 境界` を参照します。

## v0 成果物

- 成果物カタログ
- Markdown テンプレート
- JSON Schema
- Forge / ATO マッピング
- UI/UX Mission Control 設計方針
- Forge Dashboard Epic のサンプル

## CLI

`fda` は PoC 用の薄い実行 CLI です。最初のサブコマンドは v0 schema と example artifact を検証する `validate-artifacts` です。

```bash
cargo run -- validate-artifacts \
  --out artifacts/runs/<run_id>/validation_report.json
```

既定では `docs/standards/delivery-artifacts-v0/schemas/` の `*.schema.json` を読み、`docs/standards/delivery-artifacts-v0/examples/forge_dashboard_epic/` の同名 JSON artifact を検証します。schema はすべて compile し、example がまだ無い schema は skipped として `validation_report.json` に残します。

あわせて `model_contracts/` と `docs/standards/delivery-artifacts-v0/model_contracts/` の YAML 構文、repo-local `.fda/` 7ファイルprofileを検証します。`validate-artifacts` は不足profileを自動生成せず、validation failure として検出します。追加の model contract directory は `--model-contracts <dir>` で渡せます。

`plan --mode fixture` は、PoC 実行前に runtime の入出力契約を固定するための fixture materialization です。モデル呼び出し、ATO / Forge 書き込み、GitHub PR 作成は行いません。

```bash
cargo run -- plan \
  --requirements docs/standards/delivery-artifacts-v0/examples/forge_dashboard_epic/requirements_definition.md \
  --out artifacts/runs/<run_id> \
  --mode fixture
```

このコマンドは `epic_delivery_plan.json`、`case_graph.json`、`task_graph.json`、`autonomy_contract.json`、`human_decision_packet.json`、`artifact_inventory.json`、`runner_explanation.json`、`validation_report.json` を出力します。

`status` は artifact dir から現在 phase、未解決 Human Decision、notification、QA / repair / merge gate、次に実行すべき command を確認します。

```bash
cargo run -- status --artifacts artifacts/runs/<run_id>
cargo run -- status --artifacts artifacts/runs/<run_id> --json
```

`ui` は全 run を横断する read-only の Mission Control（local HTTP、127.0.0.1 固定）です。Decision Inbox / AI Repair Lane / run 状態を 1 画面に集約し、UI からの状態変更はできません。設計は `docs/v1/mission_control_uiux.md` を参照します。

```bash
cargo run -- ui --open
cargo run -- ui --json
```

### Architecture gate

`src/main.rs` は CLI bootstrap に限定し、抽出済み module の依存境界は軽量 gate で確認します。

```bash
python3 scripts/check_architecture_boundaries.py
```

### Review Agent Gate

ATO / Forge / FDA を使う PR は、`artifacts/review_packets/pr-<PR番号>.md` に `REVIEW_AGENT_GATE` を持つ必要があります。
`fda review` は `pr_reviewer_receipt.json`、`functional_qa_receipt.json`、`security_qa_receipt.json`、`review_agent_gate.json`、`review_agent_gate_packet.md` を生成し、`pr_reviewer`、`functional_qa`、`security_qa` を必須 read-only reviewer として記録します。
ATO / Forge / FDA 証跡や human decision 境界に触れる場合は `forge_reviewer` または `qax2` を実行し、UI / visual 変更がない場合も `design_qa` は `not_applicable` と理由を残します。
V1では `review_agent_gate_packet.md` をPR番号付きreview packetへ自動反映しません。実PR packetへ反映した後に `--pr-number` gateを実行します。
`review_agent_gate_packet.md` があるrunでは、`artifacts/review_packets/pr-<PR番号>.md` へ未反映のまま `fda merge` に進むとblockedになります。
`REVIEW_AGENT_OK` は merge approval ではありません。

```bash
python3 scripts/check_review_agent_gate.py --pr-number <PR番号>
python3 scripts/check_review_agent_gate.py --packet-path artifacts/runs/<run_id>/review_agent_gate_packet.md
```

### AICX Study Bot Slack outbound smoke

PoC-2B では Slack API key がなくても、送信予定 payload と env readiness を dry-run artifact として生成できます。dry-run は Slack API を呼びません。

```bash
python3 scripts/aicx_study_fixture.py slack-smoke \
  --quiz-set docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json \
  --out-dir docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot \
  --artifact-path docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json \
  --preview-count 3 \
  --mode dry-run
```

実送信 smoke は `SLACK_BOT_TOKEN`、`SLACK_CHANNEL_ID`、Python package `slack_sdk` が揃っている場合だけ `--mode live` で実行します。実 token は `.env.local` または shell env に置き、git には入れません。

## Runtime skeleton

v0 成果物を読む実行層の骨格は次に置きます。

- `docs/runtime/ai_delivery_runtime.md`: ATO / Forge / runtime / adapter の責務境界
- `agents/`: AI Delivery Organization の role contract
- `model_contracts/`: role 別の model I/O、権限、禁止事項、監査 key
- `adapters/`: ATO、Forge、GitHub、Codex、sandbox、model provider との接続境界
- `skills/`: v0 契約を使う workflow skeleton
- `evals/`: Epic 分解、判断分離、proof gap、auto-merge 適格性の評価観点

この repo は実行層の実験場です。ATO は状態と証跡、Forge は Claim / Proof / Gate / PromotionDecision の SoT として扱い、runtime がそれらを置換しないようにします。
