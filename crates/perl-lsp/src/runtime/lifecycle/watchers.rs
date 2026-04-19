//! File watcher registration
//!
//! Handles registration of file watchers for workspace files.

use super::super::*;
use lsp_types::{
    DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, Registration,
    RegistrationParams, WatchKind,
    notification::{DidChangeWatchedFiles, Notification},
};
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
                tracing::error!(error = %e, "Failed to serialize file watcher options");
                return;
            }
        };
        let reg = Registration {
            id: "perl-didChangeWatchedFiles".into(),
            method: <DidChangeWatchedFiles as Notification>::METHOD.to_string(),
            register_options,
        };

        let params = RegistrationParams {
            registrations: vec![reg],
        };
        let params_value = match serde_json::to_value(&params) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize registration params");
                return;
            }
        };

        // Send the registration request without waiting for a response
        // Use a random ID since we're not tracking the response
        let request_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Send using the outbound channel
        if let Err(e) =
            self.outbound
                .send_request(request_id as i64, "client/registerCapability", params_value)
        {
            tracing::error!(error = %e, "Failed to send file watcher request");
        }
    }
}
