use std::path::Path;

use crate::application::ports::{ArtifactStore, CheckError, YamlValidator};
use crate::application::validate::{failed_check, passed_check, skipped_check, ValidationCheck};
use crate::support::paths::display_path;

pub(crate) struct SerdeYamlValidator;

impl YamlValidator for SerdeYamlValidator {
    fn validate_yaml_syntax(&self, path: &Path, body: &str) -> Result<(), String> {
        serde_yaml::from_str::<serde_yaml::Value>(body)
            .map(|_| ())
            .map_err(|e| format!("failed to parse YAML file {}: {e}", path.display()))
    }

    fn parse_yaml_value(&self, path: &Path, body: &str) -> Result<serde_json::Value, String> {
        serde_yaml::from_str::<serde_json::Value>(body)
            .map_err(|e| format!("failed to parse YAML file {}: {e}", path.display()))
    }
}

pub(crate) fn validate_yaml_dir(
    store: &impl ArtifactStore,
    validator: &impl YamlValidator,
    repo_root: &Path,
    dir: &Path,
) -> Result<Vec<ValidationCheck>, String> {
    if !store.exists(dir) {
        return Ok(vec![skipped_check(
            format!("yaml_dir:{}", display_path(repo_root, dir)),
            "yaml_syntax",
            None,
            Some(display_path(repo_root, dir)),
            "model contract directory does not exist".to_string(),
        )]);
    }

    let mut checks = Vec::new();
    for yaml_path in store.yaml_files(dir)? {
        let artifact_display = display_path(repo_root, &yaml_path);
        match store
            .read_text(&yaml_path)
            .and_then(|body| validator.validate_yaml_syntax(&yaml_path, &body))
        {
            Ok(()) => checks.push(passed_check(
                format!("yaml:{artifact_display}"),
                "yaml_syntax",
                None,
                Some(artifact_display),
            )),
            Err(message) => checks.push(failed_check(
                format!("yaml:{artifact_display}"),
                "yaml_syntax",
                None,
                Some(artifact_display),
                CheckError {
                    message,
                    instance_path: None,
                    schema_path: None,
                },
            )),
        }
    }
    Ok(checks)
}
