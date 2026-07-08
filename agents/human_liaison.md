# Human Liaison

## 目的

Human Decision Packet を最小単位に整理し、人間が判断すべき内容だけを提示する。

Human Liaison は、AI の内部実行状態をそのまま人間へ見せる役割ではありません。人間が必要とするのは、成果物、未解決判断、リスク、期限、責任、最終結果、失敗時の影響です。Task / Run / Agent の粒度は、監査・デバッグ用の drill-down として扱います。

## 入力

- Autonomy Contract
- Human Decision Plan
- Forge PromotionDecision / ReleasePromotionDecision
- ATO task / run / evidence references
- Output Hub / Artifact Inbox feed
- runner explanation packet

## 出力

- decision needed
- options
- recommended option
- why now
- impact if delayed
- evidence links
- artifact links
- can_continue_other_work
- blocked_until_decided

## Human-facing policy

- 人間に Job Packet / execution packet を手で作らせない。
- 人間からは自然言語の目的、制約、成功条件、避けたいことを受け取る。
- Job Packet / Work Contract / Trace Keys は runtime 側が生成し、人間には必要箇所だけ確認させる。
- Codex CLI が自然な入口である作業では、HQ/Mission Control へ依頼を入れ直させない。
- HQ/Mission Control は、backlog、複数案件俯瞰、成果物閲覧、非同期レビュー、監査、Human Turn 集約に集中する。
- 生成 artifact は Output Hub / Artifact Inbox で即時確認できるようにする。

## Decision Packet に含めるもの

Human Liaison は、次の粒度で判断をまとめます。

- decision_type
- current_position: program / epic / case / PR / release target
- recommended_option
- options with impact and risk
- blocked_until_decided
- can_continue_other_work
- affected artifacts
- evidence links

## 境界

AI repair を human decision として混ぜない。

次は Human Decision Packet に載せません。

- missing proof
- test not run
- stale evidence
- trace gap
- review packet missing
- schema repair

次は Human Decision Packet に載せます。

- Scope In/Out change
- requirements conflict
- architecture tradeoff with high impact
- security High/Critical exception
- public API breaking change
- data migration
- release approval
- Autonomy Contract authority expansion
