//! Window and telemetry operations
//!
//! Implements LSP window/* and telemetry/event methods for client interaction.

use super::*;

/// Message type for window/showMessageRequest
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MessageType {
    /// Error message (1)
    Error = 1,
    /// Warning message (2)
    Warning = 2,
    /// Info message (3)
    Info = 3,
    /// Log message (4)
    Log = 4,
}

/// Options for window/showDocument request
#[derive(Debug, Clone, Default)]
pub struct ShowDocumentOptions {
    /// Whether to open the document in an external program
    pub external: bool,
    /// Whether to take focus after showing
    pub take_focus: bool,
    /// Optional selection range to reveal
    pub selection: Option<lsp_types::Range>,
}

impl LspServer {
    /// Show a message dialog with action buttons and wait for user selection
    ///
    /// Sends a `window/showMessageRequest` request to the client and returns
    /// the title of the selected action, or None if dismissed.
    ///
    /// # Arguments
    /// * `message_type` - The severity level of the message
    /// * `message` - The message text to display
    /// * `actions` - Optional list of action button labels
    ///
    /// # Returns
    /// * `Ok(Some(String))` - User selected an action, returns its title
    /// * `Ok(None)` - User dismissed the dialog without selecting
    /// * `Err(_)` - Communication error or client doesn't support requests
    ///
    /// # Note
    /// This is a simplified implementation that sends the request but does not
    /// handle the response in a synchronous manner. A full implementation would
    /// require an async runtime or response handling mechanism.
    pub fn show_message_request(
        &self,
        message_type: MessageType,
        message: &str,
        actions: Vec<&str>,
    ) -> io::Result<()> {
        let action_items: Vec<Value> =
            actions.iter().map(|title| json!({ "title": title })).collect();

        let params = json!({
            "type": message_type as i32,
            "message": message,
            "actions": if action_items.is_empty() { Value::Null } else { json!(action_items) }
        });

        self.send_request_internal("window/showMessageRequest", params)?;

        Ok(())
    }

    /// Request the client to show/reveal a document
    ///
    /// Sends a `window/showDocument` request to ask the client to display
    /// a document, optionally in an external program.
    ///
    /// # Arguments
    /// * `uri` - The document URI to show (file://, http://, etc.)
    /// * `options` - Display options (external, focus, selection)
    ///
    /// # Returns
    /// * `Ok(())` - Request sent successfully
    /// * `Err(_)` - Client doesn't support showDocument or communication error
    pub fn show_document(&self, uri: &str, options: ShowDocumentOptions) -> io::Result<()> {
        if !self.client_capabilities.show_document_support {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Client doesn't support window/showDocument",
            ));
        }

        let mut params = json!({
            "uri": uri,
        });

        if let Some(obj) = params.as_object_mut() {
            if options.external {
                obj.insert("external".to_string(), json!(true));
            }
            if options.take_focus {
                obj.insert("takeFocus".to_string(), json!(true));
            }
            if let Some(range) = options.selection {
                obj.insert("selection".to_string(), json!(range));
            }
        }

        self.send_request_internal("window/showDocument", params)?;

        Ok(())
    }

    /// Create a work done progress token for long-running operations
    ///
    /// Sends a `window/workDoneProgress/create` request to register a progress
    /// token with the client before sending progress notifications.
    ///
    /// # Arguments
    /// * `token` - Unique token string to identify this progress
    ///
    /// # Returns
    /// * `Ok(())` - Token successfully created
    /// * `Err(_)` - Client doesn't support progress or token already exists
    pub fn create_work_done_progress(&self, token: &str) -> io::Result<()> {
        if !self.client_capabilities.work_done_progress_support {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Client doesn't support work done progress",
            ));
        }

        // Check if token already exists
        {
            let tokens = self.progress_tokens.lock();
            if tokens.contains(token) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("Progress token '{}' already exists", token),
                ));
            }
        }

        let params = json!({
            "token": token,
        });

        // Send request
        self.send_request_internal("window/workDoneProgress/create", params)?;

        // Register token on success
        self.progress_tokens.lock().insert(token.to_string());

        Ok(())
    }

    /// Report progress begin notification
    ///
    /// Sends a `$/progress` notification with "begin" kind to start a progress operation.
    /// The token must be created with `create_work_done_progress` first.
    ///
    /// # Arguments
    /// * `token` - The progress token
    /// * `title` - Title of the progress operation
    /// * `message` - Optional message text
    pub fn report_progress_begin(
        &self,
        token: &str,
        title: &str,
        message: Option<&str>,
    ) -> io::Result<()> {
        let mut value = json!({
            "kind": "begin",
            "title": title,
        });

        if let Some(msg) = message {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("message".to_string(), json!(msg));
            }
        }

        let params = json!({
            "token": token,
            "value": value,
        });

        self.notify("$/progress", params)
    }

    /// Report progress update notification
    ///
    /// Sends a `$/progress` notification with "report" kind to update progress.
    ///
    /// # Arguments
    /// * `token` - The progress token
    /// * `message` - Optional message text
    /// * `percentage` - Optional percentage (0-100)
    pub fn report_progress_report(
        &self,
        token: &str,
        message: Option<&str>,
        percentage: Option<u32>,
    ) -> io::Result<()> {
        let mut value = json!({
            "kind": "report",
        });

        if let Some(obj) = value.as_object_mut() {
            if let Some(msg) = message {
                obj.insert("message".to_string(), json!(msg));
            }
            if let Some(pct) = percentage {
                obj.insert("percentage".to_string(), json!(pct));
            }
        }

        let params = json!({
            "token": token,
            "value": value,
        });

        self.notify("$/progress", params)
    }

    /// Report progress end notification
    ///
    /// Sends a `$/progress` notification with "end" kind to complete a progress operation.
    ///
    /// # Arguments
    /// * `token` - The progress token
    /// * `message` - Optional final message text
    pub fn report_progress_end(&self, token: &str, message: Option<&str>) -> io::Result<()> {
        let mut value = json!({
            "kind": "end",
        });

        if let Some(msg) = message {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("message".to_string(), json!(msg));
            }
        }

        let params = json!({
            "token": token,
            "value": value,
        });

        // Remove token from active set
        self.progress_tokens.lock().remove(token);

        self.notify("$/progress", params)
    }

    /// Send telemetry event notification
    ///
    /// Sends a `telemetry/event` notification with arbitrary data.
    /// Only sends if telemetry is enabled in configuration.
    ///
    /// # Arguments
    /// * `event` - Arbitrary JSON value containing telemetry data
    pub fn send_telemetry(&self, event: Value) -> io::Result<()> {
        // Check if telemetry is enabled
        let enabled = self.config.lock().telemetry_enabled;
        if !enabled {
            return Ok(()); // Silently skip if disabled
        }

        self.notify("telemetry/event", event)
    }

    /// Handle window/workDoneProgress/cancel notification from client
    ///
    /// Client sends this to request cancellation of a progress operation.
    /// Server should cancel the associated task.
    ///
    /// # Arguments
    /// * `params` - Notification params containing the token
    pub(super) fn handle_progress_cancel(&self, params: Option<Value>) {
        if let Some(params) = params {
            if let Some(token) = params.get("token").and_then(|v| v.as_str()) {
                // Remove from active tokens
                let removed = self.progress_tokens.lock().remove(token);

                if removed {
                    eprintln!("Progress cancelled by client: {}", token);
                    // Note: Actual task cancellation would be handled by the operation
                    // using the token. This just tracks the token state.
                } else {
                    eprintln!("Progress cancel for unknown token: {}", token);
                }
            }
        }
    }

    /// Send a request to the client (internal helper)
    ///
    /// Internal helper to send JSON-RPC requests. Uses the existing send_request
    /// infrastructure which auto-generates request IDs.
    fn send_request_internal(&self, method: &str, params: Value) -> io::Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        });

        let request_str = serde_json::to_string(&request)?;

        // Send request
        let mut output = self.output.lock();
        write!(output, "Content-Length: {}\r\n\r\n{}", request_str.len(), request_str)?;
        output.flush()
    }
}
