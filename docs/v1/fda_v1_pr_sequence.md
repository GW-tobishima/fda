# FDA V1 PR Sequence

## 1. 目的

この文書は、`PR-V1-001: FDA V1 CLI Roadmap & Product Contract` の後に出す PR の本数と内容を固定する。

V1 は一度に巨大な実装 PR にしない。CLI contract、artifact contract、Profile Gate、implementation handoff、current Codex CLI execution、QA、repair、merge handoff、非実装 mode、notification、Output Hub を順番に積み、各 PR で検証可能な gate を持たせる。

2026-06-29 のV1 pivotにより、Codex / Claude MCP direct implementerはV1.5 optional automation layerへ退避する。既存PR番号と証跡は壊さず、新方針は `V1-PIVOT-*` または `PR-V1-019+` として追補する。

## 2. 結論

この PR の後に出す PR は、V1 contract coverage までが **10本**、Operational V1 completion までが追加 **7本** とする。

対象:

- V1 contract coverage: `PR-V1-002` から `PR-V1-011`
- Operational V1 completion: `PR-V1-012` から `PR-V1-018`

Operational V1 completion の要件定義とEpicは `docs/v1/fda_v1_operational_epic.md` を正本とする。2026-06-28の差分再調査で、ATO state adapter、Forge gate adapter、Operational E2E proof をV1 blockerとして追加した。

`PR-V1-011` までで command / artifact / gate / receipt の coverage は揃う。ただし、email、GitHub merge handoff、status、current Codex CLI primary execution、ATO state、Forge gate が dry-run または receipt 生成止まりの場合は Operational V1 とは呼ばない。

## 3. PR一覧

| PR | 名前 | 主目的 | 主な成果物 | 完了条件 |
|---|---|---|---|---|
| PR-V1-002 | Intake command contract | `fda start` の dry-run 契約を実装する | requirements / NFR / risk / human decision / runner explanation | CLI stdout と artifact の両方に Human Decision が出る |
| PR-V1-003 | Design command contract | `fda design` の設計成果物生成を実装する | basic design / detailed design / case graph / task graph / planned PRs | Design Gate の必須項目が揃い、判断が必要なら停止する |
| PR-V1-004 | Implementation handoff and optional automation schemas | current Codex CLI handoff / role policyを固定し、MCP schemaはV1.5 optional automationとして保持する | handoff schema / receipt schema / thread state schema / role policy schema | Orchestrator / Implementer / Functional QA / Security QA の境界が schema で表現できる |
| PR-V1-005 | Current Codex CLI implementation gate | target repo を変更する前に `.fda/` Profile Gate、role switch、handoffを検証する | profile gate / implementation handoff / prompt view / readiness receipt | `.fda/` が無い実在repoでは不足profileを作成し、cwd、prompt、approval policy、禁止事項を検証できる |
| PR-V1-006 | Current Codex CLI implementer receipt | current Codex CLIで実装 PR を作れるようにする | current Codex CLI handoff / implementation receipt / external PR receipt / test evidence | planned PR と actual PR が対応し、test 結果が receipt に残る |
| PR-V1-007 | Review agents | PR Reviewer、Functional QA、Security QA を必須 read-only reviewer として分離して実行する | pr reviewer receipt / functional QA receipt / security QA receipt / AC test mapping / review agent gate / review agent gate packet | reviewer 出力が分離され、packet projection が Review Agent Gate checker に通り、FAIL 時の戻し先が決まる |
| PR-V1-008 | Repair loop | QA FAIL や missing proof を AI repair へ戻す | repair receipt / retry history / failure classification | retry 上限、同一原因分類、Human Turn 条件が明確になる |
| PR-V1-009 | PR / Merge gate | Forge / CI / QA / risk に基づく merge 判定を実装する | merge receipt / approval packet / gate summary | V1ではauto mergeせず、merge可能状態とhuman-only approvalを分けて止まる |
| PR-V1-010 | Non-implementation modes | research / uiux / design-only を成果物完了にする | research report / UIUX mock / design-only readiness / `human_decision_packet.md` | 実装不可でも report / mock / design artifact が出る |
| PR-V1-011 | Notification and Output Hub v0 | Human Decision 通知と成果物閲覧導線を作る | notification request / receipt / output hub / decision inbox / execution status | Human Decision で通知され、成果物が Output Hub で見える |

`PR-V1-011` までで V1 contract coverage は揃う。ただし Operational V1 と呼ぶには、外部 adapter が dry-run / fixture / receipt 生成止まりではなく、実運用で動く必要がある。

Codex CLI primary rebaseline の追補 Pivot PR:

| Pivot PR | 対応Case | 目的 | Done |
|---|---|---|---|
| V1-PIVOT-001 | CASE-CLI-001 | Codex CLI primary architecture 正本化 | V1主経路が `Human -> Codex CLI -> FDA Skill Pack -> repo` として文書化される |
| V1-PIVOT-002 | CASE-CLI-002 / CASE-CLI-008 | MCP primary文書をV1.5 optional automationへ再配置 | V1 Done blockerからMCP direct implementerが外れる |
| V1-PIVOT-003 | CASE-CLI-003 | `.fda/` profile schema / examples整備 | 7ファイルprofileがvalidation対象になる |
| V1-PIVOT-004 | CASE-CLI-004 | FDA run model / Output Hub / status artifact固定 | 再開可能なrun artifact契約が決まる |
| V1-PIVOT-005 | CASE-CLI-005 | runtime Profile Gate実装 | `.fda/` が無い実在repoでは作業前に不足profileを作成する |
| V1-PIVOT-006 | CASE-CLI-006 | current Codex CLI implementer handoff実装 | `current_codex_cli_handoff.json` が生成される |
| V1-PIVOT-007 | CASE-CLI-007 | Review Agent Gate必須reviewer artifact生成 | `review_agent_gate.json` が生成される |
| V1-PIVOT-008 | CASE-CLI-007 | Review Agent Gate packet projection生成 | `review_agent_gate_packet.md` が `check_review_agent_gate.py --packet-path` に通る |
| V1-PIVOT-009 | CASE-CLI-007 | PR review packet反映方針固定 | V1ではPR番号付きreview packetへ自動反映せず、明示コマンドまたは人間確認後に反映する |
| V1-PIVOT-010 | CASE-CLI-007 | PR review packet未反映のmerge前block | `review_agent_gate_packet.md` があるrunでは、`artifacts/review_packets/pr-<PR番号>.md` に未反映のまま `fda merge` が通らない |

Operational V1 の追加 PR:

| PR | 名称 | 目的 | 主な成果物 | Done |
|---|---|---|---|---|
| PR-V1-012 | Slack live notification adapter | P0 Slack Incoming Webhookで実送信できるようにする | `fda notify test --channel slack --live`、Slack send receipt、credential policy | `FDA_SLACK_WEBHOOK_URL` が設定されている場合にSlackへ送信し、未設定なら fail-closed で status に出る |
| PR-V1-013 | Status command and CLI discoverability | `fda status` で現在 phase / 未解決判断 / 通知状態 / 次 command を表示し、helpを実装済みcommandと一致させる | status summary、decision summary、notification summary、help更新 | artifact と ATO state から現在地が分かる |
| PR-V1-014 | GitHub merge execution adapter | Merge Gate 通過後に実 GitHub merge 結果を回収する | external merge receipt、merge result receipt | policy が許す PR を merge し、失敗時は Human Decision か repair に戻る |
| PR-V1-015 | Fixture-free current Codex CLI execution evidence | PR-V1-006のcurrent Codex CLI実行を fixture なしで検証する | live_execution_evidence、PR receipt、test receipt | current Codex CLI が実装 PR を作り、QA/repair/merge gate へ渡せる。未完了時もfixture成功扱いにせず失敗証跡を残す |
| PR-V1-016 | ATO state adapter | FDA CLI自身がATO task / run / decision / evidenceへ書き戻す | `ato_state_receipt.json`、decision sync receipt、evidence edge receipt | `--ato-sync` 明示時に `fda start` / `decide` / 各stageがATOへ主要stateを書き戻せる |
| PR-V1-017 | Forge gate adapter | Forge Claim / Proof / PromotionDecisionをmerge判断へ反映する | forge_promotion_receipt、claim/proof mapping、promotion decision summary | PromotionDecisionが `promote` でない場合はmergeしない |
| PR-V1-018 | Operational V1 E2E proof pack | 実装runと非実装modeの代表E2E証跡を揃える | end_to_end_receipt、status_summary、Output Hub proof | V1 Done DefinitionをfixtureなしCodex CLI primary証跡で確認できる。PR #87でactual PR URL、test status、scope evidenceを回収する |

PR-V1-018 の proof pack は `docs/standards/delivery-artifacts-v0/examples/fda_v1_operational_e2e/` に置く。`end_to_end_receipt.status=succeeded` でなければ Operational V1 完了ではない。2026-06-29時点では、PR #87がfixture-free current Codex CLI primary実行として actual PR URL / test status / scope evidence を回収し、MCP live未達blockerをV1.5 optional automationへ退避した。

## 4. 分割理由

### PR-V1-002 と PR-V1-003 を分ける理由

Intake と Design は停止条件が違う。Intake は人間が渡した目的を要件と判断点へ変換する。Design は実装前の構造、PR計画、QA brief を作る。ここを同じ PR にすると、Human Decision の責務と Design Gate の責務が混ざる。

### PR-V1-004 と PR-V1-005 を分ける理由

handoff / role policy schema は静的契約であり、implementation gate は実際の repo profile、cwd、approval policy、禁止事項の検出である。schema を先に固定し、gate で current Codex CLI が実装へ進める状態を検証する。MCP schema / dry-runはV1.5 optional automationとして保持する。

### PR-V1-006 と PR-V1-007 を分ける理由

実装者は write 権限を持つが、QA は read-only である。live implementer と QA を同じ PR にすると、権限境界と receipt 境界が曖昧になる。

### PR-V1-008 と PR-V1-009 を分ける理由

Repair loop は AI 側で戻せる失敗を扱う。Merge gate は human-only approval と merge readiness policy を扱う。V1ではauto mergeせず、missing proof や test not run を merge approval に混ぜないために分ける。

### PR-V1-010 と PR-V1-011 を分ける理由

非実装 mode は成果物生成の範囲であり、notification / Output Hub は人間の受領体験の範囲である。Output Hub は research / uiux / design-only の成果物も表示するため、最後に統合する。

## 5. 各PRのGate

| PR | Gate |
|---|---|
| PR-V1-002 | Intake Gate |
| PR-V1-003 | Design Gate |
| PR-V1-004 | Schema Gate |
| PR-V1-005 | Profile Gate / Implementation Handoff Gate |
| PR-V1-006 | Development Gate |
| PR-V1-007 | Review Agent Gate / Functional QA Gate / Security QA Gate |
| PR-V1-008 | Repair Gate |
| PR-V1-009 | Merge Gate |
| PR-V1-010 | Non-implementation Completion Gate |
| PR-V1-011 | Notification Gate / Output Hub Gate |

## 6. PR作成時の共通条件

各 PR は次を満たす。

- Human Decision 未解決時は実装または merge へ進まない。
- ATO checkpoint に進捗と検証を残す。
- schema / artifact / receipt がある場合は検証を実行する。
- docs と CLI behavior が矛盾しない。
- review comment にはすべて回答する。
- Codex / Claude / GitHub / email の adapter availability は検出結果として扱い、存在を hardcode しない。

## 7. V1到達条件との対応

| V1 到達条件 | 対応PR |
|---|---|
| CLI からやりたいことを入力できる | PR-V1-002 |
| 要件定義書と Human Decision が生成される | PR-V1-002 |
| `fda decide` で判断を記録できる | PR-V1-002 |
| `fda design` で設計成果物を作れる | PR-V1-003 |
| 判断が必要なら止まり、通知できる | PR-V1-002, PR-V1-011 |
| current Codex CLI で実装できる | PR-V1-005, PR-V1-006 |
| PR Reviewer / Functional QA / Security QA を別 read-only reviewerとして実行できる | PR-V1-007 |
| QA FAIL なら repair loop に入れる | PR-V1-008 |
| PR を作成できる | PR-V1-006 |
| Forge Gate を確認できる | PR-V1-009 |
| policy に応じて merge または Human approval に回せる | PR-V1-009 |
| 調査 / UIUX / 設計のみで成果物を出せる | PR-V1-010 |
| Output Hub で成果物を見られる | PR-V1-011 |
| `status` で今どこか分かる | PR-V1-013 |
| ATO task / run / decision / evidence と同期できる | PR-V1-016 |
| Forge PromotionDecisionをmerge判断へ反映できる | PR-V1-017 |
| Operational V1のE2E証跡が揃う | PR-V1-018 |
