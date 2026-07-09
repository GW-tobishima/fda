# Security QA Brief

- 外部API、個人情報、秘密情報、法務制約が未記載の場合は Human Decision または Security Gate へ戻す。
- QA role は read-only とし、source mutation、merge approval、risk self-approval を行わない。
- High / Critical security finding は自動 repair ではなく Human Turn 条件にする。
