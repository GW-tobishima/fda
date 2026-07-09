//! F2 Epic 継続ループ: `fda continue --epic` は epic run dir の `planned_prs.json` と、
//! `--artifacts-root`（既定 `artifacts/runs`）配下の全 run の receipt を `planned_pr_id` で
//! 突合し、PR ごとの状態を `epic_progress_state.json` に、次に進める planned PR /
//! waiting_human / blocked / complete を `next_planned_pr_decision.json` に投影する。
//!
//! **read-only 原則**: 既存 run の receipt は一切書き換えない。書き込むのは epic run dir の
//! 上記 2 ファイルだけ。auto merge / 自動実装開始はしない（Human Decision と merge は人間へ）。
//! 既存の `fda continue`（repair gate = `repair.rs`）は不変で、本モジュールは `--epic`
//! 指定時のみ実行される。
//!
//! **非権威 projection**: 本判定は advisory な判断支援であり merge の証明ではない。
//! merge 可否は実 merge gate（`src/application/merge.rs`）が fail-closed で担保する。
//! 自動化は epic_progress_state / next_planned_pr_decision を merge 判定に使ってはならない
//! （両出力の `advisory` フィールドに明記）。
//!
//! 依存充足の原則: sequence 順で「最初の未 merged PR」だけを見る。前 PR が全て merged で
//! ない限り後続 PR は選ばない（merge は人間承認のため、pr_open / merge_ready /
//! human_approval_required の PR は waiting_human として止める）。
//!
//! 証跡採用の原則（fail-closed と fail-soft の境界）:
//! - receipt の `epic_id` が現在の epic と一致するものだけを状態根拠に採用する。
//!   不一致・欠落は無視し `scan_notes` に記録する（別 epic の同名 planned_pr_id
//!   receipt による偽 merged を防ぐ）。
//! - 同一 planned PR に対し merged を示す run と PR open を示す run が併存したら
//!   `conflicting_evidence` として fail-closed で blocked にする（silently merged にしない）。
//! - receipt の parse error は **fail-soft**: `scan_errors` に記録して走査を継続する。
//!   trade-off: 本判定は advisory であり安全性は実 merge gate 側で担保されるため、
//!   壊れた 1 receipt で epic 全体の進捗可視性を失うより可用性を優先する。壊れた run は
//!   `fda gc` の棚卸しへ委ねる。ただし epic run dir の正本（planned_prs.json /
//!   human_decision_packet.json）の parse error は判定根拠そのものの欠損なので
//!   fail-closed（Err）のまま停止する。

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    read_json_value, recorded_decision_receipts_from_packet, value_string,
};
use crate::application::merge::is_merge_approval_decision;
use crate::application::ports::ArtifactStore;
use crate::cli::args::ContinueConfig;
use crate::domain::entities::HumanDecisionSummary;
use crate::domain::policies::decision::decision_blockers;
use crate::infra::clock::system_unix_seconds;
use crate::infra::fs_store::{list_dir_names, FsArtifactStore};
use crate::support::paths::{display_path, resolve_path};

const EPIC_PROGRESS_STATE_SCHEMA_VERSION: &str = "fda.epic_progress_state.v1";
const NEXT_PLANNED_PR_DECISION_SCHEMA_VERSION: &str = "fda.next_planned_pr_decision.v1";
/// 両出力 artifact に埋め込む提案性の明文（schema の required const と一致させる）。
const ADVISORY_TEXT: &str =
    "この判定は非権威の提案であり、実装開始許可・merge 承認・merge の証明ではない";

/// planned PR 1 件の進捗状態（epic_progress_state.json の `prs[]` 要素）。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct EpicPrState {
    pub(crate) planned_pr_id: String,
    pub(crate) sequence: u64,
    pub(crate) title: String,
    /// not_started | in_progress | pr_open | human_approval_required | merge_ready |
    /// merged | blocked
    pub(crate) status: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) reasons: Vec<String>,
    /// resume command 解決用の実 run dir（`<artifacts-root>/<run>` 表示）。
    /// evidence を観測した run から決定的に選ぶ。解決不能（矛盾等）は None。
    pub(crate) resume_run_dir: Option<String>,
    /// status=human_approval_required のとき、当該 run の human_decision_packet から
    /// 解決した merge approval の decision_id（解決不能なら None）。
    pub(crate) merge_decision_id: Option<String>,
}

/// 7 種 status を 5 バケットへ集計したサマリ。
#[derive(Debug, Default, Serialize)]
pub(crate) struct EpicSummary {
    pub(crate) merged: usize,
    /// in_progress + pr_open
    pub(crate) open: usize,
    pub(crate) blocked: usize,
    /// merge_ready + human_approval_required（人間の merge 承認 / 実行待ち）
    pub(crate) waiting_human: usize,
    pub(crate) not_started: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct EpicContinueResult {
    pub(crate) schema_version: &'static str,
    /// proceed | waiting_human | blocked | complete
    pub(crate) verdict: String,
    /// 提案性の明文（出力 2 artifact にも同文を埋め込む）。
    pub(crate) advisory: &'static str,
    pub(crate) epic_id: String,
    pub(crate) artifact_dir: String,
    pub(crate) artifacts_root: String,
    pub(crate) progress_state_path: String,
    pub(crate) next_decision_path: String,
    pub(crate) next_planned_pr_id: Option<String>,
    pub(crate) prs: Vec<EpicPrState>,
    pub(crate) summary: EpicSummary,
    /// epic_id 突合で状態根拠から除外した receipt の記録。
    pub(crate) scan_notes: Vec<String>,
    /// parse error 等で読めなかった receipt の記録（fail-soft で走査は継続済み）。
    pub(crate) scan_errors: Vec<String>,
    pub(crate) reasons: Vec<String>,
    pub(crate) resume_commands: Vec<String>,
    pub(crate) next_actions: Vec<String>,
}

/// 全 run から収集した 1 planned PR 分の証跡。矛盾検出と resume run dir の解決のため
/// 状態は run 名単位で保持する（同一 run 内の merged + open 併存は正常フローの残骸として
/// merged 優先で解決し、**別 run** 間の食い違いだけを矛盾と扱う）。
#[derive(Default)]
struct PrEvidence {
    /// merged を示した run 名。
    merged_runs: Vec<String>,
    /// PR open を示した run 名（同一 run が merged も示す場合は含めない）。
    open_runs: Vec<String>,
    /// merge_ready / human_approval_granted（承認済み・実行待ち）を示した run 名。
    merge_ready_runs: Vec<String>,
    /// human_approval_required（merge 承認が未記録）を示した run 名。
    approval_required_runs: Vec<String>,
    /// blocked / rejected を示した run 名。
    blocked_runs: Vec<String>,
    in_progress: bool,
    evidence: Vec<String>,
}

impl PrEvidence {
    fn merged(&self) -> bool {
        !self.merged_runs.is_empty()
    }

    /// 同一 planned PR に対し merged を示す run と PR open を示す run が併存する矛盾。
    /// silently merged にせず fail-closed で blocked に落とす。
    fn conflicting(&self) -> bool {
        !self.merged_runs.is_empty() && !self.open_runs.is_empty()
    }
}

/// 1 run 内で観測した planned PR 単位の証跡。run 内の優先解決に使う。
#[derive(Default)]
struct RunLocalEvidence {
    merged: bool,
    merge_ready: bool,
    approval_required: bool,
    pr_open: bool,
    blocked: bool,
    in_progress: bool,
    evidence: Vec<String>,
}

/// scan の結果。個別 receipt の異常は scan_notes / scan_errors に載せ、走査は継続する。
#[derive(Default)]
struct ScanOutcome {
    map: BTreeMap<String, PrEvidence>,
    scan_notes: Vec<String>,
    scan_errors: Vec<String>,
}

pub(crate) fn continue_epic(config: &ContinueConfig) -> Result<EpicContinueResult, String> {
    let store = FsArtifactStore;
    continue_epic_with(config, &store, system_unix_seconds())
}

fn continue_epic_with(
    config: &ContinueConfig,
    store: &impl ArtifactStore,
    now_unix: u64,
) -> Result<EpicContinueResult, String> {
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let artifacts_root = resolve_path(&repo_root, &config.artifacts_root);
    let artifact_dir_display = display_path(&repo_root, &artifact_dir);
    let artifacts_root_display = display_path(&repo_root, &artifacts_root);

    // 1. epic run dir の planned_prs.json（Epic の計画正本）を読む。
    //    正本の欠損・破損は fail-closed（判定根拠そのものが無い）。
    let planned_path = artifact_dir.join("planned_prs.json");
    if !store.exists(&planned_path) {
        return Err(format!(
            "fda continue --epic は epic run dir の planned_prs.json が必要です: {}",
            planned_path.display()
        ));
    }
    let planned = read_json_value(store, &planned_path)?;
    let epic_id = value_string(&planned, "epic_id").unwrap_or_else(|| "EPIC-UNKNOWN".to_string());
    let planned_prs = planned
        .get("planned_prs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    // 2. artifacts-root 配下の全 run から receipt / handoff を planned_pr_id で突合する。
    //    epic_id が一致する receipt のみ採用。parse error は fail-soft。
    let scan = collect_pr_evidence(store, &artifacts_root, &epic_id)?;

    // 3. planned PR ごとに status を確定する。
    let mut states = Vec::new();
    for pr in &planned_prs {
        let Some(planned_pr_id) = value_string(pr, "planned_pr_id") else {
            continue;
        };
        // sequence は依存順判定の根拠なので欠落・0 以下は fail-closed。
        // 既定 0 で fail-open にすると依存順が壊れたまま誤った next を選び得る。
        let sequence = pr
            .get("sequence")
            .and_then(Value::as_u64)
            .filter(|sequence| *sequence >= 1)
            .ok_or_else(|| {
                format!(
                    "planned_prs.json の {planned_pr_id} に 1 以上の sequence がありません（依存順の判定に必須のため fail-closed で停止します）"
                )
            })?;
        let title = value_string(pr, "title").unwrap_or_default();
        let (status, evidence_links, reasons, resume_run) = match scan.map.get(&planned_pr_id) {
            // 矛盾 evidence は silently merged にせず fail-closed で blocked。
            // resume run dir は解決しない（どの run が正か不明なため人間確認へ）。
            Some(item) if item.conflicting() => (
                "blocked",
                item.evidence.clone(),
                vec![format!(
                    "conflicting_evidence: merged を示す run ({}) と PR open を示す run ({}) が同一 planned PR に矛盾する状態を報告しています。silently merged にはせず fail-closed で blocked にします。evidence の receipt を人間が確認してください。",
                    item.merged_runs.join(", "),
                    item.open_runs.join(", ")
                )],
                None,
            ),
            Some(item) => {
                let status = resolve_status(item);
                (
                    status,
                    item.evidence.clone(),
                    vec![reason_for_status(status)],
                    resume_run_for_status(item, status).cloned(),
                )
            }
            None => (
                "not_started",
                Vec::new(),
                vec!["この planned PR に紐づく receipt / handoff がありません".to_string()],
                None,
            ),
        };
        // resume command 用の実 run dir（repo 相対表示）とそこにある merge decision。
        let resume_run_dir = resume_run
            .as_ref()
            .map(|run| format!("{artifacts_root_display}/{run}"));
        let merge_decision_id = if status == "human_approval_required" {
            resume_run
                .as_ref()
                .and_then(|run| resolve_merge_decision_id(store, &artifacts_root.join(run)))
        } else {
            None
        };
        states.push(EpicPrState {
            planned_pr_id,
            sequence,
            title,
            status: status.to_string(),
            evidence: evidence_links,
            reasons,
            resume_run_dir,
            merge_decision_id,
        });
    }
    states.sort_by(|a, b| {
        a.sequence
            .cmp(&b.sequence)
            .then_with(|| a.planned_pr_id.cmp(&b.planned_pr_id))
    });

    let summary = summarize(&states);

    // 4. epic run dir の未解決 Human Decision を確認する（Epic 全体を止める判断）。
    let unresolved = unresolved_epic_decisions(store, &artifact_dir)?;

    // 5. 依存充足済みの次 PR を判定する（read-only。auto merge / 自動実装はしない）。
    //    scan_errors は verdict に影響しない: 判定は advisory であり、実 merge gate が
    //    fail-closed で安全性を担保するため。
    let decision = decide_next(&states, &unresolved, &epic_id, &artifact_dir_display);

    // 6. epic run dir に 2 ファイルだけ書く（既存 receipt は不変）。
    let progress_value = epic_progress_state_value(
        &epic_id,
        now_unix,
        &states,
        &summary,
        &scan.scan_notes,
        &scan.scan_errors,
    );
    let progress_path = artifact_dir.join("epic_progress_state.json");
    store.write_json(&progress_path, &progress_value)?;

    let decision_value = next_decision_value(&epic_id, &decision);
    let decision_path = artifact_dir.join("next_planned_pr_decision.json");
    store.write_json(&decision_path, &decision_value)?;

    let mut next_actions = decision.resume_commands.clone();
    if !scan.scan_errors.is_empty() {
        next_actions.push(
            "壊れた receipt を含む run があります。fda gc で棚卸しすることを推奨します".to_string(),
        );
    }

    Ok(EpicContinueResult {
        schema_version: "fda.epic_continue_result.v0",
        verdict: decision.verdict.to_string(),
        advisory: ADVISORY_TEXT,
        epic_id,
        artifact_dir: artifact_dir_display,
        artifacts_root: artifacts_root_display,
        progress_state_path: display_path(&repo_root, &progress_path),
        next_decision_path: display_path(&repo_root, &decision_path),
        next_planned_pr_id: decision.next_planned_pr_id.clone(),
        prs: states,
        summary,
        scan_notes: scan.scan_notes,
        scan_errors: scan.scan_errors,
        reasons: decision.reasons.clone(),
        resume_commands: decision.resume_commands,
        next_actions,
    })
}

fn collect_pr_evidence(
    store: &impl ArtifactStore,
    artifacts_root: &Path,
    epic_id: &str,
) -> Result<ScanOutcome, String> {
    let mut outcome = ScanOutcome::default();
    // artifacts-root 自体が列挙できないのは環境異常なので fail-closed のまま。
    let run_names = if store.exists(artifacts_root) {
        list_dir_names(artifacts_root)?
    } else {
        Vec::new()
    };
    for name in &run_names {
        // `_gc` / `_policy` などの内部ディレクトリは走査対象外。
        if name.starts_with('_') {
            continue;
        }
        scan_run_receipts(
            store,
            name,
            &artifacts_root.join(name),
            epic_id,
            &mut outcome,
        );
    }
    Ok(outcome)
}

/// 1 run 分の receipt / handoff を読み、planned_pr_id ごとの証跡へ畳み込む。
///
/// - epic_id が現在の epic と一致する receipt だけを採用する（不一致・欠落は
///   scan_notes へ記録して無視）。
/// - parse error は fail-soft: scan_errors へ記録して残りの走査を継続する。
///   安全性は実 merge gate（fail-closed）側で担保されるため、ここでは epic 進捗の
///   可視性（可用性）を優先する。
fn scan_run_receipts(
    store: &impl ArtifactStore,
    run: &str,
    run_dir: &Path,
    epic_id: &str,
    outcome: &mut ScanOutcome,
) {
    let mut locals: BTreeMap<String, RunLocalEvidence> = BTreeMap::new();

    // github_merge_receipt.json: merge 実行済み → merged。
    match read_receipt_with_pr_id(store, run_dir, "github_merge_receipt.json") {
        Err(error) => outcome
            .scan_errors
            .push(format!("{run}/github_merge_receipt.json: {error}")),
        Ok(None) => {}
        Ok(Some((pr_id, value))) => {
            if receipt_epic_matches(
                &value,
                epic_id,
                run,
                "github_merge_receipt.json",
                &mut outcome.scan_notes,
            ) {
                let status = value_string(&value, "status").unwrap_or_default();
                let merge_executed = value
                    .get("merge_executed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let local = locals.entry(pr_id).or_default();
                local
                    .evidence
                    .push(format!("{run}/github_merge_receipt.json (status={status})"));
                if status == "succeeded" || merge_executed {
                    local.merged = true;
                }
            }
        }
    }

    // external_pr_receipt.json: merged / opened / blocked / rejected。
    match read_receipt_with_pr_id(store, run_dir, "external_pr_receipt.json") {
        Err(error) => outcome
            .scan_errors
            .push(format!("{run}/external_pr_receipt.json: {error}")),
        Ok(None) => {}
        Ok(Some((pr_id, value))) => {
            if receipt_epic_matches(
                &value,
                epic_id,
                run,
                "external_pr_receipt.json",
                &mut outcome.scan_notes,
            ) {
                let status = value_string(&value, "status").unwrap_or_default();
                let local = locals.entry(pr_id).or_default();
                local
                    .evidence
                    .push(format!("{run}/external_pr_receipt.json (status={status})"));
                match status.as_str() {
                    "merged" => local.merged = true,
                    "opened" | "open" => local.pr_open = true,
                    "blocked" | "rejected" => local.blocked = true,
                    _ => {}
                }
            }
        }
    }

    // merge_receipt.json: 承認済み実行待ちと未承認を区別して resume を実効化する。
    // merge_ready / human_approval_granted → merge_ready（fda merge --execute 待ち）、
    // human_approval_required → human_approval_required（fda decide 待ち）、
    // blocked → blocked。
    match read_receipt_with_pr_id(store, run_dir, "merge_receipt.json") {
        Err(error) => outcome
            .scan_errors
            .push(format!("{run}/merge_receipt.json: {error}")),
        Ok(None) => {}
        Ok(Some((pr_id, value))) => {
            if receipt_epic_matches(
                &value,
                epic_id,
                run,
                "merge_receipt.json",
                &mut outcome.scan_notes,
            ) {
                let status = value_string(&value, "status").unwrap_or_default();
                let local = locals.entry(pr_id).or_default();
                local
                    .evidence
                    .push(format!("{run}/merge_receipt.json (status={status})"));
                match status.as_str() {
                    "merged" => local.merged = true,
                    "merge_ready" | "human_approval_granted" => local.merge_ready = true,
                    "human_approval_required" => local.approval_required = true,
                    "blocked" => local.blocked = true,
                    _ => {}
                }
            }
        }
    }

    // handoff 系 artifact のみ（PR 未作成）→ in_progress。
    for file in [
        "current_codex_cli_handoff.json",
        "planned_pr_execution_packet.json",
    ] {
        match read_receipt_with_pr_id(store, run_dir, file) {
            Err(error) => outcome.scan_errors.push(format!("{run}/{file}: {error}")),
            Ok(None) => {}
            Ok(Some((pr_id, value))) => {
                if receipt_epic_matches(&value, epic_id, run, file, &mut outcome.scan_notes) {
                    let local = locals.entry(pr_id).or_default();
                    local.evidence.push(format!("{run}/{file}"));
                    local.in_progress = true;
                }
            }
        }
    }

    // run 内の証跡を epic 全体へ畳み込む。run 内では merged が優先（正常フローの
    // merge_ready / opened receipt の残骸を矛盾とは扱わない）。別 run 間の
    // merged vs open の食い違いだけが conflicting() で検出される。
    for (pr_id, local) in locals {
        let item = outcome.map.entry(pr_id).or_default();
        item.evidence.extend(local.evidence);
        if local.merged {
            item.merged_runs.push(run.to_string());
            continue;
        }
        if local.pr_open {
            item.open_runs.push(run.to_string());
        }
        if local.merge_ready {
            item.merge_ready_runs.push(run.to_string());
        }
        if local.approval_required {
            item.approval_required_runs.push(run.to_string());
        }
        if local.blocked {
            item.blocked_runs.push(run.to_string());
        }
        item.in_progress |= local.in_progress;
    }
}

/// receipt の epic_id を現在の epic と突合する。一致した receipt のみを状態根拠に
/// 採用する。不一致・欠落は無視し、その事実を scan_notes に記録する
/// （別 epic の同名 planned_pr_id receipt による偽 merged を防ぐ）。
fn receipt_epic_matches(
    value: &Value,
    epic_id: &str,
    run: &str,
    file: &str,
    scan_notes: &mut Vec<String>,
) -> bool {
    match value_string(value, "epic_id") {
        Some(receipt_epic) if receipt_epic == epic_id => true,
        Some(receipt_epic) => {
            scan_notes.push(format!(
                "{run}/{file}: epic_id `{receipt_epic}` が現在の epic `{epic_id}` と一致しないため状態根拠から除外しました"
            ));
            false
        }
        None => {
            scan_notes.push(format!(
                "{run}/{file}: epic_id が無く epic 突合できないため状態根拠から除外しました"
            ));
            false
        }
    }
}

fn read_receipt_with_pr_id(
    store: &impl ArtifactStore,
    run_dir: &Path,
    file: &str,
) -> Result<Option<(String, Value)>, String> {
    let path = run_dir.join(file);
    if !store.exists(&path) {
        return Ok(None);
    }
    let value = read_json_value(store, &path)?;
    match value_string(&value, "planned_pr_id") {
        Some(pr_id) if !pr_id.is_empty() => Ok(Some((pr_id, value))),
        _ => Ok(None),
    }
}

/// 証跡の優先順位で status を確定する（矛盾＝conflicting() は呼び出し側で先に blocked へ）。
/// merged > merge_ready > human_approval_required > pr_open > blocked > in_progress >
/// not_started。正の進捗を、別 run の古い blocked receipt が覆い隠さない。
fn resolve_status(item: &PrEvidence) -> &'static str {
    if item.merged() {
        "merged"
    } else if !item.merge_ready_runs.is_empty() {
        "merge_ready"
    } else if !item.approval_required_runs.is_empty() {
        "human_approval_required"
    } else if !item.open_runs.is_empty() {
        "pr_open"
    } else if !item.blocked_runs.is_empty() {
        "blocked"
    } else if item.in_progress {
        "in_progress"
    } else {
        "not_started"
    }
}

/// status に対応する resume 用 run を決定的に選ぶ（走査はソート済み run 名順なので先頭）。
fn resume_run_for_status<'a>(item: &'a PrEvidence, status: &str) -> Option<&'a String> {
    match status {
        "merge_ready" => item.merge_ready_runs.first(),
        "human_approval_required" => item.approval_required_runs.first(),
        "pr_open" => item.open_runs.first(),
        "blocked" => item.blocked_runs.first(),
        _ => None,
    }
}

/// human_approval_required の run dir から merge approval の未解決 decision_id を解決する。
/// 判定基準は merge gate と同一（`is_merge_approval_decision`）。読めない・見つからない
/// 場合は None（resume は手動確認の案内へ fallback。判定は advisory なので fail-soft）。
fn resolve_merge_decision_id(store: &impl ArtifactStore, run_dir: &Path) -> Option<String> {
    let packet_path = run_dir.join("human_decision_packet.json");
    if !store.exists(&packet_path) {
        return None;
    }
    let packet = read_json_value(store, &packet_path).ok()?;
    let decisions = decision_summaries_from_packet(&packet);
    let mut receipts =
        read_decision_receipts(store, &run_dir.join("decision_receipts.json")).ok()?;
    for (decision_id, receipt) in recorded_decision_receipts_from_packet(&packet) {
        receipts.entry(decision_id).or_insert(receipt);
    }
    let answers = decision_answers_from_receipts(&receipts);
    decision_blockers(&decisions, &answers)
        .into_iter()
        .find(is_merge_approval_decision)
        .map(|decision| decision.decision_id)
}

fn reason_for_status(status: &str) -> String {
    match status {
        "merged" => "merge receipt があり merged です".to_string(),
        "merge_ready" => {
            "merge gate を通過し merge 実行待ちです（人間が fda merge --execute を実行）"
                .to_string()
        }
        "human_approval_required" => {
            "merge gate は到達しましたが merge approval が未記録です（人間の fda decide 待ち）"
                .to_string()
        }
        "pr_open" => "external_pr_receipt が opened で PR が開いています".to_string(),
        "blocked" => "receipt が blocked / rejected です".to_string(),
        "in_progress" => "handoff artifact があり実装中です（PR 未作成）".to_string(),
        _ => "この planned PR に紐づく receipt / handoff がありません".to_string(),
    }
}

fn summarize(states: &[EpicPrState]) -> EpicSummary {
    let mut summary = EpicSummary::default();
    for state in states {
        match state.status.as_str() {
            "merged" => summary.merged += 1,
            "in_progress" | "pr_open" => summary.open += 1,
            "merge_ready" | "human_approval_required" => summary.waiting_human += 1,
            "blocked" => summary.blocked += 1,
            "not_started" => summary.not_started += 1,
            _ => {}
        }
    }
    summary
}

fn unresolved_epic_decisions(
    store: &impl ArtifactStore,
    run_dir: &Path,
) -> Result<Vec<HumanDecisionSummary>, String> {
    let packet_path = run_dir.join("human_decision_packet.json");
    if !store.exists(&packet_path) {
        return Ok(Vec::new());
    }
    let packet = read_json_value(store, &packet_path)?;
    let decisions = decision_summaries_from_packet(&packet);
    let mut receipts = read_decision_receipts(store, &run_dir.join("decision_receipts.json"))?;
    for (decision_id, receipt) in recorded_decision_receipts_from_packet(&packet) {
        receipts.entry(decision_id).or_insert(receipt);
    }
    Ok(decision_blockers(
        &decisions,
        &decision_answers_from_receipts(&receipts),
    ))
}

/// decide_next の結果（純データ）。
struct NextDecision {
    verdict: &'static str,
    next_planned_pr_id: Option<String>,
    reasons: Vec<String>,
    resume_commands: Vec<String>,
}

/// 依存充足済みの次 PR を判定する純ロジック。
///
/// 優先順位:
/// 1. planned PR が空、または全 merged → complete。
/// 2. epic run dir に未解決 Human Decision → waiting_human（fda decide の resume）。
/// 3. sequence 順で最初の未 merged PR を見る:
///    - not_started / in_progress → proceed（その PR を next にする）。
///    - merge_ready → waiting_human（承認済み。人間が fda merge --execute）。
///    - human_approval_required → waiting_human（人間が fda decide で merge approval）。
///    - pr_open → waiting_human（人間の review / merge 承認待ち）。
///    - blocked → blocked（repair / receipt 確認へ）。
fn decide_next(
    states: &[EpicPrState],
    unresolved: &[HumanDecisionSummary],
    epic_id: &str,
    artifact_dir_display: &str,
) -> NextDecision {
    if states.is_empty() {
        return NextDecision {
            verdict: "complete",
            next_planned_pr_id: None,
            reasons: vec![format!("Epic {epic_id} に planned PR がありません。")],
            resume_commands: vec![format!(
                "planned_prs.json を確認してください（{artifact_dir_display}）。"
            )],
        };
    }
    if states.iter().all(|state| state.status == "merged") {
        return NextDecision {
            verdict: "complete",
            next_planned_pr_id: None,
            reasons: vec![format!("Epic {epic_id} の全 planned PR が merged です。")],
            resume_commands: vec![format!(
                "Epic {epic_id} は完了です。merge 承認は人間が実施済みです。"
            )],
        };
    }
    if !unresolved.is_empty() {
        let reasons = unresolved
            .iter()
            .map(|decision| {
                format!(
                    "未解決 Human Decision {}: {}",
                    decision.decision_id, decision.summary
                )
            })
            .collect();
        let resume_commands = unresolved
            .iter()
            .map(|decision| {
                format!(
                    "fda decide {} --answer <答え> --artifacts {artifact_dir_display}",
                    decision.decision_id
                )
            })
            .collect();
        return NextDecision {
            verdict: "waiting_human",
            next_planned_pr_id: None,
            reasons,
            resume_commands,
        };
    }

    // sequence 順で最初の未 merged PR。前 PR は全て merged（それ自身が最初の未 merged のため）。
    let pr = states
        .iter()
        .find(|state| state.status != "merged")
        .expect("all-merged case handled above");
    let pr_id = pr.planned_pr_id.clone();
    match pr.status.as_str() {
        "not_started" | "in_progress" => NextDecision {
            verdict: "proceed",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: vec![format!(
                "sequence {} までの依存 PR は全て merged で、次に着手する PR は {} (status={}) です。",
                pr.sequence, pr_id, pr.status
            )],
            resume_commands: vec![format!(
                "{pr_id} を実装する: fda implement --dry-run --artifacts {artifact_dir_display} --target-repo <target repo> の後 Implementer が実装し fda review"
            )],
        },
        "merge_ready" => NextDecision {
            verdict: "waiting_human",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: vec![format!(
                "次の未 merged PR {pr_id} は merge_ready（承認済み・merge 実行待ち）です。merge の実行は人間が行うため後続 PR はまだ選べません。"
            )],
            resume_commands: vec![match &pr.resume_run_dir {
                Some(run_dir) => format!(
                    "{pr_id} の merge を実行する: fda merge --artifacts {run_dir} --target-repo <target repo> --execute"
                ),
                None => format!(
                    "{pr_id} の run dir が特定できません。fda gc で棚卸しして run を確認してください"
                ),
            }],
        },
        "human_approval_required" => NextDecision {
            verdict: "waiting_human",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: vec![format!(
                "次の未 merged PR {pr_id} は human_approval_required（merge approval 未記録）です。人間の承認が必要なため後続 PR はまだ選べません。"
            )],
            resume_commands: vec![match (&pr.merge_decision_id, &pr.resume_run_dir) {
                (Some(decision_id), Some(run_dir)) => format!(
                    "{pr_id} の merge 承認を人間が記録する: fda decide {decision_id} --answer approve --artifacts {run_dir} の後 fda merge --artifacts {run_dir} --target-repo <target repo> --execute"
                ),
                (None, Some(run_dir)) => format!(
                    "{pr_id} の merge approval decision が解決できませんでした。fda status --artifacts {run_dir} で未解決判断を確認してください"
                ),
                _ => format!(
                    "{pr_id} の run dir が特定できません。fda gc で棚卸しして run を確認してください"
                ),
            }],
        },
        "pr_open" => NextDecision {
            verdict: "waiting_human",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: vec![format!(
                "次の未 merged PR {pr_id} は pr_open です。review と merge（人間承認）が必要なため後続 PR はまだ選べません。"
            )],
            resume_commands: vec![match &pr.resume_run_dir {
                Some(run_dir) => format!(
                    "{pr_id} の PR を review し人間が merge を承認する: fda review --artifacts {run_dir} の後 fda merge --artifacts {run_dir} --target-repo <target repo>"
                ),
                None => format!(
                    "{pr_id} の run dir が特定できません。fda gc で棚卸しして run を確認してください"
                ),
            }],
        },
        _ => NextDecision {
            verdict: "blocked",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: pr.reasons.clone(),
            resume_commands: vec![match &pr.resume_run_dir {
                Some(run_dir) => format!(
                    "{pr_id} を repair する: fda continue --artifacts {run_dir} --target-repo <target repo>"
                ),
                None => format!(
                    "{pr_id} の evidence の receipt を人間が確認してください（run dir が特定できない場合は fda gc で棚卸し）"
                ),
            }],
        },
    }
}

fn epic_progress_state_value(
    epic_id: &str,
    now_unix: u64,
    states: &[EpicPrState],
    summary: &EpicSummary,
    scan_notes: &[String],
    scan_errors: &[String],
) -> Value {
    json!({
        "schema_version": EPIC_PROGRESS_STATE_SCHEMA_VERSION,
        "advisory": ADVISORY_TEXT,
        "epic_id": epic_id,
        "generated_at_unix": now_unix,
        "prs": states
            .iter()
            .map(|state| json!({
                "planned_pr_id": state.planned_pr_id,
                "sequence": state.sequence,
                "title": state.title,
                "status": state.status,
                "evidence": state.evidence,
                "reasons": state.reasons,
            }))
            .collect::<Vec<_>>(),
        "summary": {
            "merged": summary.merged,
            "open": summary.open,
            "blocked": summary.blocked,
            "waiting_human": summary.waiting_human,
            "not_started": summary.not_started,
        },
        "scan_notes": scan_notes,
        "scan_errors": scan_errors,
    })
}

fn next_decision_value(epic_id: &str, decision: &NextDecision) -> Value {
    json!({
        "schema_version": NEXT_PLANNED_PR_DECISION_SCHEMA_VERSION,
        "advisory": ADVISORY_TEXT,
        "epic_id": epic_id,
        "verdict": decision.verdict,
        "next_planned_pr_id": decision.next_planned_pr_id,
        "reasons": decision.reasons,
        "resume_commands": decision.resume_commands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::AtoConfig;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_repo(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = system_unix_seconds();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("{name}-{unique}-{seq}"));
        FsArtifactStore.create_dir_all(&dir).unwrap();
        dir
    }

    const EPIC_RUN: &str = "epic-run";

    fn epic_config(repo: &Path) -> ContinueConfig {
        ContinueConfig {
            repo_root: repo.to_path_buf(),
            artifact_dir: PathBuf::from(format!("artifacts/runs/{EPIC_RUN}")),
            out: None,
            target_repo: PathBuf::from("."),
            max_retries: 3,
            ato: AtoConfig::default(),
            print_json: false,
            epic: true,
            artifacts_root: PathBuf::from("artifacts/runs"),
        }
    }

    /// epic run dir に planned_prs.json を書く。prs: (id, sequence, title)。
    fn write_planned_prs(repo: &Path, epic_id: &str, prs: &[(&str, u64, &str)]) {
        let store = FsArtifactStore;
        let epic_dir = repo.join("artifacts").join("runs").join(EPIC_RUN);
        store.create_dir_all(&epic_dir).unwrap();
        let planned: Vec<Value> = prs
            .iter()
            .map(|(id, seq, title)| json!({"planned_pr_id": id, "sequence": seq, "title": title}))
            .collect();
        store
            .write_json(
                &epic_dir.join("planned_prs.json"),
                &json!({
                    "schema_version": "forge_delivery.planned_prs.v0",
                    "epic_id": epic_id,
                    "planned_prs": planned,
                }),
            )
            .unwrap();
    }

    fn write_receipt(repo: &Path, run: &str, file: &str, value: Value) {
        let store = FsArtifactStore;
        let run_dir = repo.join("artifacts").join("runs").join(run);
        store.create_dir_all(&run_dir).unwrap();
        store.write_json(&run_dir.join(file), &value).unwrap();
    }

    fn run_epic(repo: &Path) -> EpicContinueResult {
        continue_epic_with(&epic_config(repo), &FsArtifactStore, 1_783_555_200).unwrap()
    }

    fn status_of<'a>(result: &'a EpicContinueResult, pr_id: &str) -> &'a str {
        result
            .prs
            .iter()
            .find(|state| state.planned_pr_id == pr_id)
            .map(|state| state.status.as_str())
            .unwrap_or("<missing>")
    }

    #[test]
    fn merged_open_notstarted_does_not_select_dependency_unsatisfied_pr() {
        // seq1 merged / seq2 pr_open / seq3 not_started。
        let repo = temp_repo("fda-epic-dep");
        write_planned_prs(
            &repo,
            "EPIC-T1",
            &[("PR-1", 1, "one"), ("PR-2", 2, "two"), ("PR-3", 3, "three")],
        );
        write_receipt(
            &repo,
            "run-pr1",
            "github_merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T1", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-pr2",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-2", "epic_id": "EPIC-T1", "status": "opened"}),
        );

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-1"), "merged");
        assert_eq!(status_of(&result, "PR-2"), "pr_open");
        assert_eq!(status_of(&result, "PR-3"), "not_started");

        // 依存充足していない PR-3 は絶対に next に選ばれない（PR-2 が merged でないため）。
        assert_ne!(result.next_planned_pr_id.as_deref(), Some("PR-3"));
        assert_eq!(result.verdict, "waiting_human");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-2"));
        // resume command はプレースホルダでなく実 run dir を指す。
        assert!(result
            .resume_commands
            .iter()
            .any(|command| command.contains("run-pr2") && !command.contains("<PR-2")));
    }

    #[test]
    fn selects_dependency_satisfied_not_started_pr() {
        // seq1,2 merged / seq3 not_started → next = PR-3。
        let repo = temp_repo("fda-epic-proceed");
        write_planned_prs(
            &repo,
            "EPIC-T2",
            &[("PR-1", 1, "one"), ("PR-2", 2, "two"), ("PR-3", 3, "three")],
        );
        write_receipt(
            &repo,
            "run-pr1",
            "github_merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T2", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-pr2",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-2", "epic_id": "EPIC-T2", "status": "merged"}),
        );

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-2"), "merged");
        assert_eq!(status_of(&result, "PR-3"), "not_started");
        assert_eq!(result.verdict, "proceed");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-3"));
    }

    #[test]
    fn unresolved_human_decision_yields_waiting_human_with_resume_command() {
        let repo = temp_repo("fda-epic-decision");
        write_planned_prs(&repo, "EPIC-T3", &[("PR-1", 1, "one"), ("PR-2", 2, "two")]);
        // PR-1 は着手可能だが、epic run dir に未解決判断があるため止まる。
        let store = FsArtifactStore;
        let epic_dir = repo.join("artifacts").join("runs").join(EPIC_RUN);
        store
            .write_json(
                &epic_dir.join("human_decision_packet.json"),
                &json!({
                    "schema_version": "fda.human_decision_packet.v0",
                    "decisions": [{
                        "decision_id": "HD-EPIC-001",
                        "type": "spec_decision",
                        "summary": "Epic 全体のスコープを固定してよいか",
                        "required_before": "Design Gate",
                        "options": [{"id": "yes"}, {"id": "no"}],
                        "recommended_option_id": "yes"
                    }]
                }),
            )
            .unwrap();

        let result = run_epic(&repo);

        assert_eq!(result.verdict, "waiting_human");
        assert!(result.next_planned_pr_id.is_none());
        assert!(result
            .resume_commands
            .iter()
            .any(|command| command.contains("fda decide") && command.contains("HD-EPIC-001")));
    }

    #[test]
    fn all_merged_yields_complete() {
        let repo = temp_repo("fda-epic-complete");
        write_planned_prs(&repo, "EPIC-T4", &[("PR-1", 1, "one"), ("PR-2", 2, "two")]);
        write_receipt(
            &repo,
            "run-pr1",
            "github_merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T4", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-pr2",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-2", "epic_id": "EPIC-T4", "status": "merged"}),
        );

        let result = run_epic(&repo);

        assert_eq!(result.verdict, "complete");
        assert!(result.next_planned_pr_id.is_none());
        assert_eq!(result.summary.merged, 2);
    }

    #[test]
    fn detects_merge_ready_in_progress_and_blocked_statuses() {
        let repo = temp_repo("fda-epic-status");
        write_planned_prs(
            &repo,
            "EPIC-T5",
            &[
                ("PR-1", 1, "one"),
                ("PR-2", 2, "two"),
                ("PR-3", 3, "three"),
                ("PR-4", 4, "four"),
            ],
        );
        // PR-1: merge_ready (merge_receipt)
        write_receipt(
            &repo,
            "run-pr1",
            "merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T5", "status": "merge_ready"}),
        );
        // PR-2: in_progress (handoff のみ)
        write_receipt(
            &repo,
            "run-pr2",
            "current_codex_cli_handoff.json",
            json!({"planned_pr_id": "PR-2", "epic_id": "EPIC-T5"}),
        );
        // PR-3: blocked (external_pr rejected)
        write_receipt(
            &repo,
            "run-pr3",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-3", "epic_id": "EPIC-T5", "status": "rejected"}),
        );
        // PR-4: 証跡なし → not_started

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-1"), "merge_ready");
        assert_eq!(status_of(&result, "PR-2"), "in_progress");
        assert_eq!(status_of(&result, "PR-3"), "blocked");
        assert_eq!(status_of(&result, "PR-4"), "not_started");
        assert_eq!(result.summary.waiting_human, 1); // merge_ready
        assert_eq!(result.summary.open, 1); // in_progress
        assert_eq!(result.summary.blocked, 1);
        assert_eq!(result.summary.not_started, 1);

        // 最初の未 merged が merge_ready のため waiting_human。
        assert_eq!(result.verdict, "waiting_human");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-1"));
    }

    #[test]
    fn merge_ready_resume_points_to_merge_execute_with_real_run_dir() {
        // 承認済み・実行待ち（merge_ready / human_approval_granted）→ fda merge --execute。
        let repo = temp_repo("fda-epic-mr-resume");
        write_planned_prs(&repo, "EPIC-T12", &[("PR-1", 1, "one"), ("PR-2", 2, "two")]);
        write_receipt(
            &repo,
            "run-pr1",
            "merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T12", "status": "merge_ready"}),
        );
        // human_approval_granted も同じ merge_ready バケット。
        write_receipt(
            &repo,
            "run-pr2",
            "merge_receipt.json",
            json!({"planned_pr_id": "PR-2", "epic_id": "EPIC-T12", "status": "human_approval_granted"}),
        );

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-1"), "merge_ready");
        assert_eq!(status_of(&result, "PR-2"), "merge_ready");
        assert_eq!(result.verdict, "waiting_human");
        // resume はプレースホルダではなく実 run dir + --execute。
        let resume = result.resume_commands.join(" / ");
        assert!(resume.contains("fda merge --artifacts"));
        assert!(resume.contains("run-pr1"));
        assert!(resume.contains("--execute"));
        assert!(!resume.contains("<PR-1"));
    }

    #[test]
    fn human_approval_required_resume_points_to_decide_with_decision_id() {
        // 未承認（human_approval_required）→ 当該 run の merge approval decision を解決し
        // fda decide <decision_id> --answer approve を提示。
        let repo = temp_repo("fda-epic-har-resume");
        write_planned_prs(&repo, "EPIC-T13", &[("PR-1", 1, "one")]);
        write_receipt(
            &repo,
            "run-pr1",
            "merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T13", "status": "human_approval_required"}),
        );
        write_receipt(
            &repo,
            "run-pr1",
            "human_decision_packet.json",
            json!({
                "schema_version": "fda.human_decision_packet.v0",
                "decisions": [{
                    "decision_id": "HD-MERGE-001",
                    "type": "merge_decision",
                    "summary": "PR-1 の merge を承認してよいか",
                    "required_before": "Merge Gate",
                    "options": [{"id": "approve"}, {"id": "reject"}],
                    "recommended_option_id": "approve"
                }]
            }),
        );

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-1"), "human_approval_required");
        assert_eq!(result.summary.waiting_human, 1);
        assert_eq!(result.verdict, "waiting_human");
        let resume = result.resume_commands.join(" / ");
        assert!(resume.contains("fda decide HD-MERGE-001 --answer approve"));
        assert!(resume.contains("run-pr1"));
        assert!(!resume.contains("<PR-1"));
    }

    #[test]
    fn first_unmerged_blocked_pr_yields_blocked_verdict() {
        // 先頭の未 merged PR 自体が blocked → verdict=blocked（repair へ）。
        let repo = temp_repo("fda-epic-blocked-verdict");
        write_planned_prs(&repo, "EPIC-T8", &[("PR-1", 1, "one"), ("PR-2", 2, "two")]);
        write_receipt(
            &repo,
            "run-pr1",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T8", "status": "rejected"}),
        );

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-1"), "blocked");
        assert_eq!(result.verdict, "blocked");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-1"));
        // resume は実 run dir を指した repair コマンド。
        assert!(result
            .resume_commands
            .iter()
            .any(|command| command.contains("fda continue") && command.contains("run-pr1")));
    }

    #[test]
    fn fake_merged_receipt_from_other_epic_is_ignored() {
        // 別 epic_id の merged receipt / epic_id 欠落 receipt は状態根拠にならない。
        let repo = temp_repo("fda-epic-foreign");
        write_planned_prs(&repo, "EPIC-T7", &[("PR-1", 1, "one")]);
        // 別 epic の偽 merged receipt。
        write_receipt(
            &repo,
            "run-foreign",
            "github_merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-OTHER", "status": "succeeded", "merge_executed": true}),
        );
        // epic_id 欠落の merged receipt も無視される。
        write_receipt(
            &repo,
            "run-no-epic",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-1", "status": "merged"}),
        );

        let result = run_epic(&repo);

        // 偽 merged は無視され not_started のまま → verdict=proceed / next=PR-1。
        assert_eq!(status_of(&result, "PR-1"), "not_started");
        assert_eq!(result.verdict, "proceed");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-1"));
        // 無視した事実が scan_notes に記録される。
        assert!(result
            .scan_notes
            .iter()
            .any(|note| note.contains("run-foreign") && note.contains("EPIC-OTHER")));
        assert!(result
            .scan_notes
            .iter()
            .any(|note| note.contains("run-no-epic") && note.contains("epic_id が無く")));
        // epic_progress_state.json にも記録される。
        let store = FsArtifactStore;
        let progress = read_json_value(
            &store,
            &repo
                .join("artifacts")
                .join("runs")
                .join(EPIC_RUN)
                .join("epic_progress_state.json"),
        )
        .unwrap();
        let notes = progress
            .get("scan_notes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn conflicting_merged_and_open_evidence_is_blocked() {
        // 別 run から merged と open が同一 planned PR に矛盾報告 → fail-closed で blocked。
        let repo = temp_repo("fda-epic-conflict");
        write_planned_prs(&repo, "EPIC-T9", &[("PR-1", 1, "one")]);
        write_receipt(
            &repo,
            "run-merged",
            "github_merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T9", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-open",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T9", "status": "opened"}),
        );

        let result = run_epic(&repo);

        // silently merged にならず blocked（人間の確認へ）。
        assert_eq!(status_of(&result, "PR-1"), "blocked");
        assert_eq!(result.verdict, "blocked");
        let pr = result
            .prs
            .iter()
            .find(|state| state.planned_pr_id == "PR-1")
            .unwrap();
        assert!(pr
            .reasons
            .iter()
            .any(|reason| reason.contains("conflicting_evidence")
                && reason.contains("run-merged")
                && reason.contains("run-open")));
        // 根拠パス（両 receipt）が evidence に列挙される。
        assert!(pr
            .evidence
            .iter()
            .any(|link| link.contains("run-merged/github_merge_receipt.json")));
        assert!(pr
            .evidence
            .iter()
            .any(|link| link.contains("run-open/external_pr_receipt.json")));
        // 矛盾はどの run が正か不明のため resume run dir を解決しない（人間確認へ誘導）。
        assert!(pr.resume_run_dir.is_none());
        assert!(result
            .resume_commands
            .iter()
            .any(|command| command.contains("receipt を人間が確認")));
    }

    #[test]
    fn broken_receipt_json_is_fail_soft_and_recorded_in_scan_errors() {
        // 無関係 run の壊れた receipt JSON があっても判定は完走し、scan_errors に載る。
        let repo = temp_repo("fda-epic-broken");
        write_planned_prs(&repo, "EPIC-T10", &[("PR-1", 1, "one"), ("PR-2", 2, "two")]);
        write_receipt(
            &repo,
            "run-pr1",
            "github_merge_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T10", "status": "succeeded", "merge_executed": true}),
        );
        // 壊れた JSON を持つ無関係の run。
        let store = FsArtifactStore;
        let broken_dir = repo.join("artifacts").join("runs").join("run-broken");
        store.create_dir_all(&broken_dir).unwrap();
        store
            .write_text(
                &broken_dir.join("external_pr_receipt.json"),
                "{ this is not json",
            )
            .unwrap();

        let result = run_epic(&repo);

        // 判定は完走: PR-1 merged / PR-2 not_started → proceed。
        assert_eq!(status_of(&result, "PR-1"), "merged");
        assert_eq!(result.verdict, "proceed");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-2"));
        // scan_errors に記録され、next_actions で fda gc が推奨される。
        assert!(result
            .scan_errors
            .iter()
            .any(|error| error.contains("run-broken/external_pr_receipt.json")));
        assert!(result
            .next_actions
            .iter()
            .any(|action| action.contains("fda gc")));
        // epic_progress_state.json にも scan_errors が載る。
        let progress = read_json_value(
            &store,
            &repo
                .join("artifacts")
                .join("runs")
                .join(EPIC_RUN)
                .join("epic_progress_state.json"),
        )
        .unwrap();
        let errors = progress
            .get("scan_errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(errors.len(), 1);
        // 壊れたファイル自体は変更されない（read-only 原則）。
        assert_eq!(
            store
                .read_text(&broken_dir.join("external_pr_receipt.json"))
                .unwrap(),
            "{ this is not json"
        );
    }

    #[test]
    fn missing_sequence_is_a_fail_closed_error() {
        // sequence 欠落・0 は依存順の判定不能なので fail-closed で Err。
        let repo = temp_repo("fda-epic-noseq");
        let store = FsArtifactStore;
        let epic_dir = repo.join("artifacts").join("runs").join(EPIC_RUN);
        store.create_dir_all(&epic_dir).unwrap();
        store
            .write_json(
                &epic_dir.join("planned_prs.json"),
                &json!({
                    "schema_version": "forge_delivery.planned_prs.v0",
                    "epic_id": "EPIC-T11",
                    "planned_prs": [{"planned_pr_id": "PR-1", "title": "no sequence"}],
                }),
            )
            .unwrap();

        let err = continue_epic_with(&epic_config(&repo), &FsArtifactStore, 1).unwrap_err();
        assert!(err.contains("PR-1") && err.contains("sequence"));
    }

    #[test]
    fn writes_two_files_and_leaves_receipts_unchanged() {
        let repo = temp_repo("fda-epic-readonly");
        write_planned_prs(&repo, "EPIC-T6", &[("PR-1", 1, "one")]);
        write_receipt(
            &repo,
            "run-pr1",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-1", "epic_id": "EPIC-T6", "status": "opened"}),
        );

        let result = run_epic(&repo);

        let store = FsArtifactStore;
        let epic_dir = repo.join("artifacts").join("runs").join(EPIC_RUN);
        // 2 ファイルが epic run dir に生成される。
        let progress = read_json_value(&store, &epic_dir.join("epic_progress_state.json")).unwrap();
        assert_eq!(
            progress.get("schema_version").and_then(Value::as_str),
            Some("fda.epic_progress_state.v1")
        );
        // 提案性の明文（advisory）が両出力に埋め込まれる。
        assert!(progress
            .get("advisory")
            .and_then(Value::as_str)
            .is_some_and(|advisory| advisory.contains("非権威")));
        let decision =
            read_json_value(&store, &epic_dir.join("next_planned_pr_decision.json")).unwrap();
        assert_eq!(
            decision.get("schema_version").and_then(Value::as_str),
            Some("fda.next_planned_pr_decision.v1")
        );
        assert!(decision
            .get("advisory")
            .and_then(Value::as_str)
            .is_some_and(|advisory| advisory.contains("非権威")));

        // 既存 receipt は書き換えられていない。
        let receipt = read_json_value(
            &store,
            &repo
                .join("artifacts")
                .join("runs")
                .join("run-pr1")
                .join("external_pr_receipt.json"),
        )
        .unwrap();
        assert_eq!(
            receipt.get("status").and_then(Value::as_str),
            Some("opened")
        );
        assert_eq!(result.verdict, "waiting_human");
    }

    #[test]
    fn missing_planned_prs_is_an_error() {
        let repo = temp_repo("fda-epic-missing");
        FsArtifactStore
            .create_dir_all(&repo.join("artifacts").join("runs").join(EPIC_RUN))
            .unwrap();
        let err = continue_epic_with(&epic_config(&repo), &FsArtifactStore, 1).unwrap_err();
        assert!(err.contains("planned_prs.json"));
    }
}
