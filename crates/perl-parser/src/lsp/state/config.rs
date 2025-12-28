//! Server configuration management
//!
//! Runtime configuration for the LSP server, including inlay hints
//! and test runner settings.

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    // Inlay hints configuration
    pub inlay_hints_enabled: bool,
    pub inlay_hints_parameter_hints: bool,
    pub inlay_hints_type_hints: bool,
    pub inlay_hints_chained_hints: bool,
    pub inlay_hints_max_length: usize,

    // Test runner configuration
    pub test_runner_enabled: bool,
    pub test_runner_command: String,
    pub test_runner_args: Vec<String>,
    pub test_runner_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            inlay_hints_enabled: true,
            inlay_hints_parameter_hints: true,
            inlay_hints_type_hints: true,
            inlay_hints_chained_hints: false,
            inlay_hints_max_length: 30,
            test_runner_enabled: true,
            test_runner_command: "perl".to_string(),
            test_runner_args: vec![],
            test_runner_timeout: 60000,
        }
    }
}

impl ServerConfig {
    /// Update configuration from LSP settings
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(inlay) = settings.get("inlayHints") {
            if let Some(enabled) = inlay.get("enabled").and_then(|v| v.as_bool()) {
                self.inlay_hints_enabled = enabled;
            }
            if let Some(param) = inlay.get("parameterHints").and_then(|v| v.as_bool()) {
                self.inlay_hints_parameter_hints = param;
            }
            if let Some(type_hints) = inlay.get("typeHints").and_then(|v| v.as_bool()) {
                self.inlay_hints_type_hints = type_hints;
            }
            if let Some(chained) = inlay.get("chainedHints").and_then(|v| v.as_bool()) {
                self.inlay_hints_chained_hints = chained;
            }
            if let Some(max_len) = inlay.get("maxLength").and_then(|v| v.as_u64()) {
                self.inlay_hints_max_length = max_len as usize;
            }
        }

        if let Some(test) = settings.get("testRunner") {
            if let Some(enabled) = test.get("enabled").and_then(|v| v.as_bool()) {
                self.test_runner_enabled = enabled;
            }
            if let Some(cmd) = test.get("command").and_then(|v| v.as_str()) {
                self.test_runner_command = cmd.to_string();
            }
            if let Some(args) = test.get("args").and_then(|v| v.as_array()) {
                self.test_runner_args =
                    args.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            }
            if let Some(timeout) = test.get("timeout").and_then(|v| v.as_u64()) {
                self.test_runner_timeout = timeout;
            }
        }
    }
}
