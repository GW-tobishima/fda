# Non-Functional Requirements

## 1. Traceability

- 入力元 `cli_goal`、実装可否分類 `implementation_candidate`、Human Decision ID を artifact 間で追跡できること。
- `runner_explanation.json` は stop condition と next action を持つこと。

## 2. Safety

- Intake dry-run は target repo を変更しないこと。
- 未解決 Human Decision がある状態では実装系 command へ進ませないこと。

## 3. Operability

- CLI stdout は未解決判断と再開 command を表示すること。
- Markdown artifact と JSON artifact の両方を生成し、人間確認と機械検証の両方に使えること。

## 4. Input Handling

- 入力要約: FDAの普段使い検証: READMEにClaude Code運用の注意書きを1行追加したい
