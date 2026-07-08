---
artifact_type: requirements_definition
version: v0
status: reviewed
---

# AICX AI ストラテジスト学習 Bot 要件定義

## 0. Metadata

- Document ID: REQ-AICX-STUDY-BOT-001
- Version: v0
- Owner: forge-delivery-agent
- Status: reviewed
- Source: Human Input Spec HIS-AICX-STUDY-BOT-001
- Related Program: PROGRAM-AICX-STUDY-BOT-001
- Related Epic: EPIC-AICX-STUDY-BOT-POC-001

## 1. Business Objective

- Problem: AI ストラテジスト資格の学習で、週ごとの必要ページ数と復習範囲を消化しきれていない。
- Desired Outcome: 毎朝の実践問題、回答後の採点、弱点ページ推薦により、学習を日次習慣化し、試験日判断に使える正答率を可視化する。
- Success Metrics:
  - 毎朝 4:30 Asia/Tokyo に当日の学習範囲から問題セットが生成される。
  - 1 回あたり約 30 問の問題が Slack に送信される。
  - 回答後に総合正答率、領域別正答率、読むべき PDF ページが返る。
  - 2026-07-05 時点の第 2 回模試で 90% 以上なら 2026-07-10 受験判断に使える。
- Non-goals:
  - AICX 公式教材や PDF 本文を repo に保存しない。
  - 試験の合格保証や公式問題の再配布はしない。
  - PoC-0 では本番 Bot 実装、課金、マルチユーザー化、UI 実装をしない。

## 2. Scope

### Scope In

- 入力メモから学習スケジュールとページ範囲を正規化する。
- PDF ページ範囲を問題生成単位に紐づける設計を作る。
- 毎朝 4:30 の問題生成・通知・回答取り込み・採点・ページ推薦の Epic Delivery Plan を作る。
- Slack-first の環境 readiness と LINE-first を後続化する判断を明示する。
- ATO Task Graph に実装単位を分解する。

### Scope Out

- 本番 Slack App / LINE Messaging API の作成。
- PDF OCR や全文抽出の実行。
- Codex CLI を使った実問題生成の本番運用。
- 公式教材の内容検証。
- GitHub PR 自動作成、merge、release、deploy。

## 3. Stakeholders / Users

- Primary users: AI ストラテジスト資格を受験予定のユーザー
- Operators: ローカル Codex CLI 実行者
- Reviewers: 学習 Bot 仕様レビュー担当、セキュリティレビュー担当
- Approvers: ユーザー

## 4. Functional Requirements

| ID | Requirement | Rationale | Priority | Acceptance Criteria | Source |
|---|---|---|---|---|---|
| FR-001 | 入力メモの学習計画を日付、ページ範囲、トピック、テスト条件に正規化できる | scheduler と問題生成範囲を決めるため | Must | AC-001: 2026-06-17 から 2026-07-10 までの範囲が機械可読な plan に落ちる | human input |
| FR-002 | 毎朝 4:30 Asia/Tokyo に当日の対象範囲から約 30 問を生成できる | 学習習慣化の中心機能 | Must | AC-002: dry-run で指定日の問題セット ID、問題数、出典ページ範囲が出る | human input |
| FR-003 | 問題は実践判断を含み、業務設計、RAG、法務、KPI、ワークフロー分岐などの領域タグを持つ | 弱点ページ推薦に必要 | Must | AC-003: 各問題に topic、source_pages、expected_answer、rationale が付く | human input |
| FR-004 | Slack に問題セットを送信でき、将来の LINE adapter と同じ境界を持つ | 通知チャネルを後から拡張できるようにする | Must | AC-004: mock adapter と Slack-first adapter の契約が同じ入出力を持つ | human decision |
| FR-005 | A-D 選択式のユーザー回答を取り込み、採点して正答率を返せる | 初期 PoC の採点を確実にするため | Must | AC-005: fixture の A-D 回答に対して総合正答率と問題別正誤が出る | human decision |
| FR-006 | 不正解または低確信の問題から PDF の推奨復習ページを返せる | ページ消化を支援するため | Must | AC-006: 誤答 topic と source_pages から優先ページリストが出る | human input |
| FR-007 | 日次 run、問題セット、回答、採点結果をローカルに保存できる | 継続学習と再採点に必要 | Should | AC-007: run log に日付、question_set_id、delivery channel、score が残る | derived |
| FR-008 | 第 1 回模試と第 2 回模試のような週末テストを日次問題と別モードで扱える | 入力スケジュールに合わせるため | Should | AC-008: 40-60 問の模試モードと弱点ページ特定が plan に含まれる | human input |

## 5. Non-Functional Requirements

| ID | Quality Attribute | Requirement | Measure | Target | Verification |
|---|---|---|---|---|---|
| NFR-SEC-001 | Security | Slack/LINE token と PDF は repo に保存しない | secret file scan | token/PDF path が git diff に出ない | git diff review |
| NFR-PRIV-001 | Privacy | 回答履歴と教材由来情報はローカル保存を基本にする | storage location | data directory は local-only | design review |
| NFR-COPY-001 | Copyright | PDF 本文の長文抜粋を通知やログに残さない | excerpt length | 問題文と短い根拠のみ、出典はページ番号中心 | content review |
| NFR-REL-001 | Reliability | 4:30 の日次送信を重複させない | duplicate run count | 同一日・同一チャネル 1 回 | scheduler dry-run |
| NFR-OBS-001 | Observability | 生成、送信、回答取り込み、採点を trace できる | run log fields | 各 phase の verdict と artifact path を記録 | fixture run review |
| NFR-OPS-001 | Operability | ローカル Codex CLI 前提で手動 dry-run できる | command availability | 1 コマンドで当日分を preview できる | manual smoke |

## 6. Constraints

- Technical: Codex CLI はローカルで実行される。scheduler は systemd timer または cron を想定する。
- Legal / compliance: 教材 PDF の利用権限、外部送信可否、本文抜粋量を確認する。
- Operational: 2026-06-21 時点では 2026-06-17 から 2026-06-23 の学習週の途中である。
- Schedule: 2026-07-05 の第 2 回模試までに採点と弱点ページ推薦を使える状態が望ましい。
- Dependencies: local-only PDF source、Slack Bot token、Slack channel ID、ローカル storage、Codex CLI 実行環境。

## 7. Assumptions

- 対象 PDF は `data/study_bot/source/pdf/aicx_ai_agent_strategist_official_text.pdf` に local-only copy として存在する。
- ページ番号と抽出テキストの対応が取れるか、少なくともページ範囲ごとの手動 topic map を作れる。
- Slack-first を初期 PoC とし、完全ローカル polling で返信取り込みまで PoC 化する。
- LINE-first は webhook 受信のために public HTTPS endpoint または tunnel が必要になるため、後続 adapter 拡張に回す。
- 1 日 30 問は Slack の message length 制限に合わせて分割送信してよい。

## 8. Open Questions

| ID | Question | Owner | Blocking? | Due |
|---|---|---|---|---|
| Q-ASB-001 | 決定済み: 初期チャネルは Slack-first | human | no | 実装開始前 |
| Q-ASB-002 | 決定済み: PDF は local-only copy `data/study_bot/source/pdf/aicx_ai_agent_strategist_official_text.pdf` を参照する。抽出方式は実装 task で検証する | human | no | ページ索引作成前 |
| Q-ASB-003 | 決定済み: 回答形式は A-D 選択式 | human | no | 採点器実装前 |
| Q-ASB-004 | 決定済み: 日次通知は毎日 4:30 Asia/Tokyo | human | no | scheduler 有効化前 |
| Q-ASB-005 | 問題生成に外部モデル API を併用してよいか | human | no | model adapter 拡張前 |

## 9. Human Decision Points

| ID | Trigger | Decision Needed | Options | Required Before |
|---|---|---|---|---|
| HDP-ASB-001 | 通知・返信 adapter の実装前 | 決定済み: Slack-first | Slack-first, LINE-first, adapter 抽象のみ | CASE-ASB-004 |
| HDP-ASB-002 | PDF ingest の実装前 | 決定済み: local-only PDF copy を使う | ローカル PDF, OCR 済みテキスト, 手動 page map | CASE-ASB-002 |
| HDP-ASB-003 | 採点器の実装前 | 決定済み: A-D 選択式 | A-D 選択式, 短答式, 混合式 | CASE-ASB-005 |
| HDP-ASB-004 | scheduler 有効化前 | 決定済み: 毎日 4:30 Asia/Tokyo | 毎日 4:30, 平日 4:30, 失敗時のみ再送 | CASE-ASB-003 |

## 10. Forge Mapping

- Claim IDs: CLM-ASB-001, CLM-ASB-002, CLM-ASB-003, CLM-ASB-004, CLM-ASB-005
- Proof Obligations: PRF-ASB-001, PRF-ASB-002, PRF-ASB-003, PRF-ASB-004, PRF-ASB-005
- Human Decision Points: HDP-ASB-001, HDP-ASB-002, HDP-ASB-003, HDP-ASB-004
- ATO Task Graph: TASK-ASB-001, TASK-ASB-002, TASK-ASB-003, TASK-ASB-004, TASK-ASB-005, TASK-ASB-006
- Planned PRs: PR-ASB-001, PR-ASB-002, PR-ASB-003, PR-ASB-004, PR-ASB-005, PR-ASB-006
- Gate Requirements: Contract QA, Schema Validation, Dry-run Evidence, Human Decision Separation, Secret/PDF Safety Review
