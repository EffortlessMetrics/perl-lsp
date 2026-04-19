//! TestContext compatibility wrapper for the LSP test harness.
//!
//! Provides a `TestContext` type that mirrors the old `TestContext` API
//! while delegating to `LspHarness` underneath.  This enables mechanical
//! migration of existing tests with minimal diff — only the import path
//! changes.

#![allow(dead_code)]

use perl_tdd_support::must;
use serde_json::{Value, json};

use super::lsp_harness::LspHarness;

/// A compatibility wrapper that provides the same API as the old TestContext
/// but uses LspHarness underneath for proper initialization and synchronization.
///
/// This enables mechanical migration of tests from TestContext to LspHarness
/// with minimal diff.
///
/// # Migration
///
/// Old code:
/// ```ignore
/// let mut ctx = TestContext::new();
/// let _ = ctx.initialize();
/// ctx.open_document(uri, text);
/// let result = ctx.send_request("textDocument/hover", params);
/// ```
///
/// New code (just change imports):
/// ```ignore
/// use support::lsp_harness::TestContext;  // <-- Changed import
/// let mut ctx = TestContext::new();  // Same API
/// let _ = ctx.initialize();
/// ctx.open_document(uri, text);
/// let result = ctx.send_request("textDocument/hover", params);
/// ```
pub struct TestContext {
    harness: LspHarness,
    version_counter: i32,
}

impl TestContext {
    /// Create a new test context with uninitialized harness
    pub fn new() -> Self {
        Self {
            harness: LspHarness::new_raw(),
            version_counter: 1,
        }
    }

    /// Initialize the LSP server and wait for it to be fully ready
    ///
    /// Returns the initialization response value.
    /// Unlike the old TestContext, this includes a barrier to ensure the server is ready.
    /// Uses default root_uri "file:///workspace" and default capabilities.
    pub fn initialize(&mut self) -> Value {
        self.initialize_with("file:///workspace", None)
    }

    /// Initialize the LSP server with custom root_uri and capabilities
    ///
    /// Returns the initialization response value.
    /// This includes a barrier to ensure the server is fully ready.
    ///
    /// # Arguments
    /// * `root_uri` - The workspace root URI (e.g., "file:///test" or a real temp directory)
    /// * `capabilities` - Optional custom client capabilities (None = sensible defaults)
    pub fn initialize_with(&mut self, root_uri: &str, capabilities: Option<Value>) -> Value {
        match self.harness.initialize_ready(root_uri, capabilities) {
            Ok(v) => v,
            Err(e) => must(Err::<Value, _>(format!(
                "initialization should succeed: {e}"
            ))),
        }
    }

    /// Send a request and wait for response
    ///
    /// Returns `Some(result)` on success, `None` on error.
    /// Note: `params: None` maps to JSON `null`, not `{}` - this is correct per JSON-RPC spec.
    pub fn send_request(&mut self, method: &str, params: Option<Value>) -> Option<Value> {
        let p = params.unwrap_or(json!(null));
        self.harness.request(method, p).ok()
    }

    /// Send a notification (no response expected)
    /// Note: `params: None` maps to JSON `null`, not `{}` - this is correct per JSON-RPC spec.
    pub fn send_notification(&mut self, method: &str, params: Option<Value>) {
        let p = params.unwrap_or(json!(null));
        self.harness.notify(method, p);
    }

    /// Open a document
    pub fn open_document(&mut self, uri: &str, text: &str) {
        match self.harness.open(uri, text) {
            Ok(_) => {}
            Err(e) => must(Err::<(), _>(format!("open should succeed: {e}"))),
        }
    }

    /// Update document content with auto-incrementing version
    pub fn update_document(&mut self, uri: &str, text: &str) {
        self.version_counter += 1;
        match self.harness.change_full(uri, self.version_counter, text) {
            Ok(_) => {}
            Err(e) => must(Err::<(), _>(format!("change should succeed: {e}"))),
        }
    }

    /// Close a document
    pub fn close_document(&mut self, uri: &str) {
        match self.harness.close(uri) {
            Ok(_) => {}
            Err(e) => must(Err::<(), _>(format!("close should succeed: {e}"))),
        }
    }

    /// Synchronization barrier - wait for server to be idle
    pub fn barrier(&mut self) {
        self.harness.barrier();
    }

    /// Get underlying harness for advanced operations
    pub fn harness(&mut self) -> &mut LspHarness {
        &mut self.harness
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}
