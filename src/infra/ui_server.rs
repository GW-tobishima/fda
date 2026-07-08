use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use crate::application::ui::{mission_control_snapshot, UiConfig};
use crate::rendering::mission_control::mission_control_page;
use crate::support::paths::resolve_path;

/// `fda ui` のローカル HTTP サーバ。
///
/// - 127.0.0.1 固定 bind の read-only projection。書き込み系エンドポイントは持たない。
/// - 毎リクエストで artifacts をディスクから読み直す（UI 側に状態を持たない）。
pub(crate) fn serve(config: &UiConfig) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", config.port))
        .map_err(|e| format!("failed to bind 127.0.0.1:{}: {e}", config.port))?;
    let address = listener
        .local_addr()
        .map_err(|e| format!("failed to resolve local address: {e}"))?;
    let url = format!("http://{address}/");
    println!("FDA Mission Control: {url}");
    println!("read-only projection（正本は artifacts / ATO / GitHub）。終了は Ctrl+C。");
    if config.open_browser {
        if let Err(error) = open_in_browser(&url) {
            println!("browser open failed: {error}");
        }
    }
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let _ = handle_connection(&mut stream, config);
    }
    Ok(())
}

fn handle_connection(stream: &mut TcpStream, config: &UiConfig) -> Result<(), String> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|e| format!("failed to clone stream: {e}"))?,
    );
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| format!("failed to read request: {e}"))?;
    // 残りのヘッダは読み捨てる（Connection: close で応答するため解釈不要）。
    loop {
        let mut header_line = String::new();
        match reader.read_line(&mut header_line) {
            Ok(0) => break,
            Ok(_) if header_line == "\r\n" || header_line == "\n" => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let response = route(config, method, path);
    write_response(stream, &response)
}

pub(crate) struct HttpResponse {
    pub(crate) status: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) body: Vec<u8>,
}

pub(crate) fn route(config: &UiConfig, method: &str, path: &str) -> HttpResponse {
    if method != "GET" {
        return plain("405 Method Not Allowed", "read-only projection です。");
    }
    match path {
        "/" => match mission_control_snapshot(config) {
            Ok(snapshot) => HttpResponse {
                status: "200 OK",
                content_type: "text/html; charset=utf-8",
                body: mission_control_page(&snapshot).into_bytes(),
            },
            Err(error) => plain("500 Internal Server Error", &error),
        },
        "/api/state.json" => match mission_control_snapshot(config) {
            Ok(snapshot) => HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::to_vec_pretty(&snapshot).unwrap_or_default(),
            },
            Err(error) => plain("500 Internal Server Error", &error),
        },
        path if path.starts_with("/artifact/") => {
            serve_artifact(config, &path["/artifact/".len()..])
        }
        _ => plain("404 Not Found", "not found"),
    }
}

fn serve_artifact(config: &UiConfig, rest: &str) -> HttpResponse {
    let segments: Vec<&str> = rest.split('/').collect();
    let &[run, file] = segments.as_slice() else {
        return plain(
            "404 Not Found",
            "artifact path must be /artifact/<run>/<file>",
        );
    };
    if !is_safe_name(run) || !is_safe_name(file) {
        return plain("404 Not Found", "invalid artifact path");
    }
    let Ok(repo_root) = std::fs::canonicalize(&config.repo_root) else {
        return plain("500 Internal Server Error", "repo root is unavailable");
    };
    let runs_root = resolve_path(&repo_root, &config.runs_root);
    let candidate = runs_root.join(run).join(file);
    // 正規化後も runs root 配下であることを確認する（path traversal ガード）。
    let Ok(canonical) = std::fs::canonicalize(&candidate) else {
        return plain("404 Not Found", "artifact not found");
    };
    let Ok(canonical_root) = std::fs::canonicalize(&runs_root) else {
        return plain("404 Not Found", "runs root not found");
    };
    if !canonical.starts_with(&canonical_root) {
        return plain("404 Not Found", "artifact is outside runs root");
    }
    match std::fs::read(&canonical) {
        Ok(body) => HttpResponse {
            status: "200 OK",
            content_type: content_type_for(&canonical),
            body,
        },
        Err(_) => plain("404 Not Found", "artifact not found"),
    }
}

fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        _ => "text/plain; charset=utf-8",
    }
}

fn plain(status: &'static str, message: &str) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "text/plain; charset=utf-8",
        body: message.as_bytes().to_vec(),
    }
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<(), String> {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(&response.body))
        .map_err(|e| format!("failed to write response: {e}"))
}

fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    result.map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        std::fs::create_dir_all(dir.join("artifacts").join("runs").join("fda-start-1")).unwrap();
        std::fs::write(
            dir.join("artifacts")
                .join("runs")
                .join("fda-start-1")
                .join("validation_report.json"),
            "{\"verdict\":\"pass\"}",
        )
        .unwrap();
        dir
    }

    fn config_for(repo: &Path) -> UiConfig {
        UiConfig {
            repo_root: repo.to_path_buf(),
            runs_root: PathBuf::from("artifacts/runs"),
            port: 0,
            open_browser: false,
            print_json: false,
        }
    }

    #[test]
    fn route_serves_mission_control_page_and_state() {
        let repo = temp_repo("fda-ui-server-page");
        let config = config_for(&repo);

        let page = route(&config, "GET", "/");
        assert_eq!(page.status, "200 OK");
        assert!(String::from_utf8_lossy(&page.body).contains("FDA Mission Control"));

        let state = route(&config, "GET", "/api/state.json");
        assert_eq!(state.status, "200 OK");
        let value: serde_json::Value = serde_json::from_slice(&state.body).unwrap();
        assert_eq!(value["schema_version"], "fda.mission_control_snapshot.v0");

        std::fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn route_serves_artifact_and_blocks_traversal() {
        let repo = temp_repo("fda-ui-server-artifact");
        let config = config_for(&repo);
        std::fs::write(repo.join("secret.txt"), "top secret").unwrap();

        let ok = route(
            &config,
            "GET",
            "/artifact/fda-start-1/validation_report.json",
        );
        assert_eq!(ok.status, "200 OK");
        assert_eq!(ok.content_type, "application/json; charset=utf-8");

        for path in [
            "/artifact/../secret.txt",
            "/artifact/fda-start-1/../../secret.txt",
            "/artifact/..%2Fsecret.txt",
            "/artifact/fda-start-1/..",
            "/artifact/fda-start-1",
        ] {
            let denied = route(&config, "GET", path);
            assert_eq!(denied.status, "404 Not Found", "path {path} must be denied");
            assert!(!String::from_utf8_lossy(&denied.body).contains("top secret"));
        }

        let post = route(&config, "POST", "/");
        assert_eq!(post.status, "405 Method Not Allowed");

        std::fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn safe_name_rejects_separators_and_dots() {
        assert!(is_safe_name("fda-start-100"));
        assert!(is_safe_name("validation_report.json"));
        assert!(!is_safe_name(".."));
        assert!(!is_safe_name("a/b"));
        assert!(!is_safe_name("a\\b"));
        assert!(!is_safe_name(""));
        assert!(!is_safe_name("a:b"));
    }

    #[test]
    fn json_value_sanity() {
        // serde_json が pretty 出力できることの回帰（api/state.json 経路の前提）。
        let body = serde_json::to_vec_pretty(&json!({"ok": true})).unwrap();
        assert!(!body.is_empty());
    }
}
