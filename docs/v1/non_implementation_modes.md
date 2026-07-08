# FDA V1 Non-implementation Modes

## 1. 目的

FDA V1 は、すべての入力を実装 PR に変換しない。調査、UIUX、設計のみが適切な依頼では、それぞれの成果物を生成して完了する。

非実装 mode でも、Human Decision、risk、artifact inventory、Output Hub は同じ方針で扱う。

## 2. Mode 一覧

| Mode | Command | 主成果物 | 完了条件 |
|---|---|---|---|
| research | `fda start "..." --mode research` | research report / source refs / risk register | 調査結果と判断事項がまとまる |
| uiux | `fda start "..." --mode uiux` | UIUX brief / user flow / HTML mock / Excalidraw | 体験案と確認点が見える |
| design-only | `fda start "..." --mode design-only` | basic design / detailed design / readiness report | 実装前判断に必要な設計が揃う |
| auto-classified | `fda start "..."` | mode に応じた成果物 | FDA が実装可否分類を出す |

## 3. Auto Classification

`fda start` は入力を次に分類する。

- `implementation_candidate`
- `research`
- `uiux`
- `design_only`
- `needs_clarification`
- `unsupported`

分類基準:

- 実装対象 repo と変更対象が明確なら `implementation_candidate`。
- 外部情報、法務、技術調査が主なら `research`。
- 画面、操作、導線、体験が主なら `uiux`。
- 基本設計、詳細設計、PR計画までが目的なら `design_only`。
- 成功条件や対象が足りなければ `needs_clarification`。
- policy 上扱えない依頼なら `unsupported`。

`needs_clarification` は Human Decision として扱う。AI が勝手に scope を決めて実装へ進めない。

## 4. Research Mode

Command:

```bash
fda start "VTuber PRページの法務リスクを調べて" --mode research
```

成果物:

- `research_report.md`
- `source_refs.md`
- `risk_register.md`
- `human_decision_packet.md`
- `artifact_inventory.json`
- `runner_explanation.json`

受入条件:

- 調査質問が明示されている。
- 参照 source と信頼度が分かる。
- risk と mitigation がある。
- 実装判断に必要な Human Decision がある。
- 次 action が research complete、design、または Human Decision として表示される。

Research mode の禁止事項:

- 調査だけで法務 / privacy / security 判断を確定する。
- source refs なしで断定する。
- 実装が必要なのに PR 計画を省略して完了扱いにする。

## 5. UIUX Mode

Command:

```bash
fda start "この機能のUIUXを考えて" --mode uiux
```

成果物:

- `uiux_brief.md`
- `user_flow.md`
- `mock.html`
- `mock.excalidraw`
- `human_decision_packet.md`
- `artifact_inventory.json`
- `runner_explanation.json`

受入条件:

- ユーザー、目的、主要 workflow が明示されている。
- Human Decision と AI Repair が混ざっていない。
- `artifact_inventory.json` に mock の Output Hub 取り込み metadata がある。実際に Output Hub から開く導線は PR-V1-011 で扱う。
- 実装へ進む場合の Design Gate 入力がある。
- 見た目だけでなく、状態、空状態、失敗状態、判断 UX がある。

UIUX mode の禁止事項:

- Human Decision を内部 agent 状態として埋もれさせる。
- Task / Run / Agent 粒度を人間向け主表示にする。
- mock を作っただけで実装 ready とみなす。

## 6. Design-only Mode

Command:

```bash
fda start "この要件の基本設計まで作って" --mode design-only
```

成果物:

- `basic_design.md`
- `detailed_design.md`
- `case_graph.json`
- `task_graph.json`
- `planned_prs.json`, if useful
- `implementation_readiness_report.md`
- `human_decision_packet.md`

受入条件:

- Requirements と設計が trace できる。
- Scope In / Scope Out がある。
- Acceptance Criteria がある。
- 未解決 Human Decision がある場合は明示される。
- 実装へ進むための不足条件が分かる。

Design-only mode の禁止事項:

- 未解決判断を仮置きして実装 ready とする。
- PR計画が不要な小設計に過剰な graph を強制する。
- Forge Gate に必要な proof obligations を無視する。

## 7. Output Hub

非実装 mode でも `fda open` は成果物を見せる。

PR-V1-010 では `artifact_inventory.json` と Output Hub feed metadata までを生成し、`fda open` による実表示は PR-V1-011 の Output Hub v0 で実装する。

Research:

- report
- source refs
- risk
- decisions

UIUX:

- brief
- flow
- mock
- decisions

Design-only:

- basic design
- detailed design
- readiness report
- decisions

## 8. Completion Policy

非実装 mode の完了は、人間判断なしで完了できる成果物が揃った状態である。

完了にしてよい:

- research report が完成し、未解決判断が次 action として明示されている。
- UIUX mock が生成され、実装判断が別 Human Decision として明示されている。
- design-only 成果物が完成し、implementation readiness が示されている。

完了にしてはいけない:

- 判断が必要なのに `human_decision_packet.md` がない。
- source refs が必要なのにない。
- mock / report / design の artifact path がない。
- `human_decision_packet.md` または `artifact_inventory.json` に成果物と未解決判断が載らない。

## 9. Implementation への昇格

非実装 mode から実装へ進む場合は、必ず次を確認する。

- Human Decision 未解決なし。
- target repo が明確。
- Scope In / Scope Out が明確。
- Acceptance Criteria がある。
- Design Gate の入力が揃っている。
- risk policy が merge / approval 条件を持つ。

昇格 command:

```bash
fda continue --mode implement
```

または:

```bash
fda implement --dry-run
```
