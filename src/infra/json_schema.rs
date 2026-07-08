use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

use crate::application::ports::{ArtifactStore, ArtifactValidator, CheckError};
use std::path::Path;

pub(crate) struct JsonSchemaArtifactValidator;

impl ArtifactValidator for JsonSchemaArtifactValidator {
    fn compile_schema(&self, schema_json: &Value) -> Result<(), CheckError> {
        JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .compile(schema_json)
            .map(|_| ())
            .map_err(|error| CheckError {
                message: error.to_string(),
                instance_path: None,
                schema_path: None,
            })
    }

    fn validate_json_schema(
        &self,
        schema_json: &Value,
        artifact_json: &Value,
    ) -> Result<Vec<CheckError>, CheckError> {
        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .compile(schema_json)
            .map_err(|error| CheckError {
                message: error.to_string(),
                instance_path: None,
                schema_path: None,
            })?;

        let errors = match compiled.validate(artifact_json) {
            Ok(()) => Vec::new(),
            Err(errors) => errors
                .map(|error| CheckError {
                    message: error.to_string(),
                    instance_path: Some(error.instance_path.to_string()),
                    schema_path: Some(error.schema_path.to_string()),
                })
                .collect(),
        };
        Ok(errors)
    }
}

pub(crate) fn read_json(store: &impl ArtifactStore, path: &Path) -> Result<Value, CheckError> {
    let body = store.read_text(path).map_err(|e| CheckError {
        message: e,
        instance_path: None,
        schema_path: None,
    })?;
    serde_json::from_str(&body).map_err(|e| CheckError {
        message: format!("failed to parse {}: {e}", path.display()),
        instance_path: None,
        schema_path: None,
    })
}
