# fda v1 アーキテクチャ

`forge-delivery-agent` は、v1 では略称として `fda` を使う。

fda v1 は、任意の対象リポジトリに対して、自然言語または要件定義書から Epic Planning、Task/PR 分解、Human Decision 抽出、実装 handoff、PR 証跡回収、ATO / Forge への状態反映までを、同じ skills と設定で再現できる AI Delivery Runtime である。

2026-06-29 の方針変更により、fda v1 の主経路は Codex CLI primary とする。人間が開いた現在の Codex CLI が実装者およびオーケストレーターであり、fda はその中で動く Delivery Skill Pack / Work Protocol である。Codex / Claude MCP direct implementer は v1.5 以降の optional automation layer として扱い、v1 Done blocker にはしない。

正本:

- `docs/v1/codex_cli_primary_architecture.md`

## v1 の境界

fda v1 は、中央集権的にすべてを 1 つの DB に集める仕組みではない。正本を分けた Control Plane として扱う。

| 領域 | 正本 | fda から見た責務 |
|---|---|---|
| ATO | 状態、Summary、Human Turn、Knowledge、Context Snapshot | task / run / decision / evidence を CLI first で記録する |
| Forge | Claim、Proof、Precedent、PromotionDecision、ReleaseDecision | planned PR と proof obligation を gate へ投影する |
| fda | 実行 runtime、skills、repo profile schema / adapter contract、handoff、artifact contract | repo 非依存に delivery flow を再現する |
| GitHub repos | 実コード、PR、CI、repo-local config、repo-local `.fda/` values | code review、CI、merge 結果を外部証跡として返す |

## Codex CLI primary

v1 主経路:

```text
Human -> Codex CLI -> FDA Skill Pack / Work Protocol -> current repo / target repo
```

v1.5 以降の optional automation layer:

```text
FDA Orchestrator -> Codex MCP / Claude MCP -> target repo
```

current Codex CLI は、明示的な role switch、ATO checkpoint、`current_codex_cli_handoff.json`、`implementation_handoff.md`、Review Agent Gate を条件に、orchestrator から implementer へ切り替わってよい。未解決 Human Decision、security High / Critical、privacy / legal、merge / release approval は自己承認しない。

## Rust module 境界

`src/main.rs` は CLI bootstrap に限定する。引数取得、`cli::runner` 呼び出し、`ExitCode` 変換、top-level error 表示だけを持ち、use case 実行、artifact 生成、filesystem IO、schema validation は持たない。

source tree の現在の完了状態と各 module の責務は `source_architecture.md` を正とする。FU-007 完了時点で、`src/lib.rs` の non-test 部分は module export、主要 command facade、共有互換 helper、既存 application module が参照する artifact IO / status 互換 helper に限定される。command の主要な orchestration と Codex process adapter は責務別 module へ移動済みだが、`write_text_file`、artifact carry-forward、implement status / marker parser などの互換 helper はまだ `src/lib.rs` に残っている。この残存 helper を「移動済み」と扱わず、architecture gate は再肥大化と主要 command facade の逸脱を検出する。unit tests は互換維持のため `src/lib.rs` に同居しているため、line count gate は non-test 部分に限定する。

| Module | 現在の責務 | 持ち込まない責務 |
|---|---|---|
| `main.rs` | `std::env::args` の取得、`cli::runner` 呼び出し、`ExitCode` 変換 | CLI parsing、use case orchestration、artifact IO、schema validation |
| `cli/` | CLI option parsing、command dispatch、stdout 表示 | domain policy、artifact rendering、filesystem 直接操作 |
| `application/` | command use case の流れ、artifact path、validation report orchestration、port trait 定義 | CLI 表示、domain policy の決定、外部 IO の concrete 実装 |
| `domain/` | Human Decision、trace、intake、decision、link validation などの純粋な状態と判定 | filesystem、process、schema crate、clock、stdout/stderr |
| `rendering/` | Markdown / JSON value の組み立て | filesystem 書き込み、process 起動、schema validation、gate 承認 |
| `infra/` | filesystem、JSON Schema、YAML、clock、ATO CLI sync の concrete adapter | Human Decision、delivery policy、validation report 集計 |

依存方向は内側へ向ける。

```text
main -> cli
cli -> application
cli -> domain
cli -> infra::ato_state
application -> domain
application -> rendering
application -> application::ports
application -> cli::args
application -> infra
infra -> application::ports
infra::yaml -> application::validate
rendering -> domain
rendering -> support
rendering -> lib.rs compatibility helpers
domain -> std value types only
support -> std/path value helpers only
```

この Epic の完了境界では、crate-level `ports/` module は作らず、`src/application/ports.rs` に trait と adapter input DTO を置く。`AtoConfig` は CLI parsing 層ではなく `application::ports` の DTO とし、`cli::args` は互換 re-export だけを持つ。各 command の config DTO は現在 `cli::args` に残っており、`application/*` は parsing ではなく入力 DTO として受け取る。既定 CLI entrypoint の composition は、下記の documented concrete infra adapter allowlist に限り `application/*` から束ねてよい。

- `application/start.rs`: `SystemClock`、`FsArtifactStore`
- `application/status.rs`: `system_unix_seconds`、`FsArtifactStore`
- `application/design.rs`: `SystemClock`、`FsArtifactStore`
- `application/decide.rs`: `SystemClock`、`FsArtifactStore`
- `application/plan.rs`: `FsArtifactStore`
- `application/implement.rs`: `FsArtifactStore`、`read_json_value`、`write_json_file`、`canonicalize_existing`、`canonicalize_existing_or_parent`
- `application/review.rs`: `FsArtifactStore`、`read_json_value`、`write_json_file`、`canonicalize_existing_or_parent`
- `application/repair.rs`: `FsArtifactStore`、`read_json_value`、`write_json_file`、`canonicalize_existing_or_parent`
- `application/merge.rs`: `FsArtifactStore`、`read_json_value`、`write_json_file`、`JsonSchemaArtifactValidator`、`canonicalize_existing_or_parent`、`run_process_command`
- `application/output_hub.rs`: `FsArtifactStore`、`list_file_names`、`read_json_value`、`write_json_file`
- `application/notify.rs`: `FsArtifactStore`、`write_json_file`、`slack_config_from_env`、`send_slack_notification`、`smtp_config_from_env`、`smtp_envelope_address`、`smtp_message_id`、`send_smtp_notification`
- `application/validate.rs`: `SystemClock`、`FsArtifactStore`、`JsonSchemaArtifactValidator`、`SerdeYamlValidator`、`read_json`、`validate_yaml_dir`
- `infra/yaml.rs`: `application::validate` の `ValidationCheck` builder を使い、YAML syntax check result を同じ report model へ変換する。

`cli -> infra::ato_state` は、`--ato-sync` 明示時の ATO CLI adapter 呼び出しと repo root 正規化 helper に限定する。`cli/` は direct filesystem / process / schema crate / システムクロックを持たない。

`rendering -> lib.rs compatibility helpers` は、`single_line`、`now_unix_seconds`、marker parser、implement gate status / guard 型など、FU-007 時点で `src/lib.rs` に残る互換 helper / 型への限定依存である。

これらの例外は `scripts/check_architecture_boundaries.py` の allowlist と一致させる。新しい concrete infra import や infra から application への依存を追加する場合は、composition root を移すか、この文書と gate の allowlist を同じ PR で更新して、なぜその層に置く必要があるかをレビュー対象にする。

### Architecture gate

module 境界の軽量確認は次で実行する。

```bash
python3 scripts/check_architecture_boundaries.py
```

レビュー観点は次を最低条件とする。

- `src/main.rs` は 100 行以下で、CLI bootstrap だけにする。
- `src/lib.rs` の non-test 部分は 520 行以下で、主要 command facade は `application::*` へ直接委譲する。
- `src/lib.rs` の non-test 部分に `std::process`、network process helper、schema crate、stdout/stderr を再導入しない。
- `cli/` は direct な `std::fs`、`std::process`、schema crate、システムクロックを持たず、`infra/` 依存は `infra::ato_state` の documented allowlist に限定する。grouped `std::{...}` import と `infra as ...` alias import も gate 対象にする。
- `domain/` は filesystem、process、schema crate、system clock、stdout/stderr、`application` / `infra` 依存を持たない。
- `rendering/` は文字列または JSON value を組み立てるだけにし、IO、process、schema validation、`SystemTime` や infra clock adapter を直接持たない。現状は artifact timestamp 投影のため `crate::now_unix_seconds` 互換 helper への依存を許容するが、この依存は拡大しない。
- `application/` は direct な `std::fs`、`std::process`、`jsonschema`、`serde_yaml`、`SystemTime`、stdout/stderr を持たない。
- `application/` から `infra/` への依存は、上記の documented allowlist だけにする。
- `infra/` は concrete adapter を持ってよいが、CLI parsing 層へ依存せず、Human Decision、delivery policy、gate 承認を決めない。`infra/` から `application/` への依存は `application::ports` と `infra/yaml.rs -> application::validate` の documented allowlist に限定し、grouped import の `application::` と `application as ...` alias import も gate 対象にする。
- README の CLI 説明が、`main.rs` と module 境界の現状に反しないことを確認する。

## v1 で扱う状態

Delivery Registry / Control Plane には、状態、要約、判断、証跡リンク、成果物インデックスを入れる。

- Program
- Epic
- Case
- Task
- Planned PR
- Actual PR
- Human Decision
- AI Repair
- Artifact
- Evidence
- Run State
- Repository Profile
- Skill Version
- Model Contract
- Knowledge / Context Snapshot reference

registry schema は `schemas/delivery-registry/` に置く。

次は正本として保存しない。

- Git の実コード
- stdout / stderr の全文
- Codex や他 AI CLI の会話全文
- モデルの全思考ログ
- 外部 repo の全ファイルコピー

ログ本体や長い実行結果は durable artifact に置き、fda / ATO / Forge には evidence edge、要約、verdict、freshness、trust level を戻す。

## ATO CLI first

fda v1 の ATO 連携は CLI first とする。MCP は CLI が使えない環境、または外部 agent integration の後段 transport adapter として扱う。

PR-V1-016 以降、通常 command は `--ato-sync` を明示した場合だけ ATO CLI へ書き戻す。暗黙に外部 state は変更しない。`--ato-task`、`--ato-run-id`、`--ato-backend`、`--ato-db` で対象 ATO backend を固定できる。成功時も失敗時も `ato_state_receipt.json` を artifact dir に残し、raw stdout / stderr 全文や secret 値は保存しない。

`fda start --ato-sync` は ATO task / run / checkpoint を作り、Human Decision を ATO typed decision として開く。`fda decide --ato-sync` は前回 receipt の FDA decision ID と ATO decision ID の対応を使い、`ato decisions answer` と `ato decisions apply` を実行する。その他の stage は主要 artifact を ATO evidence edge として checkpoint に戻す。

最小成果物は次の 3 つである。

- `ato_cli_materialization_plan.json`
- `ato_cli_commands.md`
- `ato_summary_preview.md`

materialization runner は dry-run と execute を分ける。

```bash
fda ato materialize --plan ato_cli_materialization_plan.json --mode dry-run
fda ato materialize --plan ato_cli_materialization_plan.json --mode execute
```

dry-run は ATO へ書き込まず、生成予定 command、summary、decision、evidence edge を検査する。execute は人間判断が不要な AI 側作業だけを `IN_PROGRESS` から `COMPLETED` へ閉じる。

## Repository Profile

対象 repo ごとの差分は `.fda/` に閉じ込める。

- `.fda/repo.yaml`
- `.fda/delivery_policy.yaml`
- `.fda/skills.lock`
- `.fda/agent_roles.yaml`
- `.fda/gates.yaml`
- `.fda/artifact_map.yaml`
- `.fda/notification.yaml`

`.fda/repo.yaml` は fda 対象 repo として認識するための discovery marker である。v1 で delivery-ready と判定するには上記 7 ファイルが必要である。対象 repo に `.fda/` が無い場合、FDA 作業開始前に `.fda/` folder と必須 profile を作成する。

V1-PIVOT-005 以降、Profile Gate は repo root command と target repo command の runtime entrypoint で実行する。既存 `.fda/*` は上書きせず、不足している必須 file だけを作成する。target repo path 自体が存在しない場合は偽repoを作らず、target repo missing gate に戻す。`validate-artifacts` は自動生成せず、profile不足を validation failure として検出する。

仕様と例は `repository_profile.md` を参照する。

## Cross-repo handoff

fda が計画し、対象 repo が実装する場合は、handoff と receipt を明示的に分ける。

- `implementation_handoff.json`
- `current_codex_cli_handoff.json`
- `external_pr_receipt.json`
- `evidence_return_packet.json`

仕様と oshi-note 例は `cross_repo_handoff.md` を参照する。

## Human Decision と AI Repair

fda v1 は、人間判断が必要なものと AI が修復すべきものを混ぜない。

Human Decision にするもの:

- scope 変更
- privacy / terms / legal 判断
- security High / Critical 例外
- public API breaking change
- release approval
- merge / risk / precedent conflict の承認

AI Repair に戻すもの:

- missing proof
- stale evidence
- test not run
- trace gap
- review packet missing
- schema validation failure

## Mission Control v1 の最小面

UI は中央 schema / skill registry の後に実装する。ただし Output Hub は v1 の早い段階で必要である。

Mission Control v1 の最小面:

- Program / Epic Overview
- Decision Inbox
- Output Hub
- AI Repair Lane
- Repository / PR Evidence

人間向け UI の既定表示は Task / Run / Agent 粒度ではなく、成果物、未解決判断、リスク、期限、責任、最終結果、失敗時影響に寄せる。

Output Hub v1 の形式は `output_hub.md` を参照する。

## v1 Definition of Done

fda v1 は次を満たしたときに v1 と呼ぶ。

1. 任意 repo に `.fda/repo.yaml` を置けば fda 対象 repo として認識でき、`.fda/` 7ファイルprofileが揃えば delivery-ready と判定できる。`.fda/` が無いrepoでは、FDA作業開始前に `.fda/` folder と必須profileを作成する。
2. 自然言語または `requirements_definition.md` から Epic Planning 成果物を生成できる。
3. Case Graph / Task Graph / Planned PRs が schema validation を通る。
4. Human Decision Packet が AI Repair と分離される。
5. ATO CLI materialization dry-run ができる。
6. current Codex CLI 向けの `implementation_handoff` を生成できる。
7. target repo の actual PR 結果を `external_pr_receipt` で回収できる。
8. planned PR と actual PR の差分・逸脱が見える。
9. Forge projection / claim / proof mapping がある。
10. Output Hub で成果物一覧を見られる。
11. Mission Control で Program / Epic / Decision の状態を見られる。
12. 同じ skill version なら別 AI / 別 repo でも同じ成果物形式になる。
