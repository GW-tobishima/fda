---
standard_id: delivery-artifacts-v0.uiux-mission-control
version: v0
status: draft
last_reviewed: 2026-06-20
review_cycle_days: 30
owner: forge-delivery-agent
---

# UI/UX Mission Control 設計 v0

## 方針

UI 実装は後回しにする。ただし UI/UX 設計は Phase 0 から進め、ATO と Forge の状態モデルに合う情報設計を先に固定する。

画面はチャットではなく Mission Control として設計する。主目的は AI の操作ではなく、Program / Epic / Case / PR / Human Decision / AI Repair / Evidence の観測と判断である。

## 主要原則

- Human Decision と AI Repair を混ぜない。
- 現在の詰まりが「人間判断待ち」か「AI 修復待ち」かを一目で分ける。
- AI の自律実行範囲は Autonomy Contract として常に見える。
- Evidence は隠さず、各 Gate の根拠として辿れる。
- UI は高密度で一覧性を優先する。
- 装飾的なヒーローやカード過多の画面にしない。

## 画面骨格

```text
Program Header
  Program ID / State / Human Decisions / AI Repair / Merge Ready

Left Pane
  Program Tree
  Lanes
  Filters

Main Surface
  Epic Delivery Plan
  Claim Tree
  Case Graph
  PR Plan
  Proof Coverage
  Promotion State

Right Pane
  Decision Inbox
  AI Repair Queue
  Evidence Links
  Run Summary
```

## レーン

| lane | 表示対象 | 主な操作 |
|---|---|---|
| Planning | 要件読込、Epic Plan、Autonomy Contract | review, approve, request change |
| Ready To Work | 合意済み Case / Task | start, assign |
| Running | 実装、QA、repair loop | observe, pause |
| AI Repair | 証跡不足、test 未実行、trace gap | rerun, repair |
| Judgment Required | scope/security/release 判断 | decide, request context |
| Merge Ready | PromotionDecision promote | merge review |
| Release Ready | ReleasePromotionDecision ready | release approval |

## 状態表示

| 状態 | 意味 | 人間の関与 |
|---|---|---|
| `draft` | 計画中 | 任意 |
| `ready` | AI が進めてよい | 不要 |
| `running` | 実行中 | 不要 |
| `ai_repair` | AI が修復すべき | 不要 |
| `human_turn` | 人間判断待ち | 必須 |
| `merge_ready` | merge 判断可能 | 必要に応じる |
| `blocked` | policy または証跡不足で停止 | 必須または repair |

## Human Decision UX

Decision Box には次だけを表示する。

- 判断が必要な理由。
- 選択肢。
- 推奨案と根拠。
- 影響範囲。
- 期限または required before。
- 決定しない場合の扱い。

テスト失敗や証跡不足は Decision Box に出さず、AI Repair Queue に出す。

## Accessibility

- Status は色だけで表現しない。
- キーボードで Decision と Repair を巡回できる。
- Gate、Decision、Evidence には見出し階層を持たせる。
- WCAG 2.2 AA を v0 の基準にする。

## v0 で作らないもの

- React / Tauri / Web 実装。
- アニメーション。
- ダッシュボードの完成デザイン。
- テーマ設計の詳細。

## v0 で作るもの

- 画面骨格。
- レーン定義。
- 状態定義。
- Human Decision と AI Repair の分離方針。
- Design Agreement テンプレート。
