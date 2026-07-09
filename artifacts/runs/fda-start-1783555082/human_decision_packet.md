# Human Decision Packet

Status: waiting_human

## 判断が必要です

1. HD-FDA-001: 入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか  
   - recommended: `approve_scope`  
   - required_before: `Design Gate`
2. HD-FDA-002: 実装可否分類 `implementation_candidate` と次 gate `Design Gate` を採用してよいか  
   - recommended: `accept_classification`  
   - required_before: `Design Gate`
3. HD-FDA-003: 外部API、個人情報、法務制約の未記載項目を Design Gate で明示確認する前提で進めてよいか  
   - recommended: `confirm_before_design`  
   - required_before: `Design Gate`

## 続行するには

- `fda decide HD-FDA-001 --answer yes`
- `fda decide HD-FDA-002 --answer "accept"`
- `fda decide HD-FDA-003 --answer "confirm before design"`
