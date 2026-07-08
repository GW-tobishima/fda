use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::{Child, ChildStdin, Command as ProcessCommand, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static SMTP_MESSAGE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct SmtpConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) from: String,
    pub(crate) tls_mode: String,
}

pub(crate) struct SmtpSendResponse {
    pub(crate) message_id: String,
    pub(crate) provider_response_digest: String,
}

pub(crate) fn smtp_config_from_env() -> Result<SmtpConfig, String> {
    let required = [
        "FDA_SMTP_HOST",
        "FDA_SMTP_PORT",
        "FDA_SMTP_USERNAME",
        "FDA_SMTP_PASSWORD",
        "FDA_SMTP_FROM",
    ];
    let missing = required
        .iter()
        .filter(|name| {
            env::var(name)
                .ok()
                .is_none_or(|value| value.trim().is_empty())
        })
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("missing required SMTP env: {}", missing.join(", ")));
    }
    let port_raw = env::var("FDA_SMTP_PORT").unwrap_or_default();
    let port = port_raw
        .trim()
        .parse::<u16>()
        .map_err(|_| "invalid FDA_SMTP_PORT".to_string())?;
    let tls_mode = env::var("FDA_SMTP_TLS_MODE").unwrap_or_else(|_| "starttls".to_string());
    let tls_mode = tls_mode.trim().to_ascii_lowercase();
    if !matches!(tls_mode.as_str(), "starttls" | "tls" | "none") {
        return Err("invalid FDA_SMTP_TLS_MODE; expected starttls, tls, or none".to_string());
    }
    Ok(SmtpConfig {
        host: env::var("FDA_SMTP_HOST").unwrap_or_default(),
        port,
        username: env::var("FDA_SMTP_USERNAME").unwrap_or_default(),
        password: env::var("FDA_SMTP_PASSWORD").unwrap_or_default(),
        from: env::var("FDA_SMTP_FROM").unwrap_or_default(),
        tls_mode,
    })
}

pub(crate) fn send_smtp_notification(
    config: &SmtpConfig,
    sender: &str,
    recipient: &str,
    body: &str,
    message_id: &str,
) -> Result<SmtpSendResponse, String> {
    let mut connection = SmtpConnection::connect(config)?;
    if config.tls_mode != "starttls" {
        connection.expect_ready(220)?;
    }
    connection.command("EHLO forge-delivery-agent.local", 250)?;
    connection.command("AUTH LOGIN", 334)?;
    connection.command(&smtp_auth_base64(config.username.as_bytes()), 334)?;
    connection.command(&smtp_auth_base64(config.password.as_bytes()), 235)?;
    connection.command(&format!("MAIL FROM:<{sender}>"), 250)?;
    connection.command(&format!("RCPT TO:<{recipient}>"), 250)?;
    connection.command("DATA", 354)?;
    connection.write_raw(body)?;
    let data_response = connection.read_response()?;
    if data_response.code != 250 {
        return Err(format!(
            "smtp DATA rejected with status {}",
            data_response.code
        ));
    }
    let _ = connection.command("QUIT", 221);
    Ok(SmtpSendResponse {
        message_id: message_id.to_string(),
        provider_response_digest: smtp_response_digest(&data_response.message),
    })
}

pub(crate) fn smtp_envelope_address(value: &str, label: &str) -> Result<String, String> {
    let address = value.trim();
    if address.is_empty() {
        return Err(format!("invalid {label}: empty"));
    }
    if address.len() > 320 {
        return Err(format!("invalid {label}: too long"));
    }
    if address
        .chars()
        .any(|character| character == '\r' || character == '\n')
    {
        return Err(format!("invalid {label}: contains CR/LF"));
    }
    if address.chars().any(|character| character.is_control()) {
        return Err(format!("invalid {label}: contains control character"));
    }
    if address.contains('<') || address.contains('>') {
        return Err(format!("invalid {label}: angle brackets are not allowed"));
    }
    if !address.contains('@') {
        return Err(format!("invalid {label}: missing @"));
    }
    Ok(address.to_string())
}

pub(crate) fn smtp_message_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = SMTP_MESSAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "<fda-{nanos}-{}-{counter}@forge-delivery-agent.local>",
        std::process::id()
    )
}

pub(crate) fn smtp_resolve_addresses(config: &SmtpConfig) -> Result<Vec<SocketAddr>, String> {
    let host = config.host.clone();
    let port = config.port;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = (host.as_str(), port)
            .to_socket_addrs()
            .map(|addresses| addresses.collect::<Vec<_>>())
            .map_err(|_| "smtp address resolution failed".to_string());
        let _ = sender.send(result);
    });
    receiver
        .recv_timeout(Duration::from_secs(30))
        .map_err(|_| "smtp address resolution timed out".to_string())?
}

struct SmtpResponse {
    code: u16,
    message: String,
}

enum SmtpConnection {
    Plain {
        reader: BufReader<TcpStream>,
        writer: TcpStream,
    },
    Process {
        child: Child,
        reader: Receiver<Result<String, String>>,
        writer: ChildStdin,
    },
}

impl SmtpConnection {
    fn connect(config: &SmtpConfig) -> Result<Self, String> {
        match config.tls_mode.as_str() {
            "none" => {
                let stream = smtp_tcp_connect(config)?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(30)))
                    .map_err(|_| "smtp read timeout setup failed".to_string())?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(30)))
                    .map_err(|_| "smtp write timeout setup failed".to_string())?;
                let writer = stream
                    .try_clone()
                    .map_err(|_| "smtp stream clone failed".to_string())?;
                Ok(SmtpConnection::Plain {
                    reader: BufReader::new(stream),
                    writer,
                })
            }
            "tls" | "starttls" => {
                let mut command = ProcessCommand::new("openssl");
                command
                    .arg("s_client")
                    .arg("-quiet")
                    .arg("-connect")
                    .arg(format!("{}:{}", config.host, config.port))
                    .arg("-servername")
                    .arg(config.host.as_str())
                    .arg("-verify_hostname")
                    .arg(config.host.as_str())
                    .arg("-verify_return_error")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null());
                if config.tls_mode == "starttls" {
                    command.arg("-starttls").arg("smtp");
                }
                let mut child = command
                    .spawn()
                    .map_err(|_| "failed to start openssl for SMTP TLS".to_string())?;
                let writer = child
                    .stdin
                    .take()
                    .ok_or_else(|| "failed to open SMTP TLS stdin".to_string())?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| "failed to open SMTP TLS stdout".to_string())?;
                let reader = spawn_smtp_process_reader(stdout);
                Ok(SmtpConnection::Process {
                    child,
                    reader,
                    writer,
                })
            }
            _ => Err("invalid SMTP TLS mode".to_string()),
        }
    }

    fn expect_ready(&mut self, expected: u16) -> Result<(), String> {
        let response = self.read_response()?;
        if response.code == expected {
            Ok(())
        } else {
            Err(format!("smtp server returned status {}", response.code))
        }
    }

    fn command(&mut self, command: &str, expected: u16) -> Result<SmtpResponse, String> {
        self.write_raw(&format!("{command}\r\n"))?;
        let response = self.read_response()?;
        if response.code == expected {
            Ok(response)
        } else {
            Err(format!("smtp command failed with status {}", response.code))
        }
    }

    fn write_raw(&mut self, value: &str) -> Result<(), String> {
        match self {
            SmtpConnection::Plain { writer, .. } => writer
                .write_all(value.as_bytes())
                .and_then(|_| writer.flush())
                .map_err(|_| "smtp write failed".to_string()),
            SmtpConnection::Process { writer, .. } => writer
                .write_all(value.as_bytes())
                .and_then(|_| writer.flush())
                .map_err(|_| "smtp TLS write failed".to_string()),
        }
    }

    fn read_response(&mut self) -> Result<SmtpResponse, String> {
        let mut message = String::new();
        let mut code = None;
        loop {
            let mut line = String::new();
            let read = match self {
                SmtpConnection::Plain { reader, .. } => reader
                    .read_line(&mut line)
                    .map_err(|_| "smtp read failed".to_string())?,
                SmtpConnection::Process { reader, .. } => {
                    line = reader
                        .recv_timeout(Duration::from_secs(30))
                        .map_err(|_| "smtp TLS read timed out".to_string())??;
                    line.len()
                }
            };
            if read == 0 {
                return Err("smtp connection closed".to_string());
            }
            message.push_str(&line);
            if line.len() >= 4 {
                if code.is_none() {
                    code = line[0..3].parse::<u16>().ok();
                }
                if line.as_bytes().get(3) == Some(&b' ') {
                    break;
                }
            }
        }
        Ok(SmtpResponse {
            code: code.ok_or_else(|| "smtp response parse failed".to_string())?,
            message,
        })
    }
}

fn smtp_tcp_connect(config: &SmtpConfig) -> Result<TcpStream, String> {
    let addresses = smtp_resolve_addresses(config)?;
    if addresses.is_empty() {
        return Err("smtp address resolution returned no addresses".to_string());
    }

    let mut last_error = "smtp connection failed".to_string();
    for address in addresses {
        match TcpStream::connect_timeout(&address, Duration::from_secs(30)) {
            Ok(stream) => return Ok(stream),
            Err(_) => {
                last_error = "smtp connection failed or timed out".to_string();
            }
        }
    }
    Err(last_error)
}

fn spawn_smtp_process_reader(
    stdout: std::process::ChildStdout,
) -> Receiver<Result<String, String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = sender.send(Err("smtp connection closed".to_string()));
                    break;
                }
                Ok(_) => {
                    if sender.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = sender.send(Err("smtp read failed".to_string()));
                    break;
                }
            }
        }
    });
    receiver
}

impl Drop for SmtpConnection {
    fn drop(&mut self) {
        if let SmtpConnection::Process { child, .. } = self {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn smtp_auth_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn smtp_response_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}
