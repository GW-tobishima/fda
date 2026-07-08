# FDA architecture follow-up requirements

## 背景

#44 で `src/main.rs` は CLI bootstrap へ分割済みだったが、この Epic 作成時点では `src/lib.rs` が legacy command 実装の集約点として残っていた。

Epic 開始時点の `src/main.rs` と抽出済み `domain/`、`rendering/`、`application/` の境界は architecture gate 上 pass していた。一方で、`src/lib.rs` は facade ではなく、command 実装、process IO、filesystem helper、artifact rendering helper、validation helper、unit tests を保持していた。

FU-007 完了時点では、`src/lib.rs` の non-test 部分は module export、主要 command facade、共有互換 helper、既存 application module が参照する artifact IO / status 互換 helper に限定される。command の主要な orchestration と process adapter は責務別 module へ移動済みだが、`write_text_file`、artifact carry-forward、implement status / marker parser などの残存 helper は移動済みとは扱わない。

## 目的

`src/lib.rs` を crate facade に戻し、#44 の最終意図である `main -> cli -> application -> domain/rendering/application::ports`、`infra -> application::ports`、`domain -> 外部 IO なし` の責務境界へ段階的に収束させる。

## Scope In

- `lib.rs` から result 型と pure support helper を移す。
- `implement`、`review`、`continue`、`merge`、`open`、`notify` の legacy command 実装を application 層へ移す。
- MCP / Codex process invocation を port と infra adapter へ切り出す。
- prompt / receipt / output hub / notification rendering を rendering 層へ分ける。
- architecture gate に `lib.rs` の facade 化検査を追加する。
- docs の移行段階注記を完了状態へ更新する。

## Scope Out

- CLI 挙動変更。
- artifact schema の破壊的変更。
- crate 分割や workspace 化。
- GitHub issue #68 の自動 close。
- merge / release 承認。

## 成功条件

- `src/lib.rs` の non-test 部分は module export、主要 command facade、共有互換 helper、既存 application module が参照する artifact IO / status 互換 helper に限定される。
- `cli::runner` は `start` / `status` / `decide` / `design` / `plan` / `validate-artifacts` を `application::*` use case へ直接 dispatch し、`implement` / `review` / `continue` / `merge` / `open` / `notify test` は `lib.rs` command facade 経由で `application::*` use case へ dispatch する。
- `domain/` は filesystem、process、schema crate、clock、stdout/stderr に依存しない。
- `rendering/` は文字列または JSON value の組み立てに限定される。
- `application/` は direct な `std::fs`、`std::process`、`jsonschema`、`serde_yaml`、`SystemTime`、stdout/stderr を持たない。
- `python3 scripts/check_architecture_boundaries.py`、`cargo test`、`cargo clippy -- -D warnings` が pass する。

## 参照

- GitHub Issue: https://github.com/msamunetogetoge/forge-delivery-agent/issues/68
- Parent intent: https://github.com/msamunetogetoge/forge-delivery-agent/issues/44
- Source architecture: `docs/standards/fda-v1/source_architecture.md`
- Current architecture note: `docs/standards/fda-v1/architecture.md`
