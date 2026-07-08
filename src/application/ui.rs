use serde_json::{json, Value};
use std::path::PathBuf;

use crate::application::ports::{ArtifactStore, AtoConfig};
use crate::application::status;
use crate::cli::args::StatusConfig;
use crate::infra::clock::system_unix_seconds;
use crate::infra::fs_store::{list_dir_names, list_file_names, FsArtifactStore};
use crate::support::paths::resolve_path;

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
                for decision in &result.unresolved_decisions {
                    open_decisions += 1;
                    decision_inbox.push(json!({
                        "run": name,
                        "run_dir": run_rel,
                        "decision_id": decision.decision_id,
                        "summary": decision.summary,
                        "required_before": decision.required_before,
                        "recommended_option_id": decision.recommended_option_id,
                        "option_ids": decision.option_ids,
                        "resume_command": format!(
                            "fda decide {} --answer <answer> --artifacts {run_rel}",
                            decision.decision_id
                        ),
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
    }))
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
    }
}
