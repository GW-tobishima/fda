use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

use crate::application::ports::ArtifactStore;
use crate::domain::entities::HumanDecisionSummary;

pub(crate) fn read_json_value(store: &impl ArtifactStore, path: &Path) -> Result<Value, String> {
    let body = store.read_text(path)?;
    serde_json::from_str(&body).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

pub(crate) fn read_decision_receipts(
    store: &impl ArtifactStore,
    path: &Path,
) -> Result<HashMap<String, Value>, String> {
    if !store.exists(path) {
        return Ok(HashMap::new());
    }
    let value = read_json_value(store, path)?;
    let mut receipts = HashMap::new();
    for receipt in value
        .get("receipts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(decision_id) = value_string(receipt, "decision_id") {
            receipts.insert(decision_id, receipt.clone());
        }
    }
    Ok(receipts)
}

pub(crate) fn write_decision_receipts(
    store: &impl ArtifactStore,
    path: &Path,
    receipts: &HashMap<String, Value>,
) -> Result<(), String> {
    let mut keys: Vec<&String> = receipts.keys().collect();
    keys.sort();
    let ordered_receipts: Vec<Value> = keys
        .into_iter()
        .filter_map(|key| receipts.get(key).cloned())
        .collect();
    store.write_json(
        path,
        &json!({
            "schema_version": "fda.decision_receipts.v0",
            "receipts": ordered_receipts
        }),
    )
}

pub(crate) fn joined_recorded_decisions(receipts: &HashMap<String, Value>) -> String {
    let mut keys: Vec<&String> = receipts.keys().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| {
            receipts.get(key).map(|receipt| {
                format!(
                    "{}={}",
                    key,
                    value_string(receipt, "answer").unwrap_or_else(|| "<empty>".to_string())
                )
            })
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn decision_answers_from_receipts(
    receipts: &HashMap<String, Value>,
) -> HashMap<String, String> {
    receipts
        .iter()
        .filter_map(|(decision_id, receipt)| {
            value_string(receipt, "answer").map(|answer| (decision_id.clone(), answer))
        })
        .collect()
}

pub(crate) fn recorded_decision_receipts_from_packet(packet: &Value) -> HashMap<String, Value> {
    let mut receipts = HashMap::new();
    let Some(recorded_decision) = packet.get("recorded_decision") else {
        return receipts;
    };

    let decision_text = value_string(recorded_decision, "decision")
        .or_else(|| recorded_decision.as_str().map(ToOwned::to_owned))
        .unwrap_or_default();
    for segment in decision_text.split([';', '\n']) {
        let Some((decision_id, answer)) = segment.trim().split_once('=') else {
            continue;
        };
        let decision_id = decision_id.trim();
        let answer = answer.trim();
        if decision_id.is_empty() || answer.is_empty() {
            continue;
        }
        receipts.insert(
            decision_id.to_string(),
            json!({
                "decision_id": decision_id,
                "answer": answer
            }),
        );
    }

    receipts
}

pub(crate) fn decision_summaries_from_packet(packet: &Value) -> Vec<HumanDecisionSummary> {
    let nested_decisions: Vec<HumanDecisionSummary> = packet
        .get("decisions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|decision| HumanDecisionSummary {
            decision_id: value_string(decision, "decision_id")
                .unwrap_or_else(|| "HD-FDA-UNKNOWN".to_string()),
            alias_ids: Vec::new(),
            summary: value_string(decision, "summary")
                .unwrap_or_else(|| "判断内容が未設定です".to_string()),
            recommended_option_id: value_string(decision, "recommended_option_id")
                .unwrap_or_else(|| "unknown".to_string()),
            option_ids: decision_option_ids(decision),
            required_before: value_string(decision, "required_before")
                .unwrap_or_else(|| "Design Gate".to_string()),
        })
        .collect();
    if nested_decisions.is_empty() {
        top_level_decision_summary(packet).into_iter().collect()
    } else {
        nested_decisions
    }
}

fn top_level_decision_summary(packet: &Value) -> Option<HumanDecisionSummary> {
    let decision_needed = value_string(packet, "decision_needed")?;
    let decision_packet_id = value_string(packet, "decision_packet_id");
    let decision_id = packet
        .get("forge_mapping")
        .and_then(|forge_mapping| {
            value_string_array(forge_mapping, "human_decision_points")
                .first()
                .cloned()
        })
        .or_else(|| decision_packet_id.clone())
        .unwrap_or_else(|| "HD-FDA-TOP-LEVEL".to_string());
    let alias_ids = decision_packet_id
        .into_iter()
        .filter(|alias| alias != &decision_id)
        .collect::<Vec<_>>();
    let recommended_option_id = packet
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|option| {
            option
                .get("recommended")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .and_then(|option| value_string(option, "id"))
        .or_else(|| {
            packet
                .get("options")
                .and_then(Value::as_array)
                .and_then(|options| options.first())
                .and_then(|option| value_string(option, "id"))
        })
        .unwrap_or_else(|| "approve".to_string());

    Some(HumanDecisionSummary {
        decision_id,
        alias_ids,
        summary: decision_needed,
        recommended_option_id,
        option_ids: decision_option_ids(packet),
        required_before: value_string(packet, "required_before")
            .unwrap_or_else(|| "Design Gate".to_string()),
    })
}

fn decision_option_ids(value: &Value) -> Vec<String> {
    value
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|option| value_string(option, "id"))
        .collect()
}

pub(crate) fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(crate) fn value_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
