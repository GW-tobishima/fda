use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn read_json_value(path: &Path) -> Result<Value, String> {
    let body =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&body).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

pub(crate) fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    let body = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, format!("{body}\n"))
        .map_err(|e| format!("failed to write JSON file {}: {e}", path.display()))
}
