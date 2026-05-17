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
//! - `UX_TEST_TIMEOUT_MS`: Per-request timeout in milliseconds (default: 30000).
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
pub mod diagnostics;
pub mod env;
pub mod project_fixture;
pub mod recorder;
pub mod scorecard;
pub mod taxonomy;
pub mod workspace;

pub use client::{LspEvent, UxClient};
pub use diagnostics::DiagnosticsTracker;
pub use env::{PathGuard, RestrictedPath};
pub use project_fixture::{
    ProjectFixtureFile, create_fixture_harness, fixture_content, fixture_scenario_config,
    load_catalyst_fixture_files, load_dancer2_fixture_files, load_mojolicious_fixture_files,
    open_all_fixture_files, workspace_root,
};
pub use recorder::{
    AssertionBasis, AssertionCounts, OperationTiming, RunIdentity, UxCheckFailure, UxRunRecorder,
    UxScenarioRunReceipt, UxScenarioSkip, run_ux_scenario,
};
pub use scorecard::{EditorUxScorecard, ScenarioScore, aggregate_editor_ux_scorecard};
pub use taxonomy::{
    MetricState, UxCiTier, UxComponent, UxFailureClass, UxRoute, UxScenarioResult,
    route_for_failure_class,
};
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
    /// Per-request timeout. Defaults to 30 seconds.
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
            .unwrap_or(30_000);
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

    /// Request completion and collect best-effort labels for UX assertions.
    ///
    /// Label extraction order per completion item:
    /// 1. `label` (preferred by spec)
    /// 2. `insertText` (fallback for legacy payloads)
    /// 3. `filterText` (last-resort fallback)
    pub fn completion_labels(
        &self,
        relative_path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<String>> {
        let items = self.completion(relative_path, line, character)?;
        Ok(items
            .iter()
            .filter_map(|item| {
                item.get("label")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("insertText").and_then(Value::as_str))
                    .or_else(|| item.get("filterText").and_then(Value::as_str))
                    .map(str::to_string)
            })
            .collect())
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

    /// Poll `workspace/symbol` until `predicate` returns true or `timeout`
    /// elapses.
    ///
    /// Returns the last observed symbol list in both success and timeout paths.
    pub fn wait_for_workspace_symbols(
        &self,
        query: &str,
        timeout: Duration,
        poll_interval: Duration,
        mut predicate: impl FnMut(&[Value]) -> bool,
    ) -> Result<Vec<Value>> {
        let deadline = std::time::Instant::now() + timeout;
        let mut latest = Vec::new();

        while std::time::Instant::now() < deadline {
            latest = self.workspace_symbols(query)?;
            if predicate(&latest) {
                return Ok(latest);
            }
            std::thread::sleep(poll_interval);
        }

        Ok(latest)
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
    /// for the given file, then return the first published diagnostics collected
    /// for it.
    ///
    /// Returns an empty vec if the deadline expires with no diagnostics published.
    /// To get the most recently published diagnostics instead, use
    /// [`UxHarness::wait_for_latest_diagnostics`].
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
                for ev in events.iter() {
                    if let LspEvent::Diagnostics { uri: diag_uri, diagnostics, .. } = ev {
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

    /// Wait up to `timeout` for a `textDocument/publishDiagnostics` notification
    /// for the given file, then return the most recently published diagnostics
    /// for the URI, ignoring earlier buffered publications.
    ///
    /// Returns an empty vec if the deadline expires with no diagnostics published.
    /// Use this when you need the latest server state after an edit; for the
    /// initial (first published) diagnostics use [`UxHarness::wait_for_diagnostics`].
    pub fn wait_for_latest_diagnostics(
        &self,
        relative_path: &str,
        timeout: std::time::Duration,
    ) -> Vec<Value> {
        let uri = self.workspace.uri(relative_path);
        let deadline = std::time::Instant::now() + timeout;
        loop {
            {
                let events = self.client.peek_events();
                for ev in events.iter().rev() {
                    if let LspEvent::Diagnostics { uri: diag_uri, diagnostics, .. } = ev {
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

    /// Wait for diagnostics to become empty for a file (cleared UX state).
    ///
    /// Returns `true` if an explicit `textDocument/publishDiagnostics` with an
    /// **empty** diagnostics array arrives within `timeout`, or if the latest
    /// buffered notification for that URI already has an empty array.
    ///
    /// Returns `false` on timeout.  Note that if the server clears diagnostics
    /// silently (no explicit notification) this method will timeout and return
    /// `false`.  In that case prefer checking that no *new* non-empty
    /// notifications arrive within the deadline instead.
    pub fn wait_for_no_diagnostics(
        &self,
        relative_path: &str,
        timeout: std::time::Duration,
    ) -> bool {
        let uri = self.workspace.uri(relative_path);
        DiagnosticsTracker::wait_for_uri_matching(
            || self.client.peek_events(),
            &uri,
            timeout,
            |diagnostics| diagnostics.is_empty(),
        )
        .is_some()
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

    /// Request go-to-definition and optionally retry to absorb asynchronous indexing delays.
    ///
    /// Returns immediately when the first non-empty response is observed, or after
    /// `attempts` tries (minimum 1). This keeps UX scenarios deterministic without
    /// forcing each test to hand-roll sleep/retry loops.
    pub fn definition_with_retry(
        &self,
        relative_path: &str,
        line: u32,
        character: u32,
        attempts: usize,
        pause: Duration,
    ) -> Result<Vec<Value>> {
        let mut last = Vec::new();
        let max_attempts = attempts.max(1);

        for idx in 0..max_attempts {
            let current = self.definition(relative_path, line, character)?;
            if !current.is_empty() {
                return Ok(current);
            }

            last = current;
            if idx + 1 < max_attempts {
                std::thread::sleep(pause);
            }
        }

        Ok(last)
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
    // Short-circuit: sentinel tokens already contain "$WORKSPACE" and must be returned
    // verbatim.  On Windows the url crate interprets "$WORKSPACE" as a UNC host and
    // Url::to_file_path() succeeds, producing a mangled path like `\\$workspace\foo`.
    // Checking up-front is both correct and cheaper than parsing.
    if uri.contains("$WORKSPACE") {
        return uri.to_string();
    }

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

    // Non-workspace file URI: return the url crate's canonical form, which already
    // handles Windows drive letters correctly (file:///C:/...) without reconstruction.
    parsed.to_string()
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

/// Return whether the perl-lsp binary can be resolved for UX scenario tests.
///
/// This is a lightweight guard for integration tests that need to skip when the
/// server binary has not been built in the current environment.
pub fn binary_available() -> bool {
    resolve_binary().is_ok()
}

/// Standard skip reason for scenarios that require a runnable perl-lsp binary.
pub fn missing_binary_skip() -> UxScenarioSkip {
    UxScenarioSkip::infra("PERL_LSP_BIN not set and target/debug/perl-lsp not found")
}

/// Resolve the path to the perl-lsp binary.
///
/// Resolution order:
/// 1. `PERL_LSP_BIN` env var (explicit override).
/// 2. Runtime walk from `current_exe()` — finds `target/debug/perl-lsp[.exe]` by
///    traversing parent directories. Avoids the `option_env!` compile-time approach
///    which strips backslashes on Windows CI (OS error 3 / path not found).
/// 3. `CARGO_TARGET_DIR` env var — if set, probe its `debug/` and `release/` subdirs.
/// 4. `CARGO_MANIFEST_DIR`-relative workspace root walk — same approach used by
///    `perl-lsp-rs` integration tests.
/// 5. `perl-lsp` / `perllsp` in PATH.
/// 6. Error with actionable message.
pub fn resolve_binary() -> Result<String> {
    // 1. Explicit override
    if let Ok(p) = std::env::var("PERL_LSP_BIN") {
        if !p.is_empty() {
            return Ok(p);
        }
    }

    // 2. Runtime walk from current_exe() — robust on all platforms including
    //    Windows where option_env! bakes paths with backslashes stripped.
    //
    //    Test binaries live at:
    //      <workspace>/target/debug/deps/<test-binary-name>[.exe]
    //    The LSP server lives at:
    //      <workspace>/target/debug/perl-lsp[.exe]
    //      <workspace>/target/release/perl-lsp[.exe]
    //
    //    We walk up from current_exe() until we find a `target` directory
    //    whose parent contains `Cargo.lock` (the workspace root).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(binary) = find_binary_near_exe(&exe) {
            return Ok(binary);
        }
    }

    // 3. CARGO_TARGET_DIR — if set, look directly in its debug/release subdirs.
    //    This covers custom target directories (e.g. agent worktrees using
    //    CARGO_TARGET_DIR=/tmp/agent-...). Note: CARGO_TARGET_DIR is the target
    //    directory itself (not a workspace root), so we look in
    //    CARGO_TARGET_DIR/debug/ and CARGO_TARGET_DIR/release/ directly.
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        let target_path = std::path::Path::new(&target_dir);
        let bin_name = if cfg!(windows) { "perl-lsp.exe" } else { "perl-lsp" };
        for profile in ["debug", "release"] {
            let candidate = target_path.join(profile).join(bin_name);
            if candidate.exists() {
                return Ok(candidate.to_string_lossy().into_owned());
            }
        }
    }

    // 4. CARGO_MANIFEST_DIR walk — find workspace root via Cargo.lock, then
    //    check target/{debug,release}/perl-lsp[.exe].
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let crate_dir = std::path::Path::new(&manifest_dir);
        let workspace_root =
            crate_dir.ancestors().find(|p| p.join("Cargo.lock").exists()).unwrap_or(crate_dir);
        if let Some(binary) = find_binary_in_target(workspace_root) {
            return Ok(binary);
        }
    }

    // 5. PATH lookup
    if let Ok(p) = which::which("perl-lsp") {
        return Ok(p.to_string_lossy().to_string());
    }
    if let Ok(p) = which::which("perllsp") {
        return Ok(p.to_string_lossy().to_string());
    }

    // 6. No binary found — actionable error message.
    Err(anyhow!(
        "perl-lsp binary not found. \
        Set PERL_LSP_BIN=/path/to/perl-lsp or run: cargo build -p perl-lsp-rs"
    ))
}

/// Walk up from the test binary's path to locate `perl-lsp[.exe]` in the
/// nearest `target/debug` or `target/release` directory.
///
/// Test binaries are placed in `<workspace>/target/<profile>/deps/`, so we
/// ascend until we find a directory named `target` whose parent has a
/// `Cargo.lock` file, then probe `<profile>/perl-lsp[.exe]`.
fn find_binary_near_exe(exe: &std::path::Path) -> Option<String> {
    // Walk up the ancestor chain looking for a `target` directory.
    for ancestor in exe.ancestors() {
        if ancestor.file_name().and_then(|n| n.to_str()) == Some("target") {
            let workspace_root = ancestor.parent()?;
            if workspace_root.join("Cargo.lock").exists() {
                return find_binary_in_target(workspace_root);
            }
        }
    }
    None
}

/// Given a workspace root, probe `target/debug` and `target/release` for
/// the `perl-lsp` binary (with `.exe` extension on Windows).
fn find_binary_in_target(workspace_root: &std::path::Path) -> Option<String> {
    let bin_name = if cfg!(windows) { "perl-lsp.exe" } else { "perl-lsp" };
    let alt_bin_name = if cfg!(windows) { "perllsp.exe" } else { "perllsp" };

    // Prefer debug (matches `cargo test` default profile) over release.
    let profiles = ["debug", "release"];
    for profile in profiles {
        let candidate = workspace_root.join("target").join(profile).join(bin_name);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        let alt_candidate = workspace_root.join("target").join(profile).join(alt_bin_name);
        if alt_candidate.exists() {
            return Some(alt_candidate.to_string_lossy().into_owned());
        }
    }
    None
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

#[cfg(test)]
mod normalize_tests {
    use super::{normalize_lsp_payload, normalize_uri_for_expectations};
    use serde_json::{Value, json};
    use std::path::Path;
    use tempfile::TempDir;

    // ── normalize_uri_for_expectations ────────────────────────────────────────

    #[test]
    fn non_file_uri_passes_through_unchanged() {
        let root = Path::new("/tmp/workspace");
        let result = normalize_uri_for_expectations("untitled:foo.pl", root);
        assert_eq!(result, "untitled:foo.pl");
    }

    #[test]
    fn malformed_uri_has_backslashes_replaced() {
        let root = Path::new("/tmp/workspace");
        // Not a valid URI — Url::parse will fail, so backslash-replace branch runs.
        let result = normalize_uri_for_expectations("not a uri \\path", root);
        assert_eq!(result, "not a uri /path");
    }

    #[test]
    fn workspace_file_uri_becomes_dollar_workspace_token() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let file_uri = url::Url::from_file_path(root.join("lib/Foo.pm")).unwrap().to_string();
        let result = normalize_uri_for_expectations(&file_uri, root);
        assert_eq!(result, "file://$WORKSPACE/lib/Foo.pm");
    }

    #[test]
    fn workspace_root_uri_itself_becomes_dollar_workspace_slash() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let root_uri = url::Url::from_file_path(root).unwrap().to_string();
        let result = normalize_uri_for_expectations(&root_uri, root);
        // strip_prefix of root against itself gives "" -> "file://$WORKSPACE/"
        assert_eq!(result, "file://$WORKSPACE/");
    }

    #[test]
    fn non_workspace_file_uri_preserved_with_forward_slashes() {
        let root = Path::new("/tmp/workspace");
        // A system path outside the workspace should not be rewritten as $WORKSPACE.
        let result = normalize_uri_for_expectations("file:///usr/share/perl5/strict.pm", root);
        assert_eq!(result, "file:///usr/share/perl5/strict.pm");
    }

    #[test]
    fn directory_uri_with_trailing_slash_normalizes_correctly() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        // Url::from_directory_path produces a trailing slash.
        // to_file_path() strips it, so strip_prefix sees "svc-a" (no slash).
        // The result must NOT have a trailing slash in the sentinel form.
        let dir_uri = url::Url::from_directory_path(root.join("svc-a")).unwrap().to_string();
        assert!(dir_uri.ends_with('/'), "directory URI should end with /");
        let result = normalize_uri_for_expectations(&dir_uri, root);
        assert_eq!(result, "file://$WORKSPACE/svc-a");
    }

    // ── normalize_lsp_payload ─────────────────────────────────────────────────

    #[test]
    fn null_value_passes_through() {
        let dir = TempDir::new().unwrap();
        let result = normalize_lsp_payload(&Value::Null, dir.path());
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn scalar_values_pass_through() {
        let dir = TempDir::new().unwrap();
        assert_eq!(normalize_lsp_payload(&json!(42), dir.path()), json!(42));
        assert_eq!(normalize_lsp_payload(&json!(true), dir.path()), json!(true));
        assert_eq!(normalize_lsp_payload(&json!("hello"), dir.path()), json!("hello"));
    }

    #[test]
    fn uri_key_in_object_is_normalized() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let file_uri = url::Url::from_file_path(root.join("foo.pl")).unwrap().to_string();
        let payload =
            json!({ "uri": file_uri, "range": { "start": { "line": 0, "character": 0 } } });
        let result = normalize_lsp_payload(&payload, root);
        assert_eq!(result["uri"], "file://$WORKSPACE/foo.pl");
        // Range should be preserved unchanged.
        assert_eq!(result["range"]["start"]["line"], 0);
    }

    #[test]
    fn target_uri_key_in_object_is_normalized() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let file_uri = url::Url::from_file_path(root.join("bar.pm")).unwrap().to_string();
        let payload = json!({ "targetUri": file_uri });
        let result = normalize_lsp_payload(&payload, root);
        assert_eq!(result["targetUri"], "file://$WORKSPACE/bar.pm");
    }

    #[test]
    fn workspace_folder_uri_key_is_normalized() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let folder_uri = url::Url::from_directory_path(root.join("svc-a")).unwrap().to_string();
        let payload = json!({ "workspaceFolderUri": folder_uri });
        let result = normalize_lsp_payload(&payload, root);
        // The value should start with file://$WORKSPACE/svc-a regardless of trailing slash.
        let normalized = result["workspaceFolderUri"].as_str().unwrap();
        assert!(
            normalized.starts_with("file://$WORKSPACE/svc-a"),
            "Expected svc-a token, got: {normalized}"
        );
    }

    #[test]
    fn non_uri_key_string_value_is_not_rewritten() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let file_uri = url::Url::from_file_path(root.join("foo.pl")).unwrap().to_string();
        // A key named "someOtherField" holding a file URI should NOT be normalized.
        let payload = json!({ "someOtherField": file_uri });
        let result = normalize_lsp_payload(&payload, root);
        assert_eq!(result["someOtherField"].as_str().unwrap(), file_uri.as_str());
    }

    #[test]
    fn array_of_locations_normalizes_each_entry() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let uri_a = url::Url::from_file_path(root.join("a.pl")).unwrap().to_string();
        let uri_b = url::Url::from_file_path(root.join("b.pm")).unwrap().to_string();
        let payload = json!([
            { "uri": uri_a },
            { "uri": uri_b },
        ]);
        let result = normalize_lsp_payload(&payload, root);
        assert_eq!(result[0]["uri"], "file://$WORKSPACE/a.pl");
        assert_eq!(result[1]["uri"], "file://$WORKSPACE/b.pm");
    }

    #[test]
    fn nested_uri_in_object_is_normalized_recursively() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let file_uri = url::Url::from_file_path(root.join("deep.pl")).unwrap().to_string();
        // URI nested inside a non-uri-keyed wrapper should still be normalized.
        let payload = json!({ "location": { "uri": file_uri } });
        let result = normalize_lsp_payload(&payload, root);
        assert_eq!(result["location"]["uri"], "file://$WORKSPACE/deep.pl");
    }

    #[test]
    fn dollar_workspace_token_in_expected_passes_through_unchanged() {
        // assert_normalized_eq normalizes BOTH sides. A literal "$WORKSPACE" token
        // in the expected side must survive the round-trip unchanged, so it can
        // match the normalized actual side.
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let expected_payload = json!({ "uri": "file://$WORKSPACE/foo.pl" });
        let result = normalize_lsp_payload(&expected_payload, root);
        // Url::parse("file://$WORKSPACE/foo.pl") treats $WORKSPACE as host and
        // to_file_path() fails -> backslash-replace branch returns string unchanged.
        assert_eq!(result["uri"], "file://$WORKSPACE/foo.pl");
    }
}
