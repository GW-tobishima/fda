use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::domain::entities::{CodexLiveInvocationResult, ToolProbeResult};

#[derive(Clone, Debug)]
pub(crate) struct AtoConfig {
    pub(crate) enabled: bool,
    pub(crate) task_key: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) backend: Option<String>,
    pub(crate) db_path: Option<PathBuf>,
    pub(crate) cli_command: Vec<String>,
}

impl Default for AtoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            task_key: None,
            run_id: None,
            backend: None,
            db_path: None,
            cli_command: vec!["ato".to_string()],
        }
    }
}

pub(crate) struct ProcessOutput {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) trait CodexProcessPort {
    fn query_mcp_tools_list(&self, command: &[String], cwd: &Path) -> ToolProbeResult;
    fn query_codex_live_tool(
        &self,
        cwd: &Path,
        prompt: &str,
        timeout: Duration,
    ) -> CodexLiveInvocationResult;
    fn git_head_sha(&self, repo: &Path) -> String;
}

#[derive(Serialize)]
pub(crate) struct CheckError {
    pub(crate) message: String,
    pub(crate) instance_path: Option<String>,
    pub(crate) schema_path: Option<String>,
}

pub(crate) trait ArtifactStore {
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String>;
    fn create_dir_all(&self, path: &Path) -> Result<(), String>;
    fn exists(&self, path: &Path) -> bool;
    fn copy(&self, src: &Path, dst: &Path) -> Result<(), String>;
    fn read_text(&self, path: &Path) -> Result<String, String>;
    fn write_text(&self, path: &Path, body: &str) -> Result<(), String>;
    fn write_json(&self, path: &Path, value: &Value) -> Result<(), String>;
    fn schema_files(&self, schema_dir: &Path) -> Result<Vec<PathBuf>, String>;
    fn yaml_files(&self, dir: &Path) -> Result<Vec<PathBuf>, String>;
}

pub(crate) trait ArtifactValidator {
    fn compile_schema(&self, schema_json: &Value) -> Result<(), CheckError>;
    fn validate_json_schema(
        &self,
        schema_json: &Value,
        artifact_json: &Value,
    ) -> Result<Vec<CheckError>, CheckError>;
}

pub(crate) trait YamlValidator {
    fn validate_yaml_syntax(&self, path: &Path, body: &str) -> Result<(), String>;
    fn parse_yaml_value(&self, path: &Path, body: &str) -> Result<Value, String>;
}

pub(crate) trait Clock {
    fn now_unix_seconds(&self) -> u64;
}
