# V1-PIVOT-005 Development Handoff

## EPIC全体のゴール

FDA V1を Codex CLI primary の Delivery Skill Pack / Work Protocol として再定義し、対象repoに `.fda/` 7ファイルprofileが無い場合は、FDA作業開始前に不足profileを作成する。

## 今回のPR境界

- 対象: V1-PIVOT-005 runtime Profile Gate
- 含む:
  - repo root command の `.fda/` profile gate
  - target repo command の `.fda/` profile gate
  - 不足profileのみ作成し、既存 `.fda/*` は上書きしない挙動
  - target repo path自体が存在しない場合は偽repoを作らない挙動
  - `validate-artifacts` は自動生成せず、profile不足を検証failureとして検出する役割分担
- 含まない:
  - current Codex CLIによるfixture-free実装PR作成
  - Review Agent Gateの実起動
  - GitHub merge実行
  - auto merge

## 実装内容

- `src/application/profile.rs` を追加し、`.fda/` 7ファイルの不足生成helperを実装した。
- `start`、`decide`、`design`、`plan`、`open`、`status`、`notify test` で repo root profile gate を実行するようにした。
- `implement --dry-run`、`implement --live`、`review`、`continue`、`merge` で repo root profile gate と、実在する target repo の profile gate を実行するようにした。
- `src/lib.rs` に、既存profileを上書きしないことと、implement dry-run時にtarget repoへ `.fda/` が作られることを確認するテストを追加した。
- README、repository profile仕様、Codex CLI primary正本、source architecture、roadmap、PR sequenceを更新した。

## Runtime State Hygiene

- destructiveなruntime state削除は行っていない。
- temp directoryを使う追加テストはテスト内で削除している。
- `validate-artifacts` により `artifacts/runs/v1-pivot-005-fda-profile-gate-runtime-20260629/validation_report.json` を生成した。

## Validation Results

- `cargo fmt --all -- --check`: pass
- `git diff --check`: pass
- `python3 scripts/check_architecture_boundaries.py`: pass
- `cargo test`: pass, 137 tests
- `cargo check`: pass
- `cargo run -- validate-artifacts --out artifacts/runs/v1-pivot-005-fda-profile-gate-runtime-20260629/validation_report.json`: pass, 62 passed / 0 failed / 39 skipped

## 残リスク

- target repo path自体が存在しない場合、FDAはrepo directoryを新規作成しない。これは偽repo作成を避けるための保守的な判断であり、target repo missingは既存gateでblocked/errorとして扱う。
- 生成される `.fda/repo.yaml` の stack detection は `Cargo.toml` と `package.json` の軽量判定に限定している。未知stackでは `language: unknown` と安全なplaceholder commandを入れる。
- Review Agent Gateの実行、Codex review、PR化、merge approvalはこのPR境界外である。

## 次にすること

V1-PIVOT-006で、current Codex CLI implementer handoffをfixture-free実行へ近づける。具体的には、`implementation_handoff.md` から現在のCodex CLIがapproved scope内の実装・test・PR receipt回収へ進む経路を整える。
