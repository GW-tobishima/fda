# Basic Design

## 1. 目的

Intake で定義された目的を、実装前に検証可能な Design Gate artifact へ落とす。

## 2. 入力要約

FDAの普段使い検証: READMEにClaude Code運用の注意書きを1行追加したい

## 3. Scope In

- Basic Design と Detailed Design を作る。
- Case Graph、Task Graph、Planned PRs、Autonomy Contract、Forge Projection を作る。
- Functional QA brief と Security QA brief を作る。

## 4. Scope Out

- この command は target repo の実装、MCP agent invocation、PR 作成、merge を行わない。
- 実装者、Functional QA、Security QA の実起動は後続 PR の責務とする。

## 5. Acceptance Criteria

- Given Intake の Human Decision が解決済み、When `fda design` を実行する、Then Design Gate artifact が生成される。
- Given Human Decision が未解決、When `fda design` を実行する、Then Design Gate は停止し、判断 ID と再開 command を表示する。
- Given 生成 JSON artifact、When `validate-artifacts` を実行する、Then schema validation が pass する。

## 6. OPEN_QUESTIONS

- 実装PRの分割数は後続 Planned PR refinement で確定する。
- MCP 実装 agent の実 tool capability は PR-V1-005 の dry-run で確認する。

## 7. Risk And Mitigation

- Risk: 設計 artifact が入力意図を過剰に具体化する。Mitigation: Scope Out と Human Decision dependency を Planned PRs に残す。
- Risk: Security QA が Functional QA と混ざる。Mitigation: brief を分離し、Task Graph 上も role を分ける。
