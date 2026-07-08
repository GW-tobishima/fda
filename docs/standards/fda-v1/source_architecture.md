# fda v1 Source Architecture

この文書は、FU-007 完了時点の `src/` 構成と責務境界を定義する。
`src/lib.rs` は legacy command 実装の集約点ではなく、crate public surface、module export、主要 command facade、共有互換 helper、既存 application module が参照する artifact IO / status 互換 helper の置き場として扱う。`write_text_file`、artifact carry-forward、implement status / marker parser などは残存 helper であり、移動済みとは扱わない。

## 現在の source tree

```text
src/
  main.rs
  lib.rs
  cli/
    mod.rs
    args.rs
    output.rs
    runner.rs
  application/
    mod.rs
    decide.rs
    decisions.rs
    design.rs
    implement.rs
    merge.rs
    notify.rs
    output_hub.rs
    plan.rs
    ports.rs
    profile.rs
    repair.rs
    review.rs
    runtime.rs
    start.rs
    status.rs
    validate.rs
  domain/
    mod.rs
    entities.rs
    value_objects.rs
    policies/
      mod.rs
      decision.rs
      intake.rs
      trace_links.rs
  rendering/
    mod.rs
    design.rs
    forge.rs
    implement.rs
    intake.rs
    inventory.rs
    merge.rs
    notify.rs
    output_hub.rs
    repair.rs
    review.rs
  infra/
    mod.rs
    ato_state.rs
    clock.rs
    fs_store.rs
    json_file.rs
    json_schema.rs
    paths.rs
    process.rs
    smtp.rs
    yaml.rs
  support/
    mod.rs
    paths.rs
```

## 依存方向

依存方向は内側へ向ける。外部 IO を domain に入れない。

```text
main -> cli
cli -> application
cli -> domain
cli -> infra::ato_state
application -> cli::args
application -> domain
application -> rendering
application -> application::ports
application -> infra
infra -> application::ports
infra::yaml -> application::validate
rendering -> domain
rendering -> support
rendering -> lib.rs compatibility helpers
domain -> std value types only
support -> std/path value helpers only
```

`application -> cli::args` は、各 command の config DTO を受け取るための現在の互換依存であり、application が CLI parsing を行うことを意味しない。
`cli -> domain` は、`cli::args` が `IntakeMode` と `CodexLiveStatus` などの domain DTO / value object を CLI config 型として保持する現在の依存である。
`cli -> infra::ato_state` は、`--ato-sync` 明示時の ATO CLI adapter 呼び出しと、そのための repo root 正規化 helper に限定する。`cli/` は `std::fs` を直接呼ばない。
`application -> infra` は、既定 CLI entrypoint の composition root がまだ application use case 内にあるための documented allowlist に限定する。
`infra::yaml -> application::validate` は、YAML syntax check result を既存の `ValidationCheck` report model に変換するための限定依存である。
`rendering -> lib.rs compatibility helpers` は、`single_line`、`now_unix_seconds`、`marker_value`、`parse_pr_number_from_url`、`DryRunGateStatus`、`HumanDecisionGuard` など、FU-007 時点で `src/lib.rs` に残る互換 helper / 型への限定依存である。これは facade coupling の現状追認であり、拡大しない。

この Epic の完了境界では、crate-level `ports/` module は作らず、`src/application/ports.rs` に trait と adapter input DTO を置く。top-level `ports/` への再配置は、この Epic の未完了事項ではなく、将来必要になった場合に別 architecture PR として扱う。

## 各 module の責務

| Module | 責務 | 持たない責務 |
| --- | --- | --- |
| `main.rs` | `std::env::args` の取得、`cli::runner` 呼び出し、`ExitCode` 変換、stderr への top-level error 表示 | 引数パース、use case 実行、artifact 生成、filesystem IO、schema validation |
| `lib.rs` | crate public surface の接続、module export、主要 command facade、互換維持用の共有 helper、既存 application module が参照する artifact IO / status helper | 新しい command 実装、process/network adapter、artifact rendering helper の集約、残存互換 helper の拡大 |
| `cli/args.rs` | `Command`、各 command config DTO、CLI option parsing、help 対象 command の識別、`AtoConfig` の互換 re-export | use case 実行、artifact 書き込み、domain policy |
| `cli/output.rs` | CLI stdout summary、help 表示、human-readable な結果表示 | JSON artifact 生成、validation 判定、業務判断 |
| `cli/runner.rs` | `Command` から application use case / lib facade への dispatch、JSON stdout と summary stdout の切り替え、明示 `--ato-sync` 時の ATO sync adapter 呼び出し | 引数パース詳細、domain policy、filesystem の直接操作 |
| `application/start.rs` | intake use case、requirements / NFR / risk / decision packet の生成 orchestration | CLI 表示、filesystem concrete 実装 |
| `application/decide.rs` | FDA Human Decision answer の記録、decision receipt 生成 orchestration | CLI parsing、stdout 表示、ATO decision apply |
| `application/decisions.rs` | Human Decision packet / receipt の読み取り、answer 抽出、summary 変換 helper | filesystem concrete 実装、CLI 表示 |
| `application/design.rs` | Design Gate artifact の生成 orchestration、decision blocker 判定 | CLI parsing、stdout 表示、schema crate 直接利用 |
| `application/plan.rs` | fixture plan の materialization、artifact inventory 生成 orchestration | CLI 表示、domain policy の変更 |
| `application/implement.rs` | implement dry-run/live gate、Codex MCP invocation plan、handoff / receipt orchestration | process 実装、CLI parsing、stdout 表示 |
| `application/review.rs` | Functional QA / Security QA prompt と QA receipt orchestration | QA role policy の無断変更、process 実装 |
| `application/repair.rs` | repair loop prompt、retry history、repair receipt orchestration | retry 上限を超えた独断継続、process 実装 |
| `application/merge.rs` | merge gate、risk / CI / Human Decision policy、merge receipt orchestration | merge approval の自己承認、GitHub process 実装の隠蔽 |
| `application/output_hub.rs` | Output Hub / Decision Inbox / Execution Status の投影 orchestration | HTML rendering 以外の UI policy 追加、CLI 表示 |
| `application/notify.rs` | notification request / receipt / Human Turn notice の orchestration、recipient 解決 | Slack HTTPS POST実装、SMTP socket 実装、secret の永続化 |
| `application/profile.rs` | `.fda/` 7ファイルprofileの存在確認と不足profileの既定生成 | repo固有policyの承認、既存profileの上書き、存在しないtarget repo directoryの作成 |
| `application/runtime.rs` | artifact dir から `RuntimeContext` を復元する helper | process 起動、gate 判定 |
| `application/status.rs` | artifact dir の status projection、decision blocker summary | CLI 表示、ATO state mutation |
| `application/validate.rs` | schema / YAML / trace link validation report の orchestration と report model | CLI parsing、domain policy の変更 |
| `application/ports.rs` | application / infra adapter が共有する filesystem、artifact validation、YAML validation、process、clock、ATO config DTO の trait 境界 | `std::fs`、`jsonschema`、`serde_yaml`、`SystemTime` の concrete 実装、CLI parsing |
| `domain/entities.rs` | Human Decision、trace plan、case、claim、planned PR、runtime context、Codex tool status などの純粋な状態 | filesystem、process、stdout、jsonschema crate への依存 |
| `domain/value_objects.rs` | Intake mode など、CLI や application から共有される小さな値 object | CLI parsing、filesystem、process、stdout、jsonschema crate への依存 |
| `domain/policies/*` | intake classification、decision approval/blocker、trace link validation などの純粋な判定 | artifact 読み書き、stdout、schema validation、process 起動 |
| `rendering/intake.rs` | Intake の requirements / NFR / risk register / Human Decision packet / runner explanation を Markdown または JSON value として組み立てる。現状は `crate::single_line` 互換 helper を使う | IO、CLI parsing、intake classification の判定 |
| `rendering/design.rs` | Design Gate の basic design / detailed design / QA brief / case graph / task graph / planned PR / autonomy contract / runner explanation を組み立てる | IO、Human Decision blocker 判定、schema validation |
| `rendering/inventory.rs` | start / design / plan / implement 系の artifact inventory entry と timestamp projection を組み立てる | artifact 書き込み、path 解決、validation 判定、direct `SystemTime` 依存 |
| `rendering/forge.rs` | Forge projection や proof obligation projection を JSON value として組み立てる | Forge への永続化、promotion 判定、gate 承認 |
| `rendering/implement.rs` | implement handoff、Codex prompt、MCP invocation plan、dry-run receipt の JSON / Markdown を組み立てる。現状は `marker_value`、`parse_pr_number_from_url`、`DryRunGateStatus`、`HumanDecisionGuard` などの `lib.rs` 互換 helper / 型を使う | Codex process 起動、tool 実行、artifact 書き込み |
| `rendering/review.rs` | QA prompt、QA receipt、AC mapping projection を組み立てる | QA 実行、schema validation、gate 承認 |
| `rendering/repair.rs` | repair prompt、repair receipt、retry history projection を組み立てる | retry 実行、process 起動 |
| `rendering/merge.rs` | merge receipt、runner explanation、Forge promotion receipt projection を組み立てる | GitHub process 起動、merge approval |
| `rendering/output_hub.rs` | Output Hub / Decision Inbox / Execution Status HTML を組み立てる | filesystem 書き込み、artifact 読み取り |
| `rendering/notify.rs` | notification request / receipt / Human Turn notice / Slack payload / SMTP message body を組み立てる | Slack HTTPS POST、SMTP socket 接続、secret 読み取り |
| `infra/fs_store.rs` | `ArtifactStore` の filesystem 実装、artifact file listing | use case policy、artifact schema の判定 |
| `infra/json_file.rs` | JSON file read/write helper | domain policy、validation report 集計 |
| `infra/json_schema.rs` | `jsonschema` crate を使う JSON Schema compile / validation adapter | validation report の集計、artifact path orchestration |
| `infra/yaml.rs` | `serde_yaml` を使う YAML syntax validation adapter と validation check 変換 | model contract の意味解釈、gate 承認 |
| `infra/clock.rs` | `SystemTime` を使う clock adapter | command 固有の timestamp policy |
| `infra/paths.rs` | filesystem path canonicalization adapter | domain policy、CLI 表示 |
| `infra/process.rs` | Codex MCP / child process invocation と process output adapter | Human Decision 内容、delivery policy、review 判定 |
| `infra/slack.rs` | Slack Incoming Webhook config resolution、URL validation、HTTPS送信 adapter、response digest | notification policy、Human Turn 判定、secret 永続化 |
| `infra/smtp.rs` | SMTP config resolution、message id、envelope address、SMTP送信 adapter | notification policy、Human Turn 判定、secret 永続化 |
| `infra/ato_state.rs` | `--ato-sync` 明示時に ATO CLI process、ATO receipt IO、ATO command receipt 正規化を行う adapter | command use case policy、Human Decision 内容の決定、CLI parsing 層への依存 |
| `support/paths.rs` | path 表示、relative path、artifact path 解決などの pure helper | command 固有の業務ロジック、filesystem mutation |

## command ごとの配置

| Command | CLI entry | Use case | 主な domain | 主な rendering | 主な infra/ports |
| --- | --- | --- | --- | --- | --- |
| `start` | `cli::runner` | `application::start` | `domain::policies::intake`、Human Decision | requirements / NFR / risk / decision packet | artifact store、clock、schema validator |
| `status` | `cli::runner` | `application::status` | `domain::policies::decision` | status projection | artifact store、clock |
| `decide` | `cli::runner` | `application::decide` | `domain::policies::decision` | decision receipt | artifact store、clock |
| `design` | `cli::runner` | `application::design` | `domain::policies::decision`、design gate、task graph policy | design docs、case graph、task graph | artifact store、clock、schema validator |
| `plan` | `cli::runner` | `application::plan` | fixture materialization policy | runtime fixture artifacts | artifact store、schema validator |
| `implement` | `lib.rs` facade / `cli::runner` | `application::implement` | implement gate、adapter result classification | handoff、dry-run receipt、implementation receipt | process port、artifact store、path adapter |
| `review` | `lib.rs` facade / `cli::runner` | `application::review` | review gate、QA status classification | QA prompt、QA receipt、AC mapping | artifact store、schema validator |
| `continue` | `lib.rs` facade / `cli::runner` | `application::repair` | repair gate、retry policy、failure classification | repair prompt、repair receipt、retry history | process port、artifact store |
| `merge` | `lib.rs` facade / `cli::runner` | `application::merge` | merge gate、risk / CI / Human Decision policy | merge receipt、runner explanation、Forge promotion receipt | process runner、artifact store、schema validator |
| `open` | `lib.rs` facade / `cli::runner` | `application::output_hub` | artifact / decision / status projection policy | Output Hub HTML、Decision Inbox HTML、Execution Status HTML | artifact store、JSON file helper |
| `notify test` | `lib.rs` facade / `cli::runner` | `application::notify` | recipient resolution、notification status policy | notification request、receipt、Human Turn notice、Slack payload、SMTP本文 | artifact store、Slack adapter、SMTP adapter、clock |
| `validate-artifacts` | `cli::runner` | `application::validate` | validation summary model、trace link policy | validation report | filesystem、schema validator、YAML validator |

## 完了状態の維持ルール

`start` / `decide` / `design` / `plan` / `open` / `status` / `notify test` は repo root に対して `application::profile` の Profile Gate を実行する。`implement` / `review` / `continue` / `merge` は repo root に加え、実在する target repo に対しても Profile Gate を実行する。`validate-artifacts` は不足 profile を作成せず、検証 failure として扱う。

- 挙動変更なし PR では、関数移動と visibility 調整に限定する。
- `domain` に `std::fs`、`std::process`、`jsonschema`、`serde_yaml`、stdout/stderr を持ち込まない。
- `cli/` は `std::fs`、`std::process`、システムクロック、schema crate を直接呼ばない。`--ato-sync` 用の repo root 正規化は `infra::ato_state` adapter helper に寄せる。
- `application` は artifact path や report の流れを扱ってよいが、concrete infra 依存は `architecture.md` と `scripts/check_architecture_boundaries.py` の documented allowlist に限定する。
- `rendering` は文字列や `serde_json::Value` を返すだけにし、ファイルへ直接書かない。artifact timestamp projection では現状 `crate::now_unix_seconds` 互換 helper を使ってよいが、direct `SystemTime` や infra clock adapter 依存は持たない。`single_line`、marker parser、implement gate status / guard 型などの `lib.rs` 互換 helper 依存は現状追認であり、拡大しない。
- `infra` は concrete IO を持ってよいが、Human Decision や gate policy を決めず、CLI parsing 層へ依存しない。
- `src/lib.rs` non-test 部分に command 実装、process/network adapter、artifact rendering helper を戻さない。残存している artifact IO / status 互換 helper は現状追認であり、拡大しない。
- 新しい command を追加する場合は、`cli/args.rs`、`cli/runner.rs`、`application/<command>.rs`、必要な domain/rendering/port を同じ方向で追加する。

## Architecture gate

`scripts/check_architecture_boundaries.py` は、この完了状態を軽量に fail-close する。

- `src/main.rs` は 100 行以下にし、CLI bootstrap だけを持つ。
- `src/lib.rs` の non-test 部分は 520 行以下にする。
- `implement` / `review` / `continue_run` / `merge_run` / `open_output_hub` / `notify_test` は `application::*` use case へ直接委譲する。
- `std::process`、network process helper、schema crate、stdout/stderr を `src/lib.rs` non-test 部分へ戻さない。
- `cli/` は direct な `std::fs`、`std::process`、システムクロック、schema crate を持たず、`infra/` 依存は `infra::ato_state` の documented allowlist に限定する。grouped `std::{...}` import と `infra as ...` alias import も gate 対象にする。
- `domain/` は filesystem、process、schema crate、system clock、stdout/stderr、`application` / `infra` 依存を持たない。
- `rendering/` は IO、process、schema validation、direct `SystemTime`、infra clock adapter を持たない。`crate::now_unix_seconds` 互換 helper による timestamp projection は現状許容する。
- `application/` は direct な `std::fs`、`std::process`、`jsonschema`、`serde_yaml`、`SystemTime`、stdout/stderr を持たない。
- `application/` から `infra/` への依存は、`architecture.md` の documented allowlist と一致させる。
- `infra/*` から `cli/*` へ依存しない。`infra/*` から `application/*` への依存は `application::ports` と `infra/yaml.rs -> application::validate` の documented allowlist に限定し、grouped import の `application::` と `application as ...` alias import も gate 対象にする。
- unit tests は互換維持のため `src/lib.rs` に同居してよい。line count gate は tests を除外する。
