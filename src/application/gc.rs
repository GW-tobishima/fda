//! F5 庭師: `fda gc` は artifacts/runs の stale / 不整合 run を棚卸しし、
//! 人間には例外だけを docket として提示する（read-only スキャン + docket 書き込みのみ）。
//!
//! **既存 run への変更・削除は一切しない。** 出力は `<artifacts-root>/_gc/gc_docket.{json,md}`。

use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, read_decision_receipts,
    read_json_value, recorded_decision_receipts_from_packet, value_string,
};
use crate::application::ports::ArtifactStore;
use crate::cli::args::GcConfig;
use crate::domain::policies::decision::decision_blockers;
use crate::infra::clock::system_unix_seconds;
use crate::infra::fs_store::{list_dir_names, modified_unix_seconds, FsArtifactStore};
use crate::support::paths::{display_path, resolve_path};

const GC_DOCKET_SCHEMA_VERSION: &str = "fda.gc_docket.v1";
const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GcCandidate {
    pub(crate) run: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) recommendation: String,
    pub(crate) needs_human: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct GcResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) artifacts_root: String,
    pub(crate) docket_path: String,
    pub(crate) docket_markdown_path: String,
    pub(crate) scanned_runs: usize,
    pub(crate) candidate_count: usize,
    pub(crate) candidates: Vec<GcCandidate>,
    pub(crate) next_actions: Vec<String>,
}

/// 1 run のスキャン結果（IO を分離した純データ）。
struct RunScanInput {
    run: String,
    mtime_unix: u64,
    has_completion_receipt: bool,
    has_validation_report: bool,
    ato_state_status: Option<String>,
    unresolved_decision_count: usize,
    /// 壊れた JSON 等の読取エラー。scan 全体を abort せず、この run を
    /// parse_error 候補として docket に報告する。
    parse_errors: Vec<String>,
}

pub(crate) fn gc(config: &GcConfig) -> Result<GcResult, String> {
    let store = FsArtifactStore;
    gc_with(config, &store, system_unix_seconds())
}

fn gc_with(
    config: &GcConfig,
    store: &impl ArtifactStore,
    now_unix: u64,
) -> Result<GcResult, String> {
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    let artifacts_root = resolve_path(&repo_root, &config.artifacts_root);
    let artifacts_root_display = display_path(&repo_root, &artifacts_root);

    let run_names = if store.exists(&artifacts_root) {
        list_dir_names(&artifacts_root)?
    } else {
        Vec::new()
    };

    let mut candidates = Vec::new();
    let mut scanned_runs = 0usize;
    for name in &run_names {
        // `_gc` などの内部ディレクトリはスキャン対象外。
        if name.starts_with('_') {
            continue;
        }
        scanned_runs += 1;
        let run_dir = artifacts_root.join(name);
        let input = scan_run(store, name, &run_dir)?;
        if let Some(candidate) = evaluate_run(&input, now_unix, config.max_age_days) {
            candidates.push(candidate);
        }
    }
    candidates.sort_by(|a, b| a.run.cmp(&b.run));

    // docket は既存 run を触らず `<artifacts-root>/_gc/` にだけ書く。
    let gc_dir = artifacts_root.join("_gc");
    store
        .create_dir_all(&gc_dir)
        .map_err(|e| format!("failed to create gc dir {}: {e}", gc_dir.display()))?;
    let docket_path = gc_dir.join("gc_docket.json");
    store.write_json(
        &docket_path,
        &gc_docket_value(now_unix, scanned_runs, &candidates),
    )?;
    let docket_markdown_path = gc_dir.join("gc_docket.md");
    store.write_text(
        &docket_markdown_path,
        &gc_docket_markdown(now_unix, scanned_runs, &candidates, &artifacts_root_display),
    )?;

    let candidate_count = candidates.len();
    Ok(GcResult {
        schema_version: "fda.gc_result.v0",
        verdict: "pass",
        artifacts_root: artifacts_root_display,
        docket_path: display_path(&repo_root, &docket_path),
        docket_markdown_path: display_path(&repo_root, &docket_markdown_path),
        scanned_runs,
        candidate_count,
        next_actions: gc_next_actions(&candidates),
        candidates,
    })
}

fn scan_run(
    store: &impl ArtifactStore,
    name: &str,
    run_dir: &Path,
) -> Result<RunScanInput, String> {
    let mtime_unix = modified_unix_seconds(run_dir)?;
    let has_completion_receipt = store.exists(&run_dir.join("merge_receipt.json"))
        || store.exists(&run_dir.join("end_to_end_receipt.json"));
    let has_validation_report = store.exists(&run_dir.join("validation_report.json"));
    // 壊れた JSON で scan 全体を abort しない: parse error はこの run の候補理由として
    // 記録し、残りの run のスキャンを継続する（fail-soft、docket で人間へ報告）。
    let mut parse_errors = Vec::new();
    let ato_state_status = match read_ato_state_status(store, run_dir) {
        Ok(status) => status,
        Err(error) => {
            parse_errors.push(format!("parse_error: ato_state_receipt.json ({error})"));
            None
        }
    };
    let unresolved_decision_count = match unresolved_decision_count(store, run_dir) {
        Ok(count) => count,
        Err(error) => {
            parse_errors.push(format!(
                "parse_error: human_decision_packet.json / decision_receipts.json ({error})"
            ));
            0
        }
    };
    Ok(RunScanInput {
        run: name.to_string(),
        mtime_unix,
        has_completion_receipt,
        has_validation_report,
        ato_state_status,
        unresolved_decision_count,
        parse_errors,
    })
}

/// 検出 4 種 + parse error を純ロジックで評価する。候補でなければ None。
///
/// (a) stale かつ未完了 → archive（要人間）
/// (b) validation_report.json 欠落 → resume
/// (c) ato_state_receipt.json status が succeeded 以外 → resume
/// (d) 未解決 decision かつ stale → answer_decision（要人間）
/// (e) 壊れた JSON（parse_error）→ resume（要人間）
fn evaluate_run(input: &RunScanInput, now_unix: u64, max_age_days: u64) -> Option<GcCandidate> {
    let threshold_seconds = max_age_days.saturating_mul(SECONDS_PER_DAY);
    let age_seconds = now_unix.saturating_sub(input.mtime_unix);
    let is_stale = age_seconds > threshold_seconds;

    let mut reasons = Vec::new();
    let mut recommendation = "resume";
    let mut needs_human = false;

    // (e) parse error は内容判定に使えないため人間へ報告する。
    if !input.parse_errors.is_empty() {
        reasons.extend(input.parse_errors.iter().cloned());
        needs_human = true;
    }

    // (a)
    if is_stale && !input.has_completion_receipt {
        reasons.push(format!(
            "mtime が {max_age_days} 日を超過（経過 {age_seconds} 秒）し、merge_receipt.json / end_to_end_receipt.json が無く未完了です"
        ));
        recommendation = "archive";
        needs_human = true;
    }

    // (b)
    if !input.has_validation_report {
        reasons.push("validation_report.json が欠落しています".to_string());
    }

    // (c)
    if let Some(status) = &input.ato_state_status {
        if status != "succeeded" {
            reasons.push(format!(
                "ato_state_receipt.json の status が succeeded 以外です（{status}）"
            ));
        }
    }

    // (d)
    if input.unresolved_decision_count > 0 && is_stale {
        reasons.push(format!(
            "未解決 Human Decision が {} 件あり mtime が {max_age_days} 日を超過しています",
            input.unresolved_decision_count
        ));
        recommendation = "answer_decision";
        needs_human = true;
    }

    if reasons.is_empty() {
        return None;
    }
    Some(GcCandidate {
        run: input.run.clone(),
        reasons,
        recommendation: recommendation.to_string(),
        needs_human,
    })
}

fn read_ato_state_status(
    store: &impl ArtifactStore,
    run_dir: &Path,
) -> Result<Option<String>, String> {
    let path = run_dir.join("ato_state_receipt.json");
    if !store.exists(&path) {
        return Ok(None);
    }
    let value = read_json_value(store, &path)?;
    Ok(value_string(&value, "status"))
}

fn unresolved_decision_count(store: &impl ArtifactStore, run_dir: &Path) -> Result<usize, String> {
    let packet_path = run_dir.join("human_decision_packet.json");
    if !store.exists(&packet_path) {
        return Ok(0);
    }
    let packet = read_json_value(store, &packet_path)?;
    let decisions = decision_summaries_from_packet(&packet);
    let mut receipts = read_decision_receipts(store, &run_dir.join("decision_receipts.json"))?;
    for (decision_id, receipt) in recorded_decision_receipts_from_packet(&packet) {
        receipts.entry(decision_id).or_insert(receipt);
    }
    Ok(decision_blockers(&decisions, &decision_answers_from_receipts(&receipts)).len())
}

fn gc_docket_value(now_unix: u64, scanned_runs: usize, candidates: &[GcCandidate]) -> Value {
    json!({
        "schema_version": GC_DOCKET_SCHEMA_VERSION,
        "generated_at_unix": now_unix,
        "scanned_runs": scanned_runs,
        "candidates": candidates
            .iter()
            .map(|candidate| json!({
                "run": candidate.run,
                "reasons": candidate.reasons,
                "recommendation": candidate.recommendation,
                "needs_human": candidate.needs_human,
            }))
            .collect::<Vec<_>>(),
    })
}

fn gc_docket_markdown(
    now_unix: u64,
    scanned_runs: usize,
    candidates: &[GcCandidate],
    artifacts_root_display: &str,
) -> String {
    let mut body = String::new();
    body.push_str("# GC Docket（棚卸し候補）\n\n");
    body.push_str(&format!("- artifacts-root: `{artifacts_root_display}`\n"));
    body.push_str(&format!("- generated_at_unix: {now_unix}\n"));
    body.push_str(&format!("- スキャンした run 数: {scanned_runs}\n"));
    body.push_str(&format!("- 棚卸し候補: {} 件\n\n", candidates.len()));
    body.push_str(
        "**fda gc は削除・変更を一切行いません。人間は例外（候補）だけを判断します。**\n\n",
    );

    if candidates.is_empty() {
        body.push_str("棚卸し候補はありません。\n");
        return body;
    }

    body.push_str("| run | recommendation | needs_human | reasons |\n");
    body.push_str("|---|---|---|---|\n");
    for candidate in candidates {
        let reasons = candidate
            .reasons
            .iter()
            .map(|reason| reason.replace('|', "\\|"))
            .collect::<Vec<_>>()
            .join("<br>");
        body.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            candidate.run,
            candidate.recommendation,
            if candidate.needs_human { "yes" } else { "no" },
            reasons
        ));
    }
    body
}

fn gc_next_actions(candidates: &[GcCandidate]) -> Vec<String> {
    if candidates.is_empty() {
        return vec!["棚卸し候補はありません。".to_string()];
    }
    let human = candidates.iter().filter(|c| c.needs_human).count();
    let mut actions = vec![format!(
        "gc_docket.md を確認する（候補 {} 件 / 要人間判断 {} 件）",
        candidates.len(),
        human
    )];
    if candidates
        .iter()
        .any(|c| c.recommendation == "answer_decision")
    {
        actions.push("未解決 Human Decision を fda decide で解決する".to_string());
    }
    if candidates.iter().any(|c| c.recommendation == "archive") {
        actions.push("stale 未完了 run を resume するか archive するか人間が判断する".to_string());
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = system_unix_seconds();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        FsArtifactStore.create_dir_all(&dir).unwrap();
        dir
    }

    fn base_input(run: &str) -> RunScanInput {
        RunScanInput {
            run: run.to_string(),
            mtime_unix: 0,
            has_completion_receipt: true,
            has_validation_report: true,
            ato_state_status: Some("succeeded".to_string()),
            unresolved_decision_count: 0,
            parse_errors: Vec::new(),
        }
    }

    #[test]
    fn detects_stale_incomplete_run() {
        let mut input = base_input("run-a");
        input.mtime_unix = 0;
        input.has_completion_receipt = false;
        // now = 100 days, max_age = 30 days -> stale
        let candidate = evaluate_run(&input, 100 * SECONDS_PER_DAY, 30).unwrap();
        assert_eq!(candidate.recommendation, "archive");
        assert!(candidate.needs_human);
    }

    #[test]
    fn detects_missing_validation_report() {
        let mut input = base_input("run-b");
        input.has_validation_report = false;
        // not stale (now == mtime) so (a)/(d) do not fire; only (b).
        let candidate = evaluate_run(&input, 0, 30).unwrap();
        assert_eq!(candidate.recommendation, "resume");
        assert!(!candidate.needs_human);
        assert!(candidate
            .reasons
            .iter()
            .any(|r| r.contains("validation_report.json")));
    }

    #[test]
    fn detects_failed_ato_state() {
        let mut input = base_input("run-c");
        input.ato_state_status = Some("failed".to_string());
        let candidate = evaluate_run(&input, 0, 30).unwrap();
        assert_eq!(candidate.recommendation, "resume");
        assert!(candidate
            .reasons
            .iter()
            .any(|r| r.contains("succeeded 以外")));
    }

    #[test]
    fn detects_stale_unresolved_decision() {
        let mut input = base_input("run-d");
        input.unresolved_decision_count = 2;
        let candidate = evaluate_run(&input, 100 * SECONDS_PER_DAY, 30).unwrap();
        assert_eq!(candidate.recommendation, "answer_decision");
        assert!(candidate.needs_human);
    }

    #[test]
    fn clean_run_is_not_a_candidate() {
        let input = base_input("run-clean");
        assert!(evaluate_run(&input, 100 * SECONDS_PER_DAY, 30).is_none());
    }

    #[test]
    fn parse_error_is_reported_without_aborting() {
        let mut input = base_input("run-broken");
        input.parse_errors = vec!["parse_error: ato_state_receipt.json (bad json)".to_string()];
        let candidate = evaluate_run(&input, 0, 30).unwrap();
        assert_eq!(candidate.recommendation, "resume");
        assert!(candidate.needs_human);
        assert!(candidate.reasons.iter().any(|r| r.contains("parse_error")));
    }

    #[test]
    fn broken_json_run_does_not_abort_scan_of_other_runs() {
        let store = FsArtifactStore;
        let repo = temp_dir("fda-gc-broken-json");
        let runs_root = repo.join("artifacts").join("runs");

        // 壊れた JSON を含む run（それ以外は完了扱い）。
        let broken = runs_root.join("run-broken");
        store.create_dir_all(&broken).unwrap();
        store
            .write_json(
                &broken.join("merge_receipt.json"),
                &json!({"status": "merge_ready"}),
            )
            .unwrap();
        store
            .write_json(
                &broken.join("validation_report.json"),
                &json!({"verdict": "pass"}),
            )
            .unwrap();
        store
            .write_text(&broken.join("ato_state_receipt.json"), "{ this is not json")
            .unwrap();

        // (c) を持つ通常の検出対象 run。
        let ato_failed = runs_root.join("run-ato-failed");
        store.create_dir_all(&ato_failed).unwrap();
        store
            .write_json(
                &ato_failed.join("merge_receipt.json"),
                &json!({"status": "merge_ready"}),
            )
            .unwrap();
        store
            .write_json(
                &ato_failed.join("validation_report.json"),
                &json!({"verdict": "pass"}),
            )
            .unwrap();
        store
            .write_json(
                &ato_failed.join("ato_state_receipt.json"),
                &json!({"status": "failed"}),
            )
            .unwrap();

        let config = GcConfig {
            repo_root: repo.clone(),
            artifacts_root: PathBuf::from("artifacts/runs"),
            max_age_days: 30,
            print_json: false,
        };
        let result = gc_with(&config, &store, system_unix_seconds()).unwrap();

        // scan は abort せず両 run を報告する。
        assert_eq!(result.scanned_runs, 2);
        let broken_candidate = result
            .candidates
            .iter()
            .find(|c| c.run == "run-broken")
            .expect("broken run must be a candidate");
        assert_eq!(broken_candidate.recommendation, "resume");
        assert!(broken_candidate.needs_human);
        assert!(broken_candidate
            .reasons
            .iter()
            .any(|r| r.contains("parse_error") && r.contains("ato_state_receipt.json")));
        assert!(result.candidates.iter().any(|c| c.run == "run-ato-failed"));
        // 壊れたファイル自体は変更されない。
        assert_eq!(
            store
                .read_text(&broken.join("ato_state_receipt.json"))
                .unwrap(),
            "{ this is not json"
        );
    }

    #[test]
    fn scan_writes_docket_and_leaves_existing_runs_unchanged() {
        let store = FsArtifactStore;
        let repo = temp_dir("fda-gc-scan");
        let runs_root = repo.join("artifacts").join("runs");

        // (a)+(b): stale incomplete, no validation
        let stale = runs_root.join("run-stale");
        store.create_dir_all(&stale).unwrap();
        store
            .write_text(&stale.join("sentinel.txt"), "keep-me")
            .unwrap();

        // (c): ato failed, but complete + validated
        let ato_failed = runs_root.join("run-ato-failed");
        store.create_dir_all(&ato_failed).unwrap();
        store
            .write_json(
                &ato_failed.join("merge_receipt.json"),
                &json!({"status": "merge_ready"}),
            )
            .unwrap();
        store
            .write_json(
                &ato_failed.join("validation_report.json"),
                &json!({"verdict": "pass"}),
            )
            .unwrap();
        store
            .write_json(
                &ato_failed.join("ato_state_receipt.json"),
                &json!({"status": "failed"}),
            )
            .unwrap();

        // (d): stale unresolved decision
        let decision = runs_root.join("run-decision");
        store.create_dir_all(&decision).unwrap();
        store
            .write_json(
                &decision.join("merge_receipt.json"),
                &json!({"status": "merge_ready"}),
            )
            .unwrap();
        store
            .write_json(
                &decision.join("validation_report.json"),
                &json!({"verdict": "pass"}),
            )
            .unwrap();
        store
            .write_json(
                &decision.join("human_decision_packet.json"),
                &json!({
                    "schema_version": "fda.human_decision_packet.v0",
                    "decisions": [{
                        "decision_id": "HD-GC-001",
                        "type": "spec_decision",
                        "summary": "未解決の判断",
                        "required_before": "Design Gate",
                        "options": [{"id": "yes"}, {"id": "no"}],
                        "recommended_option_id": "yes"
                    }]
                }),
            )
            .unwrap();

        // clean run: complete + validated + ato ok + no decision
        let clean = runs_root.join("run-clean");
        store.create_dir_all(&clean).unwrap();
        store
            .write_json(
                &clean.join("merge_receipt.json"),
                &json!({"status": "merge_ready"}),
            )
            .unwrap();
        store
            .write_json(
                &clean.join("validation_report.json"),
                &json!({"verdict": "pass"}),
            )
            .unwrap();
        store
            .write_json(
                &clean.join("ato_state_receipt.json"),
                &json!({"status": "succeeded"}),
            )
            .unwrap();

        let config = GcConfig {
            repo_root: repo.clone(),
            artifacts_root: PathBuf::from("artifacts/runs"),
            max_age_days: 30,
            print_json: false,
        };
        // 実 mtime + 100 日を now とし、fresh fixture でも stale 判定を決定的にする。
        let now = system_unix_seconds() + 100 * SECONDS_PER_DAY;
        let result = gc_with(&config, &store, now).unwrap();

        assert_eq!(result.verdict, "pass");
        assert_eq!(result.scanned_runs, 4);
        let runs: Vec<&str> = result.candidates.iter().map(|c| c.run.as_str()).collect();
        assert!(runs.contains(&"run-stale"));
        assert!(runs.contains(&"run-ato-failed"));
        assert!(runs.contains(&"run-decision"));
        assert!(!runs.contains(&"run-clean"));

        // docket が出力され、schema と一致する。
        let docket =
            read_json_value(&store, &runs_root.join("_gc").join("gc_docket.json")).unwrap();
        assert_eq!(
            docket.get("schema_version").and_then(Value::as_str),
            Some("fda.gc_docket.v1")
        );
        assert!(runs_root.join("_gc").join("gc_docket.md").exists());

        // 既存 run は不変（sentinel の中身が保持され、削除もされていない）。
        assert_eq!(
            store.read_text(&stale.join("sentinel.txt")).unwrap(),
            "keep-me"
        );

        // 2 回目のスキャンでも _gc はスキャン対象にならない。
        let result2 = gc_with(&config, &store, now).unwrap();
        assert_eq!(result2.scanned_runs, 4);
    }
}
