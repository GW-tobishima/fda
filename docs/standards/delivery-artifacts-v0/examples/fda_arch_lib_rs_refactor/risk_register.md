# FDA architecture follow-up risk register

| Risk ID | Level | Risk | Mitigation | Owner |
| --- | --- | --- | --- | --- |
| R-FDA-ARCH-FU-001 | Medium | 型移動で visibility と test import が壊れる | PR-FDA-ARCH-FU-001 を result/helper 移動に限定し、cargo test を必須にする | implementer |
| R-FDA-ARCH-FU-002 | High | implement 移動で MCP approval fail-close が regress する | process port 化を別 PR に分離し、approval prompt tests を維持する | implementer / functional_qa |
| R-FDA-ARCH-FU-003 | High | review/repair/merge 移動で human decision と merge approval の意味が変わる | gate behavior tests と review packet mapping を必須にする | architecture_qa |
| R-FDA-ARCH-FU-004 | Medium | architecture gate が弱まり、lib.rs 再肥大化を見逃す | PR-FDA-ARCH-FU-006 で lib.rs facade gate を追加する | architecture_qa |
| R-FDA-ARCH-FU-005 | Low | docs と実装状態がずれる | 最終 PR で docs、gate、artifact を同時確認する | contract_qa |
