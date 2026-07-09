use serde::Serialize;
use serde_json::{json, Value};

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, decision_type_from_packet,
    joined_recorded_decisions, read_decision_receipts, read_json_value, write_decision_receipts,
};
use crate::application::policy::{evaluate_contract_for_decision, read_delegation_contract_rules};
use crate::application::ports::{ArtifactStore, Clock};
use crate::application::profile::ensure_repository_profile;
use crate::cli::args::DecideConfig;
use crate::domain::entities::HumanDecisionSummary;
use crate::domain::policies::decision::decision_blockers;
use crate::infra::clock::SystemClock;
use crate::infra::fs_store::FsArtifactStore;
use crate::infra::yaml::SerdeYamlValidator;
use crate::support::date::unix_seconds_to_ymd;
use crate::support::paths::{display_path, resolve_path};

#[derive(Serialize)]
pub(crate) struct DecideResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) decision_id: String,
    /// 実際に記録した回答（`--by-contract` 時は契約の answer）。
    pub(crate) answer: String,
    /// 実際に記録した decided_by（`--by-contract` 時は `delegation_contract:<rule_id>:<authority>`）。
    pub(crate) decided_by: String,
    /// `--by-contract` で適用した契約 rule_id（明示適用時のみ）。
    pub(crate) contract_rule_id: Option<String>,
    pub(crate) packet_status: String,
    pub(crate) artifact_dir: String,
    pub(crate) receipts_path: String,
    pub(crate) unresolved_decisions: Vec<HumanDecisionSummary>,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) fn decide(config: &DecideConfig) -> Result<DecideResult, String> {
    let store = FsArtifactStore;
    let clock = SystemClock;
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    ensure_repository_profile(&store, &repo_root)?;
    let artifact_dir = resolve_path(&repo_root, &config.artifact_dir);
    let packet_path = artifact_dir.join("human_decision_packet.json");
    if !store.exists(&packet_path) {
        return Err(format!(
            "human decision packet does not exist: {}",
            packet_path.display()
        ));
    }

    let mut packet = read_json_value(&store, &packet_path)?;
    let decisions = decision_summaries_from_packet(&packet);
    let decision = decisions
        .iter()
        .find(|decision| decision.matches_id(&config.decision_id))
        .ok_or_else(|| format!("unknown decision id `{}`", config.decision_id))?;
    let canonical_decision_id = decision.decision_id.clone();
    if canonical_decision_id.is_empty() {
        return Err(format!("unknown decision id `{}`", config.decision_id));
    };

    // `--by-contract` 指定時は delegation contract を評価し、合致 + 未失効のときだけ
    // 契約の answer で回答を記録する。1 つでも満たさなければ fail-closed で人間判断へ戻す。
    let now_unix = clock.now_unix_seconds();
    let contract_application = if let Some(rule_id) = &config.by_contract {
        let yaml = SerdeYamlValidator;
        let rules = read_delegation_contract_rules(&store, &yaml, &repo_root)?;
        let decision_type =
            decision_type_from_packet(&packet, &canonical_decision_id).unwrap_or_default();
        let application = evaluate_contract_for_decision(
            &rules,
            rule_id,
            &canonical_decision_id,
            &decision_type,
            &decision.summary,
            &unix_seconds_to_ymd(now_unix),
        )?;
        Some(application)
    } else {
        None
    };

    let (effective_answer, effective_decided_by) = match &contract_application {
        Some(application) => (
            application.answer.clone(),
            format!(
                "delegation_contract:{}:{}",
                application.rule_id, application.authority
            ),
        ),
        None => (config.answer.clone(), config.decided_by.clone()),
    };

    let receipts_path = artifact_dir.join("decision_receipts.json");
    let mut receipts = read_decision_receipts(&store, &receipts_path)?;
    let decided_at = now_unix.to_string();
    let mut receipt = json!({
        "decision_id": canonical_decision_id,
        "input_decision_id": config.decision_id,
        "answer": effective_answer,
        "decided_by": effective_decided_by,
        "decided_at_unix_seconds": decided_at
    });
    if let Some(application) = &contract_application {
        let object = receipt
            .as_object_mut()
            .ok_or_else(|| "decision receipt must be an object".to_string())?;
        object.insert(
            "contract_rule_id".to_string(),
            Value::String(application.rule_id.clone()),
        );
        object.insert(
            "contract_expires".to_string(),
            Value::String(application.expires.clone()),
        );
        object.insert(
            "authority".to_string(),
            Value::String(application.authority.clone()),
        );
    }
    receipts.insert(canonical_decision_id.clone(), receipt);
    write_decision_receipts(&store, &receipts_path, &receipts)?;

    let unresolved_decisions =
        decision_blockers(&decisions, &decision_answers_from_receipts(&receipts));

    if unresolved_decisions.is_empty() {
        let object = packet
            .as_object_mut()
            .ok_or_else(|| "human_decision_packet.json must be an object".to_string())?;
        object.insert("status".to_string(), Value::String("resolved".to_string()));
        object.insert(
            "recorded_decision".to_string(),
            json!({
                "decision": joined_recorded_decisions(&receipts),
                "decided_by": effective_decided_by.clone(),
                "decided_at": decided_at,
                "rationale": "Recorded by fda decide"
            }),
        );
        store.write_json(&packet_path, &packet)?;
    } else {
        let object = packet
            .as_object_mut()
            .ok_or_else(|| "human_decision_packet.json must be an object".to_string())?;
        object.insert(
            "status".to_string(),
            Value::String("waiting_human".to_string()),
        );
        object.remove("recorded_decision");
        store.write_json(&packet_path, &packet)?;
    }

    let packet_status = if unresolved_decisions.is_empty() {
        "resolved"
    } else {
        "waiting_human"
    };

    let next_actions = if unresolved_decisions.is_empty() {
        vec![format!(
            "fda design --artifacts {}",
            display_path(&repo_root, &artifact_dir)
        )]
    } else {
        unresolved_decisions
            .iter()
            .map(|decision| {
                format!(
                    "fda decide {} --answer <answer> --artifacts {}",
                    decision.decision_id,
                    display_path(&repo_root, &artifact_dir)
                )
            })
            .collect()
    };

    Ok(DecideResult {
        schema_version: "fda.decide_result.v0",
        verdict: "pass",
        decision_id: canonical_decision_id,
        answer: effective_answer,
        decided_by: effective_decided_by,
        contract_rule_id: contract_application
            .as_ref()
            .map(|application| application.rule_id.clone()),
        packet_status: packet_status.to_string(),
        artifact_dir: display_path(&repo_root, &artifact_dir),
        receipts_path: display_path(&repo_root, &receipts_path),
        unresolved_decisions,
        next_actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::AtoConfig;
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_repo(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = SystemClock.now_unix_seconds();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("{name}-{unique}-{seq}"));
        FsArtifactStore.create_dir_all(&dir).unwrap();
        dir
    }

    const KEYWORD_SUMMARY: &str =
        "入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか";

    fn write_packet(artifact_dir: &Path) {
        FsArtifactStore
            .write_json(
                &artifact_dir.join("human_decision_packet.json"),
                &json!({
                    "schema_version": "fda.human_decision_packet.v0",
                    "status": "waiting_human",
                    "decisions": [{
                        "decision_id": "HD-FDA-001",
                        "type": "spec_decision",
                        "summary": KEYWORD_SUMMARY,
                        "required_before": "Design Gate",
                        "options": [{"id": "approve_scope"}, {"id": "revise"}],
                        "recommended_option_id": "approve_scope"
                    }]
                }),
            )
            .unwrap();
    }

    fn write_contract(repo: &Path, expires: &str) {
        let fda = repo.join(".fda");
        FsArtifactStore.create_dir_all(&fda).unwrap();
        let body = format!(
            "delegation_contract:\n  - rule_id: DC-001\n    decision_type: spec_decision\n    match_summary_keywords:\n      - \"Scope In / Scope Out\"\n    answer: approve_scope\n    authority: k_tobishima\n    enacted_from:\n      - \"run HD-FDA-001\"\n    expires: \"{expires}\"\n"
        );
        FsArtifactStore
            .write_text(&fda.join("delegation_contract.yaml"), &body)
            .unwrap();
    }

    fn by_contract_config(repo: &Path) -> DecideConfig {
        DecideConfig {
            repo_root: repo.to_path_buf(),
            artifact_dir: PathBuf::from("artifacts"),
            decision_id: "HD-FDA-001".to_string(),
            answer: String::new(),
            decided_by: "human".to_string(),
            by_contract: Some("DC-001".to_string()),
            ato: AtoConfig::default(),
            print_json: false,
        }
    }

    #[test]
    fn by_contract_records_answer_and_receipt() {
        let repo = temp_repo("fda-decide-by-contract");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_packet(&artifacts);
        write_contract(&repo, "2099-01-01");

        let result = decide(&by_contract_config(&repo)).unwrap();
        assert_eq!(result.answer, "approve_scope");
        assert_eq!(result.decided_by, "delegation_contract:DC-001:k_tobishima");
        assert_eq!(result.contract_rule_id.as_deref(), Some("DC-001"));
        assert_eq!(result.packet_status, "resolved");

        let receipts =
            read_json_value(&FsArtifactStore, &artifacts.join("decision_receipts.json")).unwrap();
        let receipt = receipts
            .get("receipts")
            .and_then(Value::as_array)
            .and_then(|receipts| receipts.first())
            .unwrap();
        assert_eq!(
            receipt.get("answer").and_then(Value::as_str),
            Some("approve_scope")
        );
        assert_eq!(
            receipt.get("decided_by").and_then(Value::as_str),
            Some("delegation_contract:DC-001:k_tobishima")
        );
        assert_eq!(
            receipt.get("contract_rule_id").and_then(Value::as_str),
            Some("DC-001")
        );
        assert_eq!(
            receipt.get("contract_expires").and_then(Value::as_str),
            Some("2099-01-01")
        );
        assert_eq!(
            receipt.get("authority").and_then(Value::as_str),
            Some("k_tobishima")
        );
    }

    #[test]
    fn by_contract_rejects_expired_contract_without_writing_receipt() {
        let repo = temp_repo("fda-decide-by-contract-expired");
        let artifacts = repo.join("artifacts");
        FsArtifactStore.create_dir_all(&artifacts).unwrap();
        write_packet(&artifacts);
        write_contract(&repo, "2000-01-01");

        match decide(&by_contract_config(&repo)) {
            Err(err) => {
                assert!(err.contains("expires"));
                assert!(err.contains("fda decide HD-FDA-001 --answer"));
            }
            Ok(_) => panic!("expired contract must be rejected"),
        }
        // fail-closed: 受領証は書かれない。
        assert!(!artifacts.join("decision_receipts.json").exists());
    }
}
