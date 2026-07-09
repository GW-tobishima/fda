# Risk Register

| ID | Risk | Impact | Mitigation | Human Decision |
|---|---|---|---|---|
| R-FDA-001 | 入力解釈が人間の意図とずれる | Design / implementation の手戻り | Scope In / Out を Human Decision として固定する | HD-FDA-001 |
| R-FDA-002 | 実装可否分類が誤る | research / uiux / design / implementation の分岐を誤る | 分類 `implementation_candidate` を Design Gate 前に承認対象にする | HD-FDA-002 |
| R-FDA-003 | 外部API、個人情報、法務制約が未確認 | Security / legal gate で停止する | 未記載制約を Design Gate の確認事項へ送る | HD-FDA-003 |
