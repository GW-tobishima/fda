# システムプロンプト書き換えガイド

作成日: 2026-07-09
対象: `~/.claude/CLAUDE.md`（Claude Code グローバル）/ `~/.codex/AGENTS.md`（Codex CLI グローバル）
目的: **FDA + ATO を既定の運用にする**システムプロンプトを、正しい場所に・正しい強度で・
陳腐化しない構造で書くための正典。どこが必須でどこが任意かを定義する。

---

## 1. このガイドの位置づけ

システムプロンプト（グローバル指示ファイル）は、AI CLI の**全セッションに無条件で注入される
唯一のレイヤ**である。Skill は呼ばれたときだけ読まれ、hook は特定イベントでしか動かない。
したがってグローバル指示には「毎回必ず効いてほしい最小の契約」だけを置き、
手順の詳細は Skill / runbook に委譲する。

このガイドは次の 3 つを定義する。

1. **どのファイルが実際に読まれるか**（間違えると指示が一切効かない）
2. **理想的なセクション構成と、各セクションの強度（MUST / SHOULD / OPTIONAL）**
3. **書いてはいけないこと**（アンチパターン）

関連ドキュメント:

- 運用正本: [`claude_code_primary_runbook.md`](./claude_code_primary_runbook.md)
- FDA V1 アーキテクチャ: [`codex_cli_primary_architecture.md`](./codex_cli_primary_architecture.md)
- 非実装モード: [`non_implementation_modes.md`](./non_implementation_modes.md)

---

## 2. どのファイルが実際に読まれるか（事実）

> **ここを間違えると、書いた指示は 1 行も効かない。**
> 実際に 2026-07-09 の調査で、`~/.codex/instructions.md` に書かれていた ATO 運用ルール 58 行が
> **codex-cli 0.139 に一度も読まれていなかった**ことが判明した（§2.4）。

### 2.1 Claude Code

| 階層 | パス | ロード条件 |
|---|---|---|
| ユーザーグローバル | `~/.claude/CLAUDE.md` | **全セッション常時** |
| プロジェクト | `<repo>/CLAUDE.md` | そのプロジェクトで起動したセッション常時（git 追跡対象） |
| path-scoped rules | `<repo>/.claude/rules/*.md` | frontmatter の `paths:`（glob 配列）にマッチしたときのみ。`paths:` 無しは常時ロード（CLAUDE.md 相当） |
| Skill | `~/.claude/skills/<name>/SKILL.md`、`<repo>/.claude/skills/<name>/SKILL.md` | Skill ツールで明示的に呼ばれたときのみ |
| hooks / 権限 | `~/.claude/settings.json`、`<repo>/.claude/settings.local.json` | 起動時・イベント時 |

- **グローバルとプロジェクトは加算される**（プロジェクト側がグローバルを消さない）。
- 自動で毎回効かせたい指示は `CLAUDE.md` に、特定パス編集時だけ効かせたい指示は
  `.claude/rules/*.md` に `paths:` 付きで置く。
- `settings.local.json` の権限は **AI が直接書き換えられない**（自己権限変更ガード）。
  変更案はファイルに出してユーザーに手動反映してもらう。

### 2.2 Codex CLI（codex-cli 0.139 で確認）

| 階層 | パス | ロード条件 |
|---|---|---|
| ユーザーグローバル | `$CODEX_HOME/AGENTS.md`（既定 `~/.codex/AGENTS.md`） | **全セッション常時** |
| プロジェクト | `<repo>/AGENTS.md`、およびサブディレクトリの `AGENTS.md` | 変更するファイルの scope を覆う `AGENTS.md` すべてに従う（階層マージ） |
| override | `AGENTS.override.md` | バイナリ内に文字列が存在。挙動は**未検証**（§2.4 の方法で確認すること） |
| 設定 | `~/.codex/config.toml` | 起動時 |

`config.toml` の関連キー（バイナリ内に存在を確認）:

| キー | 用途 |
|---|---|
| `project_doc_max_bytes` | プロジェクト doc の読み込み上限バイト数（既定 32KB 前後）。**グローバル AGENTS.md に適用されるかは未検証** |
| `project_doc_fallback_filenames` | `AGENTS.md` 以外に読むファイル名のフォールバック配列 |

### 2.3 対応表

| 概念 | Claude Code | Codex CLI |
|---|---|---|
| グローバル指示 | `~/.claude/CLAUDE.md` | `~/.codex/AGENTS.md` |
| プロジェクト指示 | `<repo>/CLAUDE.md` | `<repo>/AGENTS.md` |
| セッション開始フック | `settings.json` の SessionStart hook | **無い**（指示本文に Session Start Check を書いて代替する） |
| Skill 機構 | あり（Skill ツール） | 限定的。指示本文を正本として自足させる |

**両者は同じ運用契約を持たせる。** 片方だけ更新して乖離させない。

### 2.4 検証方法（書き換えたら必ず行う）

書いた内容が本当に読まれているかは、**推測せず実測する**。

```powershell
# Claude Code: 新しいセッションで、グローバル指示にしか書いていない固有語を尋ねる
claude -p "あなたのグローバル指示で、FDA を使う適用トリガーは何と定義されていますか"

# Codex CLI: 同上
codex exec "あなたのグローバル指示で、FDA を使う適用トリガーは何と定義されていますか"
```

- 期待: 「git repo 内でコード変更を伴い PR に至る作業」と答える。
- 答えられない場合、そのファイルは読まれていない。パスと階層（§2.1 / §2.2）を疑う。
- 固有語は「そのファイルにしか書いていない語」を選ぶ。一般語だと訓練知識で答えられてしまい検証にならない。

**過去の事故（教訓）**

| 事象 | 原因 | 対処 |
|---|---|---|
| `~/.codex/instructions.md` の ATO ルール 58 行が効いていなかった | codex-cli 0.139 はグローバル指示に `AGENTS.md` を使う。`instructions.md` は extension folder 用の別文脈でしか参照されない | `~/.codex/AGENTS.md` を正本化し、旧ファイルは `.bak-<日付>` へ退避 |

読まれないファイルを放置すると、「書いたのに効かない」を延々と再発させる。**退避か削除まで行う。**

---

## 3. 理想的な構成

グローバル指示は次の順序で書く。**順序に意味がある**（上ほど毎回参照され、下ほど参照頻度が低い）。

| # | セクション | 強度 | なぜここに置くか |
|---|---|---|---|
| 凡例 | 強度の定義表 | **MUST** | MUST/SHOULD/OPTIONAL の意味を先に固定しないと、以降の全表現が曖昧になる |
| 0 | 必須コア | **MUST** | 「毎回必ず効くべき最小契約」を 6 行程度に圧縮。ここだけ読めば破滅的なミスは避けられる状態にする |
| 1 | ATO Control Plane | **MUST** | 状態・証跡・知識の正本。全作業に適用される |
| 2 | FDA Delivery Agent | **MUST（トリガー時）** | コード変更→PR のときだけ発火する条件付き必須。トリガー / 非トリガーを最初に書く |
| 3 | レポート出力と Notion projection | **MUST（ローカル）/ SHOULD（Notion）** | レビュー・調査の成果物契約。ローカル保存は必須、外部連携は fail-soft |
| 4 | Skill Bundle | **MUST** | 詳細手順の委譲先。ここに一覧だけ置き、内容はコピペしない |
| 5 | 環境固有の事実 | **OPTIONAL** | 版数・パス・SHA。**最も陳腐化する情報なので最後**。挙動を縛らない |

### 3.1 なぜ「環境固有」を最後に置くか

版数・パス・SHA256 は**最も頻繁に古くなる情報**であり、かつ**挙動を縛らない**。
これを冒頭に置くと、

- 読み手（AI も人間も）が最初に陳腐化した情報に触れる
- 更新のたびに冒頭が変わり、diff が読みにくくなる
- 「必須の契約」と「ただの現状メモ」が視覚的に同格になる

末尾に隔離すれば、**版数が変わったときそこだけ直せばよい**。

### 3.2 なぜ FDA を「条件付き必須」にするか

FDA には `implement` のほかに `research` / `uiux` / `design-only` モードがあり、
形式上はあらゆる依頼を受けられる。しかし全作業で `fda start` を強制すると、

- 軽い調査でも run ディレクトリ・`human_decision_packet.md`・`artifact_inventory.json` が生成され重い
- 非 git ディレクトリ（共有ドライブ、Excel 作業）では `--repo-root` 解決や `.fda/` 生成が破綻する

したがって**トリガーを「git repo 内でコード変更を伴い PR に至る作業」に絞り**、
それ以外は ATO のみで進める。迷ったら FDA を使わず、コード変更が確定した時点で `fda start` に入る。

---

## 4. 必須（MUST）— 省略したら壊れるもの

以下は**どのマシン・どのプロジェクトでも必ず書く**。省略すると AI の挙動が不定になる。

### 4.1 強度の凡例

MUST / SHOULD / OPTIONAL の定義表を冒頭に置く。
これが無いと「必ず」「原則」「推奨」といった語が読み手ごとに違う重みで解釈される。

### 4.2 応答言語

`日本語で応答する。ドキュメントも日本語で書く。`
1 行でよいが、**必ず 0 番目のセクションに置く**。後半に置くと長い応答で無視されやすい。

### 4.3 ATO を正本と宣言する

- 「作業記録・知識蓄積・検索の正本は ATO」
- 「意味のある作業は `ato work begin` で task/run を開始してから着手する」
- **「意味のある作業」の定義を列挙する**（相談・コード変更・調査・設計・レビュー・PR 対応・
  複数ステップの運用作業・100 文字以上の指示）。定義が無いと AI が恣意的に「不要」と判断する。
- 「迷ったら登録する側に倒す」を明記する。

### 4.4 FDA の適用トリガーと非トリガー

**両方を書く。** トリガーだけ書くと過剰適用され、非トリガーだけ書くと使われない。

- トリガー: git repo 内で、コード変更を伴い PR に至る作業（feature / bugfix / refactor / 削除）
- 非トリガー: 調査・レポート・DB/Excel 作業・相談・非 git ディレクトリ
- 曖昧なときの既定: **FDA を使わない**（ATO のみ）

### 4.5 FDA の fail-closed 契約

これを省くと、AI が自分で merge 承認してしまう。**逐語で書く。**

- Human Decision（scope / privacy / legal / security High・Critical / risk / merge / release）を**自己承認しない**
- 未解決 Human Decision を実装で埋めない
- reviewer / QA subagent（read-only）に source mutation をさせない
- **`REVIEW_AGENT_OK` は merge approval ではない**
- **V1 は auto merge しない**
- `.fda/` の既存ファイルを上書きしない

### 4.6 ATO と FDA の関係（SoT 分離）

`ATO=状態/証跡、Forge=Claim/Proof/Gate、FDA=runtime/skills、GitHub=実コード`

これを 1 行で書く。書かないと「FDA が状態を持つ」と誤解され、二重記帳が始まる。
また **FDA → ATO は片方向**（`--ato-sync --ato-task <key> --ato-run-id <run>`）であることを明記する。

### 4.7 CLI first と fallback

- 「ATO / FDA 操作は CLI を最優先。CLI が使えない場合のみ MCP fallback」
- **`fda` が PATH に無ければ FDA をスキップして ATO のみで進め、その事実を checkpoint に残す**（fail-soft）

fallback の挙動を書かないと、`fda` 不在の環境で AI が停止するか、黙って手順を飛ばす。

### 4.8 着手前検索

`ato search "<キーワード>" --json` を第一手にする。
同じ調査・同じ誤りの繰り返しを防ぐ、費用対効果が最も高い 1 行。

---

## 5. 推奨（SHOULD）— 原則書くが、環境により調整可

| 項目 | 内容 | 調整してよい理由 |
|---|---|---|
| Skill Bundle 一覧表 | 作業種別 → 読む Skill のマッピング | Skill 機構が無い CLI（Codex）では、指示本文を正本として自足させる |
| Notion projection | レポートを Notion「AI」DB に upsert | 外部サービス依存。fail-soft で、失敗しても checkpoint に記録して続行 |
| Forge promotion gate | `ato case evaluate --no-write` で PromotionDecision を試算 | Forge 未導入の ato.exe では実行できない |
| Session Start Check | セッション開始時の task 判断と報告 | Claude Code は SessionStart hook で代替済み。**Codex は hook が無いので指示本文に書く（実質 MUST）** |
| レポート命名規約 | `artifacts/reports/YYYY-MM-DD_<topic>/<topic>.md` | プロジェクト固有の規約があればそちらを優先 |

---

## 6. 任意（OPTIONAL）— 書かなくても壊れない

**挙動を縛らない参照情報**。末尾の「環境固有の事実」セクションにまとめて隔離する。

- `ato.exe` の実体パス・現行版コミット・SHA256・バックアップ名
- ATO の機能追加履歴（どの版で何が入ったか）
- `fda.exe` の実体パス・ソース repo パス
- 環境変数（`ATO_DB_PATH` / `FDA_PYTHON` / `FDA_SLACK_WEBHOOK_URL` 等）
- MCP server の起動・登録コマンド
- Notion の database_id / data source ID

### 6.1 OPTIONAL に落としてよいかの判定

次の 2 問に**両方 YES** なら OPTIONAL:

1. この記述が消えても、AI は誤った行動を取らないか?（＝行動を規定していない）
2. この記述は環境が変われば書き換わるか?（＝普遍的な契約ではない）

例: `ato.exe は C:\Tools\ATO\ato.exe` → 消えても PATH で解決される・マシンごとに違う → **OPTIONAL**
例: `REVIEW_AGENT_OK は merge approval ではない` → 消えると誤って merge する・普遍契約 → **MUST**

---

## 7. アンチパターン

| アンチパターン | なぜ悪いか | 正しくは |
|---|---|---|
| **読まれないファイルに書く** | 指示が 1 行も効かない。しかも「書いたから効いている」と誤認する | §2.4 の方法で実測してから書く |
| 版数・SHA を冒頭に置く | 最も陳腐化する情報が最初に目に入り、必須契約と同格に見える | 末尾の OPTIONAL セクションへ隔離 |
| MUST と OPTIONAL を混在させる | どれが破ってはいけない契約か判別できない。結果、全部が「なんとなく守る」になる | 強度の凡例を定義し、セクション見出しに強度を明記 |
| Skill の中身をコピペする | 二重管理になり、Skill 側の更新が反映されず乖離する | 一覧表とポインタだけ置き、内容は Skill に委譲 |
| 「必ず」を乱発する | すべてが必須なら何も必須でない。実効性が失われる | MUST は「省略したら壊れるもの」だけに絞る（§4） |
| トリガーだけ書いて非トリガーを書かない | FDA が調査や日報にまで発火し、run ディレクトリが散乱する | トリガー・非トリガー・曖昧時の既定を 3 点セットで書く |
| fallback の挙動を書かない | ツール不在の環境で停止するか、黙って手順を飛ばす | 「無ければ ATO のみで進め、その事実を checkpoint に残す」まで書く |
| Claude 側だけ更新する | Claude と Codex の契約が乖離し、同じ repo で挙動が変わる | `~/.claude/CLAUDE.md` と `~/.codex/AGENTS.md` は常に対で更新する |
| サイズ上限を無視して肥大化させる | Codex は `project_doc_max_bytes`（32KB 前後）で切り捨てる可能性。注意も希釈される | 25KB 程度を上限の目安とし、詳細は Skill / runbook へ逃がす |

---

## 8. リファレンススケルトン

新規マシンでゼロから書く場合、次の骨組みから始める。
**実物は `~/.claude/CLAUDE.md` と `~/.codex/AGENTS.md` を参照**（このガイドと対で維持されている）。

```markdown
# グローバル指示（全プロジェクト共通）

このファイルは <Claude Code|Codex CLI> の全セッションに注入される。
<もう一方> 向けの同契約は <パス>。

**強度の凡例**
| 強度 | 意味 |
|---|---|
| MUST | 例外なく従う。逸脱する場合は理由を ATO checkpoint に残す。 |
| SHOULD | 原則従う。合理的な理由があれば逸脱してよい。 |
| OPTIONAL | 環境固有の事実・参照情報。挙動を縛らない。 |

---
## 0. 必須コア (MUST)
  1. 日本語で応答する
  2. ATO が正本 (SoT)。意味のある作業は ato work begin してから着手
  3. コード変更を伴い PR に至る作業は FDA を既定の手順にする。それ以外は ATO のみ
  4. 着手前に ato search で過去事例を検索する
  5. CLI first（CLI 不可時のみ MCP fallback）
  6. 該当する Skill を読んでから着手する

## 1. ATO Control Plane (MUST)
  1.1 正本マッピング（表）
  1.2 主要コマンド
  1.3 Task State Persistence（「意味のある作業」の定義を列挙）
  1.4 Knowledge as a First-Class Artifact
  1.5 Forge promotion gate / loop verify gate
  1.6 MCP fallback
  ※ Codex 版のみ: 1.1 の前に「Session Start Check」を置く（hook が無いため）

## 2. FDA Delivery Agent (MUST — 適用トリガー時のみ)
  2.1 適用トリガー
  2.2 非トリガー（+ 曖昧時の既定）
  2.3 前提（fda 不在時の fail-soft、.fda/ 自動生成、偽 repo を作らない）
  2.4 ジャーニー（ato work begin → fda start → decide → design →
       implement --dry-run → role switch 実装 → review → continue → merge）
  2.5 Review Agent Gate（PR 必須証跡）
  2.6 fail-closed（自己承認禁止・REVIEW_AGENT_OK は merge approval でない・auto merge しない）
  2.7 merge 前の Forge gate
  2.8 正本ドキュメントへのポインタ

## 3. レポート出力と Notion projection (MUST: ローカル / SHOULD: Notion)
  3.1 出力先（1 タスク = 1 フォルダ、<topic>.md、report.md 禁止）
  3.2 体裁（メタ情報・構造・ファイルパス提示）
  3.3 ATO registry 登録（ato report attach）
  3.4 Notion projection（fail-soft）

## 4. Skill Bundle (MUST)
  作業種別 → Skill の一覧表（内容はコピペせずポインタのみ）

## 5. 環境固有の事実 (OPTIONAL — 参照情報)
  5.1 ATO CLI（パス・版数・env・DB）
  5.2 FDA CLI（パス・ソース・env）
  5.3 MCP server 起動・登録
  5.4 システムプロンプトの書き換え → 本ガイドへのポインタ
```

---

## 9. 変更手順

システムプロンプトの書き換えは**設定変更を伴う運用作業**であり、ATO の記録対象である。

```powershell
# 1. ATO task/run を開始
ato work begin --new --title "<変更内容>" --role orchestrator --json

# 2. 過去事例を検索（同じ轍を踏まない）
ato search "システムプロンプト" --json

# 3. バックアップ（必須）
$ts = Get-Date -Format "yyyyMMdd-HHmmss"
Copy-Item ~\.claude\CLAUDE.md  ~\.claude\CLAUDE.md.bak-$ts
Copy-Item ~\.codex\AGENTS.md   ~\.codex\AGENTS.md.bak-$ts

# 4. 編集（Claude 側と Codex 側を対で更新する）

# 5. 検証（§2.4）— 実測する。推測しない
claude -p "あなたのグローバル指示で、FDA を使う適用トリガーは何と定義されていますか"
codex exec "あなたのグローバル指示で、FDA を使う適用トリガーは何と定義されていますか"

# 6. サイズ確認（目安 25KB 以下）
(Get-Item ~\.claude\CLAUDE.md).Length
(Get-Item ~\.codex\AGENTS.md).Length

# 7. ATO に記録
ato work checkpoint --task <key> --run-id <run> --summary "<何をどう変えたか・なぜ>" --json
ato work complete --task <key> --run-id <run> --summary "<結果>" --json
```

### 9.1 チェックリスト

- [ ] 書き込み先が実際に読まれるファイルか（§2.4 で実測した）
- [ ] Claude 側と Codex 側を**両方**更新した
- [ ] 追加した記述に強度（MUST / SHOULD / OPTIONAL）を割り当てた
- [ ] MUST に入れたものは「省略したら壊れるもの」か（§4 の基準）
- [ ] 版数・パスは §5（OPTIONAL）に隔離したか
- [ ] Skill の中身をコピペしていないか
- [ ] サイズが目安（25KB）以内か
- [ ] バックアップを取ったか
- [ ] ATO に checkpoint を残したか
- [ ] 読まれなくなった旧ファイルを退避・削除したか

---

## 10. 変更履歴

- 2026-07-09: 初版。`~/.claude/CLAUDE.md` の MUST/SHOULD/OPTIONAL 再構成、
  `~/.codex/AGENTS.md` の新規正本化（`instructions.md` が codex-cli 0.139 に読まれていなかった事故を受けて）、
  FDA 適用範囲を「git repo 内のコード変更→PR デリバリー」に確定したのと同時に作成。
