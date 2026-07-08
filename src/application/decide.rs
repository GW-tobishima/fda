use serde::Serialize;
use serde_json::{json, Value};

use crate::application::decisions::{
    decision_answers_from_receipts, decision_summaries_from_packet, joined_recorded_decisions,
    read_decision_receipts, read_json_value, write_decision_receipts,
};
use crate::application::ports::{ArtifactStore, Clock};
use crate::application::profile::ensure_repository_profile;
use crate::cli::args::DecideConfig;
use crate::domain::entities::HumanDecisionSummary;
use crate::domain::policies::decision::decision_blockers;
use crate::infra::clock::SystemClock;
use crate::infra::fs_store::FsArtifactStore;
use crate::support::paths::{display_path, resolve_path};

#[derive(Serialize)]
pub(crate) struct DecideResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) decision_id: String,
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

    let receipts_path = artifact_dir.join("decision_receipts.json");
    let mut receipts = read_decision_receipts(&store, &receipts_path)?;
    let decided_at = clock.now_unix_seconds().to_string();
    receipts.insert(
        canonical_decision_id.clone(),
        json!({
            "decision_id": canonical_decision_id,
            "input_decision_id": config.decision_id,
            "answer": config.answer,
            "decided_by": config.decided_by,
            "decided_at_unix_seconds": decided_at
        }),
    );
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
                "decided_by": config.decided_by,
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
        packet_status: packet_status.to_string(),
        artifact_dir: display_path(&repo_root, &artifact_dir),
        receipts_path: display_path(&repo_root, &receipts_path),
        unresolved_decisions,
        next_actions,
    })
}
