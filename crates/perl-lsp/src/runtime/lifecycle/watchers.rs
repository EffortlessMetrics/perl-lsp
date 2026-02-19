//! File watcher registration
//!
//! Handles registration of file watchers for workspace files.

use super::super::*;
use lsp_types::{
    DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, Registration,
    RegistrationParams, WatchKind,
    notification::{DidChangeWatchedFiles, Notification},
};
use serde_json::json;

impl LspServer {
    /// Register file watchers for Perl files
    pub(crate) fn register_file_watchers_async(&self) {
        if !self.advertised_features.lock().workspace_symbol {
            return;
        }

        let watchers = vec![
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.pl".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.pm".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.t".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.psgi".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
        ];

        let opts = DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = match serde_json::to_value(opts) {
            Ok(val) => Some(val),
            Err(e) => {
                eprintln!("[perl-lsp] Failed to serialize file watcher options: {}", e);
                return;
            }
        };
        let reg = Registration {
            id: "perl-didChangeWatchedFiles".into(),
            method: <DidChangeWatchedFiles as Notification>::METHOD.to_string(),
            register_options,
        };

        let params = RegistrationParams { registrations: vec![reg] };

        // Send the registration request without waiting for a response
        // Use a random ID since we're not tracking the response
        let request_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let request = json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Number(serde_json::Number::from(request_id)),
            "method": "client/registerCapability",
            "params": params
        });

        // Send using the proper output mechanism with explicit error logging
        let mut output = self.output.lock();
        match serde_json::to_string(&request) {
            Ok(msg) => {
                if let Err(e) = write!(output, "Content-Length: {}\r\n\r\n{}", msg.len(), msg) {
                    eprintln!("[perl-lsp] Failed to write file watcher request: {}", e);
                    return;
                }
                if let Err(e) = output.flush() {
                    eprintln!("[perl-lsp] Failed to flush file watcher request: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[perl-lsp] Failed to serialize file watcher request: {}", e);
            }
        }
    }
}
