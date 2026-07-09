use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::application::decisions::{
    decision_summaries_from_packet, decision_type_from_packet, read_decision_receipts,
    read_json_value, recorded_decision_receipts_from_packet, value_string,
};
use crate::application::policy::{
    applicable_rule_ids, normalize_summary_signature, read_delegation_contract_rules,
};
use crate::application::ports::{ArtifactStore, AtoConfig};
use crate::application::status;
use crate::cli::args::StatusConfig;
use crate::infra::clock::system_unix_seconds;
use crate::infra::fs_store::{list_dir_names, list_file_names, FsArtifactStore};
use crate::infra::yaml::SerdeYamlValidator;
use crate::support::date::unix_seconds_to_ymd;
use crate::support::paths::resolve_path;

/// 道場（decision journal）に載せる回答済み判断の上限。新しい順に切り取る。
const DECISION_JOURNAL_LIMIT: usize = 50;
/// Decision Inbox の 1 判断あたりに添付する precedent（過去の同型判断）の上限。
const MAX_PRECEDENTS_PER_DECISION: usize = 3;
/// 正規化署名の類似判定に使う共通接頭辞の最小長（これ未満は「似ていない」）。
const SIGNATURE_SIMILARITY_MIN_PREFIX: usize = 6;

/// `fda ui` の設定。infra の HTTP server からも参照するため
/// cli::args ではなく application 層に置く（infra は crate::cli を import できない）。
pub(crate) struct UiConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) runs_root: PathBuf,
    pub(crate) port: u16,
    pub(crate) open_browser: bool,
    pub(crate) print_json: bool,
}

/// Mission Control スナップショット。
///
/// run ごとの真実は `application::status`（`fda status` と同一ロジック）から取り、
/// UI 専用の phase 再実装をしない。読み取り専用で、何も書き込まない。
pub(crate) fn mission_control_snapshot(config: &UiConfig) -> Result<Value, String> {
    let store = FsArtifactStore;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    let runs_root = resolve_path(&repo_root, &config.runs_root);
    let runs_root_display = config.runs_root.to_string_lossy().replace('\\', "/");

    let mut run_names = if store.exists(&runs_root) {
        list_dir_names(&runs_root)?
    } else {
        Vec::new()
    };
    // run ディレクトリ名は fda-start-<unix秒> 形式が既定なので、辞書順逆順 ≒ 新しい順。
    run_names.sort();
    run_names.reverse();

    // 道場（判断の振り返り）用に、全 run の回答済み判断とその後の帰結を先に収集する。
    // これは Decision Inbox の precedent 照合にも使う（同一コーパスを二度読まない）。
    let decision_records =
        collect_decision_records(&store, &runs_root, &run_names, &runs_root_display);

    // 未解決判断に適用可能な delegation contract を precedent に添える（自動適用はしない）。
    // read-only projection のため契約 YAML が壊れていてもヒント無しへ degrade する（fail-soft）。
    let yaml = SerdeYamlValidator;
    let contract_rules =
        read_delegation_contract_rules(&store, &yaml, &repo_root).unwrap_or_default();
    let today = unix_seconds_to_ymd(system_unix_seconds());

    // (表示優先度, run名) でソートする。判断待ち → repair → エラー → 前進可能 → その他 → 完了。
    let mut prioritized_runs: Vec<(u8, String, Value)> = Vec::new();
    let mut decision_inbox = Vec::new();
    let mut repair_lane = Vec::new();
    let mut open_decisions = 0usize;
    let mut repair_count = 0usize;
    let mut merge_ready_count = 0usize;

    for name in &run_names {
        let artifact_dir = runs_root.join(name);
        let run_rel = format!("{runs_root_display}/{name}");
        let status_config = StatusConfig {
            repo_root: repo_root.clone(),
            artifact_dir: artifact_dir.clone(),
            ato: AtoConfig::default(),
            print_json: false,
        };
        match status::status(&status_config) {
            Ok(result) => {
                let value = serde_json::to_value(&result).map_err(|e| e.to_string())?;
                // 未解決判断がある run だけ packet を読み、各判断の type を得て precedent 照合する。
                let packet = if result.unresolved_decisions.is_empty() {
                    None
                } else {
                    optional_json_soft(&store, &artifact_dir.join("human_decision_packet.json"))
                };
                for decision in &result.unresolved_decisions {
                    open_decisions += 1;
                    let decision_type = packet
                        .as_ref()
                        .and_then(|packet| decision_type_from_packet(packet, &decision.decision_id))
                        .unwrap_or_default();
                    let signature = normalize_summary_signature(&decision.summary);
                    let precedents = find_precedents(
                        &decision_records,
                        name,
                        &decision.decision_id,
                        &decision_type,
                        &signature,
                    );
                    let applicable_contract = applicable_contract_for_decision(
                        &contract_rules,
                        &decision_type,
                        &decision.summary,
                        &today,
                        &decision.decision_id,
                        &run_rel,
                    );
                    decision_inbox.push(json!({
                        "run": name,
                        "run_dir": run_rel,
                        "decision_id": decision.decision_id,
                        "type": decision_type,
                        "summary": decision.summary,
                        "required_before": decision.required_before,
                        "recommended_option_id": decision.recommended_option_id,
                        "option_ids": decision.option_ids,
                        "resume_command": format!(
                            "fda decide {} --answer <answer> --artifacts {run_rel}",
                            decision.decision_id
                        ),
                        "precedents": precedents,
                        "applicable_contract": applicable_contract,
                    }));
                }
                let repair_status = result.repair.repair_loop_status.as_str();
                let qa_failed = result.qa.qa_status == "failed";
                if repair_status == "repair_planned" || qa_failed {
                    repair_count += 1;
                    repair_lane.push(json!({
                        "run": name,
                        "run_dir": run_rel,
                        "repair_loop_status": repair_status,
                        "failure_classification": result.repair.failure_classification,
                        "retry_attempt_count": result.repair.retry_attempt_count,
                        "retry_limit": result.repair.retry_limit,
                        "qa_status": result.qa.qa_status,
                        "return_to_role": result.qa.return_to_role,
                        "next_action": result.next_actions.first(),
                    }));
                }
                if result.merge.merge_gate_status == "merge_ready" {
                    merge_ready_count += 1;
                }
                let priority = run_priority(
                    &result.current_phase,
                    repair_status,
                    qa_failed,
                    &result.merge.merge_gate_status,
                );
                let artifacts = list_file_names(&artifact_dir).unwrap_or_default();
                prioritized_runs.push((
                    priority,
                    name.clone(),
                    json!({
                        "run": name,
                        "run_dir": run_rel,
                        "status": value,
                        "artifacts": artifacts,
                    }),
                ));
            }
            Err(error) => {
                prioritized_runs.push((
                    2,
                    name.clone(),
                    json!({
                        "run": name,
                        "run_dir": run_rel,
                        "error": error,
                    }),
                ));
            }
        }
    }
    prioritized_runs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    let runs: Vec<Value> = prioritized_runs
        .into_iter()
        .map(|(_, _, run)| run)
        .collect();

    // 道場: 回答済み判断を新しい順に上限まで。庭師 / Epic は既存 artifact の投影。
    let decision_journal = decision_journal(&decision_records);
    let gc_docket = build_gc_docket(&store, &runs_root);
    let epic_progress = build_epic_progress(&store, &runs_root, &run_names);

    Ok(json!({
        "schema_version": "fda.mission_control_snapshot.v0",
        "role": "read_only_projection",
        "generated_at_unix": system_unix_seconds(),
        "repo_root": display_root(&repo_root),
        "runs_root": runs_root_display,
        "summary": {
            "run_count": run_names.len(),
            "open_decisions": open_decisions,
            "repair_count": repair_count,
            "merge_ready_count": merge_ready_count,
        },
        "decision_inbox": decision_inbox,
        "repair_lane": repair_lane,
        "runs": runs,
        "decision_journal": decision_journal,
        "gc_docket": gc_docket,
        "epic_progress": epic_progress,
    }))
}

/// 1 run の帰結（回答済み判断の「その後」）。同 run の全判断で共有する。
#[derive(Clone)]
struct RunOutcome {
    merge_gate_status: String,
    merge_verdict: String,
    repair_occurred: bool,
    qa_status: String,
    /// UI バッジ用の 1 語ラベル: merged / merge_ready / blocked / repair / pending。
    label: String,
}

/// 回答済み判断 1 件 + その run の帰結（道場 / precedent 双方の素データ）。
struct DecisionRecord {
    run: String,
    run_dir: String,
    decision_id: String,
    decision_type: String,
    summary: String,
    signature: String,
    answer: String,
    decided_by: String,
    contract_rule_id: Option<String>,
    decided_at_unix: u64,
    outcome: RunOutcome,
}

/// 存在すれば JSON を読む。壊れていても Err にせず None（read-only projection の fail-soft）。
fn optional_json_soft(store: &impl ArtifactStore, path: &Path) -> Option<Value> {
    if !store.exists(path) {
        return None;
    }
    read_json_value(store, path).ok()
}

/// 同 run の receipt から「回答済み判断のその後」を組み立てる。
fn run_outcome(store: &impl ArtifactStore, run_dir: &Path) -> RunOutcome {
    let merge = optional_json_soft(store, &run_dir.join("merge_receipt.json"));
    let merge_gate_status = merge
        .as_ref()
        .and_then(|merge| value_string(merge, "merge_gate_status"))
        .or_else(|| {
            merge
                .as_ref()
                .and_then(|merge| value_string(merge, "status"))
        })
        .unwrap_or_else(|| "missing".to_string());
    let merge_verdict = merge
        .as_ref()
        .and_then(|merge| value_string(merge, "verdict"))
        .unwrap_or_else(|| "-".to_string());
    let github_merged = optional_json_soft(store, &run_dir.join("github_merge_receipt.json"))
        .map(|receipt| {
            value_string(&receipt, "status").as_deref() == Some("succeeded")
                || receipt.get("merge_executed").and_then(Value::as_bool) == Some(true)
        })
        .unwrap_or(false);
    let repair_occurred = store.exists(&run_dir.join("repair_receipt.json"));
    let qa_status = optional_json_soft(store, &run_dir.join("qa_receipt.json"))
        .as_ref()
        .and_then(|qa| value_string(qa, "status"))
        .unwrap_or_else(|| "missing".to_string());

    let merged = github_merged || merge_gate_status == "merged" || merge_verdict == "merged";
    let label = if merged {
        "merged"
    } else if qa_status == "failed" || merge_verdict == "fail" {
        "blocked"
    } else if repair_occurred {
        "repair"
    } else if merge_gate_status == "merge_ready" {
        "merge_ready"
    } else {
        "pending"
    }
    .to_string();

    RunOutcome {
        merge_gate_status,
        merge_verdict,
        repair_occurred,
        qa_status,
        label,
    }
}

/// 全 run を走査し、回答済み判断（decision_receipts に answer があるもの）を収集する。
/// packet / receipt が壊れている run は fail-soft でスキップする（主 run ループが別途エラー表示）。
fn collect_decision_records(
    store: &impl ArtifactStore,
    runs_root: &Path,
    run_names: &[String],
    runs_root_display: &str,
) -> Vec<DecisionRecord> {
    let mut records = Vec::new();
    for name in run_names {
        // `_gc` / `_policy` などの内部ディレクトリは対象外。
        if name.starts_with('_') {
            continue;
        }
        let run_dir = runs_root.join(name);
        let Some(packet) = optional_json_soft(store, &run_dir.join("human_decision_packet.json"))
        else {
            continue;
        };
        let mut receipts = read_decision_receipts(store, &run_dir.join("decision_receipts.json"))
            .unwrap_or_default();
        for (decision_id, receipt) in recorded_decision_receipts_from_packet(&packet) {
            receipts.entry(decision_id).or_insert(receipt);
        }
        if receipts.is_empty() {
            continue;
        }
        let outcome = run_outcome(store, &run_dir);
        let run_rel = format!("{runs_root_display}/{name}");
        for decision in decision_summaries_from_packet(&packet) {
            let Some(receipt) = receipts.get(&decision.decision_id).or_else(|| {
                decision
                    .alias_ids
                    .iter()
                    .find_map(|alias| receipts.get(alias))
            }) else {
                continue;
            };
            let Some(answer) = value_string(receipt, "answer") else {
                continue;
            };
            let decision_type =
                decision_type_from_packet(&packet, &decision.decision_id).unwrap_or_default();
            let decided_by = value_string(receipt, "decided_by").unwrap_or_else(|| "-".to_string());
            let contract_rule_id = value_string(receipt, "contract_rule_id");
            let decided_at_unix = value_string(receipt, "decided_at_unix_seconds")
                .and_then(|raw| raw.parse::<u64>().ok())
                .unwrap_or(0);
            let signature = normalize_summary_signature(&decision.summary);
            records.push(DecisionRecord {
                run: name.clone(),
                run_dir: run_rel.clone(),
                decision_id: decision.decision_id.clone(),
                decision_type,
                summary: decision.summary.clone(),
                signature,
                answer,
                decided_by,
                contract_rule_id,
                decided_at_unix,
                outcome: outcome.clone(),
            });
        }
    }
    records
}

/// 道場テーブル用に、回答済み判断を新しい順（decided_at → run 名）に上限まで投影する。
fn decision_journal(records: &[DecisionRecord]) -> Vec<Value> {
    let mut ordered: Vec<&DecisionRecord> = records.iter().collect();
    ordered.sort_by(|a, b| {
        b.decided_at_unix
            .cmp(&a.decided_at_unix)
            .then_with(|| b.run.cmp(&a.run))
            .then_with(|| a.decision_id.cmp(&b.decision_id))
    });
    ordered
        .into_iter()
        .take(DECISION_JOURNAL_LIMIT)
        .map(|record| {
            json!({
                "run": record.run,
                "run_dir": record.run_dir,
                "decision_id": record.decision_id,
                "type": record.decision_type,
                "summary": record.summary,
                "answer": record.answer,
                "decided_by": record.decided_by,
                "contract_rule_id": record.contract_rule_id,
                "decided_at_unix": record.decided_at_unix,
                "outcome": outcome_value(&record.outcome),
            })
        })
        .collect()
}

fn outcome_value(outcome: &RunOutcome) -> Value {
    json!({
        "label": outcome.label,
        "merge_gate_status": outcome.merge_gate_status,
        "merge_verdict": outcome.merge_verdict,
        "repair_occurred": outcome.repair_occurred,
        "qa_status": outcome.qa_status,
    })
}

/// Decision Inbox の 1 判断に対する precedent（過去の同 type + 署名類似の判断）を最大 3 件返す。
/// 自分自身（同 run + 同 decision_id）は除外する。
fn find_precedents(
    records: &[DecisionRecord],
    current_run: &str,
    current_decision_id: &str,
    decision_type: &str,
    signature: &str,
) -> Vec<Value> {
    let mut matches: Vec<&DecisionRecord> = records
        .iter()
        .filter(|record| {
            !(record.run == current_run && record.decision_id == current_decision_id)
                && !record.decision_type.is_empty()
                && record.decision_type == decision_type
                && signatures_similar(&record.signature, signature)
        })
        .collect();
    matches.sort_by(|a, b| {
        b.decided_at_unix
            .cmp(&a.decided_at_unix)
            .then_with(|| b.run.cmp(&a.run))
            .then_with(|| a.decision_id.cmp(&b.decision_id))
    });
    matches
        .into_iter()
        .take(MAX_PRECEDENTS_PER_DECISION)
        .map(|record| {
            json!({
                "run": record.run,
                "decision_id": record.decision_id,
                "answer": record.answer,
                "decided_by": record.decided_by,
                "outcome": record.outcome.label,
            })
        })
        .collect()
}

/// 正規化署名の類似判定: 完全一致、または共通接頭辞が閾値以上なら「似ている」。
fn signatures_similar(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let common_prefix = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    common_prefix >= SIGNATURE_SIMILARITY_MIN_PREFIX
}

/// 未解決判断に適用可能な delegation contract があれば `{rule_id, resume_command}` を返す。
/// 自動適用はしない（人間が `--by-contract` を明示実行する resume command を提示するだけ）。
fn applicable_contract_for_decision(
    rules: &[Value],
    decision_type: &str,
    summary: &str,
    today: &str,
    decision_id: &str,
    run_rel: &str,
) -> Value {
    let rule_ids = applicable_rule_ids(rules, decision_type, summary, today);
    match rule_ids.first() {
        Some(rule_id) => json!({
            "rule_id": rule_id,
            "resume_command": format!(
                "fda decide {decision_id} --by-contract {rule_id} --artifacts {run_rel}"
            ),
        }),
        None => Value::Null,
    }
}

/// 庭師 docket（`<runs_root>/_gc/gc_docket.json`）を投影する。無ければ Null。
fn build_gc_docket(store: &impl ArtifactStore, runs_root: &Path) -> Value {
    let Some(docket) = optional_json_soft(store, &runs_root.join("_gc").join("gc_docket.json"))
    else {
        return Value::Null;
    };
    let candidates = docket
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let needs_human_count = candidates
        .iter()
        .filter(|candidate| candidate.get("needs_human").and_then(Value::as_bool) == Some(true))
        .count();
    json!({
        "generated_at_unix": docket.get("generated_at_unix").cloned().unwrap_or(Value::Null),
        "scanned_runs": docket.get("scanned_runs").cloned().unwrap_or(Value::Null),
        "summary": {
            "candidate_count": candidates.len(),
            "needs_human_count": needs_human_count,
        },
        "candidates": candidates,
    })
}

/// Epic 進捗（各 run dir の `epic_progress_state.json`）のうち最新 1 件を投影する。無ければ Null。
fn build_epic_progress(
    store: &impl ArtifactStore,
    runs_root: &Path,
    run_names: &[String],
) -> Value {
    let mut best: Option<(u64, Value)> = None;
    for name in run_names {
        if name.starts_with('_') {
            continue;
        }
        let Some(state) = optional_json_soft(
            store,
            &runs_root.join(name).join("epic_progress_state.json"),
        ) else {
            continue;
        };
        let generated = state
            .get("generated_at_unix")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if best.as_ref().map(|(g, _)| generated >= *g).unwrap_or(true) {
            best = Some((generated, state));
        }
    }
    match best {
        Some((_, state)) => json!({
            "epic_id": state.get("epic_id").cloned().unwrap_or(Value::Null),
            "generated_at_unix": state.get("generated_at_unix").cloned().unwrap_or(Value::Null),
            "prs": state.get("prs").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "summary": state.get("summary").cloned().unwrap_or(Value::Null),
        }),
        None => Value::Null,
    }
}

fn run_priority(phase: &str, repair_status: &str, qa_failed: bool, merge_gate: &str) -> u8 {
    if phase == "human_turn" || phase == "waiting_for_decision" {
        return 0;
    }
    if repair_status == "repair_planned" || qa_failed {
        return 1;
    }
    if phase.starts_with("ready_for_") || merge_gate == "merge_ready" {
        return 3;
    }
    if phase == "merged"
        || phase == "operational_v1_complete"
        || merge_gate == "merged"
        || phase.ends_with("_complete")
    {
        return 5;
    }
    4
}

fn display_root(path: &std::path::Path) -> String {
    // Windows の fs::canonicalize は \\?\ verbatim prefix を返すため表示用に外す。
    let text = path.to_string_lossy().to_string();
    let stripped = text.strip_prefix(r"\\?\").unwrap_or(&text);
    stripped.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = system_unix_seconds();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        FsArtifactStore.create_dir_all(&dir).unwrap();
        dir
    }

    fn write_json(path: &Path, value: &Value) {
        FsArtifactStore.write_json(path, value).unwrap();
    }

    #[test]
    fn snapshot_collects_runs_decisions_and_summary() {
        let repo = temp_dir("fda-ui-snapshot");
        let runs_root = repo.join("artifacts").join("runs");
        let run_dir = runs_root.join("fda-start-100");
        FsArtifactStore.create_dir_all(&run_dir).unwrap();
        write_json(
            &run_dir.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "decisions": [{
                    "decision_id": "HD-FDA-001",
                    "type": "spec_decision",
                    "summary": "scopeを固定してよいか",
                    "required_before": "Design Gate",
                    "options": [{"id": "yes"}, {"id": "no"}],
                    "recommended_option_id": "yes"
                }]
            }),
        );

        let config = UiConfig {
            repo_root: repo.clone(),
            runs_root: PathBuf::from("artifacts/runs"),
            port: 0,
            open_browser: false,
            print_json: false,
        };
        let snapshot = mission_control_snapshot(&config).unwrap();

        assert_eq!(
            snapshot["schema_version"],
            "fda.mission_control_snapshot.v0"
        );
        assert_eq!(snapshot["summary"]["run_count"], 1);
        assert_eq!(snapshot["summary"]["open_decisions"], 1);
        let inbox = snapshot["decision_inbox"].as_array().unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0]["decision_id"], "HD-FDA-001");
        assert_eq!(
            inbox[0]["resume_command"],
            "fda decide HD-FDA-001 --answer <answer> --artifacts artifacts/runs/fda-start-100"
        );
        let runs = snapshot["runs"].as_array().unwrap();
        assert_eq!(runs[0]["run"], "fda-start-100");
        assert_eq!(runs[0]["status"]["current_phase"], "human_turn");
    }

    #[test]
    fn snapshot_with_missing_runs_root_is_empty_not_error() {
        let repo = temp_dir("fda-ui-empty");
        let config = UiConfig {
            repo_root: repo.clone(),
            runs_root: PathBuf::from("artifacts/runs"),
            port: 0,
            open_browser: false,
            print_json: false,
        };
        let snapshot = mission_control_snapshot(&config).unwrap();
        assert_eq!(snapshot["summary"]["run_count"], 0);
        assert!(snapshot["runs"].as_array().unwrap().is_empty());
        // 追加フィールドは不在時に Null / 空へ degrade する（read-only projection）。
        assert!(snapshot["decision_journal"].as_array().unwrap().is_empty());
        assert!(snapshot["gc_docket"].is_null());
        assert!(snapshot["epic_progress"].is_null());
    }

    fn config_for(repo: &Path) -> UiConfig {
        UiConfig {
            repo_root: repo.to_path_buf(),
            runs_root: PathBuf::from("artifacts/runs"),
            port: 0,
            open_browser: false,
            print_json: false,
        }
    }

    /// 回答済み判断を 1 件持つ run を書く（packet の decisions[] + decision_receipts）。
    fn write_answered_run(
        runs_root: &Path,
        run: &str,
        decision_id: &str,
        decision_type: &str,
        summary: &str,
        answer: &str,
        decided_by: &str,
        decided_at: u64,
    ) {
        let run_dir = runs_root.join(run);
        FsArtifactStore.create_dir_all(&run_dir).unwrap();
        write_json(
            &run_dir.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "resolved",
                "decisions": [{
                    "decision_id": decision_id,
                    "type": decision_type,
                    "summary": summary,
                    "required_before": "Design Gate",
                    "options": [{"id": "yes"}, {"id": "no"}],
                    "recommended_option_id": "yes"
                }]
            }),
        );
        write_json(
            &run_dir.join("decision_receipts.json"),
            &json!({
                "schema_version": "fda.decision_receipts.v0",
                "receipts": [{
                    "decision_id": decision_id,
                    "answer": answer,
                    "decided_by": decided_by,
                    "decided_at_unix_seconds": decided_at.to_string()
                }]
            }),
        );
    }

    /// 未解決判断を 1 件持つ run を書く（receipt 無し）。
    fn write_unresolved_run(
        runs_root: &Path,
        run: &str,
        decision_id: &str,
        decision_type: &str,
        summary: &str,
    ) {
        let run_dir = runs_root.join(run);
        FsArtifactStore.create_dir_all(&run_dir).unwrap();
        write_json(
            &run_dir.join("human_decision_packet.json"),
            &json!({
                "schema_version": "fda.human_decision_packet.v0",
                "status": "waiting_human",
                "decisions": [{
                    "decision_id": decision_id,
                    "type": decision_type,
                    "summary": summary,
                    "required_before": "Design Gate",
                    "options": [{"id": "yes"}, {"id": "no"}],
                    "recommended_option_id": "yes"
                }]
            }),
        );
    }

    #[test]
    fn journal_links_answered_decision_to_merge_outcome() {
        let repo = temp_dir("fda-ui-journal");
        let runs_root = repo.join("artifacts").join("runs");
        write_answered_run(
            &runs_root,
            "fda-start-200",
            "HD-A-001",
            "spec_decision",
            "Scope In / Scope Out を Intake 正本として固定してよいか",
            "approve_scope",
            "human",
            1000,
        );
        // 同 run の帰結: merge 済み。
        write_json(
            &runs_root.join("fda-start-200").join("merge_receipt.json"),
            &json!({"merge_gate_status": "merged", "verdict": "merged", "status": "merged"}),
        );

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        let journal = snapshot["decision_journal"].as_array().unwrap();
        assert_eq!(journal.len(), 1);
        assert_eq!(journal[0]["decision_id"], "HD-A-001");
        assert_eq!(journal[0]["answer"], "approve_scope");
        assert_eq!(journal[0]["decided_by"], "human");
        assert_eq!(journal[0]["type"], "spec_decision");
        assert_eq!(journal[0]["outcome"]["label"], "merged");
        assert_eq!(journal[0]["outcome"]["merge_gate_status"], "merged");
    }

    #[test]
    fn journal_orders_newest_first_and_marks_repair_outcome() {
        let repo = temp_dir("fda-ui-journal-order");
        let runs_root = repo.join("artifacts").join("runs");
        write_answered_run(
            &runs_root,
            "fda-start-100",
            "HD-OLD-001",
            "spec_decision",
            "古い判断",
            "yes",
            "human",
            100,
        );
        write_answered_run(
            &runs_root,
            "fda-start-300",
            "HD-NEW-001",
            "risk_decision",
            "新しい判断",
            "no",
            "human",
            300,
        );
        // 新しい run では repair が発生していた（帰結バッジ = repair）。
        write_json(
            &runs_root.join("fda-start-300").join("repair_receipt.json"),
            &json!({"status": "repair_planned"}),
        );

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        let journal = snapshot["decision_journal"].as_array().unwrap();
        assert_eq!(journal.len(), 2);
        // decided_at の新しい順。
        assert_eq!(journal[0]["decision_id"], "HD-NEW-001");
        assert_eq!(journal[0]["outcome"]["label"], "repair");
        assert_eq!(journal[0]["outcome"]["repair_occurred"], true);
        assert_eq!(journal[1]["decision_id"], "HD-OLD-001");
        assert_eq!(journal[1]["outcome"]["label"], "pending");
    }

    #[test]
    fn precedent_attaches_same_type_matches_capped_at_three() {
        let repo = temp_dir("fda-ui-precedent");
        let runs_root = repo.join("artifacts").join("runs");
        let summary = "入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか";
        // 過去に同 type + 同署名の回答済み判断が 4 件。
        for (idx, run) in ["run-p1", "run-p2", "run-p3", "run-p4"].iter().enumerate() {
            write_answered_run(
                &runs_root,
                run,
                "HD-PAST-001",
                "spec_decision",
                summary,
                "approve_scope",
                "human",
                (idx as u64) + 1,
            );
        }
        // 別 type の判断（precedent に混ざってはいけない）。
        write_answered_run(
            &runs_root,
            "run-other",
            "HD-OTHER-001",
            "risk_decision",
            summary,
            "accept_risk",
            "human",
            9,
        );
        // 現在の未解決判断（同 type + 同署名）。
        write_unresolved_run(
            &runs_root,
            "fda-start-999",
            "HD-NOW-001",
            "spec_decision",
            summary,
        );

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        let inbox = snapshot["decision_inbox"].as_array().unwrap();
        let now = inbox
            .iter()
            .find(|entry| entry["decision_id"] == "HD-NOW-001")
            .expect("current decision must be in inbox");
        let precedents = now["precedents"].as_array().unwrap();
        assert_eq!(precedents.len(), MAX_PRECEDENTS_PER_DECISION);
        for precedent in precedents {
            assert_eq!(precedent["decision_id"], "HD-PAST-001");
            assert_eq!(precedent["answer"], "approve_scope");
            assert!(precedent["run"].as_str().unwrap().starts_with("run-p"));
        }
    }

    #[test]
    fn applicable_contract_present_when_rule_matches() {
        let repo = temp_dir("fda-ui-contract-ok");
        let runs_root = repo.join("artifacts").join("runs");
        let summary = "入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか";
        write_unresolved_run(
            &runs_root,
            "fda-start-500",
            "HD-C-001",
            "spec_decision",
            summary,
        );
        // 有効な delegation contract（expires は十分未来）。
        FsArtifactStore.create_dir_all(&repo.join(".fda")).unwrap();
        FsArtifactStore
            .write_text(
                &repo.join(".fda").join("delegation_contract.yaml"),
                "delegation_contract:\n  - rule_id: DC-001\n    decision_type: spec_decision\n    match_summary_keywords:\n      - \"Scope In / Scope Out\"\n    answer: approve_scope\n    authority: k_tobishima\n    enacted_from:\n      - \"run HD-C-001\"\n    expires: \"2099-01-01\"\n",
            )
            .unwrap();

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        let inbox = snapshot["decision_inbox"].as_array().unwrap();
        let entry = inbox
            .iter()
            .find(|entry| entry["decision_id"] == "HD-C-001")
            .expect("decision must be in inbox");
        assert_eq!(entry["applicable_contract"]["rule_id"], "DC-001");
        assert_eq!(
            entry["applicable_contract"]["resume_command"],
            "fda decide HD-C-001 --by-contract DC-001 --artifacts artifacts/runs/fda-start-500"
        );
    }

    #[test]
    fn applicable_contract_is_null_when_contract_yaml_broken() {
        let repo = temp_dir("fda-ui-contract-broken");
        let runs_root = repo.join("artifacts").join("runs");
        write_unresolved_run(
            &runs_root,
            "fda-start-600",
            "HD-B-001",
            "spec_decision",
            "Scope In / Scope Out を固定してよいか",
        );
        // 壊れた YAML でもクラッシュせずヒント無しへ degrade する（fail-soft）。
        FsArtifactStore.create_dir_all(&repo.join(".fda")).unwrap();
        FsArtifactStore
            .write_text(
                &repo.join(".fda").join("delegation_contract.yaml"),
                "delegation_contract: [ this is : not valid yaml",
            )
            .unwrap();

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        let inbox = snapshot["decision_inbox"].as_array().unwrap();
        let entry = inbox
            .iter()
            .find(|entry| entry["decision_id"] == "HD-B-001")
            .expect("decision must be in inbox");
        assert!(entry["applicable_contract"].is_null());
    }

    #[test]
    fn gc_and_epic_snapshot_included_when_present() {
        let repo = temp_dir("fda-ui-gc-epic");
        let runs_root = repo.join("artifacts").join("runs");
        FsArtifactStore
            .create_dir_all(&runs_root.join("_gc"))
            .unwrap();
        FsArtifactStore
            .create_dir_all(&runs_root.join("epic-run"))
            .unwrap();
        // gc docket。
        write_json(
            &runs_root.join("_gc").join("gc_docket.json"),
            &json!({
                "schema_version": "fda.gc_docket.v1",
                "generated_at_unix": 1783555200u64,
                "scanned_runs": 3,
                "candidates": [
                    {"run": "run-a", "reasons": ["stale"], "recommendation": "archive", "needs_human": true},
                    {"run": "run-b", "reasons": ["no validation"], "recommendation": "resume", "needs_human": false}
                ]
            }),
        );
        // epic progress（run dir 配下）。
        write_json(
            &runs_root.join("epic-run").join("epic_progress_state.json"),
            &json!({
                "schema_version": "fda.epic_progress_state.v1",
                "epic_id": "EPIC-FDA-V1-5",
                "generated_at_unix": 1783555200u64,
                "prs": [
                    {"planned_pr_id": "PR-V15-001", "sequence": 1, "title": "one", "status": "merged", "evidence": [], "reasons": []}
                ],
                "summary": {"merged": 1, "open": 0, "blocked": 0, "waiting_human": 0, "not_started": 0}
            }),
        );

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        assert_eq!(snapshot["gc_docket"]["summary"]["candidate_count"], 2);
        assert_eq!(snapshot["gc_docket"]["summary"]["needs_human_count"], 1);
        assert_eq!(
            snapshot["gc_docket"]["candidates"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(snapshot["epic_progress"]["epic_id"], "EPIC-FDA-V1-5");
        assert_eq!(
            snapshot["epic_progress"]["prs"].as_array().unwrap()[0]["status"],
            "merged"
        );
        assert_eq!(snapshot["epic_progress"]["summary"]["merged"], 1);
    }

    #[test]
    fn epic_progress_picks_latest_by_generated_at() {
        let repo = temp_dir("fda-ui-epic-latest");
        let runs_root = repo.join("artifacts").join("runs");
        FsArtifactStore
            .create_dir_all(&runs_root.join("epic-old"))
            .unwrap();
        FsArtifactStore
            .create_dir_all(&runs_root.join("epic-new"))
            .unwrap();
        write_json(
            &runs_root.join("epic-old").join("epic_progress_state.json"),
            &json!({"schema_version": "fda.epic_progress_state.v1", "epic_id": "OLD", "generated_at_unix": 100u64, "prs": [], "summary": {}}),
        );
        write_json(
            &runs_root.join("epic-new").join("epic_progress_state.json"),
            &json!({"schema_version": "fda.epic_progress_state.v1", "epic_id": "NEW", "generated_at_unix": 500u64, "prs": [], "summary": {}}),
        );

        let snapshot = mission_control_snapshot(&config_for(&repo)).unwrap();
        assert_eq!(snapshot["epic_progress"]["epic_id"], "NEW");
    }
}
