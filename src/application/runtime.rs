use serde_json::Value;
use std::path::Path;

use crate::application::ports::ArtifactStore;
use crate::domain::entities::RuntimeContext;

pub(crate) fn runtime_context_from_output_store(
    store: &impl ArtifactStore,
    out_dir: &Path,
) -> Result<RuntimeContext, String> {
    let epic = read_json_from_store(store, &out_dir.join("epic_delivery_plan.json"))?;
    let task_graph = read_json_from_store(store, &out_dir.join("task_graph.json"))?;
    let case_graph = read_json_from_store(store, &out_dir.join("case_graph.json"))?;
    Ok(RuntimeContext {
        program_id: value_string(&epic, "program_id").unwrap_or_else(|| "UNKNOWN".to_string()),
        epic_id: value_string(&epic, "epic_id").unwrap_or_else(|| "UNKNOWN".to_string()),
        case_ids: case_graph
            .get("cases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|case| value_string(case, "case_id"))
            .collect(),
        task_ids: task_graph
            .get("tasks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|task| value_string(task, "task_id"))
            .collect(),
    })
}

fn read_json_from_store(store: &impl ArtifactStore, path: &Path) -> Result<Value, String> {
    let body = store.read_text(path)?;
    serde_json::from_str(&body).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}
