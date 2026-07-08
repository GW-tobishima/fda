# Repository Profile 仕様

fda v1 は、対象リポジトリの delivery policy を repo-local な `.fda/` ディレクトリから読む。

```text
target-repo/
  .fda/
    repo.yaml
    delivery_policy.yaml
    skills.lock
    agent_roles.yaml
    gates.yaml
    artifact_map.yaml
    notification.yaml
```

`.fda/repo.yaml` は、対象 repo を fda 対象として発見するための marker である。
delivery-ready な repo として扱うには、上記 7 ファイルが揃っている必要がある。
対象 repo に `.fda/` が無い場合、FDA は設計、実装、レビュー、PR作成へ進む前に `.fda/` folder と必須 profile を作成する。作成できない場合は Human Decision または blocker として停止する。
この 7 ファイルは、対象 repo で AI の動きがぶれないようにするための最小 contract である。

## Runtime Profile Gate

V1-PIVOT-005 以降、次の FDA command は use case 開始時に `pre_work_profile_gate` を実行する。

- repo root に対して実行する command: `start`、`decide`、`design`、`plan`、`open`、`status`、`notify test`
- target repo に対して実行する command: `implement --dry-run`、`implement --live`、`review`、`continue`、`merge`

Profile Gate は既存の `.fda/*` を上書きしない。不足している必須 7 ファイルだけを作成する。

target repo が実在する場合は、その target repo 側にも `.fda/` を作成する。target repo path 自体が存在しない場合、FDA は偽の repository directory を作らない。既存の target repo missing gate が `blocked` または error として扱う。

`validate-artifacts` は例外である。この command は不足 profile を自動生成せず、`.fda/` 7 ファイルが存在しない場合に repository profile validation を fail にする。通常 command は作成担当、`validate-artifacts` は検出担当である。

## `.fda/repo.yaml`

対象 repo の識別子、技術スタック、主要ドキュメント、標準コマンドを定義する。

必須:

- `repo.id`
- `repo.name`
- `repo.default_branch`
- `repo.stack.language`
- `repo.commands`

schema:

- `schemas/repository-profile/repo_yaml.schema.json`

## `.fda/delivery_policy.yaml`

自律実行の上限、人間判断が必要な変更、低リスク path、禁止 action を定義する。

必須:

- `delivery_policy.default_autonomy_level`
- `delivery_policy.auto_merge_allowed`
- `delivery_policy.human_required_for`
- `delivery_policy.forbidden_without_human`

schema:

- `schemas/repository-profile/delivery_policy_yaml.schema.json`

## `.fda/skills.lock`

対象 repo で使用する fda skill 名と version を固定する。
lock された skill 名と version は、fda repo 側の `skills/registry.yaml` に存在する entry として解決できなければならない。

必須:

- `skills`

schema:

- `schemas/repository-profile/skills_lock_yaml.schema.json`

## `.fda/agent_roles.yaml`

current Codex CLI、implementer、read-only reviewer、QA、merge role の executor と workspace policy を定義する。

v1 の既定:

- `implementer.executor=current_codex_cli`
- `implementer.workspace_policy=workspace_write`
- `pr_reviewer`、`functional_qa`、`security_qa` は read-only
- `forge_reviewer` は ATO / Forge / FDA 証跡、handoff、review packet、human decision 境界に触れる場合に必須
- `design_qa` は UI / frontend / browser surface に触れる場合に必須。該当しない場合も not-applicable 理由を残す
- `merge_manager.requires_human_approval=true`

schema:

- `schemas/repository-profile/agent_roles_yaml.schema.json`

## `.fda/gates.yaml`

Profile Gate、Human Decision Gate、Design Gate、Review Agent Gate、Merge Gate を定義する。

v1 では `pre_work_profile_gate.if_missing=create_before_work` を必須にする。missing proof、test not run、trace gap、review packet missing、schema validation failure は Human Decision ではなく AI repair に戻す。

schema:

- `schemas/repository-profile/gates_yaml.schema.json`

## `.fda/artifact_map.yaml`

run artifact、review packet、handoff、sandbox evidence、Output Hub、status artifact の場所を定義する。stdout / stderr 全文やAI会話全文は保存正本にしない。

schema:

- `schemas/repository-profile/artifact_map_yaml.schema.json`

## `.fda/notification.yaml`

Human Turn 通知の channel、Slack Incoming Webhook credential source、email互換adapter、Output Hub連携を定義する。credential値はartifact、ATO、PR本文に保存しない。V1のP0はSlack Incoming Webhookであり、email SMTPはdeprecated/docs-only互換として扱う。

schema:

- `schemas/repository-profile/notification_yaml.schema.json`

## v1 の解釈ルール

- `.fda/` は対象 repo 側の設定正本であり、fda repo 側の中央 registry ではない。
- fda repo 側の `skills/registry.yaml` は skill 名、version、`skill.yaml` path の解決表であり、repo-local policy values の正本ではない。
- `.fda/` が存在しない repo では、FDA 作業開始前に `.fda/` folder と必須 7 ファイルを作成する。
- `default_autonomy_level` は runtime の既定上限であり、Human Decision の必要条件を上書きしない。
- `auto_merge_allowed: false` の repo では、PR が green でも fda は merge approval を自動で出さない。
- `human_required_for` と `forbidden_without_human` に該当する変更は、ATO typed decision に戻す。
- `low_risk_paths` は AI repair / low-risk planning の優先度付けに使う。risk approval の免除ではない。
- `skills.lock` の version が変わる場合は、planned PR / handoff / receipt の再生成差分を Output Hub に残す。

## 例

- `examples/repository_profiles/oshi_note/.fda/`
- `examples/repository_profiles/generic_daily_agent_runtime/.fda/`
