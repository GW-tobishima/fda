# FDA architecture follow-up non-functional requirements

| ID | Category | Requirement | Evidence |
| --- | --- | --- | --- |
| NFR-FDA-ARCH-FU-MAINT-001 | Maintainability | `src/lib.rs` は facade と module export に限定し、command 実装を集約し続けない | lib.rs facade gate、code review |
| NFR-FDA-ARCH-FU-COMPAT-001 | Compatibility | public CLI behavior と artifact compatibility を変更しない | cargo test、command smoke、review packet |
| NFR-FDA-ARCH-FU-SEC-001 | Safety | process IO と approval prompt handling は fail-close のまま維持する | approval prompt tests、process adapter review |
| NFR-FDA-ARCH-FU-TRACE-001 | Traceability | 各 PR は Planned PR、Case、Claim、Proof に mapping される | planned_prs.json、forge_projection.json、PR body |
| NFR-FDA-ARCH-FU-GATE-001 | Verification | architecture gate、cargo test、cargo clippy を各 PR の必須 evidence にする | validation report、PR checks |
