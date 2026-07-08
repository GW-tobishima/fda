# Delivery Registry schema

Delivery Registry は fda v1 の中央 Control Plane projection である。

保存するもの:

- 状態
- 要約
- 判断
- 証跡リンク
- 成果物インデックス
- repo profile / skill version / model contract への参照

保存しないもの:

- Git の実コード
- stdout / stderr の全文
- Codex など AI CLI の会話全文
- モデルの全思考ログ
- 外部 repo の全ファイルコピー

schema:

- `program.schema.json`
- `epic.schema.json`
- `case.schema.json`
- `planned_pr.schema.json`
- `actual_pr_receipt.schema.json`
- `human_decision.schema.json`
- `artifact.schema.json`
- `evidence.schema.json`

