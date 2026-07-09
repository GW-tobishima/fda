# FDA V1.5 移植ガイド（AI 向け）

- 作成日: 2026-07-09
- 対象ブランチ: `main`（`GW-tobishima/fda`、V1.5 実装レンジ `a03e30c..c899032`）
- 対象読者: このフォークの V1.5 変更を **別リポジトリ（upstream `msamunetogetoge/forge-delivery-agent` 等）へ移植する AI**（Claude Code / Codex CLI）
- 目的: V1.5 の 6 機能を、**契約（schema）・不変条件・配線箇所・テスト観点**を落とさずに再実装するための正確な仕様書。人間向け解説ではない。

> このガイドは「何を作るか」ではなく「**どの契約と不変条件を守れば V1.5 と等価になるか**」を定義する。
> 各機能の一次情報源（このフォーク内）は本文中にパスで示す。移植先で判断に迷ったら、必ず一次情報の実物を読むこと。

---

## 1. 前提と全体像

### 1.1 V1.5 のテーマ

**「関所を減らさずに、人間がハンコ係を卒業する」**（`artifacts/runs/fda-start-1783555082/detailed_design.md` §「V1.5 実設計」）。
ゲート（関所）の**種類は一切減らさず**、人間が個別回答者から立法者へ育つための projection と委任機構を足す。

全 PR 共通の不変条件（これを破る移植は V1.5 ではない）:

- Human Decision の自己承認をしない（委任契約の**制定は常に人間**）。
- V1 の **auto merge なし**方針を維持（merge 承認は常に人間）。
- `fda.*.v0` 既存 schema は**改名しない**。追加は新 version（`*.v1`）か追記フィールドのみ。
- **fail-closed 維持**。ゲートの種類は減らさず、重さだけを risk に比例させる。
- architecture gate（`scripts/check_architecture_boundaries.py`）維持、CI green（ubuntu / windows）維持。

### 1.2 6 機能と推奨移植順

| 機能 | 内容 | 主なファイル |
|---|---|---|
| F6 表層分け | work protocol 単一正本化 + 4 概念カーネル help | docs / `src/cli/output.rs` |
| F4 比例ゲート | risk tier 判定 + merge/review の比例緩和（種類は減らさない） | `src/application/risk_tier.rs`, `merge.rs` |
| F5 庭師 | `fda gc`（stale run 棚卸し docket、破壊操作なし） | `src/application/gc.rs` |
| F1 判断の立法化 | delegation contract + `fda policy propose` + `decide --by-contract` | `src/application/policy.rs`, `decide.rs` |
| F2 Epic 継続ループ | `fda continue --epic`（read-only projection） | `src/application/epic.rs` |
| F3 道場 UI | Mission Control に判断→帰結の read-only projection | `src/application/ui.rs`, `src/rendering/mission_control.rs` |

**推奨移植順: F6 → F4 → F5 → F1 → F2 → F3**（このフォークでの実装順 PR-V15-001..005 と同一）。理由:

- **F6 が最初**: work protocol 正本（`docs/v1/work_protocol.md`）と 4 概念語彙を先に確立しないと、後続の禁止事項・ゲート表現の参照先が無い。docs 中心で blast radius 最小。
- **F4 → F5**: F4 の `risk_tier.rs` が導入する `assess_risk_tier` / delivery_policy 読取と、`infra/fs_store.rs::modified_unix_seconds`（F5 の mtime 取得で再利用）を先に置くと重複が減る。F5 は F4 と同 PR（PR-V15-002）で入った。
- **F1 は F4/F5 の後**: `policy.rs::normalize_summary_signature` を F1 で `pub(crate)` 化する。これは F3 の precedent 照合が依存する。人間権限委任を扱うため risk=high、単独 PR で慎重に。
- **F2 は F1 の後**: epic 投影は decision receipt / merge receipt を読む。F1 の delegation contract 適用の receipt 形（`decided_by=delegation_contract:...`）を先に確定させておく。
- **F3 が最後**: UI は F1（precedent / 契約バッジ）・F2（epic_progress）・F5（gc_docket）の全成果物を projection する集約点。実 schema が確定してから作らないと fixture が架空キーになる（→ §4 の罠）。

### 1.3 移植先に既に必要な V1 前提

移植先が以下を備えていること（無ければ V1.5 は載らない）:

- `.fda/` profile: `delivery_policy.yaml`（`low_risk_paths` / `human_required_for` を持つ）、`agent_roles.yaml`、`gates.yaml` 等。
- **review agent gate**: `scripts/check_review_agent_gate.py` + `tests/test_review_agent_gate.py` + `artifacts/review_packets/pr-<N>.md` の packet 契約（REVIEW_AGENT_OK 行 / 必須 reviewer）。
- **merge gate**: `src/application/merge.rs` の fail-closed 判定（必須 reviewer 検証、external_pr_receipt / qa_receipt 検証）。
- **artifacts 構造**: `artifacts/runs/<run>/` に各種 receipt を置く run dir モデル、`docs/standards/delivery-artifacts-v0/schemas/` の schema registry と `validate-artifacts` コマンド。
- architecture gate（application 層は `SystemTime` / `std::fs` 直呼び禁止、infra 経由のみ。allowlist は `scripts/check_architecture_boundaries.py`）。

---

## 2. 機能ごとの移植仕様

### F6 — 表層分け / work protocol 単一正本化（PR-V15-001, feat `43a3dc0`）

**目的**: 作業プロトコルの 4 重記述（AGENTS.md / CLAUDE.md / skill / help）による drift を、単一正本 + 参照に畳んで根絶する。

**新規 / 変更ファイル**:
- 新規: `docs/v1/work_protocol.md`（唯一の正本）、`docs/v1/system_prompt_authoring_guide.md`（正典）。
- 縮退: `AGENTS.md` / `CLAUDE.md` / `.claude/skills/fda-delivery/SKILL.md` を「位置づけ + 正本参照 + 入口固有の注意」だけに。
- `src/cli/output.rs`: `print_help` を 4 概念グループ表示に再編。

**契約（help 表示）**: `fda --help` は 4 概念カーネルで並ぶ。**コマンド仕様・各行の内容は不変**（並べ替えと見出し追加のみ）:
- `[作業 Work]`: start / design / plan / implement / continue / merge
- `[判断 Decision]`: decide / notify
- `[証跡 Evidence]`: review / status / open / ui / validate-artifacts
- 末尾に `[知識 Knowledge] は ato knowledge / ato search を使う (正本: ATO)` の 1 行、および共通 ATO 連携行。

**不変条件（レビューで確定）**:
- **安全文言（禁止事項）を単一正本化で失うな**。security_qa 初回 fail の主因は、`work_protocol.md` に集約する際に「auto merge 禁止」が**単発・例外を含む全面禁止**から「常用禁止」へ縮退し抜け道が生まれたこと（repair `39612bc` で「単発・例外を含め禁止」へ復元）。scope approval 禁止列挙・Forge PromotionDecision 自己承認禁止も脱落させない（repair `1b1d209` / `f589691`）。**畳み込みは情報の削除ではなく参照への置換**であること。
- 未実装機能を既定手順に書かない。`work_protocol.md` §3 で V1.5 の `--by-contract` / `--epic` は「未実装/新規」である旨を明記（初版に無く forge_reviewer low 指摘で追記）。

**配線**: `src/cli/output.rs` の `print_help` のみ（コマンド追加なし）。

**テスト観点**: help 文字列のグループ見出し存在。コマンド行が欠落していないこと。cargo test（このフォークで 183 passed）+ review agent gate（pytest）が pass。

---

### F4 — 比例ゲート / risk tier（PR-V15-002, feat `4a4aacb` + repair `73dbbfe` + polish `ecc8781`）

**目的**: ガバナンス税を blast radius に比例させる。低リスクは conditional reviewer を軽量化するが、**ゲートの種類と必須 reviewer は減らさない**。

**新規 / 変更ファイル**:
- 新規: `src/application/risk_tier.rs`、`docs/standards/delivery-artifacts-v0/schemas/risk_tier.schema.json`。
- 変更: `src/application/merge.rs`（merge gate 比例化 + live 再検証）、`implement.rs`（dry-run で risk_tier.json 出力）、`status.rs`（tier 表示）、`src/rendering/merge.rs`、`scripts/check_review_agent_gate.py`（forge not_applicable の条件付き許容）。

**契約（`risk_tier.schema.json` / `fda.risk_tier.v1`）**:
```json
{
  "required": ["schema_version", "tier", "reasons", "matched_low_risk_paths", "policy_source"],
  "properties": {
    "schema_version": { "const": "fda.risk_tier.v1" },
    "tier": { "enum": ["low", "standard", "high"] }
  },
  "additionalProperties": false
}
```

**不変条件（血を流して確定。絶対に守る）**:
- **保存 tier を信頼するな（TOCTOU 対策）**: merge 時に `changed_files` を `.fda/delivery_policy.yaml` の `low_risk_paths` で **live 再計算**し、**stored=low かつ live=low の両方が成立する場合のみ**緩和を受理。不一致は "stored/live tier mismatch" を記録して緩和不適用（standard 扱い）。実装は `risk_tier.rs::proportional_relaxation`。
- **governance hard guard はコードにハードコードし YAML で上書き不能**: `is_governance_critical_path`（`risk_tier.rs`）が返す真のとき、tier / `low_risk_paths` に**関係なく** forge_reviewer の緩和を却下する。対象（正規化して小文字比較）:
  - `.fda/` 配下すべて（`starts_with(".fda/")` または `contains("/.fda/")`）
  - `scripts/check_review_agent_gate.py`、`scripts/check_architecture_boundaries.py`、`tests/test_review_agent_gate.py`
  - `.github/workflows/ci.yml`
  - `src/application/merge*` / `review*` / `risk_tier*` / `policy*`
  - ※ gate checker 自身・gate のテスト・CI 定義まで対象に含めること（`ecc8781` で `tests/test_review_agent_gate.py` と CI を追補。段階的統治弱体化の経路を塞ぐ）。
- **必須 3 reviewer は不変**: `pr_reviewer` / `functional_qa` / `security_qa` は tier に関係なく blocking。緩和対象は conditional reviewer（forge_reviewer / design_qa）だけで、しかも「種類を消す」のではなく `status=not_applicable + not_applicable_reason に "risk_tier=low"` として**理由付きで記録**する（`review_agent_gate.json` の既存契約を流用）。
- **緩和は二重ゲートにするな（単一表現）**: 緩和は `fda review` が `review_agent_gate.json` に記録し、`merge.rs` は独自スキップ判定を持たず、その主張を live 再検証で「検証するだけ」。`check_review_agent_gate.py` は forge_reviewer 行が `not_applicable` かつ packet に `RISK_TIER: low — <理由>` 行があるときのみ許容（design_qa の既存 not_applicable 許容パスと同型）。
- **defense in depth**: この repo の `.fda/delivery_policy.yaml` からは `low_risk_paths` の `.fda/**` を削除（統治ファイルを low 扱いしない）。
- tier=high は**フルゲート維持のうえ** `human_required_for` 該当を blocking issue として明示。

**CLI 追加**: なし（`implement --dry-run` に risk_tier.json 出力が乗るのみ）。risk_tier.json は implement/review/repair の carry-forward に追加し、dry-run → live → review → merge 全経路で tier を伝搬させる。

**配線**: `merge.rs`（`proportional_relaxation` 呼び出し）、`implement.rs`（生成）、`status.rs` / `rendering/merge.rs`（表示）、`check_review_agent_gate.py`（`risk_tier_low_reason` ヘルパ + `validate_gate` の許容分岐）、`tests/test_review_agent_gate.py`（受理/拒否 2 テスト）。

**テスト観点**（このフォーク: repair 後 207 passed）: relaxation 純ロジック（stored≠low で拒否 / mismatch で拒否 / 一致で受理）、governance hard guard 却下、`is_governance_critical_path` の対象/非対象、merge 受理・却下、review 側の forge/design 緩和生成、dry-run docs-only で tier=low。gate 側の受理/拒否 unittest 2 件。

---

### F5 — 庭師 / `fda gc`（PR-V15-002, feat `4a4aacb`）

**目的**: `artifacts/runs` の stale run・不整合 receipt を棚卸しして人間に例外だけ提示する。**削除は一切しない**。

**新規 / 変更ファイル**:
- 新規: `src/application/gc.rs`、`docs/standards/delivery-artifacts-v0/schemas/gc_docket.schema.json`。
- 変更: `src/infra/fs_store.rs`（`modified_unix_seconds` 追加）、`src/cli/args.rs` / `output.rs` / `runner.rs` / `lib.rs`、`src/application/mod.rs`。

**契約（`gc_docket.schema.json` / `fda.gc_docket.v1`）**:
```json
{
  "required": ["schema_version", "generated_at_unix", "scanned_runs", "candidates"],
  "properties": {
    "schema_version": { "const": "fda.gc_docket.v1" },
    "candidates": { "items": { "required": ["run", "reasons", "recommendation", "needs_human"],
      "properties": { "recommendation": { "enum": ["resume", "archive", "answer_decision"] } } } }
  }
}
```

**不変条件**:
- **read-only + 破壊操作ゼロ**: docket 生成のみ。既存 run への変更・削除は禁止。人間には「例外だけ」を提示（自動アーカイブもしない）。
- **1 run の壊れた JSON で走査全体を abort しない（fail-soft）**: 破損 receipt（例 `ato_state_receipt` / `human_decision_packet`）は `parse_error` 理由の候補（`needs_human=true, recommendation=resume`）として報告し、残り run の走査を続行（pr_reviewer low 指摘の repair）。
- **mtime 取得は infra に集約**: application 層で `std::fs` を直呼びせず、`infra/fs_store.rs::modified_unix_seconds` 経由（architecture gate 準拠）。

**検出 4 種**: (a) stale 未完了 run（`--max-age-days` 超）、(b) `validation_report` 欠落、(c) `ato_state` 非 succeeded、(d) stale 未解決 decision。出力は `<artifacts-root>/_gc/gc_docket.{json,md}`。

**CLI 追加**: `fda gc [--artifacts-root <path>] [--max-age-days 30] [--repo-root <path>] [--json]`。`GcConfig { max_age_days: u64 (default 30) }`（`src/cli/args.rs::parse_gc_args`）。

**テスト観点**（gc 6 + parse_error 2）: 30 日超 stale の検出、既存 run 不変、各検出種、破損 JSON の fail-soft 継続。

---

### F1 — 判断の立法化 / delegation contract（PR-V15-003, feat `8de4e0e` + repair `83cbe7e`）

**目的**: 人間が繰り返し同型で承認した Human Decision を委任契約として明示適用可能にする。ただし**契約の制定・自動適用は一切しない**。

**新規 / 変更ファイル**:
- 新規: `src/application/policy.rs`、`src/support/date.rs`（純関数の暦日比較）、`docs/standards/fda-v1/schemas/repository-profile/delegation_contract_yaml.schema.json`、`docs/standards/delivery-artifacts-v0/schemas/policy_proposal.schema.json`、`docs/standards/fda-v1/examples/delegation_contract.example.yaml`。
- 変更: `src/application/decide.rs`（`--by-contract`）、`decisions.rs`、`status.rs`（ヒント）、`validate.rs`（optional profile 検証）、`profile.rs`、`src/cli/args.rs` / `output.rs` / `runner.rs` / `lib.rs`、`src/support/mod.rs`。

**契約（`delegation_contract_yaml.schema.json`）** — `.fda/delegation_contract.yaml`（**必須 7 ファイルには加えない optional profile**）:
```json
{
  "required": ["delegation_contract"],
  "$defs": { "rule": {
    "required": ["rule_id","decision_type","match_summary_keywords","answer","authority","enacted_from","expires"],
    "properties": {
      "rule_id": { "pattern": "^DC-" },
      "match_summary_keywords": { "minItems": 1, "items": { "minLength": 4 },
        "description": "全キーワードの AND 一致（包括委任の禁止）" },
      "expires": { "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$",
        "description": "UTC 暦日基準。today < expires のときのみ有効" }
    }, "additionalProperties": false } }
}
```
`policy_proposal.schema.json`（`fda.policy_proposal.v1`）は `min_occurrences` / `scanned_runs` / `candidate_count` / `candidates[]{proposed_rule_id, decision_type, answer, occurrences, match_summary_keywords, enacted_from, summary_signature}` を required。

**不変条件（最重要。人間権限の委任を扱う）**:
- **契約制定は人間の YAML 手編集のみ**。`.fda/delegation_contract.yaml` に人間が自ら書く行為が「制定」。**AI はこのファイルを作成・編集してはならない**。`fda policy propose` は候補を `artifacts/runs/_policy/` に**提案するだけで `.fda` へは絶対に書かない**。
- **AI は propose と example まで**。live の `.fda` に例を置くな（pr_reviewer HIGH1/HIGH2）。このフォークは初版で `.fda/delegation_contract.yaml`（実例 DC-001）を同梱して fail し、repair `83cbe7e` で live 契約を**完全撤去**、`docs/standards/fda-v1/examples/delegation_contract.example.yaml`（効力なしの書式例）へ移した。移植時は最初から live に置かないこと。
- **keyword は AND 一致**: `match_summary_keywords` は ANY ではなく **ALL（全キーワードが summary に含まれるときのみ適用）**。初版は OR（ANY）で「包括委任化」リスクを指摘され、AND 化 + schema に `minLength: 4`（`fda start` 定型文言でなく判断特定語）を追加。
- **expires は当日から無効（UTC）**: `today < expires` の UTC 暦日基準（expires 当日はもう無効）。初版は境界が緩く、strict 化。比較は `src/support/date.rs` の純関数で（application 層 `SystemTime` 禁止、clock 注入）。
- **適用は明示 `--by-contract` 時のみ + fail-closed**: (a) rule 存在 (b) `decision_type` 一致 (c) keyword AND 一致 (d) 未失効 の**全条件**成立時だけ契約 answer で記録。1 つでも欠ければ**人間判断（`fda decide --answer`）へ差し戻す**。`decided_by = delegation_contract:<rule_id>:<authority>`、receipt に `contract_rule_id` / `contract_expires` / `authority` を追記。
- 不正 / expires 無しルールは**該当ルールのみ**拒否し他ルールは有効。YAML 破損は fail-closed でエラー。不存在なら検証スキップ（従来動作を壊さない）。
- `policy_proposal` はランタイム自己検証する（出力直後に自身の schema で validate）。

**CLI 追加**:
- `fda policy propose [--min-occurrences 3] [--out artifacts/runs/_policy]`: 全 run の `decision_receipts` を (type × summary 正規化署名 × answer) でクラスタし、`min-occurrences` 以上を `policy_proposal.{json,md}` に提案。`PolicyProposeConfig { min_occurrences: u64 (default 3) }`。
- `fda decide <ID> --by-contract <rule_id>`: `--answer` と**排他**（両方 or 両方無しはエラー。`args.rs` で検証）。

**配線**: `args.rs`（`parse_policy_propose_args` / decide の `by_contract` + 排他検証）、`decide.rs`（契約適用ロジック）、`status.rs`（「DC-xxx 適用可」ヒント表示、**自動適用しない**）、`validate.rs`（optional profile schema 検証）、`runner.rs` / `lib.rs`。

**テスト観点**（このフォーク: 235 passed）: 3 回以上で候補化 / 2 回で候補化されない / `.fda` 不変（前後ハッシュ比較）、有効契約 + `--by-contract` で契約 answer + `decided_by` に契約 ID + authority、期限切れ・type 不一致・keyword 一部不一致（AND 拒否）で差し戻し、`--answer` 併用エラー、schema 違反で validation failure・不存在で従来動作。date 純関数 3、args 6。

---

### F2 — Epic 継続ループ / `fda continue --epic`（PR-V15-004, feat `535ea49` + repair `50d05ee`）

**目的**: epic run dir の `planned_prs.json` と全 run の receipt を `planned_pr_id` で突合し、planned PR ごとの進捗を判定して次に進める PR を選ぶ。

**新規 / 変更ファイル**:
- 新規: `src/application/epic.rs`、`docs/standards/delivery-artifacts-v0/schemas/epic_progress_state.schema.json`、`docs/standards/delivery-artifacts-v0/schemas/next_planned_pr_decision.schema.json`。
- 変更: `src/cli/args.rs`（continue に `--epic` / `--artifacts-root`）、`output.rs` / `runner.rs` / `lib.rs`、`mod.rs`、`docs/standards/delivery-artifacts-v0/artifact_catalog.md`（2 行）、`examples/forge_dashboard_epic/`（example 2 件）。

**契約（`epic_progress_state.schema.json` / `fda.epic_progress_state.v1`）**:
```json
{
  "required": ["schema_version","advisory","epic_id","generated_at_unix","prs","summary","scan_notes","scan_errors"],
  "properties": {
    "advisory": { "const": "この判定は非権威の提案であり、実装開始許可・merge 承認・merge の証明ではない" },
    "prs": { "items": { "required": ["planned_pr_id","sequence","title","status","evidence","reasons"],
      "properties": { "status": { "enum": ["not_started","in_progress","pr_open",
        "human_approval_required","merge_ready","merged","blocked"] } } } },
    "summary": { "required": ["merged","open","blocked","waiting_human","not_started"] }
  }
}
```
`next_planned_pr_decision.schema.json`（`fda.next_planned_pr_decision.v1`）: `verdict` enum `[proceed, waiting_human, blocked, complete]`。`allOf` で `verdict=complete → next_planned_pr_id=null` / `verdict=proceed → next_planned_pr_id` は string を強制。同じ `advisory` const を required。

**不変条件**:
- **read-only projection**: 既存 receipt を書き換えず、epic run dir へ `epic_progress_state.json` / `next_planned_pr_decision.json` の**2 ファイルだけ**出力。**auto merge / 自動実装開始はしない**。schema に `advisory` const を必須化し「**merge の証明ではない**」ことを契約に埋める（自動化はこの artifact を merge 判定に使ってはならない。可否は `fda merge` の gate が fail-closed で判定）。
- **evidence は epic_id 突合**: receipt の `epic_id` を `planned_prs.json` の `epic_id` と突合し、**一致する receipt のみ**状態根拠に採用。不一致・欠落は `scan_notes` に記録して除外。初版は epic_id 突合なしの OR ロジックで偽 merged / 別 epic 混入リスクを指摘され repair `50d05ee` で修正。
- **矛盾は blocked**: 同一 PR に merged と open の矛盾 evidence があれば `conflicting_evidence` として fail-closed で blocked（`PrEvidence::conflicting`）。
- **承認待ちと実行待ちを分離**: `human_approval_required`（merge gate 到達済み・approval 未記録＝`fda decide` 待ち）と `merge_ready`（承認済み・`fda merge --execute` 待ち）を別 status に。resume は前者に `fda decide <id> --answer approve`、後者に merge 実行を提示。
- **依存充足**: sequence 順で**最初の未 merged PR だけ**を見る（前 PR が全て merged でない限り後続を選ばない）。未解決 Human Decision があれば `waiting_human` + `fda decide` の resume。
- **fail-soft**: parse error の receipt は `scan_errors` に記録して走査継続、verdict には影響させない（安全性は実 merge gate 側で担保）。
- **既存 `fda continue`（repair gate）は不変**。`--epic` 指定時のみ epic.rs へ分岐。

**CLI 追加**: `fda continue --epic [--artifacts-root artifacts/runs]`（`--artifacts-root` は epic path でのみ使用、repair path では未使用）。

**配線**: `args.rs`（`parse_continue_args` に `--epic` / `--artifacts-root`）、`runner.rs`（`--epic` 判定で epic.rs へ分岐）、`lib.rs`、`output.rs`。

**テスト観点**（epic 7 + args 2、repair 後 247 passed）: PR ごと status 正判定、seq1,2 merged → next=seq3、未解決判断 → waiting_human + resume、seq2 が pr_open で seq3 を選ばない、全 merged → complete、epic_id 不一致除外、矛盾 → blocked、承認待ち resume が decision_id を指す、`--epic` なしで repair 動作不変。

---

### F3 — 道場 UI（PR-V15-005, feat `9002395` + repair `204de96` + polish `320cdeb`）

**目的**: 人間が過去の判断とその後の帰結（merge / repair / qa の状態）に向き合い、判断力が育つ **read-only projection** を Mission Control（`fda ui`）に足す。

**新規 / 変更ファイル**:
- `src/application/ui.rs`: snapshot 拡張 + 収集ロジック + テスト（計 13）。
- `src/rendering/mission_control.rs`: 道場 / 庭師 / Epic セクション + precedent / 契約バッジ + テスト（計 10）。
- `src/application/policy.rs`: `normalize_summary_signature` を `pub(crate)` 化（precedent 署名照合で再利用）。
- `docs/v1/mission_control_uiux.md`: §3 に Epic 進捗・道場・庭師と outcome バッジ意味論（`human_approval=琥珀` 含む）を追記。
- `scripts/check_architecture_boundaries.py`: ui.rs の infra 追加分を allowlist。

**契約**: schema `fda.mission_control_snapshot.v0` は**据置＝追加フィールドのみ**（`/api/state.json` に自動で乗る）。追加: `decision_journal`（回答済み判断 + 同 run の帰結、新しい順・上限 50）、`decision_inbox[].precedent`（同 type + 正規化署名類似の過去判断 最大 3 件）+ `applicable_contract`、`gc_docket`、`epic_progress`。

**不変条件**:
- **read-only projection**: UI からの状態変更なし・書き込みエンドポイント無し・毎リクエストで artifact 再読込。`127.0.0.1` bind のみ。
- **実 schema のキーのみ参照（fixture も実キーで書く）**: 初版は run 帰結判定が `merge_receipt` の実在しない架空キーを参照し、fixture もその架空キーを通していたため `blocked` / `human_approval_required` を誤判定した（pr HIGH＝偽 green）。repair `204de96` で `merge_receipt` の**実 schema（`status` キーのみ）**に修正し fixture も実在値化。移植時は fixture を実 schema から作れ。
- **帰結は run 単位（因果を主張しない）**: 帰結（merge/qa/repair）は run 単位の投影であり、run 内の個々の判断固有の結果ではない。道場の列は「その後（run の帰結）」と表記し「run 内 N 判断で共通」注記を付す。設計正本にも因果非主張を明記。
- **precedent の透明性 + バッジ非混同**: 一致理由（完全一致/接頭辞一致）を表示。契約適用は authority + 塗りバッジで示し、生の `decided_by` 文字列（`delegation_contract:DC-001:...`）を UI に出さない。inbox の契約ヒントは outline バッジ + 「（提案・自動適用なし）」を常時表示。
- **advisory を投影**: epic_progress の非権威文言を Epic セクション冒頭に明文表示。
- **repair 帰結を視覚分離**: repair は専用色 `--outcome-repair`（紫・ライト/ダーク両対応）で、琥珀（`human_approval` = 今の人間待ち）と分離。`outcome_badge()` に `"human_approval" => "badge-human"` の match arm（polish `320cdeb`）。良い判断も痛い判断（repair）も同列に可視化。
- **契約 YAML 破損時は fail-soft**: 契約ヒント無しへ degrade（クラッシュしない）。
- **HTML エスケープ必須 + 純関数維持**（`rendering/mission_control.rs`）。

**セクション順**: サマリ → Decision Inbox → AI Repair Lane → Epic 進捗 → Runs → 道場 → 庭師 → フッタ。Runs は完了・その他を折りたたみ（既定閉）、アクティブ run のみ常時表示。

**テスト観点**（ui 13 + rendering 10、polish 後 265 passed）: 回答済み判断 + 帰結で道場に時系列表示、未解決 + 同 type で precedent 最大 3 件、gc docket と epic progress 双方表示、`outcome_badge` の human_approval → badge-human、実 schema キー参照。

---

## 3. アーキテクチャ境界の注意（`scripts/check_architecture_boundaries.py`）

application 層は infra を直呼びできない。V1.5 で追加した import は allowlist（`ALLOWED_APPLICATION_INFRA_USES`）に**明示追記が必要**。移植時に足す分:

| ファイル | 追記する import |
|---|---|
| `src/application/status.rs` | `use crate::infra::yaml::SerdeYamlValidator;` |
| `src/application/decide.rs` | `use crate::infra::yaml::SerdeYamlValidator;` |
| `src/application/gc.rs` | `system_unix_seconds` / `{list_dir_names, modified_unix_seconds, FsArtifactStore}` |
| `src/application/policy.rs` | `system_unix_seconds` / `{list_dir_names, FsArtifactStore}` / `JsonSchemaArtifactValidator` |
| `src/application/epic.rs` | `system_unix_seconds` / `{list_dir_names, FsArtifactStore}` |
| `src/application/risk_tier.rs` | `SerdeYamlValidator` / `system_unix_seconds` / `FsArtifactStore` |
| `src/application/ui.rs` | `use crate::infra::yaml::SerdeYamlValidator;` |

**原則**: 時刻は `infra::clock`、mtime は `infra::fs_store::modified_unix_seconds`、fs 走査は `list_dir_names` / `list_file_names`、YAML 検証は `infra::yaml`、schema 検証は `infra::json_schema` 経由。application で `SystemTime` / `std::fs` を直呼びしないこと。gate 追記を忘れると F4 の governance hard guard 対象でもあり blocking。

---

## 4. 移植時の既知の罠（本 Epic の repair 6 サイクルから）

1. **fixture を実 schema から作る**（F3）: 判定ロジックが receipt の**実在キー**を参照しているか確認し、fixture も実キーで書く。架空キーの fixture は「テスト green だが本番で誤判定（偽 green）」を生む。実 schema ファイルが repo に無い receipt（例 `github_merge_receipt`）は**生成側 `rendering/merge.rs` と消費側 `epic.rs` の実装を正典**に field set を決める。
2. **単一正本化での安全文言ロスト**（F6）: プロトコルを 1 ファイルに畳むとき、禁止事項（auto merge 全面禁止・scope approval 禁止・自己承認禁止）を**縮約して抜け道を作らない**。畳み込み = 削除ではなく参照への置換。
3. **stacked PR の base 削除で子 PR が CLOSED**（F4）: PR #1 merge 時に base ブランチ（`pr-v15-001`）が削除され、それを base にしていた PR #2 が GitHub 上で自動 CLOSED になった。stacked PR を使うなら、親 merge 後は子 PR の base を `main` へ張り替えて**再作成**する（このフォークは #2 → #3 で再作成）。または最初から base=main + rebase 運用にする。
4. **fda 外 merge は receipt 追記が必要**（F2）: GitHub UI / `gh pr merge` で merge すると `fda merge` が生成する `github_merge_receipt.json` が run dir に残らず、`fda continue --epic` が当該 PR を `not_started` と誤判定する。merge 後に `github_merge_receipt.json` 相当（`epic_id` / `planned_pr_id` / `status=succeeded` / `merge_executed=true` / `merge_sha` / `expected_head_sha` / `rollback_plan`）を専用 run dir に**事後追記**して投影を最新化する（work_protocol §3 手順 8。既存 run は書き換えない）。
5. **CRLF（autocrlf）と文字列完全一致比較**（Windows）: gate checker（`check_review_agent_gate.py` の `RISK_TIER: low` 行や packet 行、advisory const 等）は**文字列/正規表現の完全一致**で判定する。Windows の `core.autocrlf` で CRLF が混ざると一致に失敗し得る。schema の `const`（advisory 文言など）も 1 文字違いで validate 落ち。改行・全角/半角・ダッシュ種別（`—`）まで正本と一致させる。
6. **Windows の `.cmd` shim spawn**（環境）: npm 版 CLI（codex 等）は `.cmd` shim で、Windows で直接 spawn すると失敗し得る。FDA は spawn 時に自動解決するが、移植先で外部プロセスを呼ぶなら Windows の shim 解決を確認（work_protocol §7）。

---

## 5. 移植後の検証チェックリスト

- [ ] `cargo fmt --all -- --check`: 差分なし。
- [ ] `cargo test`: 全 green（このフォーク最終は 269 passed。移植先の既存件数に上乗せ）。
- [ ] `python scripts/check_architecture_boundaries.py`: passed（§3 の allowlist 追記込み）。
- [ ] `cargo run -- validate-artifacts`: 新 schema 5 件（risk_tier / gc_docket / policy_proposal / epic_progress_state / next_planned_pr_decision）+ delegation_contract + **各 example が passed**。const（advisory / schema_version）まで一致していること。
- [ ] `python -m unittest discover -s tests`: OK。特に `tests/test_review_agent_gate.py` の forge not_applicable **受理/拒否 2 テスト**が pass。
- [ ] `check_review_agent_gate.py`: forge_reviewer `not_applicable` が `RISK_TIER: low` 行 + 理由なしでは**拒否**、ありでのみ**許容**。
- [ ] governance hard guard: `.fda/**` / gate checker / merge・review・risk_tier・policy 実装を含む changed_files で forge 緩和が**却下**される（unit test で固定）。
- [ ] `fda continue --epic` の complete 実証: 全 planned PR に `github_merge_receipt.json`（status=succeeded / merge_executed=true）を揃えて `fda continue --epic --artifacts <epic run dir> --json` を実行し、`verdict=complete` かつ summary が `merged=<全数>, open/blocked/waiting_human/not_started=0`、`scan_errors`/`scan_notes` 空、を確認。`fda ui --json` に epic_progress として反映されることも確認。

---

## 6. 移植しなくてよいもの（この fork 固有）

以下はこのフォークの運用証跡であり、**upstream へ移植しない**:

- `artifacts/runs/fda-start-1783555082/`（この Epic の run artifacts。設計・QA receipt 等）
- `artifacts/review_packets/pr-{1,3,4,5,6}.md`（このフォークの review packet。契約学習の一次資料としては読むが移植物ではない）
- `artifacts/reports/`（このフォークのレポート）
- `artifacts/runs/v15-merge-receipts-pr-v15-00{1..5}/`（このフォークの事後 merge receipt。GH PR 番号・SHA はこの repo 固有）
- `docs/v1/fda_v1_5_intake.md` 等の intake docs（このフォームの依頼記録）

**移植するのは**: `src/` の実装（risk_tier / gc / policy / epic / ui / merge / decide / status の V1.5 差分）、`docs/standards/.../schemas/` の新 schema 5 件 + delegation_contract schema + example、`scripts/check_review_agent_gate.py` / `check_architecture_boundaries.py` の拡張、`docs/v1/work_protocol.md`（正本、移植先語彙に合わせて調整可）、`tests/` の追加テスト。

---

## 付録: 主要一次情報の場所（移植先で判断に迷ったら実物を読む）

- 設計: `artifacts/runs/fda-start-1783555082/detailed_design.md`, `planned_prs.json`
- 実装本体: `src/application/{risk_tier,gc,policy,epic,ui,merge,decide,status,decisions,validate,profile}.rs`, `src/rendering/mission_control.rs`, `src/rendering/merge.rs`, `src/support/date.rs`, `src/infra/fs_store.rs`
- 配線: `src/cli/{args,output,runner}.rs`, `src/lib.rs`, `src/application/mod.rs`
- 契約: `docs/standards/delivery-artifacts-v0/schemas/{risk_tier,gc_docket,policy_proposal,epic_progress_state,next_planned_pr_decision}.schema.json`, `docs/standards/fda-v1/schemas/repository-profile/delegation_contract_yaml.schema.json`, `docs/standards/fda-v1/examples/delegation_contract.example.yaml`
- ゲート: `scripts/check_review_agent_gate.py`, `scripts/check_architecture_boundaries.py`, `tests/test_review_agent_gate.py`
- レビュー確定事項（repair 履歴）: `artifacts/review_packets/pr-{1,3,4,5,6}.md`
- feat / repair コミット: F6 `43a3dc0`(+`39612bc`/`1b1d209`/`f589691`) / F4+F5 `4a4aacb`(+`73dbbfe`/`ecc8781`) / F1 `8de4e0e`(+`83cbe7e`) / F2 `535ea49`(+`50d05ee`) / F3 `9002395`(+`204de96`/`320cdeb`)
