//! LSP client for UX scenario tests.
//!
//! Spawns the real `perl-lsp` binary via stdio and communicates using the
//! JSON-RPC 2.0 / LSP Content-Length framing protocol.  All server-initiated
//! messages (`window/showMessage`, `window/logMessage`, diagnostic
//! notifications, etc.) are captured in an event queue so scenarios can
//! assert on user-visible messages after the fact.

use crate::{FakeWorkspace, ScenarioConfig};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

static NEXT_ID: AtomicU64 = AtomicU64::new(100);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// A server-initiated event captured during a scenario.
#[derive(Debug, Clone)]
pub enum LspEvent {
    /// `window/showMessage` — user-visible modal notification.
    WindowMessage {
        /// LSP MessageType (1=Error, 2=Warning, 3=Info, 4=Log).
        message_type: u32,
        message: String,
    },
    /// `window/logMessage` — IDE output panel message.
    LogMessage { message_type: u32, message: String },
    /// `textDocument/publishDiagnostics` — diagnostic update.
    Diagnostics { uri: String, diagnostics: Vec<Value> },
    /// Any other server-initiated notification.
    Other { method: String, params: Value },
}

/// A lightweight LSP client that speaks directly to a spawned perl-lsp process.
pub struct UxClient {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    /// Events buffered from the server's stdout (notifications, etc.)
    events: Arc<Mutex<VecDeque<Value>>>,
    /// Responses to requests (matched by id).
    responses: Arc<Mutex<VecDeque<Value>>>,
    _stdout_thread: std::thread::JoinHandle<()>,
    _stderr_thread: std::thread::JoinHandle<()>,
}

impl UxClient {
    /// Spawn the perl-lsp binary and perform the LSP handshake
    /// (`initialize` + `initialized`).
    pub fn spawn(
        binary_path: &str,
        workspace: &FakeWorkspace,
        config: &ScenarioConfig,
    ) -> Result<Self> {
        let mut cmd = build_command(binary_path, config)?;

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn perl-lsp from {:?}", binary_path))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("perl-lsp stdin not available after spawn"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("perl-lsp stdout not available after spawn"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("perl-lsp stderr not available after spawn"))?;

        let events: Arc<Mutex<VecDeque<Value>>> = Arc::new(Mutex::new(VecDeque::new()));
        let responses: Arc<Mutex<VecDeque<Value>>> = Arc::new(Mutex::new(VecDeque::new()));

        // ── stdout reader thread ──────────────────────────────────────────────
        let ev_clone = events.clone();
        let resp_clone = responses.clone();
        let _stdout_thread = std::thread::Builder::new()
            .name("ux-lsp-stdout".into())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                while let Ok(msg) = read_one_message(&mut reader) {
                    let has_id = msg.get("id").is_some() && !msg["id"].is_null();
                    let is_response =
                        has_id && (msg.get("result").is_some() || msg.get("error").is_some());
                    if is_response {
                        if let Ok(mut guard) = resp_clone.lock() {
                            guard.push_back(msg);
                        }
                    } else if let Ok(mut guard) = ev_clone.lock() {
                        guard.push_back(msg);
                    }
                }
            })
            .context("Failed to spawn stdout reader thread")?;

        // ── stderr drain thread ───────────────────────────────────────────────
        let echo = config.echo_stderr;
        let _stderr_thread = std::thread::Builder::new()
            .name("ux-lsp-stderr".into())
            .spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(l) if echo => eprintln!("[perl-lsp stderr] {}", l),
                        _ => {}
                    }
                }
            })
            .context("Failed to spawn stderr drain thread")?;

        // Allow the server a moment to start before we send initialize.
        std::thread::sleep(Duration::from_millis(50));

        let client = Self {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            events,
            responses,
            _stdout_thread,
            _stderr_thread,
        };

        // ── LSP handshake ─────────────────────────────────────────────────────
        client.handshake(workspace, config, config.timeout)?;

        Ok(client)
    }

    fn handshake(
        &self,
        workspace: &FakeWorkspace,
        config: &ScenarioConfig,
        timeout: Duration,
    ) -> Result<()> {
        let workspace_folders = config
            .workspace_folders
            .iter()
            .map(|(relative_path, name)| {
                Ok(json!({
                    "uri": workspace.dir_uri(relative_path)?,
                    "name": name,
                }))
            })
            .collect::<Result<Vec<Value>>>()?;

        let root_uri = if workspace_folders.is_empty() {
            Value::String(workspace.root_uri.clone())
        } else {
            Value::Null
        };

        let mut params = json!({
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {
                "general": {
                    "positionEncodings": ["utf-16"]
                },
                "textDocument": {
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "formatting": {},
                    "definition": {},
                    "publishDiagnostics": {
                        "relatedInformation": true
                    }
                },
                "workspace": {
                    "workspaceFolders": true
                },
                "window": {
                    "showMessage": {}
                }
            }
        });
        if !workspace_folders.is_empty() {
            params["workspaceFolders"] = Value::Array(workspace_folders);
        }

        let init_resp = self.request("initialize", params, timeout)?;

        if let Some(err) = init_resp.get("error") {
            return Err(anyhow!("LSP initialize returned error: {}", err));
        }

        self.notify("initialized", json!({}))?;

        Ok(())
    }

    /// Send a JSON-RPC request and wait for the matching response.
    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let id = next_id();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.send_raw(&msg)?;
        self.wait_for_response(id, timeout)
    }

    /// Send a JSON-RPC notification (no response expected).
    pub fn notify(&self, method: &str, params: Value) -> Result<()> {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.send_raw(&msg)
    }

    /// Send `textDocument/didOpen`.
    pub fn did_open(&self, uri: &str, text: &str) -> Result<()> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }),
        )
    }

    /// Send `textDocument/didChange` using full-document sync.
    pub fn did_change(&self, uri: &str, text: &str, version: i32) -> Result<()> {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": uri,
                    "version": version
                },
                "contentChanges": [
                    {
                        "text": text
                    }
                ]
            }),
        )
    }

    /// Drain all buffered server-initiated events and decode them.
    ///
    /// After this call the internal queue is empty.  Use `peek_events` if you
    /// need to inspect events without consuming them.
    pub fn drain_events(&self) -> Vec<LspEvent> {
        let raw: Vec<Value> = {
            let mut guard = self.events.lock().unwrap_or_else(|e| e.into_inner());
            guard.drain(..).collect()
        };
        raw.into_iter().map(decode_event).collect()
    }

    /// Clone and decode all buffered events **without** removing them from the
    /// queue.  Safe to call before or after `drain_events` / `collect_notifications`.
    pub fn peek_events(&self) -> Vec<LspEvent> {
        let raw: Vec<Value> = {
            let guard = self.events.lock().unwrap_or_else(|e| e.into_inner());
            guard.iter().cloned().collect()
        };
        raw.into_iter().map(decode_event).collect()
    }

    /// Wait up to `timeout` for any `window/showMessage` containing `needle`.
    pub fn wait_for_message(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let guard = self.events.lock().unwrap_or_else(|e| e.into_inner());
                for msg in guard.iter() {
                    let method = msg["method"].as_str().unwrap_or("");
                    if method == "window/showMessage" || method == "window/logMessage" {
                        let text = msg["params"]["message"].as_str().unwrap_or("");
                        if text.contains(needle) {
                            return true;
                        }
                    }
                }
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn send_raw(&self, msg: &Value) -> Result<()> {
        let body = msg.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut stdin = self.stdin.lock().unwrap_or_else(|e| e.into_inner());
        stdin.write_all(header.as_bytes()).context("Failed to write LSP header to stdin")?;
        stdin.write_all(body.as_bytes()).context("Failed to write LSP body to stdin")?;
        stdin.flush().context("Failed to flush LSP stdin")?;
        Ok(())
    }

    fn wait_for_response(&self, id: u64, timeout: Duration) -> Result<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let mut guard = self.responses.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(pos) =
                    guard.iter().position(|v| v["id"].as_u64() == Some(id) || v["id"] == json!(id))
                {
                    // pos is valid since we just found it
                    if let Some(msg) = guard.remove(pos) {
                        return Ok(msg);
                    }
                }
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "Timeout waiting for LSP response to id={} after {}ms",
                    id,
                    timeout.as_millis()
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for UxClient {
    fn drop(&mut self) {
        // Best-effort graceful shutdown.
        let shutdown = r#"{"jsonrpc":"2.0","id":999998,"method":"shutdown","params":{}}"#;
        let exit = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        if let Ok(mut stdin) = self.stdin.lock() {
            for body in [shutdown, exit] {
                let hdr = format!("Content-Length: {}\r\n\r\n", body.len());
                let _ = stdin.write_all(hdr.as_bytes());
                let _ = stdin.write_all(body.as_bytes());
                let _ = stdin.flush();
            }
        }
        // Wait briefly for graceful exit then force-kill.
        for _ in 0..50 {
            if let Ok(mut child) = self.child.lock() {
                if child.try_wait().ok().flatten().is_some() {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ── Message framing ───────────────────────────────────────────────────────────

fn read_one_message(reader: &mut impl BufRead) -> Result<Value> {
    // Parse LSP Content-Length headers.
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Err(anyhow!("EOF reading LSP headers"));
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length") {
            let rest = rest.trim_start_matches(':').trim();
            content_length = rest.parse::<usize>().ok();
        }
    }
    let len = content_length.ok_or_else(|| anyhow!("No Content-Length in LSP message"))?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).context("Failed to parse LSP JSON body")
}

// ── Event decoding ────────────────────────────────────────────────────────────

fn decode_event(v: Value) -> LspEvent {
    let method = v["method"].as_str().unwrap_or("").to_string();
    match method.as_str() {
        "window/showMessage" => {
            let message_type = v["params"]["type"].as_u64().unwrap_or(0) as u32;
            let message = v["params"]["message"].as_str().unwrap_or("").to_string();
            LspEvent::WindowMessage { message_type, message }
        }
        "window/logMessage" => {
            let message_type = v["params"]["type"].as_u64().unwrap_or(0) as u32;
            let message = v["params"]["message"].as_str().unwrap_or("").to_string();
            LspEvent::LogMessage { message_type, message }
        }
        "textDocument/publishDiagnostics" => {
            let uri = v["params"]["uri"].as_str().unwrap_or("").to_string();
            let diagnostics = v["params"]["diagnostics"].as_array().cloned().unwrap_or_default();
            LspEvent::Diagnostics { uri, diagnostics }
        }
        _ => LspEvent::Other { method, params: v["params"].clone() },
    }
}

// ── Command construction ──────────────────────────────────────────────────────

fn build_command(binary_path: &str, config: &ScenarioConfig) -> Result<Command> {
    let mut cmd = Command::new(binary_path);
    cmd.arg("--stdio");

    // Apply restricted PATH if requested.
    if let Some(ref dirs) = config.path_restriction {
        use crate::env::RestrictedPath;
        let restricted = RestrictedPath::only(dirs.clone());
        cmd.env("PATH", restricted.build_path());
    }

    // Apply extra env vars / unsets.
    for (key, value) in &config.extra_env {
        match value {
            Some(v) => {
                cmd.env(key, v);
            }
            None => {
                cmd.env_remove(key);
            }
        }
    }

    Ok(cmd)
}
