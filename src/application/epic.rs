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
//! 依存充足の原則: sequence 順で「最初の未 merged PR」だけを見る。前 PR が全て merged で
//! ない限り後続 PR は選ばない（merge は人間承認のため、pr_open / merge_ready の PR は
//! waiting_human として止める）。

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    read_json_value, recorded_decision_receipts_from_packet, value_string,
};
use crate::application::ports::ArtifactStore;
use crate::cli::args::ContinueConfig;
use crate::domain::entities::HumanDecisionSummary;
use crate::domain::policies::decision::decision_blockers;
use crate::infra::clock::system_unix_seconds;
use crate::infra::fs_store::{list_dir_names, FsArtifactStore};
use crate::support::paths::{display_path, resolve_path};

const EPIC_PROGRESS_STATE_SCHEMA_VERSION: &str = "fda.epic_progress_state.v1";
const NEXT_PLANNED_PR_DECISION_SCHEMA_VERSION: &str = "fda.next_planned_pr_decision.v1";

/// planned PR 1 件の進捗状態（epic_progress_state.json の `prs[]` 要素）。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct EpicPrState {
    pub(crate) planned_pr_id: String,
    pub(crate) sequence: u64,
    pub(crate) title: String,
    /// not_started | in_progress | pr_open | merge_ready | merged | blocked
    pub(crate) status: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) reasons: Vec<String>,
}

/// 6 種 status を 5 バケットへ集計したサマリ。
#[derive(Debug, Default, Serialize)]
pub(crate) struct EpicSummary {
    pub(crate) merged: usize,
    /// in_progress + pr_open
    pub(crate) open: usize,
    pub(crate) blocked: usize,
    /// merge_ready（人間の merge 承認待ち）
    pub(crate) waiting_human: usize,
    pub(crate) not_started: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct EpicContinueResult {
    pub(crate) schema_version: &'static str,
    /// proceed | waiting_human | blocked | complete
    pub(crate) verdict: String,
    pub(crate) epic_id: String,
    pub(crate) artifact_dir: String,
    pub(crate) artifacts_root: String,
    pub(crate) progress_state_path: String,
    pub(crate) next_decision_path: String,
    pub(crate) next_planned_pr_id: Option<String>,
    pub(crate) prs: Vec<EpicPrState>,
    pub(crate) summary: EpicSummary,
    pub(crate) reasons: Vec<String>,
    pub(crate) resume_commands: Vec<String>,
    pub(crate) next_actions: Vec<String>,
}

/// 全 run から収集した 1 planned PR 分の証跡（複数 run を OR で畳み込んだ純データ）。
#[derive(Default)]
struct PrEvidence {
    merged: bool,
    merge_ready: bool,
    pr_open: bool,
    blocked: bool,
    in_progress: bool,
    evidence: Vec<String>,
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
    let evidence = collect_pr_evidence(store, &artifacts_root)?;

    // 3. planned PR ごとに status を確定する。
    let mut states = Vec::new();
    for pr in &planned_prs {
        let Some(planned_pr_id) = value_string(pr, "planned_pr_id") else {
            continue;
        };
        let sequence = pr.get("sequence").and_then(Value::as_u64).unwrap_or(0);
        let title = value_string(pr, "title").unwrap_or_default();
        let (status, evidence_links, reasons) = match evidence.get(&planned_pr_id) {
            Some(item) => {
                let status = resolve_status(item);
                (
                    status,
                    item.evidence.clone(),
                    vec![reason_for_status(status)],
                )
            }
            None => (
                "not_started",
                Vec::new(),
                vec!["この planned PR に紐づく receipt / handoff がありません".to_string()],
            ),
        };
        states.push(EpicPrState {
            planned_pr_id,
            sequence,
            title,
            status: status.to_string(),
            evidence: evidence_links,
            reasons,
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
    let decision = decide_next(&states, &unresolved, &epic_id, &artifact_dir_display);

    // 6. epic run dir に 2 ファイルだけ書く（既存 receipt は不変）。
    let progress_value = epic_progress_state_value(&epic_id, now_unix, &states, &summary);
    let progress_path = artifact_dir.join("epic_progress_state.json");
    store.write_json(&progress_path, &progress_value)?;

    let decision_value = next_decision_value(&epic_id, &decision);
    let decision_path = artifact_dir.join("next_planned_pr_decision.json");
    store.write_json(&decision_path, &decision_value)?;

    Ok(EpicContinueResult {
        schema_version: "fda.epic_continue_result.v0",
        verdict: decision.verdict.to_string(),
        epic_id,
        artifact_dir: artifact_dir_display,
        artifacts_root: artifacts_root_display,
        progress_state_path: display_path(&repo_root, &progress_path),
        next_decision_path: display_path(&repo_root, &decision_path),
        next_planned_pr_id: decision.next_planned_pr_id.clone(),
        prs: states,
        summary,
        reasons: decision.reasons.clone(),
        resume_commands: decision.resume_commands.clone(),
        next_actions: decision.resume_commands,
    })
}

fn collect_pr_evidence(
    store: &impl ArtifactStore,
    artifacts_root: &Path,
) -> Result<BTreeMap<String, PrEvidence>, String> {
    let mut map: BTreeMap<String, PrEvidence> = BTreeMap::new();
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
        scan_run_receipts(store, name, &artifacts_root.join(name), &mut map)?;
    }
    Ok(map)
}

/// 1 run 分の receipt / handoff を読み、planned_pr_id ごとの PrEvidence に畳み込む。
/// 壊れた JSON は fail-closed（`?` で Err 伝播）で止める。状態推定の誤りは次 PR の
/// 誤選択につながるため、黙って not_started へ格下げしない。
fn scan_run_receipts(
    store: &impl ArtifactStore,
    run: &str,
    run_dir: &Path,
    map: &mut BTreeMap<String, PrEvidence>,
) -> Result<(), String> {
    // github_merge_receipt.json: merge 実行済み → merged。
    if let Some((pr_id, value)) =
        read_receipt_with_pr_id(store, run_dir, "github_merge_receipt.json")?
    {
        let status = value_string(&value, "status").unwrap_or_default();
        let merge_executed = value
            .get("merge_executed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let item = map.entry(pr_id).or_default();
        item.evidence
            .push(format!("{run}/github_merge_receipt.json (status={status})"));
        if status == "succeeded" || merge_executed {
            item.merged = true;
        }
    }

    // external_pr_receipt.json: merged / opened / blocked / rejected。
    if let Some((pr_id, value)) =
        read_receipt_with_pr_id(store, run_dir, "external_pr_receipt.json")?
    {
        let status = value_string(&value, "status").unwrap_or_default();
        let item = map.entry(pr_id).or_default();
        item.evidence
            .push(format!("{run}/external_pr_receipt.json (status={status})"));
        match status.as_str() {
            "merged" => item.merged = true,
            "opened" | "open" => item.pr_open = true,
            "blocked" | "rejected" => item.blocked = true,
            _ => {}
        }
    }

    // merge_receipt.json: merge_ready / human_approval → merge_ready、blocked → blocked。
    if let Some((pr_id, value)) = read_receipt_with_pr_id(store, run_dir, "merge_receipt.json")? {
        let status = value_string(&value, "status").unwrap_or_default();
        let item = map.entry(pr_id).or_default();
        item.evidence
            .push(format!("{run}/merge_receipt.json (status={status})"));
        match status.as_str() {
            "merged" => item.merged = true,
            "merge_ready" | "human_approval_required" | "human_approval_granted" => {
                item.merge_ready = true
            }
            "blocked" => item.blocked = true,
            _ => {}
        }
    }

    // handoff 系 artifact のみ（PR 未作成）→ in_progress。
    for file in [
        "current_codex_cli_handoff.json",
        "planned_pr_execution_packet.json",
    ] {
        if let Some((pr_id, _value)) = read_receipt_with_pr_id(store, run_dir, file)? {
            let item = map.entry(pr_id).or_default();
            item.evidence.push(format!("{run}/{file}"));
            item.in_progress = true;
        }
    }

    Ok(())
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

/// 証跡の優先順位で status を確定する。
/// merged > merge_ready > pr_open > blocked > in_progress > not_started。
/// 正の進捗（merged/merge_ready/pr_open）を、別 run の古い blocked receipt が覆い隠さない。
fn resolve_status(item: &PrEvidence) -> &'static str {
    if item.merged {
        "merged"
    } else if item.merge_ready {
        "merge_ready"
    } else if item.pr_open {
        "pr_open"
    } else if item.blocked {
        "blocked"
    } else if item.in_progress {
        "in_progress"
    } else {
        "not_started"
    }
}

fn reason_for_status(status: &str) -> String {
    match status {
        "merged" => "merge receipt があり merged です".to_string(),
        "merge_ready" => "merge gate を通過し merge_ready（人間の merge 承認待ち）です".to_string(),
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
            "merge_ready" => summary.waiting_human += 1,
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
///    - merge_ready / pr_open → waiting_human（人間の review / merge 承認待ち）。
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
                "次の未 merged PR {pr_id} は merge_ready です。merge は人間の承認が必要なため後続 PR はまだ選べません。"
            )],
            resume_commands: vec![format!(
                "{pr_id} の merge を人間が承認する: fda merge --artifacts <{pr_id} の run dir> --target-repo <target repo>"
            )],
        },
        "pr_open" => NextDecision {
            verdict: "waiting_human",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: vec![format!(
                "次の未 merged PR {pr_id} は pr_open です。review と merge（人間承認）が必要なため後続 PR はまだ選べません。"
            )],
            resume_commands: vec![format!(
                "{pr_id} の PR を review し人間が merge する: fda review --artifacts <{pr_id} の run dir> の後 fda merge"
            )],
        },
        _ => NextDecision {
            verdict: "blocked",
            next_planned_pr_id: Some(pr_id.clone()),
            reasons: vec![format!(
                "次の未 merged PR {pr_id} は blocked です。repair または receipt 確認が必要です。"
            )],
            resume_commands: vec![format!(
                "{pr_id} を repair する: fda continue --artifacts <{pr_id} の run dir> --target-repo <target repo>"
            )],
        },
    }
}

fn epic_progress_state_value(
    epic_id: &str,
    now_unix: u64,
    states: &[EpicPrState],
    summary: &EpicSummary,
) -> Value {
    json!({
        "schema_version": EPIC_PROGRESS_STATE_SCHEMA_VERSION,
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
    })
}

fn next_decision_value(epic_id: &str, decision: &NextDecision) -> Value {
    json!({
        "schema_version": NEXT_PLANNED_PR_DECISION_SCHEMA_VERSION,
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
            json!({"planned_pr_id": "PR-1", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-pr2",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-2", "status": "opened"}),
        );

        let result = run_epic(&repo);

        assert_eq!(status_of(&result, "PR-1"), "merged");
        assert_eq!(status_of(&result, "PR-2"), "pr_open");
        assert_eq!(status_of(&result, "PR-3"), "not_started");

        // 依存充足していない PR-3 は絶対に next に選ばれない（PR-2 が merged でないため）。
        assert_ne!(result.next_planned_pr_id.as_deref(), Some("PR-3"));
        assert_eq!(result.verdict, "waiting_human");
        assert_eq!(result.next_planned_pr_id.as_deref(), Some("PR-2"));
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
            json!({"planned_pr_id": "PR-1", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-pr2",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-2", "status": "merged"}),
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
            json!({"planned_pr_id": "PR-1", "status": "succeeded", "merge_executed": true}),
        );
        write_receipt(
            &repo,
            "run-pr2",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-2", "status": "merged"}),
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
            json!({"planned_pr_id": "PR-1", "status": "merge_ready"}),
        );
        // PR-2: in_progress (handoff のみ)
        write_receipt(
            &repo,
            "run-pr2",
            "current_codex_cli_handoff.json",
            json!({"planned_pr_id": "PR-2"}),
        );
        // PR-3: blocked (external_pr rejected)
        write_receipt(
            &repo,
            "run-pr3",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-3", "status": "rejected"}),
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
    fn writes_two_files_and_leaves_receipts_unchanged() {
        let repo = temp_repo("fda-epic-readonly");
        write_planned_prs(&repo, "EPIC-T6", &[("PR-1", 1, "one")]);
        write_receipt(
            &repo,
            "run-pr1",
            "external_pr_receipt.json",
            json!({"planned_pr_id": "PR-1", "status": "opened"}),
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
        let decision =
            read_json_value(&store, &epic_dir.join("next_planned_pr_decision.json")).unwrap();
        assert_eq!(
            decision.get("schema_version").and_then(Value::as_str),
            Some("fda.next_planned_pr_decision.v1")
        );

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
