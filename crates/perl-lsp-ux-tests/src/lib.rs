//! UX regression test harness for perl-lsp.
//!
//! Provides a programmatic simulation of common first-5-minutes user experiences.
//! Each scenario:
//! 1. Sets up a clean-room environment (tempdir, fake workspace, controlled PATH).
//! 2. Spawns the LSP server binary (real process, real stdio).
//! 3. Sends a scripted sequence of LSP requests.
//! 4. Verifies the server responds correctly — not just "didn't crash" but
//!    "returned a useful response".
//! 5. Captures `window/showMessage` and `window/logMessage` events for assertions.
//! 6. Cleans up automatically via RAII.
//!
//! # Quick Start
//!
//! ```no_run
//! use perl_lsp_ux_tests::{UxHarness, ScenarioConfig};
//!
//! let harness = UxHarness::new(ScenarioConfig::default()).unwrap();
//! harness.open_file("test.pl", "my $x = 42;\n").unwrap();
//! let hover = harness.hover("test.pl", 0, 3).unwrap();
//! assert!(hover.is_some(), "hover should return something for $x");
//! ```
//!
//! # Adding a New Scenario
//!
//! 1. Create `tests/scenarios/my_scenario.rs`.
//! 2. Use `UxHarness::new(ScenarioConfig { ... })` to set up the environment.
//! 3. Call harness methods to drive LSP interactions.
//! 4. Assert on responses with helpers like `assert_no_crash`, `assert_message_contains`.
//! 5. The harness auto-cleans up when dropped.
//!
//! # Environment Variables
//!
//! - `PERL_LSP_BIN`: Override the path to the perl-lsp binary.
//! - `UX_TEST_TIMEOUT_MS`: Per-request timeout in milliseconds (default: 10000).
//! - `UX_TEST_ECHO_STDERR`: If set, echo perl-lsp stderr lines to test output.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::module_name_repetitions
)]

pub mod client;
pub mod env;
pub mod scorecard;
pub mod workspace;

pub use client::{LspEvent, UxClient};
pub use env::{PathGuard, RestrictedPath};
pub use scorecard::{EditorUxScorecard, ScenarioScore, aggregate_editor_ux_scorecard};
pub use workspace::FakeWorkspace;

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use url::Url;

/// Canonical cursor position for editor-facing UX requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPosition {
    /// Relative file path in the temp workspace.
    pub relative_path: String,
    /// 0-based line offset.
    pub line: u32,
    /// 0-based UTF-16 code-unit offset.
    pub character: u32,
}

impl CursorPosition {
    /// Create a cursor position at `(line, character)` inside `relative_path`.
    pub fn new(relative_path: impl Into<String>, line: u32, character: u32) -> Self {
        Self { relative_path: relative_path.into(), line, character }
    }
}

/// Configuration for a UX scenario.
///
/// Centralises all the knobs that affect the test environment without
/// requiring callers to thread individual parameters through every helper.
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    /// Per-request timeout. Defaults to 10 seconds.
    pub timeout: Duration,
    /// If `Some`, restrict PATH to only these directory entries (absolute paths).
    /// This lets scenarios simulate "perltidy not found" without touching the
    /// real environment in a way that leaks to other tests.
    ///
    /// Note: PATH restriction is applied to the *child process* environment only.
    /// The test runner process PATH is not modified.
    pub path_restriction: Option<Vec<String>>,
    /// If true, echo the LSP server's stderr to the test output.
    pub echo_stderr: bool,
    /// Extra environment variables to pass to the LSP server process.
    /// Use `None` values to unset a variable.
    pub extra_env: Vec<(String, Option<String>)>,
    /// Initial workspace files: (relative_path, content) pairs.
    pub workspace_files: Vec<(String, String)>,
    /// Optional workspace folders for multi-root initialization.
    /// Each entry is `(relative_path, name)`.
    pub workspace_folders: Vec<(String, String)>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        let timeout_ms = std::env::var("UX_TEST_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10_000);
        let echo_stderr = std::env::var_os("UX_TEST_ECHO_STDERR").is_some();
        Self {
            timeout: Duration::from_millis(timeout_ms),
            path_restriction: None,
            echo_stderr,
            extra_env: Vec::new(),
            workspace_files: Vec::new(),
            workspace_folders: Vec::new(),
        }
    }
}

impl ScenarioConfig {
    /// Create a config with only the listed directories on PATH.
    pub fn with_restricted_path(dirs: Vec<String>) -> Self {
        Self { path_restriction: Some(dirs), ..Default::default() }
    }

    /// Create a config with PATH completely cleared (simulates no tools installed).
    pub fn with_empty_path() -> Self {
        Self { path_restriction: Some(Vec::new()), ..Default::default() }
    }

    /// Add an environment variable to pass to the server process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_env.push((key.into(), Some(value.into())));
        self
    }

    /// Unset an environment variable in the server process.
    pub fn unset_env(mut self, key: impl Into<String>) -> Self {
        self.extra_env.push((key.into(), None));
        self
    }

    /// Add initial workspace files.
    pub fn with_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.workspace_files.push((path.into(), content.into()));
        self
    }

    /// Add a workspace folder for multi-root initialization.
    pub fn with_workspace_folder(
        mut self,
        relative_path: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        self.workspace_folders.push((relative_path.into(), name.into()));
        self
    }
}

/// The main UX test harness.
///
/// Wraps a spawned LSP server process and a temporary workspace.
/// Provides high-level helpers that map to common user interactions.
/// Cleans up automatically when dropped.
pub struct UxHarness {
    pub client: UxClient,
    pub workspace: FakeWorkspace,
    config: ScenarioConfig,
    document_versions: Mutex<HashMap<String, i32>>,
}

impl UxHarness {
    /// Spawn a fresh LSP server and set up a clean workspace.
    pub fn new(config: ScenarioConfig) -> Result<Self> {
        let workspace = FakeWorkspace::new()?;

        // Write any pre-seeded workspace files.
        for (path, content) in &config.workspace_files {
            workspace.write(path, content)?;
        }

        for (path, _) in &config.workspace_folders {
            workspace.ensure_dir(path)?;
        }

        let binary_path = resolve_binary()?;

        let client = UxClient::spawn(&binary_path, &workspace, &config)
            .context("Failed to spawn LSP server")?;

        Ok(Self { client, workspace, config, document_versions: Mutex::new(HashMap::new()) })
    }

    /// Open a file in the LSP server (textDocument/didOpen).
    ///
    /// Creates the file in the temp workspace first if it does not exist.
    pub fn open_file(&self, relative_path: &str, content: &str) -> Result<()> {
        self.workspace.write(relative_path, content)?;
        let uri = self.workspace.uri(relative_path);
        self.client.did_open(&uri, content)?;
        self.document_versions.lock().unwrap_or_else(|e| e.into_inner()).insert(uri, 1);
        Ok(())
    }

    /// Open a fixture file pre-seeded in `ScenarioConfig.workspace_files`.
    pub fn open_fixture(&self, relative_path: &str) -> Result<()> {
        let content = std::fs::read_to_string(self.workspace.path(relative_path))
            .with_context(|| format!("Fixture file {:?} was not pre-seeded", relative_path))?;
        self.open_file(relative_path, &content)
    }

    /// Build a canonical cursor position for subsequent UX requests.
    pub fn position_cursor(
        &self,
        relative_path: impl Into<String>,
        line: u32,
        character: u32,
    ) -> CursorPosition {
        CursorPosition::new(relative_path, line, character)
    }

    /// Apply a full-document text replacement and send `textDocument/didChange`.
    pub fn change_file_full(&self, relative_path: &str, updated_content: &str) -> Result<()> {
        self.workspace.write(relative_path, updated_content)?;
        let uri = self.workspace.uri(relative_path);
        let version = {
            let mut versions = self.document_versions.lock().unwrap_or_else(|e| e.into_inner());
            let entry = versions.entry(uri.clone()).or_insert(1);
            *entry += 1;
            *entry
        };
        self.client.did_change_full(&uri, version, updated_content)
    }

    /// Open a file in the LSP server with an explicit language identifier.
    ///
    /// Useful for UX regressions where the editor mode intentionally differs
    /// from the file extension (for example, opening `*.html.ep` as HTML).
    pub fn open_file_with_language_id(
        &self,
        relative_path: &str,
        content: &str,
        language_id: &str,
    ) -> Result<()> {
        self.workspace.write(relative_path, content)?;
        let uri = self.workspace.uri(relative_path);
        self.client.did_open_with_language_id(&uri, content, language_id)
    }

    /// Request hover information at `(line, character)` (0-indexed UTF-16).
    ///
    /// Returns `None` if the server returned a null/empty result (degraded mode is OK).
    /// Returns `Err` only if the server returned a JSON-RPC error or timed out.
    pub fn hover(&self, relative_path: &str, line: u32, character: u32) -> Result<Option<Value>> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
            self.config.timeout,
        )?;
        if resp["result"].is_null() {
            return Ok(None);
        }
        Ok(Some(resp["result"].clone()))
    }

    /// Request completion at `(line, character)`.
    pub fn completion(&self, relative_path: &str, line: u32, character: u32) -> Result<Vec<Value>> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "triggerKind": 1 }
            }),
            self.config.timeout,
        )?;
        if resp.get("error").is_some() {
            return Err(anyhow!("completion returned error: {}", resp["error"]));
        }
        match resp["result"]["items"].as_array() {
            Some(items) => Ok(items.clone()),
            None => match resp["result"].as_array() {
                Some(items) => Ok(items.clone()),
                None => Ok(Vec::new()),
            },
        }
    }

    /// Request completion at a canonical cursor position.
    pub fn completion_at(&self, cursor: &CursorPosition) -> Result<Vec<Value>> {
        self.completion(&cursor.relative_path, cursor.line, cursor.character)
    }

    /// Request document formatting.
    ///
    /// Returns the list of text edits, or `Err` if the server crashed / returned
    /// a hard error. An empty list is acceptable (formatting may be a no-op).
    pub fn format_document(&self, relative_path: &str) -> Result<FormatResult> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": uri },
                "options": { "tabSize": 4, "insertSpaces": true }
            }),
            self.config.timeout,
        )?;
        if let Some(err) = resp.get("error") {
            return Ok(FormatResult::Error(err.clone()));
        }
        match resp["result"].as_array() {
            Some(edits) => Ok(FormatResult::Edits(edits.clone())),
            None => Ok(FormatResult::Empty),
        }
    }

    /// Request document symbols (`textDocument/documentSymbol`).
    ///
    /// Returns the flat list of `SymbolInformation` or `DocumentSymbol` objects,
    /// or an empty vec if the server returned null/empty.
    pub fn document_symbols(&self, relative_path: &str) -> Result<Vec<Value>> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/documentSymbol",
            json!({
                "textDocument": { "uri": uri }
            }),
            self.config.timeout,
        )?;
        if resp.get("error").is_some() {
            return Err(anyhow!("documentSymbol returned error: {}", resp["error"]));
        }
        match resp["result"].as_array() {
            Some(syms) => Ok(syms.clone()),
            None => {
                if resp["result"].is_null() {
                    Ok(Vec::new())
                } else {
                    Ok(vec![resp["result"].clone()])
                }
            }
        }
    }

    /// Request workspace symbols (`workspace/symbol`).
    ///
    /// Returns the flat list of workspace symbol objects, or an empty vec if
    /// the server returned null/empty.
    pub fn workspace_symbols(&self, query: &str) -> Result<Vec<Value>> {
        let resp = self.client.request(
            "workspace/symbol",
            json!({
                "query": query
            }),
            self.config.timeout,
        )?;
        if resp.get("error").is_some() {
            return Err(anyhow!("workspace/symbol returned error: {}", resp["error"]));
        }
        match resp["result"].as_array() {
            Some(symbols) => Ok(symbols.clone()),
            None => {
                if resp["result"].is_null() {
                    Ok(Vec::new())
                } else {
                    Ok(vec![resp["result"].clone()])
                }
            }
        }
    }

    /// Notify the server that workspace folders changed.
    ///
    /// Each tuple is `(relative_path, name)` and is resolved relative to the
    /// temporary workspace root.
    pub fn change_workspace_folders(
        &self,
        added: &[(&str, &str)],
        removed: &[(&str, &str)],
    ) -> Result<()> {
        let added = added
            .iter()
            .map(|(relative_path, name)| {
                Ok(json!({
                    "uri": self.workspace.dir_uri(relative_path)?,
                    "name": name,
                }))
            })
            .collect::<Result<Vec<Value>>>()?;

        let removed = removed
            .iter()
            .map(|(relative_path, name)| {
                Ok(json!({
                    "uri": self.workspace.dir_uri(relative_path)?,
                    "name": name,
                }))
            })
            .collect::<Result<Vec<Value>>>()?;

        self.client.notify(
            "workspace/didChangeWorkspaceFolders",
            json!({
                "event": {
                    "added": added,
                    "removed": removed,
                }
            }),
        )
    }

    /// Notify the server about file watcher changes.
    ///
    /// Each tuple is `(relative_path, change_type)` where `change_type`
    /// follows the LSP `FileChangeType` numeric values:
    /// 1 = Created, 2 = Changed, 3 = Deleted.
    pub fn notify_watched_files(&self, changes: &[(&str, u32)]) -> Result<()> {
        let changes = changes
            .iter()
            .map(|(relative_path, change_type)| {
                json!({
                    "uri": self.workspace.uri(relative_path),
                    "type": change_type,
                })
            })
            .collect::<Vec<Value>>();

        self.client.notify(
            "workspace/didChangeWatchedFiles",
            json!({
                "changes": changes,
            }),
        )
    }

    /// Wait up to `timeout` for a `textDocument/publishDiagnostics` notification
    /// for the given file, then return all diagnostics collected for it.
    ///
    /// Returns an empty vec if the deadline expires with no diagnostics published.
    pub fn wait_for_diagnostics(
        &self,
        relative_path: &str,
        timeout: std::time::Duration,
    ) -> Vec<Value> {
        let uri = self.workspace.uri(relative_path);
        let deadline = std::time::Instant::now() + timeout;
        loop {
            {
                let events = self.client.peek_events();
                for ev in &events {
                    if let LspEvent::Diagnostics { uri: diag_uri, diagnostics } = ev {
                        if diag_uri == &uri {
                            return diagnostics.clone();
                        }
                    }
                }
            }
            if std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Vec::new()
    }

    /// Request go-to-definition.
    pub fn definition(&self, relative_path: &str, line: u32, character: u32) -> Result<Vec<Value>> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
            self.config.timeout,
        )?;
        if resp.get("error").is_some() {
            return Err(anyhow!("definition returned error: {}", resp["error"]));
        }
        match resp["result"].as_array() {
            Some(locs) => Ok(locs.clone()),
            None => {
                if resp["result"].is_null() {
                    Ok(Vec::new())
                } else {
                    // Single location object
                    Ok(vec![resp["result"].clone()])
                }
            }
        }
    }

    /// Request go-to-definition at a canonical cursor position.
    pub fn definition_at(&self, cursor: &CursorPosition) -> Result<Vec<Value>> {
        self.definition(&cursor.relative_path, cursor.line, cursor.character)
    }

    /// Request references (`textDocument/references`) at a cursor position.
    pub fn references(
        &self,
        relative_path: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Value>> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": include_declaration }
            }),
            self.config.timeout,
        )?;
        if resp.get("error").is_some() {
            return Err(anyhow!("references returned error: {}", resp["error"]));
        }
        match resp["result"].as_array() {
            Some(locs) => Ok(locs.clone()),
            None if resp["result"].is_null() => Ok(Vec::new()),
            None => Ok(vec![resp["result"].clone()]),
        }
    }

    /// Request references at a canonical cursor position.
    pub fn references_at(
        &self,
        cursor: &CursorPosition,
        include_declaration: bool,
    ) -> Result<Vec<Value>> {
        self.references(&cursor.relative_path, cursor.line, cursor.character, include_declaration)
    }

    /// Request go-to-declaration.
    pub fn declaration(
        &self,
        relative_path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>> {
        let uri = self.workspace.uri(relative_path);
        let resp = self.client.request(
            "textDocument/declaration",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
            self.config.timeout,
        )?;
        if resp.get("error").is_some() {
            return Err(anyhow!("declaration returned error: {}", resp["error"]));
        }
        match resp["result"].as_array() {
            Some(locs) => Ok(locs.clone()),
            None => {
                if resp["result"].is_null() {
                    Ok(Vec::new())
                } else {
                    // Single location object
                    Ok(vec![resp["result"].clone()])
                }
            }
        }
    }

    /// Drain any pending server-initiated messages (window/showMessage, etc.)
    /// and return them. Non-blocking — returns what's already buffered.
    ///
    /// After this call the internal event queue is empty.  Use
    /// `peek_notifications` if you need the events to remain available for
    /// subsequent `assert_no_crash` / `assert_message_contains` calls.
    pub fn collect_notifications(&self) -> Vec<LspEvent> {
        self.client.drain_events()
    }

    /// Clone pending server-initiated messages **without** removing them from
    /// the queue.  Safe to call multiple times or before assertion helpers.
    pub fn peek_notifications(&self) -> Vec<LspEvent> {
        self.client.peek_events()
    }

    /// Assert that none of the buffered events contain a crash signature.
    /// Fails the test loudly if any suspicious message is found.
    ///
    /// Uses a non-draining peek so subsequent `assert_message_contains` /
    /// `assert_no_message_containing` calls still see the same events.
    pub fn assert_no_crash(&self) {
        let events = self.client.peek_events();
        for ev in &events {
            let msg = format!("{:?}", ev);
            assert!(
                !msg.contains("panicked")
                    && !msg.contains("SIGABRT")
                    && !msg.contains("stack overflow"),
                "LSP server appears to have crashed. Event: {:?}",
                ev
            );
        }
    }

    /// Assert that at least one buffered `window/showMessage` or
    /// `window/logMessage` event contains `needle` (substring match).
    ///
    /// Uses a non-draining peek so the events remain available for
    /// `assert_no_crash` or further assertions.
    pub fn assert_message_contains(&self, needle: &str) {
        let events = self.client.peek_events();
        let found = events.iter().any(|ev| {
            if let LspEvent::WindowMessage { message, .. } | LspEvent::LogMessage { message, .. } =
                ev
            {
                message.contains(needle)
            } else {
                false
            }
        });
        assert!(
            found,
            "Expected a server message containing {:?} but none was found.\nMessages received: {:?}",
            needle,
            events
                .iter()
                .filter_map(|ev| match ev {
                    LspEvent::WindowMessage { message, .. }
                    | LspEvent::LogMessage { message, .. } => {
                        Some(message.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
    }

    /// Assert that none of the messages contain `needle`.
    ///
    /// Uses a non-draining peek so the events remain available for other
    /// assertion helpers called in the same test.
    pub fn assert_no_message_containing(&self, needle: &str) {
        let events = self.client.peek_events();
        for ev in &events {
            if let LspEvent::WindowMessage { message, .. } | LspEvent::LogMessage { message, .. } =
                ev
            {
                assert!(
                    !message.contains(needle),
                    "Unexpected message containing {:?}: {:?}",
                    needle,
                    message
                );
            }
        }
    }

    /// Returns the root URI of the workspace (useful for the `rootUri` initialize param).
    pub fn root_uri(&self) -> &str {
        &self.workspace.root_uri
    }

    /// Apply a full-document edit and wait for diagnostics from that file.
    pub fn apply_edit_and_collect_diagnostics(
        &self,
        relative_path: &str,
        updated_content: &str,
        timeout: Duration,
    ) -> Result<Vec<Value>> {
        self.change_file_full(relative_path, updated_content)?;
        Ok(self.wait_for_diagnostics(relative_path, timeout))
    }

    /// Normalize LSP payloads for platform-stable expectations.
    ///
    /// - Workspace file URIs are rewritten as `file://$WORKSPACE/<relative-path>`.
    /// - Directory separators in normalized URIs are always `/`.
    pub fn normalize_response(&self, payload: &Value) -> Value {
        normalize_lsp_payload(payload, self.workspace.dir.path())
    }

    /// Assert that two LSP payloads match after canonical normalization.
    pub fn assert_normalized_eq(&self, actual: &Value, expected: &Value) {
        let normalized_actual = self.normalize_response(actual);
        let normalized_expected = self.normalize_response(expected);
        assert_eq!(
            normalized_actual, normalized_expected,
            "normalized payload mismatch\nactual={:#}\nexpected={:#}",
            normalized_actual, normalized_expected
        );
    }
}

/// Normalize editor-facing LSP payloads so fixture assertions are OS-stable.
pub fn normalize_lsp_payload(payload: &Value, workspace_root: &Path) -> Value {
    match payload {
        Value::Array(values) => Value::Array(
            values.iter().map(|entry| normalize_lsp_payload(entry, workspace_root)).collect(),
        ),
        Value::Object(map) => {
            let mut normalized = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                if matches!(key.as_str(), "uri" | "targetUri" | "workspaceFolderUri") {
                    if let Some(uri) = value.as_str() {
                        normalized.insert(
                            key.clone(),
                            Value::String(normalize_uri_for_expectations(uri, workspace_root)),
                        );
                        continue;
                    }
                }
                normalized.insert(key.clone(), normalize_lsp_payload(value, workspace_root));
            }
            Value::Object(normalized)
        }
        _ => payload.clone(),
    }
}

fn normalize_uri_for_expectations(uri: &str, workspace_root: &Path) -> String {
    let Ok(parsed) = Url::parse(uri) else {
        return uri.replace('\\', "/");
    };

    if parsed.scheme() != "file" {
        return uri.to_string();
    }

    let Ok(path) = parsed.to_file_path() else {
        return uri.replace('\\', "/");
    };

    if let Ok(relative) = path.strip_prefix(workspace_root) {
        let relative = relative.to_string_lossy().replace('\\', "/");
        return format!("file://$WORKSPACE/{}", relative.trim_start_matches('/'));
    }

    format!("file://{}", path.to_string_lossy().replace('\\', "/"))
}

/// Outcome of a formatting request.
#[derive(Debug)]
pub enum FormatResult {
    /// Formatter returned text edits.
    Edits(Vec<Value>),
    /// Formatter returned null/empty (no-op, acceptable).
    Empty,
    /// Formatter returned a JSON-RPC error object.
    Error(Value),
}

impl FormatResult {
    /// True if the result is an error (not just empty / no-op).
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Extract the error message string if this is an error.
    pub fn error_message(&self) -> Option<&str> {
        if let Self::Error(v) = self { v["message"].as_str() } else { None }
    }

    /// True if there are text edits.
    pub fn has_edits(&self) -> bool {
        matches!(self, Self::Edits(v) if !v.is_empty())
    }
}

// ─────────────────────────────── Binary Resolution ───────────────────────────

/// Resolve the path to the perl-lsp binary.
///
/// Resolution order:
/// 1. `PERL_LSP_BIN` env var (explicit override).
/// 2. `CARGO_BIN_EXE_perl-lsp` compile-time constant (set by Cargo during tests).
/// 3. `perl-lsp` in PATH.
/// 4. `cargo run -p perl-lsp-rs` fallback (slow but always works).
pub fn resolve_binary() -> Result<String> {
    // 1. Explicit override
    if let Ok(p) = std::env::var("PERL_LSP_BIN") {
        if !p.is_empty() {
            return Ok(p);
        }
    }

    // 2. Compile-time constant (only available when tests run via `cargo test`)
    if let Some(p) = option_env!("CARGO_BIN_EXE_perl-lsp") {
        return Ok(p.to_string());
    }

    // 3. PATH lookup
    if let Ok(p) = which::which("perl-lsp") {
        return Ok(p.to_string_lossy().to_string());
    }
    if let Ok(p) = which::which("perllsp") {
        return Ok(p.to_string_lossy().to_string());
    }

    // 4. cargo run fallback — we return the cargo invocation as a string
    // handled specially by UxClient::spawn.
    Err(anyhow!(
        "perl-lsp binary not found. \
        Set PERL_LSP_BIN=/path/to/perl-lsp or run: cargo build -p perl-lsp-rs"
    ))
}

/// Utility: find `perl` on PATH, returning its path or `None`.
pub fn find_perl() -> Option<String> {
    which::which("perl").ok().map(|p| p.to_string_lossy().to_string())
}

/// Utility: find `perltidy` on PATH, returning its path or `None`.
pub fn find_perltidy() -> Option<String> {
    which::which("perltidy").ok().map(|p| p.to_string_lossy().to_string())
}

/// Utility: find `perlcritic` on PATH, returning its path or `None`.
pub fn find_perlcritic() -> Option<String> {
    which::which("perlcritic").ok().map(|p| p.to_string_lossy().to_string())
}
