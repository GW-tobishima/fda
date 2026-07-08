use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command as ProcessCommand, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::application::ports::{CodexProcessPort, ProcessOutput};
use crate::domain::entities::{
    CodexLiveInvocationResult, CodexLiveStatus, ToolProbeResult, ToolProbeStatus,
};

pub(crate) struct CodexMcpProcessAdapter;

impl CodexProcessPort for CodexMcpProcessAdapter {
    fn query_mcp_tools_list(&self, command: &[String], cwd: &Path) -> ToolProbeResult {
        query_mcp_tools_list(command, cwd)
    }

    fn query_codex_live_tool(
        &self,
        cwd: &Path,
        prompt: &str,
        timeout: Duration,
    ) -> CodexLiveInvocationResult {
        query_codex_live_tool(cwd, prompt, timeout)
    }

    fn git_head_sha(&self, repo: &Path) -> String {
        git_head_sha(repo)
    }
}

struct CodexLiveInvocationError {
    summary: String,
    tool_call_sent: bool,
}

impl CodexLiveInvocationError {
    fn before_tool_call(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            tool_call_sent: false,
        }
    }

    fn after_tool_call(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            tool_call_sent: true,
        }
    }
}

pub(crate) fn query_codex_live_tool(
    cwd: &Path,
    prompt: &str,
    timeout: Duration,
) -> CodexLiveInvocationResult {
    match query_codex_live_tool_inner(cwd, prompt, timeout) {
        Ok(result) => result,
        Err(error)
            if error.summary.contains("not found") || error.summary.contains("No such file") =>
        {
            CodexLiveInvocationResult {
                status: CodexLiveStatus::AdapterUnavailable,
                thread_id: None,
                content: String::new(),
                summary: error.summary,
                exit_code: None,
                tool_call_sent: error.tool_call_sent,
            }
        }
        Err(error) if error.summary.contains("MCP approval prompt received") => {
            CodexLiveInvocationResult {
                status: CodexLiveStatus::Blocked,
                thread_id: mcp_thread_id_from_error(&error.summary),
                content: String::new(),
                summary: error.summary,
                exit_code: None,
                tool_call_sent: error.tool_call_sent,
            }
        }
        Err(error) => CodexLiveInvocationResult {
            status: CodexLiveStatus::Failed,
            thread_id: None,
            content: String::new(),
            summary: error.summary,
            exit_code: None,
            tool_call_sent: error.tool_call_sent,
        },
    }
}

/// Windows では npm install された CLI (codex 等) が .cmd / .ps1 shim で提供され、
/// CreateProcess は拡張子なし名を .exe しか解決しないため spawn に失敗する。
/// PATH 上に <name>.exe が無く <name>.cmd がある場合は .cmd のフルパスへ解決する。
#[cfg(windows)]
pub(crate) fn resolve_program_for_spawn(program: &str) -> String {
    use std::path::Path as StdPath;
    if StdPath::new(program).extension().is_some()
        || program.contains('\\')
        || program.contains('/')
    {
        return program.to_string();
    }
    let path_var = env::var_os("PATH").unwrap_or_default();
    let dirs: Vec<std::path::PathBuf> = env::split_paths(&path_var).collect();
    if dirs
        .iter()
        .any(|dir| dir.join(format!("{program}.exe")).is_file())
    {
        return program.to_string();
    }
    for dir in &dirs {
        let cmd_shim = dir.join(format!("{program}.cmd"));
        if cmd_shim.is_file() {
            return cmd_shim.display().to_string();
        }
    }
    program.to_string()
}

#[cfg(not(windows))]
pub(crate) fn resolve_program_for_spawn(program: &str) -> String {
    program.to_string()
}

fn query_codex_live_tool_inner(
    cwd: &Path,
    prompt: &str,
    timeout: Duration,
) -> Result<CodexLiveInvocationResult, CodexLiveInvocationError> {
    let mut process = ProcessCommand::new(resolve_program_for_spawn("codex"));
    process
        .arg("mcp-server")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = process.spawn().map_err(|e| {
        CodexLiveInvocationError::before_tool_call(format!(
            "MCP server command `codex mcp-server` not found or unavailable: {e}"
        ))
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        CodexLiveInvocationError::before_tool_call("failed to capture MCP server stdout")
    })?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || read_mcp_frames(stdout, sender));

    let mut stdin = child.stdin.take().ok_or_else(|| {
        CodexLiveInvocationError::before_tool_call("failed to capture MCP server stdin")
    })?;
    write_mcp_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "forge-delivery-agent",
                    "version": "0.1.0"
                }
            }
        }),
    )
    .map_err(CodexLiveInvocationError::before_tool_call)?;
    if let Err(error) = wait_for_mcp_response(&receiver, 1, Duration::from_secs(5)) {
        let _ = stop_child(&mut child);
        return Err(CodexLiveInvocationError::before_tool_call(error));
    }
    write_mcp_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    )
    .map_err(CodexLiveInvocationError::before_tool_call)?;
    write_mcp_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "codex",
                "arguments": {
                    "prompt": prompt,
                    "cwd": cwd.to_string_lossy(),
                    "approval-policy": "on-request",
                    "sandbox": "workspace-write"
                }
            }
        }),
    )
    .map_err(CodexLiveInvocationError::before_tool_call)?;
    let response = wait_for_mcp_response(&receiver, 2, timeout);
    let _ = stop_child(&mut child);
    let response = response.map_err(CodexLiveInvocationError::after_tool_call)?;
    if let Some(error) = response.get("error") {
        return Err(CodexLiveInvocationError::after_tool_call(format!(
            "codex tools/call returned error: {error}"
        )));
    }
    Ok(codex_live_result_from_response(&response))
}

fn codex_live_result_from_response(response: &Value) -> CodexLiveInvocationResult {
    let result = response.get("result").unwrap_or(&Value::Null);
    let structured = result
        .get("structuredContent")
        .or_else(|| result.get("structured_content"));
    let thread_id = structured
        .and_then(|value| value_string(value, "threadId"))
        .or_else(|| structured.and_then(|value| value_string(value, "thread_id")));
    let content = structured
        .and_then(|value| value_string(value, "content"))
        .or_else(|| value_string(result, "content"))
        .unwrap_or_else(|| {
            result
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| value_string(item, "text"))
                .collect::<Vec<_>>()
                .join("\n")
        });
    let summary = if content.trim().is_empty() {
        "codex tool returned an empty content response".to_string()
    } else {
        "codex tool completed and returned content".to_string()
    };
    CodexLiveInvocationResult {
        status: if content.trim().is_empty() {
            CodexLiveStatus::Failed
        } else {
            CodexLiveStatus::Succeeded
        },
        thread_id,
        content,
        summary,
        exit_code: Some(0),
        tool_call_sent: true,
    }
}

pub(crate) fn query_mcp_tools_list(command: &[String], cwd: &Path) -> ToolProbeResult {
    match query_mcp_tools_list_inner(command, cwd) {
        Ok(tools) => ToolProbeResult {
            status: ToolProbeStatus::Succeeded,
            detected_tools: tools,
            summary: "MCP initialize and tools/list completed.".to_string(),
            exit_code: Some(0),
        },
        Err(error) if error.contains("not found") || error.contains("No such file") => {
            ToolProbeResult {
                status: ToolProbeStatus::AdapterUnavailable,
                detected_tools: Vec::new(),
                summary: error,
                exit_code: None,
            }
        }
        Err(error) => ToolProbeResult {
            status: ToolProbeStatus::Failed,
            detected_tools: Vec::new(),
            summary: error,
            exit_code: None,
        },
    }
}

fn query_mcp_tools_list_inner(command: &[String], cwd: &Path) -> Result<Vec<String>, String> {
    let program = command
        .first()
        .ok_or_else(|| "MCP server command is empty".to_string())?;
    let mut process = ProcessCommand::new(resolve_program_for_spawn(program));
    process
        .args(&command[1..])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if env::var_os("CODEX_HOME").is_none() {
        let codex_home = env::temp_dir().join(format!("fda-codex-mcp-home-{}", std::process::id()));
        fs::create_dir_all(&codex_home).map_err(|e| {
            format!(
                "failed to create temporary CODEX_HOME {}: {e}",
                codex_home.display()
            )
        })?;
        process.env("CODEX_HOME", codex_home);
    }
    let mut child = process.spawn().map_err(|e| {
        format!(
            "MCP server command `{}` not found or unavailable: {e}",
            command.join(" ")
        )
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture MCP server stdout".to_string())?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || read_mcp_frames(stdout, sender));

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to capture MCP server stdin".to_string())?;
    write_mcp_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "forge-delivery-agent",
                    "version": "0.1.0"
                }
            }
        }),
    )?;
    if let Err(error) = wait_for_mcp_response(&receiver, 1, Duration::from_secs(5)) {
        let _ = stop_child(&mut child);
        return Err(error);
    }
    write_mcp_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    )?;
    write_mcp_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )?;
    let response = wait_for_mcp_response(&receiver, 2, Duration::from_secs(5));
    let _ = stop_child(&mut child);
    let response = response?;
    if let Some(error) = response.get("error") {
        return Err(format!("tools/list returned error: {error}"));
    }
    Ok(response
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|tool| value_string(tool, "name"))
        .collect())
}

pub(crate) fn run_process_command(command: &[String], cwd: &Path) -> Result<ProcessOutput, String> {
    let program = command
        .first()
        .ok_or_else(|| "process command is empty".to_string())?;
    let output = ProcessCommand::new(program)
        .args(&command[1..])
        .current_dir(cwd)
        .output()
        .map_err(|e| {
            format!(
                "process command `{}` not found or unavailable: {e}",
                command_display(command)
            )
        })?;
    Ok(ProcessOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Resolve the Python launcher used for gate scripts.
///
/// Windows installs usually expose `python` or `py -3` but not `python3`,
/// so the launcher is resolved as: `FDA_PYTHON` (whitespace-separated
/// command), then the first of `python3` / `python` / `py -3` that answers
/// `--version`. Falls back to `python3` so failures stay explicit.
pub(crate) fn python_launcher() -> Vec<String> {
    if let Ok(value) = env::var("FDA_PYTHON") {
        let parts: Vec<String> = value.split_whitespace().map(str::to_string).collect();
        if !parts.is_empty() {
            return parts;
        }
    }
    for candidate in [&["python3"][..], &["python"][..], &["py", "-3"][..]] {
        let responds = ProcessCommand::new(candidate[0])
            .args(&candidate[1..])
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if responds {
            return candidate.iter().map(|part| part.to_string()).collect();
        }
    }
    vec!["python3".to_string()]
}

pub(crate) fn git_head_sha(repo: &Path) -> String {
    ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_mcp_frames(stdout: std::process::ChildStdout, sender: mpsc::Sender<Value>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut line = String::new();
        let Ok(bytes_read) = reader.read_line(&mut line) else {
            return;
        };
        if bytes_read == 0 {
            return;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            if sender.send(value).is_err() {
                return;
            }
        }
    }
}

fn write_mcp_message(stdin: &mut ChildStdin, value: Value) -> Result<(), String> {
    let body = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    stdin
        .write_all(format!("{body}\n").as_bytes())
        .and_then(|_| stdin.flush())
        .map_err(|e| format!("failed to write MCP message: {e}"))
}

fn wait_for_mcp_response(
    receiver: &Receiver<Value>,
    id: i64,
    timeout: Duration,
) -> Result<Value, String> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| format!("MCP response timeout is too large for response id {id}"))?;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(format!("timed out waiting for MCP response id {id}"));
        }
        match receiver.recv_timeout(deadline.saturating_duration_since(now)) {
            Ok(value) if value.get("id").and_then(Value::as_i64) == Some(id) => return Ok(value),
            Ok(value) if is_mcp_approval_prompt(&value) => {
                let method = value_string(&value, "method").unwrap_or_else(|| "<unknown>".into());
                let thread_id =
                    mcp_thread_id_from_message(&value).unwrap_or_else(|| "<unknown>".to_string());
                return Err(format!(
                    "MCP approval prompt received while waiting for response id {id}; method={method}; threadId={thread_id}; FDA V1 does not auto-approve live Codex prompts"
                ));
            }
            Ok(_) => continue,
            Err(error) => return Err(format!("failed waiting for MCP response id {id}: {error}")),
        }
    }
}

fn is_mcp_approval_prompt(value: &Value) -> bool {
    value_string(value, "method")
        .map(|method| method.to_ascii_lowercase().contains("approval"))
        .unwrap_or(false)
}

fn mcp_thread_id_from_message(value: &Value) -> Option<String> {
    value.get("params").and_then(|params| {
        value_string(params, "threadId").or_else(|| value_string(params, "thread_id"))
    })
}

fn mcp_thread_id_from_error(error: &str) -> Option<String> {
    error
        .split("threadId=")
        .nth(1)
        .and_then(|tail| tail.split(';').next())
        .map(str::trim)
        .filter(|thread_id| !thread_id.is_empty() && *thread_id != "<unknown>")
        .map(ToOwned::to_owned)
}

fn stop_child(child: &mut Child) -> Result<(), String> {
    let _ = child.kill();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => return Ok(()),
            Err(error) => return Err(format!("failed to poll MCP server shutdown: {error}")),
        }
    }
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn command_display(command: &[String]) -> String {
    command
        .iter()
        .map(|part| shell_arg(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_arg(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '=' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_program_for_spawn_returns_input_when_no_shim_applies() {
        assert_eq!(resolve_program_for_spawn("program.exe"), "program.exe");
        assert_eq!(
            resolve_program_for_spawn("fda-nonexistent-program-xyz"),
            "fda-nonexistent-program-xyz"
        );
    }

    #[test]
    fn wait_for_mcp_response_fails_fast_on_approval_prompt() {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "codex/exec_approval",
                "params": {
                    "threadId": "thread-approval-001",
                    "command": "gh pr create"
                }
            }))
            .unwrap();

        let error = wait_for_mcp_response(&receiver, 2, Duration::from_secs(30)).unwrap_err();

        assert!(error.contains("MCP approval prompt received"));
        assert!(error.contains("threadId=thread-approval-001"));
        assert!(error.contains("codex/exec_approval"));
    }
}
