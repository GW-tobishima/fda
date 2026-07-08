# Output Hub v1

Output Hub v1 は、Mission Control UI を作る前に、人間が成果物と判断待ちをすぐ確認するための projection である。

Output Hub は実行状態の正本ではない。正本は ATO / Forge / GitHub repo / fda registry に分かれる。Output Hub はそれらへのリンク、要約、verdict、次アクションをまとめる。

## 表示するもの

- Program / Epic 一覧
- Artifact 一覧
- Human Decision 一覧
- External PR Receipt 一覧
- AI Repair Lane の入口
- open artifact link

## 表示しないもの

- stdout / stderr の全文
- AI の会話全文
- GitHub repo のソースファイル複製
- model reasoning log

## 形式

- `output_hub.json`: UI / CLI が読む構造化projection
- `artifact_index.md`: 人間が直接読むMarkdown index

`output_hub.json` は最低限、`programs`、`epics`、`artifacts`、`human_decisions`、`external_pr_receipts`、`ai_repair_lane` を持つ。
Artifact summary は producer、関連 ID、latest version、open link、evidence link、作成/更新時刻を落とさない。
adapter が生成する artifact は `producer_adapter` を必須にし、`producer_agent` は agent 名がある場合だけ付ける。

schema:

- `schemas/output-hub/output_hub.schema.json`

example:

- `examples/output_hub/output_hub.json`
- `examples/output_hub/artifact_index.md`
